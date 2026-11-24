#!/usr/bin/env python3
"""Host regression test for the actual temporary mGBA Libretro VFS v3 VDir backend."""
from __future__ import annotations
import argparse
import shutil
import subprocess
import sys
from pathlib import Path

HARNESS = r'''#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "libretro-vfs.h"
#include <mgba-util/vfs.h>

struct retro_vfs_file_handle { char path[256]; };
struct retro_vfs_dir_handle {
    char path[256];
    unsigned index;
    const char* name;
    bool is_dir;
};

static unsigned open_dirs, close_dirs, remove_calls, mkdir_calls, file_open_calls;
static struct retro_vfs_dir_handle dir_handles[8];
static unsigned next_handle;
static const char* names[] = {".", "..", "one.sav", "folder", "two.state"};
static const bool types[] = {true, true, false, true, false};

struct TestVFile {
    struct VFile base;
    char path[256];
};
static struct TestVFile opened_file;

static struct retro_vfs_dir_handle* fake_opendir(const char* path, bool hidden) {
    struct retro_vfs_dir_handle* h;
    (void)hidden;
    if (!path || (strcmp(path, "card/Saves") && strcmp(path, "card/Saves/nested"))) return NULL;
    if (next_handle >= sizeof(dir_handles) / sizeof(dir_handles[0])) return NULL;
    h = &dir_handles[next_handle++];
    memset(h, 0, sizeof(*h));
    snprintf(h->path, sizeof(h->path), "%s", path);
    open_dirs++;
    return h;
}
static bool fake_readdir(struct retro_vfs_dir_handle* h) {
    if (!h) return false;
    if (h->index >= sizeof(names) / sizeof(names[0])) {
        h->name = NULL;
        return false;
    }
    h->name = names[h->index];
    h->is_dir = types[h->index];
    h->index++;
    return true;
}
static const char* fake_name(struct retro_vfs_dir_handle* h) { return h ? h->name : NULL; }
static bool fake_is_dir(struct retro_vfs_dir_handle* h) { return h && h->is_dir; }
static int fake_closedir(struct retro_vfs_dir_handle* h) {
    if (!h) return -1;
    close_dirs++;
    return 0;
}
static int fake_remove(const char* path) {
    if (!path || strcmp(path, "card/Saves/remove.sav")) return -1;
    remove_calls++;
    return 0;
}
static int fake_mkdir(const char* path) {
    if (!path || strcmp(path, "card/Saves/newdir")) return -1;
    mkdir_calls++;
    return 0;
}
static int fake_stat(const char* path, int32_t* size) {
    if (size) *size = 0;
    if (!path) return 0;
    if (!strcmp(path, "card/Saves/existing")) {
        return RETRO_VFS_STAT_IS_VALID | RETRO_VFS_STAT_IS_DIRECTORY;
    }
    return 0;
}
static struct retro_vfs_interface iface = {
    .get_path = NULL,
    .open = NULL,
    .close = NULL,
    .size = NULL,
    .tell = NULL,
    .seek = NULL,
    .read = NULL,
    .write = NULL,
    .flush = NULL,
    .remove = fake_remove,
    .rename = NULL,
    .truncate = NULL,
    .stat = fake_stat,
    .mkdir = fake_mkdir,
    .opendir = fake_opendir,
    .readdir = fake_readdir,
    .dirent_get_name = fake_name,
    .dirent_is_dir = fake_is_dir,
    .closedir = fake_closedir,
};

struct retro_vfs_interface* libretroVFSGetInterface(uint32_t* version) {
    if (version) *version = 3;
    return &iface;
}
struct VFile* VFileOpen(const char* path, int flags) {
    (void)flags;
    if (!path || !strcmp(path, "../escape.sav")) return NULL;
    snprintf(opened_file.path, sizeof(opened_file.path), "%s", path);
    file_open_calls++;
    return &opened_file.base;
}
struct VDir* VDirOpenArchive(const char* path) { (void)path; return NULL; }

#define CHECK(c, msg) do { if (!(c)) { fprintf(stderr, "FAIL: %s\n", msg); return __LINE__; } } while (0)
int main(void) {
    struct VDir* dir;
    struct VDirEntry* entry;
    struct VDir* nested;
    struct VFile* file;

    dir = VDirOpen("card/Saves");
    CHECK(dir != NULL, "VDirOpen root");
    entry = dir->listNext(dir);
    CHECK(entry && !strcmp(entry->name(entry), "one.sav"), "skip dot entries and return first file");
    CHECK(entry->type(entry) == VFS_FILE, "first entry file type");
    entry = dir->listNext(dir);
    CHECK(entry && !strcmp(entry->name(entry), "folder"), "second entry folder name");
    CHECK(entry->type(entry) == VFS_DIRECTORY, "second entry directory type");
    entry = dir->listNext(dir);
    CHECK(entry && !strcmp(entry->name(entry), "two.state"), "third entry file");
    CHECK(dir->listNext(dir) == NULL, "end of directory");
    CHECK(open_dirs == 1 && close_dirs == 0, "one root handle open");

    dir->rewind(dir);
    CHECK(open_dirs == 2 && close_dirs == 1, "rewind closes and reopens directory");
    entry = dir->listNext(dir);
    CHECK(entry && !strcmp(entry->name(entry), "one.sav"), "rewind resets enumeration");
    
    file = dir->openFile(dir, "game.sav", 0);
    CHECK(file == &opened_file.base, "openFile uses VFileOpen");
    CHECK(!strcmp(opened_file.path, "card/Saves/game.sav"), "openFile joins relative path");
    CHECK(file_open_calls == 1, "one file open");
    CHECK(dir->openFile(dir, "../escape.sav", 0) == NULL, "reject path traversal");
    CHECK(dir->openFile(dir, "/absolute.sav", 0) == NULL, "reject absolute child path");
    CHECK(dir->openFile(dir, "nested\\escape.sav", 0) == NULL, "reject backslash child path");
    CHECK(file_open_calls == 1, "rejected paths never reach backend");

    nested = dir->openDir(dir, "nested");
    CHECK(nested != NULL, "open nested directory via VDirOpen");
    CHECK(open_dirs == 3, "nested open uses VFS opendir");
    CHECK(nested->close(nested), "close nested directory");
    CHECK(close_dirs == 2, "nested close dispatched");

    CHECK(dir->deleteFile(dir, "remove.sav"), "deleteFile calls VFS remove");
    CHECK(remove_calls == 1, "remove callback invoked once");
    CHECK(!dir->deleteFile(dir, "../escape.sav"), "deleteFile rejects traversal");
    CHECK(remove_calls == 1, "rejected deletion never reaches backend");

    CHECK(VDirCreate("card/Saves/newdir"), "VDirCreate through VFS mkdir");
    CHECK(mkdir_calls == 1, "mkdir callback invoked");
    CHECK(VDirCreate("card/Saves/existing"), "VDirCreate recognizes existing directory through stat");

    CHECK(dir->close(dir), "close root directory");
    CHECK(close_dirs == 3, "root close dispatched");
    puts("PASS: mGBA VDir uses Libretro VFS v3 for iteration, rewind, nested dirs, file opens, delete and mkdir");
    return 0;
}
'''

def run(args, log):
    proc = subprocess.run(args, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    log.parent.mkdir(parents=True, exist_ok=True)
    log.write_text(proc.stdout, encoding="utf-8")
    if proc.returncode:
        print(proc.stdout[-12000:], file=sys.stderr)
        raise RuntimeError("command failed: " + " ".join(map(str, args)))
    return proc.stdout

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--source", type=Path, required=True,
                    help="the mGBA checkout already patched by build-mgba-libretro-vfs3-static.py")
    ap.add_argument("--out", type=Path, required=True)
    args = ap.parse_args()
    source, out = args.source.resolve(), args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    for tool in ("cc",):
        if not shutil.which(tool):
            raise RuntimeError("missing host compiler: " + tool)

    adapter = source / "src/platform/libretro/libretro-vfs-dir.c"
    vfs_header = source / "src/platform/libretro/libretro-vfs.h"
    if not adapter.is_file() or not vfs_header.is_file():
        raise RuntimeError("mGBA temporary VFS v3 source patch missing; run the VFS3 build first")
    harness = out / "vdir-vfs3-host-test.c"
    harness.write_text(HARNESS, encoding="utf-8")
    adapter_obj = out / "libretro-vfs-dir.o"
    harness_obj = out / "vdir-vfs3-host-test.o"
    include = source / "include"
    platform = source / "src/platform/libretro"
    flags = [
        "-std=c11", "-DENABLE_VFS=1", "-DENABLE_DIRECTORIES=1",
        "-I", str(include), "-I", str(platform),
        "-Wall", "-Wextra",
    ]
    run(["cc", *flags, "-c", str(adapter), "-o", str(adapter_obj)],
        out / "adapter-compile.log")
    run(["cc", *flags, "-c", str(harness), "-o", str(harness_obj)],
        out / "harness-compile.log")
    exe = out / "vdir-vfs3-host-test"
    run(["cc", str(adapter_obj), str(harness_obj), "-o", str(exe)],
        out / "host-link.log")
    output = run([str(exe)], out / "host-run.log")
    report = (
        "# mGBA VDir Libretro VFS v3 host regression\n\n"
        "The test compiles the actual temporary mGBA libretro-vfs-dir.c and invokes its public VDir callbacks against a fake Libretro VFS v3 backend.\n\n"
        "Coverage: dot-entry filtering, file/directory names and types, EOF, rewind close/reopen, nested VDirOpen, VFileOpen path join, path traversal/absolute/backslash rejection, VFS remove, mkdir and existing-directory stat fallback.\n\n"
        "Result: PASS (host process exit 0).\n\n"
        "Boundary: fake in-memory directory service only; no H700 SD/FAT service or ROM I/O.\n"
    )
    (out / "REPORT.md").write_text(report, encoding="utf-8")
    print(report)
    print(output)

if __name__ == "__main__":
    main()
