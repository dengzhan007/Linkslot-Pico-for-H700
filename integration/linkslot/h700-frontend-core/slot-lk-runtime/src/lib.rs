#![no_std]

extern crate alloc;

use core::{ffi::c_void, mem::size_of};

use slot_input::{Action, H700GestureInput, H700InputError, H700InputServiceV1, Millis};

pub const H700_TIME_ABI_VERSION: u32 = 1;
pub type H700MonotonicMs = unsafe extern "C" fn(context: *mut c_void) -> u64;

/// C-owned monotonic-time service. Its context and table must outlive the loop.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct H700TimeServiceV1 {
    pub abi_version: u32,
    pub struct_size: u32,
    pub context: *mut c_void,
    pub monotonic_ms: Option<H700MonotonicMs>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum H700TimeError {
    VersionMismatch,
    StructTooSmall,
    MissingClock,
}

pub struct H700MonotonicClock<'a> {
    service: &'a H700TimeServiceV1,
}

impl<'a> H700MonotonicClock<'a> {
    pub fn new(service: &'a H700TimeServiceV1) -> Result<Self, H700TimeError> {
        if service.abi_version != H700_TIME_ABI_VERSION {
            return Err(H700TimeError::VersionMismatch);
        }
        if (service.struct_size as usize) < size_of::<H700TimeServiceV1>() {
            return Err(H700TimeError::StructTooSmall);
        }
        if service.monotonic_ms.is_none() {
            return Err(H700TimeError::MissingClock);
        }
        Ok(Self { service })
    }

    #[inline]
    pub fn now_ms(&self) -> Millis {
        // The C service contract guarantees a valid context and monotonic result.
        unsafe {
            self.service.monotonic_ms.expect("validated clock callback")(self.service.context)
        }
    }
}

/// The application owns business state and side effects. This runtime serializes
/// input actions and application ticks on the LK main thread.
pub trait ActionHandler {
    fn handle_action(&mut self, action: Action, now: Millis);
    fn tick(&mut self, now: Millis);
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StepReport {
    pub now: Millis,
    pub actions_dispatched: usize,
}

/// One cooperative LK event loop. It has no scheduler, thread, sleep, file,
/// graphics or emulator dependency. The caller repeatedly invokes step().
pub struct H700EventLoop<'a> {
    clock: H700MonotonicClock<'a>,
    input: H700GestureInput<'a>,
}

impl<'a> H700EventLoop<'a> {
    pub fn new(
        time_service: &'a H700TimeServiceV1,
        input_service: &'a H700InputServiceV1,
    ) -> Result<Self, RuntimeInitError> {
        let clock = H700MonotonicClock::new(time_service).map_err(RuntimeInitError::Time)?;
        let input = H700GestureInput::new(input_service).map_err(RuntimeInitError::Input)?;
        Ok(Self { clock, input })
    }

    /// Drain input, dispatch ordered Linkslot actions, then tick the app once.
    /// Actions are borrowed from the gesture adapter's preallocated buffer and
    /// remain valid only until the next call.
    pub fn step<H: ActionHandler>(&mut self, handler: &mut H) -> StepReport {
        let now = self.clock.now_ms();
        let actions = self.input.poll_actions(now);
        let actions_dispatched = actions.len();
        for action in actions.iter().copied() {
            handler.handle_action(action, now);
        }
        handler.tick(now);
        StepReport { now, actions_dispatched }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeInitError {
    Time(H700TimeError),
    Input(H700InputError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::{ffi::c_void, mem::size_of, ptr};
    use slot_input::{
        H700InputEvent, H700InputServiceV1, H700_INPUT_PRESSED, H700_INPUT_RELEASED,
        H700_BTN_MENU, H700_BTN_POWER, MENU_HOLD_MS,
    };
    use alloc::vec::Vec;

    struct ClockState {
        now: u64,
    }

    unsafe extern "C" fn read_clock(context: *mut c_void) -> u64 {
        unsafe { (*context.cast::<ClockState>()).now }
    }

    struct QueueState {
        events: [H700InputEvent; 4],
        len: usize,
        consumed: usize,
    }

    unsafe extern "C" fn drain(
        context: *mut c_void,
        output: *mut H700InputEvent,
        capacity: usize,
    ) -> usize {
        let queue = unsafe { &mut *context.cast::<QueueState>() };
        let count = queue.len.saturating_sub(queue.consumed).min(capacity);
        unsafe {
            ptr::copy_nonoverlapping(queue.events.as_ptr().add(queue.consumed), output, count);
        }
        queue.consumed += count;
        count
    }

    #[derive(Default)]
    struct TestApp {
        actions: Vec<Action>,
        ticks: Vec<Millis>,
    }

    impl ActionHandler for TestApp {
        fn handle_action(&mut self, action: Action, _now: Millis) {
            self.actions.push(action);
        }

        fn tick(&mut self, now: Millis) {
            self.ticks.push(now);
        }
    }

    fn time_service(clock: &mut ClockState) -> H700TimeServiceV1 {
        H700TimeServiceV1 {
            abi_version: H700_TIME_ABI_VERSION,
            struct_size: size_of::<H700TimeServiceV1>() as u32,
            context: (clock as *mut ClockState).cast(),
            monotonic_ms: Some(read_clock),
        }
    }

    fn input_service(queue: &mut QueueState) -> H700InputServiceV1 {
        H700InputServiceV1 {
            abi_version: slot_input::H700_INPUT_ABI_VERSION,
            struct_size: size_of::<H700InputServiceV1>() as u32,
            context: (queue as *mut QueueState).cast(),
            drain: Some(drain),
        }
    }

    #[test]
    fn step_dispatches_input_actions_then_ticks_at_one_monotonic_time() {
        let mut clock = ClockState { now: 50 };
        let mut queue = QueueState {
            events: [
                H700InputEvent { button: H700_BTN_MENU, state: H700_INPUT_PRESSED, reserved: 0 },
                H700InputEvent::default(),
                H700InputEvent::default(),
                H700InputEvent::default(),
            ],
            len: 1,
            consumed: 0,
        };
        let time = time_service(&mut clock);
        let input = input_service(&mut queue);
        let mut main_loop = H700EventLoop::new(&time, &input).unwrap();
        let mut app = TestApp::default();

        let first = main_loop.step(&mut app);
        assert_eq!(first.now, 50);
        assert_eq!(first.actions_dispatched, 1);
        assert_eq!(app.actions, alloc::vec![Action::QuickMenu]);
        assert_eq!(app.ticks, alloc::vec![50]);

        clock.now = 50 + MENU_HOLD_MS;
        let second = main_loop.step(&mut app);
        assert_eq!(second.now, 50 + MENU_HOLD_MS);
        assert_eq!(second.actions_dispatched, 1);
        assert_eq!(app.actions, alloc::vec![Action::QuickMenu, Action::Eject]);
        assert_eq!(app.ticks, alloc::vec![50, 50 + MENU_HOLD_MS]);
    }

    #[test]
    fn input_edges_keep_their_order_through_action_dispatch() {
        let mut clock = ClockState { now: 100 };
        let mut queue = QueueState {
            events: [
                H700InputEvent { button: H700_BTN_POWER, state: H700_INPUT_PRESSED, reserved: 0 },
                H700InputEvent { button: H700_BTN_POWER, state: H700_INPUT_RELEASED, reserved: 0 },
                H700InputEvent::default(),
                H700InputEvent::default(),
            ],
            len: 2,
            consumed: 0,
        };
        let time = time_service(&mut clock);
        let input = input_service(&mut queue);
        let mut main_loop = H700EventLoop::new(&time, &input).unwrap();
        let mut app = TestApp::default();

        let report = main_loop.step(&mut app);
        assert_eq!(report.actions_dispatched, 2);
        assert_eq!(app.actions, alloc::vec![Action::PowerPress, Action::PowerTap]);
    }

    #[test]
    fn rejects_invalid_clock_and_input_service_tables() {
        let mut clock = ClockState { now: 0 };
        let mut queue = QueueState {
            events: [H700InputEvent::default(); 4],
            len: 0,
            consumed: 0,
        };
        let time = time_service(&mut clock);
        let input = input_service(&mut queue);

        let bad_version = H700TimeServiceV1 {
            abi_version: H700_TIME_ABI_VERSION + 1,
            ..time
        };
        assert_eq!(
            H700MonotonicClock::new(&bad_version).err(),
            Some(H700TimeError::VersionMismatch)
        );

        let bad_size = H700TimeServiceV1 { struct_size: 1, ..time };
        assert_eq!(
            H700MonotonicClock::new(&bad_size).err(),
            Some(H700TimeError::StructTooSmall)
        );

        let missing_clock = H700TimeServiceV1 { monotonic_ms: None, ..time };
        assert_eq!(
            H700MonotonicClock::new(&missing_clock).err(),
            Some(H700TimeError::MissingClock)
        );

        let missing_input = H700InputServiceV1 {
            abi_version: slot_input::H700_INPUT_ABI_VERSION,
            struct_size: size_of::<H700InputServiceV1>() as u32,
            context: core::ptr::null_mut(),
            drain: None,
        };
        assert!(matches!(
            H700EventLoop::new(&time, &missing_input),
            Err(RuntimeInitError::Input(_))
        ));
        let _ = input;
    }
}
