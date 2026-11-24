#!/usr/bin/env python3
"""Decode narrowly captured sun50iw9p1 SID cells without approving an OPP."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any


SPEED_RE = re.compile(r"^speed_bin_raw_le_bytes=([0-9a-fA-F]+)\s*$", re.MULTILINE)
THERMAL_RE = re.compile(r"^thermal_calibration_raw_bytes=([0-9a-fA-F]+)\s*$", re.MULTILINE)

SOC_BLOCK_MARKER = "== generic SoC identity (read-only sysfs attributes; no serial/chipid) =="
SOC_ATTRIBUTE_RE = re.compile(
    r"^\[(machine|family|soc_id|revision)\]\s*\n([^\r\n]*)",
    re.MULTILINE,
)


def _generic_soc_identity(text: str) -> dict[str, Any]:
    start = text.find(SOC_BLOCK_MARKER)
    if start < 0:
        return {
            "status": "unavailable",
            "reason": "The collector did not emit the generic SoC identity block.",
            "attributes": {},
            "opp_authorization": "not_approved",
        }

    block_end = text.find("\n== ", start + len(SOC_BLOCK_MARKER))
    block = text[start:] if block_end < 0 else text[start:block_end]
    attributes: dict[str, list[str]] = {}
    for name, value in SOC_ATTRIBUTE_RE.findall(block):
        attributes.setdefault(name, []).append(value.strip())

    status = "captured" if attributes else "unavailable"
    return {
        "status": status,
        "attributes": attributes,
        "opp_authorization": "not_approved",
        "note": (
            "These are generic SoC bus sysfs strings only. They are not substituted "
            "for the vendor sunxi_get_soc_ver() result and do not certify a speed bin "
            "or authorize any OPP."
        ),
    }



# These literals appear in the pinned vendor cpufreq consumer. They are
# reported only as reference matches: that source reads a 2-byte NVMEM cell
# through a u32 pointer, while the pinned NVMEM core allocates exactly cell->bytes.
# Never derive a usable supported_hw mask or authorize an OPP from this table.
VENDOR_SPEED_LITERALS = {
    0x2400, 0x5000, 0x5400, 0x7400, 0x2C00, 0x7C00, 0x5C00,
}


# Diagnostic-only reference decoder from Linux sun50i-cpufreq-nvmem.c
# pinned at torvalds/linux commit 8d7fe9885c62fc9a95bf55d9db244d9672a8aadc.
# Generic /sys/devices/soc0/revision is NOT equivalent to ARM SMCCC
# ARCH_SOC_ID revision and must never be supplied to the ambiguous cases.
MAINLINE_H616_DIRECT_BINS = {
    0x2000: 0,
    0x5000: 3,
    0x5400: 3,
    0x6000: 3,
    0x5C00: 4,
    0x5D00: 0,
    0x6C00: 5,
}
MAINLINE_H616_SMCCC_REVISION_BINS = {
    0x2400: (1, 2),
    0x7400: (1, 2),
    0x2C00: (1, 2),
    0x7C00: (1, 2),
}


def _mainline_reference_speed_decode(raw_u16: int | None) -> dict[str, Any]:
    if raw_u16 is None:
        return {
            "status": "unavailable",
            "candidate_bin": None,
            "reason": "No raw speed-bin cell was captured.",
            "requires_smccc_arch_soc_id_revision": False,
            "opp_authorization": "not_approved",
        }

    if raw_u16 in MAINLINE_H616_DIRECT_BINS:
        return {
            "status": "recognized_reference_value",
            "candidate_bin": MAINLINE_H616_DIRECT_BINS[raw_u16],
            "raw_u16": f"0x{raw_u16:04x}",
            "requires_smccc_arch_soc_id_revision": False,
            "opp_authorization": "not_approved",
            "warning": "This is a Linux mainline reference classification only, not an RG34XX OPP approval.",
        }

    if raw_u16 in MAINLINE_H616_SMCCC_REVISION_BINS:
        bins = MAINLINE_H616_SMCCC_REVISION_BINS[raw_u16]
        return {
            "status": "recognized_but_revision_ambiguous",
            "candidate_bin": None,
            "candidate_bins": list(bins),
            "raw_u16": f"0x{raw_u16:04x}",
            "requires_smccc_arch_soc_id_revision": True,
            "revision_rule": "Upstream selects bin 1 when SMCCC is supported and revision <= 1; it selects bin 2 for later revisions and also when SMCCC is not supported. This project deliberately keeps the value ambiguous when a trustworthy SMCCC revision is unavailable.",
            "generic_soc_sysfs_revision_accepted": False,
            "opp_authorization": "not_approved",
        }

    return {
        "status": "unsupported_reference_value",
        "candidate_bin": None,
        "raw_u16": f"0x{raw_u16:04x}",
        "requires_smccc_arch_soc_id_revision": False,
        "opp_authorization": "not_approved",
        "reason": "Unknown values are not assigned the mainline driver's permissive default bin.",
    }


class SidCellError(ValueError):
    """Malformed or inconsistent SID cell evidence."""


def _one_hex_cell(pattern: re.Pattern[str], text: str, label: str, size: int) -> bytes | None:
    matches = pattern.findall(text)
    if not matches:
        return None
    if len(matches) != 1:
        raise SidCellError(f"expected one {label} record, found {len(matches)}")
    raw_hex = matches[0]
    if len(raw_hex) != size * 2:
        raise SidCellError(
            f"{label} must contain exactly {size} bytes ({size * 2} hex digits); "
            f"got {len(raw_hex)} hex digits"
        )
    try:
        return bytes.fromhex(raw_hex)
    except ValueError as exc:
        raise SidCellError(f"{label} is not valid hexadecimal") from exc


def inspect_sid_cells(path: Path) -> dict[str, Any]:
    if not path.is_file():
        raise SidCellError(f"CPU power inventory file not found: {path}")

    text = path.read_text(encoding="utf-8", errors="replace")
    source_bytes = path.read_bytes()

    speed = _one_hex_cell(SPEED_RE, text, "speed-bin cell", 2)
    thermal = _one_hex_cell(THERMAL_RE, text, "thermal-calibration cell", 8)

    if speed is None:
        speed_report: dict[str, Any] = {
            "status": "unavailable",
            "reason": (
                "The exact sun50iw9p1 SID provider/cell was not readable or "
                "was not exported by the running kernel."
            ),
            "raw_bytes_hex": None,
            "raw_u16_le": None,
            "matches_pinned_vendor_literal": None,
            "opp_authorization": "not_approved",
            "mainline_reference_decode": _mainline_reference_speed_decode(None),
        }
    else:
        raw_u16 = int.from_bytes(speed, byteorder="little", signed=False)
        speed_report = {
            "status": "captured",
            "raw_bytes_hex": speed.hex(),
            "raw_u16_le": f"0x{raw_u16:04x}",
            "matches_pinned_vendor_literal": raw_u16 in VENDOR_SPEED_LITERALS,
            "pinned_vendor_literal_set": [f"0x{x:04x}" for x in sorted(VENDOR_SPEED_LITERALS)],
            "opp_authorization": "not_approved",
            "mainline_reference_decode": _mainline_reference_speed_decode(raw_u16),
            "warning": (
                "Do not use the pinned vendor cpufreq raw-value switch as an OPP "
                "approval: its consumer casts the NVMEM result to u32* even though "
                "the DTS cell is 2 bytes and the pinned NVMEM core allocates exactly "
                "cell->bytes. Silicon-revision semantics also remain unverified."
            ),
        }

    if thermal is None:
        thermal_report: dict[str, Any] = {
            "status": "unavailable",
            "reason": "The exact sun50iw9p1 thermal calibration cell was not readable.",
            "raw_bytes_hex": None,
            "u16_le_words": None,
            "calibration_status": "unknown",
        }
    else:
        words = [
            int.from_bytes(thermal[offset:offset + 2], byteorder="little", signed=False)
            for offset in range(0, 8, 2)
        ]
        sensor3 = (
            ((words[1] >> 12) & 0xF)
            | (((words[2] >> 12) & 0xF) << 4)
            | (((words[3] >> 12) & 0xF) << 8)
        )
        thermal_report = {
            "status": "captured",
            "raw_bytes_hex": thermal.hex(),
            "u16_le_words": [f"0x{x:04x}" for x in words],
            "factory_test_temperature_raw_0p1c": words[0] & 0x0FFF,
            "sensor0_calibration_raw_12bit": words[1] & 0x0FFF,
            "sensor1_calibration_raw_12bit": words[2] & 0x0FFF,
            "sensor2_calibration_raw_12bit": words[3] & 0x0FFF,
            "sensor3_calibration_raw_12bit": sensor3,
            "calibration_status": "not_certified",
            "warning": (
                "The field layout mirrors the pinned sunxi-thermal-ng source "
                "for sun50iw9p1. Raw fields do not prove that AVCC/thermal "
                "calibration was accepted, that live temperatures are valid, "
                "or that closed-loop DVFS is safe."
            ),
        }

    return {
        "source_file": path.name,
        "source_sha256": hashlib.sha256(source_bytes).hexdigest(),
        "sid_provider": "sunxi-sid-ng@03006000",
        "generic_soc_identity": _generic_soc_identity(text),
        "cells": {
            "speed": {
                "dts_provider_offset": "0x0",
                "length_bytes": 2,
                "report": speed_report,
            },
            "thermal_calibration": {
                "dts_provider_offset": "0x14",
                "length_bytes": 8,
                "report": thermal_report,
            },
        },
        "limitations": [
            "This report decodes only the two bounded NVMEM cells captured by the read-only inventory collector.",
            "The speed-bin value is raw evidence only. No OPP mask is inferred and no operating point is approved.",
            "The thermal calibration fields are diagnostic only and do not certify a calibrated sensor or temperature limit.",
            "No CPU clock, PMIC, regulator, PLL or thermal control register was read or written by this parser.",
        ],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("cpu_power_runtime", type=Path)
    parser.add_argument("-o", "--output", type=Path)
    args = parser.parse_args()

    try:
        report = inspect_sid_cells(args.cpu_power_runtime)
    except (OSError, SidCellError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    output = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(output, encoding="utf-8")
    else:
        sys.stdout.write(output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
