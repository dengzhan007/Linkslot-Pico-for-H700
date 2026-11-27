#!/usr/bin/env python3
"""Unit tests for the hash-pinned RG34XX BL31 UART/security audit."""
import importlib.util
import struct
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).with_name("audit_rg34xx_bl31_uart.py")
SPEC = importlib.util.spec_from_file_location("audit_rg34xx_bl31_uart", MODULE_PATH)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def put(data, offset, word):
    struct.pack_into("<I", data, offset, word)


def fixture_monitor():
    data = bytearray(MODULE.EXPECTED_SIZE)
    for offset, word in MODULE.WORDS.items():
        put(data, offset, word)
    return bytes(data)


class InstructionDecoderTests(unittest.TestCase):
    def test_bl_target(self):
        self.assertEqual(MODULE.decode_bl_target(0x940027CB, 0x21A8), 0xC0D4)
        self.assertEqual(MODULE.decode_bl_target(0x940002B1, 0x21B4), 0x2C78)
        self.assertEqual(MODULE.decode_bl_target(0x97FFFFA2, 0xC11C), 0xBFA4)

    def test_cbz_target(self):
        self.assertEqual(MODULE.decode_cbz_target(0x340000A2, 0xBFB0), 0xBFC4)

    def test_unconditional_branch_target(self):
        self.assertEqual(MODULE.decode_b_target(0x14000039, 0xBFC0), 0xC0A4)

    def test_movz_movk_pair_decodes_peripheral_address(self):
        data = bytearray(MODULE.EXPECTED_SIZE)
        put(data, 0x2C78, 0xD2900080)
        put(data, 0x2C80, 0xF2A06000)
        self.assertEqual(MODULE.decode_movz_movk_pair(data, 0x2C78, 0x2C80), 0x03008004)

    def test_movz_movk_pair_rejects_register_mismatch(self):
        data = bytearray(MODULE.EXPECTED_SIZE)
        put(data, 0x2C78, 0xD2900080)
        put(data, 0x2C80, 0xF2A06001)
        with self.assertRaises(ValueError):
            MODULE.decode_movz_movk_pair(data, 0x2C78, 0x2C80)


class MonitorAuditTests(unittest.TestCase):
    def test_synthetic_instruction_fixture(self):
        report = MODULE.audit_monitor(fixture_monitor(), require_identity=False)
        self.assertEqual(report["bl33_preparation_calls"]["security_setup"], "0x2c78")
        self.assertEqual(report["bl33_preparation_calls"]["uart_gate_selector"], "0xc0d4")
        self.assertEqual(report["uart"]["gate_register"], "0x0300190c")
        self.assertEqual(report["uart"]["fallback_path"]["uart0_base"], "0x05000000")
        self.assertEqual(report["uart"]["fallback_path"]["default_clock_hz"], 24_000_000)
        self.assertEqual(report["uart"]["fallback_path"]["baud_rate"], 115200)
        self.assertEqual(report["security_setup"]["spc_decode_port_count"], 14)
        self.assertEqual(report["security_setup"]["additional_spc_register"], "0x030080e0")
        self.assertIn("unresolved", report["security_setup"]["additional_spc_semantics"])

    def test_wrong_instruction_is_rejected(self):
        data = bytearray(fixture_monitor())
        put(data, 0x21B4, 0x94000000)
        with self.assertRaisesRegex(ValueError, "instruction mismatch"):
            MODULE.audit_monitor(bytes(data), require_identity=False)

    def test_strict_monitor_hash_is_enforced(self):
        with self.assertRaisesRegex(ValueError, "SHA-256 mismatch"):
            MODULE.audit_monitor(fixture_monitor(), require_identity=True)


if __name__ == "__main__":
    unittest.main()
