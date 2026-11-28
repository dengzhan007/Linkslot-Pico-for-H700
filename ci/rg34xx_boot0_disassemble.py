#!/usr/bin/env python3
"""Create read-only ARM/AArch64 disassembly and component metadata for RG34XX boot captures."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
import struct
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from tools.inspect_h700_boot0 import parse_boot0  # noqa: E402


PRINTABLE = re.compile(rb"[\x20-\x7e]{5,}")
RELEVANT = re.compile(
    r"AXP|PMU|DRAM|DDR|LPDDR|bus read error|bus init error|"
    r"rg34|rg35|sun50i|h700|anbernic|allwinner|SPL|BOOT0|TOC1|"
    r"BL31|BL33|U-Boot|x-powers",
    re.I,
)


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def strings_index(data: bytes, base_offset: int = 0) -> list[dict[str, str]]:
    found = []
    for match in PRINTABLE.finditer(data):
        value = match.group().decode("ascii", errors="replace")
        if RELEVANT.search(value):
            found.append(
                {
                    "offset": f"0x{match.start():x}",
                    "absolute_offset": f"0x{base_offset + match.start():x}",
                    "text": value,
                }
            )
    return found


def disassemble_bytes(
    data: bytes, output: Path, objdump: str, vma: int, arch: str = "arm"
) -> list[str]:
    with tempfile.TemporaryDirectory(prefix="rg34xx-boot-disasm-") as temporary:
        image = Path(temporary) / "component.bin"
        image.write_bytes(data)
        command = [objdump, "-D", "-b", "binary", "-m", arch]
        if arch == "arm":
            command.extend(["-M", "reg-names-std"])
        command.extend([f"--adjust-vma=0x{vma:x}", str(image)])
        with output.open("w", encoding="utf-8") as stream:
            subprocess.run(command, check=True, stdout=stream, stderr=subprocess.PIPE, text=True)
    return command


def analyze_capture(binary: Path, outdir: Path, objdump: str, aarch64_objdump: str) -> tuple[dict, bytes]:
    data = binary.read_bytes()
    report = parse_boot0(data)
    declared = data[: report["image_size"]]
    run_address = struct.unpack_from("<I", declared, 0x20)[0]
    output = outdir / f"{binary.stem}-outer.arm.disasm"
    command = disassemble_bytes(declared, output, objdump, run_address, "arm")
    aarch64_outer_output = outdir / f"{binary.stem}-outer.aarch64.disasm"
    aarch64_outer_command = disassemble_bytes(
        declared, aarch64_outer_output, aarch64_objdump, run_address, "aarch64"
    )

    report["source_file"] = binary.name
    report["source_capture_sha256"] = sha256(data)
    report["outer_disassembly"] = output.name
    report["outer_objdump_command"] = command
    report["outer_aarch64_disassembly"] = aarch64_outer_output.name
    report["outer_aarch64_objdump_command"] = aarch64_outer_command
    report["pmu_dram_board_strings"] = strings_index(declared)

    for component in report["embedded_egon_components"]:
        offset = int(component["offset"], 16)
        size = component["declared_size"]
        component_bytes = declared[offset : offset + size]
        if component["component_kind"] == "eGON Boot0":
            if offset == 0:
                continue  # The outer/standalone image was already disassembled above.
            stem = "boot0-primary-inner"
            arm_output = outdir / f"{stem}.arm.disasm"
            a64_output = outdir / f"{stem}.aarch64.disasm"
            component["arm_disassembly"] = arm_output.name
            component["aarch64_disassembly"] = a64_output.name
            component["disassembly_vma"] = "0x20000"
            component["arm_objdump_command"] = disassemble_bytes(
                component_bytes, arm_output, objdump, 0x20000, "arm"
            )
            component["aarch64_objdump_command"] = disassemble_bytes(
                component_bytes, a64_output, aarch64_objdump, 0x20000, "aarch64"
            )
        elif component["component_kind"] == "SPL2 component":
                output = outdir / "boot0-primary-spl2-rg35xx-2024.arm.disasm"
                component["disassembly"] = output.name
                component["disassembly_vma_candidate"] = "0x50000"
                component["objdump_command"] = disassemble_bytes(
                    component_bytes, output, objdump, 0x50000, "arm"
                )
                aarch64_start = 0x188
                aarch64_output = outdir / "boot0-primary-spl2-rg35xx-2024.aarch64.disasm"
                component["aarch64_disassembly"] = aarch64_output.name
                component["aarch64_candidate_start_offset"] = f"0x{aarch64_start:x}"
                component["aarch64_candidate_vma"] = f"0x{0x50000 + aarch64_start:x}"
                component["aarch64_objdump_command"] = disassemble_bytes(
                    component_bytes[aarch64_start:],
                    aarch64_output,
                    aarch64_objdump,
                    0x50000 + aarch64_start,
                    "aarch64",
                )
                component["mode_note"] = (
                    "The SPL2 component has an ARM-mode entry/header stub and AArch64-looking "
                    "instruction encodings from candidate offset 0x188. The AArch64 region is "
                    "disassembled separately; the exact exception-level transition path is not proven."
                )
                component["pmu_dram_board_strings"] = strings_index(component_bytes, offset)

    return report, data


def scan_fdt_blobs(data: bytes) -> list[dict]:
    found = []
    magic = b"\xd0\x0d\xfe\xed"
    cursor = 0
    while True:
        offset = data.find(magic, cursor)
        if offset < 0:
            break
        cursor = offset + 1
        if offset + 40 > len(data):
            continue
        total, off_struct, off_strings, off_mem_rsv, version, last_comp = struct.unpack_from(
            ">6I", data, offset + 4
        )
        if (
            total < 40
            or offset + total > len(data)
            or off_struct >= total
            or off_strings >= total
            or off_mem_rsv >= total
            or version < 16
            or version > 20
            or last_comp > version
        ):
            continue
        blob = data[offset : offset + total]
        relevant = strings_index(blob, offset)
        board_strings = [
            item for item in relevant
            if re.search(r"rg34|rg35|anbernic|sun50i-h700|x-powers,axp2202", item["text"], re.I)
        ]
        if board_strings:
            found.append(
                {
                    "offset": f"0x{offset:x}",
                    "total_size": total,
                    "version": version,
                    "last_comp_version": last_comp,
                    "sha256": sha256(blob),
                    "board_strings": board_strings,
                }
            )
    return found


def marker_offsets(data: bytes) -> dict[str, list[str]]:
    markers = {
        "TOC1": b"TOC1",
        "TOC0": b"TOC0",
        "eGON.BT1": b"eGON.BT1",
        "eGON.BT0": b"eGON.BT0",
        "BL31": b"BL31",
        "BL33": b"BL33",
        "U-Boot": b"U-Boot",
        "FDT_magic": b"\xd0\x0d\xfe\xed",
    }
    result = {}
    for name, marker in markers.items():
        offsets = []
        cursor = 0
        while True:
            offset = data.find(marker, cursor)
            if offset < 0:
                break
            offsets.append(f"0x{offset:x}")
            cursor = offset + 1
        result[name] = offsets[:100]
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--primary", type=Path, required=True)
    parser.add_argument("--secondary", type=Path, required=True)
    parser.add_argument("--toc1", type=Path, required=True)
    parser.add_argument("--fdt", type=Path, required=True)
    parser.add_argument("--outdir", type=Path, required=True)
    parser.add_argument("--objdump", default="arm-linux-gnueabihf-objdump")
    parser.add_argument("--aarch64-objdump", default="aarch64-linux-gnu-objdump")
    args = parser.parse_args()

    if not shutil.which(args.objdump):
        raise SystemExit(f"required ARM disassembler not found: {args.objdump}")
    if not shutil.which(args.aarch64_objdump):
        raise SystemExit(f"required AArch64 disassembler not found: {args.aarch64_objdump}")
    args.outdir.mkdir(parents=True, exist_ok=True)

    primary, primary_bytes = analyze_capture(args.primary, args.outdir, args.objdump, args.aarch64_objdump)
    secondary, _ = analyze_capture(args.secondary, args.outdir, args.objdump, args.aarch64_objdump)

    handoff = {}
    if len(primary_bytes) >= 0x498:
        literal = struct.unpack_from("<I", primary_bytes, 0x494)[0]
        spl = next(
            (x for x in primary["embedded_egon_components"] if x["component_kind"] == "SPL2 component"),
            None,
        )
        if spl:
            spl_offset = int(spl["offset"], 16)
            first_word = struct.unpack_from("<I", primary_bytes, spl_offset)[0]
            branch_imm = first_word & 0x00FFFFFF
            if branch_imm & 0x00800000:
                branch_imm -= 0x01000000
            branch_target = 8 + (branch_imm << 2)
            handoff = {
                "outer_stub_literal_file_offset": "0x494",
                "outer_stub_literal_value": f"0x{literal:x}",
                "spl2_component_file_offset": spl["offset"],
                "spl2_component_declared_size": spl["declared_size"],
                "spl2_first_word": f"0x{first_word:08x}",
                "spl2_branch_target_offset": f"0x{branch_target:x}",
                "candidate_spl2_load_vma": "0x50000",
                "candidate_spl2_code_entry": f"0x{literal + branch_target:x}",
                "assessment": "Address relationship is consistent with Boot0 handing off to SPL2; exact copy/selection/reachability remains unproven.",
            }

    toc1_data = args.toc1.read_bytes()
    fdt_data = args.fdt.read_bytes()
    toc1_fdt = {
        "toc1": {
            "file": args.toc1.name,
            "bytes": len(toc1_data),
            "sha256": sha256(toc1_data),
            "header_preview_hex": toc1_data[:256].hex(),
            "relevant_strings": strings_index(toc1_data),
            "marker_offsets": marker_offsets(toc1_data),
            "embedded_fdt_blobs": scan_fdt_blobs(toc1_data),
        },
        "live_fdt": {
            "file": args.fdt.name,
            "bytes": len(fdt_data),
            "sha256": sha256(fdt_data),
            "relevant_strings": strings_index(fdt_data),
            "embedded_fdt_blobs": scan_fdt_blobs(fdt_data),
        },
        "cross_component_identity_note": (
            "An RG35XX-2024 SPL2 build label alongside RG34XX-v1 FDT data is a cross-component "
            "naming discrepancy. TOC1 board strings and SPL hardware assumptions must be checked "
            "before concluding that the SPL is shared or incompatible."
        ),
    }

    (args.outdir / "boot0-static-analysis.json").write_text(
        json.dumps(
            {
                "scope": "RG34XX non-SP TF-card boot chain, read-only static analysis",
                "arm_objdump_version": subprocess.run(
                    [args.objdump, "--version"], check=True, capture_output=True, text=True
                ).stdout.splitlines()[0],
                "aarch64_objdump_version": subprocess.run(
                    [args.aarch64_objdump, "--version"], check=True, capture_output=True, text=True
                ).stdout.splitlines()[0],
                "captures": [primary, secondary],
                "candidate_handoff": handoff,
                "limitations": [
                    "Static ARM/AArch64 disassembly only; no code was executed.",
                    "Raw-binary disassembly has no ELF symbols or relocations; code/data regions may be mixed.",
                    "The SPL2 component appears mixed-mode; the AArch64 start offset 0x188 is a candidate, not a proven exception-level transition.",
                    "The SPL2 VMA 0x50000 is inferred from an outer Boot0 literal and the SPL2 branch target.",
                    "The SPL2 platform string is a build label, not proof of board incompatibility.",
                    "TOC1 FDT/string scanning does not establish which FDT or image component is selected at runtime.",
                    "This output does not by itself establish PMU register writes, DRAM profile selection, or hardware qualification.",
                ],
            },
            indent=2,
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )
    (args.outdir / "toc1-fdt-string-index.json").write_text(
        json.dumps(toc1_fdt, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    (args.outdir / "README.txt").write_text(
        "RG34XX H700 stock boot-chain static analysis bundle\n"
        "Inputs: reports-branch Boot0 primary/secondary captures, TOC1 package, and live FDT.\n"
        "The primary capture contains nested eGON Boot0 and SPL2 components.\n"
        "Boot0 components are disassembled in ARM and AArch64 modes; SPL2 is disassembled in ARM mode and its candidate AArch64 region separately.\n"
        "TOC1 and live FDT are scanned for embedded FDT blobs and board/PMIC/BL31/U-Boot strings.\n"
        "Only static parsing, string extraction, hashing and disassembly were performed. No firmware was executed and no hardware was accessed.\n",
        encoding="utf-8",
    )
    print(f"PASS: Boot0/TOC1/FDT static analysis written to {args.outdir}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, subprocess.CalledProcessError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        raise SystemExit(1)
