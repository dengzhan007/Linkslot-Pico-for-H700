#!/usr/bin/env python3
"""Build the pinned mGBA Libretro core with CMake's Generic platform backend."""
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

def names(output):
    found = set()
    for line in output.splitlines():
        parts = line.split()
        if not parts:
            continue
        name = parts[-1]
        if name.endswith(".o:"):
            continue
        if name not in ("U",):
            found.add(name)
    return found

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    source, out = args.source.resolve(), args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    for tool in ("git", "cmake", "arm-linux-gnueabi-gcc", "arm-linux-gnueabi-g++",
                 "arm-linux-gnueabi-ar", "arm-linux-gnueabi-ranlib", "llvm-nm"):
        if not shutil.which(tool):
            raise RuntimeError("required tool missing: " + tool)
    actual = run(["git", "-C", str(source), "rev-parse", "HEAD"]).strip()
    if actual != PIN:
        raise RuntimeError("mGBA source pin mismatch: " + actual)

    exports = " ".join("-D" + name + "=mgba_" + name for name in EXPORTS)
    cflags = (
        "-mcpu=cortex-a53 -marm -mfpu=neon-vfpv4 -mfloat-abi=softfp "
        "-fno-pic -fno-pie -fno-stack-protector -U_FORTIFY_SOURCE -D_FORTIFY_SOURCE=0 "
        "-ffunction-sections -fdata-sections " + exports
    )
    build = out / "cmake-generic-build"
    configure = [
        "cmake", "-S", str(source), "-B", str(build),
        "-DCMAKE_SYSTEM_NAME=Generic",
        "-DCMAKE_SYSTEM_PROCESSOR=arm",
        "-DCMAKE_C_COMPILER=" + shutil.which("arm-linux-gnueabi-gcc"),
        "-DCMAKE_CXX_COMPILER=" + shutil.which("arm-linux-gnueabi-g++"),
        "-DCMAKE_AR=" + shutil.which("arm-linux-gnueabi-ar"),
        "-DCMAKE_RANLIB=" + shutil.which("arm-linux-gnueabi-ranlib"),
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
    run(configure, out / "cmake-generic-configure.log")
    run(["cmake", "--build", str(build), "--target", "mgba_libretro", "--parallel", "2"],
        out / "cmake-generic-build.log")

    found = list(build.rglob("mgba_libretro.a"))
    if len(found) != 1:
        raise RuntimeError("expected one Generic mgba_libretro.a, found " + repr([str(p) for p in found]))
    archive = found[0]
    copied = out / "mgba_libretro-generic-aarch32-static.a"
    shutil.copy2(archive, copied)
    nm_defined = run(["llvm-nm", "-g", "--defined-only", str(copied)])
    nm_undefined = run(["llvm-nm", "-u", str(copied)])
    defined = names(nm_defined)
    undef = names(nm_undefined)
    missing = ["mgba_" + name for name in EXPORTS if "mgba_" + name not in defined]
    unprefixed = sorted(x for x in defined if re.match(r"^retro_[A-Za-z0-9_]+$", x))
    if missing or unprefixed:
        raise RuntimeError("API namespacing check failed; missing=" + repr(missing) +
                           " unprefixed=" + repr(unprefixed))
    external = sorted(undef - defined)
    internal = sorted(undef & defined)
    (out / "generic-defined-symbols.txt").write_text(nm_defined, encoding="utf-8")
    (out / "generic-undefined-raw.txt").write_text(nm_undefined, encoding="utf-8")
    (out / "generic-external-undefined.txt").write_text("\n".join(external) + "\n", encoding="utf-8")
    (out / "generic-internal-cross-object-refs.txt").write_text("\n".join(internal) + "\n", encoding="utf-8")
    report = [
        "# mGBA AArch32 Generic static Libretro archive\n\n",
        "Pinned mGBA commit: " + actual + "\n",
        "CMake platform: Generic (OS_DEFINES/OS_SRC not selected by the CMake platform branch).\n",
        "Compiler: arm-linux-gnueabi GCC; Cortex-A53 ARM, NEON/VFPv4, softfp.\n",
        "Build options: LIBMGBA_ONLY, BUILD_LIBRETRO, LIBRETRO_STATIC, M_CORE_GBA and M_CORE_GB.\n",
        "Archive bytes: " + str(copied.stat().st_size) + "\n",
        "Object members: " + str(len(run(["arm-linux-gnueabi-ar", "t", str(copied)]).splitlines())) + "\n",
        "Prefixed Libretro exports: " + str(len(EXPORTS)) + "/" + str(len(EXPORTS)) + "; no unprefixed retro_* globals.\n",
        "Undefined symbols: " + str(len(undef)) + " unique; " + str(len(internal)) +
        " resolve elsewhere in the archive; " + str(len(external)) + " are external.\n",
        "External list: generic-external-undefined.txt.\n\n",
        "Boundary: Generic is a CMake platform selection, not proof of a complete bare-metal port. The GCC driver still uses its arm-linux-gnueabi C headers/sysroot. No final LK link or runtime test was performed.\n"
    ]
    (out / "REPORT.md").write_text("".join(report), encoding="utf-8")
    print((out / "REPORT.md").read_text(encoding="utf-8"))
    print("External symbols:\n" + "\n".join(external))

if __name__ == "__main__":
    main()
