#!/usr/bin/env python3
import struct
import unittest

import pack_aarch64_image as packer


class Aarch64ImagePackerTests(unittest.TestCase):
    def test_packs_valid_header_and_branch_to_entry(self):
        body = bytes.fromhex("1f2003d5") + bytes(124)
        image, report = packer.pack_aarch64_image(body)
        code0, code1 = struct.unpack_from("<II", image, 0)
        text_offset, image_size, flags = struct.unpack_from("<QQQ", image, 8)

        self.assertEqual(code0, 0x14000010)  # B +0x40
        self.assertEqual(code1, 0xD503201F)
        self.assertEqual(text_offset, 0x80000)
        self.assertEqual(image_size, len(image))
        self.assertEqual(flags, 0)
        self.assertEqual(image[56:60], b"ARM\x64")
        self.assertEqual(image[64:68], bytes.fromhex("1f2003d5"))
        self.assertEqual(report["entry_offset"], "0x40")

    def test_rejects_empty_body(self):
        with self.assertRaisesRegex(packer.ImagePackError, "empty"):
            packer.pack_aarch64_image(b"")

    def test_rejects_invalid_entry_offset(self):
        with self.assertRaisesRegex(packer.ImagePackError, "outside"):
            packer.pack_aarch64_image(bytes(64), entry_offset=0x80)

    def test_rejects_entry_offset_before_header_end(self):
        with self.assertRaisesRegex(packer.ImagePackError, ">= 64"):
            packer.pack_aarch64_image(bytes(64), entry_offset=0x20)

    def test_rejects_non_rg34xx_text_offset(self):
        with self.assertRaisesRegex(packer.ImagePackError, "text_offset"):
            packer.pack_aarch64_image(bytes(64), text_offset=0x100000)

    def test_rejects_image_larger_than_conservative_stock_limit(self):
        with self.assertRaisesRegex(packer.ImagePackError, "conservative maximum"):
            packer.pack_aarch64_image(bytes(0x1138000), max_image_size=0x1138000)

    def test_rejects_invalid_page_size(self):
        with self.assertRaisesRegex(packer.ImagePackError, "power of two"):
            packer.pack_aarch64_image(bytes(64), page_size=3000)


if __name__ == "__main__":
    unittest.main()
