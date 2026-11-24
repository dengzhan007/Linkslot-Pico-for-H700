#!/usr/bin/env python3
"""Build pinned mGBA VFS v3 as AArch32 static archive with arm-none-eabi/newlib."""
from __future__ import annotations
import argparse
import re
import runpy
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
    "retro_unload_game", "retro_serialize_size", "retro_serialize", "retro_unserialize",
    "retro_cheat_reset", "retro_cheat_set", "retro_get_region",
    "retro_set_controller_port_device", "retro_load_game_special",
    "retro_get_memory_data", "retro_get_memory_size",
)

def run(args, log=None, cwd=None):
    p = subprocess.run(args, cwd=cwd, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    if log:
        log.parent.mkdir(parents=True, exist_ok=True)
        log.write_text(p.stdout, encoding="utf-8")
    if p.returncode:
        print(p.stdout[-20000:], file=sys.stderr)
        raise RuntimeError("command failed: " + " ".join(map(str, args)))
    return p.stdout

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--source", type=Path, required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--patch-script", type=Path, required=True)
    args = ap.parse_args()
    source, out = args.source.resolve(), args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)

    tools = (
        "git", "cmake", "arm-none-eabi-gcc", "arm-none-eabi-g++",
        "arm-none-eabi-ar", "arm-none-eabi-ranlib", "arm-none-eabi-nm",
        "arm-none-eabi-readelf",
    )
    for tool in tools:
        if not shutil.which(tool):
            raise RuntimeError("missing bare-metal toolchain component: " + tool)

    actual = run(["git", "-C", str(source), "rev-parse", "HEAD"]).strip()
    if actual != PIN:
        raise RuntimeError("mGBA source pin mismatch: " + actual)

    # The preceding VFS3 build step patches this checkout. If this script is
    # run independently against the pristine pinned checkout, apply the same
    # guarded temporary patch function once.
    vfs_c = source / "src/platform/libretro/libretro-vfs.c"
    custom_dir = source / "src/platform/libretro/libretro-vfs-dir.c"
    if not custom_dir.is_file() or "libretroVFSGetInterface" not in vfs_c.read_text(encoding="utf-8"):
        helpers = runpy.run_path(str(args.patch_script.resolve()))
        helpers["patch_tree"](source, out)

    vfs_cmake = (source / "CMakeLists.txt").read_text(encoding="utf-8")
    if "ENABLE_LIBRETRO_VFS_REQUIRED" not in vfs_cmake:
        raise RuntimeError("required Libretro VFS v3 CMake patch missing")
    if not custom_dir.is_file():
        raise RuntimeError("custom mGBA VFS v3 directory backend missing")

    exports = " ".join("-D" + x + "=mgba_" + x for x in EXPORTS)
    cflags = (
        "-mcpu=cortex-a53 -marm -mfpu=neon-vfpv4 -mfloat-abi=softfp "
        "-ffreestanding -fno-builtin -fno-pic -fno-pie -fno-stack-protector "
        "-ffunction-sections -fdata-sections " + exports
    )
    build = out / "cmake-arm-none-eabi-vfs3"
    configure = [
        "cmake", "-S", str(source), "-B", str(build),
        "-DCMAKE_SYSTEM_NAME=Generic",
        "-DCMAKE_SYSTEM_PROCESSOR=arm",
        "-DCMAKE_C_COMPILER=" + shutil.which("arm-none-eabi-gcc"),
        "-DCMAKE_CXX_COMPILER=" + shutil.which("arm-none-eabi-g++"),
        "-DCMAKE_AR=" + shutil.which("arm-none-eabi-ar"),
        "-DCMAKE_RANLIB=" + shutil.which("arm-none-eabi-ranlib"),
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
        "-DENABLE_LIBRETRO_VFS_REQUIRED=ON",
    ]
    run(configure, out / "cmake-arm-none-eabi-configure.log")
    run(["cmake", "--build", str(build), "--target", "mgba_libretro", "--parallel", "2"],
        out / "cmake-arm-none-eabi-build.log")

    archives = list(build.rglob("mgba_libretro.a"))
    if len(archives) != 1:
        raise RuntimeError("expected exactly one mGBA archive: " + repr([str(x) for x in archives]))
    archive = out / "mgba_libretro-vfs3-arm-none-eabi.a"
    shutil.copy2(archives[0], archive)
    members = run(["arm-none-eabi-ar", "t", str(archive)])
    member_names = set(members.splitlines())
    defined = run(["arm-none-eabi-nm", "-g", "--defined-only", str(archive)])
    undefined = run(["arm-none-eabi-nm", "-u", str(archive)])
    dnames = {x.split()[-1] for x in defined.splitlines() if len(x.split()) >= 2}
    unames = {x.split()[-1] for x in undefined.splitlines() if len(x.split()) >= 2}
    external = unames - dnames

    missing = ["mgba_" + x for x in EXPORTS if "mgba_" + x not in dnames]
    unprefixed = sorted(x for x in dnames if re.match(r"^retro_[A-Za-z0-9_]+$", x))
    required_backend = ["VDirOpen", "VDirCreate", "libretroVFSGetInterface"]
    missing_backend = [x for x in required_backend if x not in dnames]
    old_objects = sorted(x for x in member_names
                         if x.endswith("vfs-file.c.obj") or x.endswith("vfs-dirent.c.obj"))
    if missing or unprefixed or missing_backend or old_objects:
        raise RuntimeError("static VFS checks failed: missing=" + repr(missing) +
                           " unprefixed=" + repr(unprefixed) +
                           " backend=" + repr(missing_backend) +
                           " old objects=" + repr(old_objects))
    forbidden = sorted(external & {
        "opendir", "opendir64", "readdir", "readdir64", "closedir", "rewinddir",
        "stat", "stat64", "__stat64_time64", "mkdir", "remove",
        "fopen", "fopen64", "fread", "fwrite", "fseek", "ftell", "fflush", "fclose",
        "getcwd", "getenv", "open", "open64", "read", "write", "close", "lseek64",
        "mmap64", "msync", "munmap", "fsync", "ftruncate64",
    })
    if forbidden:
        raise RuntimeError("host/POSIX filesystem symbols remain: " + ", ".join(forbidden))

    (out / "defined-symbols.txt").write_text(defined, encoding="utf-8")
    (out / "undefined-raw.txt").write_text(undefined, encoding="utf-8")
    (out / "external-undefined.txt").write_text("\n".join(sorted(external)) + "\n", encoding="utf-8")
    (out / "object-members.txt").write_text(members, encoding="utf-8")

    # Extract one object and independently confirm that the archive contains
    # AArch32/ARM objects rather than relying only on compiler target flags.
    first = members.splitlines()[0]
    extract = out / "object-check"
    extract.mkdir(exist_ok=True)
    run(["arm-none-eabi-ar", "x", str(archive), first], cwd=extract)
    elf_header = run(["arm-none-eabi-readelf", "-h", str(extract / first)])
    if "ELF32" not in elf_header or "ARM" not in elf_header:
        raise RuntimeError("archive member is not ELF32/ARM: " + elf_header)

    report = [
        "# mGBA Libretro VFS v3 — arm-none-eabi/newlib build\n\n",
        "Pinned mGBA source: " + actual + "\n",
        "Toolchain: arm-none-eabi GCC/newlib; Cortex-A53 A32 ARM, NEON/VFPv4, softfp; freestanding flags enabled.\n",
        "VFS source patches: same as the Linux/GNU A32 VFS v3 archive; VFS v3 file + directory backend mandatory.\n",
        "Archive size: " + str(archive.stat().st_size) + " bytes; object members: " + str(len(member_names)) + ".\n",
        "First object header: ELF32/ARM verified with arm-none-eabi-readelf.\n",
        "Namespaced Libretro exports: " + str(len(EXPORTS)) + "/25; no unprefixed retro_*; VDirOpen/VDirCreate/interface accessor present.\n",
        "True unresolved external symbols: " + str(len(external)) + " after subtracting archive-global definitions.\n",
        "No guarded POSIX/stdio filesystem symbols remain.\n",
        "Exact external list: external-undefined.txt.\n\n",
        "Boundary: static archive only. No LK final link, no ROM execution, no H700 VFS backend and no device test. Newlib availability is not LK ABI compatibility.\n",
    ]
    (out / "REPORT.md").write_text("".join(report), encoding="utf-8")
    print((out / "REPORT.md").read_text(encoding="utf-8"))
    print("Bare-metal external symbols:\n" + "\n".join(sorted(external)))

if __name__ == "__main__":
    main()
