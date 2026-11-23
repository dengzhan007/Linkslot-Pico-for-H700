//! A small, safe-ish wrapper around the rcheevos runtime.
//!
//! rcheevos evaluates RetroAchievements condition strings (`MemAddr`) against
//! a memory reader. This crate deliberately exposes only what linkSlot needs
//! for phase 2: create a runtime, activate conditions by numeric id, feed one
//! frame of memory, and collect the events (notably ACHIEVEMENT_TRIGGERED).
//!
//! The runtime is not sent between threads; linkSlot creates it on the emulator
//! thread and calls [`Runtime::frame`] once per emulated frame.

use std::cell::Cell;
use std::ffi::{c_char, c_int, c_void, CString};
use std::ptr;

const EVENT_ACTIVATED: u8 = 0;
const EVENT_PAUSED: u8 = 1;
const EVENT_RESET: u8 = 2;
const EVENT_TRIGGERED: u8 = 3;
const EVENT_PRIMED: u8 = 4;

#[repr(C)]
struct RuntimeEvent {
    id: u32,
    value: i32,
    type_: u8,
}

type EventHandler = unsafe extern "C" fn(*const RuntimeEvent);
type ReadMemory = unsafe extern "C" fn(u32, *mut u8, u32, *mut c_void) -> u32;

extern "C" {
    fn rc_runtime_alloc() -> *mut c_void;
    fn rc_runtime_destroy(runtime: *mut c_void);
    fn rc_runtime_get_achievement_measured(
        runtime: *mut c_void,
        id: u32,
        measured: *mut u32,
        target: *mut u32,
    ) -> c_int;
    fn rc_runtime_activate_achievement(
        runtime: *mut c_void,
        id: u32,
        memaddr: *const c_char,
        unused_l: *mut c_void,
        unused_funcs_idx: c_int,
    ) -> c_int;
    fn rc_runtime_reset(runtime: *mut c_void);
    fn rc_runtime_do_frame(
        runtime: *mut c_void,
        event_handler: Option<EventHandler>,
        read_memory: Option<ReadMemory>,
        ud: *mut c_void,
        unused_l: *mut c_void,
    );
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Activated,
    Paused,
    Reset,
    Triggered,
    Primed,
    Other(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeEventValue {
    pub id: u32,
    pub value: i32,
    pub event: Event,
}

/// One frame of addressable memory. NES uses 2 KB of work RAM mirrored through
/// 0x1FFF; phase 2 will grow this into a per-console map.
struct Memory<'a> {
    ram: &'a [u8],
}

impl Memory<'_> {
    fn read(&self, address: u32, out: &mut [u8]) -> usize {
        let mut written = 0;
        for (offset, slot) in out.iter_mut().enumerate() {
            let Some(byte) = self.get(address.saturating_add(offset as u32)) else {
                break;
            };
            *slot = byte;
            written += 1;
        }
        written
    }

    fn get(&self, address: u32) -> Option<u8> {
        let address = address as usize;
        let mapped = match address {
            0x0000..=0x07FF => address,
            0x0800..=0x1FFF => address & 0x07FF,
            _ => return None,
        };
        self.ram.get(mapped).copied()
    }
}

struct Reader<F> {
    read: F,
}

unsafe extern "C" fn read_memory<F>(
    address: u32,
    buffer: *mut u8,
    num_bytes: u32,
    ud: *mut c_void,
) -> u32
where
    F: FnMut(u32, &mut [u8]) -> usize,
{
    if ud.is_null() || buffer.is_null() {
        return 0;
    }
    let reader = &mut *(ud as *mut Reader<F>);
    let buffer = std::slice::from_raw_parts_mut(buffer, num_bytes as usize);
    (reader.read)(address, buffer).min(num_bytes as usize) as u32
}

thread_local! {
    /// rcheevos' event callback has no userdata argument, so the collector is
    /// routed through a thread-local for the duration of `do_frame`.
    static EVENT_SINK: Cell<*mut Vec<RuntimeEventValue>> = const { Cell::new(ptr::null_mut()) };
}

unsafe extern "C" fn collect_event(event: *const RuntimeEvent) {
    if event.is_null() {
        return;
    }
    let event = &*event;
    let kind = match event.type_ {
        EVENT_ACTIVATED => Event::Activated,
        EVENT_PAUSED => Event::Paused,
        EVENT_RESET => Event::Reset,
        EVENT_TRIGGERED => Event::Triggered,
        EVENT_PRIMED => Event::Primed,
        other => Event::Other(other),
    };
    EVENT_SINK.with(|sink| {
        let sink = sink.get();
        if !sink.is_null() {
            (*sink).push(RuntimeEventValue {
                id: event.id,
                value: event.value,
                event: kind,
            });
        }
    });
}

/// Owns one `rc_runtime_t`; dropping it destroys the C runtime.
pub struct Runtime {
    raw: *mut c_void,
}

impl Runtime {
    pub fn new() -> Option<Self> {
        let raw = unsafe { rc_runtime_alloc() };
        (!raw.is_null()).then_some(Self { raw })
    }

    /// Activate one condition string under a caller-owned numeric id.
    pub fn activate(&mut self, id: u32, condition: &str) -> Result<(), String> {
        if id == 0 {
            return Err("achievement ids must be non-zero".to_string());
        }
        let condition =
            CString::new(condition).map_err(|_| "condition contains a NUL byte".to_string())?;
        let result = unsafe {
            rc_runtime_activate_achievement(self.raw, id, condition.as_ptr(), ptr::null_mut(), 0)
        };
        if result == 0 {
            Ok(())
        } else {
            Err(format!("rcheevos could not activate condition (code {result})"))
        }
    }

    /// Feed one frame of NES-style RAM (the test/fallback path).
    pub fn frame(&mut self, ram: &[u8]) -> Vec<RuntimeEventValue> {
        let memory = Memory { ram };
        self.frame_with(|address, out| memory.read(address, out))
    }

    /// Feed one frame through a caller-provided reader. This is the path the
    /// device uses once the core exposes a memory map: the closure receives
    /// emulated addresses and fills as many bytes as it can.
    pub fn frame_with<F>(&mut self, read: F) -> Vec<RuntimeEventValue>
    where
        F: FnMut(u32, &mut [u8]) -> usize,
    {
        let mut events = Vec::new();
        let mut reader = Reader { read };
        EVENT_SINK.with(|sink| sink.set(&mut events as *mut _));
        unsafe {
            rc_runtime_do_frame(
                self.raw,
                Some(collect_event),
                Some(read_memory::<F>),
                &mut reader as *mut Reader<F> as *mut c_void,
                ptr::null_mut(),
            );
        }
        EVENT_SINK.with(|sink| sink.set(ptr::null_mut()));
        events
    }

    /// Measured progress for one active achievement, if it has a target.
    pub fn measured(&self, id: u32) -> Option<(u32, u32)> {
        let mut value = 0u32;
        let mut target = 0u32;
        let ok = unsafe {
            rc_runtime_get_achievement_measured(self.raw, id, &mut value, &mut target)
        };
        (ok != 0 && target > 0).then_some((value.min(target), target))
    }

    /// Reset hit counts and trigger state (load/rewind/resume).
    pub fn reset(&mut self) {
        unsafe { rc_runtime_reset(self.raw) };
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe { rc_runtime_destroy(self.raw) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nes_ram() -> Vec<u8> {
        vec![0u8; 2048]
    }

    #[test]
    fn an_address_condition_triggers_when_the_ram_byte_changes() {
        let mut runtime = Runtime::new().expect("rcheevos runtime");
        runtime.activate(1, "0xH00bc=1").expect("condition parses");
        let mut ram = nes_ram();
        assert!(
            runtime
                .frame(&ram)
                .iter()
                .all(|event| event.event != Event::Triggered),
            "must not trigger while the byte is zero"
        );
        ram[0xbc] = 1;
        let events = runtime.frame(&ram);
        assert!(
            events
                .iter()
                .any(|event| event.id == 1 && event.event == Event::Triggered),
            "expected achievement 1 to trigger: {events:?}"
        );
    }

    #[test]
    fn a_false_to_true_transition_triggers() {
        let mut runtime = Runtime::new().expect("rcheevos runtime");
        runtime.activate(2, "0xH07ff=0").expect("condition parses");
        let mut ram = nes_ram();
        ram[0x07ff] = 1;
        let first = runtime.frame(&ram);
        assert!(
            first.iter().all(|event| event.event != Event::Triggered),
            "must not trigger while the condition is false: {first:?}"
        );
        ram[0x07ff] = 0;
        let second = runtime.frame(&ram);
        assert!(
            second
                .iter()
                .any(|event| event.id == 2 && event.event == Event::Triggered),
            "expected the transition to trigger: {second:?}"
        );
    }

    #[test]
    fn measured_progress_is_reported_with_its_target() {
        let mut runtime = Runtime::new().expect("rcheevos runtime");
        runtime
            .activate(9, "M:0xH00bc=100")
            .expect("measured condition parses");
        let mut ram = nes_ram();
        ram[0xbc] = 37;
        runtime.frame(&ram);
        assert_eq!(runtime.measured(9), Some((37, 100)));
    }

    #[test]
    fn an_achievement_without_a_target_has_no_progress() {
        let mut runtime = Runtime::new().expect("rcheevos runtime");
        runtime.activate(10, "0xH00bc=1").expect("condition parses");
        runtime.frame(&nes_ram());
        assert_eq!(runtime.measured(10), None);
    }

    #[test]
    fn a_custom_reader_can_supply_any_address_space() {
        let mut runtime = Runtime::new().expect("rcheevos runtime");
        runtime.activate(4, "0xH1234=1").expect("condition parses");
        let mut calls = 0;
        let mut on = false;
        let mut triggered = false;
        for _ in 0..3 {
            let events = runtime.frame_with(|address, out| {
                calls += 1;
                if address == 0x1234 && !out.is_empty() {
                    out[0] = u8::from(on);
                    1
                } else {
                    0
                }
            });
            triggered |= events
                .iter()
                .any(|event| event.id == 4 && event.event == Event::Triggered);
            on = true;
        }
        assert!(calls > 0, "the reader must be called");
        assert!(triggered, "expected the custom reader to trigger");
    }

    #[test]
    fn reset_clears_the_hit_state() {
        let mut runtime = Runtime::new().expect("rcheevos runtime");
        runtime.activate(3, "0xH00bc=1").expect("condition parses");
        let mut ram = nes_ram();
        ram[0xbc] = 1;
        let _ = runtime.frame(&ram);
        runtime.reset();
        ram[0xbc] = 0;
        let events = runtime.frame(&ram);
        assert!(
            events.iter().all(|event| event.event != Event::Triggered),
            "a reset must not re-trigger: {events:?}"
        );
    }
}
