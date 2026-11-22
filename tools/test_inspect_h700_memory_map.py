#!/usr/bin/env python3
"""Synthetic tests for the H700 CMA/memblock memory range normalizer."""

import importlib.util
import tempfile
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).with_name("inspect_h700_memory_map.py")
SPEC = importlib.util.spec_from_file_location("inspect_h700_memory_map", MODULE_PATH)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)

SNAPSHOT = """== CMA debugfs ==
-- /sys/kernel/debug/cma/cma-0 --
[base_pfn]
507904
[count]
16384
[order_per_bit]
0
[used]
1024
-- /sys/kernel/debug/cma/cma-0/ranges/0 --
[base_pfn]
507904
[count]
16384

== memblock debugfs ==
[memory]
   0: 0x0000000040000000..0x000000007fffffff x NONE
[reserved]
   0: 0x0000000040000000..0x000000004007ffff x RSV_KERN
   1: 0x000000007bf24800..0x000000007c075fff x NOMAP
"""


class MemoryMapInspectorTests(unittest.TestCase):
    def test_converts_cma_pfns_and_memblock_inclusive_ranges(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            debugfs = root / "debugfs.txt"
            pagesize = root / "page-size.txt"
            fdt = root / "live-fdt.json"
            meminfo = root / "meminfo.txt"
            debugfs.write_text(SNAPSHOT, encoding="utf-8")
            pagesize.write_text("4096\n", encoding="ascii")
            fdt.write_text(
                '{"sha256":"abc","memory_regions":[{"start":"0x40000000","end_exclusive":"0x80000000"}],'
                '"boot_framebuffers":[{"base":"0x7bf24800","end_exclusive":"0x7c076000"}]}',
                encoding="utf-8",
            )
            meminfo.write_text(
                "MemTotal: 996700 kB\nCmaTotal: 65536 kB\nCmaFree: 60360 kB\n",
                encoding="ascii",
            )
            report = MODULE.inspect_snapshot(debugfs, pagesize, fdt, meminfo)

        self.assertEqual(report["page_size_bytes"], 4096)
        self.assertEqual(len(report["cma_areas"]), 1)
        region = report["cma_areas"][0]["ranges"][0]
        self.assertTrue(region["valid"])
        self.assertEqual(region["start"], hex(507904 * 4096))
        self.assertEqual(region["size_bytes"], 16384 * 4096)
        self.assertEqual(region["end_exclusive"], hex((507904 + 16384) * 4096))
        self.assertEqual(report["memblock"]["memory"][0]["start"], "0x40000000")
        self.assertEqual(report["memblock"]["memory"][0]["end_exclusive"], "0x80000000")
        self.assertEqual(report["memblock"]["reserved"][0]["attributes"], "RSV_KERN")
        self.assertEqual(report["memblock"]["reserved"][1]["end_exclusive"], "0x7c076000")
        self.assertEqual(report["ram_regions"][0]["start"], "0x40000000")
        self.assertEqual(report["framebuffers"][0]["base"], "0x7bf24800")
        self.assertEqual(report["meminfo_bytes"], {
            "MemTotal": 996700 * 1024,
            "CmaTotal": 65536 * 1024,
            "CmaFree": 60360 * 1024,
        })

    def test_reports_missing_debugfs_interfaces_without_inventing_ranges(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            debugfs = root / "debugfs.txt"
            pagesize = root / "page-size.txt"
            debugfs.write_text(
                "== CMA debugfs ==\nNOT_PRESENT: /sys/kernel/debug/cma\n"
                "== memblock debugfs ==\nNOT_PRESENT: /sys/kernel/debug/memblock\n",
                encoding="utf-8",
            )
            pagesize.write_text("4096\n", encoding="ascii")
            report = MODULE.inspect_snapshot(debugfs, pagesize)

        self.assertEqual(report["cma_areas"], [])
        self.assertEqual(report["memblock"], {"memory": [], "reserved": []})
        self.assertEqual(report["debugfs_presence"]["/sys/kernel/debug/cma"], "not_present")

    def test_parses_memblock_when_page_size_is_unavailable(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            debugfs = root / "debugfs.txt"
            pagesize = root / "page-size.txt"
            debugfs.write_text(SNAPSHOT, encoding="utf-8")
            pagesize.write_text("NOT_PRESENT\\n", encoding="ascii")
            report = MODULE.inspect_snapshot(debugfs, pagesize)

        self.assertIsNone(report["page_size_bytes"])
        self.assertFalse(report["cma_pfns_normalized"])
        self.assertEqual(report["cma_areas"][0]["base_pfn"], 507904)
        self.assertEqual(report["memblock"]["memory"][0]["start"], "0x40000000")
        self.assertEqual(report["memblock"]["reserved"][1]["end_exclusive"], "0x7c076000")
        self.assertTrue(any("Memblock ranges are already physical byte addresses" in x
                            for x in report["limitations"]))

    def test_rejects_reversed_memblock_range(self):
        bad = SNAPSHOT.replace(
            "0x0000000040000000..0x000000007fffffff",
            "0x000000007fffffff..0x0000000040000000",
        )
        with self.assertRaisesRegex(MODULE.MemoryMapError, "precedes start"):
            MODULE._parse_debugfs(bad)


if __name__ == "__main__":
    unittest.main()
