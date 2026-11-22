#!/usr/bin/env python3
"""Inspect H700 DTB/FDT memory, panel, PMIC, USB and bootargs without dependencies."""
from __future__ import annotations

import argparse
import hashlib
import json
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


def inspect_fdt(path: Path) -> dict[str, Any]:
    blob = path.read_bytes()
    nodes = parse_fdt(blob)
    root = nodes.get("/", {})
    addr_cells = _cell_count(root, "#address-cells", 2)
    size_cells = _cell_count(root, "#size-cells", 1)

    model = _strings(root.get("model"))
    compatible = _strings(root.get("compatible"))
    chosen = nodes.get("/chosen", {})
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
        if any("axp2202" in item.lower() for item in comp):
            pmics.append({"node": node_path, "compatible": comp,
                          "status": (_strings(props.get("status")) or ["okay"])[0]})
        leaf = node_path.rsplit("/", 1)[-1].lower()
        if node_path.startswith("/soc@") and leaf.startswith(("ehci", "ohci")) and comp:
            usb.append({"node": node_path, "compatible": comp,
                        "status": (_strings(props.get("status")) or ["okay"])[0]})
        if "debug_mode" in props:
            debug_modes.append({"node": node_path, "value": _number(props.get("debug_mode"))})

    return {
        "file": str(path),
        "file_bytes": len(blob),
        "sha256": hashlib.sha256(blob).hexdigest(),
        "totalsize": _u32(blob, 4),
        "model": model,
        "compatible": compatible,
        "chosen_bootargs": (_strings(chosen.get("bootargs")) or [None])[0],
        "memory_regions": memory,
        "reserved_memory_nodes": reserved,
        "has_reserved_memory_node": reserved_root is not None,
        "panels": panels,
        "pmic_nodes": pmics,
        "usb_controllers": sorted(usb, key=lambda item: item["node"]),
        "debug_modes": debug_modes,
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
