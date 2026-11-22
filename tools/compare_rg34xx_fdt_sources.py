#!/usr/bin/env python3
"""Compare the three RG34XX candidate FDT sources without uploading firmware.

Inputs are local read-only captures:
  - complete Android p4 boot partition image
  - raw Allwinner TOC1 package
  - live FDT dumped from the running Linux system

The report contains hashes, node/property names, lengths and equality results.
It deliberately does not print property values (for example serial numbers or
MAC addresses). No source file or block device is modified.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import struct
from pathlib import Path
from typing import Any

import inspect_android_bootimg as android
import inspect_h700_fdt as fdt
import parse_h700_toc1 as toc1


class CompareError(ValueError):
    """Invalid input or unsupported FDT source."""


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _fdt_blob(data: bytes, label: str) -> bytes:
    if len(data) < 40:
        raise CompareError(f"{label}: FDT input is shorter than the header")
    total = struct.unpack_from(">I", data, 4)[0]
    if total < 40 or total > len(data):
        raise CompareError(
            f"{label}: FDT totalsize 0x{total:x} exceeds captured bytes 0x{len(data):x}"
        )
    blob = data[:total]
    if struct.unpack_from(">I", blob, 0)[0] != fdt.FDT_MAGIC:
        raise CompareError(f"{label}: invalid FDT magic")
    return blob


def _source_report(label: str, raw: bytes) -> tuple[dict[str, Any], dict[str, dict[str, bytes]]]:
    blob = _fdt_blob(raw, label)
    nodes = fdt.parse_fdt(blob)
    root = nodes.get("/", {})
    address_cells = fdt._cell_count(root, "#address-cells", 2)
    size_cells = fdt._cell_count(root, "#size-cells", 1)
    memory = []
    for node_path, properties in nodes.items():
        device_type = fdt._strings(properties.get("device_type"))
        if device_type == ["memory"] or node_path.rsplit("/", 1)[-1].startswith("memory@"):
            memory.extend(
                {"node": node_path, **entry}
                for entry in fdt._reg_ranges(
                    properties.get("reg"), address_cells, size_cells
                )
            )
    report = {
        "label": label,
        "component_bytes": len(raw),
        "fdt_totalsize": len(blob),
        "sha256": sha256(blob),
        "node_count": len(nodes),
        "property_count": sum(len(properties) for properties in nodes.values()),
        "model": fdt._strings(root.get("model")),
        "compatible": fdt._strings(root.get("compatible")),
        "memory_regions": memory,
    }
    return report, nodes


def compare_property_maps(
    left: dict[str, dict[str, bytes]],
    right: dict[str, dict[str, bytes]],
) -> dict[str, Any]:
    left_map = {
        (node_path, property_name): value
        for node_path, properties in left.items()
        for property_name, value in properties.items()
    }
    right_map = {
        (node_path, property_name): value
        for node_path, properties in right.items()
        for property_name, value in properties.items()
    }
    left_keys, right_keys = set(left_map), set(right_map)
    common = left_keys & right_keys
    identical = {key for key in common if left_map[key] == right_map[key]}
    changed = common - identical

    def format_key(key: tuple[str, str]) -> dict[str, Any]:
        left_value = left_map.get(key)
        right_value = right_map.get(key)
        return {
            "node": key[0],
            "property": key[1],
            "left_bytes": len(left_value) if left_value is not None else None,
            "right_bytes": len(right_value) if right_value is not None else None,
            "left_sha256": sha256(left_value) if left_value is not None else None,
            "right_sha256": sha256(right_value) if right_value is not None else None,
        }

    return {
        "left_node_count": len(left),
        "right_node_count": len(right),
        "shared_node_paths": len(set(left) & set(right)),
        "left_only_node_paths": sorted(set(left) - set(right)),
        "right_only_node_paths": sorted(set(right) - set(left)),
        "left_property_count": len(left_map),
        "right_property_count": len(right_map),
        "shared_property_keys": len(common),
        "identical_property_values": len(identical),
        "changed_property_values": len(changed),
        "left_only_properties": [format_key(key) for key in sorted(left_keys - right_keys)],
        "right_only_properties": [format_key(key) for key in sorted(right_keys - left_keys)],
        "changed_properties": [format_key(key) for key in sorted(changed)],
        "identity_ratio_over_shared_keys": (
            round(len(identical) / len(common), 6) if common else None
        ),
    }


def compare_sources(
    boot_partition: bytes, toc1_package: bytes, live_fdt: bytes
) -> dict[str, Any]:
    boot_report = android.inspect_android_boot_image(boot_partition)
    dtb_component = boot_report["components"].get("dtb")
    if not dtb_component or dtb_component["size"] == 0:
        raise CompareError("Android p4 image has no appended DTB component")
    dtb_offset = int(dtb_component["offset"], 16)
    android_dtb_raw = boot_partition[dtb_offset:dtb_offset + dtb_component["size"]]

    toc1_report = toc1.parse_toc1(toc1_package)
    toc1_dtb = next(
        (item["data"] for item in toc1_report["items"] if item["name"] == "dtb"),
        None,
    )
    if toc1_dtb is None:
        raise CompareError("TOC1 package has no dtb item")

    source_blobs = {
        "android_p4_appended_dtb": android_dtb_raw,
        "toc1_dtb": toc1_dtb,
        "live_kernel_fdt": live_fdt,
    }
    reports: dict[str, Any] = {}
    node_maps: dict[str, dict[str, dict[str, bytes]]] = {}
    for label, raw in source_blobs.items():
        reports[label], node_maps[label] = _source_report(label, raw)

    pairs = {}
    labels = list(source_blobs)
    for i, left in enumerate(labels):
        for right in labels[i + 1:]:
            pairs[f"{left}__vs__{right}"] = compare_property_maps(
                node_maps[left], node_maps[right]
            )

    return {
        "format": "RG34XX FDT provenance comparison",
        "device": "Anbernic RG34XX non-SP / Allwinner H700",
        "inputs": {
            "boot_partition_bytes": len(boot_partition),
            "boot_partition_sha256": sha256(boot_partition),
            "toc1_package_bytes": len(toc1_package),
            "toc1_package_sha256": sha256(toc1_package),
            "live_fdt_capture_bytes": len(live_fdt),
            "live_fdt_capture_sha256": sha256(live_fdt),
        },
        "sources": reports,
        "pairwise_property_comparison": pairs,
        "interpretation_limits": [
            "High property similarity supports a likely source relationship but does not prove runtime pointer provenance.",
            "A different totalsize can result from bootloader fixups, FDT open-into growth, property edits or padding.",
            "This report does not execute U-Boot or prove which source pointer was selected at runtime.",
            "Property values are not emitted; only lengths and SHA-256 hashes are included.",
        ],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("boot_partition", type=Path, help="local 64 MiB p4 boot-partition.bin")
    parser.add_argument("toc1_package", type=Path, help="local toc1-package.bin")
    parser.add_argument("live_fdt", type=Path, help="local live-fdt.dtb captured from Linux")
    parser.add_argument("-o", "--output", type=Path, required=True, help="new JSON report path")
    args = parser.parse_args()

    inputs = {args.boot_partition.resolve(), args.toc1_package.resolve(), args.live_fdt.resolve()}
    output = args.output.resolve()
    if output in inputs:
        raise CompareError("output report path must differ from all inputs")
    if output.exists():
        raise CompareError(f"refusing to overwrite existing report: {output}")

    report = compare_sources(
        args.boot_partition.read_bytes(),
        args.toc1_package.read_bytes(),
        args.live_fdt.read_bytes(),
    )
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))
    print(f"\nWrote property-level FDT comparison: {output}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, CompareError, android.BootImageError,
            toc1.Toc1Error, fdt.FdtError, struct.error) as exc:
        raise SystemExit(f"error: {exc}")
