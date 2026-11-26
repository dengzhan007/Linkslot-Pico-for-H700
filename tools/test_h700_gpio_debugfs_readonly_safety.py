#!/usr/bin/env python3
"""Static safety checks for the RG34XX gpiolib debugfs sampler."""

import re
import unittest
from pathlib import Path

SCRIPT = (
    Path(__file__).resolve().parents[1]
    / "scripts"
    / "collect-rg34xx-gpio-debugfs-readonly.sh"
)


class GpioDebugfsReadOnlySafetyTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.source = SCRIPT.read_text(encoding="utf-8")
        cls.lines = cls.source.splitlines()

    def test_reads_only_the_existing_debugfs_gpio_view(self):
        self.assertIn('OUTDIR="${1:-}"', self.source)
        self.assertIn('SAMPLES="${2:-600}"', self.source)
        self.assertNotIn(r"\\${", self.source)
        self.assertIn('GPIO_DEBUG="/sys/kernel/debug/gpio"', self.source)
        self.assertIn('cat "$GPIO_DEBUG" > "$OUTDIR/preflight-gpio.txt"', self.source)
        self.assertIn('cat "$GPIO_DEBUG" >> "$SAMPLE_FILE"', self.source)
        self.assertNotIn("/dev/mem", self.source)
        self.assertNotIn("/dev/kmem", self.source)
        self.assertNotIn("sunxi_dump", self.source)

    def test_does_not_mount_debugfs_or_claim_or_export_gpio(self):
        for forbidden in (
            "mount -t debugfs",
            "mount debugfs",
            "gpio_request",
            "gpiod_get",
            "gpio_export",
            "/sys/class/gpio/export",
            "/sys/class/gpio/unexport",
            "pinmux-select",
            "sunxi_pin_configure",
            "devmem ",
            "writel(",
            "ioctl(",
        ):
            self.assertNotIn(forbidden, self.source)
        self.assertIn('no_debugfs_mount=1', self.source)
        self.assertIn('no_gpio_export_or_request=1', self.source)
        self.assertIn('no_pinmux_pinconf_gpio_eint_gic_writes=1', self.source)

    def test_requires_root_existing_readable_file_and_is_bounded(self):
        self.assertIn('[ "$UID_VALUE" = "0" ]', self.source)
        self.assertIn('[ -r "$GPIO_DEBUG" ]', self.source)
        self.assertIn('[ "$SAMPLES" -le 3600 ]', self.source)
        self.assertIn('while [ "$sample" -lt "$SAMPLES" ]', self.source)
        self.assertIn("sleep 0.1", self.source)
        self.assertIn("read -r uptime_value _ < /proc/uptime", self.source)

    def test_all_outputs_are_confined_to_report_directory(self):
        self.assertIn('OUTDIR="/tmp/rg34xx-gpio-debugfs-$STAMP"', self.source)
        self.assertIn('> "$OUTDIR/summary.txt"', self.source)
        self.assertIn('> "$OUTDIR/preflight-gpio.txt"', self.source)
        self.assertIn('SAMPLE_FILE="$OUTDIR/gpio-debugfs-samples.txt"', self.source)
        self.assertIn('>> "$SAMPLE_FILE"', self.source)
        self.assertNotRegex(self.source, r">\s*/dev/")
        self.assertNotRegex(self.source, r">\s*/sys/")
        self.assertNotRegex(self.source, r">\s*/proc/")

    def test_script_has_no_non_read_only_shell_redirections(self):
        for line in self.lines:
            stripped = line.strip()
            if not stripped or stripped.startswith("#"):
                continue
            if re.search(r">\s*/(?:dev|sys|proc)/", stripped):
                self.fail(f"write redirection to a device/proc/sys path: {line}")


if __name__ == "__main__":
    unittest.main()
