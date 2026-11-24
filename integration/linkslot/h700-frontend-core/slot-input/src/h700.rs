use alloc::vec::Vec;
use core::{ffi::c_void, mem::size_of};

use crate::{Btn, InputSource, Millis, RawEvent};

pub const H700_INPUT_ABI_VERSION: u32 = 1;
pub const H700_INPUT_MAX_EVENTS: usize = 32;

pub const H700_BTN_UP: u16 = 1;
pub const H700_BTN_DOWN: u16 = 2;
pub const H700_BTN_LEFT: u16 = 3;
pub const H700_BTN_RIGHT: u16 = 4;
pub const H700_BTN_A: u16 = 5;
pub const H700_BTN_B: u16 = 6;
pub const H700_BTN_X: u16 = 7;
pub const H700_BTN_Y: u16 = 8;
pub const H700_BTN_L1: u16 = 9;
pub const H700_BTN_R1: u16 = 10;
pub const H700_BTN_L2: u16 = 11;
pub const H700_BTN_R2: u16 = 12;
pub const H700_BTN_START: u16 = 13;
pub const H700_BTN_SELECT: u16 = 14;
pub const H700_BTN_MENU: u16 = 15;
pub const H700_BTN_VOL_UP: u16 = 16;
pub const H700_BTN_VOL_DOWN: u16 = 17;
pub const H700_BTN_POWER: u16 = 18;
pub const H700_BTN_LID: u16 = 19;

pub const H700_INPUT_RELEASED: u8 = 0;
pub const H700_INPUT_PRESSED: u8 = 1;

/// One logical button edge produced by the C input HAL.
///
/// The C side must preserve event order and must not write more than the
/// caller-provided capacity. Unknown button codes or state values are ignored.
#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct H700InputEvent {
    pub button: u16,
    pub state: u8,
    pub reserved: u8,
}

pub type H700InputDrain = unsafe extern "C" fn(
    context: *mut c_void,
    output: *mut H700InputEvent,
    capacity: usize,
) -> usize;

/// Versioned C service table. The table and context must outlive the Rust
/// adapter. The C implementation owns its queue and all hardware access.
#[repr(C)]
pub struct H700InputServiceV1 {
    pub abi_version: u32,
    pub struct_size: u32,
    pub context: *mut c_void,
    pub drain: Option<H700InputDrain>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum H700InputError {
    VersionMismatch,
    StructTooSmall,
    MissingDrain,
}

/// Linkslot InputSource backed by a caller-owned H700 C service table.
pub struct H700InputSource<'a> {
    service: &'a H700InputServiceV1,
}

impl<'a> H700InputSource<'a> {
    pub fn new(service: &'a H700InputServiceV1) -> Result<Self, H700InputError> {
        if service.abi_version != H700_INPUT_ABI_VERSION {
            return Err(H700InputError::VersionMismatch);
        }
        if (service.struct_size as usize) < size_of::<H700InputServiceV1>() {
            return Err(H700InputError::StructTooSmall);
        }
        if service.drain.is_none() {
            return Err(H700InputError::MissingDrain);
        }
        Ok(Self { service })
    }
}

impl H700InputSource<'_> {
    /// Drain logical button edges into caller-owned storage without allocating.
    ///
    /// This is the preferred LK runtime path. A poll reads at most 32 events;
    /// any remaining events stay queued in the C service for the next call.
    pub fn drain_into(&mut self, output: &mut [RawEvent]) -> usize {
        let Some(drain) = self.service.drain else {
            return 0;
        };
        let capacity = output.len().min(H700_INPUT_MAX_EVENTS);
        if capacity == 0 {
            return 0;
        }
        let mut raw = [H700InputEvent::default(); H700_INPUT_MAX_EVENTS];
        // The callback contract forbids writes beyond the supplied capacity.
        // Clamp an over-reported count before slicing the local buffer.
        let count = unsafe {
            drain(
                self.service.context,
                raw.as_mut_ptr(),
                capacity,
            )
        }
        .min(capacity);

        let mut written = 0;
        for event in &raw[..count] {
            let Some(button) = decode_button(event.button) else {
                continue;
            };
            let mapped = match event.state {
                H700_INPUT_PRESSED => RawEvent::Down(button),
                H700_INPUT_RELEASED => RawEvent::Up(button),
                _ => continue,
            };
            output[written] = mapped;
            written += 1;
        }
        written
    }
}

impl InputSource for H700InputSource<'_> {
    fn poll(&mut self, _now: Millis) -> Vec<RawEvent> {
        let mut events = [RawEvent::Up(Btn::Up); H700_INPUT_MAX_EVENTS];
        let count = self.drain_into(&mut events);
        events[..count].to_vec()
    }
}

/// Complete C-input to Linkslot-gesture adapter for the LK event loop.
///
/// The caller supplies a reusable action vector; it is cleared and filled on
/// each poll. The raw event batch uses fixed stack storage and does not allocate.
/// The upstream gesture machine remains the single source of truth for chords,
/// long presses, volume repeat and fast-forward timing.
pub struct H700GestureInput<'a> {
    input: H700InputSource<'a>,
    gestures: crate::Gestures,
    raw: [RawEvent; H700_INPUT_MAX_EVENTS],
}

impl<'a> H700GestureInput<'a> {
    pub fn new(service: &'a H700InputServiceV1) -> Result<Self, H700InputError> {
        Ok(Self {
            input: H700InputSource::new(service)?,
            gestures: crate::Gestures::new(),
            raw: [RawEvent::Up(Btn::Up); H700_INPUT_MAX_EVENTS],
        })
    }

    /// Process queued button edges and time-based gestures.
    ///
    /// The time value must come from the H700 monotonic timer service, not
    /// wall-clock time. The returned count is the number of actions written.
    pub fn poll_actions(&mut self, now: Millis, output: &mut Vec<crate::Action>) -> usize {
        output.clear();
        let count = self.input.drain_into(&mut self.raw);
        for event in &self.raw[..count] {
            output.extend(self.gestures.feed(*event, now));
        }
        output.extend(self.gestures.tick(now));
        output.len()
    }

    pub fn ff_latched(&self) -> bool {
        self.gestures.ff_latched()
    }

    pub fn drop_ff_latch(&mut self, output: &mut Vec<crate::Action>) -> usize {
        output.clear();
        output.extend(self.gestures.drop_ff_latch());
        output.len()
    }
}

fn decode_button(code: u16) -> Option<Btn> {
    Some(match code {
        H700_BTN_UP => Btn::Up,
        H700_BTN_DOWN => Btn::Down,
        H700_BTN_LEFT => Btn::Left,
        H700_BTN_RIGHT => Btn::Right,
        H700_BTN_A => Btn::A,
        H700_BTN_B => Btn::B,
        H700_BTN_X => Btn::X,
        H700_BTN_Y => Btn::Y,
        H700_BTN_L1 => Btn::L1,
        H700_BTN_R1 => Btn::R1,
        H700_BTN_L2 => Btn::L2,
        H700_BTN_R2 => Btn::R2,
        H700_BTN_START => Btn::Start,
        H700_BTN_SELECT => Btn::Select,
        H700_BTN_MENU => Btn::Menu,
        H700_BTN_VOL_UP => Btn::VolUp,
        H700_BTN_VOL_DOWN => Btn::VolDown,
        H700_BTN_POWER => Btn::Power,
        H700_BTN_LID => Btn::Lid,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::ptr;

    struct Queue {
        events: [H700InputEvent; 4],
        len: usize,
        consumed: usize,
    }

    unsafe extern "C" fn drain(
        context: *mut c_void,
        output: *mut H700InputEvent,
        capacity: usize,
    ) -> usize {
        let queue = unsafe { &mut *context.cast::<Queue>() };
        let count = queue.len.saturating_sub(queue.consumed).min(capacity);
        unsafe {
            ptr::copy_nonoverlapping(
                queue.events.as_ptr().add(queue.consumed),
                output,
                count,
            );
        }
        queue.consumed += count;
        count
    }

    fn service<'a>(queue: &'a mut Queue) -> H700InputServiceV1 {
        H700InputServiceV1 {
            abi_version: H700_INPUT_ABI_VERSION,
            struct_size: size_of::<H700InputServiceV1>() as u32,
            context: (queue as *mut Queue).cast(),
            drain: Some(drain),
        }
    }

    #[test]
    fn c_button_edges_flow_through_upstream_gesture_actions() {
        let mut queue = Queue {
            events: [
                H700InputEvent { button: H700_BTN_MENU, state: H700_INPUT_PRESSED, reserved: 0 },
                H700InputEvent::default(),
                H700InputEvent::default(),
                H700InputEvent::default(),
            ],
            len: 1,
            consumed: 0,
        };
        let service = service(&mut queue);
        let mut runtime = H700GestureInput::new(&service).unwrap();
        let mut actions = Vec::with_capacity(8);

        assert_eq!(runtime.poll_actions(100, &mut actions), 1);
        assert_eq!(actions, alloc::vec![crate::Action::QuickMenu]);

        assert_eq!(runtime.poll_actions(100 + crate::MENU_HOLD_MS, &mut actions), 1);
        assert_eq!(actions, alloc::vec![crate::Action::Eject]);

        queue.events[1] = H700InputEvent {
            button: H700_BTN_MENU, state: H700_INPUT_RELEASED, reserved: 0
        };
        queue.events[2] = H700InputEvent {
            button: H700_BTN_POWER, state: H700_INPUT_PRESSED, reserved: 0
        };
        queue.len = 3;
        assert_eq!(runtime.poll_actions(1_000, &mut actions), 1);
        assert_eq!(actions, alloc::vec![crate::Action::PowerPress]);

        assert_eq!(runtime.poll_actions(1_000 + crate::POWER_HOLD_MS, &mut actions), 1);
        assert_eq!(actions, alloc::vec![crate::Action::PowerHold]);

        queue.events[3] = H700InputEvent {
            button: H700_BTN_POWER, state: H700_INPUT_RELEASED, reserved: 0
        };
        queue.len = 4;
        assert_eq!(runtime.poll_actions(1_001 + crate::POWER_HOLD_MS, &mut actions), 1);
        assert_eq!(actions, alloc::vec![crate::Action::PowerOff]);
    }

    #[test]
    fn ordered_hardware_edges_become_linkslot_raw_events() {
        let mut queue = Queue {
            events: [
                H700InputEvent { button: H700_BTN_A, state: H700_INPUT_PRESSED, reserved: 0 },
                H700InputEvent { button: H700_BTN_A, state: H700_INPUT_RELEASED, reserved: 0 },
                H700InputEvent { button: H700_BTN_MENU, state: H700_INPUT_PRESSED, reserved: 0 },
                H700InputEvent { button: H700_BTN_POWER, state: H700_INPUT_PRESSED, reserved: 0 },
            ],
            len: 4,
            consumed: 0,
        };
        let service = service(&mut queue);
        let mut source = H700InputSource::new(&service).unwrap();
        assert_eq!(
            source.poll(123),
            alloc::vec![
                RawEvent::Down(Btn::A),
                RawEvent::Up(Btn::A),
                RawEvent::Down(Btn::Menu),
                RawEvent::Down(Btn::Power),
            ]
        );
        assert!(source.poll(124).is_empty());
    }

    #[test]
    fn drain_into_uses_caller_storage_and_leaves_unread_events_queued() {
        let mut queue = Queue {
            events: [
                H700InputEvent { button: H700_BTN_A, state: H700_INPUT_PRESSED, reserved: 0 },
                H700InputEvent { button: H700_BTN_B, state: H700_INPUT_PRESSED, reserved: 0 },
                H700InputEvent { button: H700_BTN_X, state: H700_INPUT_RELEASED, reserved: 0 },
                H700InputEvent { button: H700_BTN_Y, state: H700_INPUT_PRESSED, reserved: 0 },
            ],
            len: 4,
            consumed: 0,
        };
        let service = service(&mut queue);
        let mut source = H700InputSource::new(&service).unwrap();
        let mut small = [RawEvent::Up(Btn::Up); 2];
        assert_eq!(source.drain_into(&mut small), 2);
        assert_eq!(small, [RawEvent::Down(Btn::A), RawEvent::Down(Btn::B)]);
        let mut rest = [RawEvent::Up(Btn::Up); 4];
        assert_eq!(source.drain_into(&mut rest), 2);
        assert_eq!(rest[..2], [RawEvent::Up(Btn::X), RawEvent::Down(Btn::Y)]);
        assert_eq!(source.drain_into(&mut []), 0);
    }

    #[test]
    fn unknown_codes_and_invalid_states_are_ignored() {
        let mut queue = Queue {
            events: [
                H700InputEvent { button: 0xffff, state: H700_INPUT_PRESSED, reserved: 0 },
                H700InputEvent { button: H700_BTN_B, state: 7, reserved: 0 },
                H700InputEvent { button: H700_BTN_B, state: H700_INPUT_RELEASED, reserved: 0 },
                H700InputEvent { button: H700_BTN_UP, state: H700_INPUT_PRESSED, reserved: 0 },
            ],
            len: 4,
            consumed: 0,
        };
        let service = service(&mut queue);
        let mut source = H700InputSource::new(&service).unwrap();
        assert_eq!(
            source.poll(0),
            alloc::vec![RawEvent::Up(Btn::B), RawEvent::Down(Btn::Up)]
        );
    }

    #[test]
    fn rejects_incompatible_service_tables() {
        let mut queue = Queue {
            events: [H700InputEvent::default(); 4],
            len: 0,
            consumed: 0,
        };
        let mut bad = service(&mut queue);
        bad.abi_version += 1;
        assert_eq!(H700InputSource::new(&bad).err(), Some(H700InputError::VersionMismatch));

        let mut small = service(&mut queue);
        small.struct_size = 1;
        assert_eq!(H700InputSource::new(&small).err(), Some(H700InputError::StructTooSmall));

        let mut absent = service(&mut queue);
        absent.drain = None;
        assert_eq!(H700InputSource::new(&absent).err(), Some(H700InputError::MissingDrain));
    }
}
