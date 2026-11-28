#!/usr/bin/env python3
"""Create read-only ARM disassembly and component metadata for RG34XX boot captures."""

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


def disassemble_bytes(data: bytes, output: Path, objdump: str, vma: int) -> list[str]:
    with tempfile.TemporaryDirectory(prefix="rg34xx-boot-disasm-") as temporary:
        image = Path(temporary) / "component.bin"
        image.write_bytes(data)
        command = [
            objdump,
            "-D",
            "-b",
            "binary",
            "-m",
            "arm",
            "-M",
            "reg-names-std",
            f"--adjust-vma=0x{vma:x}",
            str(image),
        ]
        with output.open("w", encoding="utf-8") as stream:
            subprocess.run(command, check=True, stdout=stream, stderr=subprocess.PIPE, text=True)
    return command


def analyze_capture(binary: Path, outdir: Path, objdump: str) -> tuple[dict, bytes]:
    data = binary.read_bytes()
    report = parse_boot0(data)
    declared = data[: report["image_size"]]
    run_address = struct.unpack_from("<I", declared, 0x20)[0]
    output = outdir / f"{binary.stem}-outer.arm.disasm"
    command = disassemble_bytes(declared, output, objdump, run_address)

    report["source_file"] = binary.name
    report["source_capture_sha256"] = sha256(data)
    report["outer_disassembly"] = output.name
    report["outer_objdump_command"] = command
    report["pmu_dram_board_strings"] = strings_index(declared)

    if binary.name == "boot0-primary.bin":
        for component in report["embedded_egon_components"]:
            offset = int(component["offset"], 16)
            size = component["declared_size"]
            component_bytes = declared[offset : offset + size]
            if component["component_kind"] == "eGON Boot0" and offset != 0:
                output = outdir / "boot0-primary-inner.arm.disasm"
                component["disassembly"] = output.name
                component["disassembly_vma"] = "0x20000"
                component["objdump_command"] = disassemble_bytes(
                    component_bytes, output, objdump, 0x20000
                )
            elif component["component_kind"] == "SPL2 component":
                output = outdir / "boot0-primary-spl2-rg35xx-2024.arm.disasm"
                component["disassembly"] = output.name
                component["disassembly_vma_candidate"] = "0x50000"
                component["objdump_command"] = disassemble_bytes(
                    component_bytes, output, objdump, 0x50000
                )
                component["pmu_dram_board_strings"] = strings_index(component_bytes, offset)

    return report, data


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--primary", type=Path, required=True)
    parser.add_argument("--secondary", type=Path, required=True)
    parser.add_argument("--toc1", type=Path, required=True)
    parser.add_argument("--fdt", type=Path, required=True)
    parser.add_argument("--outdir", type=Path, required=True)
    parser.add_argument("--objdump", default="arm-linux-gnueabihf-objdump")
    args = parser.parse_args()

    if not shutil.which(args.objdump):
        raise SystemExit(f"required disassembler not found: {args.objdump}")
    args.outdir.mkdir(parents=True, exist_ok=True)

    primary, primary_bytes = analyze_capture(args.primary, args.outdir, args.objdump)
    secondary, secondary_bytes = analyze_capture(args.secondary, args.outdir, args.objdump)

    # The outer Boot0 stub's literal at file offset 0x494 points to 0x50000.
    # The embedded SPL2 component begins at +0x11000 and its first ARM branch
    # targets component offset 0x60. This is an address correlation, not a
    # proof of the complete runtime copy/control-flow path.
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
                "assessment": "The address relationship is consistent with the Boot0 stub handing off to the embedded SPL2 component; exact copy/selection/reachability still requires control-flow confirmation.",
            }

    toc1_data = args.toc1.read_bytes()
    fdt_data = args.fdt.read_bytes()
    toc1_fdt = {
        "toc1": {
            "file": args.toc1.name,
            "bytes": len(toc1_data),
            "sha256": sha256(toc1_data),
            "relevant_strings": strings_index(toc1_data),
        },
        "live_fdt": {
            "file": args.fdt.name,
            "bytes": len(fdt_data),
            "sha256": sha256(fdt_data),
            "relevant_strings": strings_index(fdt_data),
        },
        "cross_component_identity_note": (
            "A RG35XX-2024 SPL2 build label alongside an RG34XX-v1 live FDT is a "
            "cross-component naming discrepancy. It does not by itself prove "
            "incompatibility; verify the SPL's hardware assumptions and TOC1 payload."
        ),
    }

    (args.outdir / "boot0-static-analysis.json").write_text(
        json.dumps(
            {
                "scope": "RG34XX non-SP TF-card boot chain, read-only static analysis",
                "objdump_version": subprocess.run(
                    [args.objdump, "--version"],
                    check=True,
                    capture_output=True,
                    text=True,
                ).stdout.splitlines()[0],
                "captures": [primary, secondary],
                "candidate_handoff": handoff,
                "limitations": [
                    "Static ARM-mode disassembly only; no code was executed.",
                    "Raw-binary disassembly has no ELF symbols or relocations; code/data regions may be mixed.",
                    "The outer header run address is used for the outer image; the SPL2 VMA 0x50000 is a candidate inferred from a literal and branch target.",
                    "The SPL2 platform string is a build label, not proof of board incompatibility.",
                    "This output does not by itself establish PMU probe order, register writes, runtime profile selection, or hardware qualification.",
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
        "The primary capture contains nested eGON Boot0 and SPL2 components; the SPL2 component is disassembled at candidate VMA 0x50000.\n"
        "The embedded SPL2 header advertises an RG35XX-2024 build path, while the live FDT contains an RG34XX-v1 marker.\n"
        "Only static parsing, string extraction, hashing and ARM-mode disassembly were performed. No firmware was executed and no hardware was accessed.\n"
        "See JSON reports for hashes, nested headers, profile tables, platform strings and caveats.\n",
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
