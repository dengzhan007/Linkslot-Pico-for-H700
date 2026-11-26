#!/usr/bin/env python3
"""Wire the pure H700 Linkslot App router into priority-safe 1.0.49 App paths."""
from __future__ import annotations

import argparse
import shutil
import subprocess
from pathlib import Path

PINNED_SNAPSHOT = "241e7c1974926fa22552b200852914ceaccbfd42"


def run_git(source: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(source), *args],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    return result.stdout.strip()


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected one patch anchor, found {count}")
    return text.replace(old, new, 1)


def add_router_dependency(source: Path) -> None:
    manifest = source / "crates/slot/Cargo.toml"
    content = manifest.read_text(encoding="utf-8")
    anchor = 'slot-model = { path = "../slot-model" }\n'
    dependency = 'slot-app-router = { path = "../slot-app-router" }\n'
    if dependency not in content:
        content = replace_once(content, anchor, anchor + dependency, "slot dependency")
        manifest.write_text(content, encoding="utf-8")


def apply_app_patch(source: Path) -> None:
    app_path = source / "crates/slot/src/app.rs"
    content = app_path.read_text(encoding="utf-8")

    import_anchor = "use slot_input::{Action, Btn, MUTE_CHORD_MS};\n"
    imports = """use slot_app_router::{
    route_app_input, AppInputCommand, AppInputOwner, KeyboardDirection, QuickMenuCommand,
    SearchKeyboardCommand, SearchKeyboardSnapshot, SearchResultsCommand,
};
"""
    content = replace_once(content, import_anchor, import_anchor + imports, "App router imports")

    helpers = r'''
    /// Dispatch an action only after App::apply has selected the owning phase.
    /// All effects remain in the existing App methods and services.
    fn dispatch_app_input(&mut self, owner: AppInputOwner, action: Action) -> bool {
        match route_app_input(owner, action) {
            AppInputCommand::DelegateToApp => false,
            AppInputCommand::OpenQuickMenu => {
                self.open_quick_menu();
                true
            }
            AppInputCommand::OpenGameSearch => {
                self.shelf_mut().release_hold();
                self.play_held = None;
                self.open_game_search();
                true
            }
            AppInputCommand::OpenInGameMenu => {
                self.utility = Some((0, 0));
                true
            }
            AppInputCommand::QuickMenu(command) => {
                self.apply_quick_menu_command(command);
                true
            }
            AppInputCommand::SearchResults(command) => {
                self.apply_search_results_command(command);
                true
            }
            AppInputCommand::SearchKeyboard(command) => {
                self.apply_search_keyboard_command(command);
                true
            }
        }
    }

    fn apply_quick_menu_command(&mut self, command: QuickMenuCommand) {
        match command {
            QuickMenuCommand::CursorMoved { row } | QuickMenuCommand::Parent { row } => {
                self.phase = Phase::QuickMenu { row };
            }
            QuickMenuCommand::ChangeSetting { row, right } => {
                self.change_setting(row, right);
            }
            QuickMenuCommand::OpenRow { row } => {
                self.open_quick_row(row);
            }
            QuickMenuCommand::Close => {
                self.phase = Phase::Shelf;
            }
            QuickMenuCommand::Ignore => {}
        }
    }

    fn apply_search_results_command(&mut self, command: SearchResultsCommand) {
        let lang = self.language;
        match command {
            SearchResultsCommand::SelectionChanged { selected, offset } => {
                if let Some(results) = self.search_results.as_mut() {
                    results.selected = selected;
                    results.offset = offset;
                }
            }
            SearchResultsCommand::SelectHit { index } => {
                let hit = self
                    .search_results
                    .as_ref()
                    .and_then(|results| results.hits.get(index))
                    .cloned();
                if let Some(hit) = hit {
                    self.search_results = None;
                    self.search_index = None;
                    self.select_search_hit(hit);
                }
            }
            SearchResultsCommand::ReturnToKeyboard => {
                let query = self.search_results.as_ref().map(|results| results.query.clone());
                self.search_results = None;
                self.search_index = None;
                self.open_game_search();
                if let (Some(query), Some(kb)) = (query, self.keyboard.as_mut()) {
                    kb.text = query;
                    kb.hint = search_keyboard_hint(lang);
                }
            }
            SearchResultsCommand::Close => {
                self.close_game_search();
            }
            SearchResultsCommand::Ignore => {}
        }
    }

    fn apply_search_keyboard_command(&mut self, command: SearchKeyboardCommand) {
        let lang = self.language;
        let mut confirm = false;
        let mut cancel_keyboard = false;
        let mut close_search = false;

        if let (Some(index), Some(kb)) = (self.search_index.as_ref(), self.keyboard.as_mut()) {
            match command {
                SearchKeyboardCommand::CandidateFocusChanged { focused } => {
                    kb.candidate_focus = focused;
                    kb.hint = if focused {
                        search_candidate_hint(lang)
                    } else {
                        search_keyboard_hint(lang)
                    };
                }
                SearchKeyboardCommand::CandidatePageChanged { page } => {
                    kb.candidate_page = page;
                    refresh_search_candidates(kb, index, lang);
                }
                SearchKeyboardCommand::CandidateSelectionChanged { selected } => {
                    kb.candidate_selected = selected;
                }
                SearchKeyboardCommand::CommitCandidate { index: candidate } => {
                    if candidate < kb.candidates.len()
                        && candidate == kb.candidate_selected
                    {
                        commit_search_candidate(kb, lang);
                    }
                }
                SearchKeyboardCommand::MoveCursor { direction } => {
                    kb.cursor = match direction {
                        KeyboardDirection::Up => kb.cursor.up(),
                        KeyboardDirection::Down => kb.cursor.down(),
                        KeyboardDirection::Left => kb.cursor.left(),
                        KeyboardDirection::Right => kb.cursor.right(),
                    };
                }
                SearchKeyboardCommand::CycleLayout => {
                    kb.cursor = kb.cursor.cycle_layout();
                }
                SearchKeyboardCommand::ActivateCurrentKey => match kb.cursor.key() {
                    slot_ui::keyboard::Key::Char(c) => {
                        search_push(kb, c);
                        refresh_search_candidates(kb, index, lang);
                    }
                    slot_ui::keyboard::Key::Space => {
                        search_push(kb, ' ');
                        refresh_search_candidates(kb, index, lang);
                    }
                    slot_ui::keyboard::Key::Backspace => {
                        search_backspace(kb);
                        refresh_search_candidates(kb, index, lang);
                    }
                    slot_ui::keyboard::Key::Layout => {
                        kb.cursor = kb.cursor.cycle_layout();
                    }
                    slot_ui::keyboard::Key::Confirm => confirm = true,
                    // Preserve the upstream on-screen Cancel key behavior:
                    // drop this keyboard without invoking the shelf reset path.
                    slot_ui::keyboard::Key::Cancel => cancel_keyboard = true,
                },
                SearchKeyboardCommand::Backspace => {
                    search_backspace(kb);
                    refresh_search_candidates(kb, index, lang);
                }
                SearchKeyboardCommand::ConfirmSearch => confirm = true,
                SearchKeyboardCommand::CloseSearch => close_search = true,
                SearchKeyboardCommand::Ignore => {}
            }
        }

        if close_search {
            self.close_game_search();
        } else if cancel_keyboard {
            self.keyboard = None;
            self.search_index = None;
        } else if confirm {
            self.search_confirm();
        }
    }

'''
    apply_anchor = "    pub fn apply(&mut self, action: Action) {\n"
    content = replace_once(content, apply_anchor, helpers + apply_anchor, "App command adapter")

    old_results_keyboard = """        if self.search_results.is_some() {
            return self.search_results_input(action);
        }
        if self.keyboard.is_some() {
            return self.keyboard_input(action);
        }
"""
    new_results_keyboard = """        if let Some(results) = self.search_results.as_ref() {
            let owner = AppInputOwner::SearchResults {
                hit_count: results.hits.len(),
                selected: results.selected,
                offset: results.offset,
            };
            self.dispatch_app_input(owner, action);
            return;
        }
        if self.keyboard.is_some() {
            let search_state = self.keyboard.as_ref().and_then(|kb| {
                if matches!(kb.purpose, KeyboardPurpose::GameSearch) {
                    Some(SearchKeyboardSnapshot {
                        candidate_focus: kb.candidate_focus,
                        composition_nonempty: !kb.composition.is_empty(),
                        candidate_count: kb.candidates.len(),
                        candidate_page: kb.candidate_page,
                        candidate_pages: kb.candidate_pages,
                        candidate_selected: kb.candidate_selected,
                    })
                } else {
                    None
                }
            });
            if let Some(state) = search_state {
                self.dispatch_app_input(AppInputOwner::SearchKeyboard { state }, action);
                return;
            }
            return self.keyboard_input(action);
        }
"""
    content = replace_once(content, old_results_keyboard, new_results_keyboard, "Search phase dispatch")

    shelf_start = "            Phase::Shelf => match action {\n"
    shelf_new = """            Phase::Shelf => {
                // CorePicker owns every Shelf action while it is open.
                if self.core_picker.is_none()
                    && self.dispatch_app_input(AppInputOwner::Shelf, action)
                {
                    return;
                }
                match action {
"""
    content = replace_once(content, shelf_start, shelf_new, "Shelf phase entry")
    shelf_end = "            },\n            Phase::Inserting { .. } if action == Action::Eject => self.eject(),\n"
    shelf_end_new = "                }\n            }\n            Phase::Inserting { .. } if action == Action::Eject => self.eject(),\n"
    content = replace_once(content, shelf_end, shelf_end_new, "Shelf phase exit")

    playing_start = "            Phase::Playing { .. } => match action {\n"
    playing_new = """            Phase::Playing { .. } => {
                if self.dispatch_app_input(AppInputOwner::Playing, action) {
                    return;
                }
                match action {
"""
    content = replace_once(content, playing_start, playing_new, "Playing phase entry")
    playing_end = "            },\n            Phase::Polaroids { .. } => match action {\n"
    playing_end_new = "                }\n            }\n            Phase::Polaroids { .. } => match action {\n"
    content = replace_once(content, playing_end, playing_end_new, "Playing phase exit")

    quick_menu_arm = "            Phase::QuickMenu { row } => self.quick_menu_input(row, action),\n"
    quick_menu_new = """            Phase::QuickMenu { row } => {
                self.dispatch_app_input(AppInputOwner::QuickMenu { row }, action);
            }
"""
    content = replace_once(content, quick_menu_arm, quick_menu_new, "QuickMenu phase dispatch")

    app_path.write_text(content, encoding="utf-8")


def apply(source: Path, crate: Path) -> None:
    if not source.is_dir() or not crate.is_dir():
        raise RuntimeError("source and persistent router crate must be directories")
    snapshot = run_git(source, "rev-parse", "HEAD")
    if snapshot != PINNED_SNAPSHOT:
        raise RuntimeError(
            f"refusing source drift: expected {PINNED_SNAPSHOT}, got {snapshot}"
        )

    destination = source / "crates/slot-app-router"
    if destination.exists():
        shutil.rmtree(destination)
    shutil.copytree(crate, destination)

    add_router_dependency(source)
    apply_app_patch(source)

    paths = [
        "crates/slot-app-router",
        "crates/slot/Cargo.toml",
        "crates/slot/src/app.rs",
    ]
    changed = run_git(source, "status", "--short", "--", *paths)
    if not changed:
        raise RuntimeError("App router integration produced no source changes")
    run_git(source, "diff", "--check", "--", *paths)
    print("Applied the phase-aware slot-app-router to the isolated Linkslot 1.0.49 tree:")
    print(changed)
    print("App::apply priority gates remain in place; App retains all command side effects.")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", required=True, type=Path)
    parser.add_argument("--crate", required=True, type=Path)
    args = parser.parse_args()
    apply(args.source, args.crate)


if __name__ == "__main__":
    main()
