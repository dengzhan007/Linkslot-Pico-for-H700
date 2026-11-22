#!/usr/bin/env python3
"""Read-only Android boot image v2 inspector for RG34XX/H700 port research.

Validates the Android component layout and image ID, then reports gzip,
AArch64 Image and embedded FDT metadata. It never writes to the input image or
a block device. Optional extraction writes only to a new host directory.
"""
from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import re
import struct
from pathlib import Path
from typing import Any

ANDROID_MAGIC = b"ANDROID!"
FDT_MAGIC = 0xD00DFEED
HEADER_V2_MIN = 0x67C
COMPONENT_FIELDS = (
    ("kernel", 0x08),
    ("ramdisk", 0x10),
    ("second", 0x18),
    ("recovery_dtbo", 0x660),
    ("dtb", 0x670),
)
MAX_DECOMPRESSED_KERNEL = 128 * 1024 * 1024


class BootImageError(ValueError):
    """Invalid or unsupported Android boot image."""


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def align(value: int, boundary: int) -> int:
    if boundary <= 0 or boundary & (boundary - 1):
        raise BootImageError(f"invalid alignment: {boundary}")
    return (value + boundary - 1) & ~(boundary - 1)


def read_u32(data: bytes, offset: int, label: str) -> int:
    if offset < 0 or offset + 4 > len(data):
        raise BootImageError(f"truncated {label}")
    return struct.unpack_from("<I", data, offset)[0]


def android_image_id(parts: list[bytes]) -> bytes:
    digest = hashlib.sha1()
    for part in parts:
        digest.update(part)
        digest.update(struct.pack("<I", len(part)))
    return digest.digest() + bytes(12)


def detect_compression(data: bytes) -> str:
    signatures = (
        (b"\x1f\x8b", "gzip"),
        (b"\xfd7zXZ\x00", "xz"),
        (b"\x28\xb5\x2f\xfd", "zstd"),
        (b"BZh", "bzip2"),
        (b"\x04\x22\x4d\x18", "lz4-frame"),
        (b"\x02\x21\x4c\x18", "lz4-legacy"),
    )
    for magic, label in signatures:
        if data.startswith(magic):
            return label
    return "uncompressed-or-unknown"


def inspect_arm64_image(data: bytes) -> dict[str, Any] | None:
    if len(data) < 64 or data[0x38:0x3C] != b"ARM\x64":
        return None
    code0, code1 = struct.unpack_from("<2I", data, 0)
    text_offset, image_size, flags = struct.unpack_from("<3Q", data, 8)
    version = re.search(rb"Linux version [0-9][^\x00\r\n]{0,180}", data)
    return {
        "magic": "ARM\\x64",
        "code0": f"0x{code0:08x}",
        "code1": f"0x{code1:08x}",
        "text_offset": f"0x{text_offset:x}",
        "image_size": f"0x{image_size:x}",
        "flags": f"0x{flags:x}",
        "image_bytes": len(data),
        "linux_version": version.group(0).decode("ascii", errors="replace") if version else None,
    }


def inspect_fdt(blob: bytes) -> dict[str, Any]:
    if len(blob) < 40:
        raise BootImageError("DTB component is shorter than an FDT header")
    fields = struct.unpack_from(">10I", blob, 0)
    (magic, total, off_struct, off_strings, off_reserve, version,
     last_compatible, boot_cpuid, size_strings, size_struct) = fields
    if magic != FDT_MAGIC:
        raise BootImageError("DTB component does not start with FDT magic")
    if total != len(blob):
        raise BootImageError(
            f"FDT totalsize {total} does not match Android DTB component size {len(blob)}"
        )
    if version < 16 or version > 17 or last_compatible > version:
        raise BootImageError(f"unsupported FDT version {version}/{last_compatible}")
    if off_reserve < 40 or off_reserve >= total:
        raise BootImageError("FDT reservation map offset is out of bounds")
    for start, size, label in (
        (off_struct, size_struct, "structure"),
        (off_strings, size_strings, "strings"),
    ):
        if start < 40 or size > total or start + size > total:
            raise BootImageError(f"FDT {label} block is out of bounds")

    strings = blob[off_strings:off_strings + size_strings]
    cursor = off_struct
    struct_end = off_struct + size_struct
    stack: list[str] = []
    node_properties: dict[str, dict[str, bytes]] = {}
    ended = False

    while cursor + 4 <= struct_end:
        token = struct.unpack_from(">I", blob, cursor)[0]
        cursor += 4
        if token == 1:  # FDT_BEGIN_NODE
            end = blob.find(b"\0", cursor, struct_end)
            if end < 0:
                raise BootImageError("unterminated FDT node name")
            name = blob[cursor:end].decode("utf-8", errors="replace")
            cursor = align(end + 1, 4)
            stack.append(name)
            path = "/" + "/".join(part for part in stack if part)
            node_properties.setdefault(path, {})
        elif token == 2:  # FDT_END_NODE
            if not stack:
                raise BootImageError("unbalanced FDT_END_NODE")
            stack.pop()
        elif token == 3:  # FDT_PROP
            if cursor + 8 > struct_end:
                raise BootImageError("truncated FDT property header")
            length, nameoff = struct.unpack_from(">2I", blob, cursor)
            cursor += 8
            if cursor + length > struct_end or nameoff >= len(strings):
                raise BootImageError("FDT property is out of bounds")
            name_end = strings.find(b"\0", nameoff)
            if name_end < 0:
                raise BootImageError("unterminated FDT property name")
            prop_name = strings[nameoff:name_end].decode("utf-8", errors="replace")
            value = blob[cursor:cursor + length]
            cursor = align(cursor + length, 4)
            if not stack:
                raise BootImageError("FDT property outside a node")
            path = "/" + "/".join(part for part in stack if part)
            node_properties.setdefault(path, {})[prop_name] = value
        elif token == 4:  # FDT_NOP
            continue
        elif token == 9:  # FDT_END
            ended = True
            break
        else:
            raise BootImageError(f"unknown FDT token 0x{token:x} at 0x{cursor - 4:x}")
    if not ended or stack:
        raise BootImageError("FDT structure block did not terminate cleanly")

    def string_list(path: str, name: str) -> list[str] | None:
        value = node_properties.get(path, {}).get(name)
        if value is None:
            return None
        return [
            item.decode("utf-8", errors="replace")
            for item in value.rstrip(b"\0").split(b"\0")
            if item
        ]

    model = string_list("/", "model")
    compatible = string_list("/", "compatible")
    chosen_bootargs = string_list("/chosen", "bootargs")
    memory_nodes = []
    for path, properties in node_properties.items():
        device_type = string_list(path, "device_type")
        if device_type and "memory" in device_type:
            memory_nodes.append({
                "path": path,
                "reg_hex": properties.get("reg", b"").hex(),
                "status": string_list(path, "status"),
            })
    return {
        "totalsize": total,
        "version": version,
        "last_comp_version": last_compatible,
        "boot_cpuid_phys": boot_cpuid,
        "model": model[0] if model else None,
        "compatible": compatible,
        "chosen_bootargs": chosen_bootargs[0] if chosen_bootargs else None,
        "memory_nodes": memory_nodes,
        "node_count": len(node_properties),
    }


def inspect_android_boot_image(image: bytes) -> dict[str, Any]:
    if len(image) < 8 or image[:8] != ANDROID_MAGIC:
        raise BootImageError("missing ANDROID! magic")
    if len(image) < HEADER_V2_MIN:
        raise BootImageError("image is too short for Android header v2")

    kernel_size = read_u32(image, 0x08, "kernel_size")
    kernel_addr = read_u32(image, 0x0C, "kernel_addr")
    ramdisk_size = read_u32(image, 0x10, "ramdisk_size")
    ramdisk_addr = read_u32(image, 0x14, "ramdisk_addr")
    second_size = read_u32(image, 0x18, "second_size")
    second_addr = read_u32(image, 0x1C, "second_addr")
    tags_addr = read_u32(image, 0x20, "tags_addr")
    page_size = read_u32(image, 0x24, "page_size")
    header_version = read_u32(image, 0x28, "header_version")
    os_version = read_u32(image, 0x2C, "os_version")
    if header_version != 2:
        raise BootImageError(
            f"this inspector supports Android header v2 only; found {header_version}"
        )
    if page_size < 512 or page_size > 65536 or page_size & (page_size - 1):
        raise BootImageError(f"invalid Android page size {page_size}")
    header_size = read_u32(image, 0x66C, "header_size")
    recovery_dtbo_size = read_u32(image, 0x660, "recovery_dtbo_size")
    recovery_dtbo_offset = struct.unpack_from("<Q", image, 0x664)[0]
    dtb_size = read_u32(image, 0x670, "dtb_size")
    dtb_addr = read_u32(image, 0x674, "dtb_addr")
    if header_size < HEADER_V2_MIN or header_size > page_size:
        raise BootImageError(f"invalid Android v2 header_size 0x{header_size:x}")

    cmdline = image[0x40:0x240].split(b"\0", 1)[0].decode("utf-8", errors="replace")
    extra_cmdline = image[0x260:0x660].split(b"\0", 1)[0].decode("utf-8", errors="replace")
    name = image[0x30:0x40].split(b"\0", 1)[0].decode("ascii", errors="replace")

    specs = (
        ("kernel", kernel_size),
        ("ramdisk", ramdisk_size),
        ("second", second_size),
        ("recovery_dtbo", recovery_dtbo_size),
        ("dtb", dtb_size),
    )
    cursor = page_size
    components: dict[str, dict[str, Any]] = {}
    payloads: dict[str, bytes] = {}
    for component, size in specs:
        start, end = cursor, cursor + size
        if end > len(image):
            raise BootImageError(
                f"truncated {component}: ends at 0x{end:x}, image is 0x{len(image):x}"
            )
        payload = image[start:end]
        payloads[component] = payload
        components[component] = {
            "offset": f"0x{start:x}",
            "size": size,
            "sha256": sha256(payload),
        }
        cursor = align(end, page_size)
        if cursor > len(image):
            raise BootImageError(f"truncated page padding after {component}")
    if recovery_dtbo_size and recovery_dtbo_offset != int(
        components["recovery_dtbo"]["offset"], 16
    ):
        raise BootImageError("recovery_dtbo_offset disagrees with computed offset")
    if recovery_dtbo_size == 0 and recovery_dtbo_offset not in (0, cursor):
        recovery_offset_note = (
            f"nonzero offset 0x{recovery_dtbo_offset:x} despite zero size"
        )
    else:
        recovery_offset_note = None

    if image[0x240:0x260] != android_image_id([payloads[name] for name, _ in specs]):
        raise BootImageError("Android boot image ID does not match components")
    dtb_info = inspect_fdt(payloads["dtb"]) if dtb_size else None

    kernel = payloads["kernel"]
    compression = detect_compression(kernel)
    kernel_info: dict[str, Any] = {
        "compression": compression,
        "compressed_bytes": len(kernel),
    }
    if compression == "gzip":
        try:
            unpacked = gzip.decompress(kernel)
        except (OSError, EOFError) as exc:
            raise BootImageError(f"gzip kernel decompression failed: {exc}") from exc
        if len(unpacked) > MAX_DECOMPRESSED_KERNEL:
            raise BootImageError(
                f"decompressed kernel exceeds {MAX_DECOMPRESSED_KERNEL} bytes"
            )
        kernel_info.update({
            "uncompressed_bytes": len(unpacked),
            "uncompressed_sha256": hashlib.sha256(unpacked).hexdigest(),
            "arm64_image": inspect_arm64_image(unpacked),
        })

    return {
        "format": "Android boot image v2",
        "image_bytes": len(image),
        "image_sha256": sha256(image),
        "header": {
            "header_version": header_version,
            "header_size": header_size,
            "page_size": page_size,
            "os_version": os_version,
            "name": name,
            "kernel_addr": f"0x{kernel_addr:x}",
            "ramdisk_addr": f"0x{ramdisk_addr:x}",
            "second_addr": f"0x{second_addr:x}",
            "tags_addr": f"0x{tags_addr:x}",
            "dtb_addr": f"0x{dtb_addr:x}",
            "cmdline": cmdline,
            "extra_cmdline": extra_cmdline,
            "recovery_dtbo_offset": f"0x{recovery_dtbo_offset:x}",
            "recovery_offset_note": recovery_offset_note,
            "android_image_id_valid": True,
        },
        "components": components,
        "kernel": kernel_info,
        "dtb": dtb_info,
        "padded_components_end": f"0x{cursor:x}",
        "unused_tail_bytes": len(image) - cursor,
    }


def extract_components(image: bytes, report: dict[str, Any], output: Path) -> None:
    if output.exists():
        raise BootImageError(f"output directory already exists: {output}")
    output.mkdir(parents=True)
    page_size = report["header"]["page_size"]
    cursor = page_size
    for component, _field in COMPONENT_FIELDS:
        size = report["components"][component]["size"]
        (output / f"{component}.bin").write_bytes(image[cursor:cursor + size])
        cursor = align(cursor + size, page_size)
    (output / "manifest.json").write_text(
        json.dumps(report, indent=2) + "\n", encoding="utf-8"
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "image", type=Path,
        help="Android boot image file or a boot partition beginning with ANDROID!",
    )
    parser.add_argument("-o", "--output-json", type=Path, help="write JSON report here")
    parser.add_argument("--extract-dir", type=Path, help="extract validated components to a new directory")
    args = parser.parse_args()
    image = args.image.read_bytes()
    report = inspect_android_boot_image(image)
    if args.extract_dir:
        extract_components(image, report, args.extract_dir)
    encoded = json.dumps(report, indent=2) + "\n"
    if args.output_json:
        args.output_json.write_text(encoded, encoding="utf-8")
    else:
        print(encoded, end="")


if __name__ == "__main__":
    try:
        main()
    except (OSError, BootImageError) as exc:
        raise SystemExit(f"error: {exc}")
