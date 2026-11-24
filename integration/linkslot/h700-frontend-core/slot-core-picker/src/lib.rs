#![no_std]

extern crate alloc;

use alloc::vec::Vec;

pub type Millis = u64;

pub const SLIDE_MS: Millis = 160;
pub const LIFT_MS: Millis = 260;
pub const OPEN_MS: Millis = SLIDE_MS + LIFT_MS;
pub const CLOSE_MS: Millis = 320;
pub const HOP_MS: Millis = 180;

const CHIP_TIP: f32 = 4.0 * core::f32::consts::PI / 180.0;
const REFUSAL_MS: Millis = 300;
const SHAKE_PX: f32 = 6.0;
const SHAKE_HZ: f32 = 14.0;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Press {
    Left,
    Right,
    Keep,
    Back,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Outcome<C> {
    Nothing,
    Refused,
    Write(C),
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct ThreeLayout<C> {
    pub slot1: C,
    pub slot2: C,
    pub tray: C,
}

impl<C> ThreeLayout<C> {
    pub const fn new(slot1: C, slot2: C, tray: C) -> Self {
        Self { slot1, slot2, tray }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Chip<C> {
    pub across: f32,
    pub lift: f32,
    pub tip: f32,
    pub seated: Option<C>,
    pub shake: f32,
}

#[derive(Copy, Clone)]
struct Hop {
    from: usize,
    to: usize,
    started: Millis,
}

#[derive(Copy, Clone)]
struct Swap {
    started: Millis,
}

#[derive(Copy, Clone)]
struct Close {
    started: Millis,
    from: f32,
}

#[derive(Copy, Clone)]
struct Refusal {
    started: Millis,
}

impl Refusal {
    fn started(now: Millis) -> Self {
        Self { started: now }
    }

    fn offset(self, now: Millis) -> f32 {
        let age = now.saturating_sub(self.started);
        if age >= REFUSAL_MS {
            return 0.0;
        }
        let decay = 1.0 - age as f32 / REFUSAL_MS as f32;
        let secs = age as f32 / 1000.0;
        libm::cosf(secs * SHAKE_HZ * core::f32::consts::TAU) * SHAKE_PX * decay
    }
}

#[inline]
fn ease(u: f32) -> f32 {
    u * u * u * (u * (u * 6.0 - 15.0) + 10.0)
}

/// Generic Linkslot core-picker state machine.
///
/// The two-core path retains the original single-chip hop interaction. The
/// optional three-core path models two board sockets and one tray without
/// depending on Linkslot's Core enum, slot-store, UI resources, or rendering.
#[derive(Clone)]
pub struct CorePicker<C> {
    options: Vec<C>,
    /// Index into options; never a persisted enum/index value.
    seat: usize,
    /// Core currently marked as running.
    active: usize,
    /// Three physical positions: fixed slot 1, hot slot 2, and spare tray.
    three: Option<ThreeLayout<C>>,
    /// Highlighted position in the three-core layout: 0, 1, or 2.
    selected_pos: usize,
    opened: Millis,
    started: Option<Millis>,
    hop: Option<Hop>,
    swap: Option<Swap>,
    close: Option<Close>,
    refusal: Option<Refusal>,
}

impl<C: Copy + PartialEq> CorePicker<C> {
    pub fn open(seat: C, options: Vec<C>, now: Millis) -> Self {
        let seat = options.iter().position(|core| *core == seat).unwrap_or(0);
        Self {
            options,
            seat,
            active: seat,
            three: None,
            selected_pos: 0,
            opened: now,
            started: None,
            hop: None,
            swap: None,
            close: None,
            refusal: None,
        }
    }

    /// Opens a three-position picker using a caller-supplied, domain-neutral
    /// layout. The layout is enabled only when its three distinct values
    /// exactly cover the available options; otherwise the legacy picker is
    /// retained as a safe fallback.
    ///
    /// If the currently active core is initially assigned to the tray, it is
    /// normalized into slot 2 so the running core always remains on the board.
    pub fn open_three(
        seat: C,
        options: Vec<C>,
        layout: ThreeLayout<C>,
        now: Millis,
    ) -> Self {
        let mut picker = Self::open(seat, options, now);
        picker.configure_three(layout);
        picker
    }

    fn configure_three(&mut self, layout: ThreeLayout<C>) {
        if self.options.len() != 3 {
            return;
        }

        let mut values = [layout.slot1, layout.slot2, layout.tray];
        if values[0] == values[1] || values[0] == values[2] || values[1] == values[2] {
            return;
        }
        if values.iter().any(|core| !self.options.contains(core))
            || self.options.iter().any(|core| !values.contains(core))
        {
            return;
        }

        let running = self.options[self.active];
        if running == values[2] {
            values.swap(1, 2);
        }
        let selected_pos = if running == values[0] {
            0
        } else if running == values[1] {
            1
        } else {
            return;
        };

        self.three = Some(ThreeLayout::new(values[0], values[1], values[2]));
        self.selected_pos = selected_pos;
        self.sync_seat_to_selection();
    }

    pub fn options(&self) -> &[C] {
        &self.options
    }

    /// Currently highlighted core. On the three-core path this is the core at
    /// selected_pos; on the legacy path it is the selected hop slot.
    pub fn seat(&self) -> C {
        match self.three {
            Some(layout) => match self.selected_pos {
                0 => layout.slot1,
                1 => layout.slot2,
                _ => layout.tray,
            },
            None => self.options[self.seat],
        }
    }

    pub fn active(&self) -> C {
        self.options[self.active]
    }

    pub fn is_three(&self) -> bool {
        self.three.is_some()
    }

    pub fn layout(&self) -> Option<ThreeLayout<C>> {
        self.three
    }

    pub fn selected_pos(&self) -> usize {
        self.selected_pos
    }

    /// Progress of a tray/slot-2 exchange, in the range [0, 1).
    pub fn swap_progress(&self, now: Millis) -> Option<f32> {
        let swap = self.swap?;
        let q = now.saturating_sub(swap.started) as f32 / HOP_MS as f32;
        (q < 1.0).then_some(q)
    }

    pub fn start(&mut self, now: Millis) {
        self.started.get_or_insert(now);
    }

    pub fn waiting(&self) -> bool {
        self.started.is_none()
    }

    pub fn waited(&self, now: Millis) -> Millis {
        now.saturating_sub(self.opened)
    }

    pub fn is_single(&self) -> bool {
        self.options.len() <= 1
    }

    pub fn closing(&self) -> bool {
        self.close.is_some()
    }

    pub fn openness(&self, now: Millis) -> f32 {
        match (self.close, self.started) {
            (Some(Close { started, from }), _) => {
                (from - now.saturating_sub(started) as f32 / CLOSE_MS as f32).max(0.0)
            }
            (None, Some(started)) => {
                (now.saturating_sub(started) as f32 / OPEN_MS as f32).min(1.0)
            }
            (None, None) => 0.0,
        }
    }

    pub fn finished(&self, now: Millis) -> bool {
        self.close.is_some() && self.openness(now) <= 0.0
    }

    pub fn press(&mut self, press: Press, now: Millis) -> Outcome<C> {
        if self.close.is_some() {
            return Outcome::Nothing;
        }
        if self.is_single() {
            return match press {
                Press::Keep | Press::Back => {
                    self.begin_close(now);
                    Outcome::Nothing
                }
                Press::Left | Press::Right => Outcome::Nothing,
            };
        }
        if self.three.is_some() {
            return self.press_three(press, now);
        }

        let target = match press {
            Press::Left => self.seat.checked_sub(1),
            Press::Right => {
                if self.seat + 1 < self.options.len() {
                    Some(self.seat + 1)
                } else {
                    None
                }
            }
            Press::Keep => {
                self.active = self.seat;
                self.begin_close(now);
                return Outcome::Write(self.seat());
            }
            Press::Back => {
                self.begin_close(now);
                return Outcome::Nothing;
            }
        };

        if self.waiting() {
            return Outcome::Nothing;
        }
        if let Some(progress) = self.hop_progress(now) {
            let Some(target) = target else {
                return Outcome::Nothing;
            };
            if target == self.seat {
                return Outcome::Nothing;
            }
            let done = ((1.0 - progress) * HOP_MS as f32) as Millis;
            self.hop = Some(Hop {
                from: self.seat,
                to: target,
                started: now.saturating_sub(done),
            });
            self.seat = target;
            self.active = target;
            self.refusal = None;
            return Outcome::Nothing;
        }

        let Some(target) = target else {
            self.refusal = Some(Refusal::started(now));
            return Outcome::Refused;
        };
        self.hop = Some(Hop {
            from: self.seat,
            to: target,
            started: now,
        });
        self.seat = target;
        self.active = target;
        self.refusal = None;
        Outcome::Nothing
    }

    fn press_three(&mut self, press: Press, now: Millis) -> Outcome<C> {
        if self.swap.is_some() || self.waiting() {
            return Outcome::Nothing;
        }

        match press {
            Press::Left => {
                if self.selected_pos == 0 {
                    self.refusal = Some(Refusal::started(now));
                    Outcome::Refused
                } else {
                    self.selected_pos -= 1;
                    self.sync_seat_to_selection();
                    self.refusal = None;
                    Outcome::Nothing
                }
            }
            Press::Right => {
                if self.selected_pos >= 2 {
                    self.refusal = Some(Refusal::started(now));
                    Outcome::Refused
                } else {
                    self.selected_pos += 1;
                    self.sync_seat_to_selection();
                    self.refusal = None;
                    Outcome::Nothing
                }
            }
            Press::Keep => {
                let selected = self.seat();
                if self.selected_pos == 2 {
                    let Some(layout) = self.three else {
                        return Outcome::Nothing;
                    };
                    let swapped = ThreeLayout::new(layout.slot1, layout.tray, layout.slot2);
                    self.three = Some(swapped);
                    self.selected_pos = 1;
                    self.sync_seat_to_selection();
                    self.active = self.seat;
                    self.swap = Some(Swap { started: now });
                    // Preserve the Linkslot tray exchange beat before closing.
                    self.close = Some(Close {
                        started: now.saturating_add(HOP_MS),
                        from: 1.0,
                    });
                    Outcome::Write(swapped.slot2)
                } else {
                    self.active = self.seat;
                    self.begin_close(now);
                    Outcome::Write(selected)
                }
            }
            Press::Back => {
                self.begin_close(now);
                Outcome::Nothing
            }
        }
    }

    fn sync_seat_to_selection(&mut self) {
        if let Some(layout) = self.three {
            let selected = match self.selected_pos {
                0 => layout.slot1,
                1 => layout.slot2,
                _ => layout.tray,
            };
            if let Some(index) = self.options.iter().position(|core| *core == selected) {
                self.seat = index;
            }
        }
    }

    pub fn chip(&self, now: Millis) -> Chip<C> {
        let shake = self.refusal.map_or(0.0, |r| r.offset(now));
        if self.three.is_some() {
            return Chip {
                across: self.selected_pos as f32 / 2.0,
                lift: 0.0,
                tip: 0.0,
                seated: Some(self.seat()),
                shake,
            };
        }
        if let Some((from, to, q)) = self.hop_state(now) {
            let arc = libm::sinf(core::f32::consts::PI * q);
            let lean = if to > from { 1.0 } else { -1.0 };
            return Chip {
                across: self.across_between(from, to, q),
                lift: arc,
                tip: lean * CHIP_TIP * arc,
                seated: None,
                shake,
            };
        }
        Chip {
            across: if self.is_single() {
                0.5
            } else {
                self.seat as f32 / (self.options.len() - 1) as f32
            },
            lift: 0.0,
            tip: 0.0,
            seated: Some(self.seat()),
            shake,
        }
    }

    fn across_between(&self, from: usize, to: usize, q: f32) -> f32 {
        let denom = (self.options.len() - 1).max(1) as f32;
        let f0 = from as f32 / denom;
        let f1 = to as f32 / denom;
        f0 + (f1 - f0) * ease(q)
    }

    fn begin_close(&mut self, now: Millis) {
        self.close = Some(Close {
            started: now,
            from: self.openness(now),
        });
    }

    fn hop_state(&self, now: Millis) -> Option<(usize, usize, f32)> {
        let hop = self.hop?;
        let q = now.saturating_sub(hop.started) as f32 / HOP_MS as f32;
        (q < 1.0).then_some((hop.from, hop.to, q))
    }

    fn hop_progress(&self, now: Millis) -> Option<f32> {
        self.hop_state(now).map(|(_, _, q)| q)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    enum Core {
        A,
        B,
        C,
    }

    fn three_options() -> Vec<Core> {
        vec![Core::A, Core::B, Core::C]
    }

    fn standard_layout() -> ThreeLayout<Core> {
        ThreeLayout::new(Core::A, Core::B, Core::C)
    }

    #[test]
    fn opens_waits_then_starts_animation() {
        let mut picker = CorePicker::open(Core::A, vec![Core::A, Core::B], 100);
        assert!(picker.waiting());
        assert_eq!(picker.waited(120), 20);
        assert_eq!(picker.openness(120), 0.0);
        picker.start(120);
        assert!(!picker.waiting());
        assert_eq!(picker.openness(120), 0.0);
        assert_eq!(picker.openness(120 + OPEN_MS), 1.0);
    }

    #[test]
    fn left_right_selection_and_edge_refusal_are_stable() {
        let mut picker = CorePicker::open(Core::A, vec![Core::A, Core::B], 0);
        picker.start(0);
        assert_eq!(picker.press(Press::Left, OPEN_MS), Outcome::Refused);
        assert!(picker.chip(OPEN_MS).shake != 0.0);
        assert_eq!(picker.press(Press::Right, OPEN_MS), Outcome::Nothing);
        assert_eq!(picker.seat(), Core::B);
        assert_eq!(picker.active(), Core::B);
        assert_eq!(picker.press(Press::Right, OPEN_MS + HOP_MS), Outcome::Refused);
    }

    #[test]
    fn reversing_mid_hop_is_continuous() {
        let mut picker = CorePicker::open(Core::A, three_options(), 0);
        picker.start(0);
        let before = picker.chip(0);
        assert_eq!(before.across, 0.0);
        picker.press(Press::Right, 0);
        let mid = picker.chip(HOP_MS / 2);
        assert!(mid.lift > 0.9);
        let across = mid.across;
        picker.press(Press::Left, HOP_MS / 2);
        let after = picker.chip(HOP_MS / 2);
        assert!((after.across - across).abs() < 0.0001);
    }

    #[test]
    fn single_core_is_information_and_never_rewrites_selection() {
        let mut picker = CorePicker::open(Core::A, vec![Core::A], 0);
        picker.start(0);
        assert!(picker.is_single());
        assert_eq!(picker.press(Press::Right, 10), Outcome::Nothing);
        assert_eq!(picker.press(Press::Keep, 20), Outcome::Nothing);
        assert!(picker.closing());
        assert_eq!(picker.press(Press::Back, 30), Outcome::Nothing);
        assert!(picker.finished(20 + CLOSE_MS));
    }

    #[test]
    fn keep_writes_selected_core_and_closes() {
        let mut picker = CorePicker::open(Core::A, vec![Core::A, Core::B], 0);
        picker.start(0);
        assert_eq!(picker.press(Press::Right, OPEN_MS), Outcome::Nothing);
        assert_eq!(picker.press(Press::Keep, OPEN_MS + HOP_MS), Outcome::Write(Core::B));
        assert!(picker.closing());
        assert!(picker.finished(OPEN_MS + HOP_MS + CLOSE_MS));
    }

    #[test]
    fn stale_chip_hop_is_removed_after_transition() {
        let mut picker = CorePicker::open(Core::A, vec![Core::A, Core::B], 0);
        picker.start(0);
        picker.press(Press::Right, 0);
        let landed = picker.chip(HOP_MS);
        assert_eq!(landed.seated, Some(Core::B));
        assert_eq!(landed.lift, 0.0);
        assert_eq!(landed.across, 1.0);
    }

    #[test]
    fn three_core_layout_keeps_active_core_separate_from_highlight() {
        let mut picker =
            CorePicker::open_three(Core::B, three_options(), standard_layout(), 0);
        assert!(picker.is_three());
        assert_eq!(picker.layout(), Some(standard_layout()));
        assert_eq!(picker.active(), Core::B);
        assert_eq!(picker.seat(), Core::B);
        assert_eq!(picker.selected_pos(), 1);

        picker.start(0);
        assert_eq!(picker.press(Press::Right, OPEN_MS), Outcome::Nothing);
        assert_eq!(picker.seat(), Core::C);
        assert_eq!(picker.selected_pos(), 2);
        assert_eq!(picker.active(), Core::B);
    }

    #[test]
    fn selecting_slot_two_writes_without_swapping_the_tray() {
        let mut picker =
            CorePicker::open_three(Core::A, three_options(), standard_layout(), 0);
        picker.start(0);
        assert_eq!(picker.press(Press::Right, OPEN_MS), Outcome::Nothing);
        assert_eq!(picker.press(Press::Keep, OPEN_MS), Outcome::Write(Core::B));
        assert_eq!(picker.active(), Core::B);
        assert_eq!(picker.layout(), Some(standard_layout()));
        assert!(picker.closing());
    }

    #[test]
    fn selecting_tray_swaps_with_slot_two_and_writes_new_active_core() {
        let mut picker =
            CorePicker::open_three(Core::A, three_options(), standard_layout(), 0);
        picker.start(0);
        assert_eq!(picker.press(Press::Right, OPEN_MS), Outcome::Nothing);
        assert_eq!(picker.press(Press::Right, OPEN_MS), Outcome::Nothing);
        assert_eq!(picker.seat(), Core::C);
        assert_eq!(picker.press(Press::Keep, OPEN_MS), Outcome::Write(Core::C));

        let swapped = ThreeLayout::new(Core::A, Core::C, Core::B);
        assert_eq!(picker.layout(), Some(swapped));
        assert_eq!(picker.selected_pos(), 1);
        assert_eq!(picker.seat(), Core::C);
        assert_eq!(picker.active(), Core::C);
        assert_eq!(picker.swap_progress(OPEN_MS), Some(0.0));
        assert_eq!(picker.swap_progress(OPEN_MS + HOP_MS), None);
        assert_eq!(picker.press(Press::Left, OPEN_MS + 1), Outcome::Nothing);
        assert!(!picker.finished(OPEN_MS + HOP_MS));
        assert!(picker.finished(OPEN_MS + HOP_MS + CLOSE_MS));
    }

    #[test]
    fn active_core_initially_in_tray_is_normalized_to_slot_two() {
        let layout = ThreeLayout::new(Core::A, Core::B, Core::C);
        let picker = CorePicker::open_three(Core::C, three_options(), layout, 0);
        assert_eq!(picker.layout(), Some(ThreeLayout::new(Core::A, Core::C, Core::B)));
        assert_eq!(picker.active(), Core::C);
        assert_eq!(picker.selected_pos(), 1);
    }

    #[test]
    fn invalid_three_layout_falls_back_to_legacy_picker() {
        let invalid = ThreeLayout::new(Core::A, Core::B, Core::B);
        let mut picker = CorePicker::open_three(Core::A, three_options(), invalid, 0);
        assert!(!picker.is_three());
        picker.start(0);
        assert_eq!(picker.press(Press::Right, OPEN_MS), Outcome::Nothing);
        assert_eq!(picker.seat(), Core::B);
        assert_eq!(picker.active(), Core::B);
    }

    #[test]
    fn three_layout_requires_exactly_the_available_three_cores() {
        let mut picker = CorePicker::open_three(
            Core::A,
            vec![Core::A, Core::B, Core::C, Core::A],
            standard_layout(),
            0,
        );
        assert!(!picker.is_three());
        picker.start(0);
        assert_eq!(picker.press(Press::Right, OPEN_MS), Outcome::Nothing);
        assert_eq!(picker.seat(), Core::B);
    }
}
