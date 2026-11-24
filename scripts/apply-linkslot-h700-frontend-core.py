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
    link_state_dependency = 'slot-link-state = { path = "../slot-link-state" }'
    link_kind_dependency = 'slot-link-kind = { path = "../slot-link-kind" }'
    video_mode_dependency = 'slot-video-mode = { path = "../slot-video-mode" }'
    rom_id_dependency = 'slot-rom-id = { path = "../slot-rom-id" }'
    game_search_dependency = 'slot-game-search = { path = "../slot-game-search" }'
    hold_state_dependency = 'slot-hold-state = { path = "../slot-hold-state" }'
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
    if link_state_dependency not in manifest_text:
        anchor = picker_dependency + "\n"
        if manifest_text.count(anchor) != 1:
            raise RuntimeError("cannot locate slot-core-picker dependency in slot/Cargo.toml")
        manifest_text = manifest_text.replace(anchor, anchor + link_state_dependency + "\n")
        changed_manifest = True
    if link_kind_dependency not in manifest_text:
        anchor = link_state_dependency + "\n"
        if manifest_text.count(anchor) != 1:
            raise RuntimeError("cannot locate slot-link-state dependency in slot/Cargo.toml")
        manifest_text = manifest_text.replace(anchor, anchor + link_kind_dependency + "\n")
        changed_manifest = True
    if video_mode_dependency not in manifest_text:
        anchor = link_kind_dependency + "\n"
        if manifest_text.count(anchor) != 1:
            raise RuntimeError("cannot locate slot-link-kind dependency in slot/Cargo.toml")
        manifest_text = manifest_text.replace(anchor, anchor + video_mode_dependency + "\n")
        changed_manifest = True
    if rom_id_dependency not in manifest_text:
        anchor = video_mode_dependency + "\n"
        if manifest_text.count(anchor) != 1:
            raise RuntimeError("cannot locate slot-video-mode dependency in slot/Cargo.toml")
        manifest_text = manifest_text.replace(anchor, anchor + rom_id_dependency + "\n")
        changed_manifest = True
    if game_search_dependency not in manifest_text:
        anchor = rom_id_dependency + "\n"
        if manifest_text.count(anchor) != 1:
            raise RuntimeError("cannot locate slot-rom-id dependency in slot/Cargo.toml")
        manifest_text = manifest_text.replace(anchor, anchor + game_search_dependency + "\n")
        changed_manifest = True
    if hold_state_dependency not in manifest_text:
        anchor = game_search_dependency + "\n"
        if manifest_text.count(anchor) != 1:
            raise RuntimeError("cannot locate slot-game-search dependency in slot/Cargo.toml")
        manifest_text = manifest_text.replace(anchor, anchor + hold_state_dependency + "\n")
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
    hold_import = "pub use slot_hold_state::{HoldPrompt as FolderHold, TimedHold};" + chr(10)
    app_import_anchor = "use crate::audio::Sfx;\n"
    if app_import_anchor not in app:
        raise RuntimeError("cannot locate App import insertion point")
    if hold_import not in app:
        app = app.replace(app_import_anchor, app_import_anchor + hold_import)
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

    hold_type_start = app.find("#[derive(Clone, Copy, Debug, PartialEq)]\npub struct FolderHold {")
    hold_type_end = app.find("\n}\n", hold_type_start)
    if hold_type_start < 0 or hold_type_end < 0:
        raise RuntimeError("original FolderHold type markers not found in pinned Linkslot source")
    app = app[:hold_type_start] + app[hold_type_end + 3:]

    field_old = "    folder_hold_since: Option<Millis>,\n    folder_hold_fade: Option<(Millis, f32)>,\n"
    field_new = "    folder_hold: TimedHold,\n"
    if app.count(field_old) != 1:
        raise RuntimeError("App folder hold fields do not match pinned source")
    app = app.replace(field_old, field_new)

    init_old = "            folder_hold_since: None,\n            folder_hold_fade: None,\n"
    init_new = "            folder_hold: TimedHold::new(FOLDER_HINT_DELAY_MS, FOLDER_HOLD_MS, FOLDER_HOLD_FADE_MS),\n"
    if app.count(init_old) != 1:
        raise RuntimeError("App folder hold initializer does not match pinned source")
    app = app.replace(init_old, init_new)

    prompt_old = """    pub fn folder_hold_prompt(&self) -> Option<FolderHold> {
        let now = self.now();
        if let Some(since) = self.folder_hold_since {
            let elapsed = now.saturating_sub(since);
            if elapsed < FOLDER_HINT_DELAY_MS {
                return None;
            }
            let span = FOLDER_HOLD_MS - FOLDER_HINT_DELAY_MS;
            let progress = ((elapsed - FOLDER_HINT_DELAY_MS) as f32 / span as f32).clamp(0.0, 1.0);
            return Some(FolderHold {
                progress,
                alpha: 1.0,
            });
        }
        let (at, progress) = self.folder_hold_fade?;
        let gone = now.saturating_sub(at) as f32 / FOLDER_HOLD_FADE_MS as f32;
        (gone < 1.0).then_some(FolderHold {
            progress,
            alpha: 1.0 - gone,
        })
    }
"""
    prompt_new = """    pub fn folder_hold_prompt(&self) -> Option<FolderHold> {
        self.folder_hold.prompt(self.now())
    }
"""
    if app.count(prompt_old) != 1:
        raise RuntimeError("folder_hold_prompt implementation does not match pinned source")
    app = app.replace(prompt_old, prompt_new)

    cancel_old = """    fn cancel_folder_hold(&mut self) {
        self.folder_hold_since = None;
        self.folder_hold_fade = None;
    }
"""
    cancel_new = """    fn cancel_folder_hold(&mut self) {
        self.folder_hold.cancel();
    }
"""
    if app.count(cancel_old) != 1:
        raise RuntimeError("cancel_folder_hold implementation does not match pinned source")
    app = app.replace(cancel_old, cancel_new)

    release_old = """    fn release_folder_hold(&mut self) {
        let now = self.now();
        if let Some(since) = self.folder_hold_since.take() {
            let elapsed = now.saturating_sub(since);
            if elapsed >= FOLDER_HINT_DELAY_MS && elapsed < FOLDER_HOLD_MS {
                let span = FOLDER_HOLD_MS - FOLDER_HINT_DELAY_MS;
                let progress = (elapsed - FOLDER_HINT_DELAY_MS) as f32 / span as f32;
                self.folder_hold_fade = Some((now, progress.clamp(0.0, 1.0)));
            }
        }
    }
"""
    release_new = """    fn release_folder_hold(&mut self) {
        self.folder_hold.release(self.now());
    }
"""
    if app.count(release_old) != 1:
        raise RuntimeError("release_folder_hold implementation does not match pinned source")
    app = app.replace(release_old, release_new)

    tick_old = """    fn tick_folder_hold(&mut self, now: Millis) {
        if let Some((at, _)) = self.folder_hold_fade {
            if now.saturating_sub(at) >= FOLDER_HOLD_FADE_MS {
                self.folder_hold_fade = None;
            }
        }
        let Some(since) = self.folder_hold_since else {
            return;
        };
        if !self.folder_hold_allowed() {
            self.cancel_folder_hold();
            return;
        }
        if now.saturating_sub(since) < FOLDER_HOLD_MS {
            return;
        }
        self.folder_hold_since = None;
        self.folder_hold_fade = None;
        self.toggle_folder_mode();
    }
"""
    tick_new = """    fn tick_folder_hold(&mut self, now: Millis) {
        let allowed = self.folder_hold_allowed();
        if self.folder_hold.tick(now, allowed) {
            self.toggle_folder_mode();
        }
    }
"""
    if app.count(tick_old) != 1:
        raise RuntimeError("tick_folder_hold implementation does not match pinned source")
    app = app.replace(tick_old, tick_new)

    app = app.replace(
        "                self.folder_hold_since = Some(self.now());\n                self.folder_hold_fade = None;\n",
        "                self.folder_hold.begin(self.now());\n",
    )
    app = app.replace(
        "        self.folder_hold_since = None;\n        self.folder_hold_fade = None;\n",
        "        self.folder_hold.cancel();\n",
    )
    app = app.replace("        self.folder_hold_fade = None;\n", "        self.folder_hold.cancel();\n")
    app = app.replace("self.folder_hold_since.is_some()", "self.folder_hold.is_active()")
    if "folder_hold_since" in app or "folder_hold_fade" in app:
        raise RuntimeError("unconverted folder hold fields remain in App")

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
    add_new_package(crate_root / "slot-link-state", source / "crates/slot-link-state")
    add_new_package(crate_root / "slot-link-kind", source / "crates/slot-link-kind")

    add_new_package(crate_root / "slot-video-mode", source / "crates/slot-video-mode")
    add_new_package(crate_root / "slot-rom-id", source / "crates/slot-rom-id")
    add_new_package(crate_root / "slot-game-search", source / "crates/slot-game-search")
    add_new_package(crate_root / "slot-hold-state", source / "crates/slot-hold-state")

    video_mode_path = source / "crates/slot/src/video_mode.rs"
    video_mode_path.write_text(
        """use std::path::Path;

use slot_store::{ini, Platform};
pub use slot_video_mode::VideoMode;

pub const VIDEO_MODE_FILE: &str = "linkSlot/Config/video_mode.txt";

pub fn video_mode_for(root: &Path, stem: &str) -> VideoMode {
    ini::value(root, VIDEO_MODE_FILE, stem)
        .as_deref()
        .and_then(VideoMode::parse)
        .unwrap_or_default()
}

pub fn write_video_mode(root: &Path, stem: &str, mode: VideoMode) -> std::io::Result<()> {
    ini::write(root, VIDEO_MODE_FILE, stem, mode.as_str())
}

pub fn source_rect(platform: Platform, mode: VideoMode) -> [f32; 4] {
    slot_video_mode::source_rect(platform.frame(), platform.picture(), mode)
}
""",
        encoding="utf-8",
    )

    link_kind_path = source / "crates/slot/src/link_kind.rs"
    link_kind_path.write_text(
        "pub use slot_link_kind::{link_carried, link_kind, serial_option, LinkKind};" + chr(10),
        encoding="utf-8",
    )
    link_state_path = source / "crates/slot/src/link_state.rs"
    link_state_path.write_text(
        "pub use slot_link_state::{is_pair, local, pair, same_bios};" + chr(10),
        encoding="utf-8",
    )
    core_picker_path = source / "crates/slot/src/core_picker.rs"
    core_picker_path.write_text(
        "pub use slot_core_picker::{Chip, CorePicker, Outcome, Press, "
        "CLOSE_MS, HOP_MS, LIFT_MS, OPEN_MS, SLIDE_MS};" + chr(10),
        encoding="utf-8",
    )
    game_search_path = source / "crates/slot/src/game_search.rs"
    original_game_search = game_search_path.read_text(encoding="utf-8")
    test_start = original_game_search.find("#[cfg(test)]")
    test_block = original_game_search[test_start:] if test_start >= 0 else ""
    game_search_path.write_text(
        """use slot_game_search::{GameSearchIndex as NoStdGameSearchIndex, PinyinProvider, SearchEntry};
use slot_store::{pinyin, Cart, Platform};

pub use slot_game_search::{CandidatePage, CANDIDATES_PER_PAGE};
pub type SearchHit = slot_game_search::SearchHit<Platform>;

struct SlotStorePinyin;

impl PinyinProvider for SlotStorePinyin {
    fn first_reading(&self, ch: char) -> Option<String> {
        pinyin::first_reading(ch).map(str::to_string)
    }

    fn readings(&self, ch: char) -> Vec<String> {
        pinyin::readings(ch)
            .split(',')
            .filter(|reading| !reading.is_empty())
            .map(str::to_string)
            .collect()
    }
}

#[derive(Clone, Debug, Default)]
pub struct GameSearchIndex(NoStdGameSearchIndex<Platform>);

impl GameSearchIndex {
    pub fn build<'a>(
        carts: impl Iterator<Item = (usize, usize, Platform, &'a Cart)>,
    ) -> GameSearchIndex {
        let entries = carts.map(|(shelf, index, platform, cart)| SearchEntry {
            shelf,
            index,
            platform,
            stem: cart.stem.as_str(),
            title: cart.title.as_str(),
        });
        GameSearchIndex(NoStdGameSearchIndex::build(entries, &SlotStorePinyin))
    }

    pub fn candidates(&self, composition: &str, page: usize, per_page: usize) -> CandidatePage {
        self.0.candidates(composition, page, per_page)
    }

    pub fn hits(&self, query: &str) -> Vec<SearchHit> {
        self.0.hits(query)
    }
}

""" + test_block,
        encoding="utf-8",
    )

    rom_id_path = source / "crates/slot/src/rom_id.rs"
    original_rom_id = rom_id_path.read_text(encoding="utf-8")
    test_start = original_rom_id.find("#[cfg(test)]")
    test_block = original_rom_id[test_start:] if test_start >= 0 else ""
    rom_id_path.write_text(
        """use std::fs::File;
use std::io::Read;
use std::path::Path;

pub use slot_rom_id::{digest_bytes, digest_from_hex, digest_hex, is_sha256, sha256_hex, Digest, RomHasher};

const BUFFER: usize = 1024 * 1024;

pub fn digest_file(path: &Path) -> std::io::Result<Digest> {
    digest_reader(File::open(path)?)
}

pub fn digest_reader<R: Read>(mut reader: R) -> std::io::Result<Digest> {
    let mut hasher = RomHasher::new();
    let mut buffer = vec![0u8; BUFFER];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize())
}

pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    Ok(digest_hex(&digest_file(path)?))
}

pub fn sha256_reader<R: Read>(reader: R) -> std::io::Result<String> {
    Ok(digest_hex(&digest_reader(reader)?))
}

""" + test_block,
        encoding="utf-8",
    )

    patch_frontend(source)

    paths = [
        "crates/slot-input",
        "crates/slot-model",
        "crates/slot-frontend-state",
        "crates/slot-core-picker",
        "crates/slot-link-state",
        "crates/slot-link-kind",
        "crates/slot-video-mode",
        "crates/slot-rom-id",
        "crates/slot-game-search",
        "crates/slot-hold-state",
        "crates/slot/src/game_search.rs",
        "crates/slot/src/rom_id.rs",
        "crates/slot/src/video_mode.rs",
        "crates/slot/src/link_kind.rs",
        "crates/slot/src/link_state.rs",
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
