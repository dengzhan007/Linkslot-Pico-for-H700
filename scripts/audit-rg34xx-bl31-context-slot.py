#!/usr/bin/env python3
"""Conservative whole-payload scan for BL31 context slot 0x48014668.

Aligned overlapping AArch64 windows are heuristic; data can decode as code.
No monitor execution or device access is performed.
"""
from __future__ import annotations
import argparse, hashlib, json
from pathlib import Path
from capstone import CS_ARCH_ARM64, CS_MODE_ARM, Cs
from capstone.arm64_const import ARM64_OP_IMM, ARM64_OP_MEM, ARM64_OP_REG

BASE, SIZE = 0x48000000, 107216
SHA = "676bc0206e3c9a7f229d93ea7f3286095f669940ad91f3c4f3d37bdb18696c31"
SLOT, WINDOW_INSNS = 0x48014668, 16

def canon(name: str) -> str:
    return "x" + name[1:] if name.startswith("w") and name[1:].isdigit() else name

def scan(raw: bytes) -> list[dict]:
    md = Cs(CS_ARCH_ARM64, CS_MODE_ARM)
    md.detail = True
    hits: dict[tuple[int, str], dict] = {}
    for start in range(0, len(raw) - 3, 4):
        regs: dict[str, int] = {}
        decoded = list(md.disasm(raw[start:min(len(raw), start + WINDOW_INSNS * 4)],
                                 BASE + start, count=WINDOW_INSNS))
        if not decoded:
            continue
        for index, ins in enumerate(decoded):
            ops = ins.operands if ins.id else []
            before = dict(regs)
            access = None
            if ins.mnemonic in ("adr", "adrp") and len(ops) >= 2:
                if ops[0].type == ARM64_OP_REG and ops[1].type == ARM64_OP_IMM:
                    dest = canon(ins.reg_name(ops[0].reg))
                    value = int(ops[1].imm)
                    regs[dest] = value & ~0xfff if ins.mnemonic == "adrp" else value
            elif ins.mnemonic in ("add", "sub") and len(ops) >= 3:
                if ops[0].type == ARM64_OP_REG and ops[1].type == ARM64_OP_REG and ops[2].type == ARM64_OP_IMM:
                    dn = ins.reg_name(ops[0].reg)
                    dest, source = canon(dn), canon(ins.reg_name(ops[1].reg))
                    if source in regs:
                        value = regs[source] + int(ops[2].imm) if ins.mnemonic == "add" else regs[source] - int(ops[2].imm)
                        regs[dest] = value & (0xffffffff if dn.startswith("w") else 0xffffffffffffffff)
            elif ins.mnemonic in ("mov", "movz", "movk", "movn") and len(ops) >= 2 and ops[0].type == ARM64_OP_REG:
                dn = ins.reg_name(ops[0].reg)
                dest = canon(dn)
                if ins.mnemonic == "mov" and ops[1].type == ARM64_OP_REG:
                    source = canon(ins.reg_name(ops[1].reg))
                    if source in regs:
                        regs[dest] = regs[source]
                elif ops[1].type == ARM64_OP_IMM:
                    imm = int(ops[1].imm)
                    shift = int(getattr(ops[1].shift, "value", 0) or 0)
                    mask = 0xffffffff if dn.startswith("w") else 0xffffffffffffffff
                    chunk = 0xffff << shift
                    if ins.mnemonic == "movk":
                        value = (regs.get(dest, 0) & ~chunk) | ((imm << shift) & chunk)
                    elif ins.mnemonic == "movn":
                        value = ~(imm << shift) & mask
                    else:
                        value = (imm << shift) & mask
                    regs[dest] = value & mask
            # Resolve PC-relative literal loads and loads from known image addresses.
            if ins.mnemonic == "ldr" and len(ops) >= 2 and ops[0].type == ARM64_OP_REG:
                dest_name = ins.reg_name(ops[0].reg)
                dest = canon(dest_name)
                literal_address = None
                if ops[1].type == ARM64_OP_IMM:
                    literal_address = int(ops[1].imm)
                elif ops[1].type == ARM64_OP_MEM:
                    base = canon(ins.reg_name(ops[1].mem.base)) if ops[1].mem.base else ""
                    index_reg = canon(ins.reg_name(ops[1].mem.index)) if ops[1].mem.index else ""
                    if base in before and not index_reg:
                        literal_address = before[base] + int(ops[1].mem.disp)
                if literal_address is not None and BASE <= literal_address <= BASE + len(raw) - 8:
                    off = literal_address - BASE
                    regs[dest] = int.from_bytes(raw[off:off + 8], "little")
            for op in ops:
                if op.type != ARM64_OP_MEM:
                    continue
                base = canon(ins.reg_name(op.mem.base)) if op.mem.base else ""
                index_reg = canon(ins.reg_name(op.mem.index)) if op.mem.index else ""
                if base in before and not index_reg and before[base] + int(op.mem.disp) == SLOT:
                    access = "possible-direct-write" if ins.mnemonic.startswith(("str", "stp", "stlr", "stur", "stxr", "stlxr")) else "possible-read-or-other"
            if access:
                key = (ins.address, access)
                lo = max(0, start + max(0, index - 3) * 4)
                hi = min(len(raw), start + min(len(decoded), index + 4) * 4)
                context = ["%08x: %s %s" % (x.address, x.mnemonic, x.op_str)
                           for x in md.disasm(raw[lo:hi], BASE + lo)]
                hits[key] = {
                    "pc": "0x%08x" % ins.address, "access": access,
                    "effective_address": "0x%08x" % SLOT,
                    "instruction": "%s %s" % (ins.mnemonic, ins.op_str),
                    "window": context,
                    "decode_scope": "overlapping aligned 16-instruction window; may include data/literal false positives",
                }
            if ops and ops[0].type == ARM64_OP_REG:
                dest = canon(ins.reg_name(ops[0].reg))
                if ins.mnemonic not in ("adr", "adrp", "add", "sub", "mov", "movz", "movk", "movn", "ldr"):
                    regs.pop(dest, None)
    return sorted(hits.values(), key=lambda x: int(x["pc"], 16))

def main() -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--monitor", type=Path, required=True)
    p.add_argument("--out-dir", type=Path, required=True)
    a = p.parse_args()
    raw = a.monitor.read_bytes()
    digest = hashlib.sha256(raw).hexdigest()
    if len(raw) != SIZE or digest != SHA:
        raise RuntimeError("monitor identity mismatch; refusing context-slot scan")
    hits = scan(raw)
    writes = [x for x in hits if x["access"] == "possible-direct-write"]
    result = {
        "schema": "rg34xx-bl31-context-slot-whole-payload-scan-v1",
        "monitor_bytes": len(raw), "monitor_sha256": digest,
        "target_slot": "0x%08x" % SLOT,
        "candidate_access_count": len(hits), "candidate_writes": writes,
        "candidate_reads_or_other": [x for x in hits if x["access"] != "possible-direct-write"],
        "runtime_context_pointer": "UNKNOWN",
        "scope": "host-only overlapping decode; no hardware access or payload execution",
        "limitations": [
            "Overlapping decode can interpret data and literal pools as instructions.",
            "Register tracking is limited to ADR/ADRP, ADD/SUB immediate and MOV within each short window.",
            "Indirect pointer construction, stores through loaded pointers, exception paths and runtime relocation are not resolved.",
            "Candidate writes require validation against function boundaries and reachable control flow.",
            "No candidate write does not prove no indirect or external initialization exists.",
        ],
    }
    out = a.out_dir.resolve()
    out.mkdir(parents=True, exist_ok=True)
    (out / "BL31-CONTEXT-SLOT-SCAN.json").write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    lines = [
        "# BL31 context-slot whole-payload scan", "",
        "- Exact monitor identity verified: %d bytes, SHA-256 %s." % (len(raw), digest),
        "- Target global slot: 0x%08x." % SLOT,
        "- Candidate accesses: %d; possible direct writes: %d." % (len(hits), len(writes)),
        "- Results are overlapping-decode leads, not proven control-flow or live writes.",
        "- Runtime context pointer remains UNKNOWN; no hardware state was read.",
    ]
    for hit in hits:
        lines += ["", "## %s at %s" % (hit["access"], hit["pc"]), "",
                  "- %s" % hit["instruction"]]
        lines += ["    " + line for line in hit["window"]]
    (out / "BL31-CONTEXT-SLOT-SCAN.md").write_text("\n".join(lines) + "\n")
    print("\n".join(lines))
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
