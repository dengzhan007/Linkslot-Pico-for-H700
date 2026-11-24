#!/usr/bin/env python3
"""Apply persistent H700 no_std Linkslot business/state crates to a temp source tree."""
from __future__ import annotations

import argparse
import shutil
import subprocess
from pathlib import Path

PINNED_SNAPSHOT = "8590d6ff1b2caa26118051069c975106fbb14bd3"
UPSTREAM_CRATES = ("slot-input", "slot-model")


def run_git(source: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(source), *args],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    return result.stdout.strip()


def replace_upstream_package(source_crate: Path, destination: Path) -> None:
    manifest = source_crate / "Cargo.toml"
    src = source_crate / "src"
    if not manifest.is_file() or not src.is_dir() or not destination.is_dir():
        raise RuntimeError(f"incomplete upstream/persistent crate: {source_crate}")
    old_src = destination / "src"
    if old_src.exists():
        shutil.rmtree(old_src)
    shutil.copytree(src, old_src)
    shutil.copy2(manifest, destination / "Cargo.toml")


def add_new_package(source_crate: Path, destination: Path) -> None:
    manifest = source_crate / "Cargo.toml"
    src = source_crate / "src"
    if not manifest.is_file() or not src.is_dir():
        raise RuntimeError(f"incomplete persistent crate: {source_crate}")
    if destination.exists():
        raise RuntimeError(f"new package destination already exists: {destination}")
    destination.mkdir(parents=True)
    shutil.copytree(src, destination / "src")
    shutil.copy2(manifest, destination / "Cargo.toml")


def patch_frontend(source: Path) -> None:
    slot_manifest = source / "crates/slot/Cargo.toml"
    manifest_text = slot_manifest.read_text(encoding="utf-8")
    dependency = 'slot-frontend-state = { path = "../slot-frontend-state" }'
    picker_dependency = 'slot-core-picker = { path = "../slot-core-picker" }'
    changed_manifest = False
    if dependency not in manifest_text:
        anchor = 'slot-model = { path = "../slot-model" }\n'
        if manifest_text.count(anchor) != 1:
            raise RuntimeError("cannot locate slot-model dependency in slot/Cargo.toml")
        manifest_text = manifest_text.replace(anchor, anchor + dependency + "\n")
        changed_manifest = True
    if picker_dependency not in manifest_text:
        anchor = dependency + "\n"
        if manifest_text.count(anchor) != 1:
            raise RuntimeError("cannot locate slot-frontend-state dependency in slot/Cargo.toml")
        manifest_text = manifest_text.replace(anchor, anchor + picker_dependency + "\n")
        changed_manifest = True
    if changed_manifest:
        slot_manifest.write_text(manifest_text, encoding="utf-8")

    frontend_path = source / "crates/slot/src/frontend.rs"
    frontend = frontend_path.read_text(encoding="utf-8")
    start_marker = "/// Which language the baked [`UiFaces`] belong to, and which build is in\n"
    end_marker = "pub struct Frontend {\n"
    if frontend.count(start_marker) != 1 or frontend.count(end_marker) != 1:
        raise RuntimeError("Linkslot FacesLang source block does not match the pinned snapshot")
    start = frontend.index(start_marker)
    end = frontend.index(end_marker, start)
    if "pub struct FacesLang" not in frontend[start:end] or "impl FacesLang" not in frontend[start:end]:
        raise RuntimeError("FacesLang declaration/implementation was not found in expected block")
    frontend = frontend[:start] + frontend[end:]

    import_anchor = "use crate::wallpaper;\n"
    imported = "use slot_frontend_state::FacesLang;\n"
    if frontend.count(import_anchor) != 1:
        raise RuntimeError("cannot locate Linkslot frontend import insertion point")
    if imported not in frontend:
        frontend = frontend.replace(import_anchor, import_anchor + imported)

    field_anchor = "    faces_lang: FacesLang,\n"
    field_replacement = "    faces_lang: FacesLang<Language>,\n"
    if frontend.count(field_anchor) != 1:
        raise RuntimeError("cannot locate Frontend FacesLang field")
    frontend = frontend.replace(field_anchor, field_replacement)
    if "pub struct FacesLang" in frontend:
        raise RuntimeError("local FacesLang implementation was not fully removed")
    frontend_path.write_text(frontend, encoding="utf-8")

    app_path = source / "crates/slot/src/app.rs"
    app = app_path.read_text(encoding="utf-8")
    app_replacements = (
        ("core_picker: Option<CorePicker>,", "core_picker: Option<CorePicker<Core>>,"),
        ("pub fn core_picker_chip(&self) -> Option<Chip> {",
         "pub fn core_picker_chip(&self) -> Option<Chip<Core>> {"),
        ("fn core_picker_shown(&self) -> Option<&CorePicker> {",
         "fn core_picker_shown(&self) -> Option<&CorePicker<Core>> {"),
        ("fn draw_core_picker(&self, picker: &CorePicker, out: &mut Vec<Draw>) {",
         "fn draw_core_picker(&self, picker: &CorePicker<Core>, out: &mut Vec<Draw>) {"),
    )
    for old, new in app_replacements:
        if app.count(old) != 1:
            raise RuntimeError(f"expected exactly one App core-picker type: {old}")
        app = app.replace(old, new)
    app_path.write_text(app, encoding="utf-8")


def apply(source: Path, crate_root: Path) -> None:
    source = source.resolve()
    crate_root = crate_root.resolve()
    if not (source / ".git").exists():
        raise RuntimeError(f"not a Git checkout: {source}")
    actual = run_git(source, "rev-parse", "HEAD")
    if actual != PINNED_SNAPSHOT:
        raise RuntimeError(
            f"Linkslot snapshot pin mismatch: expected {PINNED_SNAPSHOT}, got {actual}"
        )
    workspace = (source / "Cargo.toml").read_text(encoding="utf-8")
    if 'version = "1.0.36"' not in workspace:
        raise RuntimeError("source workspace is not Linkslot 1.0.36")
    if 'members = ["crates/*"]' not in workspace:
        raise RuntimeError("Linkslot workspace no longer uses crates/* auto-members")

    for name in UPSTREAM_CRATES:
        dest = source / "crates" / name
        if not dest.is_dir():
            raise RuntimeError(f"upstream crate directory missing: {dest}")
        replace_upstream_package(crate_root / name, dest)

    # This state crate is extracted from the pure FacesLang section of
    # crates/slot/src/frontend.rs; it has no corresponding upstream package.
    add_new_package(crate_root / "slot-frontend-state", source / "crates/slot-frontend-state")
    add_new_package(crate_root / "slot-core-picker", source / "crates/slot-core-picker")

    core_picker_path = source / "crates/slot/src/core_picker.rs"
    core_picker_path.write_text(
        "pub use slot_core_picker::{Chip, CorePicker, Outcome, Press, "
        "CLOSE_MS, HOP_MS, LIFT_MS, OPEN_MS, SLIDE_MS};" + chr(10),
        encoding="utf-8",
    )
    patch_frontend(source)

    paths = [
        "crates/slot-input",
        "crates/slot-model",
        "crates/slot-frontend-state",
        "crates/slot-core-picker",
        "crates/slot/Cargo.toml",
        "crates/slot/src/frontend.rs",
        "crates/slot/src/app.rs",
        "crates/slot/src/core_picker.rs",
    ]
    changed = run_git(source, "status", "--short", "--", *paths)
    if not changed:
        raise RuntimeError("business/state overlay produced no source changes")
    run_git(source, "diff", "--check", "--", *paths)
    print("Applied persistent H700 no_std Linkslot packages to the isolated 1.0.36 checkout:")
    print(changed)
    print("The repository's upstream/linkslot branch is not modified by this host-only tool.")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", required=True, type=Path)
    parser.add_argument("--crate-root", required=True, type=Path)
    args = parser.parse_args()
    apply(args.source, args.crate_root)


if __name__ == "__main__":
    main()
