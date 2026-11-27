#!/usr/bin/env python3
"""Compile and run the host-only M1 capsule selection policy model."""
from __future__ import annotations

import argparse
import subprocess
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cc", default="cc", help="host C compiler")
    args = parser.parse_args()

    source = ROOT / "platform/h700/boot/m1_oneshot_selection_model.c"
    test = ROOT / "tests/test_m1_oneshot_selection_model.c"
    include = ROOT / "platform/h700/boot"

    with tempfile.TemporaryDirectory(prefix="m1-oneshot-select-") as tmp:
        binary = Path(tmp) / "m1-oneshot-selection-test"
        subprocess.run(
            [
                args.cc, "-std=c11", "-O2", "-Wall", "-Wextra", "-Werror",
                "-Wconversion", "-Wsign-conversion",
                "-I", str(include), str(source), str(test), "-o", str(binary),
            ],
            check=True,
        )
        subprocess.run([str(binary)], check=True)

    print("PASS host-only M1 one-shot capsule selection model")
    print("LIMIT: not linked into Boot0/SPL/capsule; no device qualification")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
