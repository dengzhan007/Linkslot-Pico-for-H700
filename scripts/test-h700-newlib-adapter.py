#!/usr/bin/env python3
"""Host-regress H700 newlib hooks and cross-compile the production AArch32 object."""
from __future__ import annotations
import argparse
import re
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
    "_malloc_r=h700_test_malloc_r", "_calloc_r=h700_test_calloc_r",
    "_realloc_r=h700_test_realloc_r", "_free_r=h700_test_free_r",
    "__malloc_lock=h700_test_malloc_lock", "__malloc_unlock=h700_test_malloc_unlock",
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
    ap.add_argument("--lk-source", type=Path, required=True, help="pinned LK reference source")
    ap.add_argument("--out", type=Path, required=True)
    args = ap.parse_args()
    root, out = args.source.resolve(), args.out.resolve()
    lk_root = args.lk_source.resolve()
    out.mkdir(parents=True, exist_ok=True)
    src = root / "platform/h700/runtime/newlib_syscalls.c"
    header = root / "platform/h700/runtime/newlib_syscalls.h"
    harness = root / "platform/h700/runtime/tests/newlib_syscalls_host.c"
    lk_bindings = root / "platform/h700/runtime/lk_bindings.c"
    if not src.is_file() or not header.is_file() or not harness.is_file() or not lk_bindings.is_file():
        raise RuntimeError("H700 newlib adapter, LK binding or host regression source missing")
    if not (lk_root / "include/lib/heap.h").is_file():
        raise RuntimeError("pinned LK include/lib/heap.h not found")

    for tool in ("cc", "arm-none-eabi-gcc", "arm-none-eabi-nm", "arm-none-eabi-readelf"):
        if not shutil.which(tool):
            raise RuntimeError("missing required compiler/tool: " + tool)

    host_obj = out / "newlib_syscalls_host.o"
    harness_obj = out / "newlib_syscalls_host_test.o"
    host_exe = out / "newlib_syscalls_host_test"
    host_flags = [
        "-std=c11", "-D_POSIX_C_SOURCE=200112L", "-D_DEFAULT_SOURCE",
        "-Wall", "-Wextra", "-Werror", "-I", str(src.parent),
        "-I", str(root / "platform/h700/runtime/tests/include"),
    ] + ["-D" + item for item in RENAMES]

    run(["cc", *host_flags, "-c", str(src), "-o", str(host_obj)],
        out / "host-adapter-compile.log")
    run(["cc", "-std=c11", "-D_POSIX_C_SOURCE=200112L", "-D_DEFAULT_SOURCE",
         "-Wall", "-Wextra", "-Werror", "-I", str(src.parent),
         "-I", str(root / "platform/h700/runtime/tests/include"),
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
    is_elf32 = bool(re.search(r"Class:\s+ELF32\b", header_text))
    is_arm = bool(re.search(r"Machine:\s+ARM\b", header_text))
    is_eabi5 = bool(re.search(r"Flags:\s+0x5000000,\s*Version5 EABI", header_text))
    if not (is_elf32 and is_arm and is_eabi5):
        raise RuntimeError("newlib adapter object is not ELF32/ARM EABI5; readelf header was:\n" + header_text)
    # Compile the real LK heap binding against the pinned LK header. Werror
    # ensures the heap function pointer prototypes agree exactly.
    binding_obj = out / "lk_bindings-arm-none-eabi.o"
    binding_flags = cpu_flags + ["-I", str(lk_root / "include")]
    run(["arm-none-eabi-gcc", *binding_flags, "-c", str(lk_bindings), "-o", str(binding_obj)],
        out / "lk-bindings-compile.log")
    binding_defined = run(["arm-none-eabi-nm", "-g", "--defined-only", str(binding_obj)],
                          out / "lk-bindings-defined.log")
    binding_undefined = run(["arm-none-eabi-nm", "-u", str(binding_obj)],
                            out / "lk-bindings-undefined.log")
    binding_defined_names = {line.split()[-1] for line in binding_defined.splitlines()
                             if len(line.split()) >= 2}
    binding_undefined_names = {line.split()[-1] for line in binding_undefined.splitlines()
                               if len(line.split()) >= 2}
    if "h700_newlib_install_lk_bindings" not in binding_defined_names:
        raise RuntimeError("LK binding install entry missing")
    required_lk = {"heap_alloc", "heap_free",
                   "h700_platform_heap_ready", "h700_platform_single_cpu_active",
                   "h700_platform_console_read", "h700_platform_console_write",
                   "h700_platform_rtc_gettimeofday", "h700_platform_fatal"}
    if not required_lk.issubset(binding_undefined_names):
        raise RuntimeError("LK binding external dependencies mismatch: missing " +
                           repr(sorted(required_lk - binding_undefined_names)))

    defined = run(["arm-none-eabi-nm", "-g", "--defined-only", str(target_obj)],
                  out / "arm-none-eabi-defined-symbols.log")
    defined_names = {line.split()[-1] for line in defined.splitlines() if len(line.split()) >= 2}
    required = {
        "malloc", "calloc", "realloc", "free", "_sbrk", "_close", "_fstat",
        "_isatty", "_lseek", "_read", "_write", "_gettimeofday", "_getpid",
        "_kill", "_exit", "h700_newlib_install",
        "_malloc_r", "_calloc_r", "_realloc_r", "_free_r",
        "__malloc_lock", "__malloc_unlock",
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
        "Pinned LK heap binding: PASS. lk_bindings.c compiles with -Werror against the exact pinned include/lib/heap.h signatures; heap_alloc/heap_free resolve to LK externals and heap-ready/single-core/console/RTC/fatal services remain explicit platform integration points.\n\n",
        "Adapter exports: all 22 public and reentrant allocator/syscall/lock/install symbols are present.\n\n",
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
