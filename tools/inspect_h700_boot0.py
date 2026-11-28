#!/usr/bin/env python3
"""Inspect an RG34XX H700 eGON.BT0 image without modifying it.

The fixed-offset candidates are retained for compatibility with older reports.
A second scanner discovers DRAM.ext parameter tables at any image offset; this
is necessary because the captured 128 KiB primary Boot0 and 64 KiB secondary
Boot0 do not place their full profile table at the same offset.
"""

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
DRAM_EXT_MAGIC = b"DRAM.ext"
DRAM_EXT_PARAMETER_DELTA = 44
DRAM_EXT_PROFILE_COUNT = 15
DRAM_EXT_PROFILE_BYTES = PARAMETER_WORDS * 4

# H616/H700 parameter layout documented by apritzel/sunxi-fw's H616/H700
# parser. These are parameter names, not direct MMIO register assignments.
H616_H700_FIELDS = (
    "dram_clk", "dram_type", "dram_dx_odt", "dram_dx_dri", "dram_ca_dri",
    "dram_para0", "dram_para1", "dram_para2",
    "dram_mr0", "dram_mr1", "dram_mr2", "dram_mr3", "dram_mr4", "dram_mr5",
    "dram_mr6", "dram_mr11", "dram_mr12", "dram_mr13", "dram_mr14",
    "dram_mr16", "dram_mr17", "dram_mr22",
    "dram_tpr0", "dram_tpr1", "dram_tpr2", "dram_tpr3", "dram_tpr6",
    "dram_tpr10", "dram_tpr11", "dram_tpr12", "dram_tpr13", "dram_tpr14",
)
DRAM_TYPE_NAMES = {2: "DDR2", 3: "DDR3", 4: "DDR4", 6: "LPDDR2", 7: "LPDDR3", 8: "LPDDR4"}

CANDIDATES = (
    ("primary", 0x38, "primary block in mutable first sector"),
    ("lpddr3_alternate", 0x9F4, "legacy fixed-offset LPDDR3 alternate"),
    ("lpddr4_alternate", 0xA74, "legacy fixed-offset LPDDR4 alternate"),
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


def decode_h616_h700_profile(words: list[int], offset: int) -> dict[str, Any]:
    named = {
        name: (words[index] if index == 0 else f"0x{words[index]:08x}")
        for index, name in enumerate(H616_H700_FIELDS)
    }
    dram_type = words[1]
    return {
        "offset": f"0x{offset:x}",
        "dram_type_id": dram_type,
        "dram_type": DRAM_TYPE_NAMES.get(dram_type, "unknown"),
        "clock_mhz": words[0],
        "named_fields": named,
        "words": [f"0x{word:08x}" for word in words],
    }


def is_plausible_h616_h700_profile(words: list[int]) -> bool:
    """Conservative structure filter; does not establish runtime selection."""
    if len(words) != PARAMETER_WORDS:
        return False
    if not 100 <= words[0] <= 1200:
        return False
    if words[1] not in DRAM_TYPE_NAMES:
        return False
    if words[2] & 0xF0F0F0F0:
        return False
    if words[3] & 0xF0F0F0F0:
        return False
    return True


def scan_dram_ext_tables(data: bytes) -> list[dict[str, Any]]:
    """Find DRAM.ext markers and report contiguous plausible 32-word profiles.

    The 44-byte offset and 15-slot count follow the DRAM.ext constants in
    apritzel/sunxi-fw main; the 32-word field layout follows its H616/H700
    parser revision. A marker can be a string
    or embedded structure, so a table is reported only when its following
    records pass conservative field-shape checks. The result is evidence of
    embedded parameter data, not proof that Boot0 selected or consumed it.
    """
    tables: list[dict[str, Any]] = []
    cursor = 0
    while True:
        marker = data.find(DRAM_EXT_MAGIC, cursor)
        if marker < 0:
            break
        cursor = marker + 1
        parameter_start = marker + DRAM_EXT_PARAMETER_DELTA
        entries: list[dict[str, Any]] = []
        stopped_by_invalid = False
        for index in range(DRAM_EXT_PROFILE_COUNT):
            offset = parameter_start + index * DRAM_EXT_PROFILE_BYTES
            end = offset + DRAM_EXT_PROFILE_BYTES
            if end > len(data):
                stopped_by_invalid = True
                break
            words = list(struct.unpack_from(f"<{PARAMETER_WORDS}I", data, offset))
            if not is_plausible_h616_h700_profile(words):
                stopped_by_invalid = True
                break
            entry = decode_h616_h700_profile(words, offset)
            entry["index"] = index
            entries.append(entry)
        if entries:
            tables.append({
                "marker_offset": f"0x{marker:x}",
                "parameter_offset": f"0x{parameter_start:x}",
                "valid_entry_count": len(entries),
                "stopped_at_invalid_or_end": stopped_by_invalid,
                "entries": entries,
                "interpretation": (
                    "Contiguous structurally plausible H616/H700 32-word profiles; "
                    "runtime selection and use are not established by this scan."
                ),
            })
    return tables



def _read_c_string(data: bytes, offset: int, limit: int = 256) -> str | None:
    if offset < 0 or offset >= len(data):
        return None
    end = data.find(b"\0", offset, min(len(data), offset + limit))
    if end < 0:
        end = min(len(data), offset + limit)
    raw = data[offset:end]
    if not raw:
        return None
    return raw.decode("ascii", errors="replace")


def scan_embedded_egon_images(data: bytes) -> list[dict[str, Any]]:
    """Describe aligned nested eGON headers without assuming every header is Boot0.

    The RG34XX primary capture contains an outer 128 KiB eGON image, an inner
    64 KiB eGON Boot0 image at +0x1000, and a nonstandard SPL2 component at
    +0x11000. The SPL2 component reuses the eGON.BT0 magic but has a distinct
    header marker and platform string, so the standard Boot0 checksum rule is
    deliberately not applied to it.
    """
    components: list[dict[str, Any]] = []
    cursor = 0
    while True:
        marker = data.find(b"eGON.BT0", cursor)
        if marker < 4:
            break
        cursor = marker + 1
        offset = marker - 4
        if offset % 4:
            continue
        if offset + 0x30 > len(data):
            continue

        declared_size = struct.unpack_from("<I", data, offset + IMAGE_SIZE_OFFSET)[0]
        header_tag = data[offset + 0x14:offset + 0x18]
        stored_checksum = struct.unpack_from("<I", data, offset + CHECKSUM_OFFSET)[0]
        word_7 = struct.unpack_from("<I", data, offset + 0x1C)[0]
        word_8 = struct.unpack_from("<I", data, offset + 0x20)[0]
        in_bounds = (
            declared_size >= 0x30
            and declared_size % 4 == 0
            and offset + declared_size <= len(data)
        )
        if not in_bounds:
            continue

        component_data = data[offset:offset + declared_size]
        is_standard_boot0 = header_tag == b"\0\0\0\0" and declared_size >= BOOT0_BYTES
        checksum_valid: bool | None = None
        if is_standard_boot0:
            checksum_valid = allwinner_checksum(component_data) == stored_checksum

        active_profile = None
        if is_standard_boot0 and 0x38 + PARAMETER_WORDS * 4 <= declared_size:
            words = list(struct.unpack_from("<32I", component_data, 0x38))
            if is_plausible_h616_h700_profile(words):
                active_profile = decode_h616_h700_profile(words, 0x38)

        tables = scan_dram_ext_tables(component_data)
        for table in tables:
            table["marker_offset_absolute"] = f"0x{offset + int(table['marker_offset'], 16):x}"
            table["parameter_offset_absolute"] = f"0x{offset + int(table['parameter_offset'], 16):x}"
            for entry in table["entries"]:
                entry["offset_absolute"] = f"0x{offset + int(entry['offset'], 16):x}"

        is_spl2 = header_tag.startswith(b"SPL")
        platform_string = _read_c_string(data, offset + word_8) if is_spl2 and word_8 < declared_size else None
        component: dict[str, Any] = {
            "offset": f"0x{offset:x}",
            "magic": "eGON.BT0",
            "header_tag_hex": header_tag.hex(),
            "header_tag_ascii": header_tag.decode("ascii", errors="replace"),
            "declared_size": declared_size,
            "stored_checksum": f"0x{stored_checksum:08x}",
            "checksum_valid_standard_egon": checksum_valid,
            "sha256_declared_component": sha256(component_data),
            "component_kind": "eGON Boot0" if is_standard_boot0 else ("SPL2 component" if is_spl2 else "nonstandard eGON-tagged component"),
            "word_7": f"0x{word_7:08x}",
            "word_8": f"0x{word_8:08x}",
            "platform_string": platform_string,
            "active_dram_profile": active_profile,
            "dram_ext_tables": tables,
        }
        if is_spl2:
            component["checksum_note"] = "Nonstandard SPL2 header; standard eGON Boot0 checksum is not asserted."
            component["platform_string_note"] = "Build/platform label only; it does not by itself prove hardware incompatibility or runtime use."
        components.append(component)
    return components


def parse_boot0(data: bytes) -> dict[str, Any]:
    captured_bytes = len(data)
    if captured_bytes < BOOT0_BYTES:
        raise Boot0Error(
            f"expected a {BOOT0_BYTES}-byte Boot0 image or larger, "
            f"received {captured_bytes} bytes"
        )
    if data[4:12] != b"eGON.BT0":
        raise Boot0Error("missing eGON.BT0 signature")

    image_size = struct.unpack_from("<I", data, IMAGE_SIZE_OFFSET)[0]
    if (
        image_size < BOOT0_BYTES
        or image_size > captured_bytes
        or image_size % 512
    ):
        raise Boot0Error(
            f"Boot0 header declares 0x{image_size:x} bytes, expected a "
            f"512-byte-aligned size between 0x{BOOT0_BYTES:x} and "
            f"0x{captured_bytes:x}"
        )
    data = data[:image_size]

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
        candidate = {
            "name": name,
            "offset": f"0x{offset:x}",
            "description": description,
            "dram_type_id": dram_type,
            "dram_type": DRAM_TYPE_NAMES.get(dram_type, "unknown"),
            "clock_mhz_from_word0": words[0],
            "words": [f"0x{word:08x}" for word in words],
        }
        if len(words) == len(H616_H700_FIELDS):
            candidate["named_fields"] = decode_h616_h700_profile(words, offset)["named_fields"]
        candidates.append(candidate)

    return {
        "format": "Allwinner eGON.BT0",
        "bytes": len(data),
        "captured_bytes": captured_bytes,
        "image_size": image_size,
        "stored_checksum": f"0x{stored_checksum:08x}",
        "checksum_valid": True,
        "debug_byte": data[DEBUG_OFFSET],
        "sha256_full": sha256(data),
        "sha256_first_512_bytes": sha256(data[:512]),
        "sha256_body_after_first_sector": sha256(data[512:]),
        "dram_candidates": candidates,
        "dram_ext_tables": scan_dram_ext_tables(data),
        "embedded_egon_components": scan_embedded_egon_images(data),
        "interpretation_notes": [
            "The primary parameter block at 0x38 lies in the first 512-byte sector and may be rewritten by Boot0 during DRAM training.",
            "The body hash intentionally excludes that mutable first sector; it is not a substitute for validating the live checksum.",
            "Legacy fixed-offset candidates are retained for compatibility but are not exhaustive across Boot0 layouts.",
            "DRAM.ext markers are scanned across the declared image; only contiguous records passing conservative H616/H700 structural checks are reported.",
            "DRAM.ext profile extraction does not prove which profile Boot0 selected, whether a marker is consumed on this boot path, or the physical DRAM map.",
            "Aligned nested eGON headers are reported as separate components; nonstandard SPL2 headers are not validated with the standard Boot0 checksum rule.",
            "An embedded platform string such as rg35xx-2024 is a build label and is not, by itself, proof that the component is incompatible with an RG34XX board.",
        ],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "boot0", type=Path, help="Boot0 copy from the inventory collector (64 KiB or larger)"
    )
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
    for table in report["dram_ext_tables"]:
        print(
            f"DRAM.ext @ {table['marker_offset']}: "
            f"{table['valid_entry_count']} plausible H616/H700 profiles",
            file=sys.stderr,
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
