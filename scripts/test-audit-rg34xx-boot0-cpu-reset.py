#!/usr/bin/env python3
"""Unit tests for the Boot0 CPU-reset bit interpretation (H616 reference only)."""
from __future__ import annotations

import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts/audit-rg34xx-boot0-cpu-reset.py"
SPEC = importlib.util.spec_from_file_location("boot0_cpu_reset_audit", SCRIPT)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("cannot load Boot0 CPU-reset audit module")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class CpuResetBitTests(unittest.TestCase):
    def test_h616_reference_default_keeps_cpu0_released_and_cpu1_to_3_reset(self) -> None:
        reset_default = MODULE.RESET_DEFAULT
        self.assertEqual(reset_default, 0x13FF0101)
        self.assertFalse(MODULE.cpu_reset_bit_asserted(reset_default, 0))
        for core in (1, 2, 3):
            with self.subTest(core=core):
                self.assertTrue(MODULE.cpu_reset_bit_asserted(reset_default, core))

    def test_clearing_reset_bit_means_asserted_under_h616_reference(self) -> None:
        reset_default = MODULE.RESET_DEFAULT
        for core in (1, 2, 3):
            with self.subTest(core=core):
                self.assertTrue(MODULE.cpu_reset_bit_asserted(reset_default & ~(1 << core), core))

    def test_invalid_core_is_rejected(self) -> None:
        for core in (-1, 4, True, 1.5):
            with self.subTest(core=core):
                with self.assertRaises(ValueError):
                    MODULE.cpu_reset_bit_asserted(MODULE.RESET_DEFAULT, core)

    def test_boolean_reset_value_is_rejected(self) -> None:
        with self.assertRaises(TypeError):
            MODULE.cpu_reset_bit_asserted(True, 0)


if __name__ == "__main__":
    unittest.main()
