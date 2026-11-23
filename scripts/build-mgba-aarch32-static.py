#!/usr/bin/env python3
"""Build the pinned Linkslot mGBA core as an AArch32 namespaced static archive."""
from __future__ import annotations
import argparse
import re
import shutil
import subprocess
import sys
from pathlib import Path

PIN = "e31759b24e7a4e3899285ff720d7b573ac328ae7"
EXPORTS = (
    "retro_api_version", "retro_set_environment", "retro_set_video_refresh",
    "retro_set_audio_sample", "retro_set_audio_sample_batch", "retro_set_input_poll",
    "retro_set_input_state", "retro_get_system_info", "retro_get_system_av_info",
    "retro_init", "retro_deinit", "retro_run", "retro_reset", "retro_load_game",
    "retro_unload_game", "retro_serialize_size", "retro_serialize",
    "retro_unserialize", "retro_cheat_reset", "retro_cheat_set", "retro_get_region",
    "retro_set_controller_port_device", "retro_load_game_special",
    "retro_get_memory_data", "retro_get_memory_size",
)

def run(args, log=None):
    proc = subprocess.run(args, text=True, stdout=subprocess.PIPE,
                          stderr=subprocess.STDOUT)
    if log:
        log.parent.mkdir(parents=True, exist_ok=True)
        log.write_text(proc.stdout, encoding="utf-8")
    if proc.returncode:
        print(proc.stdout[-18000:], file=sys.stderr)
        raise RuntimeError("command failed: " + " ".join(map(str, args)))
    return proc.stdout

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    source, out = args.source.resolve(), args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)

    for tool in ("git", "cmake", "arm-linux-gnueabi-gcc", "arm-linux-gnueabi-g++",
                 "llvm-nm", "llvm-ar"):
        if not shutil.which(tool):
            raise RuntimeError("required tool missing: " + tool)

    actual = run(["git", "-C", str(source), "rev-parse", "HEAD"]).strip()
    if actual != PIN:
        raise RuntimeError(f"mGBA source pin mismatch: expected {PIN}, got {actual}")

    defines = " ".join("-D" + symbol + "=mgba_" + symbol for symbol in EXPORTS)
    cflags = (
        "-mcpu=cortex-a53 -marm -mfpu=neon-vfpv4 -mfloat-abi=softfp "
        "-fno-pic -fno-pie -ffunction-sections -fdata-sections " + defines
    )
    build = out / "cmake-build"
    configure = [
        "cmake", "-S", str(source), "-B", str(build),
        "-DCMAKE_SYSTEM_NAME=Linux",
        "-DCMAKE_SYSTEM_PROCESSOR=arm",
        "-DCMAKE_C_COMPILER=arm-linux-gnueabi-gcc",
        "-DCMAKE_CXX_COMPILER=arm-linux-gnueabi-g++",
        "-DCMAKE_AR=arm-linux-gnueabi-ar",
        "-DCMAKE_RANLIB=arm-linux-gnueabi-ranlib",
        "-DCMAKE_TRY_COMPILE_TARGET_TYPE=STATIC_LIBRARY",
        "-DCMAKE_BUILD_TYPE=Release",
        "-DCMAKE_C_FLAGS=" + cflags,
        "-DLIBMGBA_ONLY=ON",
        "-DBUILD_LIBRETRO=ON",
        "-DLIBRETRO_STATIC=ON",
        "-DBUILD_STATIC=ON",
        "-DBUILD_SHARED=OFF",
        "-DDISABLE_DEPS=ON",
        "-DM_CORE_GBA=ON",
        "-DM_CORE_GB=ON",
        "-DBUILD_TEST=OFF",
        "-DBUILD_SUITE=OFF",
        "-DBUILD_QT=OFF",
        "-DBUILD_SDL=OFF",
    ]
    run(configure, out / "cmake-configure.log")
    run(["cmake", "--build", str(build), "--target", "mgba_libretro", "--parallel", "2"],
        out / "cmake-build.log")

    archives = list(build.rglob("mgba_libretro.a"))
    if len(archives) != 1:
        raise RuntimeError("expected one mgba_libretro.a, found: " + repr([str(p) for p in archives]))
    archive = archives[0]
    copied = out / "mgba_libretro-aarch32-static.a"
    shutil.copy2(archive, copied)

    members = run(["arm-linux-gnueabi-ar", "t", str(copied)])
    if not members.strip():
        raise RuntimeError("static archive is empty")
    symbols = run(["llvm-nm", "-g", "--defined-only", str(copied)])
    undefined = run(["llvm-nm", "-u", str(copied)])
    defined_names = set()
    unprefixed = []
    for line in symbols.splitlines():
        cols = line.split()
        if len(cols) >= 2:
            name = cols[-1]
            defined_names.add(name)
            if re.match(r"^retro_[A-Za-z0-9_]+$", name):
                unprefixed.append(name)
    missing = ["mgba_" + symbol for symbol in EXPORTS if "mgba_" + symbol not in defined_names]
    if missing:
        raise RuntimeError("namespaced Libretro exports missing: " + ", ".join(missing))
    if unprefixed:
        raise RuntimeError("unprefixed Libretro exports remain: " + ", ".join(sorted(set(unprefixed))))

    report = [
        "# mGBA AArch32 static Libretro archive build\n\n",
        "Pinned source: libretro/mgba @ " + actual + "\n",
        "Toolchain: Debian arm-linux-gnueabi GCC/G++; Cortex-A53, ARM state, NEON/VFPv4, softfp ABI.\n",
        "CMake: LIBMGBA_ONLY, BUILD_LIBRETRO, LIBRETRO_STATIC, M_CORE_GBA and M_CORE_GB enabled; DISABLE_DEPS enabled; Release.\n",
        "Every Libretro entry point from src/platform/libretro/libretro.c was compile-time renamed retro_* -> mgba_retro_*.\n",
        "Archive: " + copied.name + " (" + str(copied.stat().st_size) + " bytes), " +
        str(len(members.splitlines())) + " object members.\n",
        "Expected API exports checked: " + str(len(EXPORTS)) + "/" + str(len(EXPORTS)) +
        " present; no unprefixed retro_* global definitions remain.\n",
        "Undefined archive references are captured in mgba_libretro-undefined.txt. Static archives intentionally retain references for final LK/libc/HAL linking.\n",
        "This is a real mGBA source compilation for AArch32, not a firmware link or runtime test.\n\n",
        "## Unresolved references\n\n",
        "The complete llvm-nm -u list is saved in mgba_libretro-undefined.txt. It must be grouped into LK libc, platform/HAL services and remaining blockers before final static linking.\n",
    ]
    (out / "mgba_libretro-undefined.txt").write_text(undefined, encoding="utf-8")
    (out / "mgba_libretro-defined-symbols.txt").write_text(symbols, encoding="utf-8")
    (out / "mgba_libretro-members.txt").write_text(members, encoding="utf-8")
    (out / "REPORT.md").write_text("".join(report), encoding="utf-8")
    print((out / "REPORT.md").read_text(encoding="utf-8"))

if __name__ == "__main__":
    main()
