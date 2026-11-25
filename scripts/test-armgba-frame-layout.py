#!/usr/bin/env python3
"""Verify the pinned armGBA video.frameCounter offset across host and H700 ABIs."""

from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
from pathlib import Path

PINNED_COMMIT = "fad6c4b0c0176a76e10667f4aa071a8feb4425c0"
EXPECTED_GBA_SIZE = 788712


def run(command: list[str], *, capture: bool = False) -> str:
    result = subprocess.run(
        command,
        check=True,
        text=True,
        stdout=subprocess.PIPE if capture else None,
        stderr=subprocess.PIPE if capture else None,
    )
    return result.stdout.strip() if capture else ""


def require_tool(name: str) -> str:
    path = shutil.which(name)
    if not path:
        raise SystemExit(f"required compiler not found: {name}")
    return path


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path, required=True,
                        help="checkout of the pinned armGBA source")
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--cc", default="cc")
    parser.add_argument("--arm-cc", default="arm-none-eabi-gcc")
    parser.add_argument("--clang", default="clang")
    args = parser.parse_args()

    source = args.source.resolve()
    header = source / "src" / "gba.h"
    probe = Path(__file__).resolve().parents[1] / "tools" / "armgba-frame-layout-probe.c"
    if not header.is_file() or not probe.is_file():
        raise SystemExit("pinned armGBA src/gba.h or layout probe source is missing")

    commit = run(["git", "-C", str(source), "rev-parse", "HEAD"], capture=True)
    if commit != PINNED_COMMIT:
        raise SystemExit(f"armGBA source pin mismatch: {commit} != {PINNED_COMMIT}")

    cc = require_tool(args.cc)
    arm_cc = require_tool(args.arm_cc)
    clang = require_tool(args.clang)
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)

    host_exe = out / "armgba-layout-host"
    run([
        cc, "-std=c11", "-Wall", "-Wextra", "-DARMGBA_PRINT_LAYOUT",
        "-I", str(source / "src"), str(probe), "-o", str(host_exe),
    ])
    raw = run([str(host_exe)], capture=True)
    values = {key: int(value) for key, value in
              re.findall(r"^([a-z_]+)=(\d+)$", raw, flags=re.MULTILINE)}
    required = {
        "gba_size", "video_offset", "frame_counter_offset",
        "gba_frame_counter_offset",
    }
    if set(values) != required:
        raise SystemExit(f"unexpected host probe output: {raw!r}")
    if values["gba_size"] != EXPECTED_GBA_SIZE:
        raise SystemExit(
            f"pinned armGBA size assertion changed: {values['gba_size']} != "
            f"{EXPECTED_GBA_SIZE}"
        )
    if values["gba_frame_counter_offset"] != (
        values["video_offset"] + values["frame_counter_offset"]
    ):
        raise SystemExit("host frame-counter offset arithmetic mismatch")

    common_defines = [
        f"-DARMGBA_EXPECT_GBA_SIZE={values['gba_size']}",
        f"-DARMGBA_EXPECT_VIDEO_OFFSET={values['video_offset']}",
        f"-DARMGBA_EXPECT_COUNTER_OFFSET={values['frame_counter_offset']}",
        f"-DARMGBA_EXPECT_FRAME_OFFSET={values['gba_frame_counter_offset']}",
    ]
    compiled = []
    targets = [
        ("aarch32-arm", arm_cc, ["-mcpu=cortex-a53", "-marm"]),
        ("aarch32-thumb2", arm_cc, ["-mcpu=cortex-a53", "-mthumb"]),
    ]
    for label, compiler, flags in targets:
        obj = out / f"armgba-layout-{label}.o"
        run([
            compiler, "-std=c11", "-Wall", "-Wextra", "-ffreestanding",
            *flags, *common_defines, "-I", str(source / "src"),
            "-c", str(probe), "-o", str(obj),
        ])
        compiled.append({"target": label, "object": str(obj)})

    a64_obj = out / "armgba-layout-aarch64.o"
    run([
        clang, "--target=aarch64-none-elf", "-std=c11", "-Wall", "-Wextra",
        "-ffreestanding", *common_defines, "-I", str(source / "src"),
        "-c", str(probe), "-o", str(a64_obj),
    ])
    compiled.append({"target": "aarch64-none-elf", "object": str(a64_obj)})

    report = {
        "source_repository": "MagnominusGit/armGBA",
        "source_commit": commit,
        "counter_source": "struct GBA::video.frameCounter",
        "counter_increment": "src/video/video.c at VBlank frame end",
        "libretro_global": "libretro.c: struct GBA* gba",
        "layout": values,
        "cross_compile_checks": compiled,
        "result": "PASS",
        "scope": "compile-time source-layout equivalence only; no device memory read",
    }
    (out / "armgba-frame-layout.json").write_text(
        json.dumps(report, indent=2) + "\n", encoding="utf-8"
    )
    (out / "REPORT.md").write_text(
        "# armGBA committed-frame layout probe\n\n"
        f"- Source: `MagnominusGit/armGBA@{commit}`\n"
        f"- GBA size: {values['gba_size']} bytes\n"
        f"- `GBA.video` offset: {values['video_offset']}\n"
        f"- `Video.frameCounter` offset: {values['frame_counter_offset']}\n"
        f"- Combined `gba->video.frameCounter` offset: "
        f"**{values['gba_frame_counter_offset']} bytes**\n"
        "- Cross-checks: AArch32 ARM, AArch32 Thumb-2, AArch64 all compiled "
        "with static offset assertions.\n"
        "- Scope: source-layout compile check only; no ELF identity claim and "
        "no target process memory access.\n",
        encoding="utf-8",
    )
    print((out / "REPORT.md").read_text(encoding="utf-8"), end="")


if __name__ == "__main__":
    main()
