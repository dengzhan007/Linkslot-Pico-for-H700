#!/usr/bin/env python3
"""Validate the published read-only RG34XX Boot0 captures against the parser."""

from __future__ import annotations

import argparse
import hashlib
import subprocess
import sys

from inspect_h700_boot0 import parse_boot0


REPORT_PATH = "reports/rg34xx-inventory-20261127-uart"
EXPECTED = {
    "boot0-primary.bin": {
        "capture_sha": "bf886995745a0eb9b5030296f9f6163035e164b3a82183716e67ff876319289f",
        "image_sha": "bf886995745a0eb9b5030296f9f6163035e164b3a82183716e67ff876319289f",
    },
    "boot0-secondary.bin": {
        "capture_sha": "5f143679e0d2c5293d22fee47a47ea6c17120c7a19e6007247c8b13710aa5ef9",
        "image_sha": "4701a85e4db7e8339b91b92f691a83cc954ec63a09369087c2e41d80eafa9f5d",
    },
}


def read_git_blob(ref: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{ref}:{path}"],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    return result.stdout


def find_table(report: dict, marker_offset: str) -> dict:
    for table in report["dram_ext_tables"]:
        if table["marker_offset"] == marker_offset:
            return table
    raise AssertionError(f"missing DRAM.ext table marker {marker_offset}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--reports-ref",
        default="refs/remotes/origin/reports",
        help="git ref containing the reports branch",
    )
    args = parser.parse_args()

    reports = {}
    for filename, expected in EXPECTED.items():
        path = f"{REPORT_PATH}/{filename}"
        data = read_git_blob(args.reports_ref, path)
        capture_sha = hashlib.sha256(data).hexdigest()
        if capture_sha != expected["capture_sha"]:
            raise AssertionError(
                f"{filename}: raw-capture SHA-256 {capture_sha} != "
                f"{expected['capture_sha']}"
            )
        report = parse_boot0(data)
        if report["sha256_full"] != expected["image_sha"]:
            raise AssertionError(
                f"{filename}: declared-image SHA-256 {report['sha256_full']} != "
                f"{expected['image_sha']}"
            )
        reports[filename] = report

    primary = reports["boot0-primary.bin"]
    secondary = reports["boot0-secondary.bin"]

    assert primary["bytes"] == 0x20000, primary["bytes"]
    assert secondary["bytes"] == 0x10000, secondary["bytes"]
    for name, report in reports.items():
        active = report["dram_candidates"][0]
        assert active["dram_type"] == "LPDDR4", (name, active)
        assert active["clock_mhz_from_word0"] == 672, (name, active)

    primary_relocated = find_table(primary, "0x1348")
    secondary_table = find_table(secondary, "0x348")
    assert primary_relocated["valid_entry_count"] == 15
    assert secondary_table["valid_entry_count"] == 15
    for name, table in (
        ("primary relocated DRAM.ext", primary_relocated),
        ("secondary DRAM.ext", secondary_table),
    ):
        assert table["entries"][13]["dram_type"] == "LPDDR3", (name, table)
        assert table["entries"][14]["dram_type"] == "LPDDR4", (name, table)
        assert table["entries"][13]["clock_mhz"] == 672, (name, table)
        assert table["entries"][14]["clock_mhz"] == 672, (name, table)

    print("PASS: both published RG34XX Boot0 raw captures and declared images pass SHA-256 and eGON checksum checks")
    print("PASS: active header parameter block in both copies reports LPDDR4 at 672 MHz")
    print("PASS: primary Boot0 DRAM.ext table is discovered at 0x1348 (15 profiles)")
    print("PASS: secondary Boot0 DRAM.ext table is discovered at 0x348 (15 profiles)")
    print("LIMIT: profile-table extraction does not prove runtime selection, PMIC writes, or DRAM map")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (AssertionError, subprocess.CalledProcessError) as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
