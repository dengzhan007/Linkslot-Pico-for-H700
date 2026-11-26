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
    (0x48000000, 0x49000000, "BL31 monitor window"),
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

def build_report(identity: dict, entry: int, refs: list[dict], count: int) -> tuple[dict, str]:
    counts: dict[str, int] = {}
    for item in refs:
        counts[item["range"]] = counts.get(item["range"], 0) + 1
    report = {
        "schema": "rg34xx-bl31-static-memory-triage-v1",
        "monitor": identity,
        "image_base": "0x%08x" % IMAGE_BASE,
        "entry": "0x%08x" % entry,
        "reachable_decoded_instruction_count": count,
        "recoverable_address_reference_count": len(refs),
        "reference_counts_by_range": counts,
        "references": refs,
        "limits": [
            "Linear disassembly can include literal/data bytes that happen to decode as instructions.",
            "Register tracking is block-local and conservative; computed and indirect pointers may be missed.",
            "No references found in a range would not prove that BL31 never accesses it.",
            "This does not prove Secure/Non-secure ownership, DMA reachability, cache aliases, runtime scratch, or per-boot behavior.",
            "No hardware registers were read or written; no image was flashed or executed.",
        ],
    }
    lines = [
        "# RG34XX BL31 static memory triage", "",
        "- Monitor payload: %d bytes" % identity["size_bytes"],
        "- SHA-256: %s" % identity["sha256"],
        "- Matched native firmware / TOC1 component: yes",
        "- BL31 image base: 0x%08x" % IMAGE_BASE,
        "- First-entry branch target: 0x%08x" % entry,
        "- Reachable decoded instructions: %d" % count,
        "- Recoverable address references: %d" % len(refs), "",
        "## References grouped by mapped range", "",
    ]
    if counts:
        for name in sorted(counts):
            lines.append("- %s: %d" % (name, counts[name]))
    else:
        lines.append("- No references into audited DRAM ranges were recovered by this limited scan.")
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
        "- Only instructions reachable via direct control-flow edges are decoded; indirect branches/calls and exception-return paths may be incomplete.",
        "- Dynamic address construction, indirect calls, secure-world services, DMA, runtime scratch, cache aliases and secondary-core activity are not fully modeled.",
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
