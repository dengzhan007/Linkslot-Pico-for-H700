#!/usr/bin/env python3
"""Build pinned mGBA with Libretro VFS v3 replacing stdio and POSIX dirent."""
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
    "retro_unload_game", "retro_serialize_size", "retro_serialize", "retro_unserialize",
    "retro_cheat_reset", "retro_cheat_set", "retro_get_region",
    "retro_set_controller_port_device", "retro_load_game_special",
    "retro_get_memory_data", "retro_get_memory_size",
)

def run(args, log=None):
    p = subprocess.run(args, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    if log:
        log.parent.mkdir(parents=True, exist_ok=True)
        log.write_text(p.stdout, encoding="utf-8")
    if p.returncode:
        print(p.stdout[-20000:], file=sys.stderr)
        raise RuntimeError("command failed: " + " ".join(map(str, args)))
    return p.stdout

def replace_once(path, old, new):
    data = path.read_text(encoding="utf-8")
    if data.count(old) != 1:
        raise RuntimeError(f"{path}: expected one patch anchor, found {data.count(old)}")
    path.write_text(data.replace(old, new), encoding="utf-8")

def patch_tree(source, out):
    vfs_c = source / "src/platform/libretro/libretro-vfs.c"
    replace_once(vfs_c, """\tstruct retro_vfs_interface_info info = {
\t\t.required_interface_version = 2,
\t\t.iface = NULL,
\t};
\tif (!env(RETRO_ENVIRONMENT_GET_VFS_INTERFACE, &info)) {
\t\tinfo.required_interface_version = 1;
\t\tinfo.iface = NULL;
\t\tif (!env(RETRO_ENVIRONMENT_GET_VFS_INTERFACE, &info)) {
\t\t\t// Keep any interface we already have; probe callbacks fail this
\t\t\t// query on a running core
\t\t\treturn;
\t\t}
\t}
""", """\tstruct retro_vfs_interface_info info = {
\t\t.required_interface_version = 3,
\t\t.iface = NULL,
\t};
\tif (!env(RETRO_ENVIRONMENT_GET_VFS_INTERFACE, &info)) {
\t\tinfo.required_interface_version = 2;
\t\tinfo.iface = NULL;
\t\tif (!env(RETRO_ENVIRONMENT_GET_VFS_INTERFACE, &info)) {
\t\t\tinfo.required_interface_version = 1;
\t\t\tinfo.iface = NULL;
\t\t\tif (!env(RETRO_ENVIRONMENT_GET_VFS_INTERFACE, &info)) {
\t\t\t\t// Keep any interface we already have; probe callbacks fail this
\t\t\t\t// query on a running core
\t\t\t\treturn;
\t\t\t}
\t\t}
\t}
""")
    replace_once(vfs_c, """\t_vfs = info.iface;
\t_vfsVersion = info.required_interface_version;
}

""", """\t_vfs = info.iface;
\t_vfsVersion = info.required_interface_version;
}

struct retro_vfs_interface* libretroVFSGetInterface(uint32_t* version) {
\tif (version) *version = _vfsVersion;
\tif (!_vfs || _vfsVersion < 3) return NULL;
\treturn _vfs;
}

""")
    replace_once(source / "src/platform/libretro/libretro-vfs.h",
        """void libretroVFSInit(retro_environment_t env);
struct VFile* VFileOpenLibretro(const char* path, int flags);
""",
        """void libretroVFSInit(retro_environment_t env);
struct retro_vfs_interface* libretroVFSGetInterface(uint32_t* version);
struct VFile* VFileOpenLibretro(const char* path, int flags);
""")
    replace_once(source / "src/util/vfs.c",
        """#elif defined(ENABLE_VFS_FD)
\treturn VFileOpenFD(path, flags);
#else
#error "Can't build VFS subsystem without a VFile backend"
""",
        """#elif defined(ENABLE_VFS_FD)
\treturn VFileOpenFD(path, flags);
#elif defined(ENABLE_LIBRETRO_VFS_REQUIRED)
\treturn NULL;
#else
#error "Can't build VFS subsystem without a VFile backend"
""")

    directory_c = r'''/* mGBA VDir backend using the frontend's Libretro VFS v3. */
#include "libretro-vfs.h"
#include <mgba-util/vfs.h>
#include <mgba-util/string.h>
#include <fcntl.h>
#include <stddef.h>
#include <stdlib.h>
#include <string.h>

#if defined(ENABLE_VFS) && defined(ENABLE_DIRECTORIES)
struct VDirRetro {
\tstruct VDir d;
\tstruct VDirEntry entry;
\tstruct retro_vfs_interface* vfs;
\tstruct retro_vfs_dir_handle* handle;
\tchar* path;
\tconst char* currentName;
\tbool currentIsDir;
};
static bool vdr_close(struct VDir*);
static void vdr_rewind(struct VDir*);
static struct VDirEntry* vdr_list_next(struct VDir*);
static struct VFile* vdr_open_file(struct VDir*, const char*, int);
static struct VDir* vdr_open_dir(struct VDir*, const char*);
static bool vdr_delete_file(struct VDir*, const char*);
static const char* vdr_entry_name(struct VDirEntry*);
static enum VFSType vdr_entry_type(struct VDirEntry*);

static char* vdr_join(const char* base, const char* child) {
\tsize_t bl, cl, sep;
\tchar* out;
\tif (!base || !child || !child[0] || child[0] == '/' || strchr(child, '\\')) return NULL;
\tif (!strcmp(child, "..") || !strncmp(child, "../", 3) ||
\t    strstr(child, "/../") || (strlen(child) >= 3 && !strcmp(child + strlen(child) - 3, "/.."))) return NULL;
\tbl = strlen(base);
\tcl = strlen(child);
\tsep = bl && base[bl - 1] != '/' ? 1 : 0;
\tout = malloc(bl + sep + cl + 1);
\tif (!out) return NULL;
\tmemcpy(out, base, bl);
\tif (sep) out[bl] = '/';
\tmemcpy(out + bl + sep, child, cl + 1);
\treturn out;
}
static bool vdr_close(struct VDir* dir) {
\tstruct VDirRetro* r = (struct VDirRetro*) dir;
\tint result = r->handle ? r->vfs->closedir(r->handle) : 0;
\tfree(r->path);
\tfree(r);
\treturn result == 0;
}
static void vdr_rewind(struct VDir* dir) {
\tstruct VDirRetro* r = (struct VDirRetro*) dir;
\tif (r->handle) {
\t\tr->vfs->closedir(r->handle);
\t\tr->handle = NULL;
\t}
\tr->currentName = NULL;
\tr->currentIsDir = false;
\tif (r->path) r->handle = r->vfs->opendir(r->path, false);
}
static struct VDirEntry* vdr_list_next(struct VDir* dir) {
\tstruct VDirRetro* r = (struct VDirRetro*) dir;
\tif (!r->handle) return NULL;
\twhile (r->vfs->readdir(r->handle)) {
\t\tconst char* name = r->vfs->dirent_get_name(r->handle);
\t\tif (!name || !name[0] || !strcmp(name, ".") || !strcmp(name, "..")) continue;
\t\tr->currentName = name;
\t\tr->currentIsDir = r->vfs->dirent_is_dir(r->handle);
\t\treturn &r->entry;
\t}
\tr->currentName = NULL;
\treturn NULL;
}
static struct VDirRetro* vdr_parent(struct VDirEntry* entry) {
\treturn (struct VDirRetro*)((char*)entry - offsetof(struct VDirRetro, entry));
}
static const char* vdr_entry_name(struct VDirEntry* entry) {
\treturn vdr_parent(entry)->currentName;
}
static enum VFSType vdr_entry_type(struct VDirEntry* entry) {
\treturn vdr_parent(entry)->currentIsDir ? VFS_DIRECTORY : VFS_FILE;
}
static struct VFile* vdr_open_file(struct VDir* dir, const char* name, int mode) {
\tstruct VDirRetro* r = (struct VDirRetro*) dir;
\tchar* path = vdr_join(r->path, name);
\tstruct VFile* file;
\tif (!path) return NULL;
\tfile = VFileOpen(path, mode);
\tfree(path);
\treturn file;
}
static struct VDir* vdr_open_dir(struct VDir* dir, const char* name) {
\tstruct VDirRetro* r = (struct VDirRetro*) dir;
\tchar* path = vdr_join(r->path, name);
\tstruct VDir* child;
\tif (!path) return NULL;
\tchild = VDirOpen(path);
\tif (!child) child = VDirOpenArchive(path);
\tfree(path);
\treturn child;
}
static bool vdr_delete_file(struct VDir* dir, const char* name) {
\tstruct VDirRetro* r = (struct VDirRetro*) dir;
\tchar* path = vdr_join(r->path, name);
\tint result;
\tif (!path) return false;
\tresult = r->vfs->remove(path);
\tfree(path);
\treturn result == 0;
}
struct VDir* VDirOpen(const char* path) {
\tuint32_t version = 0;
\tstruct retro_vfs_interface* vfs = libretroVFSGetInterface(&version);
\tstruct retro_vfs_dir_handle* handle;
\tstruct VDirRetro* r;
\tif (!path || !vfs || version < 3) return NULL;
\thandle = vfs->opendir(path, false);
\tif (!handle) return NULL;
\tr = calloc(1, sizeof(*r));
\tif (!r) {
\t\tvfs->closedir(handle);
\t\treturn NULL;
\t}
\tr->path = strdup(path);
\tif (!r->path) {
\t\tvfs->closedir(handle);
\t\tfree(r);
\t\treturn NULL;
\t}
\tr->vfs = vfs;
\tr->handle = handle;
\tr->d.close = vdr_close;
\tr->d.rewind = vdr_rewind;
\tr->d.listNext = vdr_list_next;
\tr->d.openFile = vdr_open_file;
\tr->d.openDir = vdr_open_dir;
\tr->d.deleteFile = vdr_delete_file;
\tr->entry.name = vdr_entry_name;
\tr->entry.type = vdr_entry_type;
\treturn &r->d;
}
bool VDirCreate(const char* path) {
\tuint32_t version = 0;
\tstruct retro_vfs_interface* vfs = libretroVFSGetInterface(&version);
\tint32_t size = 0;
\tint result;
\tif (!path || !vfs || version < 3) return false;
\tresult = vfs->mkdir(path);
\tif (result == 0) return true;
\tresult = vfs->stat(path, &size);
\treturn (result & RETRO_VFS_STAT_IS_VALID) && (result & RETRO_VFS_STAT_IS_DIRECTORY);
}
#endif
'''
    directory_c = directory_c.replace("\\t", "\t")
    (source / "src/platform/libretro/libretro-vfs-dir.c").write_text(directory_c, encoding="utf-8")

    current_source = "$" + "{CMAKE_CURRENT_SOURCE_DIR}"
    old_cmake = (
        "\t\telse()\n"
        "\t\t\tlist(APPEND LIBRETRO_CORE_SRC " + current_source + "/src/util/vfs/vfs-file.c)\n"
        "\t\t\tlist(APPEND LIBRETRO_CORE_SRC " + current_source + "/src/util/vfs/vfs-dirent.c)\n"
        "\t\t\tlist(APPEND LIBRETRO_EXTRA_DEFINES ENABLE_VFS_FILE)\n"
        "\t\tendif()\n"
    )
    new_cmake = (
        "\t\telseif(ENABLE_LIBRETRO_VFS_REQUIRED)\n"
        "\t\t\tlist(FILTER LIBRETRO_CORE_SRC EXCLUDE REGEX \"/util/vfs/vfs-dirent\\\\.c$\")\n"
        "\t\t\tlist(FILTER LIBRETRO_CORE_SRC EXCLUDE REGEX \"/util/vfs/vfs-file\\\\.c$\")\n"
        "\t\t\tlist(APPEND LIBRETRO_EXTRA_DEFINES ENABLE_LIBRETRO_VFS_REQUIRED)\n"
        "\t\telse()\n"
        "\t\t\tlist(APPEND LIBRETRO_CORE_SRC " + current_source + "/src/util/vfs/vfs-file.c)\n"
        "\t\t\tlist(APPEND LIBRETRO_CORE_SRC " + current_source + "/src/util/vfs/vfs-dirent.c)\n"
        "\t\t\tlist(APPEND LIBRETRO_EXTRA_DEFINES ENABLE_VFS_FILE)\n"
        "\t\tendif()\n"
    )
    replace_once(source / "CMakeLists.txt", old_cmake, new_cmake)
    (out / "source-patches.txt").write_text(
        "Pinned mGBA commit: " + PIN + "\n"
        "Requests Libretro VFS v3 with v2/v1 fallback and exposes the negotiated interface.\n"
        "Adds VDirOpen/VDirCreate based on v3 callbacks; rewind uses close/reopen of stored path.\n"
        "Removes vfs-file.c and vfs-dirent.c from this experimental build; VFileOpen requires frontend VFS.\n"
        "Only the CI checkout is patched; Linkslot snapshot and upstream repository are unchanged.\n",
        encoding="utf-8"
    )

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--source", type=Path, required=True)
    ap.add_argument("--out", type=Path, required=True)
    args = ap.parse_args()
    source, out = args.source.resolve(), args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    for tool in ("git", "cmake", "arm-linux-gnueabi-gcc", "arm-linux-gnueabi-g++",
                 "arm-linux-gnueabi-ar", "arm-linux-gnueabi-ranlib", "llvm-nm"):
        if not shutil.which(tool):
            raise RuntimeError("missing tool: " + tool)
    actual = run(["git", "-C", str(source), "rev-parse", "HEAD"]).strip()
    if actual != PIN:
        raise RuntimeError("mGBA source pin mismatch: " + actual)
    patch_tree(source, out)

    exports = " ".join("-D" + x + "=mgba_" + x for x in EXPORTS)
    cflags = ("-mcpu=cortex-a53 -marm -mfpu=neon-vfpv4 -mfloat-abi=softfp "
              "-fno-pic -fno-pie -fno-stack-protector -U_FORTIFY_SOURCE -D_FORTIFY_SOURCE=0 "
              "-ffunction-sections -fdata-sections " + exports)
    build = out / "cmake-libretro-vfs3-build"
    configure = [
        "cmake", "-S", str(source), "-B", str(build),
        "-DCMAKE_SYSTEM_NAME=Generic", "-DCMAKE_SYSTEM_PROCESSOR=arm",
        "-DCMAKE_C_COMPILER=" + shutil.which("arm-linux-gnueabi-gcc"),
        "-DCMAKE_CXX_COMPILER=" + shutil.which("arm-linux-gnueabi-g++"),
        "-DCMAKE_AR=" + shutil.which("arm-linux-gnueabi-ar"),
        "-DCMAKE_RANLIB=" + shutil.which("arm-linux-gnueabi-ranlib"),
        "-DCMAKE_TRY_COMPILE_TARGET_TYPE=STATIC_LIBRARY", "-DCMAKE_BUILD_TYPE=Release",
        "-DCMAKE_C_FLAGS=" + cflags, "-DLIBMGBA_ONLY=ON", "-DBUILD_LIBRETRO=ON",
        "-DLIBRETRO_STATIC=ON", "-DBUILD_STATIC=ON", "-DBUILD_SHARED=OFF",
        "-DDISABLE_DEPS=ON", "-DM_CORE_GBA=ON", "-DM_CORE_GB=ON",
        "-DBUILD_TEST=OFF", "-DBUILD_SUITE=OFF", "-DBUILD_QT=OFF", "-DBUILD_SDL=OFF",
        "-DENABLE_LIBRETRO_VFS_REQUIRED=ON",
    ]
    run(configure, out / "cmake-vfs3-configure.log")
    run(["cmake", "--build", str(build), "--target", "mgba_libretro", "--parallel", "2"],
        out / "cmake-vfs3-build.log")

    archives = list(build.rglob("mgba_libretro.a"))
    if len(archives) != 1:
        raise RuntimeError("expected one VFS3 archive: " + repr([str(x) for x in archives]))
    archive = out / "mgba_libretro-vfs3-aarch32-static.a"
    shutil.copy2(archives[0], archive)
    members = run(["arm-linux-gnueabi-ar", "t", str(archive)])
    defined = run(["llvm-nm", "-g", "--defined-only", str(archive)])
    undefined = run(["llvm-nm", "-u", str(archive)])
    dnames = {x.split()[-1] for x in defined.splitlines() if len(x.split()) >= 2}
    unames = {x.split()[-1] for x in undefined.splitlines() if len(x.split()) >= 2}
    missing = ["mgba_" + x for x in EXPORTS if "mgba_" + x not in dnames]
    unprefixed = sorted(x for x in dnames if re.match(r"^retro_[A-Za-z0-9_]+$", x))
    if missing or unprefixed:
        raise RuntimeError("Libretro exports check failed: missing=" + repr(missing) + " unprefixed=" + repr(unprefixed))
    forbidden = sorted(unames & {
        "opendir", "readdir", "closedir", "rewinddir", "stat", "mkdir", "remove",
        "fopen", "fopen64", "fread", "fwrite", "fseek", "ftell", "fflush", "fclose"
    })
    if forbidden:
        raise RuntimeError("POSIX dirent/stdio symbols remain: " + ", ".join(forbidden))

    (out / "vfs3-defined-symbols.txt").write_text(defined, encoding="utf-8")
    (out / "vfs3-undefined-raw.txt").write_text(undefined, encoding="utf-8")
    (out / "vfs3-external-undefined.txt").write_text("\n".join(sorted(unames)) + "\n", encoding="utf-8")
    (out / "vfs3-object-members.txt").write_text(members, encoding="utf-8")
    report = [
        "# mGBA AArch32 static archive using Libretro VFS v3\n\n",
        "Pinned source: libretro/mgba @ " + actual + "\n",
        "Target: Cortex-A53 A32 ARM, NEON/VFPv4, softfp ABI; CMake Generic.\n",
        "mGBA requests VFS v3 (falls back to v2/v1); VFileOpen requires the frontend VFS; VDirOpen/VDirCreate use VFS v3.\n",
        "Archive size: " + str(archive.stat().st_size) + " bytes; object members: " + str(len(members.splitlines())) + ".\n",
        "Namespaced Libretro exports: " + str(len(EXPORTS)) + "/25; no unprefixed retro_* globals.\n",
        "Unique external undefined symbols: " + str(len(unames)) + ".\n",
        "POSIX dirent/stdio fallback symbols: 0 (guarded list checked).\n",
        "Full undefined list: vfs3-external-undefined.txt.\n\n",
        "Boundary: archive/dependency proof only. No final LK link, no actual ROM run, and no persistent H700 storage backend.\n",
    ]
    (out / "REPORT.md").write_text("".join(report), encoding="utf-8")
    print((out / "REPORT.md").read_text(encoding="utf-8"))
    print("External symbols:\n" + "\n".join(sorted(unames)))

if __name__ == "__main__":
    main()
