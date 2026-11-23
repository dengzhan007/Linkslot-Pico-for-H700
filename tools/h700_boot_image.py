#!/usr/bin/env python3
"""Build and inspect the RG34XX BaseOS-native BOL4 / Android boot-v2 container.

This is a host-side packaging tool only. It does not read or write a device,
SD card, Boot0, TOC1, U-Boot, monitor, or firmware partition.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import struct
from pathlib import Path

PAGE_SIZE = 2048
ANDROID_MAGIC = b"ANDROID!"
BOL4_MAGIC = b"BOL4"
ANDROID_COMPONENT_SIZE_OFFSETS = (8, 16, 24, 0x660, 0x670)
ANDROID_HEADER_SIZE_OFFSET = 0x66C
ANDROID_V2_HEADER_SIZE = 1660
ANDROID_KERNEL_ADDR = 0x40080000
ANDROID_RAMDISK_ADDR = 0x42000000
ANDROID_SECOND_ADDR = 0x40F00000
ANDROID_TAGS_ADDR = 0x40000100
MAX_KERNEL_OUTPUT = 0x01F80000
MAX_ANDROID_IMAGE = 0x01000000


class ImageError(ValueError):
    """Invalid or unsupported kernel/boot-image input."""


def _u32(data: bytes, offset: int) -> int:
    return struct.unpack_from("<I", data, offset)[0]


def fnv1a32(data: bytes) -> int:
    value = 2166136261
    for byte in data:
        value = ((value ^ byte) * 16777619) & 0xFFFFFFFF
    return value


def align_page(size: int) -> int:
    if size < 0 or size > 0xFFFFFFFF - (PAGE_SIZE - 1):
        raise ImageError("component size overflows Android page alignment")
    return (size + PAGE_SIZE - 1) & ~(PAGE_SIZE - 1)


def _append_lz4_length(out: bytearray, length: int) -> None:
    while length >= 255:
        out.append(255)
        length -= 255
    out.append(length)


def lz4_compress_block(source: bytes) -> bytes:
    """Encode a raw LZ4 block, keeping the final five bytes literal."""
    size = len(source)
    if size == 0:
        return b"\x00"
    out = bytearray()
    table: dict[int, int] = {}
    anchor = 0
    pos = 0
    search_end = size - 12

    while pos <= search_end:
        word = source[pos:pos + 4]
        key = int.from_bytes(word, "little")
        ref = table.get(key)
        table[key] = pos
        if ref is None or pos - ref > 0xFFFF or source[ref:ref + 4] != word:
            pos += 1
            continue

        match_len = 4
        max_match = size - 5 - pos
        while match_len < max_match and source[ref + match_len] == source[pos + match_len]:
            match_len += 1

        literal_len = pos - anchor
        match_code = match_len - 4
        token_at = len(out)
        out.append(0)
        token = (min(literal_len, 15) << 4) | min(match_code, 15)
        if literal_len >= 15:
            _append_lz4_length(out, literal_len - 15)
        out.extend(source[anchor:pos])
        out.extend(struct.pack("<H", pos - ref))
        if match_code >= 15:
            _append_lz4_length(out, match_code - 15)
        out[token_at] = token

        previous = pos
        pos += match_len
        anchor = pos
        seed_end = min(pos, size - 3)
        for seed in range(previous + 1, seed_end):
            table[int.from_bytes(source[seed:seed + 4], "little")] = seed

    literal_len = size - anchor
    token_at = len(out)
    out.append(min(literal_len, 15) << 4)
    if literal_len >= 15:
        _append_lz4_length(out, literal_len - 15)
    out.extend(source[anchor:])
    return bytes(out)


def lz4_decompress_block(block: bytes, expected_size: int,
                         max_output: int = MAX_KERNEL_OUTPUT) -> bytes:
    """Decode a raw LZ4 block with strict input/output bounds."""
    if expected_size < 0 or expected_size > max_output:
        raise ImageError("declared LZ4 output exceeds configured capacity")
    if not block:
        raise ImageError("empty LZ4 block")
    output = bytearray()
    cursor = 0
    while cursor < len(block):
        token = block[cursor]
        cursor += 1
        literal_len = token >> 4
        if literal_len == 15:
            while True:
                if cursor >= len(block):
                    raise ImageError("truncated LZ4 literal-length extension")
                extra = block[cursor]
                cursor += 1
                literal_len += extra
                if extra != 255:
                    break
        if literal_len > len(block) - cursor:
            raise ImageError("LZ4 literal overruns packed block")
        if literal_len > expected_size - len(output):
            raise ImageError("LZ4 literal overruns declared output")
        output.extend(block[cursor:cursor + literal_len])
        cursor += literal_len
        if cursor == len(block):
            break
        if len(block) - cursor < 2:
            raise ImageError("truncated LZ4 match offset")
        offset = block[cursor] | (block[cursor + 1] << 8)
        cursor += 2
        if offset == 0 or offset > len(output):
            raise ImageError("invalid LZ4 back-reference offset")
        match_len = (token & 0x0F) + 4
        if (token & 0x0F) == 15:
            while True:
                if cursor >= len(block):
                    raise ImageError("truncated LZ4 match-length extension")
                extra = block[cursor]
                cursor += 1
                match_len += extra
                if extra != 255:
                    break
        if match_len > expected_size - len(output):
            raise ImageError("LZ4 match overruns declared output")
        for _ in range(match_len):
            output.append(output[-offset])
    if len(output) != expected_size:
        raise ImageError(f"LZ4 output length mismatch: got {len(output)}, expected {expected_size}")
    return bytes(output)


def validate_arm64_image(kernel: bytes, max_output: int = MAX_KERNEL_OUTPUT) -> None:
    if len(kernel) < 64:
        raise ImageError("AArch64 Image is shorter than its 64-byte header")
    if kernel[56:60] != b"ARM\x64":
        raise ImageError("kernel payload is not an AArch64 Linux Image (ARM64 magic missing)")
    declared = struct.unpack_from("<Q", kernel, 16)[0]
    if declared and declared < len(kernel):
        raise ImageError("AArch64 Image header image_size is smaller than the file")
    if len(kernel) > max_output:
        raise ImageError("AArch64 Image exceeds configured native-loader output capacity")


def build_bol4(kernel: bytes, max_output: int = MAX_KERNEL_OUTPUT,
               max_frame: int = MAX_ANDROID_IMAGE) -> bytes:
    validate_arm64_image(kernel, max_output)
    packed = lz4_compress_block(kernel)
    if len(packed) > 0xFFFFFFFF or len(kernel) > 0xFFFFFFFF:
        raise ImageError("BOL4 length cannot be represented in its header")
    frame = BOL4_MAGIC + struct.pack("<III", len(packed), len(kernel), fnv1a32(packed)) + packed
    if len(frame) > max_frame:
        raise ImageError("BOL4 frame exceeds configured Android image size budget")
    unpack_bol4(frame, max_output=max_output)
    return frame


def unpack_bol4(frame: bytes, max_output: int = MAX_KERNEL_OUTPUT) -> bytes:
    if len(frame) < 16 or frame[:4] != BOL4_MAGIC:
        raise ImageError("invalid BOL4 magic/header")
    packed_len, output_len, expected_fnv = struct.unpack_from("<III", frame, 4)
    if packed_len == 0 or output_len == 0:
        raise ImageError("BOL4 packed/output length must be nonzero")
    if output_len > max_output:
        raise ImageError("BOL4 output exceeds native loader capacity")
    if packed_len != len(frame) - 16:
        raise ImageError("BOL4 packed length does not match component extent")
    packed = frame[16:]
    if fnv1a32(packed) != expected_fnv:
        raise ImageError("BOL4 packed-payload FNV-1a mismatch")
    kernel = lz4_decompress_block(packed, output_len, max_output=max_output)
    validate_arm64_image(kernel, max_output)
    return kernel


def android_image_id(parts: tuple[bytes, ...]) -> bytes:
    digest = hashlib.sha1()
    for part in parts:
        digest.update(part)
        digest.update(struct.pack("<I", len(part)))
    return digest.digest() + bytes(12)


def validate_android_v2_header(header: bytes) -> None:
    if len(header) != PAGE_SIZE or header[:8] != ANDROID_MAGIC:
        raise ImageError("Android v2 header template must be exactly 2048 bytes with ANDROID! magic")
    if _u32(header, 36) != PAGE_SIZE or _u32(header, 40) != 2:
        raise ImageError("only 2048-byte-page Android boot header version 2 is supported")
    if _u32(header, ANDROID_HEADER_SIZE_OFFSET) != ANDROID_V2_HEADER_SIZE:
        raise ImageError("Android v2 header_size field is not 1660")
    expected = (ANDROID_KERNEL_ADDR, ANDROID_RAMDISK_ADDR,
                ANDROID_SECOND_ADDR, ANDROID_TAGS_ADDR)
    actual = tuple(_u32(header, offset) for offset in (12, 20, 28, 32))
    if actual != expected:
        raise ImageError("Android load addresses differ from the audited RG34XX loader contract")
    if b"\0" not in header[0x40:0x240] or b"\0" not in header[0x260:0x660]:
        raise ImageError("Android command-line fields are not NUL-terminated")


def split_android_v2(image: bytes, max_image: int = MAX_ANDROID_IMAGE) -> tuple[bytes, tuple[bytes, ...]]:
    if len(image) < PAGE_SIZE or len(image) > max_image:
        raise ImageError("Android image size is outside configured bounds")
    header = image[:PAGE_SIZE]
    validate_android_v2_header(header)
    parts = []
    cursor = PAGE_SIZE
    for offset in ANDROID_COMPONENT_SIZE_OFFSETS:
        size = _u32(header, offset)
        if size > len(image) - cursor:
            raise ImageError("Android component exceeds image extent")
        parts.append(image[cursor:cursor + size])
        padded = align_page(size)
        if padded > len(image) - cursor:
            raise ImageError("Android component padding exceeds image extent")
        cursor += padded
    parts_tuple = tuple(parts)
    if image[0x240:0x260] != android_image_id(parts_tuple):
        raise ImageError("Android v2 image ID mismatch")
    if cursor != len(image):
        raise ImageError("trailing bytes after Android v2 components")
    return header, parts_tuple


def build_android_v2(header_template: bytes, bol4_kernel: bytes, dtb: bytes,
                     initrd: bytes = b"", command_line: bytes | None = None,
                     max_image: int = MAX_ANDROID_IMAGE) -> bytes:
    validate_android_v2_header(header_template)
    if not dtb or len(dtb) < 40 or dtb[:4] != b"\xd0\x0d\xfe\xed":
        raise ImageError("DTB component is missing or does not have an FDT header")
    dtb_size = int.from_bytes(dtb[4:8], "big")
    if dtb_size < 40 or dtb_size > len(dtb):
        raise ImageError("DTB totalsize is outside supplied DTB bytes")
    if len(initrd) > 0x100000:
        raise ImageError("initrd exceeds the audited 1 MiB loader limit")
    kernel = unpack_bol4(bol4_kernel)
    validate_arm64_image(kernel)
    head = bytearray(header_template)
    if command_line is not None:
        if len(command_line) >= 512 or b"\0" in command_line:
            raise ImageError("primary Android command line must be shorter than 512 bytes and contain no NUL")
        head[0x40:0x240] = command_line.ljust(512, b"\0")
    parts = (bol4_kernel, initrd, b"", b"", dtb)
    for offset, part in zip(ANDROID_COMPONENT_SIZE_OFFSETS, parts):
        struct.pack_into("<I", head, offset, len(part))
    head[0x240:0x260] = android_image_id(parts)
    image = bytes(head) + b"".join(part + bytes(align_page(len(part)) - len(part)) for part in parts)
    if len(image) > max_image:
        raise ImageError("complete Android v2 image exceeds configured 16 MiB budget")
    parsed_header, parsed_parts = split_android_v2(image, max_image=max_image)
    if parsed_header != bytes(head) or parsed_parts != parts:
        raise ImageError("Android v2 pack round-trip mismatch")
    if unpack_bol4(parsed_parts[0]) != kernel:
        raise ImageError("Android v2 kernel component round-trip mismatch")
    return image


def manifest_for(header_template: bytes, kernel: bytes, bol4: bytes, image: bytes,
                 dtb: bytes, initrd: bytes) -> dict:
    return {
        "format": "RG34XX BaseOS-native BOL4 in Android boot header v2",
        "host_only": True,
        "device_io_performed": False,
        "kernel_bytes": len(kernel),
        "kernel_sha256": hashlib.sha256(kernel).hexdigest(),
        "bol4_bytes": len(bol4),
        "bol4_sha256": hashlib.sha256(bol4).hexdigest(),
        "bol4_packed_bytes": _u32(bol4, 4),
        "bol4_packed_fnv1a32": f"0x{_u32(bol4, 12):08x}",
        "initrd_bytes": len(initrd),
        "dtb_bytes": len(dtb),
        "android_header_template_sha256": hashlib.sha256(header_template).hexdigest(),
        "android_image_bytes": len(image),
        "android_image_sha256": hashlib.sha256(image).hexdigest(),
        "android_image_id": image[0x240:0x260].hex(),
        "not_installable": True,
        "not_hardware_validated": True,
    }


def command_build(args: argparse.Namespace) -> int:
    kernel = args.kernel.read_bytes()
    header = args.header.read_bytes()
    dtb = args.dtb.read_bytes()
    initrd = args.initrd.read_bytes() if args.initrd else b""
    bol4 = build_bol4(kernel, max_frame=args.max_image_bytes)
    image = build_android_v2(header, bol4, dtb, initrd,
                             args.command_line.encode("ascii") if args.command_line is not None else None,
                             max_image=args.max_image_bytes)
    manifest = manifest_for(header, kernel, bol4, image, dtb, initrd)
    bol4_path = args.bol4_output or args.output.with_suffix(args.output.suffix + ".bol4")
    manifest_path = args.manifest or args.output.with_suffix(args.output.suffix + ".json")
    input_paths = {path.resolve() for path in (args.kernel, args.header, args.dtb) if path is not None}
    if args.initrd is not None:
        input_paths.add(args.initrd.resolve())
    output_paths = [args.output.resolve(), bol4_path.resolve(), manifest_path.resolve()]
    if len(set(output_paths)) != len(output_paths) or any(path in input_paths for path in output_paths):
        raise ImageError("output paths must be distinct and must not overwrite any input file")
    for path in (args.output, bol4_path, manifest_path):
        path.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(image)
    bol4_path.write_bytes(bol4)
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(manifest, indent=2))
    return 0


def command_inspect(args: argparse.Namespace) -> int:
    image = args.image.read_bytes()
    header, parts = split_android_v2(image, max_image=args.max_image_bytes)
    kernel = unpack_bol4(parts[0])
    report = {
        "android_image_bytes": len(image),
        "kernel_bytes": len(kernel),
        "kernel_sha256": hashlib.sha256(kernel).hexdigest(),
        "bol4_bytes": len(parts[0]),
        "initrd_bytes": len(parts[1]),
        "dtb_bytes": len(parts[4]),
        "page_size": _u32(header, 36),
        "header_version": _u32(header, 40),
        "header_size": _u32(header, ANDROID_HEADER_SIZE_OFFSET),
        "image_id": image[0x240:0x260].hex(),
        "host_only": True,
        "not_installable": True,
    }
    print(json.dumps(report, indent=2))
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    build = sub.add_parser("build", help="wrap an AArch64 Image in BOL4 and Android v2")
    build.add_argument("--kernel", type=Path, required=True)
    build.add_argument("--header", type=Path, required=True,
                       help="verified 2048-byte RG34XX Android v2 header template")
    build.add_argument("--dtb", type=Path, required=True)
    build.add_argument("--initrd", type=Path)
    build.add_argument("--command-line",
                       help="optional replacement for the primary 512-byte command-line field")
    build.add_argument("--output", type=Path, required=True)
    build.add_argument("--bol4-output", type=Path)
    build.add_argument("--manifest", type=Path)
    build.add_argument("--max-image-bytes", type=int, default=MAX_ANDROID_IMAGE)
    build.set_defaults(func=command_build)
    inspect = sub.add_parser("inspect", help="verify an Android v2 image and its BOL4 kernel")
    inspect.add_argument("image", type=Path)
    inspect.add_argument("--max-image-bytes", type=int, default=MAX_ANDROID_IMAGE)
    inspect.set_defaults(func=command_inspect)
    args = parser.parse_args(argv)
    try:
        return args.func(args)
    except (OSError, UnicodeError, struct.error, ImageError) as exc:
        parser.exit(2, f"h700_boot_image: {exc}\n")


if __name__ == "__main__":
    raise SystemExit(main())
