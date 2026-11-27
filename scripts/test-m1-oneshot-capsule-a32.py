#!/usr/bin/env python3
"""Build an AArch32 ARM-state ELF fixture for the host-only M1 selector.

This verifies target-ISA compile/link compatibility only. It does not build or
modify a Boot0 capsule/SPL, access hardware, copy images, or authorize a test.
"""
from __future__ import annotations

import argparse
import shutil
import subprocess
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SELECTOR = ROOT / "platform/h700/boot/m1_oneshot_selection_model.c"
FIXTURE = ROOT / "tests/m1_oneshot_capsule_a32_fixture.c"
INCLUDE = ROOT / "platform/h700/boot"


def resolve_tool(name: str) -> str:
    path = shutil.which(name)
    if path is None:
        raise RuntimeError(f"required tool not found on PATH: {name}")
    return path


def run(command: list[str]) -> str:
    result = subprocess.run(
        command,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    if result.returncode != 0:
        raise RuntimeError(
            f"command failed ({result.returncode}): {' '.join(command)}\n"
            f"{result.stdout}"
        )
    return result.stdout


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--clang", default="clang")
    parser.add_argument("--readelf", default="readelf")
    parser.add_argument("--nm", default="nm")
    args = parser.parse_args()

    for path in (SELECTOR, FIXTURE):
        if not path.is_file():
            raise RuntimeError(f"required source file is missing: {path}")

    clang = resolve_tool(args.clang)
    readelf = resolve_tool(args.readelf)
    nm = resolve_tool(args.nm)

    compile_flags = [
        clang,
        "--target=arm-none-eabi",
        "-mcpu=cortex-a53",
        "-marm",
        "-mfloat-abi=soft",
        "-std=c11",
        "-O2",
        "-Wall",
        "-Wextra",
        "-Werror",
        "-Wconversion",
        "-Wsign-conversion",
        "-ffreestanding",
        "-fno-builtin",
        "-fno-stack-protector",
        "-fno-unwind-tables",
        "-fno-asynchronous-unwind-tables",
        "-ffunction-sections",
        "-fdata-sections",
        "-I",
        str(INCLUDE),
    ]

    with tempfile.TemporaryDirectory(prefix="h700-m1-a32-selector-") as tmp:
        out = Path(tmp)
        selector_obj = out / "selector.o"
        fixture_obj = out / "fixture.o"
        elf = out / "m1-selector-fixture.elf"

        run(compile_flags + ["-c", str(SELECTOR), "-o", str(selector_obj)])
        run(compile_flags + ["-c", str(FIXTURE), "-o", str(fixture_obj)])

        run([
            clang,
            "--target=arm-none-eabi",
            "-mcpu=cortex-a53",
            "-marm",
            "-nostdlib",
            "-fuse-ld=lld",
            "-Wl,--no-undefined",
            "-Wl,--gc-sections",
            "-Wl,-e,h700_m1_capsule_dispatch_fixture",
            "-Wl,-Ttext=0x20400",
            str(selector_obj),
            str(fixture_obj),
            "-o",
            str(elf),
        ])

        header = run([readelf, "-h", str(elf)])
        if "ELF32" not in header or "Machine:" not in header or "ARM" not in header:
            raise RuntimeError("linked fixture is not an ELF32 ARM image")

        undefined = run([nm, "-u", str(elf)])
        if undefined.strip():
            raise RuntimeError("AArch32 selector fixture has undefined symbols:\n" + undefined)

        symbols = run([nm, str(elf)])
        required = (
            "h700_m1_capsule_select",
            "h700_m1_capsule_fnv1a",
            "h700_m1_capsule_dispatch_fixture",
        )
        missing = [symbol for symbol in required if symbol not in symbols]
        if missing:
            raise RuntimeError("linked selector symbols are missing: " + ", ".join(missing))

    print("PASS M1 selector compiles and links as a freestanding AArch32 ARM-state ELF32")
    print("PASS selector fixture has no undefined runtime symbols")
    print("LIMIT: not linked into Boot0 capsule or SPL; no copy, MMIO, RTC, WDT, or block I/O")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError, subprocess.SubprocessError) as error:
        raise SystemExit(f"FAIL: {error}")
