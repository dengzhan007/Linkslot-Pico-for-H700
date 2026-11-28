#!/usr/bin/env python3
"""Create a read-only ARM disassembly bundle for the RG34XX stock Boot0 captures."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from tools.inspect_h700_boot0 import parse_boot0  # noqa: E402


PRINTABLE = re.compile(rb"[\x20-\x7e]{5,}")


def strings_index(data: bytes) -> list[dict[str, str]]:
    found = []
    for match in PRINTABLE.finditer(data):
        value = match.group().decode("ascii", errors="replace")
        if re.search(r"AXP|PMU|DRAM|dram|bus read error|bus init error|DDR|LPDDR", value, re.I):
            found.append({"offset": f"0x{match.start():x}", "text": value})
    return found


def disassemble(binary: Path, output: Path, objdump: str) -> dict:
    data = binary.read_bytes()
    report = parse_boot0(data)
    image_size = report["image_size"]
    run_address = int.from_bytes(data[0x20:0x24], "little")
    if run_address == 0 or run_address > 0xFFFFFFFF:
        raise ValueError(f"invalid Boot0 run address 0x{run_address:x}")

    command = [
        objdump,
        "-D",
        "-b",
        "binary",
        "-m",
        "arm",
        "-M",
        "reg-names-std",
        f"--adjust-vma=0x{run_address:x}",
        str(binary),
    ]
    with output.open("w", encoding="utf-8") as stream:
        subprocess.run(command, check=True, stdout=stream, stderr=subprocess.PIPE, text=True)

    return {
        "file": binary.name,
        "captured_bytes": len(data),
        "declared_image_bytes": image_size,
        "sha256_declared_image": report["sha256_full"],
        "eGON_checksum_valid": report["checksum_valid"],
        "load_run_address": f"0x{run_address:x}",
        "objdump_command": command,
        "disassembly": output.name,
        "pmu_dram_strings": strings_index(data[:image_size]),
        "dram_ext_tables": report["dram_ext_tables"],
        "limitations": [
            "Static ARM-mode disassembly only; no code was executed.",
            "Raw-binary disassembly has no ELF symbols or relocations; code/data regions may be mixed.",
            "This output does not by itself establish control-flow reachability, runtime PMU probe order, or register writes.",
            "The image header run address is used as the disassembly VMA; this does not prove every internal section uses that address.",
        ],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--primary", type=Path, required=True)
    parser.add_argument("--secondary", type=Path, required=True)
    parser.add_argument("--outdir", type=Path, required=True)
    parser.add_argument("--objdump", default="arm-linux-gnueabihf-objdump")
    args = parser.parse_args()

    if not shutil.which(args.objdump):
        raise SystemExit(f"required disassembler not found: {args.objdump}")
    args.outdir.mkdir(parents=True, exist_ok=True)

    reports = []
    for binary in (args.primary, args.secondary):
        output = args.outdir / f"{binary.stem}.arm.disasm"
        reports.append(disassemble(binary, output, args.objdump))

    (args.outdir / "boot0-static-analysis.json").write_text(
        json.dumps(
            {
                "scope": "RG34XX non-SP stock Boot0, read-only static analysis",
                "objdump_version": subprocess.run(
                    [args.objdump, "--version"],
                    check=True,
                    capture_output=True,
                    text=True,
                ).stdout.splitlines()[0],
                "captures": reports,
            },
            indent=2,
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )
    (args.outdir / "README.txt").write_text(
        "RG34XX stock Boot0 static disassembly bundle\n"
        "The source images were read from the reports branch and validated by the eGON parser.\n"
        "Only static ARM-mode disassembly was performed. No Boot0 code was executed and no hardware was accessed.\n"
        "Raw-binary objdump has no symbols/relocations; data may be disassembled as instructions.\n"
        "Use boot0-static-analysis.json for image hashes, load address, strings and DRAM.ext extraction.\n",
        encoding="utf-8",
    )
    print(f"PASS: static disassembly written to {args.outdir}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, subprocess.CalledProcessError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        raise SystemExit(1)
