#!/usr/bin/env python3
"""Diagnostic OPP candidate enumerator for RG34XX/H700 Linux 4.9 reports.

This is a read-only model of the Linux 4.9 OPP-core supported-hw and
opp-microvolt property-selection rules. It does not decode SID, authorize an
OPP, or write hardware state.
"""
from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any, Dict, List, Optional, Sequence, Tuple


class AnalysisError(ValueError):
    pass


def parse_runtime_frequencies(cpu_runtime_text: str) -> List[int]:
    """Read kHz values from all scaling_available_frequencies blocks."""
    pattern = re.compile(
        r"^\[scaling_available_frequencies\]\s*\r?\n([0-9 \t]+)$",
        re.MULTILINE,
    )
    found: List[List[int]] = []
    for match in pattern.finditer(cpu_runtime_text):
        values = [int(value) * 1000 for value in match.group(1).split()]
        if values:
            found.append(sorted(set(values)))
    if not found:
        raise AnalysisError(
            "cpu-power-runtime.txt has no readable scaling_available_frequencies block"
        )
    unique = {tuple(values) for values in found}
    if len(unique) != 1:
        raise AnalysisError(
            "multiple scaling_available_frequencies blocks disagree; "
            "inspect policy/CPU path identities before candidate analysis"
        )
    return list(next(iter(unique)))


def _valid_voltage_cells(cells: Any) -> bool:
    return isinstance(cells, list) and len(cells) in (1, 3) and all(
        isinstance(value, int) and value > 0 for value in cells
    )


def _has_valid_voltage(entry: Dict[str, Any], suffix: Optional[str]) -> bool:
    """Mirror opp_parse_supplies(): selected named property, else base.

    If the selected named property exists but has an invalid number of cells,
    it is an invalid entry. The base property is only tried when the named
    property is absent.
    """
    if suffix is not None:
        variant_names = entry["voltage_microvolt_variant_names"]
        if suffix in variant_names:
            variants = entry["voltage_microvolt_variants"]
            return _valid_voltage_cells(variants.get(suffix, []))

    if entry["voltage_microvolt_present"]:
        return _valid_voltage_cells(entry["voltage_microvolt_cells"])
    return False


def _opp_supported_by_model(
    entry: Dict[str, Any], supported_hw_mask: Optional[int]
) -> bool:
    """Mirror the one-cell supported_hw case in Linux 4.9 OPP core."""
    present = entry["supported_hw_present"]
    cells = entry["supported_hw_cells"]

    if supported_hw_mask is None:
        # With no table supported_hw array, any present opp-supported-hw
        # property is rejected, including a present zero-cell property.
        return not present

    # With one table mask cell, every OPP must have index 0 and intersect it.
    if not present or not cells:
        return False
    return (cells[0] & supported_hw_mask) != 0


def _entry_is_enabled(entry: Dict[str, Any]) -> bool:
    return entry.get("status", "okay") not in ("disabled", "fail", "failed")


def _candidate_frequencies(
    entries: Sequence[Dict[str, Any]],
    supported_hw_mask: Optional[int],
    suffix: Optional[str],
) -> Tuple[List[int], List[str]]:
    frequencies = set()
    nodes: List[str] = []
    for entry in entries:
        if not _entry_is_enabled(entry):
            continue
        if not _opp_supported_by_model(entry, supported_hw_mask):
            continue
        if not _has_valid_voltage(entry, suffix):
            continue
        frequency = entry.get("frequency_hz")
        if not isinstance(frequency, int) or frequency <= 0:
            continue
        frequencies.add(frequency)
        nodes.append(str(entry.get("node", "<unnamed>")))
    return sorted(frequencies), sorted(nodes)


def analyze_opp_candidates(
    live_fdt: Dict[str, Any], cpu_runtime_text: str
) -> Dict[str, Any]:
    runtime_hz = parse_runtime_frequencies(cpu_runtime_text)
    cpu_power = live_fdt.get("cpu_power", {})
    tables = cpu_power.get("cpu_opp_tables", [])
    if not tables:
        raise AnalysisError("live-fdt.json has no cpu_power.cpu_opp_tables")
    entries = tables[0].get("entries", [])
    if not entries:
        raise AnalysisError("the first CPU OPP table has no entries")

    required = (
        "supported_hw_present",
        "supported_hw_cells",
        "voltage_microvolt_present",
        "voltage_microvolt_cells",
        "voltage_microvolt_variant_names",
        "voltage_microvolt_variants",
    )
    for index, entry in enumerate(entries):
        missing = [key for key in required if key not in entry]
        if missing:
            raise AnalysisError(
                "parser-v3 fields missing from OPP entry "
                f"{index} ({entry.get('node', '<unnamed>')}): {', '.join(missing)}; "
                "re-run the inventory with the updated FDT parser"
            )

    suffixes: List[Optional[str]] = [None]
    all_suffixes = sorted({
        name
        for entry in entries
        for name in entry["voltage_microvolt_variant_names"]
    })
    suffixes.extend(all_suffixes)

    # Current examined sun50iw9 candidates call dev_pm_opp_set_supported_hw()
    # with one u32 version mask. Enumerate all non-zero one-cell masks.
    masks: List[Optional[int]] = [None] + list(range(1, 256))
    runtime_set = set(runtime_hz)
    results: List[Dict[str, Any]] = []

    for mask in masks:
        for suffix in suffixes:
            available, nodes = _candidate_frequencies(entries, mask, suffix)
            available_set = set(available)
            missing = sorted(runtime_set - available_set)
            extra = sorted(available_set - runtime_set)
            results.append({
                "supported_hw_mode": (
                    "not_registered" if mask is None else "one_cell_mask"
                ),
                "supported_hw_mask": mask,
                "voltage_property_suffix": suffix,
                "matches_runtime": not missing and not extra,
                "accepted_frequencies_hz": available,
                "missing_runtime_frequencies_hz": missing,
                "extra_candidate_frequencies_hz": extra,
                "accepted_opp_nodes": nodes,
            })

    results.sort(key=lambda item: (
        len(item["missing_runtime_frequencies_hz"])
        + len(item["extra_candidate_frequencies_hz"]),
        0 if item["supported_hw_mask"] is None else 1,
        item["voltage_property_suffix"] or "",
        item["supported_hw_mask"] or 0,
    ))
    exact = [item for item in results if item["matches_runtime"]]
    return {
        "diagnostic_only": True,
        "runtime_available_frequencies_hz": runtime_hz,
        "opp_table_node": tables[0].get("node"),
        "opp_table_nvmem_cell_names": tables[0].get("nvmem_cell_names", []),
        "exact_matches": exact,
        "closest_candidates": results[:10],
        "evaluated_one_cell_supported_hw_masks": 255,
        "voltage_suffixes_evaluated": suffixes,
        "notes": [
            "The model mirrors Linux 4.9 OPP-core property presence semantics.",
            "Only no-mask and one-cell supported_hw models are enumerated.",
            "An exact frequency-list match is a source/table fingerprint, not SID decoding.",
            "The exact running kernel source and cpufreq driver must still be identified.",
            "No OPP, voltage, governor, PLL, CCU or PMIC writes are performed or authorized.",
        ],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--live-fdt", required=True, type=Path)
    parser.add_argument("--cpu-runtime", required=True, type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()

    try:
        live_fdt = json.loads(args.live_fdt.read_text(encoding="utf-8"))
        runtime_text = args.cpu_runtime.read_text(encoding="utf-8", errors="replace")
        report = analyze_opp_candidates(live_fdt, runtime_text)
    except (OSError, json.JSONDecodeError, AnalysisError) as error:
        parser.error(str(error))

    encoded = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(encoded, encoding="utf-8")
    print(json.dumps({
        "exact_match_count": len(report["exact_matches"]),
        "exact_matches": [
            {
                "supported_hw_mode": item["supported_hw_mode"],
                "supported_hw_mask": item["supported_hw_mask"],
                "voltage_property_suffix": item["voltage_property_suffix"],
                "frequencies_mhz": [
                    value // 1000000 for value in item["accepted_frequencies_hz"]
                ],
            }
            for item in report["exact_matches"]
        ],
        "output": str(args.output) if args.output else None,
    }, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
