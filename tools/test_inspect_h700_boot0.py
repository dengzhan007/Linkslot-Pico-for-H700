#!/usr/bin/env python3
"""Synthetic tests for the RG34XX H700 Boot0 inspector."""

import hashlib
import importlib.util
import struct
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).with_name("inspect_h700_boot0.py")
SPEC = importlib.util.spec_from_file_location("inspect_h700_boot0", MODULE_PATH)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def make_boot0(size=MODULE.BOOT0_BYTES):
    image = bytearray(size)
    image[4:12] = b"eGON.BT0"
    struct.pack_into("<I", image, MODULE.IMAGE_SIZE_OFFSET, size)
    image[MODULE.DEBUG_OFFSET] = 3
    primary = [0x2A0, 8] + [0x100 + i for i in range(30)]
    lpddr3 = [0x2A0, 7] + [0x200 + i for i in range(30)]
    lpddr4 = [0x2A0, 8] + [0x300 + i for i in range(30)]
    for offset, words in (
        (0x38, primary),
        (0x9F4, lpddr3),
        (0xA74, lpddr4),
    ):
        struct.pack_into("<32I", image, offset, *words)
    struct.pack_into("<I", image, MODULE.CHECKSUM_OFFSET, 0)
    checksum = MODULE.allwinner_checksum(bytes(image))
    struct.pack_into("<I", image, MODULE.CHECKSUM_OFFSET, checksum)
    return bytes(image)


class Boot0InspectorTests(unittest.TestCase):
    def test_valid_boot0_and_dram_candidates(self):
        data = make_boot0()
        report = MODULE.parse_boot0(data)
        self.assertEqual(report["bytes"], 65536)
        self.assertTrue(report["checksum_valid"])
        self.assertEqual(report["debug_byte"], 3)
        self.assertEqual(
            [entry["dram_type"] for entry in report["dram_candidates"]],
            ["LPDDR4", "LPDDR3", "LPDDR4"],
        )
        self.assertEqual(
            report["sha256_body_after_first_sector"],
            hashlib.sha256(data[512:]).hexdigest(),
        )

    def test_accepts_128kib_boot0_declared_by_header(self):
        data = make_boot0(0x20000)
        report = MODULE.parse_boot0(data)
        self.assertEqual(report["bytes"], 0x20000)
        self.assertEqual(report["captured_bytes"], 0x20000)
        self.assertTrue(report["checksum_valid"])
        self.assertEqual(report["sha256_full"], hashlib.sha256(data).hexdigest())

    def test_crops_capture_tail_to_header_declared_size(self):
        image = make_boot0(MODULE.BOOT0_BYTES)
        captured = image + bytes(0x10000)
        report = MODULE.parse_boot0(captured)
        self.assertEqual(report["bytes"], MODULE.BOOT0_BYTES)
        self.assertEqual(report["captured_bytes"], 0x20000)
        self.assertTrue(report["checksum_valid"])
        self.assertEqual(report["sha256_full"], hashlib.sha256(image).hexdigest())

    def test_first_sector_changes_do_not_change_stable_body_hash(self):
        original = make_boot0()
        changed = bytearray(original)
        changed[0x38 + 4] ^= 1
        struct.pack_into("<I", changed, MODULE.CHECKSUM_OFFSET, 0)
        struct.pack_into(
            "<I",
            changed,
            MODULE.CHECKSUM_OFFSET,
            MODULE.allwinner_checksum(bytes(changed)),
        )
        before = MODULE.parse_boot0(original)
        after = MODULE.parse_boot0(bytes(changed))
        self.assertNotEqual(before["sha256_full"], after["sha256_full"])
        self.assertEqual(
            before["sha256_body_after_first_sector"],
            after["sha256_body_after_first_sector"],
        )

    def test_rejects_bad_signature(self):
        data = bytearray(make_boot0())
        data[4:12] = b"BAD!BT00"
        struct.pack_into("<I", data, MODULE.CHECKSUM_OFFSET, 0)
        struct.pack_into(
            "<I", data, MODULE.CHECKSUM_OFFSET, MODULE.allwinner_checksum(bytes(data))
        )
        with self.assertRaisesRegex(MODULE.Boot0Error, "signature"):
            MODULE.parse_boot0(bytes(data))

    def test_rejects_bad_checksum(self):
        data = bytearray(make_boot0())
        data[0x200] ^= 1
        with self.assertRaisesRegex(MODULE.Boot0Error, "checksum mismatch"):
            MODULE.parse_boot0(bytes(data))

    def test_rejects_wrong_image_size(self):
        data = bytearray(make_boot0())
        struct.pack_into("<I", data, MODULE.IMAGE_SIZE_OFFSET, 0x8000)
        struct.pack_into("<I", data, MODULE.CHECKSUM_OFFSET, 0)
        struct.pack_into(
            "<I", data, MODULE.CHECKSUM_OFFSET, MODULE.allwinner_checksum(bytes(data))
        )
        with self.assertRaisesRegex(MODULE.Boot0Error, "declares"):
            MODULE.parse_boot0(bytes(data))

    def test_rejects_short_input(self):
        with self.assertRaisesRegex(MODULE.Boot0Error, "expected a"):
            MODULE.parse_boot0(bytes(1024))


    def test_scans_relocated_dram_ext_tables_and_stops_at_invalid_record(self):
        data = bytearray(make_boot0(0x20000))

        def profile(clock, dram_type, dx_odt=0x08080808, dx_dri=0x0E0E0E0E):
            words = [0] * 32
            words[0] = clock
            words[1] = dram_type
            words[2] = dx_odt
            words[3] = dx_dri
            words[4] = 0x00000E0E
            words[5] = 0x7887BBBB
            words[6] = 0x000030FA
            words[7] = 0
            return words

        # First marker contains one valid profile followed by code-like bytes.
        data[0x348:0x350] = b"DRAM.ext"
        struct.pack_into("<32I", data, 0x348 + 44, *profile(792, 3))
        invalid = [0xE58F0090] * 32
        invalid[0] = 792
        invalid[1] = 3
        struct.pack_into("<32I", data, 0x348 + 44 + 128, *invalid)

        # A second marker at a relocated offset contains the full 15-profile table.
        data[0x1348:0x1350] = b"DRAM.ext"
        for index in range(15):
            if index < 13:
                words = profile(792, 3)
            elif index == 13:
                words = profile(672, 7, 0x06060606, 0x0D0D0D0D)
            else:
                words = profile(672, 8)
            struct.pack_into("<32I", data, 0x1348 + 44 + index * 128, *words)

        struct.pack_into("<I", data, MODULE.CHECKSUM_OFFSET, 0)
        struct.pack_into(
            "<I", data, MODULE.CHECKSUM_OFFSET, MODULE.allwinner_checksum(bytes(data))
        )
        report = MODULE.parse_boot0(bytes(data))
        tables = report["dram_ext_tables"]
        first = next(table for table in tables if table["marker_offset"] == "0x348")
        relocated = next(table for table in tables if table["marker_offset"] == "0x1348")
        self.assertEqual(first["valid_entry_count"], 1)
        self.assertEqual(relocated["valid_entry_count"], 15)
        self.assertEqual(relocated["entries"][13]["dram_type"], "LPDDR3")
        self.assertEqual(relocated["entries"][14]["dram_type"], "LPDDR4")
        self.assertEqual(relocated["entries"][14]["clock_mhz"], 672)


if __name__ == "__main__":
    unittest.main()
