#!/usr/bin/env python3
"""Add a static Libretro API entry to the temporary Linkslot 1.0.45 checkout."""
from __future__ import annotations
import argparse
import os
import shutil
import subprocess
import sys
from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected one patch anchor, found {count}")
    return text.replace(old, new, 1)


def patch_checkout(source: Path) -> None:
    cargo_file = source / "crates/slot-retro/Cargo.toml"
    lib_file = source / "crates/slot-retro/src/lib.rs"
    ffi_file = source / "crates/slot-retro/src/ffi.rs"
    retro_file = source / "crates/slot-retro/src/libretro.rs"
    if not all(p.is_file() for p in (cargo_file, lib_file, ffi_file, retro_file)):
        raise RuntimeError("slot-retro source files are missing")

    workspace = (source / "Cargo.toml").read_text(encoding="utf-8")
    if 'version = "1.0.45"' not in workspace:
        raise RuntimeError("expected immutable Linkslot 1.0.45 source snapshot")

    cargo = cargo_file.read_text(encoding="utf-8")
    lib = lib_file.read_text(encoding="utf-8")
    ffi = ffi_file.read_text(encoding="utf-8")
    retro = retro_file.read_text(encoding="utf-8")

    if 'dynamic = ["dep:libloading"]' in cargo or "pub fn open_static(" in retro:
        raise RuntimeError("static API patch already appears to be applied")
    if "crate::vfs::GET_VFS_INTERFACE" not in retro:
        raise RuntimeError("VFS v3 patch must run before the static API patch")

    cargo = replace_once(
        cargo,
        "[dependencies]\nlibloading.workspace = true\n",
        '[features]\ndefault = ["dynamic"]\ndynamic = ["dep:libloading"]\n\n'
        '[dependencies]\nlibloading = { workspace = true, optional = true }\n',
        "slot-retro Cargo feature block",
    )

    ffi = replace_once(ffi, "use libloading::Library;\n",
                       '#[cfg(feature = "dynamic")]\nuse libloading::Library;\n',
                       "ffi Library import")
    ffi = replace_once(ffi, "use crate::core::CoreError;\n",
                       '#[cfg(feature = "dynamic")]\nuse crate::core::CoreError;\n',
                       "ffi CoreError import")
    ffi = replace_once(ffi, "pub struct Api {\n",
                       "#[derive(Clone, Copy)]\npub struct Api {\n",
                       "Api declaration")
    ffi = replace_once(ffi, "impl Api {\n",
                       '#[cfg(feature = "dynamic")]\nimpl Api {\n',
                       "dynamic Api loader impl")

    lib = replace_once(
        lib, "mod rumble;\n",
        "mod rumble;\n\n"
        "pub use ffi::{\n"
        "    Api, AudioBatchFn, AudioSampleFn, EnvironmentFn, GameGeometry, GameInfo,\n"
        "    InputPollFn, InputStateFn, SystemAvInfo, SystemTiming, VideoRefreshFn,\n"
        "};\n",
        "public static API types",
    )

    retro = replace_once(retro, "use libloading::Library;\n",
                         '#[cfg(feature = "dynamic")]\nuse libloading::Library;\n',
                         "libretro Library import")
    retro = replace_once(
        retro, "pub struct LibretroCore {\n",
        "enum LibraryOwner {\n"
        '    #[cfg(feature = "dynamic")]\n'
        "    Dynamic(Library),\n"
        "    Static,\n"
        "}\n\n"
        "pub struct LibretroCore {\n",
        "library owner enum",
    )
    retro = replace_once(retro, "    _lib: Library,\n",
                         "    _library: LibraryOwner,\n",
                         "LibretroCore library field")

    start = retro.index("    pub fn open(dylib: &Path)")
    end = retro.index("    pub fn reported_system_dir(&self)", start)
    open_methods = '''    pub fn open(dylib: &Path) -> Result<Self, CoreError> {
        let dir = dylib.parent().unwrap_or(Path::new(".")).to_path_buf();
        Self::open_with(dylib, &dir, &dir)
    }

    /// Open a core whose Libretro entry points are linked into this executable.
    ///
    /// The caller must provide function pointers to resident code. The static
    /// and dynamic paths share the same host callbacks and core lifecycle.
    pub fn open_static(
        api: Api,
        system_dir: &Path,
        save_dir: &Path,
    ) -> Result<Self, CoreError> {
        if LIVE.swap(true, Ordering::SeqCst) {
            return Err(CoreError::Unsupported("a core is already open".into()));
        }
        Self::open_api(api, LibraryOwner::Static, system_dir, save_dir)
            .inspect_err(|_| LIVE.store(false, Ordering::SeqCst))
    }

    pub fn open_with(
        dylib: &Path,
        system_dir: &Path,
        save_dir: &Path,
    ) -> Result<Self, CoreError> {
        #[cfg(feature = "dynamic")]
        {
            if LIVE.swap(true, Ordering::SeqCst) {
                return Err(CoreError::Unsupported("a core is already open".into()));
            }
            Self::open_inner(dylib, system_dir, save_dir)
                .inspect_err(|_| LIVE.store(false, Ordering::SeqCst))
        }
        #[cfg(not(feature = "dynamic"))]
        {
            let _ = (dylib, system_dir, save_dir);
            Err(CoreError::Unsupported(
                "dynamic Libretro loading is disabled; provide a static Api table".into(),
            ))
        }
    }

'''
    retro = retro[:start] + open_methods + retro[end:]

    start = retro.index("    fn open_inner(dylib: &Path")
    end = retro.index("    fn unload(&mut self)", start)
    open_impl = '''    #[cfg(feature = "dynamic")]
    fn open_inner(
        dylib: &Path,
        system_dir: &Path,
        save_dir: &Path,
    ) -> Result<Self, CoreError> {
        let lib = unsafe { Library::new(dylib) }
            .map_err(|e| CoreError::Load(e.to_string()))?;
        let api = unsafe { Api::load(&lib) }?;
        Self::open_api(api, LibraryOwner::Dynamic(lib), system_dir, save_dir)
    }

    fn open_api(
        api: Api,
        library: LibraryOwner,
        system_dir: &Path,
        save_dir: &Path,
    ) -> Result<Self, CoreError> {
        let version = unsafe { (api.api_version)() };
        if version != API_VERSION {
            return Err(CoreError::Unsupported(format!("libretro api {version}")));
        }
        let mut host = Box::new(Host {
            video: vec![0; VIDEO_BYTES],
            canvas_w: GBA_W,
            canvas_h: GBA_H,
            format: PixelFormat::Xrgb8888,
            audio: Vec::new(),
            inputs: [0; 2],
            system_dir: cdir(system_dir)?,
            save_dir: cdir(save_dir)?,
            rumble: Rumble::default(),
            asked_for_rumble: false,
            netpacket: None,
            net: Link::default(),
            net_peer: None,
            audio_status: None,
            options: std::collections::HashMap::new(),
            options_dirty: false,
            declared: std::collections::HashMap::new(),
            memory_regions: Vec::new(),
        });
        unsafe {
            let _a = Active::bind(&mut host);
            (api.set_environment)(environment);
            (api.set_video_refresh)(video_refresh);
            (api.set_audio_sample)(audio_sample);
            (api.set_audio_sample_batch)(audio_batch);
            (api.set_input_poll)(input_poll);
            (api.set_input_state)(input_state);
            (api.init)();
        }
        Ok(LibretroCore {
            api,
            host,
            rom: Vec::new(),
            rom_path: None,
            av: AvInfo {
                fps: 0.0,
                sample_rate: 0.0,
            },
            loaded: false,
            six_button: false,
            state_size: None,
            _library: library,
        })
    }

'''
    retro = retro[:start] + open_impl + retro[end:]

    retro += '''

#[cfg(test)]
mod static_api_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

    static INIT_CALLS: AtomicUsize = AtomicUsize::new(0);
    static DEINIT_CALLS: AtomicUsize = AtomicUsize::new(0);

    unsafe extern "C" fn mock_init() {
        INIT_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
    }
    unsafe extern "C" fn mock_deinit() {
        DEINIT_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
    }
    unsafe extern "C" fn mock_api_version() -> c_uint { API_VERSION }
    unsafe extern "C" fn mock_set_environment(_: EnvironmentFn) {}
    unsafe extern "C" fn mock_set_video(_: VideoRefreshFn) {}
    unsafe extern "C" fn mock_set_audio(_: AudioSampleFn) {}
    unsafe extern "C" fn mock_set_audio_batch(_: AudioBatchFn) {}
    unsafe extern "C" fn mock_set_input_poll(_: InputPollFn) {}
    unsafe extern "C" fn mock_set_input_state(_: InputStateFn) {}
    unsafe extern "C" fn mock_set_controller(_: c_uint, _: c_uint) {}
    unsafe extern "C" fn mock_get_av(_: *mut SystemAvInfo) {}
    unsafe extern "C" fn mock_load_game(_: *const GameInfo) -> bool { false }
    unsafe extern "C" fn mock_unload_game() {}
    unsafe extern "C" fn mock_run() {}
    unsafe extern "C" fn mock_serialize_size() -> usize { 0 }
    unsafe extern "C" fn mock_serialize(_: *mut c_void, _: usize) -> bool { false }
    unsafe extern "C" fn mock_unserialize(_: *const c_void, _: usize) -> bool { false }
    unsafe extern "C" fn mock_memory_data(_: c_uint) -> *mut c_void { ptr::null_mut() }
    unsafe extern "C" fn mock_memory_size(_: c_uint) -> usize { 0 }

    fn mock_static_api() -> Api {
        Api {
            cheat_reset: None,
            cheat_set: None,
            init: mock_init,
            deinit: mock_deinit,
            api_version: mock_api_version,
            get_system_av_info: mock_get_av,
            set_environment: mock_set_environment,
            set_video_refresh: mock_set_video,
            set_audio_sample: mock_set_audio,
            set_audio_sample_batch: mock_set_audio_batch,
            set_input_poll: mock_set_input_poll,
            set_input_state: mock_set_input_state,
            set_controller_port_device: mock_set_controller,
            load_game: mock_load_game,
            unload_game: mock_unload_game,
            run: mock_run,
            serialize_size: mock_serialize_size,
            serialize: mock_serialize,
            unserialize: mock_unserialize,
            get_memory_data: mock_memory_data,
            get_memory_size: mock_memory_size,
        }
    }

    #[test]
    fn static_api_uses_the_shared_core_lifecycle() {
        INIT_CALLS.store(0, AtomicOrdering::SeqCst);
        DEINIT_CALLS.store(0, AtomicOrdering::SeqCst);
        let core = LibretroCore::open_static(
            mock_static_api(),
            Path::new("/sdcard/system"),
            Path::new("/sdcard/saves"),
        ).expect("static API should open without a dynamic library handle");

        assert_eq!(INIT_CALLS.load(AtomicOrdering::SeqCst), 1);
        assert_eq!(DEINIT_CALLS.load(AtomicOrdering::SeqCst), 0);
        assert_eq!(core.reported_system_dir(), "/sdcard/system");
        assert_eq!(core.reported_save_dir(), "/sdcard/saves");
        drop(core);
        assert_eq!(DEINIT_CALLS.load(AtomicOrdering::SeqCst), 1);
    }

    #[cfg(not(feature = "dynamic"))]
    #[test]
    fn dynamic_open_is_rejected_in_static_only_builds() {
        match LibretroCore::open_with(
            Path::new("core.so"),
            Path::new("/sdcard/system"),
            Path::new("/sdcard/saves"),
        ) {
            Err(CoreError::Unsupported(_)) => {}
            _ => panic!("dynamic loading must be disabled without the feature"),
        }
    }
}
'''

    cargo_file.write_text(cargo, encoding="utf-8")
    lib_file.write_text(lib, encoding="utf-8")
    ffi_file.write_text(ffi, encoding="utf-8")
    retro_file.write_text(retro, encoding="utf-8")


def run(command: list[str], cwd: Path, env: dict[str, str], log: Path) -> str:
    result = subprocess.run(command, cwd=cwd, env=env, text=True,
                            stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    log.parent.mkdir(parents=True, exist_ok=True)
    log.write_text(result.stdout, encoding="utf-8")
    if result.returncode:
        print(result.stdout[-20000:], file=sys.stderr)
        raise RuntimeError("command failed: " + " ".join(command))
    return result.stdout


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path, required=True,
                        help="temporary checkout already patched for VFS v3")
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    source, out = args.source.resolve(), args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    if not (source / ".git").exists():
        raise RuntimeError("source must be the checkout used by the preceding VFS test")
    for tool in ("cargo", "git"):
        if not shutil.which(tool):
            raise RuntimeError("missing required tool: " + tool)
    snapshot = subprocess.check_output(
        ["git", "-C", str(source), "rev-parse", "HEAD"], text=True
    ).strip()

    patch_checkout(source)
    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(out / "cargo-target")
    dynamic = run(
        ["cargo", "test", "--locked", "-p", "slot-retro", "--lib"],
        source, env, out / "cargo-test-dynamic-default.log",
    )
    static_only = run(
        ["cargo", "test", "--locked", "--no-default-features", "-p", "slot-retro", "--lib"],
        source, env, out / "cargo-test-static-only.log",
    )

    report = (
        "# Linkslot slot-retro static Libretro API entry validation\\n\\n"
        f"- Immutable source snapshot: {snapshot}\\n"
        "- Applied after the preceding temporary VFS v3 integration patch.\\n"
        "- Added an optional dynamic Cargo feature, enabled by default for existing host use.\\n"
        "- Added LibretroCore::open_static(Api, system_dir, save_dir) and shared init/deinit lifecycle.\\n"
        "- Re-exported Api and callback/ABI types needed to build a table from statically linked symbols.\\n\\n"
        "Validation:\\n"
        "- PASS: default-feature cargo test --locked -p slot-retro --lib.\\n"
        "- PASS: static-only cargo test --locked --no-default-features -p slot-retro --lib.\\n"
        "- Static-only build compiles without libloading and rejects the dynamic open entry explicitly.\\n"
        "- Regression opens a mock static API, checks the system/save paths and verifies init/deinit.\\n\\n"
        "Default-feature output:\\n\\n~~~text\\n" + dynamic.strip()
        + "\\n~~~\\n\\nStatic-only output:\\n\\n~~~text\\n" + static_only.strip()
        + "\\n~~~\\n\\n"
        "Boundary: this validates a static function-table entry inside slot-retro. It does not yet bind "
        "the namespaced real mGBA symbols, link the Linkslot application with the mGBA archive, remove "
        "std from slot-retro, implement H700 storage, build a complete LK application, or run on device. "
        "The immutable upstream branch is unchanged.\\n"
    )
    (out / "REPORT.md").write_text(report, encoding="utf-8")
    print(report)


if __name__ == "__main__":
    main()
