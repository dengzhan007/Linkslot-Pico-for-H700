#!/usr/bin/env python3
"""Read-only static triage of the exact RG34XX BL31 monitor payload.

Verifies the monitor against both uploaded manifests, disassembles AArch64
instructions from the entry branch, and records recoverable DRAM references.
This is not a proof that memory is free.
"""
from __future__ import annotations
import argparse
import hashlib
import json
import struct
from pathlib import Path
from capstone import CS_ARCH_ARM64, CS_MODE_ARM, Cs
from capstone.arm64_const import ARM64_OP_IMM, ARM64_OP_MEM, ARM64_OP_REG

IMAGE_BASE = 0x48000000
EXPECTED_MONITOR_BYTES = 107216
EXPECTED_MONITOR_SHA256 = "676bc0206e3c9a7f229d93ea7f3286095f669940ad91f3c4f3d37bdb18696c31"
RANGES = (
    (0x40080000, 0x42000000, "Linux kernel destination / proposed app candidate"),
    (0x42000000, 0x42001000, "Linux initrd reservation"),
    (0x42E00000, 0x42E46200, "native SPL firmware package buffer"),
    (0x44000000, 0x44100000, "Linux FDT buffer"),
    (0x45000000, 0x46000000, "BOL4 staging buffer"),
    (0x48000000, 0x4801A2D0, "BL31 loaded monitor payload bytes"),
    (0x4801A2D0, 0x48041000, "BL31 adjacent BSS/runtime/page-table bytes in unconditional map"),
    (0x48041000, 0x48041001, "BL31 unconditional-map end pointer (not mapped data)"),
    (0x48041001, 0x48200000, "adjacent DRAM gap with no recovered direct reference"),
    (0x48200000, 0x4A000000, "conditional BL31 identity-map range (inactive when header flag is zero)"),
    (0x4A000000, 0x4A100000, "BL33 loader slot"),
    (0x77C00000, 0x7BC00000, "CMA reservation"),
    (0x7BF24800, 0x7C076000, "framebuffer reservation"),
    (0x7EC00000, 0x7FC00000, "unknown top-of-DRAM reservation"),
    (0x40000000, 0x80000000, "installed 1 GiB DRAM (other)"),
)

def classify_address(address: int) -> str | None:
    for start, end, label in RANGES:
        if start <= address < end:
            return label
    return None

def decode_a64_b(raw: bytes, offset: int, image_base: int = IMAGE_BASE) -> int:
    if offset < 0 or offset + 4 > len(raw):
        raise ValueError("AArch64 branch instruction is outside the monitor image")
    word = struct.unpack_from("<I", raw, offset)[0]
    if (word & 0x7C000000) != 0x14000000 or (word & 0x80000000):
        raise ValueError("monitor image does not begin with an AArch64 B instruction")
    imm26 = word & 0x03FFFFFF
    if imm26 & (1 << 25):
        imm26 -= 1 << 26
    return image_base + offset + (imm26 << 2)

def verify_monitor_identity(reports_root: Path, monitor_path: Path) -> dict:
    fw = json.loads((reports_root / "rg34xx-native-firmware/firmware-manifest.json").read_text(encoding="utf-8"))
    toc = json.loads((reports_root / "rg34xx-inventory-20261126-timer/toc1-extracted/manifest.json").read_text(encoding="utf-8"))
    fw_record = fw["extents"]["monitor"]
    toc_record = next(item for item in toc["items"] if item["name"] == "monitor")
    raw = monitor_path.read_bytes()
    digest = hashlib.sha256(raw).hexdigest()
    for name, record in (("native firmware manifest", fw_record), ("TOC1 manifest", toc_record)):
        size = record.get("payload_bytes", record.get("size"))
        if size != EXPECTED_MONITOR_BYTES or record.get("sha256") != EXPECTED_MONITOR_SHA256:
            raise RuntimeError(name + " no longer matches the pinned monitor identity")
    if len(raw) != EXPECTED_MONITOR_BYTES or digest != EXPECTED_MONITOR_SHA256:
        raise RuntimeError("monitor.bin does not match the matched firmware/TOC1 manifests")
    return {
        "size_bytes": len(raw),
        "sha256": digest,
        "firmware_fnv1a32": fw.get("firmware_fnv1a32"),
        "firmware_package_sha256": fw.get("firmware_sha256"),
        "toc1_package_sha256": toc.get("package_sha256"),
        "matched_firmware_and_toc1_monitor": True,
        "conditional_mapping_flag_file_offset": "0x18",
        "conditional_mapping_flag_runtime_address": "0x48000018",
        "conditional_mapping_flag_value": "0x%08x" % struct.unpack_from("<I", raw, 0x18)[0],
        "conditional_mapping_enabled": bool(struct.unpack_from("<I", raw, 0x18)[0]),
    }

def canonical_register(name: str) -> str:
    if name.startswith("w") and name[1:].isdigit():
        return "x" + name[1:]
    return name

def shift_value(operand) -> int:
    try:
        return int(operand.shift.value)
    except (AttributeError, TypeError, ValueError):
        return 0

def scan_addresses(raw: bytes, entry: int) -> tuple[list[dict], str, int]:
    """Disassemble only control-flow-reachable instructions from the BL31 entry.

    Direct calls and conditional branches are followed with a work queue.
    Indirect targets are not guessed; register constants are reported only
    when consumed by a statically resolved load/store base.
    """
    engine = Cs(CS_ARCH_ARM64, CS_MODE_ARM)
    engine.detail = True
    refs: list[dict] = []
    seen_refs: set[tuple[int, str, int]] = set()
    visited: set[int] = set()
    queued = [entry]
    decoded_lines: dict[int, str] = {}

    def record(pc: int, kind: str, address: int, expression: str) -> None:
        label = classify_address(address)
        key = (pc, kind, address)
        if label is None or key in seen_refs:
            return
        seen_refs.add(key)
        refs.append({
            "pc": "0x%08x" % pc,
            "kind": kind,
            "address": "0x%08x" % address,
            "range": label,
            "expression": expression,
        })

    while queued:
        pc = queued.pop()
        regs: dict[str, int] = {}
        while IMAGE_BASE <= pc < IMAGE_BASE + len(raw) and pc not in visited:
            offset = pc - IMAGE_BASE
            decoded = next(engine.disasm(raw[offset:offset + 4], pc, count=1), None)
            if decoded is None or decoded.size != 4:
                break
            insn = decoded
            visited.add(pc)
            byte_text = " ".join("%02x" % byte for byte in insn.bytes)
            decoded_lines[pc] = "%08x: %-11s %s %s" % (
                insn.address, byte_text, insn.mnemonic, insn.op_str
            )
            ops = insn.operands if insn.id else []

            if insn.mnemonic in ("adr", "adrp") and len(ops) >= 2:
                if ops[0].type == ARM64_OP_REG and ops[1].type == ARM64_OP_IMM:
                    dest = canonical_register(insn.reg_name(ops[0].reg))
                    address = int(ops[1].imm)
                    if insn.mnemonic == "adrp":
                        address &= ~0xFFF
                    regs[dest] = address
                    record(insn.address, insn.mnemonic, address, insn.op_str)

            elif insn.mnemonic in ("movz", "movk", "movn") and len(ops) >= 2:
                if ops[0].type == ARM64_OP_REG and ops[1].type == ARM64_OP_IMM:
                    dest_name = insn.reg_name(ops[0].reg)
                    dest = canonical_register(dest_name)
                    immediate = int(ops[1].imm)
                    shift = shift_value(ops[1])
                    mask = (1 << (32 if dest_name.startswith("w") else 64)) - 1
                    chunk_mask = 0xFFFF << shift
                    if insn.mnemonic == "movz":
                        value = (immediate << shift) & mask
                    elif insn.mnemonic == "movn":
                        value = (~(immediate << shift)) & mask
                    else:
                        value = (regs.get(dest, 0) & ~chunk_mask) | ((immediate << shift) & chunk_mask)
                    regs[dest] = value & mask

            elif insn.mnemonic == "mov" and len(ops) >= 2:
                if ops[0].type == ARM64_OP_REG:
                    dest_name = insn.reg_name(ops[0].reg)
                    dest = canonical_register(dest_name)
                    if ops[1].type == ARM64_OP_REG:
                        source = canonical_register(insn.reg_name(ops[1].reg))
                        if source in regs:
                            regs[dest] = regs[source]
                    elif ops[1].type == ARM64_OP_IMM:
                        value = int(ops[1].imm)
                        if dest_name.startswith("w"):
                            value &= 0xFFFFFFFF
                        regs[dest] = value

            elif insn.mnemonic in ("add", "sub") and len(ops) >= 3:
                if ops[0].type == ARM64_OP_REG and ops[1].type == ARM64_OP_REG and ops[2].type == ARM64_OP_IMM:
                    dest_name = insn.reg_name(ops[0].reg)
                    dest = canonical_register(dest_name)
                    source = canonical_register(insn.reg_name(ops[1].reg))
                    if source in regs:
                        amount = int(ops[2].imm) << shift_value(ops[2])
                        value = regs[source] + amount if insn.mnemonic == "add" else regs[source] - amount
                        if dest_name.startswith("w"):
                            value &= 0xFFFFFFFF
                        regs[dest] = value & 0xFFFFFFFFFFFFFFFF
                        record(insn.address, "derived-register-address", regs[dest], insn.op_str)
                elif ops[0].type == ARM64_OP_REG and ops[1].type == ARM64_OP_REG and ops[2].type == ARM64_OP_REG:
                    source = canonical_register(insn.reg_name(ops[1].reg))
                    if source in regs:
                        record(insn.address, "indexed-address-base", regs[source], insn.op_str)

            if insn.mnemonic == "msr" and "ttbr0_el3" in insn.op_str.lower() and len(ops) >= 2 and ops[-1].type == ARM64_OP_REG:
                source = canonical_register(insn.reg_name(ops[-1].reg))
                if source in regs:
                    record(insn.address, "system-register-value", regs[source], insn.op_str)

            for op in ops:
                if op.type == ARM64_OP_IMM and insn.mnemonic in ("ldr", "ldrsw", "prfm"):
                    record(insn.address, "literal-reference", int(op.imm), insn.op_str)
                if op.type == ARM64_OP_MEM:
                    base = canonical_register(insn.reg_name(op.mem.base)) if op.mem.base else ""
                    index = canonical_register(insn.reg_name(op.mem.index)) if op.mem.index else ""
                    if base in regs and not index:
                        effective = regs[base] + int(op.mem.disp)
                        record(insn.address, "memory-effective-address", effective, insn.op_str)

            mnemonic = insn.mnemonic
            immediate_operands = [int(op.imm) for op in ops if op.type == ARM64_OP_IMM]
            if mnemonic in ("b", "bl", "b.eq", "b.ne", "b.cs", "b.hs", "b.cc", "b.lo",
                            "b.mi", "b.pl", "b.vs", "b.vc", "b.hi", "b.ls", "b.ge",
                            "b.lt", "b.gt", "b.le", "b.al") or mnemonic.startswith("b."):
                if immediate_operands:
                    target = immediate_operands[-1]
                    if IMAGE_BASE <= target < IMAGE_BASE + len(raw):
                        queued.append(target)
                if mnemonic == "b":
                    break
                regs.clear()
            elif mnemonic in ("cbz", "cbnz", "tbz", "tbnz"):
                if immediate_operands:
                    target = immediate_operands[-1]
                    if IMAGE_BASE <= target < IMAGE_BASE + len(raw):
                        queued.append(target)
            elif mnemonic in ("ret", "br", "eret", "drps"):
                break
            elif mnemonic == "blr":
                regs.clear()

            pc += insn.size

    lines = "\n".join(decoded_lines[address] for address in sorted(decoded_lines))
    return refs, lines + "\n", len(visited)

BL31_TRANSLATION_MAP_REQUESTS = [
    {"callsite": "0x480033a0", "helper": "0x4800bcb4", "arg0_base": "0x48000000",
     "arg1_base": "0x48000000", "arg2_length": "0x41000", "arg3_raw_attribute": "3",
     "condition": "unconditional"},
    {"callsite": "0x480033b4", "helper": "0x4800bcb4", "arg0_base": "0x48000000",
     "arg1_base": "0x48000000", "arg2_length": "0x1a000", "arg3_raw_attribute": "1",
     "condition": "unconditional"},
    {"callsite": "0x480033c8", "helper": "0x4800bcb4", "arg0_base": "0x48040000",
     "arg1_base": "0x48040000", "arg2_length": "0x1000", "arg3_raw_attribute": "2",
     "condition": "unconditional"},
    {"callsite": "0x480033f4", "helper": "0x4800bcb4", "arg0_base": "0x48200000",
     "arg1_base": "0x48200000", "arg2_length": "0x1e00000", "arg3_raw_attribute": "3",
     "condition": "word_at_0x48000018_nonzero"},
    {"callsite": "0x48003408", "helper": "0x4800bcb4", "arg0_base": "0x48200000",
     "arg1_base": "0x48200000", "arg2_length": "0x400000", "arg3_raw_attribute": "7",
     "condition": "word_at_0x48000018_nonzero"},
]
BL31_PAGE_TABLE_ALLOCATOR = {
    "pool_base": "0x4803e000",
    "page_size": "0x1000",
    "counter_address": "0x4803cb80",
    "counter_guard": "incremented_counter_le_2",
    "allocator_path_start": "0x4800bb38",
    "page_index_shift_instruction": "0x4800bc34",
    "page_pointer_add_instruction": "0x4800bc40",
}

def build_report(identity: dict, entry: int, refs: list[dict], count: int) -> tuple[dict, str]:
    counts: dict[str, int] = {}
    for item in refs:
        counts[item["range"]] = counts.get(item["range"], 0) + 1

    def find_ref(kind: str, address: str) -> dict | None:
        return next((item for item in refs if item["kind"] == kind and item["address"] == address), None)

    ttbr = find_ref("system-register-value", "0x4803cb40")
    tcr_source = find_ref("memory-effective-address", "0x4803c8e0")
    per_core = find_ref("indexed-address-base", "0x4803cd40")
    active_mapped_refs = counts.get("BL31 adjacent BSS/runtime/page-table bytes in unconditional map", 0)
    map_end_pointer_refs = counts.get("BL31 unconditional-map end pointer (not mapped data)", 0)
    outside_payload = active_mapped_refs + map_end_pointer_refs
    inside_payload = counts.get("BL31 loaded monitor payload bytes", 0)
    conditional_map_enabled = bool(identity["conditional_mapping_enabled"])
    observed = {
        "ttbr0_el3": None if ttbr is None else {
            "value": ttbr["address"],
            "write_instruction_pc": ttbr["pc"],
            "decoded_instruction": ttbr["expression"],
        },
        "tcr_el3_configuration_source": None if tcr_source is None else {
            "address": tcr_source["address"],
            "load_instruction_pc": tcr_source["pc"],
            "decoded_instruction": tcr_source["expression"],
        },
        "tpidr_el3_per_core_context_base": None if per_core is None else {
            "address": per_core["address"],
            "instruction_pc": per_core["pc"],
            "decoded_instruction": per_core["expression"],
        },
    }
    report = {
        "schema": "rg34xx-bl31-static-memory-triage-v1",
        "monitor": identity,
        "image_base": "0x%08x" % IMAGE_BASE,
        "payload_end": "0x%08x" % (IMAGE_BASE + identity["size_bytes"]),
        "entry": "0x%08x" % entry,
        "reachable_decoded_instruction_count": count,
        "recoverable_address_reference_count": len(refs),
        "reference_counts_by_range": counts,
        "loaded_payload_reference_count": inside_payload,
        "adjacent_runtime_window_reference_count": outside_payload,
        "active_unconditional_map_reference_count": active_mapped_refs,
        "unconditional_map_end_pointer_reference_count": map_end_pointer_refs,
        "unconditional_bl31_map_range": {
            "start": "0x48000000", "end_exclusive": "0x48041000", "bytes": "0x41000"
        },
        "conditional_bl31_map": {
            "flag_file_offset": identity["conditional_mapping_flag_file_offset"],
            "flag_runtime_address": identity["conditional_mapping_flag_runtime_address"],
            "flag_value": identity["conditional_mapping_flag_value"],
            "enabled": conditional_map_enabled,
            "conditional_range": "[0x48200000,0x4a000000)",
            "branch_pc": "0x480033e0",
            "skip_target": "0x4800340c"
        },
        "observed_el3_memory_setup": observed,
        "observed_translation_map_requests": BL31_TRANSLATION_MAP_REQUESTS,
        "observed_page_table_allocator": BL31_PAGE_TABLE_ALLOCATOR,
        "references": refs,
        "limits": [
            "Only instructions reachable via direct control-flow edges are decoded; indirect branches/calls and exception-return paths may be incomplete.",
            "Register tracking is block-local and conservative; computed and indirect pointers may be missed.",
            "Recovered addresses are static leads; register aliasing/clobbers and indirect dataflow may still create false positives or miss references.",
            "No references found in a range would not prove that BL31 never accesses it.",
            "This does not prove Secure/Non-secure ownership, DMA reachability, cache aliases, runtime scratch, or per-boot behavior.",
            "The adjacent runtime window label describes statically referenced addresses outside the file payload, not a complete allocation map.",
            "The 0x48000018 word is read from the exact matched payload; runtime mutation before BL31 consumes it is checked separately against the pinned BaseOS SPL source.",
            "Static mapping requests and source-level mutation checks do not prove every possible Boot0/ROM or indirect runtime memory access.",
            "No hardware registers were read or written; no image was flashed or executed.",
        ],
    }
    lines = [
        "# RG34XX BL31 static memory triage", "",
        "- Monitor payload: %d bytes" % identity["size_bytes"],
        "- SHA-256: %s" % identity["sha256"],
        "- Matched native firmware / TOC1 component: yes",
        "- BL31 image base: 0x%08x" % IMAGE_BASE,
        "- Payload end (exclusive): 0x%08x" % (IMAGE_BASE + identity["size_bytes"]),
        "- First-entry branch target: 0x%08x" % entry,
        "- Reachable decoded instructions: %d" % count,
        "- Recoverable address references: %d" % len(refs),
        "- References into actual payload bytes: %d" % inside_payload,
        "- References outside payload but inside the unconditional BL31 map: %d" % active_mapped_refs,
        "- References to the one-past-end map boundary (not dereferenced data): %d" % map_end_pointer_refs,
        "",
        "## References grouped by mapped range", "",
    ]
    if counts:
        for name in sorted(counts):
            lines.append("- %s: %d" % (name, counts[name]))
    else:
        lines.append("- No references into audited DRAM ranges were recovered by this limited scan.")

    lines += ["", "## Decoded EL3 translation-map requests", ""]
    lines.append("- The caller passes the following base/length/raw-attribute arguments to helper 0x4800bcb4:")
    for request in BL31_TRANSLATION_MAP_REQUESTS:
        lines.append("- Callsite %s: base args %s / %s, length %s, raw attribute %s; condition: %s." % (
            request["callsite"], request["arg0_base"], request["arg1_base"],
            request["arg2_length"], request["arg3_raw_attribute"], request["condition"]
        ))
    lines.append("- Matched payload word at file offset 0x18 / runtime address 0x48000018: %s." % identity["conditional_mapping_flag_value"])
    if conditional_map_enabled:
        lines.append("- The matched word is nonzero, so BL31 takes the conditional path and issues the 30 MiB and nested 4 MiB mapping requests.")
    else:
        lines.append("- The matched word is zero, so the CBZ at 0x480033e0 branches to 0x4800340c; the 30 MiB and nested 4 MiB calls at 0x480033f4 and 0x48003408 are skipped for this exact payload.")
    lines.append("- The pinned BaseOS SPL source is checked separately to confirm its explicit monitor patches do not overwrite offset 0x18 before BL31 handoff.")
    lines.append("- The page-table construction path uses pool base %s, 4 KiB page indexing, and a cursor at %s; its local counter check permits at most two page indices on this path." % (
        BL31_PAGE_TABLE_ALLOCATOR["pool_base"], BL31_PAGE_TABLE_ALLOCATOR["counter_address"]
    ))
    lines.append("- Raw mapping attributes are recorded without assigning device/normal-memory or security semantics.")

    lines += ["", "## EL3 memory-setup references", ""]
    if ttbr:
        lines.append("- TTBR0_EL3 is written with %s at %s (%s)." % (
            ttbr["address"], ttbr["pc"], ttbr["expression"]
        ))
    else:
        lines.append("- No statically tracked TTBR0_EL3 value was recovered.")
    if tcr_source:
        lines.append("- TCR_EL3 setup loads its configuration from %s at %s (%s)." % (
            tcr_source["address"], tcr_source["pc"], tcr_source["expression"]
        ))
    else:
        lines.append("- No statically tracked TCR_EL3 configuration source was recovered.")
    if per_core:
        lines.append("- Per-core TPIDR_EL3 context addressing uses base %s at %s (%s)." % (
            per_core["address"], per_core["pc"], per_core["expression"]
        ))
    else:
        lines.append("- No per-core TPIDR_EL3 context base was recovered.")

    candidate = [item for item in refs if item["range"] == RANGES[0][2]]
    lines += ["", "## Proposed application candidate references", ""]
    if candidate:
        for item in candidate:
            lines.append("- %s at %s (%s): %s" % (
                item["address"], item["pc"], item["kind"], item["expression"]
            ))
    else:
        lines.append("- No direct candidate-range references were recovered by this scan.")

    lines += [
        "", "## Interpretation limits", "",
        "- This is static triage of a matched BL31 binary, not memory-ownership approval.",
        "- The BL31 code references runtime/page-table/context addresses beyond the 107,216-byte file payload; file length is not the monitor's total runtime memory footprint.",
        "- Only instructions reachable via direct control-flow edges are decoded; indirect branches/calls and exception-return paths may be incomplete.",
        "- Dynamic address construction, indirect calls, secure-world services, DMA, runtime scratch, cache aliases and secondary-core activity are not fully modeled.",
        "- The exact matched monitor unconditionally requests mappings only inside [0x48000000,0x48041000). Its 0x48000018 flag is zero, so the [0x48200000,0x4a000000) request path is skipped when the pinned SPL leaves that word unchanged. Keep [0x48000000,0x48041000) excluded; do not reserve the inactive 30 MiB range solely from its skipped callsites.",
        "- The one reference to 0x48041000 is an end pointer for the unconditional map, not a dereferenced byte. The endpoint 0x4a000000 is the separate BL33 loader-slot start.",
        "- The candidate [0x40080000,0x42000000) remains UNAPPROVED. Keep H700_MEMORY_LAYOUT_APPROVED unset; do not assign MEMBASE/MEMSIZE or execute a payload.",
        "- No hardware MMIO access, boot-media write, flash, or device payload execution was performed.", "",
    ]
    return report, "\n".join(lines)

def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reports-root", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    reports_root = args.reports_root.resolve()
    monitor_path = reports_root / "rg34xx-inventory-20261126-timer/toc1-extracted/items/monitor.bin"
    identity = verify_monitor_identity(reports_root, monitor_path)
    raw = monitor_path.read_bytes()
    entry = decode_a64_b(raw, 0)
    if entry <= IMAGE_BASE or entry >= IMAGE_BASE + len(raw):
        raise RuntimeError("BL31 entry branch target is outside the monitor payload")
    refs, disassembly, count = scan_addresses(raw, entry)
    report, markdown = build_report(identity, entry, refs, count)
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    (out / "report.json").write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    (out / "REPORT.md").write_text(markdown + "\n", encoding="utf-8")
    (out / "bl31-reachable-disassembly.txt").write_text(disassembly, encoding="utf-8")
    print(markdown)
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
