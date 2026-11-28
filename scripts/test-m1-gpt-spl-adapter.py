#!/usr/bin/env python3
"""Host-test M1's read-only GPT-to-U-Boot block adapter and package preflight."""
from __future__ import annotations

import argparse
import re
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BOOT = ROOT / "platform/h700/boot"
GPT = ROOT / "platform/h700/storage/gpt"
FAKE = ROOT / "tests/fake_uboot"
TEST = ROOT / "tests/test_m1_gpt_spl_adapter.c"


def run(command: list[str]) -> str:
    result = subprocess.run(command, stdout=subprocess.PIPE,
                            stderr=subprocess.STDOUT, text=True)
    if result.returncode:
        raise RuntimeError(f"failed: {' '.join(command)}\n{result.stdout}")
    return result.stdout


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--cc", default="cc")
    ap.add_argument("--nm", default="nm")
    args = ap.parse_args()

    with tempfile.TemporaryDirectory(prefix="h700-m1-gpt-spl-") as tmp:
        exe = Path(tmp) / "m1-gpt-spl-test"
        run([
            args.cc, "-std=c11", "-O2", "-Wall", "-Wextra", "-Werror",
            "-Wconversion", "-Wsign-conversion",
            "-I", str(FAKE), "-I", str(BOOT), "-I", str(GPT),
            str(GPT / "h700_gpt_readonly.c"),
            str(BOOT / "m1_gpt_spl_adapter.c"),
            str(BOOT / "m1_firmware_readonly.c"),
            str(BOOT / "m1_firmware_spl_adapter.c"),
            str(TEST), "-o", str(exe),
        ])
        output = run([str(exe)])
        undefined = run([args.nm, "-u", str(exe)])
        symbols = run([args.nm, "-S", str(exe)])
        if "blk_dwrite" in undefined or "blk_dwrite" in symbols:
            raise RuntimeError("forbidden block-write symbol present")
        if "blk_dread" not in symbols:
            raise RuntimeError("expected read-only blk_dread symbol missing")
        if "m1_boot_partitions" not in symbols:
            raise RuntimeError("bounded eight-entry GPT table is not present in the linked adapter")
        scratch = re.search(r"^[0-9a-fA-F]+\\s+([0-9a-fA-F]+)\\s+[bB]\\s+m1_package_scratch$", symbols, re.M)
        if not scratch or int(scratch.group(1), 16) != 4096:
            raise RuntimeError("expected aligned 4KiB static M1 package scratch symbol missing")
        print(output, end="")
        print("PASS adapter link has blk_dread only; no blk_dwrite")
        print("PASS bounded eight-slot GPT table is a linked static object")
        print("PASS named 4KiB static package scratch symbol is linked")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError, subprocess.SubprocessError) as error:
        raise SystemExit(f"FAIL: {error}")
