#!/usr/bin/env python3
"""Heuristic whole-payload scan for CPU/SoC MMIO candidates in matched BL31.

The scanner uses overlapping AArch64 decode windows at every aligned word.
Data and literal-pool bytes may decode as instructions, so results are leads,
not a call graph or proof that a path executes.
"""
from __future__ import annotations
import argparse
import hashlib
import json
import struct
from pathlib import Path
from capstone import CS_ARCH_ARM64, CS_MODE_ARM, Cs
from capstone.arm64_const import ARM64_OP_IMM, ARM64_OP_MEM, ARM64_OP_REG

BASE = 0x48000000
EXPECTED_SIZE = 107216
EXPECTED_SHA = "676bc0206e3c9a7f229d93ea7f3286095f669940ad91f3c4f3d37bdb18696c31"
WINDOW = 40
# More-specific H700 ranges precede the broad SYS_CFG/SID window so
# reports distinguish CCU and PIO candidates without losing other SYS reads.
RANGES = (
    (0x03001000, 0x03002000, "CPUX_CCU"),
    (0x0300B000, 0x0300B400, "PIO"),
    (0x03000000, 0x03010000, "SID_SYS"),
    (0x07000000, 0x07010000, "AON_CPUCFG_PRCM"),
    (0x08100000, 0x08110000, "CPU_SUBSYSTEM"),
    (0x09010000, 0x09010200, "C0_CPUX_CFG"),
)

PIO_BASE = 0x0300B000
# Source-family labels from the sun50iw9p1 pinctrl bank layout.
# PH is bank 7 (7 * 0x24 = 0xfc); PI is bank 8 (8 * 0x24 = 0x120).
PIO_REGISTERS = {
    0x100: ("PH_CFG1", "PH10 mux field"),
    0x110: ("PH_DRV0", "PH10 drive field"),
    0x118: ("PH_PUL0", "PH10 pull field"),
    0x124: ("PI_CFG1", "PI12 mux field"),
    0x130: ("PI_DAT0", "PI12 data bit"),
    0x134: ("PI_DRV0", "PI12 drive field"),
    0x13C: ("PI_PUL0", "PI12 pull field"),
}

def label(address: int) -> str | None:
    for low, high, name in RANGES:
        if low <= address < high:
            return name
    return None

def pio_register(address: int) -> tuple[str, str] | None:
    """Return family-reference register and pin-field names for known PIO offsets."""
    if not (PIO_BASE <= address < PIO_BASE + 0x400):
        return None
    return PIO_REGISTERS.get(address - PIO_BASE)

def canon(name: str) -> str:
    if name.startswith("w") and name[1:].isdigit():
        return "x" + name[1:]
    return name

def shift(operand) -> int:
    try:
        return int(operand.shift.value)
    except (AttributeError, TypeError, ValueError):
        return 0

def literal_hits(raw: bytes) -> list[dict]:
    result = []
    for off in range(0, len(raw) - 3, 4):
        value = struct.unpack_from("<I", raw, off)[0]
        name = label(value)
        if name:
            result.append({
                "file_offset": "0x%08x" % off,
                "image_address": "0x%08x" % (BASE + off),
                "value": "0x%08x" % value,
                "window": name,
            })
    return result

def instruction_hits(raw: bytes) -> list[dict]:
    md = Cs(CS_ARCH_ARM64, CS_MODE_ARM)
    md.detail = True
    hits: dict[tuple[int, int, str], dict] = {}
    for off in range(0, len(raw) - 3, 4):
        start = BASE + off
        regs: dict[str, int] = {}
        listing: list[str] = []
        for ins in md.disasm(raw[off:min(len(raw), off + WINDOW * 4)], start, count=WINDOW):
            ops = ins.operands if ins.id else []
            listing.append("%08x: %s %s" % (ins.address, ins.mnemonic, ins.op_str))
            if ins.mnemonic in ("movz", "movk", "movn") and len(ops) >= 2:
                if ops[0].type == ARM64_OP_REG and ops[1].type == ARM64_OP_IMM:
                    original = ins.reg_name(ops[0].reg)
                    dest = canon(original)
                    imm = int(ops[1].imm)
                    sh = shift(ops[1])
                    mask = (1 << (32 if original.startswith("w") else 64)) - 1
                    cmask = 0xffff << sh
                    if ins.mnemonic == "movz":
                        value = (imm << sh) & mask
                    elif ins.mnemonic == "movn":
                        value = (~(imm << sh)) & mask
                    else:
                        value = (regs.get(dest, 0) & ~cmask) | ((imm << sh) & cmask)
                    regs[dest] = value & mask
            elif ins.mnemonic in ("adr", "adrp"):
                if len(ops) >= 2 and ops[0].type == ARM64_OP_REG and ops[1].type == ARM64_OP_IMM:
                    value = int(ops[1].imm)
                    if ins.mnemonic == "adrp":
                        value &= ~0xfff
                    regs[canon(ins.reg_name(ops[0].reg))] = value
            elif ins.mnemonic == "mov" and len(ops) >= 2 and ops[0].type == ARM64_OP_REG:
                dest = canon(ins.reg_name(ops[0].reg))
                if ops[1].type == ARM64_OP_REG:
                    source = canon(ins.reg_name(ops[1].reg))
                    if source in regs:
                        regs[dest] = regs[source]
                elif ops[1].type == ARM64_OP_IMM:
                    width = 0xffffffff if ins.reg_name(ops[0].reg).startswith("w") else 0xffffffffffffffff
                    regs[dest] = int(ops[1].imm) & width
            elif ins.mnemonic in ("add", "sub") and len(ops) >= 3:
                if ops[0].type == ARM64_OP_REG and ops[1].type == ARM64_OP_REG and ops[2].type == ARM64_OP_IMM:
                    dest_name = ins.reg_name(ops[0].reg)
                    dest = canon(dest_name)
                    source = canon(ins.reg_name(ops[1].reg))
                    if source in regs:
                        amount = int(ops[2].imm) << shift(ops[2])
                        value = regs[source] + amount if ins.mnemonic == "add" else regs[source] - amount
                        width = 0xffffffff if dest_name.startswith("w") else 0xffffffffffffffff
                        regs[dest] = value & width
            if ins.mnemonic.startswith(("ldr", "str", "ldp", "stp", "ldur", "stur")):
                for op in ops:
                    if op.type != ARM64_OP_MEM or not op.mem.base:
                        continue
                    base = canon(ins.reg_name(op.mem.base))
                    if base not in regs:
                        continue
                    address = regs[base] + int(op.mem.disp)
                    name = label(address)
                    if name:
                        key = (ins.address, address, ins.mnemonic)
                        row = {
                            "pc": "0x%08x" % ins.address,
                            "address": "0x%08x" % address,
                            "mnemonic": ins.mnemonic,
                            "operands": ins.op_str,
                            "window": name,
                            "decode_window_start": "0x%08x" % start,
                            "context": listing[-WINDOW:],
                        }
                        pio_info = pio_register(address)
                        if pio_info:
                            row["pio_register"] = pio_info[0]
                            row["pio_pin_field"] = pio_info[1]
                        hits[key] = row
            if ins.mnemonic in ("b", "br", "ret", "eret", "drps") or ins.mnemonic.startswith("b."):
                break
            if ins.mnemonic in ("bl", "blr"):
                regs.clear()
    return sorted(hits.values(), key=lambda row: (int(row["pc"], 16), int(row["address"], 16)))

def region_disassembly(raw: bytes, start: int, end: int) -> tuple[str, list[dict]]:
    md = Cs(CS_ARCH_ARM64, CS_MODE_ARM)
    md.detail = True
    first = max(start, BASE)
    last = min(end, BASE + len(raw))
    listing = []
    branches = []
    for ins in md.disasm(raw[first - BASE:last - BASE], first):
        byte_text = " ".join("%02x" % byte for byte in ins.bytes)
        listing.append("%08x: %-11s %s %s" % (ins.address, byte_text, ins.mnemonic, ins.op_str))
        if ins.mnemonic in ("b", "bl", "b.eq", "b.ne", "cbz", "cbnz", "tbz", "tbnz"):
            if ins.operands and ins.operands[-1].type == ARM64_OP_IMM:
                branches.append({
                    "pc": "0x%08x" % ins.address,
                    "mnemonic": ins.mnemonic,
                    "target": "0x%08x" % int(ins.operands[-1].imm),
                })
    return "\n".join(listing) + "\n", branches


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--reports-root", type=Path, required=True)
    ap.add_argument("--out", type=Path, required=True)
    args = ap.parse_args()
    monitor = args.reports_root.resolve() / "rg34xx-inventory-20261126-timer/toc1-extracted/items/monitor.bin"
    raw = monitor.read_bytes()
    digest = hashlib.sha256(raw).hexdigest()
    if len(raw) != EXPECTED_SIZE or digest != EXPECTED_SHA:
        raise RuntimeError("monitor payload identity mismatch; refusing whole-image scan")
    literals = literal_hits(raw)
    accesses = instruction_hits(raw)
    # The full-image candidate pass identified a dense cluster of CPUCFG
    # accesses here. Save a contiguous linear listing so reviewers can inspect
    # function shape and direct calls; it remains a candidate region, not CFG proof.
    cpu_region_listing, cpu_region_branches = region_disassembly(
        raw, 0x48002D00, 0x48003180
    )
    counts = {name: sum(1 for hit in accesses if hit["window"] == name) for _, _, name in RANGES}
    report = {
        "schema": "rg34xx-bl31-whole-image-cpu-mmio-candidates-v1",
        "monitor": {"size": len(raw), "sha256": digest, "base": "0x48000000"},
        "method": "all aligned overlapping AArch64 decode windows, max 16 instructions",
        "literal_word_hits": literals,
        "decoded_memory_access_candidates": accesses,
        "candidate_counts_by_window": counts,
        "limitations": [
            "Overlapping disassembly may decode data/literal bytes as instructions.",
            "Window-local register tracking is not a call graph and can produce false positives.",
            "No candidate does not prove the peripheral is never accessed through indirect or secure paths.",
            "Use candidates only to guide a second control-flow/source review.",
            "No device MMIO, writes, flash or payload execution were performed.",
        ],
    }
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    report["cpu_control_region_direct_branches"] = cpu_region_branches
    (out / "full-image-cpu-mmio-candidates.json").write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    (out / "cpu-control-region-0x48002d00-0x48003180.txt").write_text(
        cpu_region_listing, encoding="utf-8"
    )
    lines = [
        "# BL31 whole-image CPU/MMIO candidate scan", "",
        "- Monitor: %d bytes, SHA-256 %s." % (len(raw), digest),
        "- Method: all aligned overlapping AArch64 decode windows, max %d instructions per window." % WINDOW,
        "- Literal-pool words in peripheral windows: %d." % len(literals),
        "- Decoded memory-operation candidates: %d." % len(accesses), "",
        "## Candidate counts by window", "",
    ]
    for name, count in counts.items():
        lines.append("- %s: %d" % (name, count))
    lines += [
        "", "## Interpretation", "",
        "- These are leads only. Overlapping decoding can interpret data as instructions.",
        "- A hit outside the primary-entry reachable report is not proof that the path executes.",
        "- A missing hit does not prove the absence of indirect/secure-world CPU control.",
        "- No hardware MMIO, boot-media write, flash or payload execution occurred.", "",
    ]
    (out / "FULL-IMAGE-MMIO.md").write_text("\n".join(lines), encoding="utf-8")
    print("\n".join(lines))
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
