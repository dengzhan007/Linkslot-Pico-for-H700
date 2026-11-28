#!/usr/bin/env python3
"""Produce a conservative reachable-callgraph and store-instruction audit for an AArch64 SPL ELF disassembly.

This is a triage report, not a complete semantic MMIO proof. It follows direct BL
calls and direct tail branches to known function symbols, and records all
reachable store-class instructions for manual review.
"""
from __future__ import annotations

import argparse
import json
import re
from collections import deque
from pathlib import Path

FUNCTION = re.compile(r"^\s*([0-9a-fA-F]+) <([^>]+)>:\s*$")
INSTRUCTION = re.compile(
    r"^\s*([0-9a-fA-F]+):\s+[0-9a-fA-F]+\s+([a-zA-Z0-9.]+)\s*(.*?)\s*$"
)
BRANCH = re.compile(r"^(?:0x)?([0-9a-fA-F]+)\s+<([^>]+)>")
STORE = re.compile(
    r"^(?:str|stp|stur|sttr|stlr|stlrb|stlrh|stlxp|stxp|stadd|stclr|steor|stset|swp|cas)[a-z0-9.]*$"
)
KNOWN_SIDE_EFFECT_PATHS = {
    "sunxi_sram_init", "tzpc_init", "timer_init", "clock_init",
    "clock_init_safe", "clock_h6_set_cpu_pll", "mmc_initialize",
    "mmc_init", "find_mmc_device", "baseos_early_mmc",
}


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--disassembly", type=Path, required=True)
    ap.add_argument("--output", type=Path, required=True)
    ap.add_argument("--root", default="board_init_f")
    args = ap.parse_args()

    functions: dict[str, dict] = {}
    current = None
    for line in args.disassembly.read_text(errors="replace").splitlines():
        m = FUNCTION.match(line)
        if m:
            address, name = m.groups()
            current = name
            functions[current] = {"address": int(address, 16), "calls": [], "stores": []}
            continue
        if current is None:
            continue
        m = INSTRUCTION.match(line)
        if not m:
            continue
        address, mnemonic, operands = m.groups()
        mnemonic = mnemonic.lower()
        if mnemonic in {"bl", "blr", "b"}:
            target = BRANCH.match(operands)
            if target:
                target_name = target.group(2).split("+")[0]
                if target_name in functions or target_name:
                    # Retain direct BL calls and direct B tail branches only.
                    if mnemonic in {"bl", "b"}:
                        functions[current]["calls"].append({
                            "mnemonic": mnemonic, "target": target_name,
                            "address": int(address, 16),
                        })
        if STORE.match(mnemonic):
            functions[current]["stores"].append({
                "address": int(address, 16), "mnemonic": mnemonic,
                "operands": operands,
            })

    if args.root not in functions:
        raise SystemExit(f"root function not found: {args.root}")

    reachable: set[str] = set()
    queue = deque([args.root])
    while queue:
        name = queue.popleft()
        if name in reachable or name not in functions:
            continue
        reachable.add(name)
        for edge in functions[name]["calls"]:
            if edge["target"] in functions and edge["target"] not in reachable:
                queue.append(edge["target"])

    edges = []
    stores = []
    for name in sorted(reachable):
        for edge in functions[name]["calls"]:
            if edge["target"] in reachable:
                edges.append({"from": name, **edge})
        for store in functions[name]["stores"]:
            stores.append({"function": name, **store})

    side_effect_paths = sorted(reachable & KNOWN_SIDE_EFFECT_PATHS)
    report = {
        "kind": "conservative AArch64 SPL reachable callgraph/store audit",
        "root": args.root,
        "reachable_function_count": len(reachable),
        "reachable_functions": sorted(reachable),
        "direct_call_edges": edges,
        "reachable_store_instruction_count": len(stores),
        "reachable_store_instructions": stores,
        "known_initialization_or_device_io_paths_reachable": side_effect_paths,
        "execution_safe": False,
        "execution_authorized": False,
        "limitations": [
            "Follows direct BL and direct B-to-symbol edges only; indirect calls, function pointers, exception vectors and dynamically selected paths may be missed.",
            "Lists store-class instructions but does not prove whether each effective address targets SRAM, DRAM, MMIO, or stack.",
            "Does not decode all register dataflow or prove absence of SMC/system-register side effects.",
            "A clean report would still require manual source, linker-map, ABI and hardware-memory-ownership review."
        ],
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({
        "root": args.root,
        "reachable_function_count": len(reachable),
        "reachable_store_instruction_count": len(stores),
        "known_initialization_or_device_io_paths_reachable": side_effect_paths,
        "execution_safe": False,
        "execution_authorized": False,
        "report": str(args.output),
    }, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
