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


def make_package(items, *, count_override=None, size_override=None, overlap=False):
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
    if size_override is not None:
        declared = size_override
    else:
        declared = total
    blob = bytearray(total)
    struct.pack_into("<II", blob, 32, count, declared)
    for index, (offset, end, name, payload) in enumerate(entries):
        at = MODULE.HEADER_BYTES + index * MODULE.ITEM_BYTES
        encoded = name.encode("ascii")
        blob[at : at + len(encoded)] = encoded
        struct.pack_into("<II", blob, at + 64, offset, len(payload))
        blob[offset:end] = payload
    return bytes(blob)


class Toc1ParserTests(unittest.TestCase):
    def test_parse_and_extract_items(self):
        original = make_package(
            [
                ("u-boot", b"U-BOOT-A32"),
                ("monitor", b"VENDOR-BL31"),
                ("dtb", b"FDT-DATA"),
                ("dtbo", b"OVERLAY"),
            ]
        )
        parsed = MODULE.parse_toc1(original)
        self.assertEqual(parsed["declared_item_count"], 4)
        self.assertEqual(parsed["package_sha256"], hashlib.sha256(original).hexdigest())
        self.assertEqual(
            {entry["name"] for entry in parsed["items"]},
            {"u-boot", "monitor", "dtb", "dtbo"},
        )
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "extract"
            # Exercise the public extractor through a temporary input file.
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

    def test_rejects_invalid_count(self):
        blob = bytearray(make_package([("u-boot", b"x")]))
        struct.pack_into("<I", blob, 32, MODULE.ITEM_LIMIT + 1)
        with self.assertRaises(MODULE.Toc1Error):
            MODULE.parse_toc1(bytes(blob))

    def test_rejects_declared_size_mismatch(self):
        blob = make_package([("u-boot", b"x")], size_override=0x140000)
        with self.assertRaises(MODULE.Toc1Error):
            MODULE.parse_toc1(blob)

    def test_rejects_duplicate_names(self):
        with self.assertRaises(MODULE.Toc1Error):
            MODULE.parse_toc1(make_package([("dtb", b"a"), ("dtb", b"b")]))

    def test_rejects_unsafe_name(self):
        with self.assertRaises(MODULE.Toc1Error):
            MODULE.parse_toc1(make_package([("../boot0", b"x")]))

    def test_rejects_overlapping_payloads(self):
        with self.assertRaises(MODULE.Toc1Error):
            MODULE.parse_toc1(
                make_package([("u-boot", b"first"), ("monitor", b"second")], overlap=True)
            )

    def test_rejects_item_past_package_end(self):
        blob = bytearray(make_package([("u-boot", b"x")]))
        struct.pack_into("<I", blob, MODULE.HEADER_BYTES + 64, len(blob))
        with self.assertRaises(MODULE.Toc1Error):
            MODULE.parse_toc1(bytes(blob))


if __name__ == "__main__":
    unittest.main()
