#!/usr/bin/env python3
import struct
import tempfile
import unittest
import zipfile
from pathlib import Path

import audit_rg34xx_uboot_smc as audit


def a32_bl(offset: int, target: int) -> bytes:
    immediate = ((target - (offset + 8)) >> 2) & 0x00FFFFFF
    return struct.pack("<I", 0xEB000000 | immediate)


def a32_mov_half(base: int, register: int, immediate: int) -> bytes:
    word = base | (((immediate >> 12) & 0xF) << 16) | (register << 12) | (immediate & 0xFFF)
    return struct.pack("<I", word)


def make_toc1() -> bytes:
    uboot = bytearray(0xC00)
    uboot[0x40:0x44] = struct.pack("<I", 0x8000FF04)
    uboot[0x80:0x84] = audit.A32_SMC0
    uboot[0xA0:0xA4] = a32_mov_half(0xE3000000, 0, 0xFF04)  # MOVW r0, #0xFF04
    uboot[0xA4:0xA8] = a32_mov_half(0xE3400000, 0, 0x8000)  # MOVT r0, #0x8000
    uboot[0xC0:0xC4] = a32_bl(0xC0, 0xB18)
    uboot[0xC4:0xC8] = a32_bl(0xC4, 0xB58)
    uboot[0xB24:0xB28] = audit.A32_SMC0
    uboot[0xB5C:0xB60] = audit.A32_SMC0
    data = {
        "u-boot": bytes(uboot),
        "monitor": b"M" * 128,
        "dtbo": b"D" * 64,
        "dtb": b"T" * 96,
    }
    table_end = audit.ITEM_TABLE_OFFSET + audit.ITEM_SIZE * 4
    package = bytearray(table_end)
    package[:16] = audit.PACKAGE_MAGIC
    struct.pack_into("<I", package, 16, 0x89119800)
    struct.pack_into("<I", package, 32, 4)
    package[60:64] = b"MIE;"
    cursor = table_end
    for index, (name, blob) in enumerate(data.items()):
        cursor = (cursor + 3) & ~3
        at = audit.ITEM_TABLE_OFFSET + audit.ITEM_SIZE * index
        package[at:at + len(name)] = name.encode()
        struct.pack_into("<4I", package, at + 64, cursor, len(blob), 0, 3)
        package[at + audit.ITEM_SIZE - 4:at + audit.ITEM_SIZE] = b"IIE;"
        package.extend(bytes(cursor - len(package)))
        package.extend(blob)
        cursor += len(blob)
    package.extend(bytes((-len(package)) % 4))
    struct.pack_into("<I", package, 36, len(package))
    words = struct.unpack(f"<{len(package) // 4}I", package)
    checksum = (sum(words) + audit.CHECKSUM_STAMP) & 0xFFFFFFFF
    struct.pack_into("<I", package, 20, checksum)
    return bytes(package)


class UbootSmcAuditTests(unittest.TestCase):
    def test_extracts_toc1_and_finds_a32_smc_pattern(self):
        items = audit.extract_toc1_items(make_toc1())
        self.assertEqual(set(items), {"u-boot", "monitor", "dtbo", "dtb"})
        report = audit.scan_uboot(items["u-boot"])
        self.assertEqual(report["a32_smc0_sites"][0]["file_offset"], "0x80")
        self.assertEqual(
            report["constant_occurrences"]["vendor_linux_smc_0x8000ff04"],
            ["0x40"],
        )
        self.assertEqual(
            report["direct_callers_of_smc_wrappers"]["0xb18"][0]["callsite"],
            "0xc0",
        )
        self.assertEqual(
            report["direct_callers_of_smc_wrappers"]["0xb58"][0]["callsite"],
            "0xc4",
        )
        self.assertEqual(
            report["movw_movt_constructed_linux_smc_id"][0]["value"],
            "0x8000ff04",
        )

    def test_traces_ldr_literal_reference_to_linux_smc_id(self):
        image = bytearray(0x1600)
        target = 0x1510
        instruction = 0x520
        immediate = target - (instruction + 8)
        struct.pack_into("<I", image, instruction, 0xE59F0000 | immediate)
        struct.pack_into("<I", image, target, 0x8000FF04)
        refs = audit.find_a32_ldr_literal_refs(bytes(image), target)
        self.assertEqual(len(refs), 1)
        self.assertEqual(refs[0]["instruction_offset"], "0x520")

    def test_finds_movw_movt_pair_for_linux_smc_id(self):
        image = bytearray(64)
        movw = 0xE3000000 | (0xF << 16) | 0xF04
        movt = 0xE3400000 | (0x8 << 16)
        struct.pack_into("<II", image, 0x10, movw, movt)
        matches = audit.find_movw_movt_smc_id(bytes(image))
        self.assertEqual(matches[0]["value"], "0x8000ff04")

    def test_finds_pointer_to_wrapper_at_reference_load_base(self):
        image = bytearray(64)
        struct.pack_into("<I", image, 0x20, 0x4A000B18)
        self.assertEqual(
            audit.find_word_pointer_refs(bytes(image), 0x4A000B18),
            ["0x20"],
        )
    def test_reads_toc1_from_inventory_zip(self):
        package = make_toc1()
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "inventory.zip"
            with zipfile.ZipFile(path, "w") as archive:
                archive.writestr("reports/toc1-package.bin", package)
            extracted, member = audit.find_toc1(path)
            self.assertEqual(extracted, package)
            self.assertIn("toc1-package.bin", member)

    def test_rejects_bad_checksum(self):
        package = bytearray(make_toc1())
        package[-1] ^= 1
        with self.assertRaisesRegex(audit.AuditError, "checksum"):
            audit.extract_toc1_items(bytes(package))


if __name__ == "__main__":
    unittest.main()
