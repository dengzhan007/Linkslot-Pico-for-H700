#!/usr/bin/env python3
"""Test that the slot crate selects its built-in static mGBA core correctly."""
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
        raise RuntimeError(f"{label}: expected one anchor, found {count}")
    return text.replace(old, new, 1)


def run(command: list[str], cwd: Path, env: dict[str, str], log: Path) -> str:
    proc = subprocess.run(
        command, cwd=cwd, env=env, text=True,
        stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
    )
    log.parent.mkdir(parents=True, exist_ok=True)
    log.write_text(proc.stdout, encoding="utf-8")
    if proc.returncode:
        print(proc.stdout[-20000:], file=sys.stderr)
        raise RuntimeError("command failed: " + " ".join(command))
    return proc.stdout


def patch_slot(source: Path, stub: Path) -> None:
    manifest_file = source / "crates/slot/Cargo.toml"
    core_file = source / "crates/slot/src/core.rs"
    retro_lib = source / "crates/slot-retro/src/lib.rs"
    static_module = source / "crates/slot-retro/src/mgba_static.rs"
    if not all(p.is_file() for p in (manifest_file, core_file, retro_lib, static_module, stub)):
        raise RuntimeError("slot or static mGBA module files are missing")

    manifest = manifest_file.read_text(encoding="utf-8")
    core = core_file.read_text(encoding="utf-8")
    lib = retro_lib.read_text(encoding="utf-8")
    if 'static-mgba = ["slot-retro/static-mgba"]' in manifest:
        raise RuntimeError("slot static-mGBA feature already exists")
    if "pub use mgba_static::api as mgba_static_api;" not in lib:
        raise RuntimeError("slot-retro static mGBA API export is missing")
    if "open_static_mgba(" in core:
        raise RuntimeError("slot static mGBA selector already appears to be patched")

    manifest = replace_once(
        manifest,
        'device = ["slot-gfx/device"]\n',
        'device = ["slot-gfx/device"]\nstatic-mgba = ["slot-retro/static-mgba"]\n',
        "slot static-mGBA Cargo feature",
    )

    core = replace_once(
        core,
        "    let paths = candidates(root, core);\n",
        '''    #[cfg(feature = "static-mgba")]
    if core == Core::Mgba && std::env::var_os(CORE_ENV).is_none() {
        if let Some(opened) =
            open_static_mgba(root, platform, serial, colour, palette, link)
        {
            return Opened { core: opened, named: true };
        }
    }

    let paths = candidates(root, core);
''',
        "open_core static selection",
    )

    helper = '''#[cfg(feature = "static-mgba")]
fn open_static_mgba(
    root: &Path,
    platform: Platform,
    serial: &str,
    colour: bool,
    palette: Option<GbPalette>,
    link: Option<LinkSetup>,
) -> Option<Box<dyn RetroCore>> {
    let bios = root::bios_dir(root);
    let saves = root::saves_dir(root);
    match LibretroCore::open_static(slot_retro::mgba_static_api(), &bios, &saves) {
        Ok(mut opened) => {
            let (frame_w, frame_h) = platform.frame();
            opened.set_canvas(frame_w, frame_h);
            opened.set_six_button_pads(six_button_pads(platform));
            apply_core_options(
                &mut opened,
                Core::Mgba,
                serial,
                root::has_real_bios(root),
                colour,
                palette,
            );
            if let Some(link) = link {
                apply_link_options(&mut opened, Core::Mgba, link);
            }
            eprintln!("slot: core mgba (built-in static)");
            Some(Box::new(opened))
        }
        Err(e) => {
            eprintln!("slot: built-in static mGBA: {e}");
            None
        }
    }
}

'''
    core = replace_once(
        core,
        "pub fn palette_for(\n",
        helper + "pub fn palette_for(\n",
        "static mGBA open helper",
    )

    test_code = '''

    #[cfg(feature = "static-mgba")]
    unsafe extern "C" {
        fn linkslot_mgba_stub_reset_counters();
        fn linkslot_mgba_stub_init_calls() -> u32;
        fn linkslot_mgba_stub_deinit_calls() -> u32;
        fn linkslot_mgba_stub_install_vfs() -> bool;
        fn linkslot_mgba_stub_vfs_request_ok() -> bool;
    }

    #[cfg(feature = "static-mgba")]
    #[test]
    fn open_core_selects_builtin_static_mgba_without_a_path_override() {
        let _g = lock();
        std::env::remove_var(CORE_ENV);
        unsafe {
            linkslot_mgba_stub_reset_counters();
            assert!(
                linkslot_mgba_stub_install_vfs(),
                "the H700 VFS backend must be installed before opening the static core"
            );
        }

        let root = tempfile::tempdir().unwrap();
        let opened = open_core(
            root.path(),
            Core::Mgba,
            Platform::Gba,
            "TEST0001",
            false,
            None,
            None,
        );

        assert!(opened.named, "the built-in static core must count as a named core");
        assert_eq!(unsafe { linkslot_mgba_stub_init_calls() }, 1);
        assert!(
            unsafe { linkslot_mgba_stub_vfs_request_ok() },
            "retro_init must receive Libretro VFS v3 from the preinstalled H700 backend"
        );
        assert_eq!(unsafe { linkslot_mgba_stub_deinit_calls() }, 0);
        drop(opened);
        assert_eq!(unsafe { linkslot_mgba_stub_deinit_calls() }, 1);
    }

    #[cfg(feature = "static-mgba")]
    #[test]
    fn explicit_slot_core_override_keeps_the_dynamic_path() {
        let _g = lock();
        unsafe { linkslot_mgba_stub_reset_counters() };
        std::env::set_var(CORE_ENV, "/missing/explicit-core.so");

        let root = tempfile::tempdir().unwrap();
        let opened = open_core(
            root.path(),
            Core::Mgba,
            Platform::Gba,
            "TEST0001",
            false,
            None,
            None,
        );
        std::env::remove_var(CORE_ENV);

        assert!(!opened.named, "a missing explicit override should not fall back to built-in mGBA");
        assert_eq!(unsafe { linkslot_mgba_stub_init_calls() }, 0);
        assert_eq!(unsafe { linkslot_mgba_stub_deinit_calls() }, 0);
    }
'''
    end = core.rfind("}")
    if end < 0:
        raise RuntimeError("slot core test module closing brace not found")
    core = core[:end] + test_code + core[end:]

    stub_text = stub.read_text(encoding="utf-8")
    required_stub_symbols = (
        "void linkslot_mgba_stub_reset_counters(void)",
        "bool linkslot_mgba_stub_install_vfs(void)",
        "bool linkslot_mgba_stub_vfs_request_ok(void)",
    )
    missing_stub_symbols = [
        symbol for symbol in required_stub_symbols if symbol not in stub_text
    ]
    if missing_stub_symbols:
        raise RuntimeError(
            "mGBA C stub is missing required lifecycle helpers: "
            + ", ".join(missing_stub_symbols)
        )
    manifest_file.write_text(manifest, encoding="utf-8")
    core_file.write_text(core, encoding="utf-8")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path, required=True,
                        help="temporary Linkslot checkout patched for static mGBA Rust binding")
    parser.add_argument("--stub", type=Path, required=True,
                        help="C test stub exporting the namespaced mGBA symbols")
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()

    source = args.source.resolve()
    stub = args.stub.resolve()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    if not (source / ".git").exists():
        raise RuntimeError("source must be the temporary checkout used by the preceding binding test")
    for tool in ("cc", "ar", "cargo"):
        if not shutil.which(tool):
            raise RuntimeError("missing required tool: " + tool)

    patch_slot(source, stub)
    stub_object = out / "mgba_static_stub.o"
    stub_archive = out / "libmgba_test_stubs.a"
    run(
        ["cc", "-std=c11", "-Wall", "-Wextra", "-Werror",
         "-c", str(stub), "-o", str(stub_object)],
        source, os.environ.copy(), out / "compile-stubs.log",
    )
    run(
        ["ar", "rcs", str(stub_archive), str(stub_object)],
        source, os.environ.copy(), out / "archive-stubs.log",
    )

    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(out / "cargo-target")
    flags = env.get("RUSTFLAGS", "").strip()
    env["RUSTFLAGS"] = (
        flags + f" -L native={out} -l static=mgba_test_stubs"
    ).strip()

    output = run(
        [
            "cargo", "test", "--locked", "--no-default-features",
            "--features", "static-mgba", "-p", "slot", "--lib",
            "core::tests::", "--", "--test-threads=1", "--nocapture",
        ],
        source, env, out / "cargo-test.log",
    )
    report = (
        "# Linkslot slot static mGBA core selection validation\n\n"
        "- Adds a slot-level static-mgba feature forwarding to slot-retro/static-mgba.\n"
        "- open_core() prefers the built-in static mGBA API when no SLOT_CORE override is set.\n"
        "- Explicit SLOT_CORE continues to select only the requested dynamic path; a missing override does not silently fall back to the built-in core.\n"
        "- Dynamic candidate search and open_core_for() remain unchanged.\n"
        "- PASS: focused slot core tests with no default host features and static-mgba enabled.\n"
        "- The tests validate built-in init/deinit lifecycle selection and explicit override precedence using host C stubs.\n"
        "- The C core stub requests GET_VFS_INTERFACE during retro_init(); a complete fake H700 backend is installed first, and the test requires successful VFS v3 negotiation before core initialization completes.\n\n"
        "This checks the frontend core-selection seam only. It is not a complete Linkslot application or LK build, and does not access SD/FAT or execute a game.\n\n"
        "Cargo output:\n\n~~~text\n" + output.strip() + "\n~~~\n"
    )
    (out / "REPORT.md").write_text(report, encoding="utf-8")
    print(report)


if __name__ == "__main__":
    main()
