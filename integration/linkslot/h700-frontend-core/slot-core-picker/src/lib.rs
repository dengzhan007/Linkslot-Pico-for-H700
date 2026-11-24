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
/// It owns only the list of available core identifiers and animation/input
/// state. It has no dependency on slot-store, slot-ui, files, or a renderer.
#[derive(Clone)]
pub struct CorePicker<C> {
    options: Vec<C>,
    /// Index into the options vector, never the core's persisted enum/index value.
    seat: usize,
    opened: Millis,
    started: Option<Millis>,
    hop: Option<Hop>,
    close: Option<Close>,
    refusal: Option<Refusal>,
}

impl<C: Copy + PartialEq> CorePicker<C> {
    pub fn open(seat: C, options: Vec<C>, now: Millis) -> Self {
        let seat = options.iter().position(|core| *core == seat).unwrap_or(0);
        Self {
            options,
            seat,
            opened: now,
            started: None,
            hop: None,
            close: None,
            refusal: None,
        }
    }

    pub fn options(&self) -> &[C] {
        &self.options
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

    pub fn seat(&self) -> C {
        self.options[self.seat]
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
        self.refusal = None;
        Outcome::Nothing
    }

    pub fn chip(&self, now: Millis) -> Chip<C> {
        let shake = self.refusal.map_or(0.0, |r| r.offset(now));
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
        Mgba,
        Gpsp,
        Supafaust,
    }

    #[test]
    fn opens_waits_then_starts_animation() {
        let mut picker = CorePicker::open(Core::Mgba, vec![Core::Mgba, Core::Gpsp], 100);
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
        let mut picker = CorePicker::open(Core::Mgba, vec![Core::Mgba, Core::Gpsp], 0);
        picker.start(0);
        assert_eq!(picker.press(Press::Left, OPEN_MS), Outcome::Refused);
        assert!(picker.chip(OPEN_MS).shake != 0.0);
        assert_eq!(picker.press(Press::Right, OPEN_MS), Outcome::Nothing);
        assert_eq!(picker.seat(), Core::Gpsp);
        assert_eq!(picker.press(Press::Right, OPEN_MS + HOP_MS), Outcome::Refused);
    }

    #[test]
    fn reversing_mid_hop_is_continuous() {
        let mut picker = CorePicker::open(
            Core::Mgba,
            vec![Core::Mgba, Core::Gpsp, Core::Supafaust],
            0,
        );
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
        let mut picker = CorePicker::open(Core::Mgba, vec![Core::Mgba], 0);
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
        let mut picker = CorePicker::open(Core::Mgba, vec![Core::Mgba, Core::Gpsp], 0);
        picker.start(0);
        assert_eq!(picker.press(Press::Right, OPEN_MS), Outcome::Nothing);
        assert_eq!(picker.press(Press::Keep, OPEN_MS + HOP_MS), Outcome::Write(Core::Gpsp));
        assert!(picker.closing());
        assert!(picker.finished(OPEN_MS + HOP_MS + CLOSE_MS));
    }

    #[test]
    fn stale_chip_hop_is_removed_after_transition() {
        let mut picker = CorePicker::open(Core::Mgba, vec![Core::Mgba, Core::Gpsp], 0);
        picker.start(0);
        picker.press(Press::Right, 0);
        let landed = picker.chip(HOP_MS);
        assert_eq!(landed.seated, Some(Core::Gpsp));
        assert_eq!(landed.lift, 0.0);
        assert_eq!(landed.across, 1.0);
    }
}
