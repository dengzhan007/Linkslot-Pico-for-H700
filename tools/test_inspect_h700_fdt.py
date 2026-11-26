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
        "device_type", "reg", "lcd_driver_name", "lcd_x", "lcd_y", "lcd_if", "lcd_hv_if",
        "lcd_lvds_if", "lcd_dclk_freq", "lcd_ht", "lcd_hbp", "lcd_hspw", "lcd_vt",
        "lcd_vbp", "lcd_vspw", "status", "boot_fb0",
        "no-map", "debug_mode", "ranges", "linux,initrd-start", "linux,initrd-end",
        "method", "heap-name", "heap-id", "heap-base", "heap-size", "heap-type",
        "phandle", "cpu-supply", "regulator-name", "regulator-min-microvolt",
        "regulator-max-microvolt", "regulator-always-on", "regulator-boot-on",
        "operating-points-v2", "opp-hz", "opp-microvolt", "opp-supported-hw",
        "opp-shared", "nvmem-cells", "nvmem-cell-names", "opp-microvolt-a1", "opp-microvolt-b0",
        "opp-microvolt-c1", "thermal-sensors",
        "clock-frequency", "turbo-mode", "#thermal-sensor-cells",
        "#gpio-cells", "#interrupt-cells", "gpio-controller", "gpios",
        "label", "linux,code", "linux,input-type", "linux,input-value",
        "poll-interval", "key_cnt", "channel_num", "channel_select", "interrupts", "clocks",
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
    prop("linux,initrd-start", cells(0, 0x42000000))
    prop("linux,initrd-end", cells(0, 0x42000200))
    end()

    begin("memory@40000000")
    prop("device_type", b"memory\0")
    prop("reg", cells(0, 0x40000000, 0, 0x40000000))
    end()

    begin("cpus")
    prop("#address-cells", be32(1))
    prop("#size-cells", be32(0))
    begin("cpu@0")
    prop("device_type", b"cpu\0")
    prop("compatible", b"arm,cortex-a53\0")
    prop("reg", cells(0))
    prop("clock-frequency", be32(1512000000))
    prop("cpu-supply", be32(0x20))
    prop("operating-points-v2", be32(0x30))
    prop("nvmem-cells", be32(0x40))
    prop("nvmem-cell-names", b"speed-bin\0")
    prop("thermal-sensors", cells(0x50, 1))
    end()
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
    prop("compatible", b"linaro,optee-tz\0")
    prop("method", b"smc\0")
    end()
    end()

    begin("ion")
    begin("heap_secure@0")
    prop("compatible", b"allwinner,secure\0")
    prop("heap-name", b"secure\0")
    prop("heap-id", be32(6))
    prop("heap-base", be32(0))
    prop("heap-size", be32(0))
    prop("heap-type", b"ion_secure\0")
    end()
    end()

    begin("soc@03000000")
    begin("lcd0@01c0c000")
    prop("lcd_driver_name", b"rg34xx_v1\0")
    prop("lcd_x", be32(720))
    prop("lcd_y", be32(480))
    prop("lcd_if", be32(0))
    prop("lcd_hv_if", be32(0))
    prop("lcd_lvds_if", be32(0))
    prop("lcd_dclk_freq", be32(54))
    prop("lcd_ht", be32(1679))
    prop("lcd_hbp", be32(46))
    prop("lcd_hspw", be32(2))
    prop("lcd_vt", be32(536))
    prop("lcd_vbp", be32(20))
    prop("lcd_vspw", be32(5))
    prop("status", b"okay\0")
    end()

    begin("pinctrl@0300b000")
    prop("compatible", b"allwinner,sun50iw9p1-pinctrl\0")
    prop("reg", cells(0, 0x0300b000, 0, 0x400))
    prop("#gpio-cells", be32(6))
    prop("gpio-controller", b"")
    prop("phandle", be32(0x60))
    end()

    begin("gpio_keys")
    prop("compatible", b"gpio-keys-polled\0")
    prop("label", b"RG34XX Controller\0")
    prop("poll-interval", be32(20))
    begin("keyA")
    prop("label", b"A\0")
    prop("gpios", cells(0x60, 0, 0, 0, 1, 2, 1))
    prop("linux,code", be32(0x130))
    end()
    begin("keyUP")
    prop("label", b"Up\0")
    prop("gpios", cells(0x60, 0, 6, 0, 1, 2, 1))
    prop("linux,code", be32(0x11))
    prop("linux,input-type", be32(3))
    prop("linux,input-value", be32(0xffffffff))
    end()
    end()

    begin("gpadc")
    prop("compatible", b"allwinner,sunxi-gpadc\0")
    prop("reg", cells(0, 0x05070000, 0, 0x400))
    prop("interrupts", cells(0, 0x12, 0))
    prop("status", b"disabled\0")
    prop("key_cnt", be32(5))
    end()

    begin("keyboard")
    prop("compatible", b"allwinner,keyboard_1350mv\0")
    prop("reg", cells(0, 0x05070800, 0, 0x400))
    prop("status", b"disable\0")
    end()

    begin("powerkey@0")
    prop("compatible", b"x-powers,axp2202-pek\0")
    prop("status", b"okay\0")
    end()

    begin("pmu")
    prop("compatible", b"x-powers,axp2202\0")
    begin("regulator@0")
    prop("phandle", be32(0x20))
    prop("regulator-name", b"CPU_VDD\0")
    prop("regulator-min-microvolt", be32(800000))
    prop("regulator-max-microvolt", be32(1200000))
    prop("regulator-always-on", b"")
    end()
    end()

    begin("efuse@03006000")
    begin("speed-bin@0")
    prop("phandle", be32(0x40))
    prop("reg", cells(0x00, 2))
    end()
    end()

    begin("thermal@05070400")
    prop("phandle", be32(0x50))
    prop("#thermal-sensor-cells", be32(1))
    prop("compatible", b"allwinner,sun50i-thermal\0")
    end()

    begin("ehci2-controller@05310000")
    prop("compatible", b"allwinner,sunxi-ehci2\0")
    prop("status", b"okay\0")
    end()

    begin("ohci1-controller@05200400")
    prop("compatible", b"allwinner,sunxi-ohci1\0")
    prop("status", b"late\0")
    end()


    begin("opp-table@0")
    prop("phandle", be32(0x30))
    prop("compatible", b"operating-points-v2\0")
    prop("opp-shared", b"")
    prop("nvmem-cells", be32(0x40))
    prop("nvmem-cell-names", b"speed-bin\0")
    begin("opp-1000000000")
    prop("opp-hz", cells(0, 1000000000))
    prop("opp-microvolt", cells(900000))
    prop("opp-supported-hw", cells(1, 0xff))
    end()
    begin("opp-1512000000")
    prop("opp-hz", cells(0, 1512000000))
    prop("opp-microvolt", cells(1100000, 1050000, 1150000))
    prop("opp-microvolt-a1", cells(1150000))
    prop("opp-microvolt-b0", cells(1100000, 1050000, 1150000))
    prop("opp-supported-hw", cells(1, 0xff))
    prop("turbo-mode", b"")
    end()
    begin("opp-1800000000-no-supported-hw")
    prop("opp-hz", cells(0, 1800000000))
    prop("opp-microvolt-c1", cells(1180000))
    end()
    begin("opp-2000000000-empty-supported-hw")
    prop("opp-hz", cells(0, 2000000000))
    prop("opp-supported-hw", b"")
    prop("opp-microvolt", cells(1200000))
    end()
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
        panel = report["panels"][0]
        self.assertEqual(panel["driver"], "rg34xx_v1")
        self.assertEqual((panel["width"], panel["height"]), (720, 480))
        self.assertEqual(panel["interface"], "rgb666")
        self.assertEqual(panel["interface_props"]["lcd_if"], 0)
        self.assertEqual(panel["interface_props"]["lcd_hv_if"], 0)
        self.assertEqual(panel["interface_props"]["lcd_lvds_if"], 0)
        self.assertEqual(panel["timing"]["pixel_clock_hz"], 54000000)
        self.assertEqual(panel["timing"]["pixel_clock_mhz"], 54)
        self.assertEqual(panel["timing"]["h_active"], 720)
        self.assertEqual(panel["timing"]["h_total"], 1679)
        self.assertEqual(panel["timing"]["h_back_porch"], 46)
        self.assertEqual(panel["timing"]["h_sync_pulse"], 2)
        self.assertEqual(panel["timing"]["h_front_porch"], 911)
        self.assertEqual(panel["timing"]["v_active"], 480)
        self.assertEqual(panel["timing"]["v_total"], 536)
        self.assertEqual(panel["timing"]["v_back_porch"], 20)
        self.assertEqual(panel["timing"]["v_sync_pulse"], 5)
        self.assertEqual(panel["timing"]["v_front_porch"], 31)
        self.assertEqual(panel["timing"]["nominal_refresh_hz"], 60.004)
        self.assertEqual(report["pmic_nodes"][0]["compatible"], ["x-powers,axp2202"])
        cpu_power = report["cpu_power"]
        self.assertEqual(len(cpu_power["cpu_nodes"]), 1)
        cpu = cpu_power["cpu_nodes"][0]
        self.assertEqual(cpu["node"], "/cpus/cpu@0")
        self.assertEqual(cpu["clock_frequency_hz"], 1512000000)
        self.assertEqual(cpu["cpu_supply"][0]["node"], "/soc@03000000/pmu/regulator@0")
        self.assertEqual(cpu["cpu_supply"][0]["regulator"]["name"], "CPU_VDD")
        self.assertEqual(cpu["operating_points_v2"][0]["node"], "/soc@03000000/opp-table@0")
        self.assertEqual(cpu["nvmem_cells"][0]["node"], "/soc@03000000/efuse@03006000/speed-bin@0")
        self.assertEqual(cpu["nvmem_cells"][0]["name"], "speed-bin")
        self.assertEqual(cpu["nvmem_cells"][0]["node_details"]["reg_cells"], [0, 2])
        self.assertEqual(cpu["thermal_sensors"][0]["node"], "/soc@03000000/thermal@05070400")
        self.assertEqual(cpu["thermal_sensors"][0]["node_details"]["compatible"], ["allwinner,sun50i-thermal"])
        self.assertEqual(cpu["thermal_sensors"][0]["arguments"], [1])
        self.assertEqual(len(cpu["thermal_sensors"]), 1)
        opp_table = cpu_power["cpu_opp_tables"][0]
        self.assertEqual(opp_table["nvmem_cell_names"], ["speed-bin"])
        self.assertEqual(opp_table["nvmem_cells"][0]["node"], "/soc@03000000/efuse@03006000/speed-bin@0")
        self.assertEqual(opp_table["nvmem_cells"][0]["name"], "speed-bin")
        opp_entries = opp_table["entries"]
        self.assertEqual(
            [entry["frequency_hz"] for entry in opp_entries],
            [1000000000, 1512000000, 1800000000, 2000000000],
        )
        self.assertTrue(opp_entries[0]["supported_hw_present"])
        self.assertEqual(opp_entries[0]["supported_hw_cell_count"], 2)
        self.assertTrue(opp_entries[0]["voltage_microvolt_present"])
        self.assertEqual(opp_entries[1]["voltage_microvolt_cells"], [1100000, 1050000, 1150000])
        self.assertTrue(opp_entries[1]["voltage_microvolt_present"])
        self.assertEqual(opp_entries[1]["voltage_microvolt_variant_names"], ["a1", "b0"])
        self.assertEqual(opp_entries[1]["voltage_microvolt_variants"], {
            "a1": [1150000],
            "b0": [1100000, 1050000, 1150000],
        })
        self.assertFalse(opp_entries[2]["supported_hw_present"])
        self.assertEqual(opp_entries[2]["supported_hw_cell_count"], 0)
        self.assertFalse(opp_entries[2]["voltage_microvolt_present"])
        self.assertEqual(opp_entries[2]["voltage_microvolt_variant_names"], ["c1"])
        self.assertEqual(opp_entries[2]["voltage_microvolt_variants"], {"c1": [1180000]})
        self.assertTrue(opp_entries[3]["supported_hw_present"])
        self.assertEqual(opp_entries[3]["supported_hw_cells"], [])
        self.assertEqual(opp_entries[3]["supported_hw_cell_count"], 0)
        self.assertTrue(opp_entries[3]["voltage_microvolt_present"])
        self.assertIn("do not prove", cpu_power["note"])
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

    def test_extracts_gpio_input_map_and_disabled_analog_key_nodes(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "synthetic.dtb"
            path.write_bytes(make_fdt())
            report = MODULE.inspect_fdt(path)

        inputs = report["input_devices"]
        pio = next(
            item for item in inputs["gpio_controllers"]
            if item["compatible"] == ["allwinner,sun50iw9p1-pinctrl"]
        )
        self.assertEqual(pio["gpio_cells"], 6)
        self.assertEqual(pio["reg_cells"], [0, 0x0300b000, 0, 0x400])

        gpio_keys = inputs["gpio_key_devices"][0]
        self.assertEqual(gpio_keys["compatible"], ["gpio-keys-polled"])
        self.assertEqual(gpio_keys["status"], "okay")
        self.assertEqual(gpio_keys["poll_interval_ms"], 20)

        key_a = next(item for item in gpio_keys["keys"] if item["label"] == "A")
        self.assertEqual(key_a["linux_code"], 0x130)
        self.assertEqual(key_a["input_type_name"], "EV_KEY")
        gpio_a = key_a["gpios"][0]
        self.assertEqual(gpio_a["controller_node"], "/soc@03000000/pinctrl@0300b000")
        self.assertEqual(gpio_a["pin_name"], "PA0")
        self.assertEqual(gpio_a["mux_select"], 0)
        self.assertEqual(gpio_a["pull_config"], 1)
        self.assertEqual(gpio_a["drive_level"], 2)
        self.assertTrue(gpio_a["active_low"])

        key_up = next(item for item in gpio_keys["keys"] if item["label"] == "Up")
        self.assertEqual(key_up["input_type_name"], "EV_ABS")
        self.assertEqual(key_up["input_value"], -1)
        self.assertEqual(key_up["gpios"][0]["pin_name"], "PA6")

        analog = {item["compatible"][0]: item for item in inputs["analog_key_controllers"]}
        self.assertEqual(analog["allwinner,sunxi-gpadc"]["status"], "disabled")
        self.assertEqual(analog["allwinner,sunxi-gpadc"]["key_count"], 5)
        self.assertEqual(analog["allwinner,keyboard_1350mv"]["status"], "disable")
        self.assertEqual(inputs["pmic_power_keys"][0]["compatible"], ["x-powers,axp2202-pek"])
        self.assertIn("do not prove", inputs["note"])

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
