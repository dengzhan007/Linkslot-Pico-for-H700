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
from capstone.arm64 import ARM64_OP_IMM

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


def classify_pointer(value: int) -> str:
    if BASE <= value < BASE + SIZE:
        return "monitor-internal-address"
    if 0x03000000 <= value < 0x04000000:
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
        if 0x07000000 <= value < 0x08000000:
            nearby_mmio_candidates.append({
                "signed_lookup_index_if_selected": relative_offset // 8,
                "relative_byte_offset": "0x%03x" % relative_offset,
                "address": "0x%08x" % (lookup_origin + relative_offset),
                "raw_qword": "0x%016x" % value,
            })

    global_context_slot = read_u64(blob, 0x48014668)
    global_controller_slot = read_u64(blob, 0x48015548)

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
            "selector_load_sites": ["0x48012520", "0x48012694", "0x480126d4"],
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
        "limitations": [
            "The eight reported qwords are only bytes at relative offsets +0x00 through +0x38 from lookup origin 0x48014628; they are not asserted to be the selected entries. The SXTW index is signed, so an unknown negative context +0xa4 value may select memory before the lookup origin.",
            "The raw qword at 0x48014668 is 2, although reviewed code loads this slot as a context pointer. No BaseOS native SPL patch targets this slot.",
            "The bounded +0x500 data scan finds any R-domain MMIO-looking qwords and reports their relative signed lookup indices as conditional candidates, not observed selections.",
            "The raw qword at 0x48015548 is 2, but BL31 explicitly overwrites it with the selected object pointer at 0x48013b14 before the TWI setup helper consumes it.",
            "This audit verifies the exact static instructions, raw initial values, and the pinned BaseOS patch sites. It does not exclude indirect writes, BL31 runtime initialization, another loader, or runtime mutation outside the reviewed windows.",
            "No TWI bus index, controller base, PMIC slave address, command sequence, or final power action is established.",
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
        "- RTC context selector: offset `+0xa4`; code loads it at `0x48012520`, `0x48012694`, and `0x480126d4`.",
        "- RTC continuation/helper: `0x48012670 → 0x48012070`; controller initialization `0x480129f0 → 0x48013ac8`.",
        "- The initializer performs `LDR X1, [X1, W0, SXTW #3]` from lookup origin `0x48014628`: W0 is a signed 32-bit index scaled by 8, and the unknown value may address before or after that origin. The loaded value is then published to `0x48015548` at `0x48013b14`.",
        "- The raw initial qword at slot `0x48014668` (loaded as a context pointer by reviewed paths) is `0x%016x`; the controller-object global slot `0x48015548` initially contains `0x%016x`." % (global_context_slot, global_controller_slot),
        "- Pinned BaseOS native SPL only patches monitor offsets: %s." % ", ".join("`+%s`" % ("%x" % x) for x in patch_offsets),
        "",
        "- R-domain MMIO-like qwords in the bounded lookup-origin +0x500 data window: %s." % (", ".join("`index %d → %s`" % (x["signed_lookup_index_if_selected"], x["raw_qword"]) for x in nearby_mmio_candidates) or "none"),
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
        "## Conditional R-domain controller candidates",
        "",
        "| Signed index if selected | Address in monitor | Raw qword |",
        "|---:|---:|---:|",
        *["| %d | `%s` | `%s` |" % (x["signed_lookup_index_if_selected"], x["address"], x["raw_qword"]) for x in nearby_mmio_candidates],
        "",
        "These are data candidates only. For example, a qword equal to `0x07081400` at relative index 73 would be selected only if the actual context `+0xa4` were 73; the runtime selector has not been captured.",
        "",
        "## Interpretation",
        "",
        "The code treats the value selected by a signed, 8-byte-scaled index from lookup origin `0x48014628` as a pointer. The eight qwords shown above are only the adjacent raw bytes at offsets +0x00..+0x38; the actual signed selector is unknown, so the selected slot could be before or after this window. The raw adjacent values are not plausible monitor-internal or known H700 MMIO addresses. The static value at `0x48014668` is `2`, even though multiple paths load it as a context pointer. The pinned BaseOS native SPL does not patch either location; it only patches `+0x24`, `+0x30`, and `+0xb0`.",
        "",
        "This narrows the missing evidence to **runtime initialization, indirect mutation, or another loader-side contract**. Do not describe `0x48014628` as a proven runtime pointer table. Do not infer R-TWI0, a bus number, PMIC address, or command sequence from the FDT alone.",
        "",
        "No hardware access or modification was performed. `H700_MEMORY_LAYOUT_APPROVED=false`; `hardware_applied=0`.",
        "",
    ]
    (out / "TWI-DESCRIPTOR.md").write_text("\n".join(lines), encoding="utf-8")
    print("\n".join(lines))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
