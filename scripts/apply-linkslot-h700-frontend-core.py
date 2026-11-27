#!/usr/bin/env python3
"""Apply persistent H700 no_std Linkslot business/state crates to a temp source tree."""
from __future__ import annotations

import argparse
import shutil
import subprocess
from pathlib import Path

PINNED_SNAPSHOT = "6122c8f2357ecae43295fdb1a158ff5ef7488325"
UPSTREAM_CRATES = ("slot-input", "slot-model", "slot-i18n")


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

    # QuickRow is a pure row/page model; rendering and QuickValue faces stay in slot-ui.
    quick_ui_manifest = source / "crates/slot-ui/Cargo.toml"
    quick_ui_cargo = quick_ui_manifest.read_text(encoding="utf-8")
    quick_ui_anchor = 'slot-model = { path = "../slot-model" }' + chr(10)
    quick_ui_dependency = 'slot-quick-menu = { path = "../slot-quick-menu" }' + chr(10)
    if quick_ui_dependency not in quick_ui_cargo:
        if quick_ui_cargo.count(quick_ui_anchor) != 1:
            raise RuntimeError("cannot locate slot-model dependency in slot-ui/Cargo.toml")
        quick_ui_manifest.write_text(
            quick_ui_cargo.replace(quick_ui_anchor, quick_ui_anchor + quick_ui_dependency),
            encoding="utf-8",
        )

    quick_menu_path = source / "crates/slot-ui/src/quick_menu.rs"
    quick_menu_text = quick_menu_path.read_text(encoding="utf-8")
    row_start = "#[derive(Copy, Clone, PartialEq, Eq, Debug)]" + chr(10) + "pub enum QuickRow {" + chr(10)
    value_start = "#[derive(Copy, Clone, PartialEq, Eq, Debug)]" + chr(10) + "pub enum QuickValue {" + chr(10)
    if quick_menu_text.count(row_start) != 1 or quick_menu_text.count(value_start) != 1:
        raise RuntimeError("Linkslot QuickRow/QuickValue extraction anchors changed")
    if "pub use slot_quick_menu::{QuickRow, ResumeAction};" in quick_menu_text:
        raise RuntimeError("QuickRow no_std re-export already appears in quick_menu.rs")
    row_at = quick_menu_text.index(row_start)
    value_at = quick_menu_text.index(value_start, row_at)
    quick_menu_text = (
        quick_menu_text[:row_at]
        + "pub use slot_quick_menu::{QuickRow, ResumeAction};" + chr(10) + chr(10)
        + quick_menu_text[value_at:]
    )
    host_import = "use slot_store::{Platform, ResumeKind};" + chr(10)
    if quick_menu_text.count(host_import) != 1:
        raise RuntimeError("QuickRow host-only Platform/ResumeKind import changed")
    quick_menu_text = quick_menu_text.replace(host_import, "")

    # Keep the CPU row visible but strictly unavailable until H700 runtime
    # initialization has an approved board profile and real telemetry.
    value_enum_anchor = "    LanguageEn,\n}\n\nimpl QuickValue {"
    value_enum_new = (
        "    LanguageEn,\n"
        "    /// H700 policy service is not initialized/approved on this target.\n"
        "    CpuPerformanceUnavailable,\n"
        "}\n\nimpl QuickValue {"
    )
    if quick_menu_text.count(value_enum_anchor) != 1:
        raise RuntimeError("QuickValue enum append anchor changed")
    quick_menu_text = quick_menu_text.replace(value_enum_anchor, value_enum_new)

    value_all_anchor = "        QuickValue::LanguageEn,\n    ];"
    value_all_new = "        QuickValue::LanguageEn,\n        QuickValue::CpuPerformanceUnavailable,\n    ];"
    if quick_menu_text.count(value_all_anchor) != 1:
        raise RuntimeError("QuickValue::ALL append anchor changed")
    quick_menu_text = quick_menu_text.replace(value_all_anchor, value_all_new)
    quick_menu_text = quick_menu_text.replace(
        "pub const ALL: [QuickValue; 37]",
        "pub const ALL: [QuickValue; 38]",
    )

    value_text_anchor = (
        '            QuickValue::LanguageEn => slot_i18n::t(lang, "device.quick.value.language_en"),\n'
    )
    value_text_new = value_text_anchor + (
        '            QuickValue::CpuPerformanceUnavailable => match lang {\n'
        '                Language::ZhCn => "暂不可用",\n'
        '                Language::EnUs => "Unavailable",\n'
        '            },\n'
    )
    if quick_menu_text.count(value_text_anchor) != 1:
        raise RuntimeError("QuickValue::text language append anchor changed")
    quick_menu_text = quick_menu_text.replace(value_text_anchor, value_text_new)
    quick_menu_path.write_text(quick_menu_text, encoding="utf-8")

    slot_ui_lib_path = source / "crates/slot-ui/src/lib.rs"
    slot_ui_lib = slot_ui_lib_path.read_text(encoding="utf-8")
    export_anchor = "QuickMenuFaces, QuickRow, QuickValue, QUICK_CARD_H,"
    export_new = "QuickMenuFaces, QuickRow, QuickValue, ResumeAction, QUICK_CARD_H,"
    if slot_ui_lib.count(export_anchor) != 1:
        raise RuntimeError("slot-ui QuickMenu re-export block changed")
    slot_ui_lib_path.write_text(slot_ui_lib.replace(export_anchor, export_new), encoding="utf-8")

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

    quick_row_import_anchor = "QuickMenu, QuickMenuFaces, QuickRow, QuickValue, Refusal"
    quick_row_import_new = "QuickMenu, QuickMenuFaces, QuickRow, QuickValue, ResumeAction, Refusal"
    if app.count(quick_row_import_anchor) != 1:
        raise RuntimeError("App QuickRow import anchor changed")
    app = app.replace(quick_row_import_anchor, quick_row_import_new)

    app_replacements = (
        ("core_picker: Option<CorePicker>,", "core_picker: Option<CorePicker<Core>>,"),
        ("pub fn core_picker_chip(&self) -> Option<Chip> {",
         "pub fn core_picker_chip(&self) -> Option<Chip<Core>> {"),
        ("pub fn core_picker_layout(&self) -> Option<ThreeLayout> {",
         "pub fn core_picker_layout(&self) -> Option<ThreeLayout<Core>> {"),
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

    open_three_old = "CorePicker::open_three(seat, options, alt, now)"
    open_three_new = """{
                let slot2 = if seat == Core::Mgba {
                    alt.filter(|core| *core != Core::Mgba).unwrap_or(Core::Gpsp)
                } else {
                    seat
                };
                let layout = if options.contains(&Core::Mgba)
                    && options.contains(&Core::Gpsp)
                    && options.contains(&Core::Armgba)
                {
                    let tray = if slot2 == Core::Gpsp {
                        Core::Armgba
                    } else {
                        Core::Gpsp
                    };
                    Some(ThreeLayout::new(Core::Mgba, slot2, tray))
                } else {
                    None
                };
                match layout {
                    Some(layout) => CorePicker::open_three(seat, options, layout, now),
                    None => CorePicker::open(seat, options, now),
                }
            }"""
    if app.count(open_three_old) != 1:
        raise RuntimeError("Linkslot 1.0.49 three-core constructor call does not match source")
    app = app.replace(open_three_old, open_three_new)

    default_resume_old = """            QuickRow::DefaultLoad | QuickRow::DefaultSave => row
                .resume_default()
                .map(|kind| QuickValue::flag(self.state.resume_default(kind))),
"""
    default_resume_new = """            QuickRow::DefaultLoad | QuickRow::DefaultSave => row
                .resume_default()
                .map(|action| {
                    let kind = match action {
                        ResumeAction::Load => ResumeKind::Load,
                        ResumeAction::Save => ResumeKind::Save,
                    };
                    QuickValue::flag(self.state.resume_default(kind))
                }),
"""
    if app.count(default_resume_old) != 1:
        raise RuntimeError("App default resume QuickRow adapter anchor changed")
    app = app.replace(default_resume_old, default_resume_new)

    resume_old = """            | QuickRow::SaveGbc => row.resume().map(|(platform, kind)| {
                match self.state.resume_override(platform, kind) {
"""
    resume_new = """            | QuickRow::SaveGbc => row.resume().map(|(platform, action)| {
                let kind = match action {
                    ResumeAction::Load => ResumeKind::Load,
                    ResumeAction::Save => ResumeKind::Save,
                };
                match self.state.resume_override(platform, kind) {
"""
    if app.count(resume_old) != 1:
        raise RuntimeError("App per-platform resume QuickRow adapter anchor changed")
    app = app.replace(resume_old, resume_new)

    cpu_value_old = """            QuickRow::Language => Some(match self.language {
                Language::ZhCn => QuickValue::LanguageZh,
                Language::EnUs => QuickValue::LanguageEn,
            }),
            QuickRow::ButtonLayout => None,
"""
    cpu_value_new = """            QuickRow::Language => Some(match self.language {
                Language::ZhCn => QuickValue::LanguageZh,
                Language::EnUs => QuickValue::LanguageEn,
            }),
            QuickRow::CpuPerformance => Some(QuickValue::CpuPerformanceUnavailable),
            QuickRow::ButtonLayout => None,
"""
    if app.count(cpu_value_old) != 1:
        raise RuntimeError("App CPU Performance quick_value anchor changed")
    app = app.replace(cpu_value_old, cpu_value_new)

    cpu_change_old = "        if row == QuickRow::Language {\n"
    cpu_change_new = (
        "        if row == QuickRow::CpuPerformance {\n"
        "            // No approved H700 runtime/profile: this row is display-only.\n"
        "            // Do not persist a request or imply that hardware changed.\n"
        "            return;\n"
        "        }\n"
        "        if row == QuickRow::Language {\n"
    )
    if app.count(cpu_change_old) != 1:
        raise RuntimeError("App CPU Performance change guard anchor changed")
    app = app.replace(cpu_change_old, cpu_change_new)

    cpu_match_old = "            | QuickRow::Language => return,\n"
    cpu_match_new = "            | QuickRow::CpuPerformance\n            | QuickRow::Language => return,\n"
    if app.count(cpu_match_old) != 1:
        raise RuntimeError("App CPU Performance exhaustive match anchor changed")
    app = app.replace(cpu_match_old, cpu_match_new)

    open_row_old = (
        "            | QuickRow::LidTimeout" + chr(10)
        + "            | QuickRow::Language" + chr(10)
        + "            | QuickRow::Rewind" + chr(10)
    )
    open_row_new = (
        "            | QuickRow::LidTimeout" + chr(10)
        + "            | QuickRow::Language" + chr(10)
        + "            | QuickRow::CpuPerformance" + chr(10)
        + "            | QuickRow::Rewind" + chr(10)
    )
    if app.count(open_row_old) != 1:
        raise RuntimeError("App open_quick_row CPU no-op anchor changed")
    app = app.replace(open_row_old, open_row_new)

    cpu_quick_row_tests = r"""
#[cfg(test)]
mod h700_cpu_performance_quick_row_tests {
    use super::*;

    #[test]
    fn cpu_performance_row_is_visible_but_explicitly_unavailable() {
        let mut app = App::new(Vec::new());
        assert_eq!(
            QuickValue::ALL.len(),
            38,
            "CPU unavailable value must append without shifting previous values"
        );
        assert_eq!(QuickValue::CpuPerformanceUnavailable.index(), 37);
        assert_eq!(
            app.quick_value(QuickRow::CpuPerformance),
            Some(QuickValue::CpuPerformanceUnavailable)
        );
        assert!(!QuickRow::CpuPerformance.opens());
        app.change_setting(QuickRow::CpuPerformance, true);
        app.change_setting(QuickRow::CpuPerformance, false);
        assert_eq!(
            app.quick_value(QuickRow::CpuPerformance),
            Some(QuickValue::CpuPerformanceUnavailable)
        );
    }

    #[test]
    fn unavailable_cpu_performance_request_does_not_write_to_storage() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = App::new(Vec::new());
        app.root = Some(dir.path().to_path_buf());

        app.change_setting(QuickRow::CpuPerformance, true);
        app.change_setting(QuickRow::CpuPerformance, false);

        assert_eq!(
            std::fs::read_dir(dir.path()).unwrap().count(),
            0,
            "an unavailable performance request must not create or persist settings"
        );
    }
}
"""
    if "mod h700_cpu_performance_quick_row_tests" in app:
        raise RuntimeError("CPU QuickRow regression tests already injected")
    app += chr(10) + cpu_quick_row_tests

    app_path.write_text(app, encoding="utf-8")



def patch_session_power_led(source: Path) -> None:
    """Tie the RG34XX PI12 work LED to Linkslot's actual game session."""
    path = source / "crates/slot/src/session.rs"
    text = path.read_text(encoding="utf-8")

    import_anchor = "use crate::prefetch::Prefetch;\n"
    if text.count(import_anchor) != 1:
        raise RuntimeError("unexpected Linkslot Session imports")
    rust_ffi = """
#[cfg(target_os = "none")]
extern "C" {
    fn h700_rg34xx_power_led_set_game_active(active: u32) -> i32;
}

fn set_h700_power_led_game_active(active: bool) -> i32 {
    #[cfg(target_os = "none")]
    {
        // The C service retries initialization when the early target hook
        // could not access PIO yet.
        unsafe { h700_rg34xx_power_led_set_game_active(u32::from(active)) }
    }
    #[cfg(not(target_os = "none"))]
    {
        let _ = active;
        0
    }
}
"""
    text = text.replace(import_anchor, import_anchor + rust_ffi, 1)

    field_anchor = "    driven: bool,\n"
    if text.count(field_anchor) != 1:
        raise RuntimeError("unexpected Linkslot Session fields")
    text = text.replace(
        field_anchor,
        field_anchor + "    power_led: slot_frontend_state::GamePowerLedState,\n",
        1,
    )

    init_anchor = "            driven: false,\n"
    if text.count(init_anchor) != 1:
        raise RuntimeError("unexpected Linkslot Session initializer")
    text = text.replace(
        init_anchor,
        init_anchor + "            power_led: slot_frontend_state::GamePowerLedState::default(),\n",
        1,
    )

    feed_anchor = "        self.sync_pad();\n    }\n\n    fn sync_pad(&mut self) {"
    if text.count(feed_anchor) != 1:
        raise RuntimeError("unexpected Linkslot input-feed tail")
    text = text.replace(
        feed_anchor,
        "        self.sync_power_led();\n" + feed_anchor,
        1,
    )

    update_anchor = "    pub fn update(&mut self, dt: f32) {\n        self.bridge_link(|app| app.update(dt));\n"
    if text.count(update_anchor) != 1:
        raise RuntimeError("unexpected Linkslot Session update entry")
    method = """    /// Keep the work LED aligned with game lifecycle transitions. The
    /// no_std state suppresses repeat requests, so MMIO is called only once
    /// on each transition rather than on every rendered frame.
    fn sync_power_led(&mut self) {
        let active = matches!(
            self.app.phase(),
            Phase::Playing { .. } | Phase::Polaroids { .. }
        );
        if let Some(active) = self.power_led.request_game_active(active) {
            let result = set_h700_power_led_game_active(active);
            if result != 0 {
                eprintln!("slot: RG34XX power LED transition failed: {result}");
            }
        }
    }

"""
    text = text.replace(update_anchor, method + update_anchor, 1)
    text = text.replace(
        update_anchor,
        update_anchor + "        self.sync_power_led();\n",
        1,
    )

    path.write_text(text, encoding="utf-8")

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
    if 'version = "1.0.49"' not in workspace:
        raise RuntimeError("source workspace is not Linkslot 1.0.49")
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
    add_new_package(crate_root / "slot-quick-menu", source / "crates/slot-quick-menu")

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
        "pub use slot_link_kind::{armgba_link_carried, armgba_port, link_carried, link_kind, serial_option, LinkKind};" + chr(10),
        encoding="utf-8",
    )
    link_state_path = source / "crates/slot/src/link_state.rs"
    link_state_path.write_text(
        "pub use slot_link_state::{is_pair, local, pair, same_bios};" + chr(10),
        encoding="utf-8",
    )
    core_picker_path = source / "crates/slot/src/core_picker.rs"
    core_picker_path.write_text(
        "pub use slot_core_picker::{Chip, CorePicker, Outcome, Press, ThreeLayout, "
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
    patch_session_power_led(source)

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
        "crates/slot-i18n",
        "crates/slot-quick-menu",
        "crates/slot-ui/Cargo.toml",
        "crates/slot-ui/src/quick_menu.rs",
        "crates/slot-ui/src/lib.rs",
        "crates/slot/src/game_search.rs",
        "crates/slot/src/rom_id.rs",
        "crates/slot/src/video_mode.rs",
        "crates/slot/src/link_kind.rs",
        "crates/slot/src/link_state.rs",
        "crates/slot/Cargo.toml",
        "crates/slot/src/frontend.rs",
        "crates/slot/src/app.rs",
        "crates/slot/src/core_picker.rs",
        "crates/slot/src/session.rs",
    ]
    changed = run_git(source, "status", "--short", "--", *paths)
    if not changed:
        raise RuntimeError("business/state overlay produced no source changes")
    run_git(source, "diff", "--check", "--", *paths)
    print("Applied persistent H700 no_std Linkslot packages to the isolated 1.0.49 checkout:")
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
