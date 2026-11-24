#!/usr/bin/env python3
"""Synthetic tests for bounded RG34XX/H700 SID cell decoding."""

import importlib.util
import tempfile
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).with_name("inspect_h700_sid_cells.py")
SPEC = importlib.util.spec_from_file_location("inspect_h700_sid_cells", MODULE_PATH)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class SidCellInspectorTests(unittest.TestCase):
    def test_decodes_bounded_speed_and_thermal_cells_without_approving_opp(self):
        with tempfile.TemporaryDirectory() as tmp:
            source = Path(tmp) / "cpu-power-runtime.txt"
            source.write_text(
                "speed_bin_raw_le_bytes=0024\n"
                "thermal_calibration_raw_bytes=6004231156247837\n",
                encoding="ascii",
            )
            report = MODULE.inspect_sid_cells(source)

        speed = report["cells"]["speed"]["report"]
        self.assertEqual(speed["status"], "captured")
        self.assertEqual(speed["raw_u16_le"], "0x2400")
        self.assertTrue(speed["matches_pinned_vendor_literal"])
        self.assertEqual(speed["opp_authorization"], "not_approved")
        self.assertNotIn("supported_hw_mask", speed)

        thermal = report["cells"]["thermal_calibration"]["report"]
        self.assertEqual(thermal["u16_le_words"], ["0x0460", "0x1123", "0x2456", "0x3778"])
        self.assertEqual(thermal["factory_test_temperature_raw_0p1c"], 0x460)
        self.assertEqual(thermal["sensor0_calibration_raw_12bit"], 0x123)
        self.assertEqual(thermal["sensor1_calibration_raw_12bit"], 0x456)
        self.assertEqual(thermal["sensor2_calibration_raw_12bit"], 0x778)
        self.assertEqual(thermal["sensor3_calibration_raw_12bit"], 0x321)
        self.assertEqual(thermal["calibration_status"], "not_certified")

    def test_unknown_speed_value_stays_unapproved(self):
        with tempfile.TemporaryDirectory() as tmp:
            source = Path(tmp) / "cpu-power-runtime.txt"
            source.write_text("speed_bin_raw_le_bytes=adde\n", encoding="ascii")
            report = MODULE.inspect_sid_cells(source)

        speed = report["cells"]["speed"]["report"]
        self.assertEqual(speed["raw_u16_le"], "0xdead")
        self.assertFalse(speed["matches_pinned_vendor_literal"])
        self.assertEqual(speed["opp_authorization"], "not_approved")

    def test_missing_sid_reads_are_reported_as_unavailable(self):
        with tempfile.TemporaryDirectory() as tmp:
            source = Path(tmp) / "cpu-power-runtime.txt"
            source.write_text(
                "NOT_PRESENT: no NVMEM provider matched sunxi-sid-ng@03006000\n",
                encoding="ascii",
            )
            report = MODULE.inspect_sid_cells(source)

        self.assertEqual(report["cells"]["speed"]["report"]["status"], "unavailable")
        self.assertEqual(report["cells"]["thermal_calibration"]["report"]["status"], "unavailable")

    def test_rejects_wrong_cell_lengths(self):
        with tempfile.TemporaryDirectory() as tmp:
            source = Path(tmp) / "cpu-power-runtime.txt"
            source.write_text("speed_bin_raw_le_bytes=240000\n", encoding="ascii")
            with self.assertRaisesRegex(MODULE.SidCellError, "exactly 2 bytes"):
                MODULE.inspect_sid_cells(source)

    def test_rejects_duplicate_cell_records(self):
        with tempfile.TemporaryDirectory() as tmp:
            source = Path(tmp) / "cpu-power-runtime.txt"
            source.write_text(
                "speed_bin_raw_le_bytes=0024\nspeed_bin_raw_le_bytes=002c\n",
                encoding="ascii",
            )
            with self.assertRaisesRegex(MODULE.SidCellError, "expected one speed-bin cell"):
                MODULE.inspect_sid_cells(source)


if __name__ == "__main__":
    unittest.main()
