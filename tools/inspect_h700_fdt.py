#!/usr/bin/env python3
"""Inspect H700 DTB/FDT memory, panel, PMIC, USB and bootargs without dependencies."""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import struct
import sys
from pathlib import Path
from typing import Any

FDT_MAGIC = 0xD00DFEED
FDT_BEGIN_NODE = 1
FDT_END_NODE = 2
FDT_PROP = 3
FDT_NOP = 4
FDT_END = 9
HEADER_BYTES = 40


class FdtError(ValueError):
    """Invalid or unsupported flattened device tree."""


def _u32(data: bytes, offset: int) -> int:
    if offset < 0 or offset + 4 > len(data):
        raise FdtError("32-bit field extends beyond input")
    return struct.unpack_from(">I", data, offset)[0]


def parse_fdt(data: bytes) -> dict[str, dict[str, bytes]]:
    if len(data) < HEADER_BYTES:
        raise FdtError("input is shorter than the 40-byte FDT header")
    (magic, total, off_struct, off_strings, off_rsv, version, last_comp,
     boot_cpu, size_strings, size_struct) = struct.unpack_from(">10I", data, 0)
    if magic != FDT_MAGIC:
        raise FdtError("invalid FDT magic")
    if total < HEADER_BYTES or total > len(data):
        raise FdtError("FDT totalsize is outside the captured input")
    if version < 16 or last_comp > version:
        raise FdtError("unsupported FDT version")
    if off_struct % 4 or off_strings % 4:
        raise FdtError("FDT structure/string blocks are not 4-byte aligned")
    struct_end = off_struct + size_struct
    strings_end = off_strings + size_strings
    if off_struct < HEADER_BYTES or struct_end > total:
        raise FdtError("FDT structure block is out of bounds")
    if off_strings < HEADER_BYTES or strings_end > total:
        raise FdtError("FDT strings block is out of bounds")
    if off_rsv < HEADER_BYTES or off_rsv + 16 > total:
        raise FdtError("FDT reserve-map offset is out of bounds")

    structure = data[off_struct:struct_end]
    strings = data[off_strings:strings_end]
    nodes: dict[str, dict[str, bytes]] = {}
    stack: list[str] = []
    pos = 0
    root_seen = False
    ended = False

    while pos + 4 <= len(structure):
        token = _u32(structure, pos)
        pos += 4
        if token == FDT_BEGIN_NODE:
            end = structure.find(b"\0", pos)
            if end < 0:
                raise FdtError("unterminated FDT node name")
            try:
                name = structure[pos:end].decode("utf-8")
            except UnicodeDecodeError as exc:
                raise FdtError("FDT node name is not UTF-8") from exc
            pos = (end + 4) & ~3
            if pos > len(structure):
                raise FdtError("FDT node name alignment exceeds structure block")
            if not stack:
                if root_seen or name != "":
                    raise FdtError("FDT must contain exactly one empty-name root node")
                root_seen = True
                path = "/"
            else:
                parent_names = [part for part in stack if part]
                path = "/" + "/".join([*parent_names, name])
            if path in nodes:
                raise FdtError(f"duplicate FDT node path: {path}")
            nodes[path] = {}
            stack.append(name)
        elif token == FDT_END_NODE:
            if not stack:
                raise FdtError("FDT END_NODE without a matching BEGIN_NODE")
            stack.pop()
        elif token == FDT_PROP:
            if not stack or pos + 8 > len(structure):
                raise FdtError("FDT property appears outside a node or has a short header")
            length, nameoff = struct.unpack_from(">II", structure, pos)
            pos += 8
            if nameoff >= len(strings):
                raise FdtError("FDT property name offset is out of bounds")
            name_end = strings.find(b"\0", nameoff)
            if name_end < 0:
                raise FdtError("unterminated FDT property name")
            try:
                prop_name = strings[nameoff:name_end].decode("utf-8")
            except UnicodeDecodeError as exc:
                raise FdtError("FDT property name is not UTF-8") from exc
            value_end = pos + length
            if value_end > len(structure):
                raise FdtError("FDT property value extends beyond structure block")
            value = structure[pos:value_end]
            pos = (value_end + 3) & ~3
            if pos > len(structure):
                raise FdtError("FDT property alignment exceeds structure block")
            stack_names = [part for part in stack if part]
            path = "/" if not stack_names else "/" + "/".join(stack_names)
            if prop_name in nodes[path]:
                raise FdtError(f"duplicate property {prop_name!r} in {path}")
            nodes[path][prop_name] = value
        elif token == FDT_NOP:
            continue
        elif token == FDT_END:
            if stack:
                raise FdtError("FDT END reached with unclosed nodes")
            ended = True
            break
        else:
            raise FdtError(f"unknown FDT structure token 0x{token:08x}")

    if not root_seen or not ended:
        raise FdtError("FDT structure is missing its root or END token")
    return nodes


def _strings(value: bytes | None) -> list[str]:
    if value is None:
        return []
    return [part.decode("utf-8", "replace") for part in value.split(b"\0") if part]


def _cells(value: bytes | None) -> list[int]:
    if value is None or len(value) % 4:
        return []
    return list(struct.unpack(">" + "I" * (len(value) // 4), value))


def _cell_count(props: dict[str, bytes], name: str, default: int) -> int:
    values = _cells(props.get(name))
    count = values[0] if values else default
    if count < 1 or count > 2:
        raise FdtError(f"unsupported {name} value {count}; expected 1 or 2")
    return count


def _number(value: bytes | None) -> int | None:
    values = _cells(value)
    if not values:
        return None
    if len(values) == 1:
        return values[0]
    return (values[-2] << 32) | values[-1]


def _reg_ranges(value: bytes | None, address_cells: int, size_cells: int) -> list[dict[str, str]]:
    if value is None:
        return []
    stride = (address_cells + size_cells) * 4
    if not stride or len(value) % stride:
        raise FdtError("reg property length does not match address/size cell counts")
    ranges = []
    for offset in range(0, len(value), stride):
        cells = struct.unpack_from(">" + "I" * (address_cells + size_cells), value, offset)
        address = 0
        size = 0
        for cell in cells[:address_cells]:
            address = (address << 32) | cell
        for cell in cells[address_cells:]:
            size = (size << 32) | cell
        ranges.append({"start": f"0x{address:x}", "size": f"0x{size:x}",
                       "end_exclusive": f"0x{address + size:x}"})
    return ranges


def _phandle_index(nodes: dict[str, dict[str, bytes]]) -> dict[int, str]:
    index: dict[int, str] = {}
    for node_path, props in nodes.items():
        for prop_name in ("phandle", "linux,phandle"):
            values = _cells(props.get(prop_name))
            if len(values) == 1 and values[0] != 0:
                index[values[0]] = node_path
    return index


def _phandle_refs(
    value: bytes | None,
    index: dict[int, str],
    nodes: dict[str, dict[str, bytes]] | None = None,
    argument_count_property: str | None = None,
) -> list[dict[str, Any]]:
    values = _cells(value)
    refs = []
    offset = 0
    while offset < len(values):
        handle = values[offset]
        offset += 1
        node_path = index.get(handle)
        argument_count = 0
        if argument_count_property and nodes and node_path in nodes:
            count = _number(nodes[node_path].get(argument_count_property))
            argument_count = count if count is not None else 0
        if argument_count < 0 or offset + argument_count > len(values):
            refs.append({
                "phandle": f"0x{handle:x}",
                "node": node_path,
                "arguments": values[offset:],
                "arguments_truncated": True,
            })
            break
        arguments = values[offset:offset + argument_count]
        offset += argument_count
        ref = {
            "phandle": f"0x{handle:x}",
            "node": node_path,
        }
        if argument_count:
            ref["arguments"] = arguments
        refs.append(ref)
    return refs


def _regulator_summary(node_path: str, props: dict[str, bytes]) -> dict[str, Any]:
    minimum = _number(props.get("regulator-min-microvolt"))
    maximum = _number(props.get("regulator-max-microvolt"))
    return {
        "node": node_path,
        "name": (_strings(props.get("regulator-name")) or [None])[0],
        "minimum_microvolt": minimum,
        "maximum_microvolt": maximum,
        "always_on": "regulator-always-on" in props,
        "boot_on": "regulator-boot-on" in props,
        "status": (_strings(props.get("status")) or ["okay"])[0],
    }


def _cpu_power_inventory(nodes: dict[str, dict[str, bytes]]) -> dict[str, Any]:
    phandles = _phandle_index(nodes)
    regulators = {
        node_path: _regulator_summary(node_path, props)
        for node_path, props in nodes.items()
        if "regulator-name" in props
    }

    def node_details(node_path: str | None) -> dict[str, Any] | None:
        if not node_path or node_path not in nodes:
            return None
        props = nodes[node_path]
        return {
            "compatible": _strings(props.get("compatible")),
            "status": (_strings(props.get("status")) or ["okay"])[0],
            "name": (_strings(props.get("name")) or [None])[0],
            "reg_cells": _cells(props.get("reg")),
        }

    opp_tables: dict[str, dict[str, Any]] = {}
    cpus = []

    for node_path, props in nodes.items():
        device_type = (_strings(props.get("device_type")) or [""])[0]
        leaf = node_path.rsplit("/", 1)[-1].lower()
        if device_type != "cpu" and not (
            node_path.startswith("/cpus/") and leaf.startswith("cpu@")
        ):
            continue

        supply_refs = _phandle_refs(props.get("cpu-supply"), phandles)
        for ref in supply_refs:
            resolved = regulators.get(ref["node"])
            if resolved:
                ref["regulator"] = resolved

        opp_refs = _phandle_refs(props.get("operating-points-v2"), phandles)
        for ref in opp_refs:
            table_path = ref["node"]
            if not table_path or table_path not in nodes:
                continue
            if table_path not in opp_tables:
                entries = []
                prefix = table_path.rstrip("/") + "/"
                for child_path, child_props in nodes.items():
                    if not child_path.startswith(prefix):
                        continue
                    frequency = _number(child_props.get("opp-hz"))
                    if frequency is None:
                        continue
                    voltage_variants = {}
                    voltage_variant_names = []
                    for property_name, raw_value in child_props.items():
                        if not property_name.startswith("opp-microvolt-"):
                            continue
                        variant_name = property_name[len("opp-microvolt-"):]
                        voltage_variant_names.append(variant_name)
                        values = _cells(raw_value)
                        if values:
                            voltage_variants[variant_name] = values
                    supported_hw_present = "opp-supported-hw" in child_props
                    voltage_microvolt_present = "opp-microvolt" in child_props
                    entries.append({
                        "node": child_path,
                        "frequency_hz": frequency,
                        "voltage_microvolt_present": voltage_microvolt_present,
                        "voltage_microvolt_cells": _cells(child_props.get("opp-microvolt")),
                        "voltage_microvolt_variant_names": sorted(voltage_variant_names),
                        "voltage_microvolt_variants": voltage_variants,
                        "supported_hw_present": supported_hw_present,
                        "supported_hw_cells": _cells(child_props.get("opp-supported-hw")),
                        "supported_hw_cell_count": len(_cells(child_props.get("opp-supported-hw"))),
                        "turbo": "turbo-mode" in child_props,
                        "status": (_strings(child_props.get("status")) or ["okay"])[0],
                    })
                table_props = nodes[table_path]
                table_nvmem_names = _strings(table_props.get("nvmem-cell-names"))
                table_nvmem_refs = _phandle_refs(
                    table_props.get("nvmem-cells"),
                    phandles,
                    nodes,
                    "#nvmem-cell-cells",
                )
                for index, ref in enumerate(table_nvmem_refs):
                    if index < len(table_nvmem_names):
                        ref["name"] = table_nvmem_names[index]
                opp_tables[table_path] = {
                    "node": table_path,
                    "compatible": _strings(table_props.get("compatible")),
                    "shared": "opp-shared" in table_props,
                    "nvmem_cells": table_nvmem_refs,
                    "nvmem_cell_names": table_nvmem_names,
                    "entries": sorted(entries, key=lambda entry: entry["frequency_hz"]),
                }

        nvmem_names = _strings(props.get("nvmem-cell-names"))
        nvmem_refs = _phandle_refs(
            props.get("nvmem-cells"),
            phandles,
            nodes,
            "#nvmem-cell-cells",
        )
        for i, ref in enumerate(nvmem_refs):
            if i < len(nvmem_names):
                ref["name"] = nvmem_names[i]
            ref["node_details"] = node_details(ref["node"])

        thermal_refs = _phandle_refs(
            props.get("thermal-sensors"),
            phandles,
            nodes,
            "#thermal-sensor-cells",
        )
        for ref in thermal_refs:
            ref["node_details"] = node_details(ref["node"])

        cpus.append({
            "node": node_path,
            "compatible": _strings(props.get("compatible")),
            "status": (_strings(props.get("status")) or ["okay"])[0],
            "reg_cells": _cells(props.get("reg")),
            "clock_frequency_hz": _number(props.get("clock-frequency")),
            "cpu_supply": supply_refs,
            "operating_points_v2": opp_refs,
            "nvmem_cells": nvmem_refs,
            "thermal_sensors": thermal_refs,
        })

    return {
        "cpu_nodes": cpus,
        "regulator_nodes": sorted(regulators.values(), key=lambda item: item["node"]),
        "cpu_opp_tables": list(opp_tables.values()),
        "phandle_node_count": len(phandles),
        "note": (
            "These are static device-tree declarations only. They do not prove "
            "the active CPU voltage, current OPP, SID raw value, thermal calibration, "
            "regulator readback, or that an OPP is safe to apply."
        ),
    }


def inspect_fdt(path: Path) -> dict[str, Any]:
    blob = path.read_bytes()
    nodes = parse_fdt(blob)
    root = nodes.get("/", {})
    addr_cells = _cell_count(root, "#address-cells", 2)
    size_cells = _cell_count(root, "#size-cells", 1)

    model = _strings(root.get("model"))
    compatible = _strings(root.get("compatible"))
    chosen = nodes.get("/chosen", {})
    chosen_bootargs = (_strings(chosen.get("bootargs")) or [None])[0]
    initrd_start = _number(chosen.get("linux,initrd-start"))
    initrd_end = _number(chosen.get("linux,initrd-end"))
    initrd = None
    if initrd_start is not None or initrd_end is not None:
        valid_initrd = (
            initrd_start is not None
            and initrd_end is not None
            and initrd_end >= initrd_start
        )
        initrd = {
            "start": f"0x{initrd_start:x}" if initrd_start is not None else None,
            "end_exclusive": f"0x{initrd_end:x}" if initrd_end is not None else None,
            "size_bytes": initrd_end - initrd_start if valid_initrd else None,
            "range_valid": valid_initrd,
        }
    disp_reserve_match = re.search(
        r"(?:^|\s)disp_reserve=([0-9]+),(0x[0-9a-fA-F]+)(?:\s|$)",
        chosen_bootargs or "",
    )
    disp_reserve = None
    if disp_reserve_match:
        disp_reserve = {
            "size_bytes": int(disp_reserve_match.group(1), 10),
            "base": int(disp_reserve_match.group(2), 16),
        }
    memory = []
    for node_path, props in nodes.items():
        if _strings(props.get("device_type")) == ["memory"] or node_path.rsplit("/", 1)[-1].startswith("memory@"):
            memory.extend({"node": node_path, **entry}
                          for entry in _reg_ranges(props.get("reg"), addr_cells, size_cells))

    reserved = []
    reserved_root = nodes.get("/reserved-memory")
    if reserved_root is not None:
        r_addr = _cell_count(reserved_root, "#address-cells", addr_cells)
        r_size = _cell_count(reserved_root, "#size-cells", size_cells)
        for node_path, props in nodes.items():
            if node_path.startswith("/reserved-memory/"):
                entries = _reg_ranges(props.get("reg"), r_addr, r_size)
                reserved.append({
                    "node": node_path,
                    "ranges": entries,
                    "size": f"0x{_number(props.get('size')):x}" if _number(props.get("size")) is not None else None,
                    "no_map": "no-map" in props,
                    "reusable": "reusable" in props,
                    "status": (_strings(props.get("status")) or ["okay"])[0],
                })

    panels = []
    pmics = []
    usb = []
    debug_modes = []
    boot_framebuffers = []
    firmware_nodes = []
    ion_heaps = []
    for node_path, props in nodes.items():
        if "lcd_driver_name" in props:
            panels.append({
                "node": node_path,
                "driver": (_strings(props.get("lcd_driver_name")) or [""])[0],
                "width": _number(props.get("lcd_x")),
                "height": _number(props.get("lcd_y")),
                "status": (_strings(props.get("status")) or ["okay"])[0],
            })
        comp = _strings(props.get("compatible"))
        if node_path.startswith("/firmware/") and comp:
            firmware_nodes.append({
                "node": node_path,
                "compatible": comp,
                "method": (_strings(props.get("method")) or [None])[0],
                "status": (_strings(props.get("status")) or ["okay"])[0],
            })
        if "heap-name" in props:
            heap_base = _number(props.get("heap-base"))
            heap_size = _number(props.get("heap-size"))
            ion_heaps.append({
                "node": node_path,
                "compatible": comp,
                "name": (_strings(props.get("heap-name")) or [""])[0],
                "id": _number(props.get("heap-id")),
                "base": f"0x{heap_base:x}" if heap_base is not None else None,
                "size_bytes": heap_size,
                "type": (_strings(props.get("heap-type")) or [None])[0],
                "status": (_strings(props.get("status")) or ["okay"])[0],
            })
        if any("axp2202" in item.lower() for item in comp):
            pmics.append({"node": node_path, "compatible": comp,
                          "status": (_strings(props.get("status")) or ["okay"])[0]})
        leaf = node_path.rsplit("/", 1)[-1].lower()
        if node_path.startswith("/soc@") and leaf.startswith(("ehci", "ohci")) and comp:
            usb.append({"node": node_path, "compatible": comp,
                        "status": (_strings(props.get("status")) or ["okay"])[0]})
        if "debug_mode" in props:
            debug_modes.append({"node": node_path, "value": _number(props.get("debug_mode"))})
        if "boot_fb0" in props:
            raw_fb = (_strings(props.get("boot_fb0")) or [""])[0]
            fields = raw_fb.split(",")
            fb_report: dict[str, Any] = {"node": node_path, "raw": raw_fb}
            try:
                if len(fields) < 5:
                    raise ValueError("boot_fb0 has fewer than five fields")
                base = int(fields[0], 16)
                width = int(fields[1], 16)
                height = int(fields[2], 16)
                bits_per_pixel = int(fields[3], 16)
                stride_bytes = int(fields[4], 16)
                size_bytes = stride_bytes * height
                fb_report.update({
                    "base": f"0x{base:x}",
                    "width": width,
                    "height": height,
                    "bits_per_pixel": bits_per_pixel,
                    "stride_bytes": stride_bytes,
                    "size_bytes": size_bytes,
                    "end_exclusive": f"0x{base + size_bytes:x}",
                    "remaining_hex_fields": fields[5:],
                    "matches_disp_reserve": bool(
                        disp_reserve
                        and disp_reserve["base"] == base
                        and disp_reserve["size_bytes"] == size_bytes
                    ),
                })
            except (ValueError, OverflowError) as exc:
                fb_report["parse_error"] = str(exc)
            boot_framebuffers.append(fb_report)

    return {
        "file": str(path),
        "file_bytes": len(blob),
        "sha256": hashlib.sha256(blob).hexdigest(),
        "totalsize": _u32(blob, 4),
        "model": model,
        "compatible": compatible,
        "chosen_bootargs": chosen_bootargs,
        "initrd": initrd,
        "firmware_nodes": firmware_nodes,
        "ion_heaps": ion_heaps,
        "disp_reserve_bootarg": ({
            "size_bytes": disp_reserve["size_bytes"],
            "base": f"0x{disp_reserve['base']:x}",
        } if disp_reserve else None),
        "memory_regions": memory,
        "reserved_memory_nodes": reserved,
        "has_reserved_memory_node": reserved_root is not None,
        "panels": panels,
        "pmic_nodes": pmics,
        "usb_controllers": sorted(usb, key=lambda item: item["node"]),
        "debug_modes": debug_modes,
        "boot_framebuffers": boot_framebuffers,
        "cpu_power": _cpu_power_inventory(nodes),
        "node_count": len(nodes),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("dtb", nargs="+", type=Path, help="one or more DTB/FDT files")
    parser.add_argument("-o", "--output", type=Path, help="write JSON report to this path")
    args = parser.parse_args()
    try:
        reports = [inspect_fdt(path) for path in args.dtb]
    except (OSError, FdtError, struct.error) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1
    rendered = json.dumps(reports if len(reports) > 1 else reports[0], indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    else:
        print(rendered, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
