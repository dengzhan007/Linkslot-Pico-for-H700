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
    # Force the H700 AArch64 preprocessor branch on the host. These narrow declarations
    # mirror the LK mutex/critical-section API; this is a compile guard, not an LK link test.
    stub = out / "lk-stubs"
    (stub / "kernel").mkdir(parents=True, exist_ok=True)
    (stub / "kernel" / "thread.h").write_text(
        "typedef struct { int opaque; } mutex_t;\n"
        "void enter_critical_section(void);\n"
        "void exit_critical_section(void);\n", encoding="utf-8")
    (stub / "kernel" / "mutex.h").write_text(
        "#include <kernel/thread.h>\n"
        "void mutex_init(mutex_t *);\n"
        "void mutex_acquire(mutex_t *);\n"
        "void mutex_release(mutex_t *);\n", encoding="utf-8")
    cmd = [cc, "-D__aarch64__", "-std=c11", "-O2", "-Wall", "-Wextra", "-Werror",
           "-I", str(stub), "-I", str(src), "-c", str(src / "fat_wr.c"),
           "-o", str(out / "fat_wr-aarch64-lock.o")]
    proc = subprocess.run(cmd, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    (out / "fat_wr-aarch64-lock.log").write_text(proc.stdout, encoding="utf-8")
    if proc.returncode:
        print(proc.stdout)
        raise SystemExit("AArch64 mutex branch compile failed")
    results.append("PASS fat_wr.c AArch64 mutex branch (LK API stub declarations)")

    report = "# H700 generic FAT/exFAT host compile audit\n\n"
    report += "\n".join(results)
    report += "\n\nCompile-only validation. No image was mounted or modified; no block driver, "
    report += "path resolver integration, persistence ordering or hardware I/O is claimed.\n"
    (out / "REPORT.md").write_text(report, encoding="utf-8")
    print(report)

if __name__ == "__main__":
    main()
