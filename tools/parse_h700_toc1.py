#!/usr/bin/env python3
"""Parse and extract the H700 TOC1 layout used by the BaseOS boot-chain audit."""

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
ITEM_LIMIT = 16
TOC1_BYTES = 0x140000
SAFE_NAME = re.compile(r"^[A-Za-z0-9_.+-]+$")


class Toc1Error(ValueError):
    """Raised when the TOC1 package is malformed or unsafe to extract."""


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def parse_toc1(data: bytes) -> dict[str, Any]:
    """Validate the TOC1 directory and return item metadata and bytes.

    The offsets and item stride follow the parser used in
    BaseOS-MultiFrontend/experiments/h700-boot-speed/spl/vendor-handoff.c:
    item table at 64, stride 368, item offset/size at +64/+68.
    """
    if len(data) < HEADER_BYTES:
        raise Toc1Error(f"input is shorter than the {HEADER_BYTES}-byte header")

    count, declared_size = struct.unpack_from("<II", data, 32)
    if count < 1 or count > ITEM_LIMIT:
        raise Toc1Error(f"item count {count} is outside 1..{ITEM_LIMIT}")
    if declared_size != len(data):
        raise Toc1Error(
            f"declared package size 0x{declared_size:x} does not match "
            f"input size 0x{len(data):x}"
        )
    if declared_size > TOC1_BYTES:
        raise Toc1Error(
            f"package size 0x{declared_size:x} exceeds the audited 0x{TOC1_BYTES:x}-byte window"
        )

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

        offset, size = struct.unpack_from("<II", item_header, ITEM_NAME_BYTES)
        end = offset + size
        if size == 0:
            raise Toc1Error(f"item {name} has zero length")
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
        "format": "BaseOS H700 TOC1 item directory layout",
        "source_file": input_path.name,
        "package_bytes": parsed["package_bytes"],
        "package_sha256": parsed["package_sha256"],
        "declared_item_count": parsed["declared_item_count"],
        "items": manifest_items,
        "limitations": [
            "This parses the TOC1 item directory layout used by the audited BaseOS H700 code.",
            "It does not authenticate signatures, validate Boot0, or prove a live handoff ABI.",
            "The package hash is an input fingerprint, not a per-boot hardware attestation.",
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
