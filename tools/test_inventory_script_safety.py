#!/usr/bin/env python3
"""Static safety regression for the on-device read-only inventory script."""

import re
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "collect-rg34xx-readonly-inventory.sh"


class InventoryScriptSafetyTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.source = SCRIPT.read_text(encoding="utf-8")
        cls.lines = cls.source.splitlines()

    def test_every_dd_writes_only_to_the_report_directory(self):
        dd_lines = [
            line.strip()
            for line in self.lines
            if re.match(r"^\s*dd\s+", line)
        ]
        self.assertGreaterEqual(len(dd_lines), 2)
        for line in dd_lines:
            self.assertIn("if=", line, line)
            self.assertIn('of="$OUTDIR/', line, line)
            self.assertNotRegex(line, r"\bof\s*=\s*['\"]?\$?DISK\b", line)

    def test_expected_boot_regions_are_read_with_fixed_lengths(self):
        required = [
            'read_exact "boot0-primary" 16 256 "boot0-primary.bin"',
            'read_exact "boot0-secondary" 512 256 "boot0-secondary.bin"',
            'read_exact "toc1-package" 32800 2560 "toc1-package.bin"',
        ]
        for statement in required:
            self.assertIn(statement, self.source)

    def test_no_flash_or_power_mutation_commands(self):
        forbidden = re.compile(
            r"^\s*(?:sudo\s+)?(?:fw_setenv|flashcp|flash_erase|nandwrite|"
            r"sunxi-fel|reboot|poweroff|mkfs(?:\.\w+)?|parted)\b",
            re.IGNORECASE,
        )
        offenders = [line for line in self.lines if forbidden.search(line)]
        self.assertEqual(offenders, [])


    def test_windows_wrapper_only_runs_the_read_only_collector(self):
        runner = SCRIPT.parent / "run-rg34xx-inventory.ps1"
        source = runner.read_text(encoding="utf-8")
        self.assertIn("& adb @adbPrefix push $Collector $remoteScript", source)
        self.assertIn("& adb @adbPrefix shell sh $remoteScript $BlockDevice $remoteOutput", source)
        self.assertIn("& adb @adbPrefix pull $remoteOutput $OutputDirectory", source)
        forbidden = re.compile(
            r"^\\s*&\\s*adb\\b.*\\bshell\\s+(?:reboot|poweroff|fw_setenv|"
            r"flashcp|flash_erase|nandwrite|sunxi-fel)\\b",
            re.IGNORECASE,
        )
        offenders = [line for line in source.splitlines() if forbidden.search(line)]
        self.assertEqual(offenders, [])

    def test_env_partition_is_discovered_by_exact_gpt_label(self):
        self.assertIn('for node in /sys/class/block/"$BASE"p*', self.source)
        self.assertIn("grep -qx 'PARTNAME=env' \"$node/uevent\"", self.source)
        self.assertIn('ENV_PART="/dev/$ENV_DEVNAME"', self.source)
        self.assertIn('of="$OUTDIR/env-partition-first64k.bin"', self.source)
        self.assertNotIn("/dev/mmcblk0p3", self.source)

    def test_device_is_never_used_as_dd_output(self):
        self.assertNotIn('of="$DISK"', self.source)
        self.assertNotIn("of=$DISK", self.source)


if __name__ == "__main__":
    unittest.main()
