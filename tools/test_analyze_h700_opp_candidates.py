#!/usr/bin/env python3
"""Regression tests for the read-only H700 OPP candidate enumerator."""
import importlib.util
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).with_name("analyze_h700_opp_candidates.py")
SPEC = importlib.util.spec_from_file_location("analyze_h700_opp_candidates", MODULE_PATH)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def opp(
    mhz,
    *,
    supported_present=False,
    supported_cells=None,
    base_present=False,
    base=None,
    variants=None,
    variant_names=None,
    status="okay",
):
    variants = variants or {}
    variant_names = variant_names if variant_names is not None else list(variants)
    return {
        "node": f"/opp-table/opp@{mhz}000000",
        "frequency_hz": mhz * 1000000,
        "supported_hw_present": supported_present,
        "supported_hw_cell_count": len(supported_cells or []),
        "supported_hw_cells": supported_cells or [],
        "voltage_microvolt_present": base_present,
        "voltage_microvolt_cells": base or [],
        "voltage_microvolt_variant_names": variant_names,
        "voltage_microvolt_variants": variants,
        "status": status,
    }


def report(entries):
    return {
        "cpu_power": {
            "cpu_opp_tables": [{
                "node": "/opp_l_table",
                "nvmem_cell_names": ["speed"],
                "entries": entries,
            }]
        }
    }


class OppCandidateTests(unittest.TestCase):
    def test_no_mask_b1_model_uses_unmasked_nodes_and_drops_empty_present_hw(self):
        fdt = report([
            opp(480, variants={"b1": [900000]}),
            opp(720, variants={"b1": [900000]}),
            opp(936, variants={"b0": [900000]}),
            opp(1008, supported_present=True, supported_cells=[1],
                base_present=True, base=[950000]),
            opp(1104, supported_present=True, supported_cells=[],
                base_present=True, base=[1000000]),
        ])
        runtime = "[scaling_available_frequencies]\n480000 720000\n"
        result = MODULE.analyze_opp_candidates(fdt, runtime)
        matches = result["exact_matches"]
        self.assertEqual(len(matches), 1)
        self.assertEqual(matches[0]["supported_hw_mode"], "not_registered")
        self.assertIsNone(matches[0]["supported_hw_mask"])
        self.assertEqual(matches[0]["voltage_property_suffix"], "b1")
        self.assertEqual(matches[0]["accepted_frequencies_hz"], [480000000, 720000000])
        self.assertNotIn(1104000000, matches[0]["accepted_frequencies_hz"])

    def test_one_cell_mask_requires_supported_hw_property_and_intersection(self):
        fdt = report([
            opp(480, supported_present=True, supported_cells=[0x33],
                base_present=True, base=[900000]),
            opp(720, supported_present=True, supported_cells=[0x30],
                base_present=True, base=[900000]),
            opp(936, supported_present=False, variants={"b1": [900000]}),
            opp(1008, supported_present=True, supported_cells=[],
                base_present=True, base=[950000]),
        ])
        runtime = "[scaling_available_frequencies]\n480000 720000\n"
        result = MODULE.analyze_opp_candidates(fdt, runtime)
        candidate = next(
            item for item in result["closest_candidates"]
            if item["supported_hw_mask"] == 0x10
            and item["voltage_property_suffix"] is None
        )
        self.assertEqual(candidate["accepted_frequencies_hz"], [480000000, 720000000])
        self.assertTrue(candidate["matches_runtime"])

    def test_runtime_frequency_fingerprint_matches_nine_b1_opps(self):
        frequencies = [480, 720, 936, 1008, 1104, 1200, 1320, 1416, 1512]
        dropped = [600, 792, 1032, 1296, 1344]
        entries = [
            opp(mhz, variants={"b1": [900000 + index * 10000]})
            for index, mhz in enumerate(frequencies)
        ]
        entries.extend(
            opp(mhz, variants={"a1": [950000]})
            for mhz in dropped
        )
        runtime = (
            "[scaling_available_frequencies]\n"
            + " ".join(str(value * 1000) for value in frequencies)
            + "\n"
        )
        result = MODULE.analyze_opp_candidates(report(entries), runtime)
        matches = [
            item for item in result["exact_matches"]
            if item["supported_hw_mode"] == "not_registered"
            and item["voltage_property_suffix"] == "b1"
        ]
        self.assertEqual(len(matches), 1)
        self.assertEqual(
            matches[0]["accepted_frequencies_hz"],
            [value * 1000000 for value in frequencies],
        )

    def test_named_property_present_but_empty_does_not_fall_back_to_base(self):
        entry = opp(
            480,
            base_present=True,
            base=[900000],
            variants={"b1": []},
            variant_names=["b1"],
        )
        self.assertFalse(MODULE._has_valid_voltage(entry, "b1"))

    def test_named_property_absent_falls_back_to_base(self):
        entry = opp(480, base_present=True, base=[900000])
        disabled = opp(600, variants={"b1": [950000]}, status="disabled")
        fdt = report([entry, disabled])
        runtime = "[scaling_available_frequencies]\n480000\n"
        result = MODULE.analyze_opp_candidates(fdt, runtime)
        matches = [
            item for item in result["exact_matches"]
            if item["supported_hw_mask"] is None
            and item["voltage_property_suffix"] == "b1"
        ]
        self.assertEqual(len(matches), 1)
        self.assertEqual(matches[0]["accepted_frequencies_hz"], [480000000])

    def test_multi_version_opp_property_accepts_any_intersecting_version(self):
        fdt = report([
            opp(480, supported_present=True,
                supported_cells=[0x01, 0x80000000],
                base_present=True, base=[900000]),
            opp(600, supported_present=True, supported_cells=[0x02],
                base_present=True, base=[950000]),
        ])
        runtime = "[scaling_available_frequencies]\n480000\n"
        result = MODULE.analyze_opp_candidates(fdt, runtime)
        exact_masks = {
            item["supported_hw_mask"] for item in result["exact_matches"]
            if item["supported_hw_mode"] == "one_cell_mask"
            and item["voltage_property_suffix"] is None
        }
        self.assertIn(0x80000000, exact_masks)
        self.assertIn(0x01, exact_masks)
        self.assertTrue(any(
            item["mask_is_derived_exact_witness"]
            for item in result["exact_matches"]
            if item["supported_hw_mode"] == "one_cell_mask"
        ))

    def test_derived_witness_finds_combined_bits_above_low_byte(self):
        fdt = report([
            opp(480, supported_present=True, supported_cells=[0x80000001],
                base_present=True, base=[900000]),
            opp(720, supported_present=True, supported_cells=[0x00000100],
                base_present=True, base=[950000]),
            opp(936, supported_present=True, supported_cells=[0x00000200],
                base_present=True, base=[1000000]),
        ])
        runtime = "[scaling_available_frequencies]\n480000 720000\n"
        result = MODULE.analyze_opp_candidates(fdt, runtime)
        matches = [
            item for item in result["exact_matches"]
            if item["supported_hw_mode"] == "one_cell_mask"
            and item["voltage_property_suffix"] is None
        ]
        self.assertTrue(matches)
        self.assertTrue(any(
            item["supported_hw_mask"] & 0x80000000
            for item in matches
        ))

    def test_voltage_triplet_order_is_validated(self):
        self.assertTrue(MODULE._valid_voltage_cells([800000, 900000, 1000000]))
        self.assertFalse(MODULE._valid_voltage_cells([900000, 800000, 1000000]))
        self.assertFalse(MODULE._valid_voltage_cells([900000, 1000000, 800000]))

    def test_requires_parser_v3_property_presence_fields(self):
        entry = opp(480, variants={"b1": [900000]})
        del entry["supported_hw_present"]
        with self.assertRaisesRegex(MODULE.AnalysisError, "parser-v3 fields missing"):
            MODULE.analyze_opp_candidates(
                report([entry]),
                "[scaling_available_frequencies]\n480000\n",
            )

    def test_runtime_policy_blocks_must_agree(self):
        fdt = report([opp(480, base_present=True, base=[900000])])
        runtime = (
            "[scaling_available_frequencies]\n480000\n"
            "[scaling_available_frequencies]\n480000 720000\n"
        )
        with self.assertRaisesRegex(MODULE.AnalysisError, "blocks disagree"):
            MODULE.analyze_opp_candidates(fdt, runtime)


if __name__ == "__main__":
    unittest.main()
