#!/usr/bin/env python3
"""Patch a clean Linkslot 1.0.45 checkout with the H700 VFS seam and host-test it."""
from __future__ import annotations
import argparse
import os
import shutil
import subprocess
import sys
from pathlib import Path


def patch_checkout(source: Path, module: Path) -> None:
    package = source / "Cargo.toml"
    lib_rs = source / "crates/slot-retro/src/lib.rs"
    libretro_rs = source / "crates/slot-retro/src/libretro.rs"
    vfs_rs = source / "crates/slot-retro/src/vfs.rs"
    if not all(p.is_file() for p in (package, lib_rs, libretro_rs, module)):
        raise RuntimeError("Linkslot workspace or VFS bridge module is missing")
    if 'version = "1.0.45"' not in package.read_text(encoding="utf-8"):
        raise RuntimeError("expected the supplied Linkslot 1.0.45 source snapshot")

    lib_text = lib_rs.read_text(encoding="utf-8")
    retro_text = libretro_rs.read_text(encoding="utf-8")
    if vfs_rs.exists() or "mod vfs;" in lib_text or "crate::vfs::GET_VFS_INTERFACE" in retro_text:
        raise RuntimeError("checkout is already patched; refusing ambiguous reapplication")
    if "mod ffi;\n" not in lib_text:
        raise RuntimeError("slot-retro lib.rs anchor changed")
    lib_text = lib_text.replace("mod ffi;\n", "mod ffi;\nmod vfs;\n", 1)

    signature = 'unsafe extern "C" fn environment(cmd: c_uint, data: *mut c_void) -> bool'
    if signature not in retro_text:
        raise RuntimeError("slot-retro environment callback signature changed")
    retro_text = retro_text.replace(
        signature,
        'pub(crate) unsafe extern "C" fn environment(cmd: c_uint, data: *mut c_void) -> bool',
        1,
    )
    anchor = "    match cmd {\n        GET_CAN_DUPE => {"
    if anchor not in retro_text:
        raise RuntimeError("slot-retro environment match anchor changed")
    retro_text = retro_text.replace(
        anchor,
        "    match cmd {\n"
        "        crate::vfs::GET_VFS_INTERFACE => crate::vfs::environment(cmd, data),\n"
        "        GET_CAN_DUPE => {",
        1,
    )

    vfs_rs.write_text(module.read_text(encoding="utf-8"), encoding="utf-8")
    lib_rs.write_text(lib_text, encoding="utf-8")
    libretro_rs.write_text(retro_text, encoding="utf-8")


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--source", type=Path, required=True, help="clean Linkslot 1.0.45 checkout")
    ap.add_argument("--module", type=Path, required=True, help="H700 VFS bridge Rust module")
    ap.add_argument("--out", type=Path, required=True)
    args = ap.parse_args()
    source, module, out = args.source.resolve(), args.module.resolve(), args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    for tool in ("cargo", "git"):
        if not shutil.which(tool):
            raise RuntimeError("missing required tool: " + tool)
    if not (source / ".git").exists():
        raise RuntimeError("source must be a Git checkout to identify the snapshot")

    snapshot = subprocess.check_output(
        ["git", "-C", str(source), "rev-parse", "HEAD"], text=True
    ).strip()
    patch_checkout(source, module)

    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(out / "cargo-target")
    command = ["cargo", "test", "--locked", "-p", "slot-retro", "--lib", "--", "--nocapture"]
    result = subprocess.run(
        command, cwd=source, env=env, text=True,
        stdout=subprocess.PIPE, stderr=subprocess.STDOUT
    )
    (out / "cargo-test.log").write_text(result.stdout, encoding="utf-8")
    if result.returncode:
        print(result.stdout[-16000:], file=sys.stderr)
        raise RuntimeError("patched slot-retro host tests failed")

    report = (
        "# Linkslot slot-retro VFS v3 integration validation\n\n"
        f"- Immutable Linkslot archive snapshot: {snapshot} (temporary checkout only)\n"
        "- Patch scope: add H700 VFS bridge and route the real slot-retro environment callback.\n"
        "- Test command: cargo test --locked -p slot-retro --lib -- --nocapture\n"
        "- Result: PASS. The regression calls the patched production environment callback, "
        "tests missing-backend and invalid requests, negotiates VFS v3, and dispatches file "
        "and directory operations through a fake backend.\n\n"
        "Cargo output:\n\n~~~text\n" + result.stdout.strip() + "\n~~~\n\n"
        "Boundary: this is source-integration and host-test evidence only. It does not implement "
        "or test H700 SD/FAT/exFAT, durable storage, path sandboxing, SMP synchronization, "
        "on-device mGBA execution, a boot image or the RG34XX boot handoff. The immutable "
        "upstream/linkslot branch is not modified.\n"
    )
    (out / "REPORT.md").write_text(report, encoding="utf-8")
    print(report)


if __name__ == "__main__":
    main()
