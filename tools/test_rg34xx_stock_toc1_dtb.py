#!/usr/bin/env python3
"""Pin RG34XX stock TOC1 package, DRAM strap GPIOs, PMIC, U-Boot and BL31 identity."""

from __future__ import annotations

import hashlib
import struct
import subprocess
import unittest


REPORT_PATH = "reports/rg34xx-inventory-20261127-uart"
TOC1_SHA256 = "e6c20e3a54c51b9336c798e3a4fa1ca6c45b28c6f70456e2db9c252aa4ba4604"
EXPECTED_PAYLOAD_SHA256 = {
    "u-boot": "4d60a22ff13eac00dba89e0ee7df52a49852c0d9d856bec545ce40126191d127",
    "monitor": "676bc0206e3c9a7f229d93ea7f3286095f669940ad91f3c4f3d37bdb18696c31",
    "dtbo": "4bee281701610f96dcbdd1d38cb8df28386224d8c68b668b9a7a7a9df8e73ec2",
    "dtb": "c28c87380ffab00148bf4dc457e70b8c6c576470dcfbc53df64addddf89603a1",
}
EXPECTED_FDT_SHA256 = "7017a8f970234f7518e7f417fa9f86dd4ad3e46529bc5282c05a31e9f007dbc5"


def read_git_blob(ref: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{ref}:{path}"],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    return result.stdout


def parse_sunxi_package(data: bytes) -> dict[str, bytes]:
    if not data.startswith(b"sunxi-package") or len(data) < 0x30:
        raise AssertionError("TOC1 input is not a sunxi-package")
    magic, checksum, serial, status, count, total_size, boot_media = struct.unpack_from(
        "<7I", data, 0x10
    )
    if magic != 0x89119800 or total_size > len(data) or count != 4:
        raise AssertionError("unexpected sunxi-package header")
    items: dict[str, bytes] = {}
    for index in range(count):
        start = 0x30 + index * 0x170
        name = data[start + 0x10 : start + 0x50].split(b"\0", 1)[0].decode("ascii")
        offset, length, _reserved, _kind = struct.unpack_from("<4I", data, start + 0x50)
        if not name or offset < 0x30 or offset + length > total_size:
            raise AssertionError(f"invalid sunxi-package item {index}")
        if name in items:
            raise AssertionError(f"duplicate sunxi-package item {name}")
        items[name] = data[offset : offset + length]
    return items


def parse_fdt(data: bytes) -> dict[str, dict[str, bytes]]:
    if len(data) < 40 or struct.unpack_from(">I", data, 0)[0] != 0xD00DFEED:
        raise AssertionError("invalid FDT magic")
    (
        _magic, total_size, struct_offset, strings_offset, _reserve_offset,
        version, _last_compatible, _boot_cpu, strings_size, struct_size,
    ) = struct.unpack_from(">10I", data, 0)
    if total_size > len(data) or version < 17:
        raise AssertionError("unsupported or truncated FDT")
    strings = data[strings_offset : strings_offset + strings_size]
    structure = data[struct_offset : struct_offset + struct_size]

    def string_at(offset: int) -> str:
        if offset >= len(strings):
            raise AssertionError(f"FDT string offset out of range: {offset}")
        end = strings.find(b"\0", offset)
        if end < 0:
            raise AssertionError("unterminated FDT property name")
        return strings[offset:end].decode("utf-8", "replace")

    stack: list[str] = []
    nodes: dict[str, dict[str, bytes]] = {}
    pos = 0
    while pos + 4 <= len(structure):
        token = struct.unpack_from(">I", structure, pos)[0]
        pos += 4
        if token == 1:  # FDT_BEGIN_NODE
            end = structure.find(b"\0", pos)
            if end < 0:
                raise AssertionError("unterminated FDT node name")
            name = structure[pos:end].decode("utf-8", "replace")
            pos = (end + 4) & ~3
            stack.append(name)
            path = "/" + "/".join(part for part in stack if part)
            nodes.setdefault(path, {})
        elif token == 2:  # FDT_END_NODE
            if not stack:
                raise AssertionError("unbalanced FDT_END_NODE")
            stack.pop()
        elif token == 3:  # FDT_PROP
            length, name_offset = struct.unpack_from(">II", structure, pos)
            pos += 8
            value = structure[pos : pos + length]
            if len(value) != length:
                raise AssertionError("truncated FDT property")
            pos = (pos + length + 3) & ~3
            path = "/" + "/".join(part for part in stack if part)
            nodes[path][string_at(name_offset)] = value
        elif token == 4:  # FDT_NOP
            continue
        elif token == 9:  # FDT_END
            return nodes
        else:
            raise AssertionError(f"unknown FDT token {token} at 0x{pos - 4:x}")
    raise AssertionError("FDT_END token missing")


def be32(value: bytes) -> int:
    if len(value) != 4:
        raise AssertionError(f"expected one FDT cell, got {len(value)} bytes")
    return struct.unpack(">I", value)[0]


class TestRG34XXStockTOC1(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.package = read_git_blob(
            "refs/remotes/origin/reports",
            f"{REPORT_PATH}/toc1-package.bin",
        )
        cls.package_sha256 = hashlib.sha256(cls.package).hexdigest()
        cls.items = parse_sunxi_package(cls.package)
        cls.dtb = cls.items["dtb"]
        cls.fdt = parse_fdt(cls.dtb)
        cls.strings = cls.dtb

    def test_package_components_and_hashes(self) -> None:
        self.assertEqual(self.package_sha256, TOC1_SHA256)
        self.assertEqual(set(self.items), {"u-boot", "monitor", "dtbo", "dtb"})
        for name, expected in EXPECTED_PAYLOAD_SHA256.items():
            self.assertEqual(hashlib.sha256(self.items[name]).hexdigest(), expected)
        self.assertEqual(self.items["u-boot"][:4], bytes.fromhex("8e0100ea"))
        self.assertEqual(self.items["monitor"][:4], bytes.fromhex("3e000014"))
        self.assertIn(b"U-Boot 2018.05 (May 26 2026", self.items["u-boot"])
        self.assertIn(b"U-Boot.armv7", self.items["u-boot"])
        self.assertIn(b"pmu_axp2202_probe", self.items["u-boot"])
        self.assertIn(b"bmu_axp2202_probe", self.items["u-boot"])
        self.assertIn(b"plat/sun50iw9p1/bl31_sunxi_setup.c", self.items["monitor"])
        self.assertIn(b"plat/sun50iw9p1/plat_pm.c", self.items["monitor"])
        self.assertIn(b"psci_afflvl0_suspend", self.items["monitor"])
        self.assertIn(b"psci_afflvl0_on", self.items["monitor"])
        self.assertIn(b"psci_afflvl0_off", self.items["monitor"])

    def test_embedded_rg34xx_fdt_and_axp2202_r_twi0(self) -> None:
        total_size = be32(self.dtb[4:8])
        self.assertEqual(total_size, 139822)
        self.assertEqual(
            hashlib.sha256(self.dtb[:total_size]).hexdigest(),
            EXPECTED_FDT_SHA256,
        )
        self.assertIn(b":rg34xx_v1", self.dtb)
        twi = self.fdt["/soc@03000000/twi@0x07081400"]
        self.assertEqual(twi["device_type"].rstrip(b"\0"), b"twi5")
        self.assertEqual(twi["reg"], bytes.fromhex("00000000070814000000000000000400"))
        pmu = self.fdt["/soc@03000000/twi@0x07081400/pmu"]
        self.assertEqual(pmu["compatible"].rstrip(b"\0"), b"x-powers,axp2202")
        self.assertEqual(be32(pmu["reg"]), 0x34)

    def test_rg34xx_dram_gpio_straps_and_lpddr_profiles(self) -> None:
        selector = self.fdt["/soc@03000000/dram_select_para"]
        self.assertEqual(be32(selector["select_mode"]), 1)  # GPIO auto-detection mode
        self.assertEqual(be32(selector["gpadc_channel"]), 1)
        pinctrl = self.fdt["/soc@03000000/pinctrl@0300b000"]
        self.assertEqual(be32(pinctrl["phandle"]), 0x4B)
        self.assertEqual(
            pinctrl["compatible"].rstrip(b"\\0"),
            b"allwinner,sun50iw9p1-pinctrl",
        )
        self.assertEqual(be32(pinctrl["#gpio-cells"]), 6)
        expected_pins = [6, 7, 8, 10]
        for index, expected_pin in enumerate(expected_pins):
            cells = struct.unpack(">7I", selector[f"select_gpio{index}"])
            self.assertEqual(cells[0], 0x4B)  # pinctrl phandle
            self.assertEqual(cells[1], 7)  # Allwinner port H
            self.assertEqual(cells[2], expected_pin)
            self.assertEqual(cells[3], 0)  # GPIO input function
            self.assertEqual(cells[4], 1)  # pull-up
            self.assertEqual(cells[5:], (0xFFFFFFFF, 0xFFFFFFFF))

        lpddr3 = self.fdt["/soc@03000000/dram_para14"]
        lpddr4 = self.fdt["/soc@03000000/dram_para15"]
        self.assertEqual(be32(lpddr3["dram_type"]), 7)
        self.assertEqual(be32(lpddr3["dram_clk"]), 672)
        self.assertEqual(be32(lpddr4["dram_type"]), 8)
        self.assertEqual(be32(lpddr4["dram_clk"]), 672)
        self.assertEqual(lpddr4["dram_tpr6"], bytes.fromhex("40808080"))

    def test_pmic_regulator_ranges(self) -> None:
        root = "/soc@03000000/twi@0x07081400/pmu/regulators@0"
        dcdc2 = self.fdt[root + "/dcdc2"]
        dcdc3 = self.fdt[root + "/dcdc3"]
        self.assertEqual(be32(dcdc2["regulator-min-microvolt"]), 500000)
        self.assertEqual(be32(dcdc2["regulator-max-microvolt"]), 3400000)
        self.assertEqual(be32(dcdc3["regulator-min-microvolt"]), 500000)
        self.assertEqual(be32(dcdc3["regulator-max-microvolt"]), 1840000)


if __name__ == "__main__":
    unittest.main()
