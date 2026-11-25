#!/usr/bin/env python3
"""Patch slot-retro with a Rust binding to namespaced mGBA symbols and test it."""
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
        command,
        cwd=cwd,
        env=env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    log.parent.mkdir(parents=True, exist_ok=True)
    log.write_text(proc.stdout, encoding="utf-8")
    if proc.returncode:
        print(proc.stdout[-18000:], file=sys.stderr)
        raise RuntimeError("command failed: " + " ".join(command))
    return proc.stdout


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path, required=True,
                        help="temporary Linkslot checkout already patched for static API")
    parser.add_argument("--module", type=Path, required=True)
    parser.add_argument("--stub", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()

    source = args.source.resolve()
    module = args.module.resolve()
    stub = args.stub.resolve()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)

    cargo_file = source / "crates/slot-retro/Cargo.toml"
    lib_file = source / "crates/slot-retro/src/lib.rs"
    retro_file = source / "crates/slot-retro/src/libretro.rs"
    if not all(p.is_file() for p in (cargo_file, lib_file, retro_file, module, stub)):
        raise RuntimeError("slot-retro source or mGBA static binding assets are missing")
    workspace = (source / "Cargo.toml").read_text(encoding="utf-8")
    if 'version = "1.0.49"' not in workspace:
        raise RuntimeError("expected immutable Linkslot 1.0.49 source snapshot")

    cargo = cargo_file.read_text(encoding="utf-8")
    lib = lib_file.read_text(encoding="utf-8")
    retro = retro_file.read_text(encoding="utf-8")
    if 'dynamic = ["dep:libloading"]' not in cargo or "pub fn open_static(" not in retro:
        raise RuntimeError("static Libretro API patch must run before this test")
    if "mod mgba_static;" in lib or "mgba_static_binding_tests" in retro:
        raise RuntimeError("mGBA Rust binding patch already appears to be applied")

    cargo = replace_once(
        cargo,
        'dynamic = ["dep:libloading"]\n',
        'dynamic = ["dep:libloading"]\nstatic-mgba = []\n',
        "static-mgba Cargo feature",
    )
    lib = replace_once(
        lib,
        "mod mock;\n",
        "mod mock;\n"
        '#[cfg(feature = "static-mgba")]\nmod mgba_static;\n'
        '#[cfg(feature = "static-mgba")]\npub use mgba_static::api as mgba_static_api;\n',
        "mGBA static module registration",
    )
    retro += '''

#[cfg(all(test, feature = "static-mgba"))]
mod mgba_static_binding_tests {
    use super::*;
    use core::ffi::c_uint;

    unsafe extern "C" {
        fn linkslot_mgba_stub_init_calls() -> c_uint;
        fn linkslot_mgba_stub_deinit_calls() -> c_uint;
    }

    #[test]
    fn namespaced_mgba_api_uses_the_real_static_binding_and_core_lifecycle() {
        let api = crate::mgba_static_api();
        assert_eq!(unsafe { (api.api_version)() }, API_VERSION);
        assert!(api.cheat_reset.is_some());
        assert!(api.cheat_set.is_some());

        assert_eq!(unsafe { linkslot_mgba_stub_init_calls() }, 0);
        assert_eq!(unsafe { linkslot_mgba_stub_deinit_calls() }, 0);

        let core = LibretroCore::open_static(
            api,
            Path::new("/sdcard/system"),
            Path::new("/sdcard/saves"),
        ).expect("namespaced static mGBA API should open");
        assert_eq!(unsafe { linkslot_mgba_stub_init_calls() }, 1);
        assert_eq!(unsafe { linkslot_mgba_stub_deinit_calls() }, 0);
        assert_eq!(core.reported_system_dir(), "/sdcard/system");
        assert_eq!(core.reported_save_dir(), "/sdcard/saves");

        drop(core);
        assert_eq!(unsafe { linkslot_mgba_stub_deinit_calls() }, 1);
    }
}
'''
    (source / "crates/slot-retro/src/mgba_static.rs").write_text(
        module.read_text(encoding="utf-8"), encoding="utf-8"
    )
    cargo_file.write_text(cargo, encoding="utf-8")
    lib_file.write_text(lib, encoding="utf-8")
    retro_file.write_text(retro, encoding="utf-8")

    for tool in ("cc", "ar", "cargo"):
        if not shutil.which(tool):
            raise RuntimeError("missing required tool: " + tool)

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
    extra = f"-L native={out} -l static=mgba_test_stubs"
    env["RUSTFLAGS"] = (flags + " " + extra).strip()

    output = run(
        ["cargo", "test", "--locked", "--no-default-features",
         "--features", "static-mgba", "-p", "slot-retro", "--lib",
         "--", "--test-threads=1", "--nocapture"],
        source, env, out / "cargo-test.log",
    )
    report = (
        "# Linkslot Rust static mGBA binding validation\n\n"
        "- Adds a H700 Rust module mapping all 21 namespaced mGBA functions to the actual slot-retro Api struct.\n"
        "- Enables the module only with the static-mgba Cargo feature; dynamic loading remains independently optional.\n"
        "- Host C stubs export the same names and signatures for the Rust lifecycle test.\n"
        "- PASS: static-only slot-retro tests with the static-mgba feature.\n"
        "- The new regression calls mgba_static_api() through LibretroCore::open_static(), checks API version,\n"
        "  init/deinit counters, and the registered system/save paths.\n\n"
        "This validates the Rust function-table mapping and lifecycle against C stubs. The separately passing\n"
        "AArch32 whole-archive probe validates real mGBA symbol resolution. It does not emulate a game,\n"
        "build the complete LK application, implement H700 storage or run on the RG34XX.\n"
        "\nCargo output:\n\n~~~text\n" + output.strip() + "\n~~~\n"
    )
    (out / "REPORT.md").write_text(report, encoding="utf-8")
    print(report)


if __name__ == "__main__":
    main()
