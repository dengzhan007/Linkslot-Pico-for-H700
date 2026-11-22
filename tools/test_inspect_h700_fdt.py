#!/usr/bin/env python3
"""Synthetic tests for the H700 flattened-device-tree inventory parser."""

import importlib.util
import struct
import tempfile
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).with_name("inspect_h700_fdt.py")
SPEC = importlib.util.spec_from_file_location("inspect_h700_fdt", MODULE_PATH)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def be32(value):
    return struct.pack(">I", value)


def cells(*values):
    return b"".join(be32(value) for value in values)


def make_fdt():
    property_names = (
        "#address-cells", "#size-cells", "model", "compatible", "bootargs",
        "device_type", "reg", "lcd_driver_name", "lcd_x", "lcd_y", "status", "boot_fb0",
        "no-map", "debug_mode", "ranges", "linux,initrd-start", "linux,initrd-end",
        "method", "heap-name", "heap-id", "heap-base", "heap-size", "heap-type",
    )
    strings = bytearray()
    name_offsets = {}
    for name in property_names:
        name_offsets[name] = len(strings)
        strings.extend(name.encode("ascii") + b"\0")

    structure = bytearray()

    def token(value):
        structure.extend(be32(value))

    def begin(name):
        token(MODULE.FDT_BEGIN_NODE)
        structure.extend(name.encode("utf-8") + b"\0")
        structure.extend(bytes((-len(structure)) % 4))

    def end():
        token(MODULE.FDT_END_NODE)

    def prop(name, value):
        token(MODULE.FDT_PROP)
        structure.extend(be32(len(value)))
        structure.extend(be32(name_offsets[name]))
        structure.extend(value)
        structure.extend(bytes((-len(structure)) % 4))

    begin("")
    prop("#address-cells", be32(2))
    prop("#size-cells", be32(2))
    prop("model", b"sun50iw9\0")
    prop("compatible", b"allwinner,h616\0arm,sun50iw9p1\0")

    begin("chosen")
    prop("bootargs", b"root=/dev/mmcblk0p5 cma=64M disp_reserve=1382400,0x7bf24800\0")
    end()

    begin("memory@40000000")
    prop("device_type", b"memory\0")
    prop("reg", cells(0, 0x40000000, 0, 0x40000000))
    end()

    begin("reserved-memory")
    prop("#address-cells", be32(2))
    prop("#size-cells", be32(2))
    prop("ranges", b"")
    begin("framebuffer@7bf24800")
    prop("reg", cells(0, 0x7bf24800, 0, 1382400))
    prop("no-map", b"")
    end()
    end()

    begin("firmware")
    begin("optee")
    prop("compatible", b"linaro,optee-tz\\0")
    prop("method", b"smc\\0")
    end()
    end()

    begin("ion")
    begin("heap_secure@0")
    prop("compatible", b"allwinner,secure\\0")
    prop("heap-name", b"secure\\0")
    prop("heap-id", be32(6))
    prop("heap-base", be32(0))
    prop("heap-size", be32(0))
    prop("heap-type", b"ion_secure\\0")
    end()
    end()

    begin("soc@03000000")
    begin("lcd0@01c0c000")
    prop("lcd_driver_name", b"rg34xx_v1\0")
    prop("lcd_x", be32(720))
    prop("lcd_y", be32(480))
    prop("status", b"okay\0")
    end()

    begin("pmu")
    prop("compatible", b"x-powers,axp2202\0")
    end()

    begin("ehci2-controller@05310000")
    prop("compatible", b"allwinner,sunxi-ehci2\0")
    prop("status", b"okay\0")
    end()

    begin("ohci1-controller@05200400")
    prop("compatible", b"allwinner,sunxi-ohci1\0")
    prop("status", b"late\0")
    end()

    begin("disp@01000000")
    prop("boot_fb0", b"7bf24800,2d0,1e0,20,b40,0,0,2d0,1e0\0")
    end()

    begin("platform")
    prop("debug_mode", be32(0))
    end()
    end()
    end()
    token(MODULE.FDT_END)

    reserve_map = bytes(16)
    off_reserve = MODULE.HEADER_BYTES
    off_struct = off_reserve + len(reserve_map)
    off_strings = off_struct + len(structure)
    total = off_strings + len(strings)
    header = struct.pack(
        ">10I", MODULE.FDT_MAGIC, total, off_struct, off_strings, off_reserve,
        17, 16, 0, len(strings), len(structure),
    )
    return header + reserve_map + structure + strings


class FdtInspectorTests(unittest.TestCase):
    def test_extracts_memory_reserved_panel_pmic_usb_and_bootargs(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "synthetic.dtb"
            path.write_bytes(make_fdt())
            report = MODULE.inspect_fdt(path)

        self.assertEqual(report["model"], ["sun50iw9"])
        self.assertEqual(report["compatible"], ["allwinner,h616", "arm,sun50iw9p1"])
        self.assertEqual(report["memory_regions"][0]["start"], "0x40000000")
        self.assertEqual(report["memory_regions"][0]["size"], "0x40000000")
        self.assertTrue(report["has_reserved_memory_node"])
        self.assertEqual(report["reserved_memory_nodes"][0]["ranges"][0]["start"], "0x7bf24800")
        self.assertTrue(report["reserved_memory_nodes"][0]["no_map"])
        self.assertEqual(report["panels"][0]["driver"], "rg34xx_v1")
        self.assertEqual((report["panels"][0]["width"], report["panels"][0]["height"]), (720, 480))
        self.assertEqual(report["pmic_nodes"][0]["compatible"], ["x-powers,axp2202"])
        self.assertEqual([x["status"] for x in report["usb_controllers"]], ["okay", "late"])
        self.assertEqual(report["debug_modes"][0]["value"], 0)
        self.assertEqual(report["chosen_bootargs"], "root=/dev/mmcblk0p5 cma=64M disp_reserve=1382400,0x7bf24800")
        self.assertEqual(report["initrd"], {
            "start": "0x42000000",
            "end_exclusive": "0x42000200",
            "size_bytes": 512,
            "range_valid": True,
        })
        self.assertEqual(report["firmware_nodes"], [{
            "node": "/firmware/optee",
            "compatible": ["linaro,optee-tz"],
            "method": "smc",
            "status": "okay",
        }])
        self.assertEqual(report["ion_heaps"], [{
            "node": "/ion/heap_secure@0",
            "compatible": ["allwinner,secure"],
            "name": "secure",
            "id": 6,
            "base": "0x0",
            "size_bytes": 0,
            "type": "ion_secure",
            "status": "okay",
        }])
        self.assertEqual(report["disp_reserve_bootarg"], {"size_bytes": 1382400, "base": "0x7bf24800"})
        framebuffer = report["boot_framebuffers"][0]
        self.assertEqual(framebuffer["base"], "0x7bf24800")
        self.assertEqual((framebuffer["width"], framebuffer["height"]), (720, 480))
        self.assertEqual(framebuffer["bits_per_pixel"], 32)
        self.assertEqual(framebuffer["stride_bytes"], 2880)
        self.assertEqual(framebuffer["size_bytes"], 1382400)
        self.assertEqual(framebuffer["end_exclusive"], "0x7c076000")
        self.assertTrue(framebuffer["matches_disp_reserve"])
        self.assertEqual(len(report["sha256"]), 64)

    def test_rejects_bad_magic(self):
        data = bytearray(make_fdt())
        struct.pack_into(">I", data, 0, 0)
        with self.assertRaisesRegex(MODULE.FdtError, "magic"):
            MODULE.parse_fdt(bytes(data))

    def test_rejects_totalsize_beyond_capture(self):
        data = bytearray(make_fdt())
        struct.pack_into(">I", data, 4, len(data) + 1)
        with self.assertRaisesRegex(MODULE.FdtError, "totalsize"):
            MODULE.parse_fdt(bytes(data))


if __name__ == "__main__":
    unittest.main()
