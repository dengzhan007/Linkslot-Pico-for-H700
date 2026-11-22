#!/usr/bin/env python3
import unittest

import compare_rg34xx_fdt_sources as compare
import inspect_android_bootimg as android
from test_android_bootimg import make_boot_image
from test_parse_h700_toc1 import make_package


class FdtSourceComparisonTests(unittest.TestCase):
    def test_compares_identical_and_changed_property_values(self):
        left = {
            "/": {"compatible": b"allwinner,h616\0", "model": b"sun50iw9\0"},
            "/chosen": {"bootargs": b"console=ttyS0\0"},
        }
        right = {
            "/": {"compatible": b"allwinner,h616\0", "model": b"sun50iw9-new\0"},
            "/chosen": {"bootargs": b"console=ttyS0\0", "linux,initrd-start": b"\x00" * 8},
            "/memory@40000000": {"reg": b"\x00" * 16},
        }
        result = compare.compare_property_maps(left, right)
        self.assertEqual(result["shared_node_paths"], 2)
        self.assertEqual(result["identical_property_values"], 2)
        self.assertEqual(result["changed_property_values"], 1)
        self.assertEqual(result["left_only_node_paths"], [])
        self.assertEqual(result["right_only_node_paths"], ["/memory@40000000"])
        self.assertEqual(result["right_only_properties"][0]["property"], "linux,initrd-start")

    def test_compares_android_appended_toc1_and_live_fdt_inputs(self):
        boot = make_boot_image()
        boot_report = android.inspect_android_boot_image(boot)
        dtb_info = boot_report["components"]["dtb"]
        dtb_offset = int(dtb_info["offset"], 16)
        dtb = boot[dtb_offset:dtb_offset + dtb_info["size"]]
        package = make_package([
            ("u-boot", b"u-boot"),
            ("monitor", b"monitor"),
            ("dtbo", b"dtbo"),
            ("dtb", dtb),
        ])
        report = compare.compare_sources(boot, package, dtb)
        self.assertEqual(
            report["sources"]["android_p4_appended_dtb"]["sha256"],
            report["sources"]["toc1_dtb"]["sha256"],
        )
        self.assertEqual(
            report["sources"]["toc1_dtb"]["sha256"],
            report["sources"]["live_kernel_fdt"]["sha256"],
        )
        pair = report["pairwise_property_comparison"][
            "toc1_dtb__vs__live_kernel_fdt"
        ]
        self.assertEqual(pair["identity_ratio_over_shared_keys"], 1.0)
        self.assertEqual(pair["changed_property_values"], 0)


if __name__ == "__main__":
    unittest.main()
