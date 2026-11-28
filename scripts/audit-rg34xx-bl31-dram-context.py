#!/usr/bin/env python3
"""Trace trained DRAM parameters from the pinned BaseOS SPL into BL31 retention context.

Host-only static/source audit. The derived midpoint is conditional on the captured
Boot0 primary candidate and matching 1-GiB trained geometry; it is not a live
suspend trace or permission to access DRAM.
"""
from __future__ import annotations
import argparse
import hashlib
import json
from pathlib import Path

BASE = 0x48000000
SIZE = 107216
SHA = "676bc0206e3c9a7f229d93ea7f3286095f669940ad91f3c4f3d37bdb18696c31"

# Exact instruction words from the matched monitor: standby context copy,
# DRAM selector helper, caller's selector store and second-window address math.
WORDS = {
    0x48001A28: 0xD2A00053,  # mov x19, #0x20000
    0x48001A90: 0xD2800601,  # mov x1, #0x30
    0x48001A94: 0xD2801002,  # mov x2, #0x80
    0x48001A9C: 0xF2A90001,  # movk x1, #0x4800, lsl #16
    0x48001AA4: 0xAA1303E0,  # mov x0, x19
    0x48001AA8: 0x940023F6,  # bl memcpy
    0x4800C730: 0x29430C01,  # ldp w1, w3, [x0, #0x18]
    0x4800C734: 0xD3442C26,
    0x4800C738: 0xD34C3424,
    0x4800C73C: 0xD34E3C25,
    0x4800C740: 0xD34C3C62,
    0x4800C744: 0xF2400C7F,
    0x4800C748: 0x0B0400C4,
    0x4800C74C: 0x0B0200A2,
    0x4800C750: 0x12000C21,
    0x4800C754: 0x0B020082,
    0x4800C758: 0x0B010041,
    0x4800C760: 0x51004821,
    0x4800C764: 0x52800022,
    0x4800C768: 0x1AC12041,
    0x4800C76C: 0x7940F400,
    0x4800C770: 0xF240081F,
    0x4800C774: 0x540000C0,
    0x4800C784: 0x0B010421,
    0x4800C788: 0x53027C21,
    0x4800C78C: 0x2A0103E0,
    0x48011990: 0x97FFEB68,  # bl 0x4800c730
    0x480119C0: 0x12003CE0,  # and w0, w7, #0xffff
    0x480119C4: 0x2A0A4000,  # orr w0, w0, w10, lsl #16
    0x480119C8: 0xB9001E60,  # str w0, [x19, #0x1c]
    0x480119D0: 0x36D80880,  # tbz w0, #27, 0x48011ae0
    0x480119E8: 0x52A80001,  # mov w1, #0x40000000
    0x480119EC: 0x53117C00,  # lsr w0, w0, #17
    0x480119F8: 0x0B005020,  # add w0, w1, w0, lsl #20
    0x48011AE0: 0x97FFEB32,  # fallback controller configuration helper
    0x48011AF4: 0x3607F7A0,  # tbz w0, #0, 0x480119e8
}

def u32(raw: bytes, address: int) -> int:
    off = address - BASE
    if off < 0 or off + 4 > len(raw):
        raise ValueError(f"address outside monitor: {address:#x}")
    return int.from_bytes(raw[off:off + 4], "little")

def helper_result(word6: int, word7: int, word30: int) -> int:
    """Reproduce the integer-only operations in BL31 helper 0x4800c730."""
    total = ((word6 >> 4) & 0xff) + ((word6 >> 12) & 3)
    total += ((word6 >> 14) & 3) + ((word7 >> 12) & 0xf)
    total += word6 & 0xf
    shift = total - (0x13 if word7 & 0xf else 0x12)
    result = 1 << (shift & 31)  # AArch64 variable W-register shift semantics
    halfword = (word30 >> 16) & 0xffff
    if halfword & 7 and ((word7 >> 30) & 3) != 2:
        result = ((result * 3) & 0xffffffff) >> 2
    return result & 0xffffffff

def derive(words: list[int]) -> dict:
    if len(words) != 32:
        raise ValueError("Boot0 DRAM candidate must contain exactly 32 words")
    word6, word7, word30 = words[6], words[7], words[30]
    result = helper_result(word6, word7, word30)
    rewritten_word7 = (word7 & 0xffff) | ((result & 0xffff) << 16)
    selector = (rewritten_word7 >> 17) & 0x7fff
    second = 0x40000000 + (selector << 20)
    return {
        "candidate_type": words[1],
        "candidate_clock_mhz": words[0],
        "word6": f"0x{word6:08x}",
        "word7_before": f"0x{word7:08x}",
        "word30": f"0x{word30:08x}",
        "word30_high_halfword": f"0x{(word30 >> 16) & 0xffff:04x}",
        "rows": (word6 >> 4) & 0xff,
        "columns": word6 & 0xf,
        "capacity_mib_from_word7": (word7 >> 16) & 0x7fff,
        "rank2_flag": bool(word7 & 0x1000),
        "narrow_bus_flag": bool(word7 & 1),
        "helper_result": f"0x{result:08x}",
        "word7_after_helper": f"0x{rewritten_word7:08x}",
        "midpoint_selector_mib": selector,
        "second_source_window": [f"0x{second:08x}", f"0x{second + 0x140:08x}"],
        "context_plus_78_bit27": bool(word30 & (1 << 27)),
        "context_plus_1c_bit0": bool(rewritten_word7 & 1),
        "context_plus_04_is_four": words[1] == 4,
    }

def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--monitor", type=Path, required=True)
    ap.add_argument("--boot0-primary-json", type=Path, required=True)
    ap.add_argument("--native-boot-source", type=Path, required=True)
    ap.add_argument("--dram-select-source", type=Path, required=True)
    ap.add_argument("--out-dir", type=Path, required=True)
    args = ap.parse_args()

    raw = args.monitor.read_bytes()
    digest = hashlib.sha256(raw).hexdigest()
    if len(raw) != SIZE or digest != SHA:
        raise RuntimeError("matched monitor identity mismatch")
    for address, expected in WORDS.items():
        actual = u32(raw, address)
        if actual != expected:
            raise RuntimeError(f"monitor instruction changed at {address:#x}: {actual:#010x}")

    handoff = args.native_boot_source.read_text(encoding="utf-8")
    dram_source = args.dram_select_source.read_text(encoding="utf-8")
    required_handoff = (
        "const u32 *parameters = baseos_selected_vendor_params();",
        "memcpy(monitor + 0x30, parameters, 128);",
    )
    required_dram = (
        "const u32 *baseos_selected_vendor_params(void) { return trained_vendor_params; }",
        "trained_vendor_params[i] = dram_vendor_candidates[candidate][i];",
        "trained_vendor_params[6] = (3u << 12) | (config->rows << 4) | config->cols;",
        "trained_vendor_params[7] = (size >> 20 << 16) |",
    )
    for fragment in required_handoff:
        if fragment not in handoff:
            raise RuntimeError("native SPL monitor parameter handoff changed: " + fragment)
    for fragment in required_dram:
        if fragment not in dram_source:
            raise RuntimeError("native SPL trained-parameter producer changed: " + fragment)

    boot = json.loads(args.boot0_primary_json.read_text(encoding="utf-8"))
    primary = next(x for x in boot["dram_candidates"] if x["name"] == "primary")
    words = [int(x, 0) for x in primary["words"]]
    result = derive(words)
    # This report is the captured primary candidate, not a claim that the
    # native SPL selected it during a live boot.
    if (result["candidate_type"], result["word6"], result["word7_before"], result["word30"]) != (
        8, "0x000030fa", "0x04000000", "0x02008c60"
    ):
        raise RuntimeError("captured RG34XX primary candidate changed")
    if result["helper_result"] != "0x00000400" or result["word7_after_helper"] != "0x04000000":
        raise RuntimeError("captured 1-GiB candidate no longer yields the expected selector")
    if result["second_source_window"] != ["0x60000000", "0x60000140"]:
        raise RuntimeError("captured 1-GiB candidate no longer yields the expected midpoint")

    report = {
        "schema": "rg34xx-bl31-dram-context-parameter-flow-v1",
        "monitor": {"bytes": len(raw), "sha256": digest},
        "parameter_flow": {
            "spl_array": "trained_vendor_params[32]",
            "normalization": "copy selected 32-word candidate; replace word[6] from detected rows/columns and word[7] from detected size/rank/bus; preserve word[30]",
            "spl_to_monitor": "memcpy(monitor + 0x30, parameters, 128)",
            "bl31_to_sram": "0x48001a04 copies monitor +0x30..+0xaf to SRAM context base 0x20000",
            "context_fields": {"plus_0x18": "parameter word[6]", "plus_0x1c": "parameter word[7]",
                               "plus_0x78": "parameter word[30]", "plus_0x7a": "high halfword of parameter word[30]"},
        },
        "captured_boot0_primary_candidate": result,
        "interpretation": [
            "For this captured primary candidate and matching 1-GiB trained geometry, BL31 helper 0x4800c730 returns 0x400.",
            "The caller forms context +0x1c = 0x04000040 and the second DRAM save/restore source window is [0x60000000,0x60000140).",
            "This is a conditional source/binary derivation; candidate selection, actual native SPL execution, patched parameter bytes, and suspend/resume execution were not captured together.",
            "The source window is not free RAM. It is part of BL31's retention save/restore protocol and remains excluded from Linkslot allocations.",
        ],
        "hardware_qualified": False,
        "live_parameter_block_captured": False,
        "suspend_resume_execution_observed": False,
        "device_io_performed": False,
        "payload_execution_performed": False,
        "memory_layout_approved": False,
    }
    out = args.out_dir.resolve()
    out.mkdir(parents=True, exist_ok=True)
    (out / "BL31-DRAM-CONTEXT.json").write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    lines = [
        "# BL31 DRAM context parameter flow", "",
        f"- Matched monitor: {len(raw)} bytes, SHA-256 {digest}.",
        "- SPL source: trained_vendor_params -> monitor +0x30 (128 bytes).",
        "- BL31 power-entry: monitor +0x30..+0xaf -> SRAM context base 0x20000.",
        "- Context +0x18/+0x1c/+0x78/+0x7a map to parameter words [6]/[7]/[30]/high-halfword([30]).",
        f"- Captured primary candidate: type {result['candidate_type']}, {result['capacity_mib_from_word7']} MiB, rows {result['rows']}, columns {result['columns']}.",
        f"- Helper result: {result['helper_result']}; context +0x1c becomes {result['word7_after_helper']}.",
        f"- Conditional second DRAM source window: {result['second_source_window'][0]}..{result['second_source_window'][1]}.",
        "", "## Evidence boundary", "",
        "- This joins pinned source and captured binary/report inputs; it is not a live parameter capture.",
        "- It does not prove the candidate was selected, the suspend handler ran, or save/restore completed.",
        "- Keep both DRAM source pages excluded; no hardware access, memory readback, MMIO/SMC/TWI, suspend/wake, media write, or payload execution occurred.",
        "",
    ]
    (out / "BL31-DRAM-CONTEXT.md").write_text("\n".join(lines), encoding="utf-8")
    print("\n".join(lines))
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
