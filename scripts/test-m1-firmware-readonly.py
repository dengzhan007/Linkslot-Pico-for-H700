#!/usr/bin/env python3
"""Run host and freestanding AArch32 checks for the M1 read-only package preflight."""
from __future__ import annotations

import argparse
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "platform/h700/boot/m1_firmware_readonly.c"
TEST = ROOT / "tests/test_m1_firmware_readonly.c"
INCLUDE = ROOT / "platform/h700/boot"


def run(command: list[str]) -> str:
    result = subprocess.run(command, stdout=subprocess.PIPE,
                            stderr=subprocess.STDOUT, text=True)
    if result.returncode:
        raise RuntimeError(f"failed: {' '.join(command)}\n{result.stdout}")
    return result.stdout


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cc", default="cc")
    parser.add_argument("--clang", default="clang")
    parser.add_argument("--readelf", default="readelf")
    parser.add_argument("--nm", default="nm")
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="h700-m1-readonly-") as tmp:
        out = Path(tmp)
        host = out / "host-test"
        run([args.cc, "-std=c11", "-O2", "-Wall", "-Wextra", "-Werror",
             "-Wconversion", "-Wsign-conversion", "-I", str(INCLUDE),
             str(SOURCE), str(TEST), "-o", str(host)])
        run([str(host)])

        obj = out / "m1-readonly-a32.o"
        elf = out / "m1-readonly-a32.elf"
        flags = [args.clang, "--target=arm-none-eabi", "-mcpu=cortex-a53",
                 "-marm", "-mfloat-abi=soft", "-std=c11", "-O2",
                 "-Wall", "-Wextra", "-Werror", "-Wconversion",
                 "-Wsign-conversion", "-ffreestanding", "-fno-builtin",
                 "-fno-stack-protector", "-fno-unwind-tables",
                 "-fno-asynchronous-unwind-tables", "-ffunction-sections",
                 "-fdata-sections", "-I", str(INCLUDE)]
        run(flags + ["-c", str(SOURCE), "-o", str(obj)])
        run([args.clang, "--target=arm-none-eabi", "-mcpu=cortex-a53",
             "-marm", "-nostdlib", "-fuse-ld=lld", "-Wl,--no-undefined",
             "-Wl,-e,h700_m1_package_read_validate", "-Wl,-Ttext=0x20400",
             str(obj), "-o", str(elf)])
        header = run([args.readelf, "-h", str(elf)])
        if "ELF32" not in header or "Machine:" not in header or "ARM" not in header:
            raise RuntimeError("preflight object did not link as ELF32 ARM")
        if run([args.nm, "-u", str(elf)]).strip():
            raise RuntimeError("AArch32 package preflight has undefined symbols")
    print("PASS M1 fake-card read-only package tests")
    print("PASS freestanding AArch32 ARM-state ELF32 link")
    print("LIMIT: host fixture only; no BaseOS integration, MMIO, or card writes")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError, subprocess.SubprocessError) as error:
        raise SystemExit(f"FAIL: {error}")
