#!/usr/bin/env python3
"""Host-compile the imported generic FAT16/32 and exFAT modules and test harnesses."""
from __future__ import annotations
import argparse
import shutil
import subprocess
from pathlib import Path

def main() -> None:
    p = argparse.ArgumentParser()
    p.add_argument("--source", type=Path, required=True)
    p.add_argument("--out", type=Path, required=True)
    a = p.parse_args()
    cc = shutil.which("cc") or shutil.which("gcc")
    if not cc:
        raise SystemExit("cc/gcc is required")
    src = a.source.resolve()
    out = a.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    files = [
        "fat_ro.c", "fat_wr.c", "fat_exfat.c",
        "tests/fat_ro_test.c", "tests/fat_exfat_rw_test.c",
    ]
    results = []
    for rel in files:
        obj = out / (rel.replace("/", "_") + ".o")
        cmd = [cc, "-std=c11", "-O2", "-Wall", "-Wextra", "-Werror",
               "-I", str(src), "-c", str(src / rel), "-o", str(obj)]
        proc = subprocess.run(cmd, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        (out / (obj.name + ".log")).write_text(proc.stdout, encoding="utf-8")
        if proc.returncode:
            print(proc.stdout)
            raise SystemExit(f"compile failed: {rel}")
        results.append(f"PASS {rel}")
    report = "# H700 generic FAT/exFAT host compile audit\n\n"
    report += "\n".join(results)
    report += "\n\nCompile-only validation. No image was mounted or modified; no block driver, "
    report += "path resolver integration, persistence ordering or hardware I/O is claimed.\n"
    (out / "REPORT.md").write_text(report, encoding="utf-8")
    print(report)

if __name__ == "__main__":
    main()
