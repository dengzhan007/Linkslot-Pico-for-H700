#!/usr/bin/env python3
"""Build a host-only RG34XX Android-v2 kernel-slot candidate partition.

This tool does NOT flash media. It accepts an existing p4 partition image and
an uncompressed AArch64 Image, gzip-wraps the Image for the observed RG34XX
U-Boot selector, updates only Android v2 kernel size/image ID, and preserves
all non-kernel components plus the original unused partition tail.
"""
from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import struct
from pathlib import Path
from typing import Any

import inspect_android_bootimg as android


class PayloadBuildError(ValueError):
    """Input cannot safely fit the observed RG34XX stock boot path."""


EXPECTED_KERNEL_ADDR = 0x40080000
EXPECTED_RAMDISK_ADDR = 0x42000000
EXPECTED_DTB_ADDR = 0x44000000
EXPECTED_TEXT_OFFSET = 0x80000
DRAM_BASE = 0x40000000
ANDROID_HEADER_KERNEL_SIZE = 0x08
ANDROID_HEADER_IMAGE_ID = 0x240
ANDROID_HEADER_RECOVERY_DTBO_OFFSET = 0x664
ANDROID_HEADER_RECOVERY_DTBO_SIZE = 0x660


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def component_bytes(
    partition: bytes, report: dict[str, Any], name: str
) -> bytes:
    component = report["components"][name]
    offset = int(component["offset"], 16)
    size = component["size"]
    end = offset + size
    if end > len(partition):
        raise PayloadBuildError(f"component {name} exceeds partition bounds")
    return partition[offset:end]


def validate_stock_profile(report: dict[str, Any]) -> int:
    header = report["header"]
    if report["format"] != "Android boot image v2":
        raise PayloadBuildError("only Android boot image v2 is supported")
    if header["page_size"] != 2048:
        raise PayloadBuildError("RG34XX stock page size must be 2048")
    if int(header["kernel_addr"], 16) != EXPECTED_KERNEL_ADDR:
        raise PayloadBuildError("unexpected RG34XX kernel load address")
    if int(header["ramdisk_addr"], 16) != EXPECTED_RAMDISK_ADDR:
        raise PayloadBuildError("unexpected RG34XX ramdisk address")
    if int(header["dtb_addr"], 16) != EXPECTED_DTB_ADDR:
        raise PayloadBuildError("unexpected RG34XX DTB address")
    if report["kernel"]["compression"] != "gzip":
        raise PayloadBuildError(
            "current stock boot path is validated for a gzip kernel component"
        )
    slot_bytes = EXPECTED_RAMDISK_ADDR - EXPECTED_KERNEL_ADDR
    if slot_bytes <= 0:
        raise PayloadBuildError("invalid kernel/ramdisk address ordering")
    return slot_bytes


def validate_payload_image(payload: bytes, slot_bytes: int) -> dict[str, Any]:
    if len(payload) < 64 or payload[0x38:0x3C] != b"ARM\x64":
        raise PayloadBuildError("payload must have an AArch64 Image header (ARM\\x64)")
    code0, code1 = struct.unpack_from("<2I", payload, 0)
    text_offset, image_size, flags = struct.unpack_from("<3Q", payload, 8)
    if text_offset != EXPECTED_TEXT_OFFSET:
        raise PayloadBuildError(
            f"Image text_offset must be 0x{EXPECTED_TEXT_OFFSET:x}, got 0x{text_offset:x}"
        )
    if image_size <= 0:
        raise PayloadBuildError("Image image_size must be nonzero")
    if image_size < len(payload):
        raise PayloadBuildError(
            f"Image image_size 0x{image_size:x} is smaller than file length 0x{len(payload):x}"
        )
    if image_size > slot_bytes or len(payload) > slot_bytes:
        raise PayloadBuildError(
            f"AArch64 Image exceeds kernel slot before ramdisk (0x{slot_bytes:x} bytes)"
        )
    if EXPECTED_KERNEL_ADDR - DRAM_BASE != text_offset:
        raise PayloadBuildError("kernel_addr and Image text_offset are inconsistent")

    # AArch64 Image header code0 is expected to branch from the header to the
    # first executable entry code. Reject a blank/data-only first word.
    if (code0 & 0x7C000000) != 0x14000000:
        raise PayloadBuildError(
            f"Image code0 is not an AArch64 unconditional B instruction: 0x{code0:08x}"
        )
    imm26 = code0 & 0x03FFFFFF
    if imm26 & 0x02000000:
        imm26 -= 0x04000000
    entry_offset = imm26 << 2
    if entry_offset < 64 or entry_offset >= min(image_size, len(payload)):
        raise PayloadBuildError(
            f"Image code0 branch target 0x{entry_offset:x} is outside payload code"
        )

    return {
        "bytes": len(payload),
        "sha256": sha256(payload),
        "code0": f"0x{code0:08x}",
        "code1": f"0x{code1:08x}",
        "text_offset": f"0x{text_offset:x}",
        "image_size": f"0x{image_size:x}",
        "flags": f"0x{flags:x}",
        "header_entry_offset": f"0x{entry_offset:x}",
        "slot_bytes_before_ramdisk": slot_bytes,
    }


def build_candidate_partition(
    original_partition: bytes, payload_image: bytes
) -> tuple[bytes, dict[str, Any]]:
    if len(original_partition) % 512:
        raise PayloadBuildError("input partition length must be a multiple of 512 bytes")
    if len(original_partition) < 2048:
        raise PayloadBuildError("input partition is too short")

    original_report = android.inspect_android_boot_image(original_partition)
    slot_bytes = validate_stock_profile(original_report)
    payload_report = validate_payload_image(payload_image, slot_bytes)
    compressed_kernel = gzip.compress(payload_image, compresslevel=9, mtime=0)
    if not compressed_kernel:
        raise PayloadBuildError("gzip produced an empty kernel component")

    preserved = {
        name: component_bytes(original_partition, original_report, name)
        for name in ("ramdisk", "second", "recovery_dtbo", "dtb")
    }
    components = [
        compressed_kernel,
        preserved["ramdisk"],
        preserved["second"],
        preserved["recovery_dtbo"],
        preserved["dtb"],
    ]

    page_size = original_report["header"]["page_size"]
    header = bytearray(original_partition[:page_size])
    struct.pack_into("<I", header, ANDROID_HEADER_KERNEL_SIZE, len(compressed_kernel))

    cursor = page_size
    offsets: dict[str, int] = {}
    names = ("kernel", "ramdisk", "second", "recovery_dtbo", "dtb")
    rebuilt = bytearray(header)
    for name, component in zip(names, components):
        offsets[name] = cursor
        rebuilt.extend(component)
        cursor += len(component)
        padded = android.align(cursor, page_size)
        rebuilt.extend(bytes(padded - cursor))
        cursor = padded

    recovery_size = len(preserved["recovery_dtbo"])
    if recovery_size:
        struct.pack_into(
            "<Q", rebuilt, ANDROID_HEADER_RECOVERY_DTBO_OFFSET,
            offsets["recovery_dtbo"],
        )

    image_id = android.android_image_id(components)
    rebuilt[ANDROID_HEADER_IMAGE_ID:ANDROID_HEADER_IMAGE_ID + 32] = image_id

    old_end = int(original_report["padded_components_end"], 16)
    new_end = len(rebuilt)
    if new_end > old_end:
        raise PayloadBuildError(
            f"rebuilt components end at 0x{new_end:x}, beyond original component area 0x{old_end:x}"
        )
    if old_end > len(original_partition):
        raise PayloadBuildError("original component end exceeds partition length")

    # Clear only the old component region that is no longer occupied. Preserve
    # the unused p4 tail byte-for-byte from the original component boundary.
    candidate = bytes(rebuilt) + bytes(old_end - new_end) + original_partition[old_end:]
    if len(candidate) != len(original_partition):
        raise PayloadBuildError("candidate partition size changed")
    if candidate[old_end:] != original_partition[old_end:]:
        raise PayloadBuildError("unused partition tail was not preserved")

    candidate_report = android.inspect_android_boot_image(candidate)
    candidate_components = {
        name: component_bytes(candidate, candidate_report, name)
        for name in names
    }
    for name in ("ramdisk", "second", "recovery_dtbo", "dtb"):
        if candidate_components[name] != preserved[name]:
            raise PayloadBuildError(f"non-kernel component changed: {name}")

    manifest = {
        "artifact_type": "offline-only RG34XX Android boot partition candidate",
        "flashable_release": False,
        "device": "Anbernic RG34XX non-SP / H700",
        "original_partition_bytes": len(original_partition),
        "original_partition_sha256": sha256(original_partition),
        "candidate_partition_bytes": len(candidate),
        "candidate_partition_sha256": sha256(candidate),
        "android_boot_header_version": 2,
        "page_size": page_size,
        "kernel_load_address": f"0x{EXPECTED_KERNEL_ADDR:x}",
        "ramdisk_address": f"0x{EXPECTED_RAMDISK_ADDR:x}",
        "fdt_handoff_address": f"0x{EXPECTED_DTB_ADDR:x}",
        "kernel_slot_bytes_before_ramdisk": slot_bytes,
        "original_kernel": {
            "bytes": original_report["components"]["kernel"]["size"],
            "sha256": original_report["components"]["kernel"]["sha256"],
            "compression": original_report["kernel"]["compression"],
        },
        "new_kernel": {
            "uncompressed": payload_report,
            "compressed_bytes": len(compressed_kernel),
            "compressed_sha256": sha256(compressed_kernel),
            "compression": "gzip",
        },
        "preserved_components": {
            name: {
                "bytes": len(blob),
                "sha256": sha256(blob),
                "unchanged": True,
            }
            for name, blob in preserved.items()
        },
        "original_components_end": f"0x{old_end:x}",
        "candidate_components_end": f"0x{new_end:x}",
        "unused_partition_tail_preserved": True,
        "android_image_id_valid": True,
        "limits": [
            "Host-side image construction only; no block device is accessed.",
            "This does not validate monitor ERET state, runtime RAM ownership, or payload behavior.",
            "Do not flash this candidate to a daily-use card.",
        ],
    }
    return candidate, manifest


def write_new_file(path: Path, data: bytes) -> None:
    if path.exists():
        raise PayloadBuildError(f"refusing to overwrite existing file: {path}")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("original_partition", type=Path)
    parser.add_argument("payload_image", type=Path, help="uncompressed AArch64 Image binary")
    parser.add_argument("--output-partition", type=Path, required=True)
    parser.add_argument("--report", type=Path)
    args = parser.parse_args()

    if args.original_partition.resolve() == args.output_partition.resolve():
        raise PayloadBuildError("output partition path must differ from input")
    if args.report and args.report.resolve() in (
        args.original_partition.resolve(), args.payload_image.resolve(),
    ):
        raise PayloadBuildError("report path must differ from both input files")

    original = args.original_partition.read_bytes()
    payload = args.payload_image.read_bytes()
    candidate, manifest = build_candidate_partition(original, payload)

    write_new_file(args.output_partition, candidate)
    if args.report:
        write_new_file(args.report, (json.dumps(manifest, indent=2) + "\n").encode())
    print(json.dumps(manifest, indent=2))


if __name__ == "__main__":
    try:
        main()
    except (OSError, android.BootImageError, PayloadBuildError) as exc:
        raise SystemExit(f"error: {exc}")
