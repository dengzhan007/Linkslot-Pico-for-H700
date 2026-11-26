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


pub const H700_VFS_SERVICE_ABI_VERSION: u32 = 1;
pub type H700VfsInstallFn = unsafe extern "C" fn(context: *mut c_void) -> bool;

/// C-owned VFS backend installer. The callback must be idempotent: return true
/// when the intended process-lifetime backend is already installed, and false
/// only when no usable backend is available.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct H700VfsServiceV1 {
    pub abi_version: u32,
    pub struct_size: u32,
    pub context: *mut c_void,
    pub install: Option<H700VfsInstallFn>,
}

pub const H700_RUNTIME_SERVICES_ABI_VERSION: u32 = 1;

/// Aggregate C-owned service descriptor for the first LK runtime slice.
///
/// The descriptor and time/input callback contexts must outlive the session.
/// The caller must initialize and qualify the LK heap before bootstrap because
/// H700GestureInput allocates its bounded action buffer during construction.
#[repr(C)]
pub struct H700RuntimeServicesV1 {
    pub abi_version: u32,
    pub struct_size: u32,
    pub time: H700TimeServiceV1,
    pub input: H700InputServiceV1,
    pub vfs: H700VfsServiceV1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum H700BootstrapError<E> {
    ServicesVersionMismatch,
    ServicesStructTooSmall,
    VfsVersionMismatch,
    VfsStructTooSmall,
    MissingVfsInstaller,
    Runtime(RuntimeInitError),
    VfsInstallFailed,
    CoreOpen(E),
}

/// A successfully bootstrapped runtime owns a core RAII handle and the
/// cooperative event loop. Dropping the core handle must deinitialize the
/// Libretro core; the process-lifetime VFS backend intentionally stays installed.
pub struct H700RuntimeSession<'a, C> {
    event_loop: H700EventLoop<'a>,
    core: C,
}

impl<'a, C> H700RuntimeSession<'a, C> {
    /// Validate every descriptor first, install VFS second, then open the
    /// static core. The core opener (which calls retro_init) cannot run before
    /// VFS installation succeeds.
    ///
    /// If core opening fails, the process-lifetime VFS backend remains installed.
    /// The installer must therefore be idempotent for a later bootstrap retry.
    /// The opener is responsible for cleaning up any partially initialized core.
    pub fn bootstrap<E, F>(
        services: &'a H700RuntimeServicesV1,
        open_static_core: F,
    ) -> Result<Self, H700BootstrapError<E>>
    where
        F: FnOnce() -> Result<C, E>,
    {
        if services.abi_version != H700_RUNTIME_SERVICES_ABI_VERSION {
            return Err(H700BootstrapError::ServicesVersionMismatch);
        }
        if (services.struct_size as usize) < size_of::<H700RuntimeServicesV1>() {
            return Err(H700BootstrapError::ServicesStructTooSmall);
        }
        if services.vfs.abi_version != H700_VFS_SERVICE_ABI_VERSION {
            return Err(H700BootstrapError::VfsVersionMismatch);
        }
        if (services.vfs.struct_size as usize) < size_of::<H700VfsServiceV1>() {
            return Err(H700BootstrapError::VfsStructTooSmall);
        }
        let install = services.vfs.install.ok_or(H700BootstrapError::MissingVfsInstaller)?;

        // This only validates the time/input service tables and allocates the
        // fixed gesture action buffer; it does not drain input or start a core.
        let event_loop = H700EventLoop::new(&services.time, &services.input)
            .map_err(H700BootstrapError::Runtime)?;

        if !unsafe { install(services.vfs.context) } {
            return Err(H700BootstrapError::VfsInstallFailed);
        }

        let core = open_static_core().map_err(H700BootstrapError::CoreOpen)?;
        Ok(Self { event_loop, core })
    }

    pub fn step<H: ActionHandler>(&mut self, handler: &mut H) -> StepReport {
        self.event_loop.step(handler)
    }

    pub fn core(&self) -> &C {
        &self.core
    }

    pub fn core_mut(&mut self) -> &mut C {
        &mut self.core
    }

    /// Consume the session and return its core handle. The event loop is dropped.
    pub fn into_core(self) -> C {
        self.core
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::{ffi::c_void, mem::size_of, ptr};
    use slot_input::{
        H700InputEvent, H700InputServiceV1, H700_INPUT_PRESSED, H700_INPUT_RELEASED,
        H700_BTN_MENU, H700_BTN_POWER, MENU_HOLD_MS,
    };
    use alloc::{rc::Rc, vec::Vec};
    use core::cell::RefCell;

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

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum StartupEvent {
        VfsInstall,
        CoreOpen,
        CoreDrop,
    }

    struct InstallState {
        events: Rc<RefCell<Vec<StartupEvent>>>,
        succeeds: bool,
    }

    unsafe extern "C" fn install_vfs(context: *mut c_void) -> bool {
        let state = unsafe { &mut *context.cast::<InstallState>() };
        state.events.borrow_mut().push(StartupEvent::VfsInstall);
        state.succeeds
    }

    struct TestCore {
        events: Rc<RefCell<Vec<StartupEvent>>>,
    }

    impl Drop for TestCore {
        fn drop(&mut self) {
            self.events.borrow_mut().push(StartupEvent::CoreDrop);
        }
    }

    fn runtime_services(
        clock: &mut ClockState,
        queue: &mut QueueState,
        install_state: &mut InstallState,
    ) -> H700RuntimeServicesV1 {
        H700RuntimeServicesV1 {
            abi_version: H700_RUNTIME_SERVICES_ABI_VERSION,
            struct_size: size_of::<H700RuntimeServicesV1>() as u32,
            time: time_service(clock),
            input: input_service(queue),
            vfs: H700VfsServiceV1 {
                abi_version: H700_VFS_SERVICE_ABI_VERSION,
                struct_size: size_of::<H700VfsServiceV1>() as u32,
                context: (install_state as *mut InstallState).cast(),
                install: Some(install_vfs),
            },
        }
    }

    fn empty_queue() -> QueueState {
        QueueState {
            events: [H700InputEvent::default(); 4],
            len: 0,
            consumed: 0,
        }
    }

    #[test]
    fn bootstrap_installs_vfs_before_opening_static_core_and_drop_owns_teardown() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut install_state = InstallState {
            events: events.clone(),
            succeeds: true,
        };
        let mut clock = ClockState { now: 10 };
        let mut queue = empty_queue();
        let services = runtime_services(&mut clock, &mut queue, &mut install_state);

        let session = H700RuntimeSession::bootstrap(&services, || {
            assert_eq!(events.borrow().as_slice(), &[StartupEvent::VfsInstall]);
            events.borrow_mut().push(StartupEvent::CoreOpen);
            Ok::<_, u8>(TestCore { events: events.clone() })
        })
        .unwrap();

        assert_eq!(
            events.borrow().as_slice(),
            &[StartupEvent::VfsInstall, StartupEvent::CoreOpen]
        );
        assert_eq!(session.core().events.borrow().len(), 2);
        drop(session);
        assert_eq!(
            events.borrow().as_slice(),
            &[StartupEvent::VfsInstall, StartupEvent::CoreOpen, StartupEvent::CoreDrop]
        );
    }

    #[test]
    fn bootstrap_never_opens_core_when_vfs_install_fails() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut install_state = InstallState {
            events: events.clone(),
            succeeds: false,
        };
        let mut clock = ClockState { now: 0 };
        let mut queue = empty_queue();
        let services = runtime_services(&mut clock, &mut queue, &mut install_state);
        let mut core_opened = false;

        let result = H700RuntimeSession::<TestCore>::bootstrap(&services, || {
            core_opened = true;
            Err::<TestCore, _>(1u8)
        });

        assert_eq!(result.err(), Some(H700BootstrapError::VfsInstallFailed));
        assert!(!core_opened);
        assert_eq!(events.borrow().as_slice(), &[StartupEvent::VfsInstall]);
    }

    #[test]
    fn bootstrap_validates_time_and_input_before_installing_vfs() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut install_state = InstallState {
            events: events.clone(),
            succeeds: true,
        };
        let mut clock = ClockState { now: 0 };
        let mut queue = empty_queue();
        let mut services = runtime_services(&mut clock, &mut queue, &mut install_state);
        services.time.monotonic_ms = None;

        let result = H700RuntimeSession::<()>::bootstrap(&services, || Ok::<(), u8>(()));
        assert_eq!(
            result.err(),
            Some(H700BootstrapError::Runtime(RuntimeInitError::Time(
                H700TimeError::MissingClock
            )))
        );
        services.time.monotonic_ms = Some(read_clock);
        services.input.drain = None;
        let result = H700RuntimeSession::<()>::bootstrap(&services, || Ok::<(), u8>(()));
        assert_eq!(
            result.err(),
            Some(H700BootstrapError::Runtime(RuntimeInitError::Input(
                H700InputError::MissingDrain
            )))
        );
        assert!(events.borrow().is_empty());
    }

    #[test]
    fn bootstrap_rejects_invalid_service_descriptor_without_side_effects() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut install_state = InstallState {
            events: events.clone(),
            succeeds: true,
        };
        let mut clock = ClockState { now: 0 };
        let mut queue = empty_queue();
        let mut services = runtime_services(&mut clock, &mut queue, &mut install_state);

        services.abi_version += 1;
        assert_eq!(
            H700RuntimeSession::<()>::bootstrap(&services, || Ok::<(), u8>(())).err(),
            Some(H700BootstrapError::ServicesVersionMismatch)
        );
        services.abi_version = H700_RUNTIME_SERVICES_ABI_VERSION;
        services.struct_size = 1;
        assert_eq!(
            H700RuntimeSession::<()>::bootstrap(&services, || Ok::<(), u8>(())).err(),
            Some(H700BootstrapError::ServicesStructTooSmall)
        );
        services.struct_size = size_of::<H700RuntimeServicesV1>() as u32;
        services.vfs.abi_version += 1;
        assert_eq!(
            H700RuntimeSession::<()>::bootstrap(&services, || Ok::<(), u8>(())).err(),
            Some(H700BootstrapError::VfsVersionMismatch)
        );
        services.vfs.abi_version = H700_VFS_SERVICE_ABI_VERSION;
        services.vfs.struct_size = 1;
        assert_eq!(
            H700RuntimeSession::<()>::bootstrap(&services, || Ok::<(), u8>(())).err(),
            Some(H700BootstrapError::VfsStructTooSmall)
        );
        services.vfs.struct_size = size_of::<H700VfsServiceV1>() as u32;
        services.vfs.install = None;
        assert_eq!(
            H700RuntimeSession::<()>::bootstrap(&services, || Ok::<(), u8>(())).err(),
            Some(H700BootstrapError::MissingVfsInstaller)
        );
        assert!(events.borrow().is_empty());
    }

    #[test]
    fn bootstrap_reports_core_open_error_after_vfs_is_installed() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut install_state = InstallState {
            events: events.clone(),
            succeeds: true,
        };
        let mut clock = ClockState { now: 0 };
        let mut queue = empty_queue();
        let services = runtime_services(&mut clock, &mut queue, &mut install_state);

        let result = H700RuntimeSession::<()>::bootstrap(&services, || {
            assert_eq!(events.borrow().as_slice(), &[StartupEvent::VfsInstall]);
            events.borrow_mut().push(StartupEvent::CoreOpen);
            Err::<(), _>(7u8)
        });
        assert_eq!(result.err(), Some(H700BootstrapError::CoreOpen(7)));
        assert_eq!(
            events.borrow().as_slice(),
            &[StartupEvent::VfsInstall, StartupEvent::CoreOpen]
        );
    }

}
