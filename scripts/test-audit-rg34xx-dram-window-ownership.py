#!/usr/bin/env python3
"""Host regression tests for the fail-closed DRAM retention page classifier."""
from __future__ import annotations
import importlib.util
from pathlib import Path

root = Path(__file__).resolve().parents[1]
path = root / "scripts/audit-rg34xx-dram-window-ownership.py"
spec = importlib.util.spec_from_file_location("dram_window_audit", path)
if spec is None or spec.loader is None:
    raise RuntimeError("cannot load DRAM window audit module")
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


def make_report(reserved: list[dict], memory: list[dict] | None = None) -> dict:
    return {
        "page_size_bytes": 4096,
        "fdt_sha256": "fixture",
        "memblock": {
            "memory": memory or [{"start": "0x40000000", "end_exclusive": "0x80000000"}],
            "reserved": reserved,
        },
        "debugfs_presence": {"/sys/kernel/debug/cma": "not_present"},
        "cma_areas": [],
        "meminfo_bytes": {"CmaTotal": 67108864},
    }


def main() -> int:
    # Relevant ranges from reports/rg34xx-inventory-20261126-timer.
    snapshot = make_report([
        {"start": "0x40080000", "end_exclusive": "0x411b5000"},
        {"start": "0x411b6000", "end_exclusive": "0x411b8000"},
        {"start": "0x48000000", "end_exclusive": "0x49000000"},
    ])
    result = module.analyze_report(snapshot)
    assert result["source"]["address_start"] == "0x40000000"
    assert result["source"]["address_end_exclusive"] == "0x40000140"
    assert result["pages"][0]["page_start"] == "0x40000000"
    assert result["pages"][0]["pfn"] == "0x40000"
    assert result["pages"][0]["memblock_memory_covered_bytes"] == 4096
    assert result["pages"][0]["memblock_reserved_covered_bytes"] == 0
    assert result["classification"] == "UNRESERVED_SYSTEM_RAM"
    assert result["gate"] == "BLOCKED_SOURCE_PAGE_NOT_RESERVED"
    assert result["H700_MEMORY_LAYOUT_APPROVED"] is False
    assert result["runtime_ownership"]["pfn_allocator_state"] == "NOT_CAPTURED"
    assert result["cma_inventory"]["status"] == "UNAVAILABLE_DEBUGFS_NOT_PRESENT"

    # Even a fully reserved memblock page cannot prove PFN or secure ownership.
    full = module.analyze_report(make_report([
        {"start": "0x40000000", "end_exclusive": "0x40001000"}
    ]))
    assert full["classification"] == "ALL_SOURCE_PAGES_FULLY_RESERVED_BY_MEMBLOCK"
    assert full["gate"] == "BLOCKED_PFN_AND_SECURE_OWNERSHIP_UNVERIFIED"
    assert full["H700_MEMORY_LAYOUT_APPROVED"] is False

    partial = module.analyze_report(make_report([
        {"start": "0x40000000", "end_exclusive": "0x40000800"}
    ]))
    assert partial["classification"] == "PARTIALLY_RESERVED_SYSTEM_RAM"
    assert partial["pages"][0]["memblock_reserved_covered_bytes"] == 2048

    adjacent = module.analyze_report(make_report([
        {"start": "0x40000000", "end_exclusive": "0x40000800"},
        {"start": "0x40000800", "end_exclusive": "0x40001000"},
    ]))
    assert adjacent["classification"] == "ALL_SOURCE_PAGES_FULLY_RESERVED_BY_MEMBLOCK"

    # CMA metadata/statistics are never substituted for memblock reservation.
    cma_only = make_report([])
    cma_only["debugfs_presence"] = {"/sys/kernel/debug/cma": "present"}
    cma_only["cma_areas"] = [{"start": "0x40000000", "end_exclusive": "0x40010000"}]
    cma_only["meminfo_bytes"]["CmaTotal"] = 65536
    cma_result = module.analyze_report(cma_only)
    assert cma_result["classification"] == "UNRESERVED_SYSTEM_RAM"
    assert cma_result["gate"] == "BLOCKED_SOURCE_PAGE_NOT_RESERVED"

    outside = module.analyze_report(make_report([], [
        {"start": "0x50000000", "end_exclusive": "0x60000000"}
    ]))
    assert outside["classification"] == "SOURCE_PAGE_NOT_FULLY_MAPPED_IN_MEMBLOCK_MEMORY"
    assert outside["gate"] == "BLOCKED_MEMORY_MAP_INCOMPLETE_OR_DIFFERENT"

    missing = module.analyze_report({"page_size_bytes": 4096})
    assert missing["classification"] == "UNKNOWN_MEMBLOCK_OR_PAGE_SIZE_DATA"
    assert missing["gate"] == "BLOCKED_INSUFFICIENT_MEMORY_EVIDENCE"

    print("PASS: fixed DRAM source-page memblock coverage and fail-closed ownership gates")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
