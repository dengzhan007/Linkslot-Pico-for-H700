#!/usr/bin/env python3
"""Verify the matched monitor's 0x18 flag and pinned native-SPL preservation."""
from __future__ import annotations
import argparse
import hashlib
import json
import re
import struct
from pathlib import Path

EXPECTED_BYTES = 107216
EXPECTED_SHA256 = "676bc0206e3c9a7f229d93ea7f3286095f669940ad91f3c4f3d37bdb18696c31"
EXPECTED_BASEOS_COMMIT = "cbe253a43f46cf551cc9a6c79a84015bde3686cf"
FLAG_OFFSET = 0x18

def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reports-root", type=Path, required=True)
    parser.add_argument("--baseos-root", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    reports = args.reports_root.resolve()
    baseos = args.baseos_root.resolve()
    monitor_path = reports / "rg34xx-inventory-20261126-timer/toc1-extracted/items/monitor.bin"
    firmware_manifest = json.loads((reports / "rg34xx-native-firmware/firmware-manifest.json").read_text(encoding="utf-8"))
    toc_manifest = json.loads((reports / "rg34xx-inventory-20261126-timer/toc1-extracted/manifest.json").read_text(encoding="utf-8"))
    raw = monitor_path.read_bytes()
    digest = hashlib.sha256(raw).hexdigest()
    toc_item = next(item for item in toc_manifest["items"] if item["name"] == "monitor")
    firmware_item = firmware_manifest["extents"]["monitor"]
    if len(raw) != EXPECTED_BYTES or digest != EXPECTED_SHA256:
        raise RuntimeError("monitor payload does not match pinned identity")
    for item in (toc_item, firmware_item):
        size = item.get("payload_bytes", item.get("size"))
        if item.get("sha256") != EXPECTED_SHA256 or size != EXPECTED_BYTES:
            raise RuntimeError("source manifest no longer matches the pinned monitor identity")
    flag = struct.unpack_from("<I", raw, FLAG_OFFSET)[0]
    if flag != 0:
        raise RuntimeError("monitor header word +0x18 changed; conditional BL31 mapping would be enabled")

    spl_path = baseos / "experiments/h700-boot-speed/spl/native-boot.c"
    spl = spl_path.read_text(encoding="utf-8")
    required = (
        "u8 *uboot = (void *)0x4a000000, *monitor = (void *)0x48000000;",
        "memcpy((void *)(ulong)item->destination, package + item->offset, item->bytes);",
        "fnv(package, BASEOS_FIRMWARE_BYTES) != BASEOS_FIRMWARE_FNV",
        "memcpy(monitor + 0x30, parameters, 128);",
        "memcpy(monitor + 0x24, board, 8);",
        "*(u32 *)(monitor + 0xb0) = 0x4a200000;",
    )
    for fragment in required:
        if fragment not in spl:
            raise RuntimeError("pinned native SPL source contract changed: " + fragment)
    for pattern in (r"monitor\s*\+\s*0x18\b", r"monitor\s*\[\s*0x18\s*\]"):
        if re.search(pattern, spl, flags=re.IGNORECASE):
            raise RuntimeError("native SPL contains a direct alias to monitor flag +0x18")

    copy_at = spl.index("memcpy((void *)(ulong)item->destination, package + item->offset, item->bytes);")
    monitor_at = spl.index("u8 *uboot = (void *)0x4a000000, *monitor = (void *)0x48000000;")
    if copy_at >= monitor_at:
        raise RuntimeError("firmware extent copy must precede explicit monitor header patches")
    rvbar_line = "ulong rvbar = (readl((void *)0x03000024) & 7) == 2 ? 0x08100040 : 0x09010040;"
    if rvbar_line not in spl or "writel(0x48000000, (void *)rvbar);" not in spl:
        raise RuntimeError("native SPL primary RVBAR setup changed")
    if "writel(0, (void *)(rvbar + 4));" not in spl:
        raise RuntimeError("native SPL primary RVBAR high word is no longer cleared")
    if re.search(r"rvbar\s*\+\s*(?:0x)?(?:8|0xc|0x10|0x14|0x18|0x1c)\b", spl, re.IGNORECASE):
        raise RuntimeError("native SPL handoff now references additional per-core RVBAR registers")

    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    report = {
        "schema": "rg34xx-bl31-startup-contract-v1",
        "monitor": {
            "bytes": len(raw), "sha256": digest, "load_base": "0x48000000",
            "flag_offset": "0x18", "flag_runtime_address": "0x48000018",
            "flag_value": "0x%08x" % flag,
            "conditional_30mib_mapping_enabled": False,
        },
        "native_spl": {
            "source_commit": EXPECTED_BASEOS_COMMIT,
            "monitor_load_base": "0x48000000",
            "explicit_monitor_patch_offsets": ["0x24..0x2b", "0x30..0xaf", "0xb0..0xb3"],
            "writes_to_flag_offset_0x18_found": False,
            "extent_copy_precedes_header_mutations": True,
            "firmware_package_fnv_check_before_extent_copy": True,
            "primary_rvbar_register_pair": "0x09010040/0x09010044 for H616",
            "other_per_core_rvbar_registers_written_in_handoff_function": False,
            "secondary_core_initial_reset_state": "UNKNOWN; not established by this source inspection",
        },
        "conclusion": [
            "The exact matched monitor payload has a zero word at offset +0x18.",
            "The pinned SPL copies the matched firmware extent, then patches +0x24, +0x30 and +0xb0 only.",
            "The 30 MiB conditional BL31 map call path is skipped for this image if no unreviewed earlier stage mutates +0x18.",
            "The SPL handoff programs the primary RVBAR pair; it does not prove secondary-core initial reset state.",
            "This is a host-only binary/source audit, not a live hardware state proof.",
        ],
    }
    (out / "startup-contract.json").write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    markdown = [
        "# RG34XX BL31 startup contract check", "",
        "- Monitor payload: %d bytes; SHA-256 %s." % (len(raw), digest),
        "- Header word at file offset 0x18 / runtime address 0x48000018: 0x%08x." % flag,
        "- Conditional BL31 30 MiB mapping path: SKIPPED for this exact image.",
        "- Pinned native SPL explicit monitor patches: +0x24..+0x2b, +0x30..+0xaf, +0xb0..+0xb3.",
        "- SPL writes to monitor +0x18: NONE FOUND in the pinned handoff source.",
        "- Firmware extent copy precedes explicit patches and is preceded by package FNV validation.",
        "- H616 native handoff programs the primary RVBAR low/high pair at 0x09010040 / 0x09010044.",
        "- This handoff function does not establish secondary-core initial reset state: UNKNOWN.",
        "",
        "## Interpretation", "",
        "- The inactive [0x48200000,0x4a000000) identity-map request should not be counted as an active mapping for this exact monitor/SPL pair.",
        "- Keep the unconditional BL31-associated range [0x48000000,0x48041000) excluded.",
        "- This source audit does not prove that Boot0/ROM or another unreviewed stage cannot mutate the monitor header before SPL handoff.",
        "- No hardware MMIO, Boot0/TOC1/SD write, flash or payload execution was performed.", "",
    ]
    (out / "STARTUP-CONTRACT.md").write_text("\n".join(markdown) + "\n", encoding="utf-8")
    print("\n".join(markdown))
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
