#!/usr/bin/env python3
"""Pack a raw AArch64 entry body into a Linux-style 64-byte ARM64 Image header.

Host-only packaging helper. It does not access a block device or create an
Android boot partition. The output is a static Image-format build artifact,
not an approved device payload.
"""
from __future__ import annotations

import argparse
import json
import struct
from pathlib import Path
from typing import Any

ARM64_MAGIC = b"ARM\x64"
HEADER_BYTES = 64
EXPECTED_TEXT_OFFSET = 0x80000
MAX_IMAGE_SIZE = 0x1138000
DEFAULT_PAGE_SIZE = 4096
AARCH64_B_UNCONDITIONAL = 0x14000000
AARCH64_NOP = 0xD503201F


class ImagePackError(ValueError):
    """Invalid Image body or output path."""


def align(value: int, boundary: int) -> int:
    if boundary <= 0 or boundary & (boundary - 1):
        raise ImagePackError(f"alignment must be a positive power of two: {boundary}")
    return (value + boundary - 1) & ~(boundary - 1)


def pack_aarch64_image(
    body: bytes,
    entry_offset: int = HEADER_BYTES,
    text_offset: int = EXPECTED_TEXT_OFFSET,
    page_size: int = DEFAULT_PAGE_SIZE,
    max_image_size: int = MAX_IMAGE_SIZE,
) -> tuple[bytes, dict[str, Any]]:
    if not body:
        raise ImagePackError("entry body is empty")
    if text_offset != EXPECTED_TEXT_OFFSET:
        raise ImagePackError(
            f"RG34XX stock kernel slot requires text_offset 0x{EXPECTED_TEXT_OFFSET:x}"
        )
    if entry_offset < HEADER_BYTES or entry_offset % 4:
        raise ImagePackError("entry offset must be >= 64 and 4-byte aligned")
    if entry_offset >= HEADER_BYTES + len(body):
        raise ImagePackError("entry offset points outside the packaged Image body")
    if page_size < 512 or page_size & (page_size - 1):
        raise ImagePackError("page size must be a power of two >= 512")

    raw_size = HEADER_BYTES + len(body)
    image_size = align(raw_size, page_size)
    if image_size > max_image_size:
        raise ImagePackError(
            f"packed Image size 0x{image_size:x} exceeds conservative maximum 0x{max_image_size:x}"
        )

    branch_delta = entry_offset
    if branch_delta % 4:
        raise ImagePackError("AArch64 branch target is not instruction aligned")
    immediate = branch_delta // 4
    if immediate >= (1 << 25):
        raise ImagePackError("AArch64 Image header branch target is out of range")
    code0 = AARCH64_B_UNCONDITIONAL | immediate

    header = bytearray(HEADER_BYTES)
    struct.pack_into("<II", header, 0, code0, AARCH64_NOP)
    struct.pack_into("<QQQ", header, 8, text_offset, image_size, 0)
    header[56:60] = ARM64_MAGIC

    output = bytes(header) + body + bytes(image_size - raw_size)
    report = {
        "format": "AArch64 Linux Image header",
        "artifact_type": "host-only passive entry probe Image",
        "device_profile": "RG34XX non-SP / Allwinner H700",
        "image_bytes": len(output),
        "body_bytes": len(body),
        "entry_offset": f"0x{entry_offset:x}",
        "code0": f"0x{code0:08x}",
        "code1": f"0x{AARCH64_NOP:08x}",
        "text_offset": f"0x{text_offset:x}",
        "image_size": f"0x{image_size:x}",
        "flags": "0x0",
        "magic": ARM64_MAGIC.decode("ascii", errors="replace"),
        "limits": [
            "Static host build artifact only; it has not been executed on RG34XX.",
            "Private stack and entry record are embedded in this Image's file-backed storage.",
            "No generic allocator, MMIO, SMC, external RAM, or Linux runtime is used.",
            "Do not flash this Image until monitor ERET and FDT provenance gates pass.",
        ],
    }
    return output, report


def write_new_file(path: Path, content: bytes) -> None:
    if path.exists():
        raise ImagePackError(f"refusing to overwrite existing output: {path}")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(content)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("body", type=Path, help="raw entry body emitted by llvm-objcopy")
    parser.add_argument("--entry-offset", type=lambda value: int(value, 0), default=HEADER_BYTES)
    parser.add_argument("--output", type=Path, required=True, help="new Image file")
    parser.add_argument("--report", type=Path, help="new JSON report file")
    parser.add_argument("--page-size", type=lambda value: int(value, 0), default=DEFAULT_PAGE_SIZE)
    args = parser.parse_args()

    outputs = {args.output.resolve()}
    if args.report:
        outputs.add(args.report.resolve())
    if len(outputs) != (2 if args.report else 1):
        raise ImagePackError("Image and report output paths must differ")
    if any(path.exists() for path in outputs):
        raise ImagePackError("refusing to overwrite existing output file(s)")

    image, report = pack_aarch64_image(
        args.body.read_bytes(), entry_offset=args.entry_offset, page_size=args.page_size
    )
    write_new_file(args.output, image)
    if args.report:
        write_new_file(
            args.report,
            (json.dumps(report, indent=2, sort_keys=True) + "\n").encode("utf-8"),
        )
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ImagePackError) as exc:
        raise SystemExit(f"error: {exc}")
