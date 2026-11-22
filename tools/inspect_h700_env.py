#!/usr/bin/env python3
"""Inspect an Allwinner H700 U-Boot environment image without modifying it.

The input must be a complete raw env partition/image, not a short prefix.
The tool reports variables even if the checksum layout is not recognized, but
never labels the environment active/valid unless a stored CRC32 matches.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import struct
import sys
import zlib
from pathlib import Path
from typing import Any

SECTOR_BYTES = 512
HEADER_BYTES = 4
FLAGS_BYTES = 1
KEY_RE = re.compile(rb"^[A-Za-z0-9_.+-]+$")


class EnvError(ValueError):
    """Malformed U-Boot environment image."""


def _parse_variables(data: bytes, offset: int) -> tuple[list[dict[str, str]], list[str]]:
    payload = data[offset:]
    end = payload.find(b"\0\0")
    if end >= 0:
        payload = payload[:end]
    tokens = [token for token in payload.split(b"\0") if token]
    variables: list[dict[str, str]] = []
    warnings: list[str] = []
    seen: set[str] = set()

    for token in tokens:
        if b"=" not in token:
            warnings.append(f"ignored non key=value token at data offset {offset}: {token[:32]!r}")
            continue
        key_raw, value_raw = token.split(b"=", 1)
        if not KEY_RE.fullmatch(key_raw):
            warnings.append(f"ignored variable with invalid key bytes: {key_raw[:32]!r}")
            continue
        key = key_raw.decode("ascii")
        value = value_raw.decode("utf-8", "replace")
        if key in seen:
            warnings.append(f"duplicate variable name: {key}")
        seen.add(key)
        variables.append({"name": key, "value": value})

    if end < 0:
        warnings.append("double-NUL environment terminator was not found")
    return variables, warnings


def _crc_matches(data: bytes, stored_crc: int) -> list[dict[str, Any]]:
    matches: list[dict[str, Any]] = []
    # Allwinner/U-Boot builds use different CONFIG_ENV_SIZE values. Scan
    # sector-aligned end offsets so a valid 128 KiB or 16 MiB image can be
    # recognized without assuming the enclosing GPT partition equals ENV_SIZE.
    for data_offset in (HEADER_BYTES, HEADER_BYTES + FLAGS_BYTES):
        crc = 0
        position = data_offset
        end = SECTOR_BYTES
        while end <= len(data):
            if end > position:
                crc = zlib.crc32(data[position:end], crc)
                position = end
                if crc == stored_crc:
                    matches.append({
                        "data_offset": data_offset,
                        "env_image_bytes": end,
                        "data_bytes": end - data_offset,
                        "crc32": f"0x{crc:08x}",
                    })
            end += SECTOR_BYTES

        if position < len(data):
            crc = zlib.crc32(data[position:], crc)
            if crc == stored_crc:
                matches.append({
                    "data_offset": data_offset,
                    "env_image_bytes": len(data),
                    "data_bytes": len(data) - data_offset,
                    "crc32": f"0x{crc:08x}",
                })

    return matches


def inspect_env(data: bytes, source: str = "") -> dict[str, Any]:
    if len(data) < HEADER_BYTES + 2:
        raise EnvError("input is too short to contain an environment header")
    stored_crc = struct.unpack_from("<I", data, 0)[0]

    matches = _crc_matches(data, stored_crc)

    # Allwinner redundant env commonly stores a flags byte after the CRC.
    # When a CRC match exists, use the matching data offset as stronger
    # evidence than byte-shape heuristics. Otherwise report a parse candidate.
    offset4_vars, offset4_warnings = _parse_variables(data, 4)
    offset5_vars, offset5_warnings = _parse_variables(data, 5)
    flag = data[4]
    matched_offsets = {item["data_offset"] for item in matches}
    if len(matched_offsets) == 1:
        data_offset = next(iter(matched_offsets))
    elif flag in (0, 1) and len(offset5_vars) >= len(offset4_vars):
        data_offset = 5
    else:
        data_offset = 4

    if data_offset == 5:
        variables = offset5_vars
        parse_warnings = offset5_warnings
        header_format = (
            "CRC32 + flags byte + NUL-separated variables (CRC matched)"
            if 5 in matched_offsets else
            "CRC32 + flags byte + NUL-separated variables (candidate)"
        )
    else:
        variables = offset4_vars
        parse_warnings = offset4_warnings
        header_format = (
            "CRC32 + NUL-separated variables (CRC matched)"
            if 4 in matched_offsets else
            "CRC32 + NUL-separated variables (candidate)"
        )

    return {
        "format": "Allwinner/U-Boot environment (structural candidate)",
        "source": source,
        "file_bytes": len(data),
        "file_sha256": hashlib.sha256(data).hexdigest(),
        "stored_crc32_le": f"0x{stored_crc:08x}",
        "crc_valid": bool(matches),
        "crc_matches": matches,
        "crc_note": (
            "CRC32 matched a sector-aligned environment size and header offset."
            if matches else
            "No CRC32 match was found for data offsets 4/5 at sector-aligned image sizes. "
            "Do not treat the extracted variables as a verified active U-Boot environment."
        ),
        "header_format_candidate": header_format,
        "flags_byte_candidate": flag,
        "variables_data_offset": data_offset,
        "variable_count": len(variables),
        "variables": variables,
        "parse_warnings": parse_warnings,
        "limitations": [
            "A valid CRC proves only that this byte range is internally consistent; it does not prove U-Boot selected this copy at boot.",
            "A valid environment dump does not prove the runtime bootcmd was not modified by board-specific startup code.",
            "Environment parsing is read-only and never writes or repairs the source image.",
        ],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("env_image", type=Path, help="complete raw U-Boot env partition/image")
    parser.add_argument("-o", "--output", type=Path, help="optional JSON report path")
    args = parser.parse_args()

    try:
        blob = args.env_image.read_bytes()
        report = inspect_env(blob, str(args.env_image))
    except (OSError, EnvError, struct.error) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    rendered = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    else:
        print(rendered, end="")

    crc_state = "CRC OK" if report["crc_valid"] else "CRC NOT VERIFIED"
    print(
        f"{crc_state}: {report['variable_count']} variables, "
        f"stored CRC {report['stored_crc32_le']}, image {report['file_bytes']} bytes",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
