#!/usr/bin/env python3
import gzip
import hashlib
import json
import struct
import tempfile
import unittest
from pathlib import Path

import inspect_android_bootimg as bootimg


def make_fdt() -> bytes:
    strings = b"model\0compatible\0bootargs\0device_type\0reg\0"
    names = {name: strings.index(name.encode() + b"\0") for name in
             ("model", "compatible", "bootargs", "device_type", "reg")}

    def node(name: str) -> bytes:
        raw = name.encode() + b"\0"
        return struct.pack(">I", 1) + raw + bytes((-len(raw)) % 4)

    def prop(name: str, value: bytes) -> bytes:
        raw = struct.pack(">III", 3, len(value), names[name]) + value
        return raw + bytes((-len(raw)) % 4)

    structure = bytearray(node(""))
    structure += prop("model", b"Test H700\0")
    structure += prop("compatible", b"allwinner,h616\0arm,sun50iw9p1\0")
    structure += node("chosen")
    structure += prop("bootargs", b"console=ttyS0,115200\0")
    structure += struct.pack(">I", 2)
    structure += node("memory@40000000")
    structure += prop("device_type", b"memory\0")
    structure += prop("reg", struct.pack(">4I", 0, 0x40000000, 0, 0x40000000))
    structure += struct.pack(">I", 2)  # end memory node
    structure += struct.pack(">I", 2)  # end root node
    structure += struct.pack(">I", 9)  # FDT_END

    reserve_offset = 40
    reserve_map = bytes(16)
    structure_offset = reserve_offset + len(reserve_map)
    strings_offset = structure_offset + len(structure)
    total = strings_offset + len(strings)
    header = struct.pack(
        ">10I", bootimg.FDT_MAGIC, total, structure_offset, strings_offset,
        reserve_offset, 17, 16, 0, len(strings), len(structure)
    )
    return header + reserve_map + bytes(structure) + strings


def make_arm64_image() -> bytes:
    image = bytearray(128)
    struct.pack_into("<2I3Q", image, 0, 0x14000010, 0, 0x80000, 0x100000, 0xA)
    image[0x38:0x3C] = b"ARM\x64"
    version = b"Linux version 4.9.170 test\0"
    image[64:64 + len(version)] = version
    return bytes(image)


def make_boot_image() -> bytes:
    page = 2048
    parts = [
        gzip.compress(make_arm64_image(), mtime=0),
        b"R" * 512,
        b"",
        b"",
        make_fdt(),
    ]
    header = bytearray(page)
    header[:8] = bootimg.ANDROID_MAGIC
    for offset, part in zip((8, 16, 24, 0x660, 0x670), parts):
        struct.pack_into("<I", header, offset, len(part))
    struct.pack_into("<I", header, 0x0C, 0x40080000)
    struct.pack_into("<I", header, 0x14, 0x42000000)
    struct.pack_into("<I", header, 0x1C, 0x40F00000)
    struct.pack_into("<I", header, 0x20, 0x40000100)
    struct.pack_into("<3I", header, 0x24, page, 2, 0)
    header[0x30:0x40] = b"sun50i_arm64\0".ljust(16, b"\0")
    header[0x40:0x240] = b"loglevel=1 console=tty\0".ljust(512, b"\0")
    struct.pack_into("<Q", header, 0x664, 0)
    struct.pack_into("<I", header, 0x66C, 0x67C)
    struct.pack_into("<I", header, 0x674, 0x44000000)
    header[0x240:0x260] = bootimg.android_image_id(parts)
    return bytes(header) + b"".join(
        part + bytes(bootimg.align(len(part), page) - len(part)) for part in parts
    )


class AndroidBootImageTests(unittest.TestCase):
    def test_inspects_v2_layout_gzip_kernel_arm64_and_fdt(self):
        image = make_boot_image()
        report = bootimg.inspect_android_boot_image(image)
        self.assertEqual(report["format"], "Android boot image v2")
        self.assertTrue(report["header"]["android_image_id_valid"])
        self.assertEqual(report["header"]["kernel_addr"], "0x40080000")
        self.assertEqual(report["components"]["kernel"]["offset"], "0x800")
        self.assertEqual(report["kernel"]["compression"], "gzip")
        self.assertEqual(report["kernel"]["arm64_image"]["magic"], "ARM\\x64")
        self.assertIn("Linux version 4.9.170", report["kernel"]["arm64_image"]["linux_version"])
        self.assertEqual(report["dtb"]["model"], "Test H700")
        self.assertIn("allwinner,h616", report["dtb"]["compatible"])
        self.assertEqual(report["dtb"]["chosen_bootargs"], "console=ttyS0,115200")
        self.assertEqual(report["dtb"]["memory_nodes"][0]["path"], "/memory@40000000")

    def test_rejects_bad_magic(self):
        with self.assertRaisesRegex(bootimg.BootImageError, "ANDROID"):
            bootimg.inspect_android_boot_image(b"NOTBOOT!" + bytes(4096))

    def test_rejects_corrupted_android_image_id(self):
        image = bytearray(make_boot_image())
        image[0x240] ^= 1
        with self.assertRaisesRegex(bootimg.BootImageError, "image ID"):
            bootimg.inspect_android_boot_image(bytes(image))

    def test_rejects_truncated_component(self):
        with self.assertRaisesRegex(bootimg.BootImageError, "truncated"):
            bootimg.inspect_android_boot_image(make_boot_image()[:-32])

    def test_rejects_mismatched_fdt_totalsize(self):
        image = bytearray(make_boot_image())
        page = 2048
        kernel_size = struct.unpack_from("<I", image, 8)[0]
        ramdisk_size = struct.unpack_from("<I", image, 16)[0]
        dtb_offset = bootimg.align(bootimg.align(page + kernel_size, page) + ramdisk_size, page)
        struct.pack_into(">I", image, dtb_offset + 4, len(make_fdt()) + 1)
        parts = []
        cursor = page
        for field in (8, 16, 24, 0x660, 0x670):
            size = struct.unpack_from("<I", image, field)[0]
            parts.append(bytes(image[cursor:cursor + size]))
            cursor = bootimg.align(cursor + size, page)
        image[0x240:0x260] = bootimg.android_image_id(parts)
        with self.assertRaisesRegex(bootimg.BootImageError, "FDT totalsize"):
            bootimg.inspect_android_boot_image(bytes(image))

    def test_extracts_components_only_to_new_directory(self):
        image = make_boot_image()
        report = bootimg.inspect_android_boot_image(image)
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / "parts"
            bootimg.extract_components(image, report, output)
            kernel_offset = int(report["components"]["kernel"]["offset"], 16)
            kernel_size = report["components"]["kernel"]["size"]
            self.assertEqual((output / "kernel.bin").read_bytes(),
                             image[kernel_offset:kernel_offset + kernel_size])
            manifest = json.loads((output / "manifest.json").read_text())
            self.assertEqual(manifest["image_sha256"], hashlib.sha256(image).hexdigest())
            with self.assertRaisesRegex(bootimg.BootImageError, "already exists"):
                bootimg.extract_components(image, report, output)


if __name__ == "__main__":
    unittest.main()
