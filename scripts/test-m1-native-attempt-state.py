#!/usr/bin/env python3
"""Run a host-only regression model for the pinned BaseOS RTC attempt pair."""
from __future__ import annotations

import argparse
import subprocess
import tempfile
from pathlib import Path

PIN = "cbe253a43f46cf551cc9a6c79a84015bde3686cf"
ROOT = Path(__file__).resolve().parents[1]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseos-root", type=Path, required=True)
    parser.add_argument("--cc", default="cc")
    args = parser.parse_args()

    baseos = args.baseos_root.resolve()
    head = subprocess.run(
        ["git", "-C", str(baseos), "rev-parse", "HEAD"],
        check=True, capture_output=True, text=True,
    ).stdout.strip()
    if head != PIN:
        raise RuntimeError(f"BaseOS must be pinned at {PIN}; found {head}")
    header = baseos / "src/native-attempt.h"
    if not header.is_file():
        raise RuntimeError("pinned native-attempt.h is missing")

    source = ROOT / "tests/test_m1_native_attempt_state.c"
    with tempfile.TemporaryDirectory(prefix="m1-rtc-attempt-model-") as tmp:
        binary = Path(tmp) / "m1-native-attempt-state-test"
        subprocess.run(
            [
                args.cc, "-std=c11", "-O2", "-Wall", "-Wextra", "-Werror",
                "-Wconversion", "-Wsign-conversion",
                "-I", str(baseos / "src"), str(source), "-o", str(binary),
            ],
            check=True,
        )
        subprocess.run([str(binary)], check=True)

    print("PASS pinned BaseOS RTC attempt-pair host-only model")
    print("LIMIT: fake registers only; no RTC/WDT MMIO, device I/O, or hardware qualification")
    print("LIMIT: validates the existing BaseOS v2 pair semantics, not approval to reuse them in M1")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
