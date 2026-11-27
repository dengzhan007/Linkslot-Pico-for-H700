#!/usr/bin/env python3
"""Host regression for the RG34XX A32 BL33 header wrapper."""
from __future__ import annotations

import importlib.util
import struct
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("package-h700-bl33.py")
SPEC = importlib.util.spec_from_file_location("package_h700_bl33", SCRIPT)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def branch(pc: int, target: int) -> bytes:
    delta = target - (pc + 8)
    if delta % 4:
        raise ValueError("unaligned branch target")
    imm = (delta >> 2) & 0x00FFFFFF
    return struct.pack("<I", 0xEA000000 | imm)


def fixtures() -> tuple[bytes, bytes]:
    reference = bytearray(0x8000)
    reference[4:9] = b"uboot"
    # Retain representative non-zero vendor fields, including the runtime
    # SPL-patched DRAM / RAM-size area; semantics beyond the documented
    # patch offsets are intentionally opaque.
    reference[0x0C:0x10] = bytes.fromhex("82744ca2")
    reference[0x30:0xB0] = bytes(range(128))
    reference[0x4E8:0x4EB] = b"xyz"

    raw = bytearray(0x3000)
    raw[0:4] = branch(MODULE.IMAGE_BASE, MODULE.IMAGE_BASE + 0x1020)
    for index in range(8):
        pc = MODULE.IMAGE_BASE + MODULE.VECTOR_OFFSET + index * 4
        target = MODULE.IMAGE_BASE + 0x1100 + index * 4
        raw[MODULE.VECTOR_OFFSET + index * 4:MODULE.VECTOR_OFFSET + index * 4 + 4] = branch(pc, target)
    raw[0x2000:0x2010] = b"LK-RAW-CODE-TEST"
    return bytes(raw), bytes(reference)


class PackageH700Bl33Tests(unittest.TestCase):
    def test_copies_only_vendor_header_and_preserves_entry_and_vectors(self) -> None:
        raw, reference = fixtures()
        output, report = MODULE.package_image(
            raw, reference, require_pinned_reference=False
        )
        self.assertEqual(output[:4], raw[:4])
        self.assertEqual(output[4:0x640], reference[4:0x640])
        self.assertEqual(output[0x640:0x1000], raw[0x640:0x1000])
        self.assertEqual(output[0x1000:0x1020], raw[0x1000:0x1020])
        self.assertEqual(output[0x2000:0x2010], raw[0x2000:0x2010])
        self.assertEqual(len(output) % 512, 0)
        self.assertEqual(report["entry_branch_target"], hex(MODULE.IMAGE_BASE + 0x1020))
        self.assertEqual(report["header_magic"], "uboot")

    def test_rejects_non_pinned_reference_for_release_mode(self) -> None:
        raw, reference = fixtures()
        with self.assertRaisesRegex(ValueError, "SHA-256"):
            MODULE.package_image(raw, reference, require_pinned_reference=True)

    def test_rejects_wrong_magic(self) -> None:
        raw, reference = fixtures()
        reference = bytearray(reference)
        reference[4:9] = b"other"
        with self.assertRaisesRegex(ValueError, "uboot"):
            MODULE.package_image(raw, bytes(reference), require_pinned_reference=False)

    def test_rejects_nonzero_raw_header_reservation(self) -> None:
        raw, reference = fixtures()
        raw = bytearray(raw)
        raw[0x100] = 1
        with self.assertRaisesRegex(ValueError, "reserved/zero"):
            MODULE.package_image(bytes(raw), reference, require_pinned_reference=False)

    def test_rejects_bad_entry_branch(self) -> None:
        raw, reference = fixtures()
        raw = bytearray(raw)
        raw[:4] = b"\0\0\0\0"
        with self.assertRaisesRegex(ValueError, "A32 B"):
            MODULE.package_image(bytes(raw), reference, require_pinned_reference=False)

    def test_rejects_vector_branch_outside_image(self) -> None:
        raw, reference = fixtures()
        raw = bytearray(raw)
        pc = MODULE.IMAGE_BASE + MODULE.VECTOR_OFFSET
        raw[MODULE.VECTOR_OFFSET:MODULE.VECTOR_OFFSET + 4] = branch(pc, MODULE.IMAGE_BASE + 0x800000)
        with self.assertRaisesRegex(ValueError, "vector 0"):
            MODULE.package_image(bytes(raw), reference, require_pinned_reference=False)

    def test_accepts_exact_active_32k_loader_extent(self) -> None:
        raw, reference = fixtures()
        raw = raw + bytes(MODULE.ACTIVE_LOADER_EXTENT_BYTES - len(raw))
        output, report = MODULE.package_image(
            raw, reference, require_pinned_reference=False
        )
        self.assertEqual(report["raw_lk_bytes"], 0x8000)
        self.assertEqual(report["active_loader_extent_bytes"], 0x8000)
        self.assertEqual(report["fits_active_loader_extent"], True)
        self.assertEqual(len(output), 0x8000)

    def test_rejects_image_over_active_32k_loader_extent(self) -> None:
        raw, reference = fixtures()
        raw = raw + bytes(MODULE.ACTIVE_LOADER_EXTENT_BYTES - len(raw) + 1)
        with self.assertRaisesRegex(ValueError, "active native loader extent"):
            MODULE.package_image(raw, reference, require_pinned_reference=False)


if __name__ == "__main__":
    unittest.main(verbosity=2)
