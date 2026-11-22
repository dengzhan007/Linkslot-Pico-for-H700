#!/usr/bin/env python3
"""Normalize read-only H700 CMA and memblock debugfs snapshots into byte ranges."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any

SECTION_RE = re.compile(r"^== (.+) ==$")
NODE_RE = re.compile(r"^-- (.+) --$")
REGION_RE = re.compile(
    r"^\s*(\d+):\s*(0x[0-9a-fA-F]+)\.\.(0x[0-9a-fA-F]+)(?:\s*,?\s*(.*))?$"
)
REPORTED_SIZE_RE = re.compile(r"(?:^|[,\s])size:\s*(\d+)\s*([KMG]?)\b", re.IGNORECASE)
INTEGER_RE = re.compile(r"^\s*(0x[0-9a-fA-F]+|[0-9]+)\s*$")


class MemoryMapError(ValueError):
    """Invalid or incomplete debugfs memory snapshot."""


def _read_integer(path: Path) -> int | None:
    try:
        raw = path.read_text(encoding="utf-8").strip()
    except OSError:
        return None
    match = INTEGER_RE.fullmatch(raw)
    if not match:
        return None
    return int(match.group(1), 0)


def _parse_debugfs(text: str) -> dict[str, Any]:
    current_section: str | None = None
    current_node: str | None = None
    current_attr: str | None = None
    cma: list[dict[str, Any]] = []
    memblock: dict[str, list[dict[str, Any]]] = {"memory": [], "reserved": []}
    presence: dict[str, str] = {}
    active_cma: dict[str, Any] | None = None
    active_range: dict[str, Any] | None = None

    def finish_cma_range() -> None:
        nonlocal active_range, active_cma
        if active_range is not None and active_cma is not None:
            active_cma.setdefault("ranges", []).append(active_range)
        active_range = None

    def finish_cma() -> None:
        nonlocal active_cma
        finish_cma_range()
        if active_cma is not None:
            cma.append(active_cma)
        active_cma = None

    for line in text.splitlines():
        section = SECTION_RE.match(line)
        if section:
            finish_cma()
            current_section = section.group(1).strip().lower()
            current_node = None
            current_attr = None
            continue

        if line.startswith("NOT_PRESENT:"):
            presence[line.split(":", 1)[1].strip()] = "not_present"
            current_attr = None
            continue

        node = NODE_RE.match(line)
        if node:
            if current_section == "cma debugfs":
                name = node.group(1)
                if "/ranges/" in name:
                    finish_cma_range()
                    active_range = {"path": name, "base_pfn": None, "count": None}
                else:
                    finish_cma()
                    active_cma = {
                        "path": name,
                        "base_pfn": None,
                        "count": None,
                        "order_per_bit": None,
                        "used": None,
                        "ranges": [],
                    }
                current_node = name
            current_attr = None
            continue

        if line.startswith("[") and line.endswith("]"):
            current_attr = line[1:-1].strip()
            continue

        if current_section == "cma debugfs" and current_attr and line.strip():
            value = line.strip()
            try:
                parsed: int | str = int(value, 0)
            except ValueError:
                parsed = value
            if active_range is not None and current_attr in ("base_pfn", "count"):
                active_range[current_attr] = parsed
            elif active_cma is not None and current_attr in (
                "base_pfn", "count", "order_per_bit", "used"
            ):
                active_cma[current_attr] = parsed
            continue

        if current_section == "memblock debugfs" and current_attr in ("memory", "reserved"):
            match = REGION_RE.match(line)
            if match:
                start = int(match.group(2), 16)
                end_inclusive = int(match.group(3), 16)
                if end_inclusive < start:
                    raise MemoryMapError(f"memblock region end precedes start: {line}")
                suffix = (match.group(4) or "").strip()
                size_match = REPORTED_SIZE_RE.search(suffix)
                reported_size = None
                if size_match:
                    size_value = int(size_match.group(1))
                    size_unit = size_match.group(2).upper()
                    unit_power = {"": 0, "K": 1, "M": 2, "G": 3}[size_unit]
                    reported_size = {
                        "value": size_value,
                        "unit": size_unit or "B",
                        "bytes": size_value * (1024 ** unit_power),
                    }
                memblock[current_attr].append({
                    "index": int(match.group(1)),
                    "start": f"0x{start:x}",
                    "end_inclusive": f"0x{end_inclusive:x}",
                    "end_exclusive": f"0x{end_inclusive + 1:x}",
                    "size_bytes": end_inclusive - start + 1,
                    "reported_size": reported_size,
                    "attributes": suffix,
                })

    finish_cma()
    return {
        "cma_areas": cma,
        "memblock": memblock,
        "presence": presence,
    }


def _normalize_cma(areas: list[dict[str, Any]], page_size: int) -> list[dict[str, Any]]:
    normalized = []
    for area in areas:
        ranges = area.get("ranges") or []
        # Newer kernels expose per-range PFNs. Older 4.9 kernels normally
        # expose only base_pfn/count on the CMA area itself.
        if ranges:
            source_ranges = ranges
        else:
            source_ranges = [{
                "path": area["path"],
                "base_pfn": area.get("base_pfn"),
                "count": area.get("count"),
            }]
        regions = []
        for item in source_ranges:
            base_pfn = item.get("base_pfn")
            count = item.get("count")
            if not isinstance(base_pfn, int) or not isinstance(count, int) or base_pfn < 0 or count <= 0:
                regions.append({
                    "path": item.get("path"),
                    "valid": False,
                    "reason": "base_pfn/count unavailable or invalid",
                })
                continue
            start = base_pfn * page_size
            size = count * page_size
            regions.append({
                "path": item.get("path"),
                "valid": True,
                "base_pfn": base_pfn,
                "page_count": count,
                "start": f"0x{start:x}",
                "size_bytes": size,
                "end_exclusive": f"0x{start + size:x}",
            })
        normalized.append({
            "path": area["path"],
            "base_pfn": area.get("base_pfn"),
            "count_pages": area.get("count"),
            "order_per_bit": area.get("order_per_bit"),
            "used_pages": area.get("used"),
            "ranges": regions,
        })
    return normalized


def inspect_snapshot(debugfs_path: Path, page_size_path: Path | None = None,
                     fdt_path: Path | None = None,
                     meminfo_path: Path | None = None) -> dict[str, Any]:
    if not debugfs_path.is_file():
        raise MemoryMapError(f"debugfs snapshot not found: {debugfs_path}")
    page_size = _read_integer(page_size_path) if page_size_path else None
    if page_size is not None and page_size <= 0:
        page_size = None

    parsed = _parse_debugfs(debugfs_path.read_text(encoding="utf-8", errors="replace"))
    limitations = [
        "CMA PFN ranges identify CMA-managed physical pages; they do not by themselves describe every secure-world or device-specific reservation.",
        "memblock data can be absent after boot on kernels that do not preserve memblock arrays; absence is not proof that no reserved regions exist.",
        "This report is based only on the captured read-only debugfs snapshot.",
    ]
    if page_size is None:
        limitations.append(
            "PAGE_SIZE is unavailable: CMA PFNs are retained as raw PFN/count values and are not converted to physical byte ranges. "
            "Memblock ranges are already physical byte addresses and are still parsed."
        )
    report: dict[str, Any] = {
        "page_size_bytes": page_size,
        "cma_pfns_normalized": page_size is not None,
        "cma_areas": (
            _normalize_cma(parsed["cma_areas"], page_size)
            if page_size is not None else parsed["cma_areas"]
        ),
        "memblock": parsed["memblock"],
        "debugfs_presence": parsed["presence"],
        "limitations": limitations,
    }
    if fdt_path and fdt_path.is_file():
        fdt = json.loads(fdt_path.read_text(encoding="utf-8"))
        report["ram_regions"] = fdt.get("memory_regions", [])
        report["framebuffers"] = fdt.get("boot_framebuffers", [])
        report["fdt_sha256"] = fdt.get("sha256")
    if meminfo_path and meminfo_path.is_file():
        values: dict[str, int] = {}
        for line in meminfo_path.read_text(encoding="utf-8", errors="replace").splitlines():
            match = re.match(r"^([A-Za-z_()]+):\s*(\d+)\s+kB$", line)
            if match:
                values[match.group(1)] = int(match.group(2)) * 1024
        report["meminfo_bytes"] = {
            key: values[key] for key in ("MemTotal", "CmaTotal", "CmaFree") if key in values
        }
    return report


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("debugfs_snapshot", type=Path)
    parser.add_argument("--page-size", type=Path)
    parser.add_argument("--fdt-json", type=Path)
    parser.add_argument("--meminfo", type=Path)
    parser.add_argument("-o", "--output", type=Path)
    args = parser.parse_args()
    try:
        report = inspect_snapshot(args.debugfs_snapshot, args.page_size, args.fdt_json, args.meminfo)
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1
    rendered = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    else:
        print(rendered, end="")
    cma_regions = sum(
        1 for area in report["cma_areas"] for region in area["ranges"] if region.get("valid")
    )
    print(
        f"parsed {len(report['cma_areas'])} CMA areas, {cma_regions} valid CMA ranges, "
        f"{len(report['memblock']['memory'])} memblock memory ranges and "
        f"{len(report['memblock']['reserved'])} reserved ranges",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
