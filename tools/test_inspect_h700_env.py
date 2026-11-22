#!/usr/bin/env python3
"""Synthetic tests for the H700 U-Boot environment inspector."""

import importlib.util
import struct
import unittest
import zlib
from pathlib import Path

MODULE_PATH = Path(__file__).with_name("inspect_h700_env.py")
SPEC = importlib.util.spec_from_file_location("inspect_h700_env", MODULE_PATH)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def make_env(size=4096, redundant=True):
    variables = (
        b"bootcmd=run setargs_nand boot_normal\0"
        b"mmc_root=/dev/mmcblk0p5\0"
        b"setargs_mmc=setenv bootargs root=/dev/mmcblk0p5 rootwait\0"
        b"bootdelay=0\0\0"
    )
    header_bytes = 5 if redundant else 4
    if len(variables) > size - header_bytes:
        raise ValueError("test environment is too small")
    data = variables + bytes(size - header_bytes - len(variables))
    crc = zlib.crc32(data) & 0xFFFFFFFF
    header = struct.pack("<I", crc) + (b"\0" if redundant else b"")
    return header + data


class H700EnvInspectorTests(unittest.TestCase):
    def test_valid_redundant_environment_and_variables(self):
        report = MODULE.inspect_env(make_env(), "synthetic-env")
        self.assertTrue(report["crc_valid"])
        self.assertEqual(report["flags_byte_candidate"], 0)
        self.assertEqual(report["variables_data_offset"], 5)
        self.assertEqual(report["variable_count"], 3)
        variables = {item["name"]: item["value"] for item in report["variables"]}
        self.assertEqual(variables["bootcmd"], "run setargs_nand boot_normal")
        self.assertEqual(variables["mmc_root"], "/dev/mmcblk0p5")
        self.assertEqual(report["crc_matches"][0]["env_image_bytes"], 4096)

    def test_valid_non_redundant_environment(self):
        report = MODULE.inspect_env(make_env(redundant=False))
        self.assertTrue(report["crc_valid"])
        self.assertEqual(report["variables_data_offset"], 4)
        self.assertEqual(report["variable_count"], 3)

    def test_bad_crc_does_not_prevent_read_only_variable_extraction(self):
        image = bytearray(make_env())
        image[64] ^= 0x01
        report = MODULE.inspect_env(bytes(image))
        self.assertFalse(report["crc_valid"])
        self.assertIn("Do not treat", report["crc_note"])
        self.assertGreaterEqual(report["variable_count"], 3)

    def test_rejects_short_input(self):
        with self.assertRaisesRegex(MODULE.EnvError, "too short"):
            MODULE.inspect_env(b"\0" * 4)


if __name__ == "__main__":
    unittest.main()
