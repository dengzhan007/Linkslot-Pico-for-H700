#!/usr/bin/env python3
"""Wrap a raw GammaOS LK BL33 image in the verified RG34XX native-loader header."""
from __future__ import annotations

import argparse
import hashlib
import json
import struct
import sys
from pathlib import Path

IMAGE_BASE = 0x4A000000
HEADER_END = 0x640
VECTOR_OFFSET = 0x1000
MAX_IMAGE_BYTES = 0xE0000
REFERENCE_LOADER_BYTES = 0x8000
REFERENCE_LOADER_SHA256 = "48b2a837ac5d8daad8f524c976f2b3911d26a838f8b68fa695e3283412dd8590"
HEADER_MAGIC = b"uboot"


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def decode_a32_branch(word: int, pc: int) -> int:
    if (word & 0x0F000000) != 0x0A000000 or (word & 0x01000000):
        raise ValueError(f"expected A32 B (not BL) at {pc:#010x}, got {word:#010x}")
    imm = word & 0x00FFFFFF
    if imm & 0x00800000:
        imm -= 1 << 24
    return pc + 8 + (imm << 2)


def package_image(
    lk_raw: bytes,
    reference_loader: bytes,
    *,
    require_pinned_reference: bool = True,
) -> tuple[bytes, dict]:
    """Insert only the tested vendor header bytes; do not change LK entry or vectors."""
    if len(reference_loader) != REFERENCE_LOADER_BYTES:
        raise ValueError(
            f"reference loader must be exactly {REFERENCE_LOADER_BYTES:#x} bytes"
        )
    reference_sha = sha256(reference_loader)
    if require_pinned_reference and reference_sha != REFERENCE_LOADER_SHA256:
        raise ValueError(
            "reference loader SHA-256 is not the audited active RG34XX loader: "
            f"{reference_sha}"
        )
    if reference_loader[4:9] != HEADER_MAGIC:
        raise ValueError("reference loader does not contain the expected 'uboot' header magic")

    if len(lk_raw) < VECTOR_OFFSET + 8 * 4:
        raise ValueError("raw LK image is too small to contain the fixed A32 vector table")
    if len(lk_raw) > MAX_IMAGE_BYTES:
        raise ValueError(
            f"raw LK image exceeds the native BL33 limit {MAX_IMAGE_BYTES:#x}"
        )
    if any(lk_raw[4:HEADER_END]):
        raise ValueError(
            "raw LK image is not the unwrapped H700 build: bytes [+4,+0x640) must be reserved/zero"
        )

    entry_target = decode_a32_branch(struct.unpack_from("<I", lk_raw, 0)[0], IMAGE_BASE)
    if not (IMAGE_BASE + HEADER_END <= entry_target < IMAGE_BASE + len(lk_raw)):
        raise ValueError(f"BL33 entry branch target is outside the LK code image: {entry_target:#x}")

    vector_targets = []
    for index in range(8):
        pc = IMAGE_BASE + VECTOR_OFFSET + index * 4
        word = struct.unpack_from("<I", lk_raw, VECTOR_OFFSET + index * 4)[0]
        target = decode_a32_branch(word, pc)
        if not (IMAGE_BASE + HEADER_END <= target < IMAGE_BASE + len(lk_raw)):
            raise ValueError(f"A32 vector {index} branches outside LK code image: {target:#x}")
        vector_targets.append(target)

    output = bytearray(lk_raw)
    output[4:HEADER_END] = reference_loader[4:HEADER_END]
    output.extend(bytes(-len(output) % 512))
    if len(output) > MAX_IMAGE_BYTES:
        raise ValueError(
            f"512-byte padded BL33 image exceeds the native limit {MAX_IMAGE_BYTES:#x}"
        )
    if bytes(output[:4]) != lk_raw[:4]:
        raise AssertionError("packaging changed the LK entry branch")
    if bytes(output[4:HEADER_END]) != reference_loader[4:HEADER_END]:
        raise AssertionError("vendor header template was not copied byte-for-byte")
    if bytes(output[VECTOR_OFFSET:VECTOR_OFFSET + 32]) != lk_raw[VECTOR_OFFSET:VECTOR_OFFSET + 32]:
        raise AssertionError("packaging changed the A32 vector table")

    report = {
        "format": "RG34XX native A32 BL33 image wrapping",
        "host_only": True,
        "device_io_performed": False,
        "not_installable": True,
        "not_hardware_validated": True,
        "base_image_address": hex(IMAGE_BASE),
        "raw_lk_bytes": len(lk_raw),
        "raw_lk_sha256": sha256(lk_raw),
        "entry_branch_target": hex(entry_target),
        "vector_table_offset": hex(VECTOR_OFFSET),
        "vector_branch_targets": [hex(item) for item in vector_targets],
        "reference_loader_bytes": len(reference_loader),
        "reference_loader_sha256": reference_sha,
        "copied_header_range": ["0x4", hex(HEADER_END)],
        "header_magic": HEADER_MAGIC.decode("ascii"),
        "output_bytes": len(output),
        "output_padding_bytes": len(output) - len(lk_raw),
        "output_sha256": sha256(bytes(output)),
        "runtime_spl_patch_offsets": {
            "dram_parameters": ["0x30", "0xb0"],
            "boot_mode_word": "0xe4",
            "board_bytes": ["0x4e8", "0x4eb"],
            "ram_size_mib": "0x4f8",
            "ram_base_size_word": "0x4f4"
        },
        "pairing_note": (
            "Header bytes are copied from the exact SHA-pinned 32 KiB loader "
            "extracted from the currently deployed RG34XX p4+40 MiB native "
            "firmware package. Re-qualify this template if Boot0, BL31, native "
            "SPL or the p4+40 MiB firmware package changes."
        )
    }
    return bytes(output), report


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--lk-raw", type=Path, required=True,
                        help="raw lk-no-mtk-header.bin produced by the H700 GammaOS LK build")
    parser.add_argument("--reference-loader", type=Path, required=True,
                        help="audited 32 KiB loader.bin extracted from RG34XX p4+40 MiB firmware")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    if args.output.exists():
        parser.error(f"refusing to overwrite existing output: {args.output}")
    try:
        image, report = package_image(
            args.lk_raw.read_bytes(),
            args.reference_loader.read_bytes(),
            require_pinned_reference=True,
        )
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_bytes(image)
        manifest_path = args.output.with_suffix(args.output.suffix + ".manifest.json")
        manifest_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
        print(json.dumps(report, indent=2))
        print(f"BL33 image: {args.output}")
        print(f"Manifest: {manifest_path}")
        return 0
    except (OSError, ValueError) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
