#!/usr/bin/env python3
import gzip
import random
import struct
import unittest

import build_rg34xx_bootimg_payload as builder
from test_android_bootimg import make_arm64_image, make_boot_image
import inspect_android_bootimg as android


def make_partition() -> tuple[bytes, int]:
    image = make_boot_image()
    report = android.inspect_android_boot_image(image)
    component_end = int(report["padded_components_end"], 16)
    tail = bytes((i * 17 + 3) & 0xFF for i in range(65536 - component_end))
    return image + tail, component_end


class KernelSlotBuilderTests(unittest.TestCase):
    def test_builds_candidate_and_preserves_non_kernel_components_and_tail(self):
        original, old_end = make_partition()
        payload = make_arm64_image()
        candidate, manifest = builder.build_candidate_partition(original, payload)

        self.assertEqual(len(candidate), len(original))
        self.assertTrue(manifest["android_image_id_valid"])
        self.assertTrue(manifest["unused_partition_tail_preserved"])
        self.assertEqual(candidate[old_end:], original[old_end:])
        self.assertEqual(
            manifest["preserved_components"]["dtb"]["sha256"],
            android.inspect_android_boot_image(original)["components"]["dtb"]["sha256"],
        )

        old_report = android.inspect_android_boot_image(original)
        new_report = android.inspect_android_boot_image(candidate)
        self.assertEqual(new_report["kernel"]["compression"], "gzip")
        self.assertEqual(
            gzip.decompress(
                candidate[
                    int(new_report["components"]["kernel"]["offset"], 16):
                    int(new_report["components"]["kernel"]["offset"], 16)
                    + new_report["components"]["kernel"]["size"]
                ]
            ),
            payload,
        )
        for name in ("ramdisk", "second", "recovery_dtbo", "dtb"):
            self.assertEqual(
                old_report["components"][name]["sha256"],
                new_report["components"][name]["sha256"],
            )
        self.assertEqual(new_report["header"]["kernel_addr"], "0x40080000")
        self.assertEqual(new_report["header"]["ramdisk_addr"], "0x42000000")
        self.assertEqual(new_report["header"]["dtb_addr"], "0x44000000")

    def test_rejects_wrong_text_offset(self):
        original, _ = make_partition()
        payload = bytearray(make_arm64_image())
        struct.pack_into("<Q", payload, 8, 0x100000)
        with self.assertRaisesRegex(builder.PayloadBuildError, "text_offset"):
            builder.build_candidate_partition(original, bytes(payload))

    def test_rejects_payload_without_aarch64_image_magic(self):
        original, _ = make_partition()
        payload = bytearray(make_arm64_image())
        payload[0x38:0x3C] = b"NOPE"
        with self.assertRaisesRegex(builder.PayloadBuildError, "AArch64 Image"):
            builder.build_candidate_partition(original, bytes(payload))

    def test_rejects_invalid_image_entry_branch(self):
        original, _ = make_partition()
        payload = bytearray(make_arm64_image())
        struct.pack_into("<I", payload, 0, 0)
        with self.assertRaisesRegex(builder.PayloadBuildError, "code0"):
            builder.build_candidate_partition(original, bytes(payload))

    def test_rejects_payload_image_size_larger_than_kernel_slot(self):
        original, _ = make_partition()
        payload = bytearray(make_arm64_image())
        struct.pack_into(
            "<Q", payload, 16,
            builder.EXPECTED_RAMDISK_ADDR - builder.EXPECTED_KERNEL_ADDR + 1,
        )
        with self.assertRaisesRegex(builder.PayloadBuildError, "kernel slot"):
            builder.build_candidate_partition(original, bytes(payload))

    def test_rejects_candidate_that_would_consume_original_partition_tail(self):
        original, _ = make_partition()
        # High-entropy, slot-sized input cannot fit inside the original
        # Android component area; the builder must not consume unused p4 tail.
        payload = bytearray(random.Random(7).randbytes(256 * 1024))
        payload[:64] = make_arm64_image()[:64]
        payload[0x38:0x3C] = b"ARM\x64"
        struct.pack_into("<Q", payload, 8, 0x80000)
        struct.pack_into("<Q", payload, 16, len(payload))
        with self.assertRaisesRegex(builder.PayloadBuildError, "beyond original component area"):
            builder.build_candidate_partition(original, bytes(payload))


if __name__ == "__main__":
    unittest.main()
