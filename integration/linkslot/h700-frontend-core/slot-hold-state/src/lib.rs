#![no_std]

pub type Millis = u64;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HoldPrompt {
    pub progress: f32,
    pub alpha: f32,
}

/// Pure timed-hold state machine. The owner decides whether a hold is allowed
/// and what action a completed hold triggers; this type only tracks timing and
/// the visible progress/fade state.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TimedHold {
    started: Option<Millis>,
    fade: Option<(Millis, f32)>,
    hint_delay: Millis,
    hold_duration: Millis,
    fade_duration: Millis,
}

impl TimedHold {
    pub const fn new(
        hint_delay: Millis,
        hold_duration: Millis,
        fade_duration: Millis,
    ) -> Self {
        Self {
            started: None,
            fade: None,
            hint_delay,
            hold_duration,
            fade_duration,
        }
    }

    pub fn begin(&mut self, now: Millis) {
        self.started = Some(now);
        self.fade = None;
    }

    pub fn cancel(&mut self) {
        self.started = None;
        self.fade = None;
    }

    pub fn is_active(&self) -> bool {
        self.started.is_some()
    }

    /// Release the button. A progress bar that has become visible fades from
    /// its current position; a short tap remains invisible.
    pub fn release(&mut self, now: Millis) {
        let Some(started) = self.started.take() else {
            return;
        };
        let elapsed = now.saturating_sub(started);
        if elapsed >= self.hint_delay && elapsed < self.hold_duration {
            let span = self.hold_duration.saturating_sub(self.hint_delay);
            let progress = if span == 0 {
                1.0
            } else {
                (elapsed - self.hint_delay) as f32 / span as f32
            };
            self.fade = Some((now, progress.clamp(0.0, 1.0)));
        }
    }

    /// Advance time. Returns true exactly once when an allowed hold completes.
    /// An owner that disallows the hold cancels it without firing.
    pub fn tick(&mut self, now: Millis, allowed: bool) -> bool {
        if let Some((started, _)) = self.fade {
            if now.saturating_sub(started) >= self.fade_duration {
                self.fade = None;
            }
        }
        let Some(started) = self.started else {
            return false;
        };
        if !allowed {
            self.cancel();
            return false;
        }
        if now.saturating_sub(started) < self.hold_duration {
            return false;
        }
        self.cancel();
        true
    }

    /// The current hold bar or its release fade, if visible.
    pub fn prompt(&self, now: Millis) -> Option<HoldPrompt> {
        if let Some(started) = self.started {
            let elapsed = now.saturating_sub(started);
            if elapsed < self.hint_delay {
                return None;
            }
            let span = self.hold_duration.saturating_sub(self.hint_delay);
            let progress = if span == 0 {
                1.0
            } else {
                (elapsed.saturating_sub(self.hint_delay) as f32 / span as f32)
                    .clamp(0.0, 1.0)
            };
            return Some(HoldPrompt {
                progress,
                alpha: 1.0,
            });
        }
        let (started, progress) = self.fade?;
        if self.fade_duration == 0 {
            return None;
        }
        let gone = now.saturating_sub(started) as f32 / self.fade_duration as f32;
        (gone < 1.0).then_some(HoldPrompt {
            progress,
            alpha: 1.0 - gone,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOLD: TimedHold = TimedHold::new(120, 1_000, 80);

    #[test]
    fn tap_before_hint_delay_never_shows_a_bar_or_fade() {
        let mut state = HOLD;
        state.begin(100);
        assert_eq!(state.prompt(219), None);
        state.release(219);
        assert_eq!(state.prompt(219), None);
        assert!(!state.is_active());
    }

    #[test]
    fn visible_progress_tracks_elapsed_hold_time() {
        let mut state = HOLD;
        state.begin(100);
        assert_eq!(state.prompt(219), None);
        let prompt = state.prompt(220).unwrap();
        assert_eq!(prompt.progress, 0.0);
        assert_eq!(prompt.alpha, 1.0);
        assert!((state.prompt(660).unwrap().progress - 0.5).abs() < 0.0001);
    }

    #[test]
    fn completion_fires_once_at_the_hold_threshold() {
        let mut state = HOLD;
        state.begin(100);
        assert!(!state.tick(1_099, true));
        assert!(state.tick(1_100, true));
        assert!(!state.tick(1_101, true));
        assert!(!state.is_active());
        assert_eq!(state.prompt(1_101), None);
    }

    #[test]
    fn release_fades_the_current_progress_then_disappears() {
        let mut state = HOLD;
        state.begin(100);
        state.release(560);
        let prompt = state.prompt(560).unwrap();
        assert!((prompt.progress - 0.4).abs() < 0.0001);
        assert_eq!(prompt.alpha, 1.0);
        assert!((state.prompt(600).unwrap().alpha - 0.5).abs() < 0.0001);
        assert_eq!(state.prompt(640), None);
        assert!(!state.tick(640, true));
    }

    #[test]
    fn disallowed_hold_is_cancelled_without_triggering() {
        let mut state = HOLD;
        state.begin(100);
        assert_eq!(state.prompt(400).unwrap().alpha, 1.0);
        assert!(!state.tick(400, false));
        assert!(!state.is_active());
        assert_eq!(state.prompt(400), None);
    }

    #[test]
    fn begin_replaces_a_pending_fade_and_cancel_clears_everything() {
        let mut state = HOLD;
        state.begin(100);
        state.release(560);
        assert!(state.prompt(560).is_some());
        state.begin(700);
        assert!(state.prompt(700).is_none());
        state.cancel();
        assert_eq!(state.prompt(700), None);
        assert!(!state.is_active());
    }

    #[test]
    fn backwards_time_is_saturating_not_underflowing() {
        let mut state = HOLD;
        state.begin(1_000);
        assert_eq!(state.prompt(900), None);
        assert!(!state.tick(900, true));
        state.release(900);
        assert_eq!(state.prompt(900), None);
    }
}
