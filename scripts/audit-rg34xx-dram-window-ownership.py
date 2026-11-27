#!/usr/bin/env python3
"""Fail-closed memblock page audit for the stock BL31 DRAM retention window."""
from __future__ import annotations
import argparse
import json
import sys
from pathlib import Path
from typing import Any

FIXED_SOURCE_START = 0x40000000
FIXED_SOURCE_BYTES = 0x140
CMA_DEBUGFS_KEY = "/sys/kernel/debug/cma"


def number(value: Any, field: str) -> int:
    if isinstance(value, bool):
        raise ValueError(f"{field}: boolean is invalid")
    if isinstance(value, int):
        result = value
    elif isinstance(value, str):
        try:
            result = int(value, 0)
        except ValueError as exc:
            raise ValueError(f"{field}: invalid integer {value!r}") from exc
    else:
        raise ValueError(f"{field}: expected integer or numeric string")
    if result < 0:
        raise ValueError(f"{field}: negative value")
    return result


def intervals(rows: Any, field: str) -> list[tuple[int, int]]:
    if not isinstance(rows, list):
        raise ValueError(f"{field}: expected array")
    parsed = []
    for index, row in enumerate(rows):
        if not isinstance(row, dict) or "start" not in row:
            raise ValueError(f"{field}[{index}]: missing start")
        start = number(row["start"], f"{field}[{index}].start")
        if "end_exclusive" in row:
            end = number(row["end_exclusive"], f"{field}[{index}].end_exclusive")
        elif "end_inclusive" in row:
            end = number(row["end_inclusive"], f"{field}[{index}].end_inclusive") + 1
        elif "size_bytes" in row:
            end = start + number(row["size_bytes"], f"{field}[{index}].size_bytes")
        else:
            raise ValueError(f"{field}[{index}]: missing interval end")
        if end <= start:
            raise ValueError(f"{field}[{index}]: empty or inverted interval")
        parsed.append((start, end))
    return parsed


def coverage(start: int, end: int, ranges: list[tuple[int, int]]) -> tuple[int, list[tuple[int, int]]]:
    clipped = sorted((max(start, lo), min(end, hi)) for lo, hi in ranges if lo < end and hi > start)
    merged: list[tuple[int, int]] = []
    for lo, hi in clipped:
        if not merged or lo > merged[-1][1]:
            merged.append((lo, hi))
        else:
            merged[-1] = (merged[-1][0], max(merged[-1][1], hi))
    return sum(hi - lo for lo, hi in merged), merged


def analyze_report(data: dict[str, Any], window_start: int = FIXED_SOURCE_START,
                   window_bytes: int = FIXED_SOURCE_BYTES) -> dict[str, Any]:
    if window_bytes <= 0:
        raise ValueError("window_bytes must be positive")
    window_end = window_start + window_bytes
    page_size = None
    if data.get("page_size_bytes") is not None:
        page_size = number(data["page_size_bytes"], "page_size_bytes")
    page_size_valid = bool(page_size and (page_size & (page_size - 1)) == 0)
    memblock = data.get("memblock")
    available = (isinstance(memblock, dict)
                 and isinstance(memblock.get("memory"), list)
                 and isinstance(memblock.get("reserved"), list)
                 and page_size_valid)
    pages: list[dict[str, Any]] = []

    if available:
        memory = intervals(memblock["memory"], "memblock.memory")
        reserved = intervals(memblock["reserved"], "memblock.reserved")
        page_start = (window_start // page_size) * page_size
        page_limit = ((window_end + page_size - 1) // page_size) * page_size
        for start in range(page_start, page_limit, page_size):
            end = start + page_size
            ram_bytes, ram_parts = coverage(start, end, memory)
            reserved_bytes, reserved_parts = coverage(start, end, reserved)
            if ram_bytes != page_size:
                state = "NOT_FULLY_COVERED_BY_MEMBLOCK_MEMORY"
            elif reserved_bytes == page_size:
                state = "FULLY_RESERVED_BY_MEMBLOCK"
            elif reserved_bytes:
                state = "PARTIALLY_RESERVED_BY_MEMBLOCK"
            else:
                state = "UNRESERVED_SYSTEM_RAM"
            pages.append({
                "page_start": hex(start),
                "page_end_exclusive": hex(end),
                "pfn": hex(start // page_size),
                "memblock_memory_covered_bytes": ram_bytes,
                "memblock_reserved_covered_bytes": reserved_bytes,
                "memblock_memory_intersections": [
                    {"start": hex(lo), "end_exclusive": hex(hi)} for lo, hi in ram_parts
                ],
                "memblock_reserved_intersections": [
                    {"start": hex(lo), "end_exclusive": hex(hi)} for lo, hi in reserved_parts
                ],
                "classification": state,
            })

    if not available:
        classification = "UNKNOWN_MEMBLOCK_OR_PAGE_SIZE_DATA"
        gate = "BLOCKED_INSUFFICIENT_MEMORY_EVIDENCE"
    elif all(p["classification"] == "FULLY_RESERVED_BY_MEMBLOCK" for p in pages):
        classification = "ALL_SOURCE_PAGES_FULLY_RESERVED_BY_MEMBLOCK"
        gate = "BLOCKED_PFN_AND_SECURE_OWNERSHIP_UNVERIFIED"
    elif any(p["classification"] == "UNRESERVED_SYSTEM_RAM" for p in pages):
        classification = "UNRESERVED_SYSTEM_RAM"
        gate = "BLOCKED_SOURCE_PAGE_NOT_RESERVED"
    elif any(p["classification"] == "PARTIALLY_RESERVED_BY_MEMBLOCK" for p in pages):
        classification = "PARTIALLY_RESERVED_SYSTEM_RAM"
        gate = "BLOCKED_SOURCE_PAGE_NOT_FULLY_RESERVED"
    else:
        classification = "SOURCE_PAGE_NOT_FULLY_MAPPED_IN_MEMBLOCK_MEMORY"
        gate = "BLOCKED_MEMORY_MAP_INCOMPLETE_OR_DIFFERENT"

    presence = data.get("debugfs_presence", {})
    cma_debugfs = presence.get(CMA_DEBUGFS_KEY) if isinstance(presence, dict) else None
    cma_areas = data.get("cma_areas")
    if cma_debugfs == "not_present":
        cma_status = "UNAVAILABLE_DEBUGFS_NOT_PRESENT"
    elif isinstance(cma_areas, list):
        cma_status = f"REPORTED_AREAS_{len(cma_areas)}"
    else:
        cma_status = "UNAVAILABLE_OR_UNPARSED"

    return {
        "schema": "rg34xx-dram-retention-page-audit-v1",
        "source": {
            "address_start": hex(window_start),
            "address_end_exclusive": hex(window_end),
            "length_bytes": window_bytes,
            "evidence": "matched stock BL31 static analysis; runtime selector/SMC not executed",
        },
        "page_size_bytes": page_size,
        "pages": pages,
        "classification": classification,
        "cma_inventory": {
            "debugfs_status": cma_debugfs or "NOT_RECORDED",
            "status": cma_status,
            "reported_area_count": len(cma_areas) if isinstance(cma_areas, list) else None,
            "note": "CMA totals or missing CMA debugfs data do not prove ownership of this PFN.",
        },
        "runtime_ownership": {
            "pfn_allocator_state": "NOT_CAPTURED",
            "page_owner": "NOT_CAPTURED",
            "secure_world_ownership": "NOT_CAPTURED",
            "bl31_accessibility": "NOT_CAPTURED",
        },
        "gate": gate,
        "H700_MEMORY_LAYOUT_APPROVED": False,
        "hardware_applied": 0,
        "safety_note": (
            "Memblock classification is not proof that a page is allocated/free, "
            "not proof of Secure/Non-secure ownership, and not permission to issue SMC, "
            "access MMIO/DRAM, or execute suspend/resume."
        ),
        "input_fdt_sha256": data.get("fdt_sha256"),
    }


def render_markdown(report: dict[str, Any], input_name: str) -> str:
    lines = [
        "# RG34XX DRAM retention source-page audit",
        "",
        f"- Input: `{input_name}`",
        f"- Source window: `{report['source']['address_start']}`–`{report['source']['address_end_exclusive']}` (exclusive), {report['source']['length_bytes']} bytes",
        f"- Page size: `{report['page_size_bytes']}`",
        f"- Classification: **{report['classification']}**",
        f"- Gate: **{report['gate']}**",
        "- `H700_MEMORY_LAYOUT_APPROVED`: `false`",
        "",
        "| Page | PFN | RAM coverage | Reserved coverage | Classification |",
        "|---|---:|---:|---:|---|",
    ]
    for page in report["pages"]:
        size = report["page_size_bytes"] or 0
        lines.append(
            f"| `{page['page_start']}`–`{page['page_end_exclusive']}` "
            f"| `{page['pfn']}` | {page['memblock_memory_covered_bytes']}/{size} "
            f"| {page['memblock_reserved_covered_bytes']}/{size} "
            f"| {page['classification']} |"
        )
    lines.extend([
        "",
        f"- CMA inventory: {report['cma_inventory']['status']} "
        f"(debugfs: `{report['cma_inventory']['debugfs_status']}`)",
        "- PFN allocator state: NOT CAPTURED",
        "- Page-owner state: NOT CAPTURED",
        "- Secure-world ownership: NOT CAPTURED",
        "- BL31 accessibility: NOT CAPTURED",
        "",
        f"> {report['safety_note']}",
        "",
    ])
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", required=True, type=Path, help="memory-map-report.json")
    parser.add_argument("--out-dir", required=True, type=Path)
    args = parser.parse_args()
    try:
        data = json.loads(args.input.read_text(encoding="utf-8"))
        if not isinstance(data, dict):
            raise ValueError("input JSON root must be an object")
        report = analyze_report(data)
        args.out_dir.mkdir(parents=True, exist_ok=True)
        json_path = args.out_dir / "DRAM-WINDOW-OWNERSHIP.json"
        md_path = args.out_dir / "DRAM-WINDOW-OWNERSHIP.md"
        json_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
        md_path.write_text(render_markdown(report, str(args.input)), encoding="utf-8")
        print(json.dumps({
            "classification": report["classification"],
            "gate": report["gate"],
            "output_json": str(json_path),
            "output_markdown": str(md_path),
        }, indent=2))
        return 0
    except (OSError, json.JSONDecodeError, ValueError) as exc:
        print(f"DRAM window audit failed closed: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
