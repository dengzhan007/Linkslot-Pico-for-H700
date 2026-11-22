#!/usr/bin/env python3
"""Parse and extract the Allwinner H700 TOC1 package used by RG34XX."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import struct
import sys
from pathlib import Path
from typing import Any

HEADER_BYTES = 64
ITEM_BYTES = 368
ITEM_NAME_BYTES = 64
ITEM_COUNT = 4
TOC1_BYTES = 0x140000
TOC1_MAGIC = b"sunxi-package\0\0\0"
TOC1_MAGIC_WORD = 0x89119800
CHECKSUM_STAMP = 0x5F0A6C39
ITEM_NAMES = {"u-boot", "monitor", "dtbo", "dtb"}
SAFE_NAME = re.compile(r"^[A-Za-z0-9_.+-]+$")


class Toc1Error(ValueError):
    """Raised when the TOC1 package is malformed or unsafe to extract."""


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def allwinner_checksum(data: bytes) -> int:
    """Return the Allwinner package checksum with checksum word at byte 20."""
    if len(data) % 4:
        raise Toc1Error("TOC1 package length is not 4-byte aligned")
    words = struct.unpack(f"<{len(data) // 4}I", data)
    return (sum(words) - words[5] + CHECKSUM_STAMP) & 0xFFFFFFFF


def parse_toc1(data: bytes) -> dict[str, Any]:
    """Validate the Allwinner TOC1 header, directory, item ranges and checksum.

    Layout follows BaseOS-MultiFrontend/tools/kernel_gzip.py and
    experiments/h700-boot-speed/spl/vendor-handoff.c:
    64-byte header; four 368-byte items at byte 64; item start/size and
    encryption/type words at item offsets +64/+68/+72/+76.
    """
    if len(data) < HEADER_BYTES:
        raise Toc1Error(f"input is shorter than the {HEADER_BYTES}-byte header")
    if len(data) > TOC1_BYTES:
        raise Toc1Error(
            f"input size 0x{len(data):x} exceeds the audited 0x{TOC1_BYTES:x}-byte package"
        )
    if data[:16] != TOC1_MAGIC:
        raise Toc1Error("invalid Allwinner TOC1 package magic")
    magic_word = struct.unpack_from("<I", data, 16)[0]
    if magic_word != TOC1_MAGIC_WORD:
        raise Toc1Error(f"unexpected TOC1 magic word 0x{magic_word:08x}")

    stored_checksum, = struct.unpack_from("<I", data, 20)
    calculated_checksum = allwinner_checksum(data)
    if stored_checksum != calculated_checksum:
        raise Toc1Error(
            f"TOC1 checksum mismatch: stored 0x{stored_checksum:08x}, "
            f"calculated 0x{calculated_checksum:08x}"
        )

    count, declared_size = struct.unpack_from("<II", data, 32)
    if count != ITEM_COUNT:
        raise Toc1Error(f"expected {ITEM_COUNT} TOC1 items, found {count}")
    if declared_size != len(data):
        raise Toc1Error(
            f"declared package size 0x{declared_size:x} does not match "
            f"input size 0x{len(data):x}"
        )
    if data[60:64] != b"MIE;":
        raise Toc1Error("missing TOC1 MIE; table marker")

    table_end = HEADER_BYTES + count * ITEM_BYTES
    if table_end > declared_size:
        raise Toc1Error("item table extends beyond the declared package")

    items: list[dict[str, Any]] = []
    names: set[str] = set()
    ranges: list[tuple[int, int, str]] = []

    for index in range(count):
        item_at = HEADER_BYTES + index * ITEM_BYTES
        item_header = data[item_at : item_at + ITEM_BYTES]
        raw_name = item_header[:ITEM_NAME_BYTES].split(b"\0", 1)[0]
        if not raw_name:
            raise Toc1Error(f"item {index} has an empty name")
        try:
            name = raw_name.decode("ascii")
        except UnicodeDecodeError as exc:
            raise Toc1Error(f"item {index} name is not ASCII") from exc
        if not SAFE_NAME.fullmatch(name):
            raise Toc1Error(f"item {index} has an unsafe output name: {name!r}")
        if name in names:
            raise Toc1Error(f"duplicate item name: {name}")
        names.add(name)

        offset, size, encryption, kind = struct.unpack_from("<4I", item_header, ITEM_NAME_BYTES)
        end = offset + size
        if size == 0:
            raise Toc1Error(f"item {name} has zero length")
        if encryption != 0 or kind != 3:
            raise Toc1Error(
                f"item {name} has unsupported encryption/type values {encryption}/{kind}"
            )
        if item_header[364:368] != b"IIE;":
            raise Toc1Error(f"item {name} is missing its IIE; footer")
        if offset < table_end:
            raise Toc1Error(f"item {name} overlaps the TOC1 header/table")
        if end > declared_size or end < offset:
            raise Toc1Error(
                f"item {name} range 0x{offset:x}..0x{end:x} exceeds package bounds"
            )
        ranges.append((offset, end, name))
        payload = data[offset:end]
        items.append(
            {
                "index": index,
                "name": name,
                "offset": offset,
                "size": size,
                "sha256": sha256(payload),
                "data": payload,
            }
        )

    if names != ITEM_NAMES:
        raise Toc1Error(
            "unexpected TOC1 components: " + ", ".join(sorted(names))
        )

    ranges.sort()
    for previous, current in zip(ranges, ranges[1:]):
        if current[0] < previous[1]:
            raise Toc1Error(
                f"items {previous[2]} and {current[2]} overlap in the package"
            )

    return {
        "package_bytes": len(data),
        "package_sha256": sha256(data),
        "declared_item_count": count,
        "stored_checksum": f"0x{stored_checksum:08x}",
        "items": items,
    }


def extract_toc1(input_path: Path, output_dir: Path) -> dict[str, Any]:
    raw = input_path.read_bytes()
    parsed = parse_toc1(raw)

    if output_dir.exists() and any(output_dir.iterdir()):
        raise Toc1Error(f"output directory is not empty: {output_dir}")
    output_dir.mkdir(parents=True, exist_ok=True)
    item_dir = output_dir / "items"
    item_dir.mkdir(exist_ok=True)

    manifest_items = []
    for item in parsed["items"]:
        output_name = f"{item['name']}.bin"
        (item_dir / output_name).write_bytes(item["data"])
        manifest_items.append(
            {
                "index": item["index"],
                "name": item["name"],
                "file": f"items/{output_name}",
                "offset": item["offset"],
                "size": item["size"],
                "sha256": item["sha256"],
            }
        )

    manifest = {
        "format": "Allwinner H700 TOC1 item directory",
        "source_file": input_path.name,
        "package_bytes": parsed["package_bytes"],
        "package_sha256": parsed["package_sha256"],
        "stored_checksum": parsed["stored_checksum"],
        "declared_item_count": parsed["declared_item_count"],
        "items": manifest_items,
        "limitations": [
            "This validates the known RG34XX H700 TOC1 container layout and checksum.",
            "It does not authenticate a vendor signature, parse Boot0 DRAM training, or prove a live handoff ABI.",
            "The package hash is a byte-level fingerprint, not a per-boot hardware attestation.",
        ],
    }
    (output_dir / "manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return manifest


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("toc1", type=Path, help="raw TOC1 dump from the inventory collector")
    parser.add_argument(
        "-o",
        "--output-dir",
        type=Path,
        default=Path("toc1-extracted"),
        help="new or empty destination directory (default: toc1-extracted)",
    )
    args = parser.parse_args()

    try:
        manifest = extract_toc1(args.toc1, args.output_dir)
    except (OSError, Toc1Error) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    print(f"TOC1 SHA-256: {manifest['package_sha256']}")
    print(f"TOC1 checksum: {manifest['stored_checksum']}")
    print(f"Items: {manifest['declared_item_count']}")
    for item in manifest["items"]:
        print(
            f"{item['name']:<16} {item['size']:>10} bytes  "
            f"SHA-256 {item['sha256']}"
        )
    print(f"Extracted files and manifest: {args.output_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
