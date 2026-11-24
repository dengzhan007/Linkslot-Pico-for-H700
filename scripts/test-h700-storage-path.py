#!/usr/bin/env python3
"""Compile and run the standalone H700 card-path containment regression."""
from __future__ import annotations
import argparse
import shutil
import subprocess
from pathlib import Path

def main() -> None:
    p = argparse.ArgumentParser()
    p.add_argument("--module", type=Path, required=True)
    p.add_argument("--out", type=Path, required=True)
    a = p.parse_args()
    rustc = shutil.which("rustc")
    if not rustc:
        raise SystemExit("rustc is required")
    a.out.mkdir(parents=True, exist_ok=True)
    binary = a.out / "storage-path-tests"
    cmd = [rustc, "--edition=2021", "--test", str(a.module), "-o", str(binary)]
    built = subprocess.run(cmd, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    (a.out / "compile.log").write_text(built.stdout, encoding="utf-8")
    if built.returncode:
        raise SystemExit(built.stdout)
    run = subprocess.run([str(binary), "--test-threads=1"], text=True,
                         stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    (a.out / "test.log").write_text(run.stdout, encoding="utf-8")
    print(run.stdout)
    if run.returncode:
        raise SystemExit(run.returncode)
    report = "# H700 card path containment regression\n\n"
    report += "PASS: standalone Rust module compiled and all lexical containment tests passed.\n\n"
    report += "This validates path policy only; it does not implement SD block I/O, FAT/exFAT, "
    report += "file handles, flush barriers, atomic replacement, or SMP synchronization.\n\n"
    report += run.stdout
    (a.out / "REPORT.md").write_text(report, encoding="utf-8")

if __name__ == "__main__":
    main()
