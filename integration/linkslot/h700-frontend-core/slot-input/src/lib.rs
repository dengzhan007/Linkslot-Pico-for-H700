#![no_std]
extern crate alloc;

mod gesture;
pub mod h700;
mod raw;

pub use h700::{
    H700GestureInput, H700InputDrain, H700InputError, H700InputEvent, H700InputServiceV1,
    H700InputSource, H700_ACTION_BUFFER_CAPACITY, H700_INPUT_ABI_VERSION, H700_INPUT_MAX_EVENTS,
};

pub use gesture::{
    Action, Gestures, FF_DOUBLE_TAP_MS, MENU_HOLD_MS, MUTE_CHORD_MS, POWER_HOLD_MS, SELECT_CHORD_MS,
    SELECT_TAP_MS, VOLUME_REPEAT_DELAY_MS, VOLUME_REPEAT_MS,
};
pub use raw::{Btn, InputSource, Millis, RawEvent};
