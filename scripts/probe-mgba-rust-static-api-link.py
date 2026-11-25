#!/usr/bin/env python3
"""Link the real Linkslot Rust static mGBA API table to the real AArch32 archive."""

from __future__ import annotations

import argparse
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path


EXPECTED_EXPORTS = {
    "mgba_retro_cheat_reset",
    "mgba_retro_cheat_set",
    "mgba_retro_init",
    "mgba_retro_deinit",
    "mgba_retro_api_version",
    "mgba_retro_get_system_av_info",
    "mgba_retro_set_environment",
    "mgba_retro_set_video_refresh",
    "mgba_retro_set_audio_sample",
    "mgba_retro_set_audio_sample_batch",
    "mgba_retro_set_input_poll",
    "mgba_retro_set_input_state",
    "mgba_retro_set_controller_port_device",
    "mgba_retro_load_game",
    "mgba_retro_unload_game",
    "mgba_retro_run",
    "mgba_retro_serialize_size",
    "mgba_retro_serialize",
    "mgba_retro_unserialize",
    "mgba_retro_get_memory_data",
    "mgba_retro_get_memory_size",
}


def run(command: list[str], log: Path, *, cwd: Path | None = None,
        env: dict[str, str] | None = None) -> str:
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
        print(proc.stdout[-24000:], file=sys.stderr)
        raise RuntimeError("command failed: " + " ".join(command))
    return proc.stdout


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--source", type=Path, required=True,
                    help="temporary Linkslot 1.0.49 checkout after static API patch")
    ap.add_argument("--module", type=Path, required=True,
                    help="repository platform/h700/runtime/mgba_static.rs")
    ap.add_argument("--archive", type=Path, required=True,
                    help="real namespaced mGBA VFS v3 AArch32 archive")
    ap.add_argument("--adapter-object", type=Path, required=True,
                    help="compiled H700 newlib adapter object")
    ap.add_argument("--out", type=Path, required=True)
    args = ap.parse_args()

    source = args.source.resolve()
    module = args.module.resolve()
    archive = args.archive.resolve()
    adapter = args.adapter_object.resolve()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)

    ffi_source = source / "crates/slot-retro/src/ffi.rs"
    workspace = source / "Cargo.toml"
    for required in (ffi_source, workspace, module, archive, adapter):
        if not required.is_file():
            raise RuntimeError("required static mGBA link input is missing: " + str(required))
    if 'version = "1.0.49"' not in workspace.read_text(encoding="utf-8"):
        raise RuntimeError("expected immutable Linkslot 1.0.49 source snapshot")

    ffi = ffi_source.read_text(encoding="utf-8")
    if "#[derive(Clone, Copy)]\npub struct Api" not in ffi:
        raise RuntimeError("the preceding Linkslot static API patch must make Api Copy")
    if 'use std::ffi::{c_char, c_int, c_uint, c_void};' not in ffi:
        raise RuntimeError("unexpected pinned Linkslot FFI imports")
    ffi = ffi.replace(
        'use std::ffi::{c_char, c_int, c_uint, c_void};',
        'use core::ffi::{c_char, c_int, c_uint, c_void};',
        1,
    )
    ffi = ffi.replace("use libloading::Library;\n", "", 1)
    ffi = ffi.replace("use crate::core::CoreError;\n", "", 1)
    dynamic_impl = '\n#[cfg(feature = "dynamic")]\nimpl Api {\n'
    if ffi.count(dynamic_impl) != 1:
        raise RuntimeError("could not isolate dynamic Api loader from pinned FFI types")
    ffi = ffi[:ffi.index(dynamic_impl)].rstrip() + "\n"
    if "std::" in ffi or "libloading" in ffi or "CoreError" in ffi:
        raise RuntimeError("dynamic/std dependencies remain in the static FFI type module")

    crate = out / "rust-static-api"
    src = crate / "src"
    src.mkdir(parents=True, exist_ok=True)
    (crate / "Cargo.toml").write_text(
        """[package]
name = "linkslot-mgba-rust-static-link-probe"
version = "0.1.0"
edition = "2021"
rust-version = "1.96"

[lib]
crate-type = ["staticlib"]

[profile.release]
panic = "abort"
codegen-units = 1
lto = false
""",
        encoding="utf-8",
    )
    (src / "ffi.rs").write_text(ffi, encoding="utf-8")
    shutil.copy2(module, src / "mgba_static.rs")
    (src / "lib.rs").write_text(
        """#![no_std]

pub mod ffi;
pub mod mgba_static;

use core::panic::PanicInfo;

/// Return the resident Linkslot Api table without exposing its Rust layout to C.
#[no_mangle]
pub extern "C" fn linkslot_mgba_static_api_table() -> *const ffi::Api {
    &mgba_static::API
}

/// Call the real linked Libretro API-version entry point through the Rust table.
#[no_mangle]
pub extern "C" fn linkslot_mgba_static_api_version() -> u32 {
    unsafe { (mgba_static::API.api_version)() }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
""",
        encoding="utf-8",
    )

    for tool in ("cargo", "arm-none-eabi-gcc", "arm-none-eabi-nm", "arm-none-eabi-readelf"):
        if not shutil.which(tool):
            raise RuntimeError("required tool missing: " + tool)

    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(out / "cargo-target")
    base_flags = env.get("RUSTFLAGS", "").strip()
    env["RUSTFLAGS"] = (
        base_flags + " -C panic=abort -C force-unwind-tables=no "
        "-C target-cpu=cortex-a53"
    ).strip()
    run(
        ["cargo", "build", "--manifest-path", str(crate / "Cargo.toml"),
         "--target", "armv7a-none-eabi", "--release"],
        out / "cargo-build.log",
        env=env,
    )
    rust_archive = out / "cargo-target/armv7a-none-eabi/release/"
    rust_archive /= "liblinkslot_mgba_rust_static_link_probe.a"
    if not rust_archive.is_file():
        raise RuntimeError("Rust AArch32 staticlib was not produced")

    rust_undefined = subprocess.check_output(
        ["arm-none-eabi-nm", "-u", str(rust_archive)], text=True
    )
    seen_exports = set(re.findall(r"\b(mgba_retro_[A-Za-z0-9_]+)\b", rust_undefined))
    if seen_exports != EXPECTED_EXPORTS:
        raise RuntimeError(
            "Rust Api table does not reference the exact 21 mGBA exports; "
            f"missing={sorted(EXPECTED_EXPORTS - seen_exports)}, "
            f"extra={sorted(seen_exports - EXPECTED_EXPORTS)}"
        )

    entry = out / "entry.c"
    entry.write_text(
        """#include <stdint.h>

extern const void *linkslot_mgba_static_api_table(void);
extern uint32_t linkslot_mgba_static_api_version(void);

static volatile uintptr_t api_table_address;
static volatile uint32_t observed_api_version;

__attribute__((noreturn)) void _start(void) {
    api_table_address = (uintptr_t)linkslot_mgba_static_api_table();
    observed_api_version = linkslot_mgba_static_api_version();
    for (;;) {
        __asm__ volatile("wfi");
    }
}
""",
        encoding="utf-8",
    )
    flags = [
        "-mcpu=cortex-a53", "-marm", "-mfpu=neon-vfpv4",
        "-mfloat-abi=softfp", "-ffreestanding", "-fno-builtin",
        "-fno-pic", "-fno-pie", "-fno-stack-protector",
        "-ffunction-sections", "-fdata-sections", "-Wall", "-Wextra", "-Werror",
    ]
    entry_object = out / "entry.o"
    run(
        ["arm-none-eabi-gcc", *flags, "-c", str(entry), "-o", str(entry_object)],
        out / "entry-compile.log",
    )

    rust_defined = subprocess.check_output(
        ["arm-none-eabi-nm", "-g", "--defined-only", str(rust_archive)], text=True
    )
    for symbol in ("linkslot_mgba_static_api_table", "linkslot_mgba_static_api_version"):
        if symbol not in rust_defined:
            raise RuntimeError("Rust static library does not export required C entry point: " + symbol)

    elf = out / "linkslot-mgba-rust-static.elf"
    mapfile = out / "linkslot-mgba-rust-static.map"
    command = [
        "arm-none-eabi-gcc", *flags, "-nostartfiles", "-static",
        "-Wl,-e,_start", "-Wl,--no-undefined", "-Wl,-Map=" + str(mapfile),
        "-Wl,--whole-archive", str(archive), "-Wl,--no-whole-archive",
        str(entry_object), str(rust_archive), str(adapter),
        "-Wl,--start-group", "-lc", "-lm", "-lgcc", "-Wl,--end-group",
        "-o", str(elf),
    ]
    run(command, out / "final-link.log")

    header = subprocess.check_output(
        ["arm-none-eabi-readelf", "-h", str(elf)], text=True
    )
    if "ELF32" not in header or "ARM" not in header:
        raise RuntimeError("final Rust+mGBA ELF is not ELF32/ARM")
    final_undefined = subprocess.check_output(
        ["arm-none-eabi-nm", "-u", str(elf)], text=True
    ).strip()
    if final_undefined:
        raise RuntimeError("final Rust+mGBA ELF has undefined symbols:\n" + final_undefined)
    symbols = subprocess.check_output(
        ["arm-none-eabi-nm", str(elf)], text=True
    )
    for symbol in ("linkslot_mgba_static_api_table", "linkslot_mgba_static_api_version",
                   "mgba_retro_api_version", "mgba_retro_run", "h700_newlib_install"):
        if symbol not in symbols:
            raise RuntimeError("expected linked symbol missing: " + symbol)

    report = (
        "# Rust Linkslot static mGBA / H700 newlib AArch32 link probe\n\n"
        "- Uses the exact patched Linkslot 1.0.49 slot-retro FFI API type definitions.\n"
        "- Compiles the repository platform/h700/runtime/mgba_static.rs as a no_std AArch32 static library.\n"
        "- Verifies all 21 namespaced mGBA function references are present in the Rust static library.\n"
        "- Links the actual mGBA VFS v3 archive, Rust API table, H700 newlib adapter and newlib/libm/libgcc.\n"
        "- PASS: final ELF32/ARM link with --no-undefined; no unresolved symbols remain.\n"
        "- The entry stub calls the actual retro_api_version through the Rust function table if executed; it is not executed here.\n\n"
        "Boundary: this proves ABI/table relocation and static symbol closure only. It does not call core init/load/run, "
        "does not supply installed H700 newlib services, does not link the full LK kernel, and is not firmware or a device test.\n"
    )
    (out / "REPORT.md").write_text(report, encoding="utf-8")
    print(report)


if __name__ == "__main__":
    main()
