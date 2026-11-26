#!/usr/bin/env python3
"""Static integration guard for the real Linkslot 1.0.49 App::apply route."""
from __future__ import annotations

import argparse
from pathlib import Path


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(f"FAIL: {message}")


def ordered(text: str, markers: list[str], label: str) -> None:
    positions = [text.find(marker) for marker in markers]
    require(all(position >= 0 for position in positions), f"{label}: missing marker")
    require(positions == sorted(positions), f"{label}: priority order changed")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", required=True, type=Path)
    args = parser.parse_args()
    source = args.source
    app_path = source / "crates/slot/src/app.rs"
    manifest_path = source / "crates/slot/Cargo.toml"
    router_path = source / "crates/slot-app-router/src/lib.rs"
    require(app_path.is_file(), "Linkslot app.rs missing")
    require(manifest_path.is_file(), "Linkslot slot manifest missing")
    require(router_path.is_file(), "persistent slot-app-router was not copied")

    app = app_path.read_text(encoding="utf-8")
    manifest = manifest_path.read_text(encoding="utf-8")
    router = router_path.read_text(encoding="utf-8")
    require(
        'slot-app-router = { path = "../slot-app-router" }' in manifest,
        "slot-app-router dependency missing from Frontend",
    )
    require("route_app_input, AppInputCommand, AppInputOwner" in app,
            "App router imports missing")
    require("fn dispatch_app_input(&mut self, owner: AppInputOwner, action: Action) -> bool" in app,
            "App-owned dispatcher adapter missing")

    adapter_start = app.find("fn dispatch_app_input(")
    apply_start = app.find("pub fn apply(&mut self, action: Action)")
    require(adapter_start >= 0 and apply_start > adapter_start,
            "dispatcher adapter must be defined before App::apply")
    adapter = app[adapter_start:apply_start]
    for variant in (
        "DelegateToApp", "OpenQuickMenu", "OpenGameSearch", "OpenInGameMenu",
        "QuickMenu(command)", "SearchResults(command)", "SearchKeyboard(command)",
    ):
        require(f"AppInputCommand::{variant}" in adapter,
                f"App adapter does not handle {variant}")
    for marker in (
        "self.change_setting(row, right)",
        "self.open_quick_row(row)",
        "self.select_search_hit(hit)",
        "self.search_confirm()",
        "self.close_game_search()",
        "refresh_search_candidates(kb, index, lang)",
        "commit_search_candidate(kb, lang)",
    ):
        require(marker in adapter, f"existing App handler delegation missing: {marker}")

    apply_end = app.find("fn open_quick_menu(", apply_start)
    require(apply_start >= 0 and apply_end > apply_start, "cannot isolate App::apply")
    apply = app[apply_start:apply_end]
    ordered(
        apply,
        [
            "if self.power_menu.is_some()",
            "if self.upgrade_ui.is_some()",
            "if self.game_intro.is_some()",
            "if self.sync_insert.is_some()",
            "if !matches!(self.shelf_sync, ShelfSync::Idle)",
            "if let Some(results) = self.search_results.as_ref()",
            "if self.keyboard.is_some()",
            "if self.web_popup.is_some()",
            "if self.link_popup.is_some()",
            "if self.game_menu.is_some()",
            "match self.phase",
        ],
        "App::apply modal/overlay priority",
    )
    require(
        "AppInputOwner::SearchResults" in apply
        and "AppInputOwner::SearchKeyboard { state }" in apply,
        "search overlay phases are not routed through the dispatcher",
    )
    require(
        "if matches!(kb.purpose, KeyboardPurpose::GameSearch)" in apply,
        "non-search keyboard must remain on the legacy keyboard handler",
    )
    require(
        "self.core_picker.is_none()\n                    && self.dispatch_app_input(AppInputOwner::Shelf, action)"
        in apply,
        "Shelf CorePicker priority gate missing",
    )
    require("self.dispatch_app_input(AppInputOwner::Playing, action)" in apply,
            "Playing phase is not routed")
    require("self.dispatch_app_input(AppInputOwner::QuickMenu { row }, action)" in apply,
            "QuickMenu phase is not routed")
    require(
        apply.find("if self.game_menu.is_some()")
        < apply.find("self.dispatch_app_input(AppInputOwner::QuickMenu { row }, action)"),
        "game-menu handling must precede QuickMenu dispatch",
    )

    for owner in ("SearchResults", "SearchKeyboard", "QuickMenu", "Shelf", "Playing"):
        require(f"AppInputOwner::{owner}" in router,
                f"pure router owner missing: {owner}")
    print("PASS: Linkslot 1.0.49 App::apply invokes the phase-aware router")
    print("PASS: power/upgrade/intro/sync/search/modal/game-menu precedence is preserved")
    print("PASS: Shelf CorePicker remains ahead of Shelf routing")
    print("PASS: command execution stays in the existing App methods")


if __name__ == "__main__":
    main()
