#!/usr/bin/env python3
"""Fail-closed post-link contract for the isolated M1 SPL ELF and callgraph."""
from __future__ import annotations
import argparse
import json
from pathlib import Path


def audit(manifest: dict, callgraph: dict, output_dir: Path) -> dict:
    required_files = [
        "m1-spl-preflight.elf", "m1-spl-preflight.bin",
        "m1-spl-preflight.map", "m1-spl-preflight.nm.txt",
        "m1-spl-preflight.disasm.txt",
    ]
    missing = [name for name in required_files if not (output_dir / name).is_file()]
    if missing:
        raise ValueError("missing linked audit artifacts: " + ", ".join(missing))
    if manifest.get("geometry_only") is not False:
        raise ValueError("expected the actual preflight link, not the geometry-only fixture")
    if manifest.get("installable") is not False or manifest.get("candidate_eligible") is not False:
        raise ValueError("M1 artifact must remain non-installable and ineligible")
    for key in ("device_io_performed", "media_write_performed",
                "firmware_flash_performed", "payload_executed",
                "firmware_payload_read_at_runtime", "hardware_qualified"):
        if manifest.get(key) is not False:
            raise ValueError(f"manifest must explicitly keep {key}=false")
    if manifest.get("test_fingerprint") != "0x00000000 (deliberate fail-closed sentinel)":
        raise ValueError("M1 package planner fail-closed sentinel changed")
    bss = manifest.get("bss", {})
    start, end = int(bss.get("start", "0"), 0), int(bss.get("end_exclusive", "0"), 0)
    used, limit = bss.get("used_bytes"), bss.get("limit_bytes")
    if start != 0x30000 or end <= start or used != end - start or limit != 0x4000 or used > limit:
        raise ValueError("linked BSS interval/consumption violates the 16KiB SRAM-C contract")
    scratch = manifest.get("m1_package_scratch", {})
    if scratch.get("bytes") != 4096 or scratch.get("alignment") != 64 or scratch.get("inside_bss") is not True:
        raise ValueError("linked 4KiB/64-byte-aligned M1 scratch is not proven inside BSS")
    scratch_addr = int(scratch.get("address", "0"), 0)
    if scratch_addr % 64 or not (start <= scratch_addr and scratch_addr + 4096 <= end):
        raise ValueError("M1 scratch address is misaligned or outside actual linked BSS")
    table = manifest.get("m1_gpt_partitions", {})
    if table.get("entry_slots") != 8 or table.get("inside_bss") is not True:
        raise ValueError("bounded eight-slot GPT table is not proven inside BSS")
    table_addr, table_size = int(table.get("address", "0"), 0), table.get("bytes")
    if not isinstance(table_size, int) or table_size <= 0 or not (start <= table_addr and table_addr + table_size <= end):
        raise ValueError("linked GPT table interval is outside actual BSS")
    stack = int(manifest.get("spl_stack_pointer", "0"), 0)
    if end >= stack or manifest.get("bss_end_to_stack_gap_bytes") != stack - end:
        raise ValueError("linked BSS overlaps stack or gap calculation is inconsistent")
    heap = manifest.get("spl_allocator_profile", {})
    heap_start = int(heap.get("heap_start", "0"), 0)
    heap_end = heap_start + heap.get("heap_bytes", 0)
    if heap_start < end or heap_end > stack or heap.get("heap_high_water_measured") is not False:
        raise ValueError("SPL heap interval or unmeasured high-water declaration violates contract")
    if callgraph.get("execution_safe") is not False or callgraph.get("execution_authorized") is not False:
        raise ValueError("callgraph must remain explicitly non-authorizing")
    if not isinstance(callgraph.get("reachable_functions"), list) or not callgraph["reachable_functions"]:
        raise ValueError("reachable callgraph is missing")
    if not isinstance(callgraph.get("reachable_store_instructions"), list):
        raise ValueError("reachable store inventory is missing")
    return {
        "schema": "m1-spl-linked-contract-v1",
        "result": "PASS_LINKED_ARTIFACTS_AND_FAIL_CLOSED_CONTRACT",
        "required_artifacts": required_files,
        "bss_used_bytes": used,
        "bss_limit_bytes": limit,
        "scratch_address": scratch["address"],
        "gpt_table_address": table["address"],
        "stack_pointer": manifest["spl_stack_pointer"],
        "reachable_function_count": callgraph["reachable_function_count"],
        "reachable_store_instruction_count": callgraph.get("reachable_store_instruction_count"),
        "execution_authorized": False,
        "candidate_eligible": False,
        "hardware_qualified": False,
    }


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--manifest", type=Path, required=True)
    ap.add_argument("--callgraph", type=Path, required=True)
    ap.add_argument("--output", type=Path, required=True)
    args = ap.parse_args()
    try:
        result = audit(json.loads(args.manifest.read_text()),
                       json.loads(args.callgraph.read_text()),
                       args.manifest.parent)
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(result, indent=2) + "\n")
        print(json.dumps(result, indent=2))
        return 0
    except (OSError, ValueError, TypeError, KeyError, json.JSONDecodeError) as exc:
        raise SystemExit("FAIL CLOSED: " + str(exc))


if __name__ == "__main__":
    raise SystemExit(main())
