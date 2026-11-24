#![no_std]

use slot_input::{Btn, RawEvent};

/// Every physical button representable by Linkslot's controls Device enum.
pub const MAX_TRACKED_BUTTONS: usize = 14;
pub const CONFIGURED_RULES: usize = 14;

/// Physical-to-logical mapping row prepared by the platform adapter.
/// Customised rows win over stock rows. Modifier rows are ignored here;
/// their chord semantics remain in the gesture/input layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ControlRule {
    pub logical: Btn,
    pub physical: Option<Btn>,
    pub has_modifier: bool,
    pub customised: bool,
}

impl ControlRule {
    pub const fn new(
        logical: Btn,
        physical: Option<Btn>,
        has_modifier: bool,
        customised: bool,
    ) -> Self {
        Self { logical, physical, has_modifier, customised }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonLayout {
    #[default]
    Off,
    SwapShoulders,
    XyShoulders,
    Both,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Held {
    physical: Btn,
    logical: Btn,
}

/// Fixed-capacity, allocation-free physical-to-logical input remapper.
#[derive(Clone)]
pub struct Remap {
    held: [Option<Held>; MAX_TRACKED_BUTTONS],
}

impl Default for Remap {
    fn default() -> Self {
        Self { held: [None; MAX_TRACKED_BUTTONS] }
    }
}

impl Remap {
    pub const fn new() -> Self {
        Self { held: [None; MAX_TRACKED_BUTTONS] }
    }

    pub fn held_count(&self) -> usize {
        self.held.iter().filter(|entry| entry.is_some()).count()
    }

    pub fn reset(&mut self) {
        self.held = [None; MAX_TRACKED_BUTTONS];
    }

    /// Apply controls and legacy shoulder layout to a physical event.
    ///
    /// controls_active is true when the complete Controls object differs
    /// from Linkslot defaults. Both customised and stock rows then
    /// participate in lookup, preserving upstream precedence.
    pub fn feed(
        &mut self,
        event: RawEvent,
        layout: ButtonLayout,
        rules: &[ControlRule],
        controls_active: bool,
    ) -> Option<RawEvent> {
        let physical = match event {
            RawEvent::Down(button) | RawEvent::Up(button) => button,
        };

        // Consume releases for already-tracked buttons before consulting the
        // current configuration. This retains the logical key chosen at Down.
        if let RawEvent::Up(_) = event {
            if let Some(index) = self.held.iter().position(|entry| {
                entry.is_some_and(|held| held.physical == physical)
            }) {
                let held = self.held[index].take().expect("located held button");
                let still_held = self.held.iter().flatten()
                    .any(|other| other.logical == held.logical);
                return (!still_held).then_some(RawEvent::Up(held.logical));
            }
        }

        let custom = if controls_active {
            Self::map_control(physical, rules)
        } else {
            None
        };

        // Fast path for keys outside controls.txt and the legacy layout modes.
        if custom.is_none() && !Self::layout_key(physical) {
            return Some(event);
        }

        match event {
            RawEvent::Down(physical) => {
                if self.held.iter().flatten().any(|held| held.physical == physical) {
                    return None;
                }
                let logical = custom.unwrap_or_else(|| Self::layout_button(layout, physical));
                let already_held = self.held.iter().flatten()
                    .any(|held| held.logical == logical);
                let Some(slot) = self.held.iter_mut().find(|entry| entry.is_none()) else {
                    // Fourteen slots cover the complete controls Device enum.
                    // Fail closed rather than emit an unmatched Down if Btn grows.
                    return None;
                };
                *slot = Some(Held { physical, logical });
                (!already_held).then_some(RawEvent::Down(logical))
            }
            RawEvent::Up(_) => None,
        }
    }

    fn map_control(physical: Btn, rules: &[ControlRule]) -> Option<Btn> {
        // Explicit user overrides always win over stock defaults.
        rules.iter()
            .find(|rule| {
                rule.customised && !rule.has_modifier && rule.physical == Some(physical)
            })
            .or_else(|| {
                // Stock rows retain the caller's CONFIGURED order.
                rules.iter().find(|rule| {
                    !rule.has_modifier && rule.physical == Some(physical)
                })
            })
            .map(|rule| rule.logical)
    }

    fn layout_key(button: Btn) -> bool {
        matches!(button, Btn::X | Btn::Y | Btn::L1 | Btn::R1 | Btn::L2 | Btn::R2)
    }

    fn layout_button(layout: ButtonLayout, physical: Btn) -> Btn {
        match (layout, physical) {
            (ButtonLayout::SwapShoulders | ButtonLayout::Both, Btn::L1) => Btn::L2,
            (ButtonLayout::SwapShoulders | ButtonLayout::Both, Btn::R1) => Btn::R2,
            (ButtonLayout::SwapShoulders | ButtonLayout::Both, Btn::L2) => Btn::L1,
            (ButtonLayout::SwapShoulders | ButtonLayout::Both, Btn::R2) => Btn::R1,
            (ButtonLayout::XyShoulders | ButtonLayout::Both, Btn::X) => Btn::L1,
            (ButtonLayout::XyShoulders | ButtonLayout::Both, Btn::Y) => Btn::R1,
            _ => physical,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slot_input::Btn::*;

    const NO_RULES: [ControlRule; 0] = [];

    fn rules(entries: &[(Btn, Btn, bool)]) -> [ControlRule; 3] {
        [
            ControlRule::new(entries[0].1, Some(entries[0].0), false, entries[0].2),
            ControlRule::new(entries[1].1, Some(entries[1].0), false, entries[1].2),
            ControlRule::new(entries[2].1, Some(entries[2].0), false, entries[2].2),
        ]
    }

    #[test]
    fn non_layout_keys_pass_through_without_being_tracked() {
        let mut remap = Remap::new();
        assert_eq!(remap.feed(RawEvent::Down(A), ButtonLayout::Off, &NO_RULES, false), Some(RawEvent::Down(A)));
        assert_eq!(remap.feed(RawEvent::Up(A), ButtonLayout::Off, &NO_RULES, false), Some(RawEvent::Up(A)));
        assert_eq!(remap.held_count(), 0);
    }

    #[test]
    fn legacy_shoulder_layouts_are_preserved() {
        let mut remap = Remap::new();
        assert_eq!(remap.feed(RawEvent::Down(L1), ButtonLayout::SwapShoulders, &NO_RULES, false), Some(RawEvent::Down(L2)));
        assert_eq!(remap.feed(RawEvent::Up(L1), ButtonLayout::SwapShoulders, &NO_RULES, false), Some(RawEvent::Up(L2)));
        assert_eq!(remap.feed(RawEvent::Down(X), ButtonLayout::XyShoulders, &NO_RULES, false), Some(RawEvent::Down(L1)));
        assert_eq!(remap.feed(RawEvent::Up(X), ButtonLayout::XyShoulders, &NO_RULES, false), Some(RawEvent::Up(L1)));
        assert_eq!(remap.feed(RawEvent::Down(Y), ButtonLayout::Both, &NO_RULES, false), Some(RawEvent::Down(R1)));
        assert_eq!(remap.feed(RawEvent::Up(Y), ButtonLayout::Both, &NO_RULES, false), Some(RawEvent::Up(R1)));
    }

    #[test]
    fn a_custom_rule_maps_any_physical_device_button() {
        let configured = rules(&[(B, A, true), (Y, Y, false), (X, X, false)]);
        let mut remap = Remap::new();
        assert_eq!(remap.feed(RawEvent::Down(B), ButtonLayout::Off, &configured, true), Some(RawEvent::Down(A)));
        assert_eq!(remap.feed(RawEvent::Up(B), ButtonLayout::Off, &configured, true), Some(RawEvent::Up(A)));
    }

    #[test]
    fn customised_alias_wins_over_a_stock_binding() {
        let configured = rules(&[(Y, X, true), (Y, Y, false), (X, X, false)]);
        let mut remap = Remap::new();
        assert_eq!(remap.feed(RawEvent::Down(Y), ButtonLayout::Off, &configured, true), Some(RawEvent::Down(X)));
    }

    #[test]
    fn modifier_bindings_are_not_consumed_as_plain_game_buttons() {
        let configured = [
            ControlRule::new(A, Some(B), true, true),
            ControlRule::new(B, Some(B), false, false),
            ControlRule::new(X, Some(X), false, false),
        ];
        let mut remap = Remap::new();
        assert_eq!(remap.feed(RawEvent::Down(B), ButtonLayout::Off, &configured, true), Some(RawEvent::Down(B)));
    }

    #[test]
    fn explicit_controls_override_shoulder_layout() {
        let configured = rules(&[(L1, A, true), (R1, R1, false), (L2, L2, false)]);
        let mut remap = Remap::new();
        assert_eq!(remap.feed(RawEvent::Down(L1), ButtonLayout::SwapShoulders, &configured, true), Some(RawEvent::Down(A)));
        assert_eq!(remap.feed(RawEvent::Up(L1), ButtonLayout::SwapShoulders, &configured, true), Some(RawEvent::Up(A)));
    }

    #[test]
    fn held_button_releases_its_original_mapping_after_config_changes() {
        let first = rules(&[(B, A, true), (Y, Y, false), (X, X, false)]);
        let second = rules(&[(B, B, false), (Y, Y, false), (X, X, false)]);
        let mut remap = Remap::new();
        assert_eq!(remap.feed(RawEvent::Down(B), ButtonLayout::Off, &first, true), Some(RawEvent::Down(A)));
        assert_eq!(remap.feed(RawEvent::Up(B), ButtonLayout::Off, &second, true), Some(RawEvent::Up(A)));
    }

    #[test]
    fn multiple_physical_buttons_share_one_logical_hold() {
        let configured = rules(&[(B, A, true), (X, A, true), (Y, Y, false)]);
        let mut remap = Remap::new();
        assert_eq!(remap.feed(RawEvent::Down(B), ButtonLayout::Off, &configured, true), Some(RawEvent::Down(A)));
        assert_eq!(remap.feed(RawEvent::Down(X), ButtonLayout::Off, &configured, true), None);
        assert_eq!(remap.feed(RawEvent::Up(B), ButtonLayout::Off, &configured, true), None);
        assert_eq!(remap.feed(RawEvent::Up(X), ButtonLayout::Off, &configured, true), Some(RawEvent::Up(A)));
    }

    #[test]
    fn duplicate_physical_down_is_ignored() {
        let mut remap = Remap::new();
        assert_eq!(remap.feed(RawEvent::Down(L1), ButtonLayout::SwapShoulders, &NO_RULES, false), Some(RawEvent::Down(L2)));
        assert_eq!(remap.feed(RawEvent::Down(L1), ButtonLayout::Off, &NO_RULES, false), None);
        assert_eq!(remap.held_count(), 1);
    }

    #[test]
    fn stock_rows_keep_the_callers_configured_order() {
        let configured = rules(&[(B, X, false), (B, A, false), (X, X, false)]);
        let mut remap = Remap::new();
        assert_eq!(remap.feed(RawEvent::Down(B), ButtonLayout::Off, &configured, true), Some(RawEvent::Down(X)));
    }

    #[test]
    fn all_fourteen_controls_buttons_fit_without_allocation() {
        let buttons = [Up, Down, Left, Right, A, B, X, Y, L1, R1, L2, R2, Select, Start];
        let identity: [ControlRule; 14] = core::array::from_fn(|i| {
            ControlRule::new(buttons[i], Some(buttons[i]), false, false)
        });
        let mut remap = Remap::new();
        for button in buttons {
            let _ = remap.feed(RawEvent::Down(button), ButtonLayout::Off, &identity, true);
        }
        assert_eq!(remap.held_count(), MAX_TRACKED_BUTTONS);
        for button in buttons {
            let _ = remap.feed(RawEvent::Up(button), ButtonLayout::Off, &identity, true);
        }
        assert_eq!(remap.held_count(), 0);
    }

    #[test]
    fn reset_clears_all_held_physical_keys() {
        let mut remap = Remap::new();
        let _ = remap.feed(RawEvent::Down(L1), ButtonLayout::SwapShoulders, &NO_RULES, false);
        remap.reset();
        assert_eq!(remap.held_count(), 0);
        assert_eq!(remap.feed(RawEvent::Up(L1), ButtonLayout::Off, &NO_RULES, false), None);
    }
}
