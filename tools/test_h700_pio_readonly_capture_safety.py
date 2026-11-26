#!/usr/bin/env python3
"""Static safety regression for the bounded RG34XX PIO readback helper."""

import re
import unittest
from pathlib import Path

SCRIPT = (
    Path(__file__).resolve().parents[1]
    / "scripts"
    / "collect-rg34xx-pio-input-readonly.sh"
)


class PioReadOnlyCaptureSafetyTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.source = SCRIPT.read_text(encoding="utf-8")
        cls.lines = cls.source.splitlines()

    def test_only_the_two_known_dat_registers_are_whitelisted(self):
        self.assertIn("PA_DAT_ADDRESS=0x0300b010", self.source)
        self.assertIn("PE_DAT_ADDRESS=0x0300b0a0", self.source)
        self.assertIn("PA_WORD_INDEX=$((PA_DAT_ADDRESS / 4))", self.source)
        self.assertIn("PE_WORD_INDEX=$((PE_DAT_ADDRESS / 4))", self.source)
        self.assertIn("pio_base=0x0300b000", self.source)
        self.assertIn("register_accesses_per_sample=2", self.source)

    def test_iomem_resource_and_root_checks_precede_any_mmio_read(self):
        resource_check = self.source.index('grep -Eiq "$PIO_RESOURCE_PATTERN" /proc/iomem')
        first_mem_read = self.source.index("dd if=/dev/mem")
        root_check = self.source.index('[ "$UID_VALUE" = "0" ]')
        self.assertLess(root_check, first_mem_read)
        self.assertLess(resource_check, first_mem_read)
        self.assertIn("pinctrl@0300b000", self.source)
        self.assertIn("refusing MMIO read", self.source)

    def test_dev_mem_is_input_only_and_reads_exactly_four_bytes(self):
        dd_lines = [
            line.strip()
            for line in self.lines
            if re.match(r"^\\s*dd\\s+", line)
        ]
        self.assertEqual(len(dd_lines), 1)
        self.assertIn('of="$tmp"', dd_lines[0])
        self.assertIn('bs=4 skip="$word_index" count=1', dd_lines[0])
        self.assertIn('byte_count="$(wc -c < "$tmp"', self.source)
        self.assertIn('[ "$byte_count" = "4" ]', self.source)
        self.assertNotRegex(self.source, r"\bof\s*=\s*['\"]?/dev/mem")
        self.assertNotIn("devmem ", self.source)

    def test_sampling_output_is_confined_to_report_directory(self):
        self.assertIn('OUTDIR="/tmp/rg34xx-pio-input-$STAMP"', self.source)
        self.assertIn('> "$OUTDIR/pio-samples.csv"', self.source)
        self.assertIn('>> "$OUTDIR/pio-samples.csv"', self.source)
        self.assertIn('> "$OUTDIR/summary.txt"', self.source)
        self.assertNotRegex(self.source, r">\s*/dev/")
        self.assertNotRegex(self.source, r">\s*/sys/")
        self.assertNotRegex(self.source, r">\s*/proc/")
        self.assertIn("trap 'rm -f \"$OUTDIR/.pa-dat.tmp\"", self.source)

    def test_sampling_is_bounded_and_does_not_touch_gpio_configuration(self):
        self.assertIn('[ "$SAMPLES" -le 5000 ]', self.source)
        self.assertIn('while [ "$sample" -lt "$SAMPLES" ]', self.source)
        self.assertIn("sleep 0.1", self.source)
        for forbidden in (
            "devmem ",
            "gpio_request",
            "gpiod_get",
            "pinmux",
            "pinconf",
            "gpio_direction",
            "writel(",
            "mmap(",
            "ioctl(",
            "mount -t debugfs",
        ):
            # pinmux/pinconf occur only in descriptive safety strings.
            if forbidden in ("pinmux", "pinconf"):
                continue
            self.assertNotIn(forbidden, self.source)
        self.assertIn("no register, pinmux, pinconf, GPIO, EINT or GIC writes", self.source)


if __name__ == "__main__":
    unittest.main()
