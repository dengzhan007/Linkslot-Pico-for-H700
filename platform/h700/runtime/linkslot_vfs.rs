//! Libretro VFS v3 frontend bridge for the H700 Linkslot port.
//!
//! The frontend installs one process-lifetime backend before calling retro_init().
//! This module owns the stable Libretro function table and forwards operations
//! to that backend. It is an ABI bridge, not an SD/FAT/exFAT implementation.
//!
//! The platform backend remains responsible for path containment, handle
//! validation, durable flush/rename semantics and callback synchronization.

use core::ffi::{c_char, c_int, c_uint, c_void};
use core::ptr;
use core::sync::atomic::{AtomicPtr, Ordering};

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
pub type CloseDirFn = unsafe extern "C" fn(*mut VfsDirHandle) -> c_int;

/// Platform-provided VFS implementation. This table and its callback state
/// must have process lifetime; the bridge publishes but does not own it.
#[repr(C)]
pub struct VfsBackend {
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
    pub closedir: Option<CloseDirFn>,
}

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
    pub closedir: Option<CloseDirFn>,
}

#[repr(C)]
pub struct VfsInterfaceInfo {
    pub required_interface_version: u32,
    pub iface: *mut VfsInterface,
}

static BACKEND: AtomicPtr<VfsBackend> = AtomicPtr::new(ptr::null_mut());

static INTERFACE: VfsInterface = VfsInterface {
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
    let p = BACKEND.load(Ordering::Acquire);
    if p.is_null() { None } else { Some(unsafe { &*p }) }
}

fn is_complete(b: &VfsBackend) -> bool {
    b.get_path.is_some() && b.open.is_some() && b.close.is_some()
        && b.size.is_some() && b.tell.is_some() && b.seek.is_some()
        && b.read.is_some() && b.write.is_some() && b.flush.is_some()
        && b.remove.is_some() && b.rename.is_some() && b.truncate.is_some()
        && b.stat.is_some() && b.mkdir.is_some() && b.opendir.is_some()
        && b.readdir.is_some() && b.dirent_get_name.is_some()
        && b.dirent_is_dir.is_some() && b.closedir.is_some()
}

/// Install the platform backend exactly once, before retro_init().
///
/// Safety: backend_ptr and all referenced callback code/data must remain valid
/// for process lifetime. The backend must validate handles and sandbox paths;
/// it must synchronize callbacks if storage can be accessed concurrently.
#[no_mangle]
pub unsafe extern "C" fn linkslot_vfs_install(backend_ptr: *const VfsBackend) -> bool {
    if backend_ptr.is_null() { return false; }
    if !is_complete(unsafe { &*backend_ptr }) { return false; }
    BACKEND.compare_exchange(
        ptr::null_mut(),
        backend_ptr as *mut VfsBackend,
        Ordering::AcqRel,
        Ordering::Acquire,
    ).is_ok()
}

/// Handle Libretro's GET_VFS_INTERFACE environment command.
pub unsafe fn environment(cmd: c_uint, data: *mut c_void) -> bool {
    if cmd != GET_VFS_INTERFACE || data.is_null() || unsafe { backend() }.is_none() {
        return false;
    }
    let info = unsafe { &mut *(data as *mut VfsInterfaceInfo) };
    if info.required_interface_version > 3 { return false; }
    info.required_interface_version = 3;
    info.iface = ptr::addr_of!(INTERFACE) as *mut VfsInterface;
    true
}

unsafe extern "C" fn vfs_get_path(h: *mut VfsFileHandle) -> *const c_char {
    if h.is_null() { return ptr::null(); }
    unsafe { backend().and_then(|b| b.get_path).map(|f| f(h)) }.unwrap_or(ptr::null())
}
unsafe extern "C" fn vfs_open(path: *const c_char, mode: c_uint, hints: c_uint) -> *mut VfsFileHandle {
    if path.is_null() || mode & (FILE_READ | FILE_WRITE) == 0
        || mode & !(FILE_READ | FILE_WRITE | FILE_UPDATE_EXISTING) != 0
        || (mode & FILE_UPDATE_EXISTING != 0 && mode & FILE_WRITE == 0) {
        return ptr::null_mut();
    }
    unsafe { backend().and_then(|b| b.open).map(|f| f(path, mode, hints)) }.unwrap_or(ptr::null_mut())
}
unsafe extern "C" fn vfs_close(h: *mut VfsFileHandle) -> c_int {
    if h.is_null() { return -1; }
    unsafe { backend().and_then(|b| b.close).map(|f| f(h)) }.unwrap_or(-1)
}
unsafe extern "C" fn vfs_size(h: *mut VfsFileHandle) -> i64 {
    if h.is_null() { return -1; }
    unsafe { backend().and_then(|b| b.size).map(|f| f(h)) }.unwrap_or(-1)
}
unsafe extern "C" fn vfs_truncate(h: *mut VfsFileHandle, n: i64) -> i64 {
    if h.is_null() || n < 0 { return -1; }
    unsafe { backend().and_then(|b| b.truncate).map(|f| f(h, n)) }.unwrap_or(-1)
}
unsafe extern "C" fn vfs_tell(h: *mut VfsFileHandle) -> i64 {
    if h.is_null() { return -1; }
    unsafe { backend().and_then(|b| b.tell).map(|f| f(h)) }.unwrap_or(-1)
}
unsafe extern "C" fn vfs_seek(h: *mut VfsFileHandle, n: i64, whence: c_int) -> i64 {
    if h.is_null() || !(SEEK_START..=SEEK_END).contains(&whence) { return -1; }
    unsafe { backend().and_then(|b| b.seek).map(|f| f(h, n, whence)) }.unwrap_or(-1)
}
unsafe extern "C" fn vfs_read(h: *mut VfsFileHandle, p: *mut c_void, n: u64) -> i64 {
    if h.is_null() || (p.is_null() && n != 0) { return -1; }
    unsafe { backend().and_then(|b| b.read).map(|f| f(h, p, n)) }.unwrap_or(-1)
}
unsafe extern "C" fn vfs_write(h: *mut VfsFileHandle, p: *const c_void, n: u64) -> i64 {
    if h.is_null() || (p.is_null() && n != 0) { return -1; }
    unsafe { backend().and_then(|b| b.write).map(|f| f(h, p, n)) }.unwrap_or(-1)
}
unsafe extern "C" fn vfs_flush(h: *mut VfsFileHandle) -> c_int {
    if h.is_null() { return -1; }
    unsafe { backend().and_then(|b| b.flush).map(|f| f(h)) }.unwrap_or(-1)
}
unsafe extern "C" fn vfs_remove(path: *const c_char) -> c_int {
    if path.is_null() { return -1; }
    unsafe { backend().and_then(|b| b.remove).map(|f| f(path)) }.unwrap_or(-1)
}
unsafe extern "C" fn vfs_rename(a: *const c_char, b: *const c_char) -> c_int {
    if a.is_null() || b.is_null() { return -1; }
    unsafe { backend().and_then(|v| v.rename).map(|f| f(a, b)) }.unwrap_or(-1)
}
unsafe extern "C" fn vfs_stat(path: *const c_char, size: *mut i32) -> c_int {
    if path.is_null() { return 0; }
    unsafe { backend().and_then(|b| b.stat).map(|f| f(path, size)) }.unwrap_or(0)
}
unsafe extern "C" fn vfs_mkdir(path: *const c_char) -> c_int {
    if path.is_null() { return -1; }
    unsafe { backend().and_then(|b| b.mkdir).map(|f| f(path)) }.unwrap_or(-1)
}
unsafe extern "C" fn vfs_opendir(path: *const c_char, hidden: bool) -> *mut VfsDirHandle {
    if path.is_null() { return ptr::null_mut(); }
    unsafe { backend().and_then(|b| b.opendir).map(|f| f(path, hidden)) }.unwrap_or(ptr::null_mut())
}
unsafe extern "C" fn vfs_readdir(h: *mut VfsDirHandle) -> bool {
    if h.is_null() { return false; }
    unsafe { backend().and_then(|b| b.readdir).map(|f| f(h)) }.unwrap_or(false)
}
unsafe extern "C" fn vfs_dirent_get_name(h: *mut VfsDirHandle) -> *const c_char {
    if h.is_null() { return ptr::null(); }
    unsafe { backend().and_then(|b| b.dirent_get_name).map(|f| f(h)) }.unwrap_or(ptr::null())
}
unsafe extern "C" fn vfs_dirent_is_dir(h: *mut VfsDirHandle) -> bool {
    if h.is_null() { return false; }
    unsafe { backend().and_then(|b| b.dirent_is_dir).map(|f| f(h)) }.unwrap_or(false)
}
unsafe extern "C" fn vfs_closedir(h: *mut VfsDirHandle) -> c_int {
    if h.is_null() { return -1; }
    unsafe { backend().and_then(|b| b.closedir).map(|f| f(h)) }.unwrap_or(-1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::ffi::CStr;
    use core::sync::atomic::AtomicBool;

    static FILE_MARKER: u8 = 0;
    static DIR_MARKER: u8 = 0;
    static READ_DIR_DONE: AtomicBool = AtomicBool::new(false);
    static FILE_PATH: &[u8] = b"Games/test.gba\0";
    static DIR_ENTRY: &[u8] = b"state.sav\0";
    static TEST_DATA: &[u8] = b"vfs";

    fn file_handle() -> *mut VfsFileHandle {
        core::ptr::addr_of!(FILE_MARKER) as *mut VfsFileHandle
    }
    fn dir_handle() -> *mut VfsDirHandle {
        core::ptr::addr_of!(DIR_MARKER) as *mut VfsDirHandle
    }
    unsafe extern "C" fn mock_get_path(h: *mut VfsFileHandle) -> *const c_char {
        if h == file_handle() { FILE_PATH.as_ptr() as *const c_char } else { ptr::null() }
    }
    unsafe extern "C" fn mock_open(path: *const c_char, _mode: c_uint, _hints: c_uint) -> *mut VfsFileHandle {
        if !path.is_null() && CStr::from_ptr(path).to_bytes() == b"Games/test.gba" {
            file_handle()
        } else { ptr::null_mut() }
    }
    unsafe extern "C" fn mock_close(h: *mut VfsFileHandle) -> c_int {
        if h == file_handle() { 0 } else { -1 }
    }
    unsafe extern "C" fn mock_size(h: *mut VfsFileHandle) -> i64 {
        if h == file_handle() { 3 } else { -1 }
    }
    unsafe extern "C" fn mock_tell(h: *mut VfsFileHandle) -> i64 {
        if h == file_handle() { 0 } else { -1 }
    }
    unsafe extern "C" fn mock_seek(h: *mut VfsFileHandle, n: i64, _w: c_int) -> i64 {
        if h == file_handle() { n } else { -1 }
    }
    unsafe extern "C" fn mock_read(h: *mut VfsFileHandle, out: *mut c_void, len: u64) -> i64 {
        if h != file_handle() || (out.is_null() && len != 0) { return -1; }
        let n = len.min(TEST_DATA.len() as u64) as usize;
        if n != 0 { ptr::copy_nonoverlapping(TEST_DATA.as_ptr(), out as *mut u8, n); }
        n as i64
    }
    unsafe extern "C" fn mock_write(h: *mut VfsFileHandle, _p: *const c_void, n: u64) -> i64 {
        if h == file_handle() { n as i64 } else { -1 }
    }
    unsafe extern "C" fn mock_flush(h: *mut VfsFileHandle) -> c_int {
        if h == file_handle() { 0 } else { -1 }
    }
    unsafe extern "C" fn mock_remove(_p: *const c_char) -> c_int { 0 }
    unsafe extern "C" fn mock_rename(_a: *const c_char, _b: *const c_char) -> c_int { 0 }
    unsafe extern "C" fn mock_truncate(h: *mut VfsFileHandle, _n: i64) -> i64 {
        if h == file_handle() { 0 } else { -1 }
    }
    unsafe extern "C" fn mock_stat(p: *const c_char, size: *mut i32) -> c_int {
        if p.is_null() { return 0; }
        if !size.is_null() { *size = 3; }
        STAT_VALID
    }
    unsafe extern "C" fn mock_mkdir(_p: *const c_char) -> c_int { 0 }
    unsafe extern "C" fn mock_opendir(p: *const c_char, _hidden: bool) -> *mut VfsDirHandle {
        if !p.is_null() && CStr::from_ptr(p).to_bytes() == b"Saves" {
            READ_DIR_DONE.store(false, Ordering::SeqCst);
            dir_handle()
        } else { ptr::null_mut() }
    }
    unsafe extern "C" fn mock_readdir(h: *mut VfsDirHandle) -> bool {
        h == dir_handle() && !READ_DIR_DONE.swap(true, Ordering::SeqCst)
    }
    unsafe extern "C" fn mock_dirent_get_name(h: *mut VfsDirHandle) -> *const c_char {
        if h == dir_handle() { DIR_ENTRY.as_ptr() as *const c_char } else { ptr::null() }
    }
    unsafe extern "C" fn mock_dirent_is_dir(_h: *mut VfsDirHandle) -> bool { false }
    unsafe extern "C" fn mock_closedir(h: *mut VfsDirHandle) -> c_int {
        if h == dir_handle() { 0 } else { -1 }
    }

    static TEST_BACKEND: VfsBackend = VfsBackend {
        get_path: Some(mock_get_path), open: Some(mock_open), close: Some(mock_close),
        size: Some(mock_size), tell: Some(mock_tell), seek: Some(mock_seek),
        read: Some(mock_read), write: Some(mock_write), flush: Some(mock_flush),
        remove: Some(mock_remove), rename: Some(mock_rename), truncate: Some(mock_truncate),
        stat: Some(mock_stat), mkdir: Some(mock_mkdir), opendir: Some(mock_opendir),
        readdir: Some(mock_readdir), dirent_get_name: Some(mock_dirent_get_name),
        dirent_is_dir: Some(mock_dirent_is_dir), closedir: Some(mock_closedir),
    };

    #[test]
    fn slot_retro_environment_exposes_a_complete_vfs_v3_table() {
        unsafe {
            let mut unavailable = VfsInterfaceInfo { required_interface_version: 3, iface: ptr::null_mut() };
            assert!(!crate::libretro::environment(GET_VFS_INTERFACE, &mut unavailable as *mut _ as *mut c_void));
            assert!(!linkslot_vfs_install(ptr::null()));
            assert!(linkslot_vfs_install(&TEST_BACKEND));
            assert!(!linkslot_vfs_install(&TEST_BACKEND), "backend installation must be one-shot");
            assert!(!crate::libretro::environment(GET_VFS_INTERFACE, ptr::null_mut()));

            let mut too_new = VfsInterfaceInfo { required_interface_version: 4, iface: ptr::null_mut() };
            assert!(!crate::libretro::environment(GET_VFS_INTERFACE, &mut too_new as *mut _ as *mut c_void));
            assert_eq!(too_new.required_interface_version, 4);
            assert!(too_new.iface.is_null());

            let mut info = VfsInterfaceInfo { required_interface_version: 1, iface: ptr::null_mut() };
            assert!(crate::libretro::environment(GET_VFS_INTERFACE, &mut info as *mut _ as *mut c_void));
            assert_eq!(info.required_interface_version, 3);
            assert!(!info.iface.is_null());

            let api = &*info.iface;
            assert!(api.get_path.is_some() && api.open.is_some() && api.close.is_some()
                && api.size.is_some() && api.tell.is_some() && api.seek.is_some()
                && api.read.is_some() && api.write.is_some() && api.flush.is_some()
                && api.remove.is_some() && api.rename.is_some() && api.truncate.is_some()
                && api.stat.is_some() && api.mkdir.is_some() && api.opendir.is_some()
                && api.readdir.is_some() && api.dirent_get_name.is_some()
                && api.dirent_is_dir.is_some() && api.closedir.is_some());

            let path = b"Games/test.gba\0";
            let file = (api.open.unwrap())(path.as_ptr() as *const c_char, FILE_READ, 0);
            assert_eq!(file, file_handle());
            assert_eq!((api.get_path.unwrap())(file), FILE_PATH.as_ptr() as *const c_char);
            assert_eq!((api.size.unwrap())(file), 3);
            let mut data = [0u8; 3];
            assert_eq!((api.read.unwrap())(file, data.as_mut_ptr() as *mut c_void, 3), 3);
            assert_eq!(&data, b"vfs");
            assert_eq!((api.close.unwrap())(file), 0);

            let mut size = -1;
            assert_eq!((api.stat.unwrap())(path.as_ptr() as *const c_char, &mut size), STAT_VALID);
            assert_eq!(size, 3);

            let dir_path = b"Saves\0";
            let dir = (api.opendir.unwrap())(dir_path.as_ptr() as *const c_char, false);
            assert_eq!(dir, dir_handle());
            assert!((api.readdir.unwrap())(dir));
            assert_eq!((api.dirent_get_name.unwrap())(dir), DIR_ENTRY.as_ptr() as *const c_char);
            assert!(!(api.dirent_is_dir.unwrap())(dir));
            assert!(!(api.readdir.unwrap())(dir));
            assert_eq!((api.closedir.unwrap())(dir), 0);
        }
    }
}
