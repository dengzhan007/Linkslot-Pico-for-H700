#!/usr/bin/env python3
"""Extract and statically inventory an A32 U-Boot image from an RG34XX TOC1.

Input may be a raw TOC1 package, inventory ZIP, or directory tree containing
toc1-package.bin. Reports offsets and optional Capstone disassembly only; it
never executes firmware or changes boot media.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import struct
import zipfile
from pathlib import Path
from typing import Any

PACKAGE_MAGIC = b"sunxi-package\0\0\0"
CHECKSUM_STAMP = 0x5F0A6C39
ITEM_TABLE_OFFSET = 64
ITEM_SIZE = 368
A32_SMC0 = struct.pack("<I", 0xE1600070)
CONSTANTS = {
    "vendor_linux_smc_0x8000ff04": struct.pack("<I", 0x8000FF04),
    "kernel_entry_0x40080000": struct.pack("<I", 0x40080000),
    "fdt_address_0x44000000": struct.pack("<I", 0x44000000),
}


class AuditError(ValueError):
    pass


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def occurrences(data: bytes, needle: bytes) -> list[int]:
    result, start = [], 0
    while True:
        pos = data.find(needle, start)
        if pos < 0:
            return result
        result.append(pos)
        start = pos + 1


def extract_toc1_items(package: bytes) -> dict[str, bytes]:
    if len(package) < ITEM_TABLE_OFFSET + ITEM_SIZE * 4:
        raise AuditError("TOC1 package is too short for four item descriptors")
    if package[:16] != PACKAGE_MAGIC:
        raise AuditError("missing Allwinner sunxi-package magic")
    item_count = struct.unpack_from("<I", package, 32)[0]
    declared_size = struct.unpack_from("<I", package, 36)[0]
    if item_count != 4:
        raise AuditError(f"expected four TOC1 items, found {item_count}")
    if declared_size != len(package):
        raise AuditError(
            f"TOC1 declared length {declared_size} differs from input length {len(package)}"
        )
    if len(package) % 4:
        raise AuditError("TOC1 package length is not word-aligned")
    if package[60:64] != b"MIE;":
        raise AuditError("TOC1 item table marker is missing")
    stored = struct.unpack_from("<I", package, 20)[0]
    words = struct.unpack(f"<{len(package) // 4}I", package)
    calculated = (sum(words) - stored + CHECKSUM_STAMP) & 0xFFFFFFFF
    if calculated != stored:
        raise AuditError("TOC1 checksum mismatch")

    items: dict[str, bytes] = {}
    intervals: list[tuple[int, int, str]] = []
    for index in range(item_count):
        at = ITEM_TABLE_OFFSET + ITEM_SIZE * index
        name = package[at:at + 64].split(b"\0", 1)[0].decode("ascii", errors="strict")
        offset, size, encryption, kind = struct.unpack_from("<4I", package, at + 64)
        if not name or name in items:
            raise AuditError(f"invalid or duplicate TOC1 item name: {name!r}")
        if encryption != 0 or kind != 3:
            raise AuditError(f"unsupported TOC1 item flags for {name}: {encryption}/{kind}")
        if offset + size > len(package):
            raise AuditError(f"TOC1 item {name} exceeds package bounds")
        if package[at + ITEM_SIZE - 4:at + ITEM_SIZE] != b"IIE;":
            raise AuditError(f"TOC1 item {name} footer is invalid")
        items[name] = package[offset:offset + size]
        intervals.append((offset, offset + size, name))
    intervals.sort()
    for previous, current in zip(intervals, intervals[1:]):
        if previous[1] > current[0]:
            raise AuditError(f"TOC1 items overlap: {previous[2]} / {current[2]}")
    if set(items) != {"u-boot", "monitor", "dtbo", "dtb"}:
        raise AuditError(f"unexpected TOC1 items: {sorted(items)}")
    return items


def find_toc1(source: Path) -> tuple[bytes, str]:
    if source.is_file():
        raw = source.read_bytes()
        if raw.startswith(PACKAGE_MAGIC):
            return raw, str(source)
        if source.suffix.lower() != ".zip":
            raise AuditError("input file is neither a TOC1 package nor an inventory ZIP")
        archives = [source]
    elif source.is_dir():
        direct = list(source.rglob("toc1-package.bin"))
        if direct:
            if len(direct) != 1:
                raise AuditError(f"found {len(direct)} toc1-package.bin files under {source}")
            return direct[0].read_bytes(), str(direct[0])
        archives = list(source.rglob("*.zip"))
    else:
        raise AuditError(f"input does not exist: {source}")

    matches: list[tuple[bytes, str]] = []
    for archive_path in archives:
        try:
            with zipfile.ZipFile(archive_path) as archive:
                names = [name for name in archive.namelist()
                         if Path(name).name == "toc1-package.bin"]
                for name in names:
                    matches.append((archive.read(name), f"{archive_path}!/{name}"))
        except zipfile.BadZipFile:
            continue
    if len(matches) != 1:
        raise AuditError(f"expected one toc1-package.bin in input; found {len(matches)}")
    return matches[0]


def disassemble_window(data: bytes, offset: int, radius: int) -> list[dict[str, Any]]:
    start = max(0, offset - radius)
    start -= start % 4
    end = min(len(data), offset + radius + 4)
    end -= (end - start) % 4
    code = data[start:end]
    try:
        from capstone import Cs, CS_ARCH_ARM, CS_MODE_ARM
    except ImportError:
        return [{"capstone": "not-installed", "raw_hex": code.hex(),
                 "window_offset": f"0x{start:x}"}]
    engine = Cs(CS_ARCH_ARM, CS_MODE_ARM)
    return [{
        "offset": f"0x{ins.address:x}",
        "bytes": ins.bytes.hex(),
        "mnemonic": ins.mnemonic,
        "operands": ins.op_str,
    } for ins in engine.disasm(code, start)]



def a32_bl_target(data: bytes, offset: int) -> int | None:
    """Decode one little-endian A32 BL immediate instruction."""
    if offset < 0 or offset + 4 > len(data) or offset % 4:
        return None
    word = struct.unpack_from("<I", data, offset)[0]
    if (word & 0x0F000000) != 0x0B000000:
        return None
    immediate = word & 0x00FFFFFF
    if immediate & 0x00800000:
        immediate -= 0x01000000
    return offset + 8 + (immediate << 2)


def a32_blx_target(data: bytes, offset: int) -> int | None:
    """Decode one little-endian A32 BLX-immediate instruction."""
    if offset < 0 or offset + 4 > len(data) or offset % 4:
        return None
    word = struct.unpack_from("<I", data, offset)[0]
    if (word & 0xFE000000) != 0xFA000000:
        return None
    immediate = ((word & 0x00FFFFFF) << 2) | ((word >> 23) & 0x2)
    if immediate & 0x02000000:
        immediate -= 0x04000000
    return offset + 8 + immediate


def find_direct_callers(data: bytes, target: int) -> list[int]:
    callers = []
    for offset in range(0, len(data) - 3, 4):
        if a32_bl_target(data, offset) == target:
            callers.append(offset)
        elif a32_blx_target(data, offset) == target:
            callers.append(offset)
    return callers



def find_a32_ldr_literal_refs(data: bytes, target: int) -> list[dict[str, Any]]:
    """Find A32 LDR (literal) instructions whose PC-relative address equals target."""
    references = []
    for offset in range(0, len(data) - 3, 4):
        word = struct.unpack_from("<I", data, offset)[0]
        if (word & 0x0F7F0000) != 0x051F0000:
            continue
        immediate = word & 0xFFF
        up = bool(word & 0x00800000)
        effective = offset + 8 + (immediate if up else -immediate)
        if effective == target:
            references.append({
                "instruction_offset": f"0x{offset:x}",
                "literal_offset": f"0x{effective:x}",
                "disassembly": disassemble_window(data, offset, 0x20),
            })
    return references


def find_word_pointer_refs(data: bytes, value: int) -> list[str]:
    return [f"0x{x:x}" for x in occurrences(data, struct.pack("<I", value))]


def find_movw_movt_smc_id(data: bytes) -> list[dict[str, str]]:
    """Find A32 MOVW/MOVT pairs that construct 0x8000ff04."""
    instructions = []
    for offset in range(0, len(data) - 3, 4):
        word = struct.unpack_from("<I", data, offset)[0]
        opcode = word & 0x0FF00000
        if opcode not in (0x03000000, 0x03400000):
            continue
        register = (word >> 12) & 0xF
        immediate = (((word >> 4) & 0xF000) | (word & 0x0FFF))
        kind = "movw" if opcode == 0x03000000 else "movt"
        instructions.append((offset, kind, register, immediate))
    matches = []
    for index, first in enumerate(instructions):
        off1, kind1, reg1, imm1 = first
        if kind1 != "movw" or imm1 != 0xFF04:
            continue
        for off2, kind2, reg2, imm2 in instructions[index + 1:index + 9]:
            if off2 - off1 > 32:
                break
            if kind2 == "movt" and reg2 == reg1 and imm2 == 0x8000:
                matches.append({
                    "movw_offset": f"0x{off1:x}",
                    "movt_offset": f"0x{off2:x}",
                    "register": f"r{reg1}",
                    "value": "0x8000ff04",
                })
    return matches


def scan_uboot(uboot: bytes, context_bytes: int = 0x50) -> dict[str, Any]:
    smc_offsets = occurrences(uboot, A32_SMC0)
    constant_offsets = {
        name: [f"0x{x:x}" for x in occurrences(uboot, needle)]
        for name, needle in CONSTANTS.items()
    }
    sites = []
    # Provisional function entries inferred from the wrapper instruction shapes.
    wrapper_entries = [0xB18, 0xB58]
    wrapper_callers = {
        f"0x{entry:x}": [
            {
                "callsite": f"0x{callsite:x}",
                "disassembly": disassemble_window(uboot, callsite, context_bytes),
            }
            for callsite in find_direct_callers(uboot, entry)
        ]
        for entry in wrapper_entries
    }
    for offset in smc_offsets:
        nearby = {}
        for name, needle in CONSTANTS.items():
            nearby[name] = [
                f"0x{x:x}" for x in occurrences(uboot, needle)
                if abs(x - offset) <= context_bytes
            ]
        sites.append({
            "file_offset": f"0x{offset:x}",
            "bytes": uboot[offset:offset + 4].hex(),
            "nearby_known_constants": nearby,
            "disassembly": disassemble_window(uboot, offset, context_bytes),
        })
    return {
        "uboot_bytes": len(uboot),
        "uboot_sha256": sha256(uboot),
        "a32_smc0_sites": sites,
        "provisional_smc_wrapper_entries": [f"0x{x:x}" for x in wrapper_entries],
        "direct_callers_of_smc_wrappers": wrapper_callers,
        "movw_movt_constructed_linux_smc_id": find_movw_movt_smc_id(uboot),
        "ldr_literal_refs_to_linux_smc_id": {
            offset: find_a32_ldr_literal_refs(uboot, offset)
            for offset in occurrences(uboot, CONSTANTS["vendor_linux_smc_0x8000ff04"])
        },
        "possible_pointer_refs_at_base_0x4a000000": {
            f"0x{entry:x}": find_word_pointer_refs(uboot, 0x4A000000 + entry)
            for entry in wrapper_entries
        },
        "constant_occurrences": constant_offsets,
        "analysis_limit": (
            "Raw A32 instruction-pattern inventory only. A match does not prove "
            "reachability or call-chain execution; absence of this byte pattern "
            "does not rule out a different SMC encoding or wrapper."
        ),
    }


def audit(source: Path, context_bytes: int = 0x50) -> dict[str, Any]:
    package, package_source = find_toc1(source)
    items = extract_toc1_items(package)
    return {
        "source": package_source,
        "toc1_bytes": len(package),
        "toc1_sha256": sha256(package),
        "components": {
            name: {"bytes": len(data), "sha256": sha256(data)}
            for name, data in items.items()
        },
        "u_boot": scan_uboot(items["u-boot"], context_bytes),
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=Path, help="raw TOC1, inventory ZIP, or directory")
    parser.add_argument("-o", "--output", type=Path, help="write JSON report to this path")
    parser.add_argument("--context-bytes", type=lambda value: int(value, 0), default=0x50)
    args = parser.parse_args()
    report = audit(args.input, args.context_bytes)
    encoded = json.dumps(report, indent=2) + "\n"
    if args.output:
        args.output.write_text(encoded, encoding="utf-8")
    else:
        print(encoded, end="")


if __name__ == "__main__":
    try:
        main()
    except (OSError, AuditError, zipfile.BadZipFile) as exc:
        raise SystemExit(f"error: {exc}")
