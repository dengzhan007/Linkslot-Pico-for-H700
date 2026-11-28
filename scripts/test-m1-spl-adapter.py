#!/usr/bin/env python3
"""Host-test the exact U-Boot SPL blk_dread adapter with a fake block device."""
from __future__ import annotations
import argparse
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BOOT = ROOT / "platform/h700/boot"
FAKE = ROOT / "tests/fake_uboot"
TEST = ROOT / "tests/test_m1_firmware_spl_adapter.c"

def run(args: list[str]) -> str:
    p = subprocess.run(args, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    if p.returncode:
        raise RuntimeError("command failed: " + " ".join(args) + "\n" + p.stdout)
    return p.stdout

def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--cc", default="cc")
    ap.add_argument("--nm", default="nm")
    a = ap.parse_args()
    with tempfile.TemporaryDirectory(prefix="h700-m1-spl-adapter-") as tmp:
        exe = Path(tmp) / "adapter-test"
        run([a.cc, "-std=c11", "-O2", "-Wall", "-Wextra", "-Werror",
             "-Wconversion", "-Wsign-conversion", "-I", str(FAKE), "-I", str(BOOT),
             str(BOOT / "m1_firmware_readonly.c"),
             str(BOOT / "m1_firmware_spl_adapter.c"), str(TEST), "-o", str(exe)])
        output = run([str(exe)])
        symbols = run([a.nm, "-u", str(exe)])
        if "blk_dwrite" in symbols:
            raise RuntimeError("forbidden block-write dependency in SPL adapter")
        if "blk_dread" not in run([a.nm, str(exe)]):
            raise RuntimeError("expected blk_dread adapter symbol missing")
        print(output, end="")
        print("PASS adapter binary has no blk_dwrite reference")
    return 0

if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError, subprocess.SubprocessError) as e:
        raise SystemExit("FAIL: " + str(e))
