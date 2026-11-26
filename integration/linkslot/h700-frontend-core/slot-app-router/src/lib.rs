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
    fn all_quick_rows_keep_navigation_inside_their_page() {
        for row in QuickRow::ALL {
            assert!(row.page().contains(&row));
            assert!(row.page().contains(&row.up()));
            assert!(row.page().contains(&row.down()));
        }
    }
}
