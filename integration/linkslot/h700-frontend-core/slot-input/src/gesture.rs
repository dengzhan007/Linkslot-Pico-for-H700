use alloc::vec::Vec;
use crate::{Btn, Millis, RawEvent};

pub const SELECT_CHORD_MS: Millis = 600;

pub const SELECT_TAP_MS: Millis = 50;
pub const MENU_HOLD_MS: Millis = 1000;
pub const FF_DOUBLE_TAP_MS: Millis = 250;
pub const MUTE_CHORD_MS: Millis = 200;
pub const POWER_HOLD_MS: Millis = 1000;

pub const VOLUME_REPEAT_DELAY_MS: Millis = 400;
pub const VOLUME_REPEAT_MS: Millis = 120;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Action {
    GbaDown(Btn),
    GbaUp(Btn),
    ShelfLeft,
    ShelfRight,
    Insert,
    Eject,
    Polaroids,
    SaveState,
    LoadState,
    RewindStart,
    RewindStop,
    FfStart,
    FfStop,
    BrightnessUp,
    BrightnessDown,
    BlueLightUp,
    BlueLightDown,
    VolumeUp,
    VolumeDown,
    QuickMenu,
    /// The MENU press that opened the menu came back up before it became the
    /// hold. Only the hold-to-leave prompt listens to this: it is how a hold
    /// gets cancelled.
    MenuUp,
    GameMenu,
    MuteToggle,
    ColourCorrectionToggle,
    PaletteNext,
    PalettePrev,
    ShaderNext,
    ShaderPrev,
    PowerPress,
    PowerTap,
    PowerHold,
    PowerOff,
    LidClose,
    LidOpen,
}

#[derive(Copy, Clone, Default)]
enum Select {
    #[default]
    Idle,
    Held {
        since: Millis,
        chorded: bool,
        sent: Option<Millis>,
    },
    ReleaseDue(Millis),
}

#[derive(Default)]
pub struct Gestures {
    select: Select,
    chord_held: u16,
    menu_down_at: Option<Millis>,
    menu_eject_fired: bool,
    power_down_at: Option<Millis>,
    power_hold_fired: bool,
    vol_up_at: Option<Millis>,
    vol_down_at: Option<Millis>,
    vol_up_ramp: Option<Millis>,
    vol_down_ramp: Option<Millis>,
    mute_fired: bool,
    ff_on: bool,
    ff_latched: bool,
    ff_latching_press: bool,
    r2_refused: bool,
    r2_last_release: Option<Millis>,
    rewinding: bool,
}

fn ramp_due(down: Option<Millis>, last: Option<Millis>, now: Millis) -> bool {
    let Some(down) = down else {
        return false;
    };
    if now.saturating_sub(down) < VOLUME_REPEAT_DELAY_MS {
        return false;
    }
    last.is_none_or(|l| now.saturating_sub(l) >= VOLUME_REPEAT_MS)
}

impl Gestures {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ff_latched(&self) -> bool {
        self.ff_latched
    }

    /// Compatibility API for host callers that own the returned vector.
    pub fn drop_ff_latch(&mut self) -> Vec<Action> {
        let mut out = Vec::new();
        self.drop_ff_latch_into(&mut out);
        out
    }

    /// Append the latch-release action into caller-owned reusable storage.
    pub fn drop_ff_latch_into(&mut self, out: &mut Vec<Action>) {
        if self.ff_latched {
            self.ff_clear_into(out);
        }
    }

    /// Compatibility API for host callers that own the returned vector.
    pub fn feed(&mut self, ev: RawEvent, now: Millis) -> Vec<Action> {
        let mut out = Vec::new();
        self.feed_into(ev, now, &mut out);
        out
    }

    /// Process one raw button edge and append actions without allocating a
    /// temporary Vec. LK callers should reserve their action buffer once.
    pub fn feed_into(&mut self, ev: RawEvent, now: Millis, out: &mut Vec<Action>) {
        match ev {
            RawEvent::Down(b) => self.down_into(b, now, out),
            RawEvent::Up(b) => self.up_into(b, now, out),
        }
    }

    /// Compatibility API for host callers that own the returned vector.
    pub fn tick(&mut self, now: Millis) -> Vec<Action> {
        let mut out = Vec::new();
        self.tick_into(now, &mut out);
        out
    }

    /// Advance timers and append any due actions into caller-owned storage.
    pub fn tick_into(&mut self, now: Millis, out: &mut Vec<Action>) {
        if let Select::ReleaseDue(due) = self.select {
            if now >= due {
                self.select = Select::Idle;
                out.push(Action::GbaUp(Btn::Select));
            }
        }
        if let Select::Held {
            since,
            chorded: false,
            sent: None,
        } = self.select
        {
            if now.saturating_sub(since) >= SELECT_CHORD_MS {
                self.hand_over_select_into(now, out);
            }
        }
        if let Some(d) = self.menu_down_at {
            if !self.menu_eject_fired && now.saturating_sub(d) >= MENU_HOLD_MS {
                self.menu_eject_fired = true;
                out.push(Action::Eject);
            }
        }
        if let Some(d) = self.power_down_at {
            if !self.power_hold_fired && now.saturating_sub(d) >= POWER_HOLD_MS {
                self.power_hold_fired = true;
                out.push(Action::PowerHold);
            }
        }
        if !self.mute_fired {
            if ramp_due(self.vol_up_at, self.vol_up_ramp, now) {
                self.vol_up_ramp = Some(now);
                out.push(Action::VolumeUp);
            }
            if ramp_due(self.vol_down_at, self.vol_down_ramp, now) {
                self.vol_down_ramp = Some(now);
                out.push(Action::VolumeDown);
            }
        }
    }

    fn down_into(&mut self, b: Btn, now: Millis, out: &mut Vec<Action>) {
        if let (true, Some((bit, action))) = (self.chording(now), chord(b)) {
            self.mark_chorded();
            self.chord_held |= bit;
            out.push(action);
            return;
        }
        match b {
            Btn::Select => self.select_down_into(now, out),
            Btn::Menu => self.menu_down_into(now, out),
            Btn::Power => {
                self.power_down_at = Some(now);
                self.power_hold_fired = false;
                out.push(Action::PowerPress);
            }
            Btn::Lid => out.push(Action::LidClose),
            Btn::VolUp | Btn::VolDown => self.volume_press_into(b, now, out),
            Btn::L2 => self.rewind_start_into(out),
            Btn::R2 => self.ff_down_into(now, out),
            _ => {
                self.hand_over_select_into(now, out);
                out.push(Action::GbaDown(b));
            }
        }
    }

    fn up_into(&mut self, b: Btn, now: Millis, out: &mut Vec<Action>) {
        if let Some((bit, _)) = chord(b) {
            if self.chord_held & bit != 0 {
                self.chord_held &= !bit;
                return;
            }
        }
        match b {
            Btn::Select => self.select_up_into(now, out),
            Btn::Menu => self.menu_up_into(out),
            Btn::Power => self.power_up_into(out),
            Btn::Lid => out.push(Action::LidOpen),
            Btn::VolUp | Btn::VolDown => self.volume_release(b),
            Btn::L2 => self.rewind_stop_into(out),
            Btn::R2 => self.ff_up_into(now, out),
            _ => out.push(Action::GbaUp(b)),
        }
    }

    fn chording(&self, now: Millis) -> bool {
        match self.select {
            Select::Held { since, chorded, .. } => {
                chorded || now.saturating_sub(since) < SELECT_CHORD_MS
            }
            _ => false,
        }
    }

    fn mark_chorded(&mut self) {
        if let Select::Held { chorded, .. } = &mut self.select {
            *chorded = true;
        }
    }

    fn select_down_into(&mut self, now: Millis, out: &mut Vec<Action>) {
        if matches!(self.select, Select::ReleaseDue(_)) {
            out.push(Action::GbaUp(Btn::Select));
        }
        if !matches!(self.select, Select::Held { .. }) {
            self.select = Select::Held {
                since: now,
                chorded: false,
                sent: None,
            };
        }
    }

    fn hand_over_select_into(&mut self, now: Millis, out: &mut Vec<Action>) {
        match &mut self.select {
            Select::Held {
                chorded: false,
                sent: sent @ None,
                ..
            } => {
                *sent = Some(now);
                out.push(Action::GbaDown(Btn::Select));
            }
            _ => {}
        }
    }

    fn select_up_into(&mut self, now: Millis, out: &mut Vec<Action>) {
        let Select::Held { chorded, sent, .. } = core::mem::take(&mut self.select) else {
            return;
        };
        match (sent, chorded) {
            (Some(at), _) if now.saturating_sub(at) >= SELECT_TAP_MS => {
                out.push(Action::GbaUp(Btn::Select));
            }
            (Some(at), _) => {
                self.select = Select::ReleaseDue(at + SELECT_TAP_MS);
            }
            (None, true) => {}
            (None, false) => {
                self.select = Select::ReleaseDue(now + SELECT_TAP_MS);
                out.push(Action::GbaDown(Btn::Select));
            }
        }
    }

    fn menu_down_into(&mut self, now: Millis, out: &mut Vec<Action>) {
        if self.chording(now) {
            self.mark_chorded();
            self.menu_down_at = None;
            out.push(Action::GameMenu);
            return;
        }
        self.menu_down_at = Some(now);
        self.menu_eject_fired = false;
        out.push(Action::QuickMenu);
    }

    fn menu_up_into(&mut self, out: &mut Vec<Action>) {
        if self.menu_down_at.take().is_none() {
            return;
        }
        if !core::mem::take(&mut self.menu_eject_fired) {
            out.push(Action::MenuUp);
        }
    }

    fn power_up_into(&mut self, out: &mut Vec<Action>) {
        let held = self.power_hold_fired;
        self.power_down_at = None;
        self.power_hold_fired = false;
        out.push(if held { Action::PowerOff } else { Action::PowerTap });
    }

    fn volume_press_into(&mut self, b: Btn, now: Millis, out: &mut Vec<Action>) {
        let (mine, other, action) = match b {
            Btn::VolUp => (&mut self.vol_up_at, self.vol_down_at, Action::VolumeUp),
            _ => (&mut self.vol_down_at, self.vol_up_at, Action::VolumeDown),
        };
        *mine = Some(now);
        match b {
            Btn::VolUp => self.vol_up_ramp = None,
            _ => self.vol_down_ramp = None,
        }
        out.push(action);
        let paired = other.is_some_and(|t| now.abs_diff(t) <= MUTE_CHORD_MS);
        if paired && !self.mute_fired {
            self.mute_fired = true;
            out.push(Action::MuteToggle);
        }
    }

    fn volume_release(&mut self, b: Btn) {
        match b {
            Btn::VolUp => (self.vol_up_at, self.vol_up_ramp) = (None, None),
            _ => (self.vol_down_at, self.vol_down_ramp) = (None, None),
        }
        if self.vol_up_at.is_none() && self.vol_down_at.is_none() {
            self.mute_fired = false;
        }
    }

    fn rewind_start_into(&mut self, out: &mut Vec<Action>) {
        if self.rewinding {
            return;
        }
        self.ff_clear_into(out);
        self.rewinding = true;
        out.push(Action::RewindStart);
    }

    fn rewind_stop_into(&mut self, out: &mut Vec<Action>) {
        if !self.rewinding {
            return;
        }
        self.rewinding = false;
        out.push(Action::RewindStop);
    }

    fn ff_down_into(&mut self, now: Millis, out: &mut Vec<Action>) {
        if self.rewinding {
            self.r2_refused = true;
            return;
        }
        if self.ff_latched {
            self.ff_latching_press = false;
        } else {
            let double = self
                .r2_last_release
                .is_some_and(|rel| now.saturating_sub(rel) <= FF_DOUBLE_TAP_MS);
            self.ff_latched = double;
            self.ff_latching_press = double;
        }
        if self.ff_on {
            return;
        }
        self.ff_on = true;
        out.push(Action::FfStart);
    }

    fn ff_up_into(&mut self, now: Millis, out: &mut Vec<Action>) {
        if core::mem::take(&mut self.r2_refused) {
            return;
        }
        self.r2_last_release = Some(now);
        if self.ff_latching_press {
            self.ff_latching_press = false;
            return;
        }
        self.ff_clear_into(out);
    }

    fn ff_clear_into(&mut self, out: &mut Vec<Action>) {
        self.ff_latched = false;
        self.ff_latching_press = false;
        if !self.ff_on {
            return;
        }
        self.ff_on = false;
        out.push(Action::FfStop);
    }
}

fn chord(b: Btn) -> Option<(u16, Action)> {
    Some(match b {
        Btn::Up => (1, Action::BrightnessUp),
        Btn::Down => (2, Action::BrightnessDown),
        Btn::Left => (4, Action::BlueLightDown),
        Btn::Right => (8, Action::BlueLightUp),
        Btn::L1 => (16, Action::LoadState),
        Btn::R1 => (32, Action::SaveState),
        Btn::Y => (64, Action::ColourCorrectionToggle),
        Btn::L2 => (128, Action::PalettePrev),
        Btn::R2 => (256, Action::PaletteNext),
        Btn::A => (512, Action::ShaderNext),
        Btn::B => (1024, Action::ShaderPrev),
        Btn::VolUp => (2048, Action::MuteToggle),
        Btn::VolDown => (4096, Action::MuteToggle),
        _ => return None,
    })
}


#[cfg(test)]
mod h700_no_std_contract_tests {
    use super::*;

    #[test]
    fn menu_press_and_hold_keep_linkslot_actions() {
        let mut gestures = Gestures::new();
        assert_eq!(gestures.feed(RawEvent::Down(Btn::Menu), 10), vec![Action::QuickMenu]);
        assert_eq!(gestures.tick(10 + MENU_HOLD_MS), vec![Action::Eject]);
        assert!(gestures.feed(RawEvent::Up(Btn::Menu), 10 + MENU_HOLD_MS + 1).is_empty());
    }

    #[test]
    fn power_tap_is_distinct_from_long_press_power_off() {
        let mut tap = Gestures::new();
        assert_eq!(tap.feed(RawEvent::Down(Btn::Power), 100), vec![Action::PowerPress]);
        assert_eq!(tap.feed(RawEvent::Up(Btn::Power), 200), vec![Action::PowerTap]);

        let mut hold = Gestures::new();
        assert_eq!(hold.feed(RawEvent::Down(Btn::Power), 300), vec![Action::PowerPress]);
        assert_eq!(hold.tick(300 + POWER_HOLD_MS), vec![Action::PowerHold]);
        assert_eq!(hold.feed(RawEvent::Up(Btn::Power), 300 + POWER_HOLD_MS + 1), vec![Action::PowerOff]);
    }

    #[test]
    fn select_tap_release_is_completed_by_monotonic_tick() {
        let mut gestures = Gestures::new();
        assert!(gestures.feed(RawEvent::Down(Btn::Select), 0).is_empty());
        assert_eq!(gestures.feed(RawEvent::Up(Btn::Select), 10), vec![Action::GbaDown(Btn::Select)]);
        assert_eq!(gestures.tick(10 + SELECT_TAP_MS), vec![Action::GbaUp(Btn::Select)]);
    }

    #[test]
    fn volume_repeat_uses_supplied_time_without_wall_clock() {
        let mut gestures = Gestures::new();
        assert_eq!(gestures.feed(RawEvent::Down(Btn::VolUp), 0), vec![Action::VolumeUp]);
        assert_eq!(gestures.tick(VOLUME_REPEAT_DELAY_MS - 1), Vec::<Action>::new());
        assert_eq!(gestures.tick(VOLUME_REPEAT_DELAY_MS), vec![Action::VolumeUp]);
    }
}
