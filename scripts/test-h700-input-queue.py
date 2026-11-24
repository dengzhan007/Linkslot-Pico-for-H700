#!/usr/bin/env python3
"""Build and run the bounded H700 input queue host regression."""
from __future__ import annotations

import argparse
import os
import subprocess
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--cc", default=os.environ.get("CC", "cc"))
    parser.add_argument("--out", type=Path, default=Path("build/h700-input-queue"))
    args = parser.parse_args()

    root = Path(__file__).resolve().parents[1]
    include = root / "platform/h700/runtime"
    source = root / "platform/h700/runtime/input_queue.c"
    test = root / "scripts/test-h700-input-queue.c"
    args.out.mkdir(parents=True, exist_ok=True)
    binary = (args.out / "test-h700-input-queue").resolve()

    command = [
        args.cc,
        "-std=c11",
        "-O2",
        "-Wall",
        "-Wextra",
        "-Werror",
        "-pedantic",
        f"-I{include}",
        str(source),
        str(test),
        "-o",
        str(binary),
    ]
    print("+", " ".join(command), flush=True)
    subprocess.run(command, check=True)
    subprocess.run([str(binary)], check=True)


if __name__ == "__main__":
    main()
