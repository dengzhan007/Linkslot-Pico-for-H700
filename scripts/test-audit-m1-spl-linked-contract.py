#!/usr/bin/env python3
"""Regression tests for the M1 linked-artifact fail-closed contract."""
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

MODULE = Path(__file__).with_name("audit-m1-spl-linked-contract.py")
SPEC = importlib.util.spec_from_file_location("m1_linked_contract", MODULE)
MOD = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MOD)


class LinkedContractTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.out = Path(self.tmp.name)
        for name in ("m1-spl-preflight.elf", "m1-spl-preflight.bin",
                     "m1-spl-preflight.map", "m1-spl-preflight.nm.txt",
                     "m1-spl-preflight.disasm.txt"):
            (self.out / name).write_bytes(b"x")
        self.manifest = {
            "geometry_only": False, "installable": False, "candidate_eligible": False,
            "device_io_performed": False, "media_write_performed": False,
            "firmware_flash_performed": False, "payload_executed": False,
            "firmware_payload_read_at_runtime": False, "hardware_qualified": False,
            "test_fingerprint": "0x00000000 (deliberate fail-closed sentinel)",
            "bss": {"start": "0x30000", "end_exclusive": "0x32000",
                    "used_bytes": 0x2000, "limit_bytes": 0x4000},
            "m1_package_scratch": {"address": "0x30000", "bytes": 4096,
                                   "alignment": 64, "inside_bss": True},
            "m1_gpt_partitions": {"address": "0x31000", "bytes": 0x100,
                                  "entry_slots": 8, "inside_bss": True},
            "spl_stack_pointer": "0x52a00", "bss_end_to_stack_gap_bytes": 0x20a00,
            "spl_allocator_profile": {"heap_start": "0x34000", "heap_bytes": 0x10000,
                                      "heap_high_water_measured": False},
        }
        self.callgraph = {"execution_safe": False, "execution_authorized": False,
                          "reachable_functions": ["board_init_f"],
                          "reachable_function_count": 1,
                          "reachable_store_instructions": [],
                          "reachable_store_instruction_count": 0}

    def tearDown(self):
        self.tmp.cleanup()

    def test_accepts_non_installable_link(self):
        result = MOD.audit(self.manifest, self.callgraph, self.out)
        self.assertEqual(result["result"], "PASS_LINKED_ARTIFACTS_AND_FAIL_CLOSED_CONTRACT")
        self.assertFalse(result["candidate_eligible"])

    def test_rejects_candidate_eligible(self):
        self.manifest["candidate_eligible"] = True
        with self.assertRaisesRegex(ValueError, "non-installable"):
            MOD.audit(self.manifest, self.callgraph, self.out)

    def test_rejects_missing_link_artifact(self):
        (self.out / "m1-spl-preflight.map").unlink()
        with self.assertRaisesRegex(ValueError, "missing linked audit artifacts"):
            MOD.audit(self.manifest, self.callgraph, self.out)

    def test_rejects_stack_overlap(self):
        self.manifest["spl_stack_pointer"] = "0x30000"
        with self.assertRaisesRegex(ValueError, "overlaps stack"):
            MOD.audit(self.manifest, self.callgraph, self.out)

    def test_rejects_authorized_callgraph(self):
        self.callgraph["execution_authorized"] = True
        with self.assertRaisesRegex(ValueError, "non-authorizing"):
            MOD.audit(self.manifest, self.callgraph, self.out)


if __name__ == "__main__":
    unittest.main()
