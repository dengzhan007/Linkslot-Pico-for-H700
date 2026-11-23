#!/usr/bin/env python3
"""Diagnostic whole-archive link of mGBA against arm-none-eabi newlib/libm/libgcc."""
from __future__ import annotations
import argparse
import re
import shutil
import subprocess
from pathlib import Path

def invoke(args, log):
    p = subprocess.run(args, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    log.parent.mkdir(parents=True, exist_ok=True)
    log.write_text(p.stdout, encoding="utf-8")
    return p.returncode, p.stdout

def parse_undefined(output):
    found = set()
    patterns = (
        r"undefined reference to [\x60']([^\x60']+)[\x60']",
        r"undefined symbol: ([A-Za-z_][A-Za-z0-9_]*)",
    )
    for pattern in patterns:
        found.update(re.findall(pattern, output))
    return sorted(found)

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--archive", type=Path, required=True)
    ap.add_argument("--out", type=Path, required=True)
    args = ap.parse_args()
    archive, out = args.archive.resolve(), args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    if not archive.is_file():
        raise RuntimeError("mGBA archive not found: " + str(archive))
    for tool in ("arm-none-eabi-gcc", "arm-none-eabi-nm", "arm-none-eabi-readelf"):
        if not shutil.which(tool):
            raise RuntimeError("missing tool: " + tool)

    stub = out / "link-entry.c"
    stub.write_text(
        "__attribute__((noreturn)) void _start(void) { "
        'for (;;) { __asm__ volatile("wfi"); } }\n',
        encoding="utf-8"
    )
    stub_obj = out / "link-entry.o"
    cpu_flags = [
        "-mcpu=cortex-a53", "-marm", "-mfpu=neon-vfpv4", "-mfloat-abi=softfp",
        "-ffreestanding", "-fno-builtin", "-fno-pic", "-fno-pie",
        "-fno-stack-protector", "-ffunction-sections", "-fdata-sections",
    ]
    rc, output = invoke(
        ["arm-none-eabi-gcc", *cpu_flags, "-c", str(stub), "-o", str(stub_obj)],
        out / "entry-compile.log"
    )
    if rc:
        raise RuntimeError("could not compile link entry stub:\n" + output[-8000:])

    groups = [
        ("newlib-only", ["-lc", "-lm", "-lgcc"]),
        ("newlib-nosys", ["-lc", "-lm", "-lgcc", "-lnosys"]),
    ]
    result_lines = ["# mGBA whole-archive link diagnostics with arm-none-eabi/newlib\n\n"]
    successful_elf = None
    for name, libs in groups:
        elf = out / (name + ".elf")
        mapfile = out / (name + ".map")
        command = [
            "arm-none-eabi-gcc", *cpu_flags, "-nostartfiles", "-static",
            "-Wl,-e,_start", "-Wl,--no-undefined",
            "-Wl,-Map=" + str(mapfile),
            "-Wl,--whole-archive", str(archive), "-Wl,--no-whole-archive",
            str(stub_obj), "-Wl,--start-group", *libs, "-Wl,--end-group",
            "-o", str(elf),
        ]
        rc, output = invoke(command, out / (name + ".log"))
        unresolved = parse_undefined(output)
        if rc == 0 and elf.is_file():
            header = subprocess.check_output(
                ["arm-none-eabi-readelf", "-h", str(elf)], text=True
            )
            nm_undefined = subprocess.check_output(
                ["arm-none-eabi-nm", "-u", str(elf)], text=True
            ).strip()
            if "ELF32" in header and "ARM" in header and not nm_undefined:
                successful_elf = elf
                result_lines.append("## " + name + "\n\n")
                result_lines.append("PASS: final test ELF32/ARM linked with no undefined symbols.\n\n")
            else:
                result_lines.append("## " + name + "\n\n")
                result_lines.append("Link returned 0 but ELF/symbol validation failed.\n")
                result_lines.append(nm_undefined + "\n\n")
        else:
            result_lines.append("## " + name + "\n\n")
            result_lines.append("NOT CLOSED: whole-archive link failed (diagnostic, not a CI failure).\n")
            result_lines.append("Undefined linker references captured: " + str(len(unresolved)) + ".\n")
            if unresolved:
                result_lines.append("\n")
                result_lines.extend("- " + symbol + "\n" for symbol in unresolved)
            else:
                result_lines.append("Inspect " + name + ".log for non-symbol linker/toolchain errors.\n")
            result_lines.append("\n")
            (out / (name + "-unresolved.txt")).write_text("\n".join(unresolved) + "\n", encoding="utf-8")

    if successful_elf:
        result_lines.append("At least one newlib link configuration closed the entire mGBA archive.\n")
    else:
        result_lines.append("Neither configuration closed the archive; unresolved references are listed above.\n")
    result_lines.append(
        "Boundary: _start is only a link-test entry stub. libnosys routines are dummy syscall stubs, "
        "not H700/LK services. This is not firmware, not a valid runtime and not a device test.\n"
    )
    (out / "REPORT.md").write_text("".join(result_lines), encoding="utf-8")
    print((out / "REPORT.md").read_text(encoding="utf-8"))

if __name__ == "__main__":
    main()
