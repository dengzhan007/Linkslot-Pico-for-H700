#![no_std]

//! Pure application-level routing for the upstream Quick Menu phase.
//!
//! The caller decides when the application is in its Quick Menu phase and
//! owns every side effect. This module preserves the upstream 1.0.49 routing
//! contract: D-pad moves the cursor, left/right change the selected row, A
//! opens a row, and B/MENU returns to the parent or closes the root menu.

use slot_input::{Action, Btn};
use slot_quick_menu::QuickRow;

/// A command for the application owner to execute after routing one action.
/// No command performs storage, display, audio, or hardware work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuickMenuCommand {
    /// The visible cursor changed to this row.
    CursorMoved { row: QuickRow },
    /// Delegate a left/right value change to the application settings owner.
    ChangeSetting { row: QuickRow, right: bool },
    /// Delegate opening the selected row to the application owner.
    OpenRow { row: QuickRow },
    /// Navigate from a child page to its parent row.
    Parent { row: QuickRow },
    /// The current root row has no parent; leave the Quick Menu.
    Close,
    /// The action is not handled by the Quick Menu phase.
    Ignore,
}

/// Pure cursor/action router for the upstream Linkslot Quick Menu phase.
///
/// Construct it when the application opens the Quick Menu, or restore it
/// with at(row) when reconstructing an existing QuickMenu phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuickMenuRouter {
    row: QuickRow,
}

impl Default for QuickMenuRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl QuickMenuRouter {
    /// Match upstream App::open_quick_menu() (QuickRow::MAIN[0]).
    pub const fn new() -> Self {
        Self { row: QuickRow::MAIN[0] }
    }

    /// Restore the router from an application-owned Quick Menu cursor.
    pub const fn at(row: QuickRow) -> Self {
        Self { row }
    }

    pub const fn row(&self) -> QuickRow {
        self.row
    }

    /// Route one action using upstream App::quick_menu_input() semantics.
    ///
    /// The caller remains responsible for invoking change_setting or
    /// open_quick_row and for applying the resulting application phase.
    pub fn route(&mut self, action: Action) -> QuickMenuCommand {
        match action {
            Action::GbaDown(Btn::Up) => {
                self.row = self.row.up();
                QuickMenuCommand::CursorMoved { row: self.row }
            }
            Action::GbaDown(Btn::Down) => {
                self.row = self.row.down();
                QuickMenuCommand::CursorMoved { row: self.row }
            }
            Action::GbaDown(Btn::Left) => QuickMenuCommand::ChangeSetting {
                row: self.row,
                right: false,
            },
            Action::GbaDown(Btn::Right) => QuickMenuCommand::ChangeSetting {
                row: self.row,
                right: true,
            },
            Action::GbaDown(Btn::A) => QuickMenuCommand::OpenRow { row: self.row },
            Action::GbaDown(Btn::B) | Action::QuickMenu => match self.row.parent() {
                Some(parent) => {
                    self.row = parent;
                    QuickMenuCommand::Parent { row: parent }
                }
                None => QuickMenuCommand::Close,
            },
            _ => QuickMenuCommand::Ignore,
        }
    }
}

/// Visible result rows per page, copied from the pinned Linkslot 1.0.49 App.
pub const SEARCH_ROWS: usize = 7;

/// A command for the application owner after routing one search-results action.
/// Query text and SearchHit values remain owned by the application.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchResultsCommand {
    SelectionChanged { selected: usize, offset: usize },
    SelectHit { index: usize },
    ReturnToKeyboard,
    Close,
    Ignore,
}

/// Allocation-free cursor and viewport state for the search-results page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchResultsRouter {
    hit_count: usize,
    selected: usize,
    offset: usize,
}

impl SearchResultsRouter {
    /// Match App::search_confirm(): first result selected, viewport at zero.
    pub const fn new(hit_count: usize) -> Self {
        Self { hit_count, selected: 0, offset: 0 }
    }

    /// Restore selection/scroll state when the application recreates the router.
    pub const fn at(hit_count: usize, selected: usize, offset: usize) -> Self {
        let selected = if hit_count == 0 { 0 } else if selected >= hit_count { hit_count - 1 } else { selected };
        let offset = if selected < offset { selected } else if selected >= offset.saturating_add(SEARCH_ROWS) { selected + 1 - SEARCH_ROWS } else { offset };
        Self { hit_count, selected, offset }
    }

    pub const fn selected(&self) -> usize { self.selected }
    pub const fn offset(&self) -> usize { self.offset }
    pub const fn hit_count(&self) -> usize { self.hit_count }

    fn keep_selected_visible(&mut self) {
        if self.selected < self.offset {
            self.offset = self.selected;
        }
        if self.selected >= self.offset.saturating_add(SEARCH_ROWS) {
            self.offset = self.selected + 1 - SEARCH_ROWS;
        }
    }

    /// Route search-results actions using upstream Linkslot 1.0.49 semantics.
    /// The caller retains the query and owns SearchHit-to-shelf selection.
    pub fn route(&mut self, action: Action) -> SearchResultsCommand {
        match action {
            Action::GbaDown(Btn::Up) | Action::GbaDown(Btn::Down) => {
                if self.hit_count == 0 {
                    return SearchResultsCommand::Ignore;
                }
                let last = self.hit_count - 1;
                self.selected = match action {
                    Action::GbaDown(Btn::Up) if self.selected == 0 => last,
                    Action::GbaDown(Btn::Up) => self.selected - 1,
                    _ if self.selected >= last => 0,
                    _ => self.selected + 1,
                };
                self.keep_selected_visible();
                SearchResultsCommand::SelectionChanged { selected: self.selected, offset: self.offset }
            }
            Action::GbaDown(Btn::Left) | Action::GbaDown(Btn::Right) => {
                if self.hit_count == 0 {
                    return SearchResultsCommand::Ignore;
                }
                self.selected = match action {
                    Action::GbaDown(Btn::Left) => self.selected.saturating_sub(SEARCH_ROWS),
                    _ => self.selected.saturating_add(SEARCH_ROWS).min(self.hit_count - 1),
                };
                self.keep_selected_visible();
                SearchResultsCommand::SelectionChanged { selected: self.selected, offset: self.offset }
            }
            Action::GbaDown(Btn::A) | Action::Insert => {
                if self.hit_count == 0 {
                    SearchResultsCommand::Ignore
                } else {
                    SearchResultsCommand::SelectHit { index: self.selected }
                }
            }
            Action::GbaDown(Btn::B) => SearchResultsCommand::ReturnToKeyboard,
            Action::GbaDown(Btn::X) | Action::QuickMenu => SearchResultsCommand::Close,
            _ => SearchResultsCommand::Ignore,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_router_matches_upstream_quick_menu_entry_row() {
        let router = QuickMenuRouter::new();
        assert_eq!(router.row(), QuickRow::MAIN[0]);
    }

    #[test]
    fn dpad_moves_with_the_existing_quick_row_navigation_table() {
        let mut router = QuickMenuRouter::at(QuickRow::SystemSettings);
        assert_eq!(
            router.route(Action::GbaDown(Btn::Up)),
            QuickMenuCommand::CursorMoved { row: QuickRow::Network }
        );
        assert_eq!(router.row(), QuickRow::Network);

        assert_eq!(
            router.route(Action::GbaDown(Btn::Down)),
            QuickMenuCommand::CursorMoved { row: QuickRow::SystemSettings }
        );
        assert_eq!(router.row(), QuickRow::SystemSettings);
    }

    #[test]
    fn left_right_delegate_setting_changes_without_moving_cursor() {
        let mut router = QuickMenuRouter::at(QuickRow::Language);
        assert_eq!(
            router.route(Action::GbaDown(Btn::Left)),
            QuickMenuCommand::ChangeSetting { row: QuickRow::Language, right: false }
        );
        assert_eq!(
            router.route(Action::GbaDown(Btn::Right)),
            QuickMenuCommand::ChangeSetting { row: QuickRow::Language, right: true }
        );
        assert_eq!(router.row(), QuickRow::Language);
    }

    #[test]
    fn confirm_delegates_row_open_without_guessing_row_side_effects() {
        let mut router = QuickMenuRouter::at(QuickRow::Network);
        assert_eq!(
            router.route(Action::GbaDown(Btn::A)),
            QuickMenuCommand::OpenRow { row: QuickRow::Network }
        );
        assert_eq!(router.row(), QuickRow::Network);
    }

    #[test]
    fn back_and_menu_return_to_parent_then_close_at_root() {
        let mut router = QuickMenuRouter::at(QuickRow::Wifi);
        assert_eq!(
            router.route(Action::GbaDown(Btn::B)),
            QuickMenuCommand::Parent { row: QuickRow::Network }
        );
        assert_eq!(router.row(), QuickRow::Network);

        assert_eq!(router.route(Action::QuickMenu), QuickMenuCommand::Close);
        assert_eq!(router.row(), QuickRow::Network);
    }

    #[test]
    fn unrelated_actions_are_ignored_without_changing_cursor() {
        let mut router = QuickMenuRouter::at(QuickRow::DateTime);
        assert_eq!(router.route(Action::PowerPress), QuickMenuCommand::Ignore);
        assert_eq!(router.row(), QuickRow::DateTime);
    }

    #[test]
    fn search_results_vertical_navigation_wraps_and_tracks_the_seven_row_viewport() {
        let mut router = SearchResultsRouter::new(17);
        assert_eq!(router.route(Action::GbaDown(Btn::Up)), SearchResultsCommand::SelectionChanged { selected: 16, offset: 10 });
        assert_eq!(router.selected(), 16);
        assert_eq!(router.offset(), 10);
        assert_eq!(router.route(Action::GbaDown(Btn::Down)), SearchResultsCommand::SelectionChanged { selected: 0, offset: 0 });
    }

    #[test]
    fn search_results_horizontal_navigation_moves_by_one_page_and_clamps() {
        let mut router = SearchResultsRouter::new(17);
        assert_eq!(router.route(Action::GbaDown(Btn::Right)), SearchResultsCommand::SelectionChanged { selected: 7, offset: 1 });
        assert_eq!(router.route(Action::GbaDown(Btn::Right)), SearchResultsCommand::SelectionChanged { selected: 14, offset: 8 });
        assert_eq!(router.route(Action::GbaDown(Btn::Right)), SearchResultsCommand::SelectionChanged { selected: 16, offset: 10 });
        assert_eq!(router.route(Action::GbaDown(Btn::Left)), SearchResultsCommand::SelectionChanged { selected: 9, offset: 9 });
        assert_eq!(router.route(Action::GbaDown(Btn::Left)), SearchResultsCommand::SelectionChanged { selected: 2, offset: 2 });
        assert_eq!(router.route(Action::GbaDown(Btn::Left)), SearchResultsCommand::SelectionChanged { selected: 0, offset: 0 });
    }

    #[test]
    fn search_results_a_and_insert_select_the_current_hit() {
        let mut router = SearchResultsRouter::at(20, 9, 7);
        assert_eq!(router.route(Action::GbaDown(Btn::A)), SearchResultsCommand::SelectHit { index: 9 });
        assert_eq!(router.route(Action::Insert), SearchResultsCommand::SelectHit { index: 9 });
    }

    #[test]
    fn search_results_back_returns_to_keyboard_but_x_and_menu_close() {
        let mut router = SearchResultsRouter::new(3);
        assert_eq!(router.route(Action::GbaDown(Btn::B)), SearchResultsCommand::ReturnToKeyboard);
        assert_eq!(router.route(Action::GbaDown(Btn::X)), SearchResultsCommand::Close);
        assert_eq!(router.route(Action::QuickMenu), SearchResultsCommand::Close);
    }

    #[test]
    fn empty_search_results_ignore_navigation_and_confirmation() {
        let mut router = SearchResultsRouter::new(0);
        assert_eq!(router.route(Action::GbaDown(Btn::Up)), SearchResultsCommand::Ignore);
        assert_eq!(router.route(Action::GbaDown(Btn::Right)), SearchResultsCommand::Ignore);
        assert_eq!(router.route(Action::GbaDown(Btn::A)), SearchResultsCommand::Ignore);
        assert_eq!(router.route(Action::Insert), SearchResultsCommand::Ignore);
        assert_eq!(router.route(Action::GbaDown(Btn::B)), SearchResultsCommand::ReturnToKeyboard);
    }

    #[test]
    fn restored_search_selection_is_clamped_and_viewport_is_visible() {
        let router = SearchResultsRouter::at(8, 99, 0);
        assert_eq!(router.selected(), 7);
        assert_eq!(router.offset(), 1);
        assert_eq!(router.hit_count(), 8);
    }
    #[test]
    fn all_quick_rows_keep_navigation_inside_their_page() {
        for row in QuickRow::ALL {
            assert!(row.page().contains(&row));
            assert!(row.page().contains(&row.up()));
            assert!(row.page().contains(&row.down()));
        }
    }
}
