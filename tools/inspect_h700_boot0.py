#!/usr/bin/env python3
"""Inspect the RG34XX H700 eGON.BT0 Boot0 image without modifying it."""

from __future__ import annotations

import argparse
import hashlib
import json
import struct
import sys
from pathlib import Path
from typing import Any

BOOT0_BYTES = 0x10000
CHECKSUM_OFFSET = 0x0C
CHECKSUM_STAMP = 0x5F0A6C39
IMAGE_SIZE_OFFSET = 0x10
DEBUG_OFFSET = 0x34
PARAMETER_WORDS = 32
CANDIDATES = (
    ("primary", 0x38, "primary block in mutable first sector"),
    ("lpddr3_alternate", 0x9F4, "LPDDR3 alternate"),
    ("lpddr4_alternate", 0xA74, "LPDDR4 alternate"),
)


class Boot0Error(ValueError):
    """Raised when a Boot0 image fails structural or checksum validation."""


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def allwinner_checksum(data: bytes, checksum_offset: int = CHECKSUM_OFFSET) -> int:
    if len(data) % 4 or checksum_offset % 4:
        raise Boot0Error("Boot0 length/checksum offset is not word-aligned")
    words = struct.unpack(f"<{len(data) // 4}I", data)
    return (sum(words) - words[checksum_offset // 4] + CHECKSUM_STAMP) & 0xFFFFFFFF


def parse_boot0(data: bytes) -> dict[str, Any]:
    if len(data) != BOOT0_BYTES:
        raise Boot0Error(
            f"expected a {BOOT0_BYTES}-byte Boot0 image, received {len(data)} bytes"
        )
    if data[4:12] != b"eGON.BT0":
        raise Boot0Error("missing eGON.BT0 signature")

    image_size = struct.unpack_from("<I", data, IMAGE_SIZE_OFFSET)[0]
    if image_size != BOOT0_BYTES:
        raise Boot0Error(
            f"Boot0 header declares 0x{image_size:x} bytes, expected 0x{BOOT0_BYTES:x}"
        )

    stored_checksum = struct.unpack_from("<I", data, CHECKSUM_OFFSET)[0]
    calculated_checksum = allwinner_checksum(data)
    if stored_checksum != calculated_checksum:
        raise Boot0Error(
            f"Boot0 checksum mismatch: stored 0x{stored_checksum:08x}, "
            f"calculated 0x{calculated_checksum:08x}"
        )

    candidates = []
    for name, offset, description in CANDIDATES:
        end = offset + PARAMETER_WORDS * 4
        if end > len(data):
            raise Boot0Error(f"{name} DRAM parameter block exceeds Boot0")
        words = list(struct.unpack_from(f"<{PARAMETER_WORDS}I", data, offset))
        dram_type = words[1]
        candidates.append(
            {
                "name": name,
                "offset": f"0x{offset:x}",
                "description": description,
                "dram_type_id": dram_type,
                "dram_type": {7: "LPDDR3", 8: "LPDDR4"}.get(dram_type, "unknown"),
                "clock_mhz_from_word0": words[0],
                "words": [f"0x{word:08x}" for word in words],
            }
        )

    return {
        "format": "Allwinner eGON.BT0",
        "bytes": len(data),
        "image_size": image_size,
        "stored_checksum": f"0x{stored_checksum:08x}",
        "checksum_valid": True,
        "debug_byte": data[DEBUG_OFFSET],
        "sha256_full": sha256(data),
        "sha256_first_512_bytes": sha256(data[:512]),
        "sha256_body_after_first_sector": sha256(data[512:]),
        "dram_candidates": candidates,
        "interpretation_notes": [
            "The primary parameter block at 0x38 lies in the first 512-byte sector and may be rewritten by Boot0 during DRAM training.",
            "The body hash intentionally excludes that mutable first sector; it is not a substitute for validating the live checksum.",
            "Candidate words are reported raw. Training success, selected memory geometry and a safe DRAM map require separate runtime evidence.",
        ],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("boot0", type=Path, help="64 KiB Boot0 copy from the inventory collector")
    parser.add_argument("-o", "--output", type=Path, help="optional JSON output path")
    args = parser.parse_args()

    try:
        report = parse_boot0(args.boot0.read_bytes())
    except (OSError, Boot0Error) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    encoded = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.write_text(encoded, encoding="utf-8")
    else:
        print(encoded, end="")
    print(
        f"Boot0 OK: {report['sha256_full']} "
        f"(stable body {report['sha256_body_after_first_sector']})",
        file=sys.stderr,
    )
    for candidate in report["dram_candidates"]:
        print(
            f"{candidate['name']:<18} {candidate['dram_type']:<8} "
            f"{candidate['clock_mhz_from_word0']} MHz  {candidate['offset']}",
            file=sys.stderr,
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
