#!/usr/bin/env python3
"""Host-regress H700 newlib hooks and cross-compile the production AArch32 object."""
from __future__ import annotations
import argparse
import shutil
import subprocess
import sys
from pathlib import Path

RENAMES = [
    "malloc=h700_test_malloc", "calloc=h700_test_calloc",
    "realloc=h700_test_realloc", "free=h700_test_free",
    "_sbrk=h700_test_sbrk", "_close=h700_test_close",
    "_fstat=h700_test_fstat", "_isatty=h700_test_isatty",
    "_lseek=h700_test_lseek", "_read=h700_test_read",
    "_write=h700_test_write", "_gettimeofday=h700_test_gettimeofday",
    "_getpid=h700_test_getpid", "_kill=h700_test_kill",
    "_exit=h700_test_exit",
]

def run(args, log):
    proc = subprocess.run(args, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    log.parent.mkdir(parents=True, exist_ok=True)
    log.write_text(proc.stdout, encoding="utf-8")
    if proc.returncode:
        print(proc.stdout[-15000:], file=sys.stderr)
        raise RuntimeError("command failed: " + " ".join(map(str, args)))
    return proc.stdout

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--source", type=Path, required=True, help="repository root")
    ap.add_argument("--out", type=Path, required=True)
    args = ap.parse_args()
    root, out = args.source.resolve(), args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    src = root / "platform/h700/runtime/newlib_syscalls.c"
    header = root / "platform/h700/runtime/newlib_syscalls.h"
    harness = root / "platform/h700/runtime/tests/newlib_syscalls_host.c"
    if not src.is_file() or not header.is_file() or not harness.is_file():
        raise RuntimeError("H700 newlib adapter source or host regression file missing")

    for tool in ("cc", "arm-none-eabi-gcc", "arm-none-eabi-nm", "arm-none-eabi-readelf"):
        if not shutil.which(tool):
            raise RuntimeError("missing required compiler/tool: " + tool)

    host_obj = out / "newlib_syscalls_host.o"
    harness_obj = out / "newlib_syscalls_host_test.o"
    host_exe = out / "newlib_syscalls_host_test"
    host_flags = [
        "-std=c11", "-D_POSIX_C_SOURCE=200112L", "-D_DEFAULT_SOURCE",
        "-Wall", "-Wextra", "-Werror", "-I", str(src.parent),
    ] + ["-D" + item for item in RENAMES]

    run(["cc", *host_flags, "-c", str(src), "-o", str(host_obj)],
        out / "host-adapter-compile.log")
    run(["cc", "-std=c11", "-D_POSIX_C_SOURCE=200112L", "-D_DEFAULT_SOURCE",
         "-Wall", "-Wextra", "-Werror", "-I", str(src.parent),
         "-c", str(harness), "-o", str(harness_obj)],
        out / "host-test-compile.log")
    run(["cc", str(host_obj), str(harness_obj), "-o", str(host_exe)],
        out / "host-link.log")
    output = run([str(host_exe)], out / "host-run.log")

    # Compile the unrenamed production object against the actual newlib headers.
    target_obj = out / "newlib_syscalls-arm-none-eabi.o"
    cpu_flags = [
        "-std=c11", "-mcpu=cortex-a53", "-marm", "-mfpu=neon-vfpv4",
        "-mfloat-abi=softfp", "-ffreestanding", "-fno-builtin",
        "-fno-pic", "-fno-pie", "-fno-stack-protector",
        "-Wall", "-Wextra", "-Werror",
        "-I", str(src.parent),
    ]
    run(["arm-none-eabi-gcc", *cpu_flags, "-c", str(src), "-o", str(target_obj)],
        out / "arm-none-eabi-adapter-compile.log")
    header_text = run(["arm-none-eabi-readelf", "-h", str(target_obj)],
                      out / "arm-none-eabi-object-header.log")
    if "ELF32" not in header_text or "ARM" not in header_text or "EABI5" not in header_text:
        raise RuntimeError("newlib adapter object is not ELF32/ARM EABI5")
    defined = run(["arm-none-eabi-nm", "-g", "--defined-only", str(target_obj)],
                  out / "arm-none-eabi-defined-symbols.log")
    defined_names = {line.split()[-1] for line in defined.splitlines() if len(line.split()) >= 2}
    required = {
        "malloc", "calloc", "realloc", "free", "_sbrk", "_close", "_fstat",
        "_isatty", "_lseek", "_read", "_write", "_gettimeofday", "_getpid",
        "_kill", "_exit", "h700_newlib_install",
    }
    missing = sorted(required - defined_names)
    if missing:
        raise RuntimeError("adapter object missing required exports: " + ", ".join(missing))

    unresolved = run(["arm-none-eabi-nm", "-u", str(target_obj)],
                     out / "arm-none-eabi-undefined-symbols.log")
    report = [
        "# H700 newlib adapter validation\n\n",
        "Host regression: PASS. " + output.strip() + "\n\n",
        "AArch32 target compile: PASS. Actual arm-none-eabi/newlib headers, Cortex-A53 ARM state, NEON/VFPv4 softfp; object is ELF32/ARM EABI5.\n\n",
        "Adapter exports: all 16 required allocator/syscall/install symbols are present.\n\n",
        "Target object undefined dependencies:\n\n~~~text\n",
        unresolved,
        "~~~\n\n",
        "Safety gates: allocation is denied until heap_ready() and single_cpu_active() are both true. free() enters the fatal path if this invariant is lost. No heap address or size is assumed. _sbrk always returns ENOMEM because the adapter routes malloc-family calls through LK heap_alloc/realloc/free instead of reserving a second contiguous arena.\n\n",
        "Service gaps: console input/output and wall-clock time are dependency-injected. Missing console/RTC services return ENOSYS. The _exit/fatal callback must bind to an LK panic path.\n\n",
        "Boundary: the host heap and console/RTC implementations are test fakes. This is not a live LK heap test, does not prove secondary CPUs are parked, does not provide an H700 RTC/UART, and is not a firmware or device test.\n",
    ]
    (out / "REPORT.md").write_text("".join(report), encoding="utf-8")
    print((out / "REPORT.md").read_text(encoding="utf-8"))

if __name__ == "__main__":
    main()
