#!/usr/bin/env python3
"""Create a fail-closed host manifest joining one A32 ELF, raw image and linker map."""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
import struct
import subprocess
import tempfile
from pathlib import Path

IMAGE_BASE = 0x4A000000
HEADER_END = 0x640
VECTOR_OFFSET = 0x1000
LOADER_EXTENT = 0x8000
RUNTIME_LINK_BOUND = 0x4A0E0000
PROFILES = {
    "pi12-mmio-layout": {
        "physical_mmio_enabled": True,
        "stub_linked": False,
        "disposition": "DISPOSABLE_LAYOUT_ONLY_PI12_MMIO_ENABLED",
    },
    "stub-overlay": {
        "physical_mmio_enabled": False,
        "stub_linked": True,
        "disposition": "STUB_LINKED_OVERLAY_ONLY",
    },
    "a32-entry-stub": {
        "physical_mmio_enabled": False,
        "stub_linked": True,
        "disposition": "STUB_LINKED_ENTRY_VECTOR_ONLY",
    },
}


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def branch_target(raw: bytes, offset: int, pc: int) -> int:
    if offset + 4 > len(raw):
        raise ValueError(f"branch word at offset {offset:#x} is truncated")
    word = struct.unpack_from("<I", raw, offset)[0]
    if (word & 0x0F000000) != 0x0A000000 or (word & 0x01000000):
        raise ValueError(f"expected A32 B (not BL) at {pc:#010x}, got {word:#010x}")
    imm = word & 0x00FFFFFF
    if imm & 0x00800000:
        imm -= 1 << 24
    return pc + 8 + (imm << 2)


def parse_elf32_arm(data: bytes) -> dict:
    if len(data) < 52 or data[:4] != b"\x7fELF":
        raise ValueError("input is not an ELF file")
    if data[4] != 1 or data[5] != 1:
        raise ValueError("ELF must be 32-bit little-endian")
    fields = struct.unpack_from("<16sHHIIIIIHHHHHH", data, 0)
    e_type, e_machine, e_entry = fields[1], fields[2], fields[4]
    e_phoff, e_shoff = fields[5], fields[6]
    e_phentsize, e_phnum = fields[9], fields[10]
    e_shentsize, e_shnum, e_shstrndx = fields[11], fields[12], fields[13]
    if e_type != 2 or e_machine != 40:
        raise ValueError(f"ELF must be ET_EXEC/EM_ARM (got type={e_type}, machine={e_machine})")
    if e_phentsize != 32 or e_phoff + e_phentsize * e_phnum > len(data):
        raise ValueError("invalid ELF32 program-header table")
    loads = []
    for i in range(e_phnum):
        p_type, p_offset, p_vaddr, p_paddr, p_filesz, p_memsz, p_flags, p_align = struct.unpack_from(
            "<IIIIIIII", data, e_phoff + i * e_phentsize
        )
        if p_type != 1:
            continue
        if p_filesz > p_memsz or p_offset + p_filesz > len(data):
            raise ValueError(f"invalid PT_LOAD segment {i}")
        loads.append({
            "index": i, "offset": p_offset, "vaddr": hex(p_vaddr),
            "paddr": hex(p_paddr), "filesz": p_filesz, "memsz": p_memsz,
            "flags": p_flags, "align": p_align,
            "runtime_end": hex(p_vaddr + p_memsz),
        })
    if not loads:
        raise ValueError("ELF has no PT_LOAD segments")
    if e_shentsize != 40 or e_shoff + e_shentsize * e_shnum > len(data):
        raise ValueError("invalid ELF32 section-header table")
    sections = []
    for i in range(e_shnum):
        sh = struct.unpack_from("<IIIIIIIIII", data, e_shoff + i * e_shentsize)
        sections.append({"type": sh[1], "offset": sh[4], "size": sh[5], "link": sh[6], "entsize": sh[9]})
    symbols = {}
    for section in sections:
        if section["type"] != 2:
            continue
        if section["link"] >= len(sections) or section["entsize"] != 16:
            raise ValueError("invalid ELF32 symbol table")
        strings_section = sections[section["link"]]
        strings = data[strings_section["offset"]:strings_section["offset"] + strings_section["size"]]
        start, end, entsize = section["offset"], section["offset"] + section["size"], section["entsize"]
        for pos in range(start, end, entsize):
            if pos + entsize > len(data):
                raise ValueError("truncated ELF32 symbol")
            name_off, value, size, info, other, shndx = struct.unpack_from("<IIIBBH", data, pos)
            if name_off >= len(strings):
                continue
            stop = strings.find(b"\0", name_off)
            if stop < 0:
                continue
            name = strings[name_off:stop].decode("utf-8", "replace")
            if name and name not in symbols:
                symbols[name] = {"value": value, "size": size, "section": shndx}
    return {"entry": e_entry, "loads": loads, "symbols": symbols}


def audit(elf_path: Path, raw_path: Path, map_path: Path, profile: str, objcopy: str) -> dict:
    elf = elf_path.read_bytes()
    raw = raw_path.read_bytes()
    map_data = map_path.read_bytes()
    parsed = parse_elf32_arm(elf)
    symbols = parsed["symbols"]
    required = ("_start", "_end")
    missing = [name for name in required if name not in symbols]
    if missing:
        raise ValueError("ELF missing required symbols: " + ", ".join(missing))
    start = symbols["_start"]["value"]
    end = symbols["_end"]["value"]
    if start != IMAGE_BASE or parsed["entry"] != IMAGE_BASE:
        raise ValueError(f"ELF entry/_start must equal {IMAGE_BASE:#x}")
    if end <= start or end > RUNTIME_LINK_BOUND:
        raise ValueError(f"runtime _end outside researched link bound: {end:#x}")
    if len(raw) > LOADER_EXTENT:
        raise ValueError(f"raw image exceeds active 32 KiB loader extent: {len(raw)}")
    if len(raw) <= VECTOR_OFFSET + 31:
        raise ValueError("raw image does not contain all eight fixed exception vectors")
    if any(raw[4:HEADER_END]):
        raise ValueError("raw image vendor-header reservation [+4,+0x640) is not zero")
    entry_target = branch_target(raw, 0, IMAGE_BASE)
    if not IMAGE_BASE + HEADER_END <= entry_target < IMAGE_BASE + len(raw):
        raise ValueError(f"entry branch target outside raw image: {entry_target:#x}")
    vectors = []
    for index in range(8):
        off = VECTOR_OFFSET + index * 4
        target = branch_target(raw, off, IMAGE_BASE + off)
        if not IMAGE_BASE + HEADER_END <= target < IMAGE_BASE + len(raw):
            raise ValueError(f"vector {index} target outside raw image: {target:#x}")
        vectors.append(hex(target))
    if b"_end" not in map_data:
        raise ValueError("linker map does not mention _end")
    if not any(re.search(rb"0x0*%x\s+_end" % end, line) for line in map_data.splitlines()):
        raise ValueError("linker map _end address does not match ELF symbol table")
    with tempfile.TemporaryDirectory(prefix="m1-a32-audit-") as tmp:
        rebuilt = Path(tmp) / "from-elf.bin"
        subprocess.run([objcopy, "-O", "binary", str(elf_path), str(rebuilt)], check=True, capture_output=True)
        rebuilt_data = rebuilt.read_bytes()
    if rebuilt_data != raw:
        raise ValueError("raw binary is not byte-identical to objcopy output from the supplied ELF")
    selected = PROFILES[profile]
    intervals = {}
    for name in ("abort_stack", "abort_stack_top", "tt", "h700_bl33_entry_snapshot"):
        if name in symbols:
            intervals[name] = hex(symbols[name]["value"])
    return {
        "schema": "h700-m1-a32-artifact-accounting-v1",
        "profile": profile,
        "profile_disposition": selected["disposition"],
        "host_only": True,
        "device_io_performed": False,
        "payload_executed": False,
        "not_installable": True,
        "hardware_validated": False,
        "candidate_eligible": False,
        "candidate_blockers": [
            "No BL31-to-BL33 physical memory ownership approval (G2)",
            "No CPU1-CPU3 reset/RVBAR handoff readback (G3)",
            "No approved proof-of-life signal / M1-S1 gate",
            "No matched M1 one-shot Boot0/SPL/firmware capsule generation",
        ],
        "physical_mmio_enabled_in_profile": selected["physical_mmio_enabled"],
        "stub_linked_profile": selected["stub_linked"],
        "files": {
            "elf": {"name": elf_path.name, "bytes": len(elf), "sha256": sha256(elf)},
            "raw": {"name": raw_path.name, "bytes": len(raw), "sha256": sha256(raw)},
            "linker_map": {"name": map_path.name, "bytes": len(map_data), "sha256": sha256(map_data)},
        },
        "elf": {
            "class": "ELF32", "endianness": "little", "machine": "ARM",
            "entry": hex(parsed["entry"]), "start": hex(start), "_end": hex(end),
            "runtime_span_bytes": end - start,
            "runtime_link_bound": hex(RUNTIME_LINK_BOUND),
            "pt_load_segments": parsed["loads"],
            "symbols": intervals,
        },
        "raw": {
            "active_loader_extent_bytes": LOADER_EXTENT,
            "headroom_bytes": LOADER_EXTENT - len(raw),
            "entry_branch_target": hex(entry_target),
            "vector_branch_targets": vectors,
            "reserved_vendor_header_range": ["0x4", hex(HEADER_END)],
            "reserved_vendor_header_is_zero": True,
            "byte_identical_to_elf_objcopy": True,
        },
        "limits": [
            "A passing raw-size check is not runtime memory ownership approval.",
            "The runtime link bound is a source-derived layout bound, not a DRAM allocation grant.",
            "The manifest does not validate the vendor header, firmware generation, SPL FNV, capsule recovery or device behavior.",
            "The profile label describes the CI build path; it is not an independent proof of MMIO behavior.",
        ],
    }


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--elf", type=Path, required=True)
    ap.add_argument("--raw", type=Path, required=True)
    ap.add_argument("--map", type=Path, required=True)
    ap.add_argument("--profile", choices=sorted(PROFILES), required=True)
    ap.add_argument("--objcopy", default="llvm-objcopy")
    ap.add_argument("--output", type=Path, required=True)
    args = ap.parse_args()
    try:
        result = audit(args.elf, args.raw, args.map, args.profile, args.objcopy)
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
        print(json.dumps(result, indent=2))
        return 0
    except (OSError, ValueError, subprocess.CalledProcessError) as exc:
        ap.exit(1, f"FAIL: {exc}\n")


if __name__ == "__main__":
    raise SystemExit(main())
