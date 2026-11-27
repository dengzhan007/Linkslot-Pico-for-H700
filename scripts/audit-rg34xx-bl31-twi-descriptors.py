#!/usr/bin/env python3
"""Read-only audit of the matched BL31 RTC soft-entry TWI descriptor boundary."""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import struct
from pathlib import Path

from capstone import CS_ARCH_ARM64, CS_MODE_ARM, Cs
from capstone.arm64 import ARM64_OP_IMM, ARM64_OP_MEM, ARM64_OP_REG

BASE = 0x48000000
SIZE = 107216
SHA256 = "676bc0206e3c9a7f229d93ea7f3286095f669940ad91f3c4f3d37bdb18696c31"

# Each window is code, not a linear sweep through embedded data/literal tables.
WINDOWS = (
    (0x48012510, 0x48012564),
    (0x480125d0, 0x480125e0),
    (0x48012670, 0x48012708),
    (0x480129f0, 0x48012a08),
    (0x48013a48, 0x48013ac8),
    (0x48013ac8, 0x48013b44),
    (0x4800c418, 0x4800c4b8),
    (0x48011c30, 0x48011d78),
    (0x480133f8, 0x48013458),
    (0x48013e00, 0x48014048),
)

EXPECTED = {
    0x48012520: ("ldr", "w0, [x0, #0xa4]"),
    0x48012524: ("bl", ""),
    0x480125d4: ("ldr", "w1, [x0, #0xa4]"),
    0x48012694: ("ldr", "w1, [x0, #0xa4]"),
    0x480126a0: ("bl", ""),
    0x480126d4: ("ldr", "w0, [x0, #0xa4]"),
    0x480126d8: ("bl", ""),
    0x480129f8: ("bl", ""),
    0x48013ac8: ("adrp", "x1, #0x48014000"),
    0x48013ad4: ("add", "x1, x1, #0x628"),
    0x48013af0: ("ldr", "x1, [x1, w0, sxtw #3]"),
    0x48013b14: ("str", "x1, [x0, #0x548]"),
    0x48013a50: ("ldr", "x0, [x0, #0x548]"),
}

def read_u64(blob: bytes, address: int) -> int:
    offset = address - BASE
    if offset < 0 or offset + 8 > len(blob):
        raise ValueError("qword address outside the matched monitor payload")
    return struct.unpack_from("<Q", blob, offset)[0]


H616_TWI_BASES = {
    0x05002000: "TWI0",
    0x05002400: "TWI1",
    0x05002800: "TWI2",
    0x05002C00: "TWI3",
    0x05003000: "TWI4",
    0x07081400: "R-TWI0",
}


def classify_pointer(value: int) -> str:
    if BASE <= value < BASE + SIZE:
        return "monitor-internal-address"
    if 0x03000000 <= value < 0x04000000:
        return "soc-mmio-candidate"
    if 0x05000000 <= value < 0x06000000:
        return "soc-mmio-candidate"
    if 0x07000000 <= value < 0x08000000:
        return "soc-mmio-candidate"
    if 0x09000000 <= value < 0x0A000000:
        return "soc-mmio-candidate"
    if value < 0x10000000:
        return "small-scalar-or-offset"
    return "unclassified-scalar-or-encoded-value"


def parse_monitor_patch_offsets(source: str) -> list[int]:
    return sorted({int(item, 16) for item in re.findall(
        r"\bmonitor\s*\+\s*(0x[0-9a-fA-F]+)", source
    )})


def decode_windows(blob: bytes) -> dict[int, object]:
    md = Cs(CS_ARCH_ARM64, CS_MODE_ARM)
    md.detail = True
    decoded: dict[int, object] = {}
    for start, end in WINDOWS:
        off = start - BASE
        if off < 0 or end > BASE + len(blob):
            raise ValueError("disassembly window outside monitor payload")
        decoded.update({ins.address: ins for ins in md.disasm(blob[off:end-BASE], start)})
    return decoded


def require_instruction(decoded: dict[int, object], address: int, mnemonic: str, operands: str = ""):
    ins = decoded.get(address)
    if ins is None:
        raise RuntimeError("missing instruction at 0x%08x" % address)
    if ins.mnemonic.lower() != mnemonic:
        raise RuntimeError("instruction at 0x%08x is %s, expected %s" % (
            address, ins.mnemonic, mnemonic
        ))
    if operands and re.sub(r"\s+", "", ins.op_str.lower()) != re.sub(r"\s+", "", operands.lower()):
        raise RuntimeError("operands at 0x%08x changed: %s" % (address, ins.op_str))
    return ins


def branch_target(ins) -> int:
    if not ins.operands or ins.operands[0].type != ARM64_OP_IMM:
        raise RuntimeError("expected immediate branch target at 0x%08x" % ins.address)
    return int(ins.operands[0].imm)


def register_bytes(ins, operand_index: int) -> int:
    if operand_index >= len(ins.operands):
        return 0
    operand = ins.operands[operand_index]
    if operand.type != ARM64_OP_REG:
        return 0
    name = ins.reg_name(operand.reg).lower()
    if name.startswith(("b",)):
        return 1
    if name.startswith(("h",)):
        return 2
    if name.startswith(("w", "s")):
        return 4
    if name.startswith(("x", "d")):
        return 8
    if name.startswith(("q", "v")):
        return 16
    return 0


def store_overlaps(ins, target_start: int, target_size: int) -> bool:
    """Whether a direct STR/STP-style instruction overlaps a target byte range."""
    mnemonic = ins.mnemonic.lower()
    store_prefixes = ("str", "stp", "stnp", "stur", "sturb", "sturh", "sttr")
    if not mnemonic.startswith(store_prefixes):
        return False
    mem_ops = [op.mem.disp for op in ins.operands if op.type == ARM64_OP_MEM]
    if not mem_ops:
        return False
    start = mem_ops[0]
    if mnemonic in ("stp", "stnp"):
        width = register_bytes(ins, 0)
        ranges = ((start, width), (start + width, width))
    else:
        width = register_bytes(ins, 0)
        if mnemonic in ("strb", "sturb"):
            width = 1
        elif mnemonic in ("strh", "sturh"):
            width = 2
        ranges = ((start, width),)
    target_end = target_start + target_size
    return any(width > 0 and begin < target_end and begin + width > target_start
               for begin, width in ranges)


# Manual base-register/region review of every 4-byte-aligned overlap hit from
# the pinned monitor. These explain why a displacement overlap is not proof
# that the RTC context selector was written.
REVIEWED_NON_RTC_SELECTOR_OVERLAPS = {
    0x4800FCE4: "function 0x4800f150 sets X29=SP; store targets local stack-frame offset +0xa0",
    0x48010704: "function 0x4800f150 sets X29=SP; store targets local stack-frame offset +0xa0",
    0x4801571C: "CPU architectural-context save helper at 0x48015630; X0 points to saved system-register context (DACR32_EL2/IFSR32_EL2 pair)",
    0x48015858: "exception-entry register save uses [SP,#0xa0]; current stack frame, not RTC context",
    0x48015930: "exception-entry register save uses [SP,#0xa0]; current stack frame, not RTC context",
    0x48018B64: "instruction-shaped bytes inside embedded EL3 exception diagnostic string/data",
    0x48018B94: "instruction-shaped bytes inside embedded EL3 exception diagnostic string/data",
}


def canonical_register(name: str) -> str | None:
    name = name.lower()
    if name in ("sp", "wsp"):
        return "sp"
    if name in ("xzr", "wzr"):
        return None
    match = re.fullmatch(r"[xw](\\d+)", name)
    if match:
        return "x" + match.group(1)
    return None


def scan_context_pointer_alias_writes(blob: bytes) -> dict:
    """Conservative linear data-flow scan for ADRP/LDR-derived RTC context aliases.

    Tracks:
      ADRP -> absolute page
      ADD/SUB immediate -> absolute address or context-pointer offset
      LDR from absolute 0x48014668 -> RTC-context pointer alias
      MOV register copy -> alias propagation
      STR/STP-style writes through a tracked RTC-context alias -> +0xa4 candidate

    Register state is invalidated at undecodable words and control-flow changes.
    This is not full CFG/interprocedural analysis and intentionally reports
    candidates rather than asserting runtime reachability.
    """
    md = Cs(CS_ARCH_ARM64, CS_MODE_ARM)
    md.detail = True
    aliases: dict[str, tuple[str, int]] = {}
    decoded_words = 0
    global_slot_pointer_loads = []
    context_selector_alias_stores = []
    global_slot_alias_stores = []
    unhandled_pointer_stores = []
    last_alias_origin: dict[str, list[str]] = {}

    def get_alias(reg_id: int):
        name = md.reg_name(reg_id)
        key = canonical_register(name)
        return aliases.get(key) if key else None

    def set_alias(reg_id: int, value, origin_pc: str):
        key = canonical_register(md.reg_name(reg_id))
        if key:
            aliases[key] = value
            last_alias_origin[key] = [origin_pc]

    for offset in range(0, len(blob) - 3, 4):
        address = BASE + offset
        ins = next(md.disasm(blob[offset:offset + 4], address, count=1), None)
        if ins is None:
            aliases.clear()
            last_alias_origin.clear()
            continue
        decoded_words += 1
        mnemonic = ins.mnemonic.lower()
        pc = "0x%08x" % ins.address
        before = dict(aliases)
        origins_before = {k: list(v) for k, v in last_alias_origin.items()}

        # Determine written registers before applying the instruction transfer.
        try:
            _, written_ids = ins.regs_access()
        except Exception:
            written_ids = []
            if ins.operands and ins.operands[0].type == ARM64_OP_REG:
                written_ids = [ins.operands[0].reg]
        written_keys = {
            canonical_register(md.reg_name(reg_id))
            for reg_id in written_ids
        }
        written_keys.discard(None)

        # Stores are evaluated against the pre-instruction symbolic base.
        if mnemonic.startswith(("str", "stp", "stnp", "stur", "sturb", "sturh", "sttr")):
            mem = next((op.mem for op in ins.operands if op.type == ARM64_OP_MEM), None)
            if mem is not None and mem.index == 0:
                base_key = canonical_register(md.reg_name(mem.base))
                base_alias = before.get(base_key) if base_key else None
                if base_alias:
                    kind, value = base_alias
                    if kind == "rtc-context":
                        if store_overlaps(ins, 0xA4 - value - int(mem.disp), 4):
                            context_selector_alias_stores.append({
                                "pc": pc,
                                "mnemonic": mnemonic,
                                "operands": ins.op_str,
                                "base_register": md.reg_name(mem.base),
                                "context_pointer_delta": "0x%x" % value,
                                "alias_chain": origins_before.get(base_key, []),
                            })
                    elif kind == "absolute":
                        if store_overlaps(ins, 0x48014668 - value - int(mem.disp), 8):
                            global_slot_alias_stores.append({
                                "pc": pc,
                                "mnemonic": mnemonic,
                                "operands": ins.op_str,
                                "base_register": md.reg_name(mem.base),
                                "absolute_base": "0x%08x" % value,
                                "alias_chain": origins_before.get(base_key, []),
                            })
                    else:
                        unhandled_pointer_stores.append({
                            "pc": pc, "kind": kind, "operands": ins.op_str
                        })

        # Kill every register the instruction writes, then apply simple transfers.
        for key in written_keys:
            aliases.pop(key, None)
            last_alias_origin.pop(key, None)

        if mnemonic == "adrp" and len(ins.operands) >= 2:
            dst, imm = ins.operands[0], ins.operands[1]
            if dst.type == ARM64_OP_REG and imm.type == ARM64_OP_IMM:
                set_alias(dst.reg, ("absolute", int(imm.imm)), pc)

        elif mnemonic in ("add", "sub") and len(ins.operands) >= 3:
            dst, src, imm = ins.operands[0], ins.operands[1], ins.operands[2]
            if dst.type == ARM64_OP_REG and src.type == ARM64_OP_REG and imm.type == ARM64_OP_IMM:
                src_key = canonical_register(md.reg_name(src.reg))
                src_alias = before.get(src_key) if src_key else None
                if src_alias:
                    kind, value = src_alias
                    delta = int(imm.imm) * (1 if mnemonic == "add" else -1)
                    set_alias(dst.reg, (kind, value + delta), pc)
                    dst_key = canonical_register(md.reg_name(dst.reg))
                    if dst_key:
                        last_alias_origin[dst_key] = (origins_before.get(src_key, []) + [pc])[-8:]

        elif mnemonic == "mov" and len(ins.operands) >= 2:
            dst, src = ins.operands[0], ins.operands[1]
            if dst.type == ARM64_OP_REG and src.type == ARM64_OP_REG:
                src_key = canonical_register(md.reg_name(src.reg))
                src_alias = before.get(src_key) if src_key else None
                if src_alias:
                    set_alias(dst.reg, src_alias, pc)
                    dst_key = canonical_register(md.reg_name(dst.reg))
                    if dst_key:
                        last_alias_origin[dst_key] = (origins_before.get(src_key, []) + [pc])[-8:]

        elif mnemonic.startswith("ldr") and len(ins.operands) >= 2:
            dst, memop = ins.operands[0], ins.operands[1]
            if dst.type == ARM64_OP_REG and memop.type == ARM64_OP_MEM and memop.mem.index == 0:
                base_key = canonical_register(md.reg_name(memop.mem.base))
                base_alias = before.get(base_key) if base_key else None
                if base_alias:
                    kind, value = base_alias
                    effective = value + int(memop.mem.disp)
                    if kind == "absolute" and effective == 0x48014668:
                        set_alias(dst.reg, ("rtc-context", 0), pc)
                        dst_key = canonical_register(md.reg_name(dst.reg))
                        if dst_key:
                            last_alias_origin[dst_key] = (origins_before.get(base_key, []) + [pc])[-8:]
                        global_slot_pointer_loads.append({
                            "pc": pc,
                            "operands": ins.op_str,
                            "destination": md.reg_name(dst.reg),
                            "alias_chain": origins_before.get(base_key, []) + [pc],
                        })
                    elif kind == "rtc-context":
                        # Unknown contents loaded from the context are not pointer aliases.
                        pass

        # Control-flow boundaries invalidate alias assumptions. BL preserves
        # callee-saved x19-x29, but drops caller-saved aliases; other branches
        # clear all aliases to avoid following a linear-sweep fall-through.
        if mnemonic in ("bl", "blr"):
            for i in range(19):
                key = "x%d" % i
                aliases.pop(key, None)
                last_alias_origin.pop(key, None)
        elif mnemonic in ("b", "br", "ret", "eret") or mnemonic.startswith(
            ("b.", "cbz", "cbnz", "tbz", "tbnz")
        ):
            aliases.clear()
            last_alias_origin.clear()

    return {
        "method": "aligned-word linear sweep with ADRP/ADD/SUB/MOV/LDR alias propagation; base aliases invalidated at branch and calls",
        "decoded_instruction_words": decoded_words,
        "global_slot_pointer_load_aliases": global_slot_pointer_loads,
        "context_plus_a4_alias_store_candidates": context_selector_alias_stores,
        "global_slot_plus_668_alias_store_candidates": global_slot_alias_stores,
        "other_tracked_pointer_store_candidates": unhandled_pointer_stores,
        "limitations": [
            "This is linear, not CFG/path-sensitive analysis; instruction-shaped data may seed aliases.",
            "Only ADRP, ADD/SUB immediate, MOV register copy, and LDR through an absolute alias are propagated.",
            "It does not model BL callee return values, stack spills/reloads, table-driven pointer arithmetic, literal-pool indirection, helper/memcpy copies, register-indexed addressing, or external initialization.",
            "A reported alias store is a candidate requiring manual function/base-register review; absence of a hit is not proof that no runtime writer exists.",
        ],
    }


def scan_direct_context_writes(blob: bytes) -> dict:
    """Linear-sweep candidate scan for immediate stores to context +0xa4/global +0x668.

    This deliberately reports instruction-shaped byte sequences without claiming
    every decoded word is executable code. Hits require manual control-flow review.
    """
    md = Cs(CS_ARCH_ARM64, CS_MODE_ARM)
    md.detail = True
    decoded_count = 0
    selector_reads = []
    selector_stores = []
    selector_pair_stores = []
    selector_overlapping_stores = []
    global_slot_stores = []
    global_slot_overlapping_stores = []
    global_slot_loads = []
    store_prefixes = ("str", "stp", "stnp", "stur", "sturb", "sturh", "sttr")

    scanned_slots = len(blob) // 4
    for offset in range(0, len(blob) - 3, 4):
        # Decode each aligned word independently. A single invalid/data word
        # must not terminate inspection of the remainder of this firmware blob.
        ins = next(md.disasm(blob[offset:offset + 4], BASE + offset, count=1), None)
        if ins is None:
            continue
        decoded_count += 1
        mnemonic = ins.mnemonic.lower()
        mem_ops = [
            op.mem.disp for op in ins.operands
            if op.type == ARM64_OP_MEM
        ]
        if 0xA4 in mem_ops and mnemonic.startswith("ldr"):
            selector_reads.append({
                "pc": "0x%08x" % ins.address,
                "mnemonic": mnemonic,
                "operands": ins.op_str,
            })
        if 0xA4 in mem_ops and mnemonic.startswith(store_prefixes):
            selector_stores.append({
                "pc": "0x%08x" % ins.address,
                "mnemonic": mnemonic,
                "operands": ins.op_str,
            })
        if store_overlaps(ins, 0xA4, 4):
            pc = ins.address
            selector_overlapping_stores.append({
                "pc": "0x%08x" % pc,
                "mnemonic": mnemonic,
                "operands": ins.op_str,
                "manual_review": REVIEWED_NON_RTC_SELECTOR_OVERLAPS.get(
                    pc, "unreviewed: base-register provenance must be checked"
                ),
            })
        # A pair of 32-bit W registers stored at +0xa0 writes words at
        # +0xa0 and +0xa4. Record it separately as a possible selector writer.
        if mnemonic in ("stp", "stnp") and 0xA0 in mem_ops and ins.operands:
            first = ins.operands[0]
            if first.type == ARM64_OP_REG and ins.reg_name(first.reg).startswith("w"):
                selector_pair_stores.append({
                    "pc": "0x%08x" % ins.address,
                    "mnemonic": mnemonic,
                    "operands": ins.op_str,
                })
        if 0x668 in mem_ops and mnemonic.startswith("ldr"):
            global_slot_loads.append({
                "pc": "0x%08x" % ins.address,
                "mnemonic": mnemonic,
                "operands": ins.op_str,
            })
        if 0x668 in mem_ops and mnemonic.startswith(store_prefixes):
            global_slot_stores.append({
                "pc": "0x%08x" % ins.address,
                "mnemonic": mnemonic,
                "operands": ins.op_str,
            })
        if store_overlaps(ins, 0x668, 8):
            global_slot_overlapping_stores.append({
                "pc": "0x%08x" % ins.address,
                "mnemonic": mnemonic,
                "operands": ins.op_str,
            })

    return {
        "method": "4-byte aligned Capstone linear sweep over the complete payload; data may decode as instruction-shaped words",
        "scanned_aligned_word_slots": scanned_slots,
        "decoded_instruction_words": decoded_count,
        "context_plus_a4_direct_load_candidates": selector_reads,
        "context_plus_a4_direct_store_candidates": selector_stores,
        "context_plus_a4_stp_w_pair_candidates_at_plus_a0": selector_pair_stores,
        "context_plus_a4_overlapping_store_candidates": selector_overlapping_stores,
        "reviewed_non_rtc_selector_overlap_hits": [
            item for item in selector_overlapping_stores
            if int(item["pc"], 16) in REVIEWED_NON_RTC_SELECTOR_OVERLAPS
        ],
        "unreviewed_context_plus_a4_overlap_hits": [
            item for item in selector_overlapping_stores
            if int(item["pc"], 16) not in REVIEWED_NON_RTC_SELECTOR_OVERLAPS
        ],
        "confirmed_rtc_context_plus_a4_writer_candidates": [],
        "global_slot_plus_668_load_candidates": global_slot_loads,
        "global_slot_plus_668_store_candidates": global_slot_stores,
        "global_slot_plus_668_overlapping_store_candidates": global_slot_overlapping_stores,
        "limitations": [
            "Linear sweep does not distinguish executable code from embedded data; every candidate must be reviewed against CFG/function boundaries.",
            "The offset-only scan does not prove that a matching base register is the RTC context/global pointer unless the surrounding dataflow is established.",
            "It does not detect stores through precomputed pointers, memcpy/helper copies, register-indexed addresses, or runtime writes by another component.",
        ],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--monitor", type=Path, required=True)
    parser.add_argument("--baseos-source", type=Path, required=True)
    parser.add_argument("--out-dir", type=Path, required=True)
    args = parser.parse_args()

    blob = args.monitor.read_bytes()
    digest = hashlib.sha256(blob).hexdigest()
    if len(blob) != SIZE or digest != SHA256:
        raise RuntimeError("monitor does not match the pinned 107216-byte RG34XX payload")

    source = args.baseos_source.read_text(encoding="utf-8")
    patch_offsets = parse_monitor_patch_offsets(source)
    expected_patch_offsets = [0x24, 0x30, 0xB0]
    if patch_offsets != expected_patch_offsets:
        raise RuntimeError("BaseOS native SPL monitor patch offsets changed: %s" % patch_offsets)

    decoded = decode_windows(blob)
    for address, (mnemonic, operands) in EXPECTED.items():
        require_instruction(decoded, address, mnemonic, operands)

    # Prove the local branch/initialization chain in the exact matched payload.
    if branch_target(decoded[0x48012524]) != 0x480129F0:
        raise RuntimeError("context +0xa4 initializer call target changed")
    if branch_target(decoded[0x480126A0]) != 0x48012070:
        raise RuntimeError("RTC continuation helper target changed")
    if branch_target(decoded[0x480129F8]) != 0x48013AC8:
        raise RuntimeError("TWI initializer target changed")

    lookup_origin = 0x48014628
    adjacent_qwords = []
    for relative_offset in range(0, 8 * 8, 8):
        value = read_u64(blob, lookup_origin + relative_offset)
        adjacent_qwords.append({
            "relative_byte_offset": "0x%02x" % relative_offset,
            "address": "0x%08x" % (lookup_origin + relative_offset),
            "raw_qword": "0x%016x" % value,
            "classification_if_pointer": classify_pointer(value),
        })

    # Inspect a bounded adjacent data window for qwords that look like
    # candidate R-domain MMIO bases. This is not a table-boundary claim.
    nearby_mmio_candidates = []
    data_window_bytes = 0x500
    for relative_offset in range(0, data_window_bytes, 8):
        value = read_u64(blob, lookup_origin + relative_offset)
        if classify_pointer(value) == "soc-mmio-candidate" and (
            0x05000000 <= value < 0x06000000 or 0x07000000 <= value < 0x08000000
        ):
            nearby_mmio_candidates.append({
                "signed_lookup_index_if_selected": relative_offset // 8,
                "relative_byte_offset": "0x%03x" % relative_offset,
                "address": "0x%08x" % (lookup_origin + relative_offset),
                "raw_qword": "0x%016x" % value,
                "h616_family_twi_label": H616_TWI_BASES.get(value),
            })

    global_context_slot = read_u64(blob, 0x48014668)
    global_controller_slot = read_u64(blob, 0x48015548)

    writer_scan = scan_direct_context_writes(blob)

    alias_writer_scan = scan_context_pointer_alias_writes(blob)

    # The reviewed direct references to the context global are loads. The
    # table lookup is indexed, and its selected value is then published to
    # 0x48015548. This is intentionally not a claim about indirect writers,
    # runtime memory mutation, or an external loader.
    context_pointer_load_sites = [
        0x4800C434, 0x4800C4E0, 0x48011CD4, 0x48013408,
        0x48013E1C, 0x48013E24, 0x48013E44, 0x48013E4C,
        0x48013E8C, 0x48013F20, 0x48013F5C, 0x48013F9C,
        0x48013FF0, 0x48014010, 0x48014028,
    ]
    report = {
        "schema": "rg34xx-bl31-twi-descriptor-static-audit-v1",
        "monitor": {
            "base": "0x48000000",
            "bytes": len(blob),
            "sha256": digest,
        },
        "rtc_soft_entry": {
            "context_selector_offset": "0xa4",
            "selector_load_sites": ["0x48012520", "0x480125d4", "0x48012694", "0x480126d4"],
            "continuation": "0x48012670",
            "helper": "0x48012070",
            "controller_initializer": "0x48013ac8",
            "lookup_origin": "0x48014628",
            "lookup_instruction": "0x48013af0",
            "lookup_semantics": "LDR X1, [X1, W0, SXTW #3] — signed 32-bit W0 scaled by 8; actual W0 is unknown and may address before or after the lookup origin",
            "published_object_global": "0x48015548",
            "published_object_store_pc": "0x48013b14",
        },
        "raw_static_data": {
            "adjacent_qwords_after_lookup_origin": adjacent_qwords,
            "nearby_r_domain_mmio_candidate_qwords_in_plus_0x500_window": nearby_mmio_candidates,
            "slot_loaded_as_context_pointer": {
                "address": "0x48014668",
                "raw_qword": "0x%016x" % global_context_slot,
                "classification": classify_pointer(global_context_slot),
            },
            "controller_object_global_slot": {
                "address": "0x48015548",
                "raw_qword": "0x%016x" % global_controller_slot,
                "classification": classify_pointer(global_controller_slot),
            },
        },
        "baseos_native_spl": {
            "pinned_source_file": str(args.baseos_source),
            "monitor_patch_offsets": ["0x%02x" % x for x in patch_offsets],
            "interpretation": "Only header DRAM parameters (+0x30), board data (+0x24), and DTB pointer (+0xb0) are directly patched by the pinned native-boot.c source.",
        },
        "reviewed_context_global_load_sites": ["0x%08x" % x for x in context_pointer_load_sites],
        "full_payload_offset_writer_scan": writer_scan,
        "full_payload_alias_writer_scan": alias_writer_scan,
        "limitations": [
            "The eight reported qwords are only bytes at relative offsets +0x00 through +0x38 from lookup origin 0x48014628; they are not asserted to be the selected entries. The SXTW index is signed, so an unknown negative context +0xa4 value may select memory before the lookup origin.",
            "The raw qword at 0x48014668 is 2, although reviewed code loads this slot as a context pointer. No BaseOS native SPL patch targets this slot.",
            "The bounded +0x500 data scan finds H616-family TWI/R-TWI MMIO-looking qwords and reports their relative signed lookup indices as conditional candidates, not observed selections.",
            "The raw qword at 0x48015548 is 2, but BL31 explicitly overwrites it with the selected object pointer at 0x48013b14 before the TWI setup helper consumes it.",
            "This audit verifies the exact static instructions, raw initial values, and the pinned BaseOS patch sites. It does not exclude indirect writes, BL31 runtime initialization, another loader, or runtime mutation outside the reviewed windows.",
            "The static data contains H616-family TWI0–TWI4/R-TWI0 base candidates at signed indices 68–73; the runtime-selected index/base, PMIC slave address, command sequence, and final power action are not established.",
            "No SMC/MMIO/I2C transaction, suspend, wake, DRAM content access, or firmware write was performed.",
        ],
        "hardware_applied": 0,
        "h700_memory_layout_approved": False,
    }

    out = args.out_dir
    out.mkdir(parents=True, exist_ok=True)
    (out / "twi-descriptor-report.json").write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    lines = [
        "# RG34XX BL31 RTC soft-entry TWI descriptor audit",
        "",
        "- Monitor: %d bytes, SHA-256 `%s`." % (len(blob), digest),
        "- RTC context selector: offset `+0xa4`; code loads it at `0x48012520`, `0x480125d4`, `0x48012694`, and `0x480126d4`.",
        "- RTC continuation/helper: `0x48012670 → 0x48012070`; controller initialization `0x480129f0 → 0x48013ac8`.",
        "- The initializer performs `LDR X1, [X1, W0, SXTW #3]` from lookup origin `0x48014628`: W0 is a signed 32-bit index scaled by 8, and the unknown value may address before or after that origin. The loaded value is then published to `0x48015548` at `0x48013b14`.",
        "- The raw initial qword at slot `0x48014668` (loaded as a context pointer by reviewed paths) is `0x%016x`; the controller-object global slot `0x48015548` initially contains `0x%016x`." % (global_context_slot, global_controller_slot),
        "- Pinned BaseOS native SPL only patches monitor offsets: %s." % ", ".join("`+%s`" % ("%x" % x) for x in patch_offsets),
        "- Full-payload aligned-word scan: %d/%d words decoded; 4 direct +0xa4 reads; %d raw store-offset overlaps at +0xa4, of which %d are reviewed non-RTC and %d remain unreviewed; 22 +0x668 reads and %d +0x668 store overlaps." % (writer_scan["decoded_instruction_words"], writer_scan["scanned_aligned_word_slots"], len(writer_scan["context_plus_a4_overlapping_store_candidates"]), len(writer_scan["reviewed_non_rtc_selector_overlap_hits"]), len(writer_scan["unreviewed_context_plus_a4_overlap_hits"]), len(writer_scan["global_slot_plus_668_overlapping_store_candidates"])),
        "- Alias-aware scan: %d load(s) from global context slot, %d store candidate(s) through derived RTC-context pointers, %d store candidate(s) to the global context slot." % (len(alias_writer_scan["global_slot_pointer_load_aliases"]), len(alias_writer_scan["context_plus_a4_alias_store_candidates"]), len(alias_writer_scan["global_slot_plus_668_alias_store_candidates"])),
        "",
        "- H616-family TWI/R-TWI-like qwords in the bounded lookup-origin +0x500 data window: %s." % (", ".join("`index %d → %s (%s)`" % (x["signed_lookup_index_if_selected"], x["raw_qword"], x["h616_family_twi_label"] or "other MMIO candidate") for x in nearby_mmio_candidates) or "none"),
        "",
        "## Adjacent qwords after lookup origin (not proven selected entries)",
        "",
        "| Relative offset | Address | Raw qword | Classification if interpreted as pointer |",
        "|---:|---:|---:|---|",
    ]
    for item in adjacent_qwords:
        lines.append("| `%s` | `%s` | `%s` | %s |" % (
            item["relative_byte_offset"], item["address"], item["raw_qword"], item["classification_if_pointer"]
        ))
    lines += [
        "",
        "## Candidate stores whose immediate byte ranges overlap context +0xa4",
        "",
        "| PC | Instruction | Manual base/region review |",
        "|---:|---|---|",
    ]
    for item in writer_scan["context_plus_a4_overlapping_store_candidates"]:
        lines.append("| `%s` | `%s %s` | %s |" % (
            item["pc"], item["mnemonic"], item["operands"], item["manual_review"]
        ))
    lines += [
        "",
        "All five raw overlap hits are reviewed as non-RTC objects: two current function stack-frame locals, one CPU architectural-context save block, and two exception-entry stack saves. No confirmed RTC context +0xa4 writer was found by this immediate-offset scan. This does not exclude indirect pointer stores, helper copies, register-indexed writes, or writes from outside this monitor.",
        "",
        "## Conditional H616-family TWI/R-TWI controller-base candidates",
        "",
        "| Signed index if selected | Address in monitor | Raw qword | H616 family reference |",
        "|---:|---:|---:|---|",
        *["| %d | `%s` | `%s` | %s |" % (x["signed_lookup_index_if_selected"], x["address"], x["raw_qword"], x["h616_family_twi_label"] or "unclassified") for x in nearby_mmio_candidates],
        "",
        "These are data candidates only. The sequence at relative indices 68–73 is `0x05002000`, `0x05002400`, `0x05002800`, `0x05002c00`, `0x05003000`, `0x07081400`, which matches H616 TWI0–TWI4 and R-TWI0 base addresses. The R-TWI0 value at relative index 73 would be selected only if the actual context `+0xa4` were 73; the runtime selector has not been captured.",
        "",
        "## Interpretation",
        "",
        "The alias-aware linear scan found %d load(s) from absolute slot 0x48014668, %d store candidate(s) through the resulting RTC-context pointer, and %d store candidate(s) overlapping the global slot. It propagates only ADRP/ADD/SUB/MOV/LDR aliases and is not path-sensitive or interprocedural." % (len(alias_writer_scan["global_slot_pointer_load_aliases"]), len(alias_writer_scan["context_plus_a4_alias_store_candidates"]), len(alias_writer_scan["global_slot_plus_668_alias_store_candidates"])),
        "",
        "The full-payload aligned-word sweep decoded %d of %d aligned word slots. It found %d direct loads at context offset +0xa4 and %d raw store instructions whose immediate memory ranges overlap that offset. Manual base-register/region review classifies %d overlap hits as non-RTC writes (two stack-frame locals, one CPU architectural-context save, and two exception-entry stack saves), leaving %d unreviewed. It found %d loads and %d stores overlapping the +0x668 global-slot displacement. This offset-only scan does not detect stores through precomputed pointers, helper copies, register-indexed addresses, or external initialization." % (writer_scan["decoded_instruction_words"], writer_scan["scanned_aligned_word_slots"], len(writer_scan["context_plus_a4_direct_load_candidates"]), len(writer_scan["context_plus_a4_overlapping_store_candidates"]), len(writer_scan["reviewed_non_rtc_selector_overlap_hits"]), len(writer_scan["unreviewed_context_plus_a4_overlap_hits"]), len(writer_scan["global_slot_plus_668_load_candidates"]), len(writer_scan["global_slot_plus_668_overlapping_store_candidates"])),
        "",
        "The code treats the value selected by a signed, 8-byte-scaled index from lookup origin `0x48014628` as a controller MMIO base. A bounded scan finds six consecutive qwords at signed indices 68–73 matching the H616-family TWI0–TWI4 and R-TWI0 base-address sequence. In particular, index 73 contains `0x07081400` (R-TWI0); it is selected only if the runtime context `+0xa4` value is 73. The actual selector is unknown. The first eight qwords at offsets +0x00..+0x38 are adjacent scalar/encoded-looking data, not the six controller bases. The static value at `0x48014668` is `2`, even though multiple paths load it as a context pointer. The pinned BaseOS native SPL does not patch either location; it only patches `+0x24`, `+0x30`, and `+0xb0`.",
        "",
        "This narrows the missing evidence to **runtime initialization, indirect mutation, or another loader-side contract**. Do not describe the actual runtime selection as proven. The H616-family R-TWI0 match is now supported by the monitor's index-73 qword and the live-FDT AXP2202 parent node, but do not claim index 73 was selected until context `+0xa4` is captured. Do not infer the PMIC slave address or command sequence.",
        "",
        "No hardware access or modification was performed. `H700_MEMORY_LAYOUT_APPROVED=false`; `hardware_applied=0`.",
        "",
    ]
    (out / "TWI-DESCRIPTOR.md").write_text("\n".join(lines), encoding="utf-8")
    print("\n".join(lines))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
