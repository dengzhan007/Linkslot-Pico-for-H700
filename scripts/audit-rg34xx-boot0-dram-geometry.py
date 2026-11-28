#!/usr/bin/env python3
"""Cross-check captured H700 Boot0 DRAM geometry fields against Linux runtime.

Host-only decoder; it never reads device memory or treats a decoded parameter
block as proof that BL31 selected a suspend/resume branch.
"""
from __future__ import annotations
import argparse, json
from pathlib import Path

EXPECTED = {
    "primary": {"sha256_full": "bf886995745a0eb9b5030296f9f6163035e164b3a82183716e67ff876319289f", "bytes": 131072},
    "secondary": {"sha256_full": "4701a85e4db7e8339b91b92f691a83cc954ec63a09369087c2e41d80eafa9f5d", "bytes": 65536},
}
DRAM_BASE = 0x40000000
WINDOW_BYTES = 0x140

def decode(report: dict, label: str) -> dict:
    expected = EXPECTED[label]
    if report.get("sha256_full") != expected["sha256_full"]:
        raise ValueError(f"{label} Boot0 identity mismatch")
    if report.get("bytes") != expected["bytes"] or report.get("checksum_valid") is not True:
        raise ValueError(f"{label} Boot0 size/checksum mismatch")
    primary = next((x for x in report.get("dram_candidates", []) if x.get("name") == "primary"), None)
    if primary is None:
        raise ValueError(f"{label} primary DRAM parameter block missing")
    words = [int(x, 16) for x in primary.get("words", [])]
    if len(words) != 32:
        raise ValueError(f"{label} primary parameter block must contain 32 words")
    if words[1] != 8:
        raise ValueError(f"{label} primary DRAM type is not LPDDR4 (type 8)")
    # Pinned BaseOS native SPL encoder:
    # word[6] = (3 << 12) | (rows << 4) | columns
    # word[7] = (size_MiB << 16) | (dual_rank ? 0x1000 : 0) | (narrow_bus ? 1 : 0)
    geom = words[6]
    if ((geom >> 12) & 0xF) != 3:
        raise ValueError(f"{label} geometry encoding version changed: 0x{geom:08x}")
    rows, columns = (geom >> 4) & 0xFF, geom & 0xF
    size_mib = words[7] >> 16
    ranks = 2 if words[7] & 0x1000 else 1
    bus_bytes = 2 if words[7] & 1 else 4
    calculated = (1 << (columns + rows + 3)) * bus_bytes * ranks
    encoded = size_mib * 1024 * 1024
    if calculated != encoded:
        raise ValueError(f"{label} geometry calculates {calculated} bytes but word[7] encodes {encoded}")
    return {
        "copy": label, "boot0_debug_byte": report.get("debug_byte"),
        "dram_type": "LPDDR4", "dram_type_id": words[1],
        "parameter_clock_mhz": words[0], "geometry_word6": f"0x{geom:08x}",
        "capacity_word7": f"0x{words[7]:08x}", "rows": rows, "columns": columns,
        "ranks": ranks, "bus_width_bits": bus_bytes * 8,
        "capacity_bytes": calculated, "capacity_mib": calculated // (1024 * 1024),
    }

def build(primary: dict, secondary: dict, memory: dict) -> dict:
    decoded = [decode(primary, "primary"), decode(secondary, "secondary")]
    fields = ("dram_type_id", "parameter_clock_mhz", "geometry_word6", "capacity_word7",
              "rows", "columns", "ranks", "bus_width_bits", "capacity_bytes")
    for field in fields:
        if decoded[0][field] != decoded[1][field]:
            raise ValueError(f"Boot0 copies disagree on {field}")
    memblock = memory.get("memblock", {}).get("memory", [])
    matches = [x for x in memblock if int(x.get("start", "0"), 16) == DRAM_BASE
               and int(x.get("size_bytes", -1)) == decoded[0]["capacity_bytes"]]
    if len(matches) != 1:
        raise ValueError("Linux memblock does not contain exactly one matching DRAM window")
    capacity = decoded[0]["capacity_bytes"]
    midpoint = DRAM_BASE + capacity // 2
    return {
        "schema": "rg34xx-boot0-dram-geometry-crosscheck-v1",
        "evidence_scope": "host-only decode of captured Boot0 metadata plus post-boot Linux memblock",
        "boot0_copies": decoded, "copies_agree_on_geometry": True,
        "linux_memblock_base": f"0x{DRAM_BASE:08x}",
        "linux_memblock_capacity_bytes": capacity,
        "boot0_geometry_matches_linux_runtime_capacity": True,
        "bl31_first_source_window": {
            "start": f"0x{DRAM_BASE:08x}", "end_exclusive": f"0x{DRAM_BASE + WINDOW_BYTES:08x}",
            "bytes": WINDOW_BYTES,
        },
        "bl31_second_source_window_candidate": {
            "formula": "DRAM_BASE + total_DRAM_capacity / 2",
            "start": f"0x{midpoint:08x}", "end_exclusive": f"0x{midpoint + WINDOW_BYTES:08x}",
            "bytes": WINDOW_BYTES, "status": "ADDRESS_DERIVED_NOT_RUNTIME_BRANCH_CONFIRMED",
        },
        "runtime_context_selector_captured": False, "bl31_resume_gate_observed": False,
        "save_restore_execution_observed": False, "memory_layout_approved": False,
        "hardware_access_performed": False,
        "limitations": [
            "Boot0 primary parameter blocks are candidate/training metadata; captures do not prove which copy or branch supplied live training state.",
            "The 672 MHz field is a Boot0 parameter value, not an independently measured live DRAM clock.",
            "Linux memblock is post-boot evidence and cannot reconstruct all earlier BL31/SPL ownership intervals.",
            "The midpoint address follows the matched BL31 static formula and 1 GiB runtime capacity; it does not prove BL31 selected or copied that window.",
            "No DRAM contents, MMIO, SMC, TWI, suspend/resume, or device state were accessed by this audit.",
        ],
    }

def main() -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--primary", type=Path, required=True)
    p.add_argument("--secondary", type=Path, required=True)
    p.add_argument("--memory-map", type=Path, required=True)
    p.add_argument("--out-dir", type=Path, required=True)
    a = p.parse_args()
    result = build(json.loads(a.primary.read_text()), json.loads(a.secondary.read_text()),
                   json.loads(a.memory_map.read_text()))
    a.out_dir.mkdir(parents=True, exist_ok=True)
    (a.out_dir / "BOOT0-DRAM-GEOMETRY.json").write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    g = result["boot0_copies"][0]
    w = result["bl31_second_source_window_candidate"]
    lines = [
        "# Boot0 DRAM geometry cross-check", "",
        "- Both captured Boot0 copies agree on the LPDDR4 primary geometry fields.",
        f"- Decoded geometry: {g['rows']} rows, {g['columns']} columns, {g['ranks']} rank, {g['bus_width_bits']}-bit bus.",
        f"- Capacity: {g['capacity_mib']} MiB; independently matches Linux memblock [0x40000000, 0x80000000).",
        f"- BL31 midpoint source-window candidate: [{w['start']}, {w['end_exclusive']}).",
        "- Candidate address is derived, not proof of runtime selector/branch execution.",
        "- H700_MEMORY_LAYOUT_APPROVED=false; no hardware access or payload execution.",
    ]
    (a.out_dir / "BOOT0-DRAM-GEOMETRY.md").write_text("\n".join(lines) + "\n")
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
