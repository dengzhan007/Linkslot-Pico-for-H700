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
        self.assertEqual(len(report["embedded_egon_components"]), 1)
        self.assertTrue(report["embedded_egon_components"][0]["checksum_valid_standard_egon"])
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



    def test_detects_nested_boot0_and_nonstandard_spl2_component(self):
        outer = bytearray(make_boot0(0x20000))
        inner = bytearray(make_boot0(0x10000))

        def profile(clock, dram_type):
            words = [0] * 32
            words[0] = clock
            words[1] = dram_type
            words[2] = 0x08080808
            words[3] = 0x0E0E0E0E
            words[4] = 0x00000E0E
            words[5] = 0x7887BBBB
            words[6] = 0x000030FA
            return words

        inner[0x348:0x350] = b"DRAM.ext"
        for index in range(15):
            dram_type = 7 if index == 13 else 8 if index == 14 else 3
            clock = 672 if index >= 13 else 792
            struct.pack_into("<32I", inner, 0x348 + 44 + index * 128, *profile(clock, dram_type))
        struct.pack_into("<I", inner, MODULE.CHECKSUM_OFFSET, 0)
        struct.pack_into(
            "<I", inner, MODULE.CHECKSUM_OFFSET, MODULE.allwinner_checksum(bytes(inner))
        )

        outer[0x1000:0x11000] = inner
        spl_offset = 0x11000
        outer[spl_offset + 4:spl_offset + 12] = b"eGON.BT0"
        struct.pack_into("<I", outer, spl_offset + 0x10, 0xC000)
        outer[spl_offset + 0x14:spl_offset + 0x18] = b"SPL\x02"
        struct.pack_into("<I", outer, spl_offset + 0x20, 0x2C)
        platform = b"allwinner/sun50i-h700-anbernic-rg35xx-2024\0"
        outer[spl_offset + 0x2C:spl_offset + 0x2C + len(platform)] = platform

        struct.pack_into("<I", outer, MODULE.CHECKSUM_OFFSET, 0)
        struct.pack_into(
            "<I", outer, MODULE.CHECKSUM_OFFSET, MODULE.allwinner_checksum(bytes(outer))
        )
        report = MODULE.parse_boot0(bytes(outer))
        components = {item["offset"]: item for item in report["embedded_egon_components"]}

        self.assertEqual(set(components), {"0x0", "0x1000", "0x11000"})
        self.assertTrue(components["0x0"]["checksum_valid_standard_egon"])
        self.assertTrue(components["0x1000"]["checksum_valid_standard_egon"])
        self.assertEqual(components["0x1000"]["declared_size"], 0x10000)
        self.assertEqual(components["0x1000"]["dram_ext_tables"][0]["marker_offset"], "0x348")
        self.assertEqual(components["0x1000"]["dram_ext_tables"][0]["valid_entry_count"], 15)
        spl = components["0x11000"]
        self.assertEqual(spl["component_kind"], "SPL2 component")
        self.assertIsNone(spl["checksum_valid_standard_egon"])
        self.assertEqual(spl["platform_string"], "allwinner/sun50i-h700-anbernic-rg35xx-2024")

if __name__ == "__main__":
    unittest.main()
