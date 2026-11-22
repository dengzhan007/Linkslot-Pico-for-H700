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
        self.assertIn('$FdtInspector = Join-Path $RepoRoot "tools\\inspect_h700_fdt.py"', source)
        self.assertIn("& $pythonExe @pythonArgs $FdtInspector $liveFdt", source)
        self.assertIn("& $pythonExe @pythonArgs $FdtInspector $packageDtb", source)
        self.assertIn('$EnvInspector = Join-Path $RepoRoot "tools\\inspect_h700_env.py"', source)
        self.assertIn("& $pythonExe @pythonArgs $EnvInspector $envImage", source)
        self.assertIn('$MemoryMapInspector = Join-Path $RepoRoot "tools\\inspect_h700_memory_map.py"', source)
        self.assertIn("$memoryMapCommandArgs += $memoryMapArgs", source)
        self.assertIn("& $pythonExe @memoryMapCommandArgs", source)
        self.assertIn('memory-map-report.json', source)
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
        self.assertIn('ENV_MAX_SECTORS=131072', self.source)
        self.assertIn('ENV_BYTES=$((ENV_PART_SIZE * 512))', self.source)
        self.assertIn('of="$OUTDIR/env-partition.bin"', self.source)
        self.assertIn('count="$ENV_PART_SIZE"', self.source)
        self.assertNotIn("env-partition-first64k.bin", self.source)
        self.assertNotIn("/dev/mmcblk0p3", self.source)

    def test_boot_partition_is_discovered_by_exact_gpt_label_and_bounded(self):
        self.assertIn("PARTNAME=boot", self.source)
        self.assertIn('BOOT_PART="/dev/$BOOT_DEVNAME"', self.source)
        self.assertIn('BOOT_MAX_SECTORS=131072', self.source)
        self.assertIn('BOOT_BYTES=$((BOOT_PART_SIZE * 512))', self.source)
        self.assertIn('of="$OUTDIR/boot-partition.bin"', self.source)
        self.assertIn('count="$BOOT_PART_SIZE"', self.source)
        self.assertIn('boot_partition_capture=skipped_no_PARTNAME_boot', self.source)
        self.assertIn('sha256sum "$OUTDIR/boot-partition.bin"', self.source)
        self.assertNotIn('/dev/mmcblk0p4', self.source)
    def test_cma_and_memblock_debugfs_capture_is_read_only(self):
        self.assertIn('/sys/kernel/debug/cma', self.source)
        self.assertIn('for name in memory reserved', self.source)
        self.assertIn('file="/sys/kernel/debug/memblock/$name"', self.source)
        self.assertIn('cat "$node/$attr"', self.source)
        self.assertIn('for range in "$node"/ranges/*', self.source)
        self.assertIn('cat "$range/$attr"', self.source)
        self.assertIn('cat "$file"', self.source)
        self.assertIn('getconf PAGESIZE', self.source)
        self.assertIn('> "$OUTDIR/page-size.txt"', self.source)
        self.assertIn('/proc/self/smaps', self.source)
        self.assertIn('KernelPageSize:', self.source)
        self.assertIn('> "$OUTDIR/page-size-source.txt"', self.source)
        self.assertIn('> "$OUTDIR/debugfs-memory-reservations.txt"', self.source)
        self.assertNotRegex(self.source, r">\s*/sys/kernel/debug/")
        self.assertNotIn('/alloc"', self.source)
        self.assertNotIn('/free"', self.source)

    def test_tee_ion_runtime_capture_is_read_only(self):
        self.assertIn('> "$OUTDIR/tee-ion-runtime.txt"', self.source)
        self.assertIn('/proc/config.gz', self.source)
        self.assertIn('CONFIG_(TEE|OPTEE|ION|ION_SUNXI|ION_OF|CMA|DMA_CMA|OF_RESERVED_MEM)', self.source)
        self.assertNotIn('CONFIG_SUNXI_ION', self.source)
        self.assertIn('/proc/kallsyms', self.source)
        self.assertIn('[address redacted]', self.source)
        for symbol in ('sunxi_ion_probe', 'optee_probe_drm_configure', 'ion_secure_heap_create'):
            self.assertIn(symbol, self.source)
        self.assertIn('/dev/ion', self.source)
        self.assertIn('/sys/kernel/debug/ion/heaps/$heap', self.source)
        self.assertIn('drm_base|drm_size|tee_base', self.source)
        self.assertIn('/proc/modules', self.source)
        self.assertIn('/sys/bus/platform/drivers/ion-sunxi', self.source)
        self.assertIn('cat "$path"', self.source)
        self.assertNotRegex(self.source, r">\s*/sys/(?:kernel/debug|bus/platform|class/misc)/")
        self.assertNotIn('ION_IOC_', self.source)
        self.assertNotIn('GET_DRM_INFO', self.source)

    def test_root_argument_is_recorded_even_when_disk_is_explicit(self):
        root_parse = self.source.index('for arg in $(cat /proc/cmdline')
        disk_guard = self.source.index('if [ -z "$DISK" ]; then')
        self.assertLess(root_parse, disk_guard)
        self.assertIn('ROOTARG="${arg#root=}"', self.source)
        self.assertIn('echo "root_argument_from_cmdline=$ROOTARG"', self.source)

    def test_device_is_never_used_as_dd_output(self):
        self.assertNotIn('of="$DISK"', self.source)
        self.assertNotIn("of=$DISK", self.source)


if __name__ == "__main__":
    unittest.main()
