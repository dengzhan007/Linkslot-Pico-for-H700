#!/usr/bin/env python3
"""Host-testable no_std Libretro VFS v3 frontend registry and A32/A64 link probe."""
from __future__ import annotations
import argparse
import os
from pathlib import Path
import shutil
import subprocess
import sys
import textwrap

TARGETS = {
    "armv7a-none-eabi": ("armv7a-none-eabi", "armelf", "ARM", "cortex-a53"),
    "aarch64-unknown-none": ("aarch64-none-elf", "aarch64elf", "AArch64", "cortex-a53"),
}
RUST = r'''#![no_std]
use core::ffi::{c_char, c_int, c_uint, c_void};
use core::panic::PanicInfo;
use core::ptr;

pub const GET_VFS_INTERFACE: c_uint = 45 | 0x10000;
pub const FILE_READ: c_uint = 1;
pub const FILE_WRITE: c_uint = 2;
pub const FILE_READ_WRITE: c_uint = FILE_READ | FILE_WRITE;
pub const FILE_UPDATE_EXISTING: c_uint = 4;
pub const SEEK_START: c_int = 0;
pub const SEEK_CURRENT: c_int = 1;
pub const SEEK_END: c_int = 2;
pub const STAT_VALID: c_int = 1;
pub const STAT_DIRECTORY: c_int = 2;
pub const STAT_CHARACTER_SPECIAL: c_int = 4;

#[repr(C)]
pub struct VfsFileHandle { _opaque: [u8; 0] }
#[repr(C)]
pub struct VfsDirHandle { _opaque: [u8; 0] }

pub type GetPathFn = unsafe extern "C" fn(*mut VfsFileHandle) -> *const c_char;
pub type OpenFn = unsafe extern "C" fn(*const c_char, c_uint, c_uint) -> *mut VfsFileHandle;
pub type CloseFn = unsafe extern "C" fn(*mut VfsFileHandle) -> c_int;
pub type SizeFn = unsafe extern "C" fn(*mut VfsFileHandle) -> i64;
pub type TruncateFn = unsafe extern "C" fn(*mut VfsFileHandle, i64) -> i64;
pub type TellFn = unsafe extern "C" fn(*mut VfsFileHandle) -> i64;
pub type SeekFn = unsafe extern "C" fn(*mut VfsFileHandle, i64, c_int) -> i64;
pub type ReadFn = unsafe extern "C" fn(*mut VfsFileHandle, *mut c_void, u64) -> i64;
pub type WriteFn = unsafe extern "C" fn(*mut VfsFileHandle, *const c_void, u64) -> i64;
pub type FlushFn = unsafe extern "C" fn(*mut VfsFileHandle) -> c_int;
pub type PathFn = unsafe extern "C" fn(*const c_char) -> c_int;
pub type RenameFn = unsafe extern "C" fn(*const c_char, *const c_char) -> c_int;
pub type StatFn = unsafe extern "C" fn(*const c_char, *mut i32) -> c_int;
pub type MkdirFn = unsafe extern "C" fn(*const c_char) -> c_int;
pub type OpenDirFn = unsafe extern "C" fn(*const c_char, bool) -> *mut VfsDirHandle;
pub type ReadDirFn = unsafe extern "C" fn(*mut VfsDirHandle) -> bool;
pub type DirNameFn = unsafe extern "C" fn(*mut VfsDirHandle) -> *const c_char;
pub type DirIsDirFn = unsafe extern "C" fn(*mut VfsDirHandle) -> bool;

#[repr(C)]
pub struct VfsInterface {
    pub get_path: Option<GetPathFn>,
    pub open: Option<OpenFn>,
    pub close: Option<CloseFn>,
    pub size: Option<SizeFn>,
    pub tell: Option<TellFn>,
    pub seek: Option<SeekFn>,
    pub read: Option<ReadFn>,
    pub write: Option<WriteFn>,
    pub flush: Option<FlushFn>,
    pub remove: Option<PathFn>,
    pub rename: Option<RenameFn>,
    pub truncate: Option<TruncateFn>,
    pub stat: Option<StatFn>,
    pub mkdir: Option<MkdirFn>,
    pub opendir: Option<OpenDirFn>,
    pub readdir: Option<ReadDirFn>,
    pub dirent_get_name: Option<DirNameFn>,
    pub dirent_is_dir: Option<DirIsDirFn>,
    pub closedir: Option<CloseFnDir>,
}
pub type CloseFnDir = unsafe extern "C" fn(*mut VfsDirHandle) -> c_int;

#[repr(C)]
pub struct VfsInterfaceInfo {
    pub required_interface_version: u32,
    pub iface: *mut VfsInterface,
}

#[repr(C)]
pub struct VfsBackend {
    pub context: *mut c_void,
    pub get_path: Option<GetPathFn>,
    pub open: Option<OpenFn>,
    pub close: Option<CloseFn>,
    pub size: Option<SizeFn>,
    pub tell: Option<TellFn>,
    pub seek: Option<SeekFn>,
    pub read: Option<ReadFn>,
    pub write: Option<WriteFn>,
    pub flush: Option<FlushFn>,
    pub remove: Option<PathFn>,
    pub rename: Option<RenameFn>,
    pub truncate: Option<TruncateFn>,
    pub stat: Option<StatFn>,
    pub mkdir: Option<MkdirFn>,
    pub opendir: Option<OpenDirFn>,
    pub readdir: Option<ReadDirFn>,
    pub dirent_get_name: Option<DirNameFn>,
    pub dirent_is_dir: Option<DirIsDirFn>,
    pub closedir: Option<CloseFnDir>,
}
static mut BACKEND: *const VfsBackend = ptr::null();

static mut INTERFACE: VfsInterface = VfsInterface {
    get_path: Some(vfs_get_path),
    open: Some(vfs_open),
    close: Some(vfs_close),
    size: Some(vfs_size),
    tell: Some(vfs_tell),
    seek: Some(vfs_seek),
    read: Some(vfs_read),
    write: Some(vfs_write),
    flush: Some(vfs_flush),
    remove: Some(vfs_remove),
    rename: Some(vfs_rename),
    truncate: Some(vfs_truncate),
    stat: Some(vfs_stat),
    mkdir: Some(vfs_mkdir),
    opendir: Some(vfs_opendir),
    readdir: Some(vfs_readdir),
    dirent_get_name: Some(vfs_dirent_get_name),
    dirent_is_dir: Some(vfs_dirent_is_dir),
    closedir: Some(vfs_closedir),
};

unsafe fn backend() -> Option<&'static VfsBackend> {
    let ptr = BACKEND;
    if ptr.is_null() { None } else { Some(&*ptr) }
}

#[no_mangle]
pub unsafe extern "C" fn linkslot_vfs_install(backend_ptr: *const VfsBackend) -> bool {
    if backend_ptr.is_null() { return false; }
    let b = &*backend_ptr;
    // Require a complete VFS v3 backend. Do not return a partial interface
    // whose missing callbacks would later be dereferenced by a core.
    if b.open.is_none() || b.close.is_none() || b.size.is_none() ||
       b.tell.is_none() || b.seek.is_none() || b.read.is_none() ||
       b.write.is_none() || b.flush.is_none() || b.remove.is_none() ||
       b.rename.is_none() || b.truncate.is_none() || b.stat.is_none() ||
       b.mkdir.is_none() || b.opendir.is_none() || b.readdir.is_none() ||
       b.dirent_get_name.is_none() || b.dirent_is_dir.is_none() ||
       b.closedir.is_none() {
        return false;
    }
    BACKEND = backend_ptr;
    true
}

#[no_mangle]
pub unsafe extern "C" fn linkslot_vfs_environment(cmd: c_uint, data: *mut c_void) -> bool {
    if cmd != GET_VFS_INTERFACE || data.is_null() || backend().is_none() {
        return false;
    }
    let info = &mut *(data as *mut VfsInterfaceInfo);
    if info.required_interface_version > 3 {
        return false;
    }
    info.required_interface_version = 3;
    info.iface = ptr::addr_of_mut!(INTERFACE);
    true
}

unsafe extern "C" fn vfs_get_path(h: *mut VfsFileHandle) -> *const c_char {
    if h.is_null() { return ptr::null(); }
    backend().and_then(|b| b.get_path.map(|f| f(h))).unwrap_or(ptr::null())
}
unsafe extern "C" fn vfs_open(p: *const c_char, mode: c_uint, hints: c_uint) -> *mut VfsFileHandle {
    if p.is_null() || mode & (FILE_READ | FILE_WRITE) == 0 ||
       mode & !(FILE_READ | FILE_WRITE | FILE_UPDATE_EXISTING) != 0 ||
       (mode & FILE_UPDATE_EXISTING != 0 && mode & FILE_WRITE == 0) {
        return ptr::null_mut();
    }
    backend().and_then(|b| b.open.map(|f| f(p, mode, hints))).unwrap_or(ptr::null_mut())
}
unsafe extern "C" fn vfs_close(h: *mut VfsFileHandle) -> c_int {
    if h.is_null() { return -1; }
    backend().and_then(|b| b.close.map(|f| f(h))).unwrap_or(-1)
}
unsafe extern "C" fn vfs_size(h: *mut VfsFileHandle) -> i64 {
    if h.is_null() { return -1; }
    backend().and_then(|b| b.size.map(|f| f(h))).unwrap_or(-1)
}
unsafe extern "C" fn vfs_truncate(h: *mut VfsFileHandle, n: i64) -> i64 {
    if h.is_null() || n < 0 { return -1; }
    backend().and_then(|b| b.truncate.map(|f| f(h,n))).unwrap_or(-1)
}
unsafe extern "C" fn vfs_tell(h: *mut VfsFileHandle) -> i64 {
    if h.is_null() { return -1; }
    backend().and_then(|b| b.tell.map(|f| f(h))).unwrap_or(-1)
}
unsafe extern "C" fn vfs_seek(h: *mut VfsFileHandle, n: i64, w: c_int) -> i64 {
    if h.is_null() || !(SEEK_START..=SEEK_END).contains(&w) { return -1; }
    backend().and_then(|b| b.seek.map(|f| f(h,n,w))).unwrap_or(-1)
}
unsafe extern "C" fn vfs_read(h: *mut VfsFileHandle, p: *mut c_void, n: u64) -> i64 {
    if h.is_null() || (p.is_null() && n != 0) { return -1; }
    backend().and_then(|b| b.read.map(|f| f(h,p,n))).unwrap_or(-1)
}
unsafe extern "C" fn vfs_write(h: *mut VfsFileHandle, p: *const c_void, n: u64) -> i64 {
    if h.is_null() || (p.is_null() && n != 0) { return -1; }
    backend().and_then(|b| b.write.map(|f| f(h,p,n))).unwrap_or(-1)
}
unsafe extern "C" fn vfs_flush(h: *mut VfsFileHandle) -> c_int {
    if h.is_null() { return -1; }
    backend().and_then(|b| b.flush.map(|f| f(h))).unwrap_or(-1)
}
unsafe extern "C" fn vfs_remove(p: *const c_char) -> c_int {
    if p.is_null() { return -1; }
    backend().and_then(|b| b.remove.map(|f| f(p))).unwrap_or(-1)
}
unsafe extern "C" fn vfs_rename(a: *const c_char, bpath: *const c_char) -> c_int {
    if a.is_null() || bpath.is_null() { return -1; }
    backend().and_then(|b| b.rename.map(|f| f(a,bpath))).unwrap_or(-1)
}
unsafe extern "C" fn vfs_stat(p: *const c_char, n: *mut i32) -> c_int {
    if p.is_null() { return 0; }
    backend().and_then(|b| b.stat.map(|f| f(p,n))).unwrap_or(0)
}
unsafe extern "C" fn vfs_mkdir(p: *const c_char) -> c_int {
    if p.is_null() { return -1; }
    backend().and_then(|b| b.mkdir.map(|f| f(p))).unwrap_or(-1)
}
unsafe extern "C" fn vfs_opendir(p: *const c_char, hidden: bool) -> *mut VfsDirHandle {
    if p.is_null() { return ptr::null_mut(); }
    backend().and_then(|b| b.opendir.map(|f| f(p,hidden))).unwrap_or(ptr::null_mut())
}
unsafe extern "C" fn vfs_readdir(h: *mut VfsDirHandle) -> bool {
    if h.is_null() { return false; }
    backend().and_then(|b| b.readdir.map(|f| f(h))).unwrap_or(false)
}
unsafe extern "C" fn vfs_dirent_get_name(h: *mut VfsDirHandle) -> *const c_char {
    if h.is_null() { return ptr::null(); }
    backend().and_then(|b| b.dirent_get_name.map(|f| f(h))).unwrap_or(ptr::null())
}
unsafe extern "C" fn vfs_dirent_is_dir(h: *mut VfsDirHandle) -> bool {
    if h.is_null() { return false; }
    backend().and_then(|b| b.dirent_is_dir.map(|f| f(h))).unwrap_or(false)
}
unsafe extern "C" fn vfs_closedir(h: *mut VfsDirHandle) -> c_int {
    if h.is_null() { return -1; }
    backend().and_then(|b| b.closedir.map(|f| f(h))).unwrap_or(-1)
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! { loop { core::hint::spin_loop(); } }
'''

C_HEADER = r'''#ifndef LINK_SLOT_VFS_ABI_H
#define LINK_SLOT_VFS_ABI_H
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
typedef struct retro_vfs_file_handle retro_vfs_file_handle;
typedef struct retro_vfs_dir_handle retro_vfs_dir_handle;
typedef const char *(*get_path_t)(retro_vfs_file_handle *);
typedef retro_vfs_file_handle *(*open_t)(const char *, unsigned, unsigned);
typedef int (*close_t)(retro_vfs_file_handle *);
typedef int64_t (*size_t_fn)(retro_vfs_file_handle *);
typedef int64_t (*truncate_t)(retro_vfs_file_handle *, int64_t);
typedef int64_t (*tell_t)(retro_vfs_file_handle *);
typedef int64_t (*seek_t)(retro_vfs_file_handle *, int64_t, int);
typedef int64_t (*read_t)(retro_vfs_file_handle *, void *, uint64_t);
typedef int64_t (*write_t)(retro_vfs_file_handle *, const void *, uint64_t);
typedef int (*flush_t)(retro_vfs_file_handle *);
typedef int (*path_fn)(const char *);
typedef int (*rename_t)(const char *, const char *);
typedef int (*stat_t)(const char *, int32_t *);
typedef int (*mkdir_t)(const char *);
typedef retro_vfs_dir_handle *(*opendir_t)(const char *, bool);
typedef bool (*readdir_t)(retro_vfs_dir_handle *);
typedef const char *(*dirname_t)(retro_vfs_dir_handle *);
typedef bool (*isdir_t)(retro_vfs_dir_handle *);
typedef int (*closedir_t)(retro_vfs_dir_handle *);
typedef struct {
    get_path_t get_path; open_t open; close_t close; size_t_fn size;
    tell_t tell; seek_t seek; read_t read; write_t write; flush_t flush;
    path_fn remove; rename_t rename; truncate_t truncate;
    stat_t stat; mkdir_t mkdir; opendir_t opendir; readdir_t readdir;
    dirname_t dirent_get_name; isdir_t dirent_is_dir; closedir_t closedir;
} VfsInterface;
typedef struct { uint32_t required_interface_version; VfsInterface *iface; } VfsInterfaceInfo;
typedef struct {
    void *context;
    get_path_t get_path; open_t open; close_t close; size_t_fn size;
    tell_t tell; seek_t seek; read_t read; write_t write; flush_t flush;
    path_fn remove; rename_t rename; truncate_t truncate;
    stat_t stat; mkdir_t mkdir; opendir_t opendir; readdir_t readdir;
    dirname_t dirent_get_name; isdir_t dirent_is_dir; closedir_t closedir;
} VfsBackend;
bool linkslot_vfs_install(const VfsBackend *);
bool linkslot_vfs_environment(uint32_t, void *);
#define RETRO_ENVIRONMENT_GET_VFS_INTERFACE (45u | 0x10000u)
#define RETRO_VFS_FILE_ACCESS_READ 1u
#define RETRO_VFS_FILE_ACCESS_WRITE 2u
#define RETRO_VFS_FILE_ACCESS_READ_WRITE (RETRO_VFS_FILE_ACCESS_READ | RETRO_VFS_FILE_ACCESS_WRITE)
#define RETRO_VFS_FILE_ACCESS_UPDATE_EXISTING 4u
#define RETRO_VFS_STAT_IS_VALID 1
#define RETRO_VFS_STAT_IS_DIRECTORY 2
#define RETRO_VFS_SEEK_POSITION_START 0
#define RETRO_VFS_SEEK_POSITION_CURRENT 1
#define RETRO_VFS_SEEK_POSITION_END 2
#endif
'''

C_BACKEND = r'''#include "linkslot_vfs_abi.h"
#include <string.h>
#include <stdint.h>
typedef struct {
    char path[64];
    unsigned char bytes[256];
    int64_t length;
    int64_t position;
    unsigned mode;
    int open;
} MockFile;
typedef struct {
    char path[64];
    int current;
    int index;
    const char *name;
    bool is_dir;
} MockDir;
static MockFile file_slot;
static MockDir dir_slot;
static unsigned flush_count, close_count, closedir_count, remove_count, rename_count, mkdir_count;
static const char *dir_names[] = {"one.sav", "folder", ".hidden", "two.state"};
static const bool dir_types[] = {false, true, false, false};

static int copy_path(char *dst, size_t cap, const char *src) {
    size_t n;
    if (!src || !*src) return -1;
    n = strlen(src);
    if (n >= cap || strstr(src, "../") || strstr(src, "/..") || src[0] == '/' || strstr(src, "\\\\")) return -1;
    memcpy(dst, src, n + 1);
    return 0;
}
static const char *mock_get_path(retro_vfs_file_handle *h) {
    MockFile *f = (MockFile *)h; return f && f->open ? f->path : 0;
}
static retro_vfs_file_handle *mock_open(const char *p, unsigned mode, unsigned hints) {
    (void)hints;
    if (copy_path(file_slot.path, sizeof(file_slot.path), p)) return 0;
    if (strcmp(p, "card/Saves/test.sav") != 0) return 0;
    if (file_slot.open) return 0;
    if ((mode & RETRO_VFS_FILE_ACCESS_WRITE) && !(mode & RETRO_VFS_FILE_ACCESS_UPDATE_EXISTING)) {
        file_slot.length = 0; file_slot.position = 0;
    }
    file_slot.mode = mode; file_slot.open = 1;
    return (retro_vfs_file_handle *)&file_slot;
}
static int mock_close(retro_vfs_file_handle *h) {
    MockFile *f = (MockFile *)h;
    if (f != &file_slot || !f->open) return -1;
    f->open = 0; close_count++; return 0;
}
static int64_t mock_size(retro_vfs_file_handle *h) {
    MockFile *f = (MockFile *)h; return f && f->open ? f->length : -1;
}
static int64_t mock_tell(retro_vfs_file_handle *h) {
    MockFile *f = (MockFile *)h; return f && f->open ? f->position : -1;
}
static int64_t mock_seek(retro_vfs_file_handle *h, int64_t off, int whence) {
    MockFile *f = (MockFile *)h; int64_t base, pos;
    if (!f || !f->open) return -1;
    base = whence == 0 ? 0 : (whence == 1 ? f->position : f->length);
    pos = base + off;
    if (pos < 0 || pos > (int64_t)sizeof(f->bytes)) return -1;
    f->position = pos; return pos;
}
static int64_t mock_read(retro_vfs_file_handle *h, void *out, uint64_t len) {
    MockFile *f = (MockFile *)h; uint64_t n;
    if (!f || !f->open || !(f->mode & RETRO_VFS_FILE_ACCESS_READ) || !out) return -1;
    n = len; if (n > (uint64_t)(f->length - f->position)) n = (uint64_t)(f->length - f->position);
    memcpy(out, f->bytes + f->position, (size_t)n); f->position += (int64_t)n; return (int64_t)n;
}
static int64_t mock_write(retro_vfs_file_handle *h, const void *in, uint64_t len) {
    MockFile *f = (MockFile *)h; uint64_t n;
    if (!f || !f->open || !(f->mode & RETRO_VFS_FILE_ACCESS_WRITE) || !in) return -1;
    n = len; if (n > sizeof(f->bytes) - (uint64_t)f->position) n = sizeof(f->bytes) - (uint64_t)f->position;
    memcpy(f->bytes + f->position, in, (size_t)n); f->position += (int64_t)n;
    if (f->position > f->length) f->length = f->position;
    return (int64_t)n;
}
static int mock_flush(retro_vfs_file_handle *h) {
    MockFile *f = (MockFile *)h; if (!f || !f->open) return -1; flush_count++; return 0;
}
static int64_t mock_truncate(retro_vfs_file_handle *h, int64_t n) {
    MockFile *f = (MockFile *)h; if (!f || !f->open || n < 0 || n > (int64_t)sizeof(f->bytes)) return -1;
    if (n > f->length) memset(f->bytes + f->length, 0, (size_t)(n - f->length));
    f->length = n; if (f->position > n) f->position = n; return 0;
}
static int mock_remove(const char *p) {
    if (!p || strcmp(p, "card/Saves/test.sav")) return -1;
    file_slot.length = 0; remove_count++; return 0;
}
static int mock_rename(const char *a, const char *b) {
    if (!a || !b || strcmp(a, "card/Saves/test.sav") || strcmp(b, "card/Saves/renamed.sav")) return -1;
    rename_count++; return 0;
}
static int mock_stat(const char *p, int32_t *size) {
    if (!p) return 0;
    if (!strcmp(p, "card/Saves")) { if (size) *size = 0; return RETRO_VFS_STAT_IS_VALID | RETRO_VFS_STAT_IS_DIRECTORY; }
    if (!strcmp(p, "card/Saves/test.sav") && file_slot.length >= 0) {
        if (size) *size = (int32_t)file_slot.length; return RETRO_VFS_STAT_IS_VALID;
    }
    return 0;
}
static int mock_mkdir(const char *p) {
    if (!p || strcmp(p, "card/Saves/new")) return -1; mkdir_count++; return 0;
}
static retro_vfs_dir_handle *mock_opendir(const char *p, bool hidden) {
    if (!p || strcmp(p, "card/Saves")) return 0;
    dir_slot.current = 0; dir_slot.index = -1; dir_slot.name = 0; dir_slot.is_dir = false;
    dir_slot.path[0] = 0; (void)hidden;
    return (retro_vfs_dir_handle *)&dir_slot;
}
static bool mock_readdir(retro_vfs_dir_handle *h) {
    MockDir *d = (MockDir *)h;
    if (d != &dir_slot) return false;
    while (d->index < 3) {
        d->index++;
        if (d->index == 2) continue; /* hidden entry is filtered by backend */
        d->name = dir_names[d->index]; d->is_dir = dir_types[d->index]; return true;
    }
    d->name = 0; return false;
}
static const char *mock_dirent_name(retro_vfs_dir_handle *h) {
    MockDir *d = (MockDir *)h; return d == &dir_slot ? d->name : 0;
}
static bool mock_dirent_is_dir(retro_vfs_dir_handle *h) {
    MockDir *d = (MockDir *)h; return d == &dir_slot && d->is_dir;
}
static int mock_closedir(retro_vfs_dir_handle *h) {
    if (h != (retro_vfs_dir_handle *)&dir_slot) return -1;
    closedir_count++; return 0;
}
VfsBackend mock_backend = {
    0, mock_get_path, mock_open, mock_close, mock_size, mock_tell, mock_seek,
    mock_read, mock_write, mock_flush, mock_remove, mock_rename, mock_truncate,
    mock_stat, mock_mkdir, mock_opendir, mock_readdir, mock_dirent_name,
    mock_dirent_is_dir, mock_closedir
};
unsigned mock_flushes(void) { return flush_count; }
unsigned mock_closes(void) { return close_count; }
unsigned mock_closedirs(void) { return closedir_count; }
unsigned mock_removes(void) { return remove_count; }
unsigned mock_renames(void) { return rename_count; }
unsigned mock_mkdirs(void) { return mkdir_count; }
'''

HARNESS = r'''#include "linkslot_vfs_abi.h"
#include <string.h>
#include <stdint.h>
extern VfsBackend mock_backend;
extern unsigned mock_flushes(void), mock_closes(void), mock_closedirs(void);
extern unsigned mock_removes(void), mock_renames(void), mock_mkdirs(void);
static int check(bool condition, int code) { return condition ? 0 : code; }
int vfs_host_run(void) {
    VfsInterfaceInfo info = {2, 0};
    VfsInterface *v;
    retro_vfs_file_handle *f;
    retro_vfs_dir_handle *d;
    char buffer[8] = {0};
    int32_t size = -1;
    const char *name;
    if (!linkslot_vfs_install(&mock_backend)) return 1;
    if (!linkslot_vfs_environment(RETRO_ENVIRONMENT_GET_VFS_INTERFACE, &info)) return 2;
    if (info.required_interface_version != 3 || !info.iface) return 3;
    v = info.iface;
    if (linkslot_vfs_environment(RETRO_ENVIRONMENT_GET_VFS_INTERFACE, 0)) return 4;
    { VfsInterfaceInfo too_new = {4, 0}; if (linkslot_vfs_environment(RETRO_ENVIRONMENT_GET_VFS_INTERFACE, &too_new)) return 5; }
    if (v->open("card/Saves/test.sav", 0, 0)) return 6;
    if (v->open("../escape.sav", RETRO_VFS_FILE_ACCESS_READ, 0)) return 7;
    f = v->open("card/Saves/test.sav",
                RETRO_VFS_FILE_ACCESS_READ_WRITE | RETRO_VFS_FILE_ACCESS_UPDATE_EXISTING, 0);
    if (!f || v->get_path(f) == 0) return 8;
    if (v->write(f, "abc", 3) != 3 || v->size(f) != 3 || v->tell(f) != 3) return 9;
    if (v->seek(f, 0, RETRO_VFS_SEEK_POSITION_START) != 0) return 10;
    if (v->read(f, buffer, 3) != 3 || memcmp(buffer, "abc", 3)) return 11;
    if (v->truncate(f, 2) != 0 || v->size(f) != 2) return 12;
    if (v->flush(f) != 0 || mock_flushes() != 1) return 13;
    if (v->close(f) != 0 || mock_closes() != 1) return 14;
    if (v->stat("card/Saves/test.sav", &size) != RETRO_VFS_STAT_IS_VALID || size != 2) return 15;
    if (!(v->stat("card/Saves", 0) & RETRO_VFS_STAT_IS_DIRECTORY)) return 16;
    if (v->mkdir("card/Saves/new") != 0 || mock_mkdirs() != 1) return 17;
    if (v->rename("card/Saves/test.sav", "card/Saves/renamed.sav") != 0 || mock_renames() != 1) return 18;
    d = v->opendir("card/Saves", false);
    if (!d) return 19;
    if (!v->readdir(d) || strcmp(v->dirent_get_name(d), "one.sav") || v->dirent_is_dir(d)) return 20;
    if (!v->readdir(d) || strcmp(v->dirent_get_name(d), "folder") || !v->dirent_is_dir(d)) return 21;
    if (!v->readdir(d) || strcmp(v->dirent_get_name(d), "two.state")) return 22;
    if (v->readdir(d)) return 23;
    if (v->closedir(d) != 0 || mock_closedirs() != 1) return 24;
    if (v->remove("card/Saves/test.sav") != 0 || mock_removes() != 1) return 25;
    if (v->open("card/Saves/test.sav", RETRO_VFS_FILE_ACCESS_READ, 0) == 0) return 26;
    (void)name;
    return 0;
}
#ifdef STATIC_VFS_CROSS
volatile int vfs_cross_result;
void _start(void) { vfs_cross_result = vfs_host_run(); for (;;) { __asm__ volatile("wfi"); } }
#else
int main(void) { return vfs_host_run(); }
#endif
'''

def run(args, log=None):
    p=subprocess.run(args,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
    if log:
        log.parent.mkdir(parents=True,exist_ok=True)
        log.write_text(p.stdout,encoding="utf-8")
    if p.returncode:
        print(p.stdout[-12000:],file=sys.stderr)
        raise RuntimeError("command failed: "+" ".join(map(str,args)))
    return p.stdout

def rust_manifest():
    return """[package]
name = "linkslot-vfs-v3-probe"
version = "0.0.0"
edition = "2021"
[lib]
crate-type = ["staticlib"]
[profile.release]
panic = "abort"
"""

def main():
    ap=argparse.ArgumentParser()
    ap.add_argument("--out",type=Path,required=True)
    args=ap.parse_args()
    out=args.out.resolve()
    (out/"src").mkdir(parents=True,exist_ok=True)
    (out/"Cargo.toml").write_text(rust_manifest(),encoding="utf-8")
    (out/"src/lib.rs").write_text(RUST,encoding="utf-8")
    (out/"linkslot_vfs_abi.h").write_text(C_HEADER,encoding="utf-8")
    freestanding_string = """#ifndef LINK_SLOT_VFS_STRING_H
#define LINK_SLOT_VFS_STRING_H
#include <stddef.h>
static inline size_t lsvfs_strlen(const char *s) { size_t n=0; while (s && s[n]) n++; return n; }
static inline int lsvfs_strcmp(const char *a, const char *b) {
    while (*a && *a == *b) { a++; b++; }
    return (unsigned char)*a - (unsigned char)*b;
}
static inline char *lsvfs_strstr(const char *h, const char *n) {
    size_t k;
    if (!*n) return (char *)h;
    for (; *h; h++) { for (k=0; n[k] && h[k] == n[k]; k++) { } if (!n[k]) return (char *)h; }
    return 0;
}
static inline void *lsvfs_memcpy(void *d, const void *s, size_t n) {
    unsigned char *o=(unsigned char *)d; const unsigned char *i=(const unsigned char *)s;
    for (size_t k=0;k<n;k++) o[k]=i[k]; return d;
}
static inline void *lsvfs_memset(void *d, int v, size_t n) {
    unsigned char *o=(unsigned char *)d; for (size_t k=0;k<n;k++) o[k]=(unsigned char)v; return d;
}
static inline int lsvfs_memcmp(const void *a, const void *b, size_t n) {
    const unsigned char *x=(const unsigned char *)a,*y=(const unsigned char *)b;
    for (size_t k=0;k<n;k++) if (x[k]!=y[k]) return (int)x[k]-(int)y[k]; return 0;
}
#define strlen lsvfs_strlen
#define strcmp lsvfs_strcmp
#define strstr lsvfs_strstr
#define memcpy lsvfs_memcpy
#define memset lsvfs_memset
#define memcmp lsvfs_memcmp
#endif
"""
    (out/"string.h").write_text(freestanding_string,encoding="utf-8")
    (out/"mock_backend.c").write_text(C_BACKEND,encoding="utf-8")
    (out/"harness.c").write_text(HARNESS,encoding="utf-8")
    report=["# Libretro VFS v3 no_std service ABI probe\n\n",
            "- A no_std Rust frontend registry exposes the 19-pointer Libretro VFS v3 interface (v1 file ops, v2 truncate, v3 stat/mkdir/directory ops).\n",
            "- It installs a backend table so an H700 SD/FAT service can provide handle operations without embedding POSIX FILE/DIR types in the core adapter.\n",
            "- The frontend environment handler accepts RETRO_ENVIRONMENT_GET_VFS_INTERFACE (45 | experimental bit), returns version 3, and rejects null data or a request for an unsupported newer version.\n",
            "- The C harness provides a fake in-memory filesystem and exercises the exported callbacks through the returned interface.\n\n"]
    for triple,(clang_target,emulation,machine,cpu) in TARGETS.items():
        rlib=out/"target"/triple/"release"/"liblinkslot_vfs_v3_probe.a"
        run(["cargo","build","--manifest-path",str(out/"Cargo.toml"),"--target",triple,"--release"],
            out/(triple+"-cargo.log"))
        objects=[]
        for src,name in [("mock_backend.c","backend.o"),("harness.c","harness-"+triple+".o")]:
            obj=out/name
            cmd=["clang","--target="+clang_target,"-mcpu="+cpu,"-ffreestanding","-fno-builtin",
                 "-fno-stack-protector","-fno-unwind-tables","-fno-asynchronous-unwind-tables",
                 "-Wall","-Wextra","-Werror","-I",str(out)]
            if src=="harness.c":cmd.append("-DSTATIC_VFS_CROSS=1")
            cmd += ["-c",str(out/src),"-o",str(obj)]
            run(cmd,out/(name+".log"));objects.append(obj)
        elf=out/("vfs-"+triple+".elf")
        mapf=out/("vfs-"+triple+".map")
        link_args=["ld.lld","-m",emulation,"-static","-e","_start","--no-undefined"]
        if triple == "armv7a-none-eabi":
            reloc=run(["readelf","-Wr",str(rlib)])
            markers=__import__("re").findall(r"\b(R_ARM_[A-Z0-9_]+)\s+[^\n]*__aeabi_unwind_cpp_pr0",reloc)
            if not markers or any(kind != "R_ARM_NONE" for kind in markers):
                raise RuntimeError("AArch32 EHABI marker changed: "+repr(markers))
            # The panic=abort, no-unwind probe has only R_ARM_NONE exidx metadata.
            # This absolute symbol is a metadata marker, not an unwinder implementation.
            link_args.append("--defsym=__aeabi_unwind_cpp_pr0=0")
        link_args += ["-Map="+str(mapf),"-o",str(elf),*map(str,objects),str(rlib)]
        run(link_args,out/(triple+"-link.log"))
        header=run(["readelf","-h",str(elf)])
        undef=run(["llvm-nm","-u",str(elf)])
        if machine not in header or undef.strip():
            raise RuntimeError(triple+": wrong ELF machine or unresolved symbols: "+undef)
        report.append("- PASS: "+triple+" static link; "+machine+" ELF; no undefined symbols.\n")
    host_archive=out/"target/x86_64-unknown-linux-gnu/release/liblinkslot_vfs_v3_probe.a"
    run(["cargo","build","--manifest-path",str(out/"Cargo.toml"),"--target","x86_64-unknown-linux-gnu","--release"],
        out/"host-cargo.log")
    host_objects=[]
    for src,name in [("mock_backend.c","host-backend.o"),("harness.c","host-harness.o")]:
        obj=out/name
        run(["cc","-std=c11","-Wall","-Wextra","-Werror","-I",str(out),"-c",str(out/src),"-o",str(obj)],out/(name+".log"))
        host_objects.append(obj)
    exe=out/"vfs-host-test"
    run(["cc","-o",str(exe),*map(str,host_objects),str(host_archive)],out/"host-link.log")
    run([str(exe)],out/"host-run.log")
    report.append("- PASS: x86_64 host VFS callback harness ran successfully (exit 0): version negotiation, file open/read/write/seek/truncate/flush/close, stat/mkdir/rename/remove, directory iteration/name/type/close, invalid paths and null/unsupported requests.\n")
    report.append("- Boundary: fake backend only. No H700 FAT driver, persistent storage or actual mGBA call site is connected; backend lifetime, locking and durable flush semantics still need implementation.\n")
    (out/"REPORT.md").write_text("".join(report),encoding="utf-8")
    print((out/"REPORT.md").read_text(encoding="utf-8"))

if __name__=="__main__":
    main()
