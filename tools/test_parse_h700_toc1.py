#!/usr/bin/env python3
"""Unit tests for the H700 TOC1 parser; uses synthetic packages only."""

import hashlib
import importlib.util
import json
import struct
import tempfile
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).with_name("parse_h700_toc1.py")
SPEC = importlib.util.spec_from_file_location("parse_h700_toc1", MODULE_PATH)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def refresh_checksum(blob):
    struct.pack_into("<I", blob, 20, 0)
    struct.pack_into("<I", blob, 20, MODULE.allwinner_checksum(bytes(blob)))


def valid_items():
    return [
        ("u-boot", b"U-BOOT-A32"),
        ("monitor", b"VENDOR-BL31"),
        ("dtbo", b"OVERLAY"),
        ("dtb", b"FDT-DATA"),
    ]


def make_package(items=None, *, count_override=None, size_override=None, overlap=False):
    items = valid_items() if items is None else items
    count = len(items) if count_override is None else count_override
    table_end = MODULE.HEADER_BYTES + len(items) * MODULE.ITEM_BYTES
    cursor = (table_end + 15) & ~15
    entries = []
    for index, (name, payload) in enumerate(items):
        offset = cursor
        if overlap and index == 1:
            offset = entries[0][0] + 1
        entries.append((offset, offset + len(payload), name, payload))
        cursor = max(cursor, offset + len(payload))
        cursor = (cursor + 15) & ~15

    total = cursor
    declared = total if size_override is None else size_override
    blob = bytearray(total)
    blob[:16] = MODULE.TOC1_MAGIC
    struct.pack_into("<I", blob, 16, MODULE.TOC1_MAGIC_WORD)
    struct.pack_into("<II", blob, 32, count, declared)
    blob[60:64] = b"MIE;"

    for index, (offset, end, name, payload) in enumerate(entries):
        at = MODULE.HEADER_BYTES + index * MODULE.ITEM_BYTES
        encoded = name.encode("ascii")
        blob[at : at + len(encoded)] = encoded
        struct.pack_into("<4I", blob, at + 64, offset, len(payload), 0, 3)
        blob[at + 364 : at + 368] = b"IIE;"
        blob[offset:end] = payload

    refresh_checksum(blob)
    return bytes(blob)


class Toc1ParserTests(unittest.TestCase):
    def test_parse_and_extract_items(self):
        original = make_package()
        parsed = MODULE.parse_toc1(original)
        self.assertEqual(parsed["declared_item_count"], 4)
        self.assertEqual(parsed["package_sha256"], hashlib.sha256(original).hexdigest())
        self.assertEqual(
            {entry["name"] for entry in parsed["items"]},
            {"u-boot", "monitor", "dtb", "dtbo"},
        )
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "extract"
            source = Path(temporary) / "toc1.bin"
            source.write_bytes(original)
            result = MODULE.extract_toc1(source, output)
            self.assertEqual(result["package_sha256"], parsed["package_sha256"])
            self.assertEqual((output / "items/u-boot.bin").read_bytes(), b"U-BOOT-A32")
            saved = json.loads((output / "manifest.json").read_text())
            self.assertEqual(saved["items"][1]["name"], "monitor")

    def test_rejects_truncated_header(self):
        with self.assertRaises(MODULE.Toc1Error):
            MODULE.parse_toc1(b"short")

    def test_rejects_invalid_magic(self):
        blob = bytearray(make_package())
        blob[0] ^= 1
        refresh_checksum(blob)
        with self.assertRaisesRegex(MODULE.Toc1Error, "magic"):
            MODULE.parse_toc1(bytes(blob))

    def test_rejects_invalid_count(self):
        blob = bytearray(make_package(count_override=3))
        with self.assertRaisesRegex(MODULE.Toc1Error, "expected 4"):
            MODULE.parse_toc1(bytes(blob))

    def test_rejects_declared_size_mismatch(self):
        blob = make_package(size_override=0x140000)
        with self.assertRaisesRegex(MODULE.Toc1Error, "declared package size"):
            MODULE.parse_toc1(blob)

    def test_rejects_checksum_mismatch(self):
        blob = bytearray(make_package())
        blob[-1] ^= 0x80
        with self.assertRaisesRegex(MODULE.Toc1Error, "checksum mismatch"):
            MODULE.parse_toc1(bytes(blob))

    def test_rejects_duplicate_names(self):
        items = valid_items()
        items[1] = ("u-boot", b"duplicate")
        with self.assertRaisesRegex(MODULE.Toc1Error, "duplicate item"):
            MODULE.parse_toc1(make_package(items))

    def test_rejects_unsafe_name(self):
        items = valid_items()
        items[0] = ("../boot0", b"unsafe")
        with self.assertRaisesRegex(MODULE.Toc1Error, "unsafe output name"):
            MODULE.parse_toc1(make_package(items))

    def test_rejects_overlapping_payloads(self):
        with self.assertRaisesRegex(MODULE.Toc1Error, "overlap"):
            MODULE.parse_toc1(make_package(overlap=True))

    def test_rejects_item_past_package_end(self):
        blob = bytearray(make_package())
        first_item = MODULE.HEADER_BYTES
        struct.pack_into("<I", blob, first_item + 64, len(blob))
        refresh_checksum(blob)
        with self.assertRaisesRegex(MODULE.Toc1Error, "exceeds package bounds"):
            MODULE.parse_toc1(bytes(blob))

    def test_rejects_missing_item_footer(self):
        blob = bytearray(make_package())
        blob[MODULE.HEADER_BYTES + 364] = 0
        refresh_checksum(blob)
        with self.assertRaisesRegex(MODULE.Toc1Error, "IIE"):
            MODULE.parse_toc1(bytes(blob))


if __name__ == "__main__":
    unittest.main()
