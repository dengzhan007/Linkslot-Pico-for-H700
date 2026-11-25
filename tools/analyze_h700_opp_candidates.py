#!/usr/bin/env python3
"""Read-only diagnostic OPP candidate enumerator for RG34XX/H700 Linux 4.9.

This models selected Linux 4.9 OPP-core property-presence, supported-hw and
named-voltage lookup rules. It does not decode SID, identify the exact running
kernel implementation, model regulator selector tables, authorize an OPP, or
write hardware state.
"""
from __future__ import annotations

import argparse
import json
import re
from functools import lru_cache
from pathlib import Path
from typing import Any, Dict, List, Optional, Sequence, Tuple


class AnalysisError(ValueError):
    pass


U32_MASK = 0xFFFFFFFF


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
    """Accept a positive scalar voltage or a well-ordered min/target/max triplet."""
    if not isinstance(cells, list) or not all(
        isinstance(value, int) and value > 0 for value in cells
    ):
        return False
    if len(cells) == 1:
        return True
    if len(cells) == 3:
        minimum, target, maximum = cells
        return minimum <= target <= maximum
    return False


def _has_valid_voltage(entry: Dict[str, Any], suffix: Optional[str]) -> bool:
    """Mirror named-property lookup: base fallback happens only if name is absent."""
    if suffix is not None:
        variant_names = entry["voltage_microvolt_variant_names"]
        if suffix in variant_names:
            variants = entry["voltage_microvolt_variants"]
            return _valid_voltage_cells(variants.get(suffix, []))

    if entry["voltage_microvolt_present"]:
        return _valid_voltage_cells(entry["voltage_microvolt_cells"])
    return False


def _supported_hw_cells(entry: Dict[str, Any]) -> List[int]:
    cells = entry.get("supported_hw_cells", [])
    if not isinstance(cells, list):
        return []
    return [
        value for value in cells
        if isinstance(value, int) and 0 <= value <= U32_MASK
    ]


def _opp_supported_by_model(
    entry: Dict[str, Any], supported_hw_mask: Optional[int]
) -> bool:
    """Mirror one table-level supported_hw u32, including OPP version alternatives."""
    present = entry["supported_hw_present"]
    cells = _supported_hw_cells(entry)

    if supported_hw_mask is None:
        # Without a table supported_hw array, any present property is rejected,
        # including a present zero-cell property.
        return not present

    # With one table-level mask cell, Linux 4.9 accepts an OPP when at least
    # one version in its property intersects that active mask.
    if not present or not cells:
        return False
    return any((cell & supported_hw_mask) != 0 for cell in cells)


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


def _find_exact_witness_mask(
    entries: Sequence[Dict[str, Any]],
    suffix: Optional[str],
    runtime_frequencies: Sequence[int],
) -> Optional[int]:
    """Find one 32-bit mask that reproduces the runtime set, if one exists.

    This is a small set-cover search over the one-cell table-mask model. An
    eligible OPP frequency must be hit by at least one of its version masks;
    no eligible OPP outside the runtime set may be hit.
    """
    runtime_set = set(runtime_frequencies)
    cells_by_frequency: Dict[int, List[int]] = {}
    excluded_bits = 0

    for entry in entries:
        if not _entry_is_enabled(entry) or not entry["supported_hw_present"]:
            continue
        if not _has_valid_voltage(entry, suffix):
            continue
        frequency = entry.get("frequency_hz")
        if not isinstance(frequency, int) or frequency <= 0:
            continue
        cells = [value for value in _supported_hw_cells(entry) if value]
        if not cells:
            continue
        if frequency in runtime_set:
            cells_by_frequency.setdefault(frequency, []).extend(cells)
        else:
            for value in cells:
                excluded_bits |= value

    allowed_bits = U32_MASK & ~excluded_bits
    required_unions: List[int] = []
    for frequency in sorted(runtime_set):
        candidates = cells_by_frequency.get(frequency, [])
        union = 0
        for value in candidates:
            union |= value & allowed_bits
        if union == 0:
            return None
        required_unions.append(union)

    # A selected bit satisfies every frequency whose OPP version mask contains it.
    @lru_cache(maxsize=None)
    def solve(remaining: Tuple[int, ...]) -> Optional[int]:
        if not remaining:
            return 0

        chosen = min(remaining, key=lambda index: bin(required_unions[index]).count("1"))
        choices = [
            1 << bit for bit in range(32)
            if required_unions[chosen] & (1 << bit)
        ]
        choices.sort(
            key=lambda bit: -sum(
                1 for index in remaining if required_unions[index] & bit
            )
        )

        for bit in choices:
            rest = tuple(
                index for index in remaining
                if not (required_unions[index] & bit)
            )
            tail = solve(rest)
            if tail is not None:
                return bit | tail
        return None

    return solve(tuple(range(len(required_unions))))


def _mask_hypotheses(entries: Sequence[Dict[str, Any]]) -> List[int]:
    """Return bounded probes: all atomic u32 bits and masks present in the FDT."""
    masks = {1 << bit for bit in range(32)}
    for entry in entries:
        masks.update(value for value in _supported_hw_cells(entry) if value)
    return sorted(masks)


def analyze_opp_candidates(
    live_fdt: Dict[str, Any], cpu_runtime_text: str
) -> Dict[str, Any]:
    runtime_hz = parse_runtime_frequencies(cpu_runtime_text)
    cpu_power = live_fdt.get("cpu_power", {})
    tables = cpu_power.get("cpu_opp_tables", [])
    if not tables:
        raise AnalysisError("live-fdt.json has no cpu_power.cpu_opp_tables")
    if len(tables) != 1:
        raise AnalysisError(
            f"expected one CPU OPP table for this diagnostic, found {len(tables)}"
        )

    table = tables[0]
    entries = table.get("entries", [])
    if not entries:
        raise AnalysisError("the CPU OPP table has no entries")

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

    mask_hypotheses = set(_mask_hypotheses(entries))
    exact_witnesses: Dict[Optional[str], int] = {}
    for suffix in suffixes:
        witness = _find_exact_witness_mask(entries, suffix, runtime_hz)
        if witness is not None:
            exact_witnesses[suffix] = witness
            mask_hypotheses.add(witness)

    runtime_set = set(runtime_hz)
    results: List[Dict[str, Any]] = []
    for mask in [None] + sorted(mask_hypotheses):
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
                "mask_is_derived_exact_witness": (
                    mask is not None and exact_witnesses.get(suffix) == mask
                ),
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
        "opp_table_node": table.get("node"),
        "opp_table_nvmem_cell_names": table.get("nvmem_cell_names", []),
        "exact_matches": exact,
        "closest_candidates": results[:10],
        "mask_hypothesis_count": len(mask_hypotheses),
        "mask_hypothesis_strategy": (
            "all 32 atomic u32 bits, unique non-zero OPP version masks, "
            "and one exact set-cover witness per voltage suffix when one exists"
        ),
        "exact_witness_masks_by_voltage_suffix": {
            suffix if suffix is not None else "<base>": mask
            for suffix, mask in exact_witnesses.items()
        },
        "voltage_suffixes_evaluated": suffixes,
        "notes": [
            "The model mirrors Linux 4.9 OPP-core property-presence semantics.",
            "The supported_hw model assumes one table-level u32; multiple OPP property cells are treated as alternative versions.",
            "Mask probes are bounded hypotheses, not exhaustive enumeration of all 2^32 possible masks.",
            "A derived exact mask is a mathematical fingerprint, not a decoded SID value or proof of the shipping driver.",
            "Voltage parsing checks property shape/order only; regulator selector tables and regulator_is_supported_voltage() are not simulated.",
            "The exact running kernel source, cpufreq driver and regulator implementation must still be identified.",
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
                "derived_witness": item["mask_is_derived_exact_witness"],
            }
            for item in report["exact_matches"]
        ],
        "output": str(args.output) if args.output else None,
    }, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
