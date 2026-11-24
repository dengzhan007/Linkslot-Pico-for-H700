use std::fmt;
use std::path::Path;

use crate::link::Link;
use crate::rumble::Rumble;

pub const GBA_W: u32 = 240;
pub const GBA_H: u32 = 160;

/// Upper bound for a frame any core we can load may hand over. It sizes the
/// shared frame buffer once, so it is not a resolution any machine runs at
/// (Game Boy is 160x144, GBA 240x160, Famicom 256x224): it only has to be at
/// least as large as the biggest canvas the frontend can ask for.
pub const MAX_FRAME_W: u32 = 1024;
pub const MAX_FRAME_H: u32 = 576;

#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub struct ButtonMask(pub u16);

impl ButtonMask {
    pub const B: u16 = 1 << 0;
    pub const Y: u16 = 1 << 1;
    pub const SELECT: u16 = 1 << 2;
    pub const START: u16 = 1 << 3;
    pub const UP: u16 = 1 << 4;
    pub const DOWN: u16 = 1 << 5;
    pub const LEFT: u16 = 1 << 6;
    pub const RIGHT: u16 = 1 << 7;
    pub const A: u16 = 1 << 8;
    pub const X: u16 = 1 << 9;
    pub const L: u16 = 1 << 10;
    pub const R: u16 = 1 << 11;

    pub fn without_turbo(self) -> ButtonMask {
        ButtonMask(self.0 & !(Self::X | Self::Y))
    }

    pub fn turbo(self, frame: u32) -> ButtonMask {
        let mut mask = self.0 & !(Self::X | Self::Y);
        if (frame / 3).is_multiple_of(2) {
            if self.0 & Self::X != 0 {
                mask |= Self::A;
            }
            if self.0 & Self::Y != 0 {
                mask |= Self::B;
            }
        }
        ButtonMask(mask)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct AvInfo {
    pub fps: f64,
    pub sample_rate: f64,
}

#[derive(Debug)]
pub enum CoreError {
    Io(std::io::Error),
    Load(String),
    Unsupported(String),
    State(String),
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoreError::Io(e) => write!(f, "io: {e}"),
            CoreError::Load(m) => write!(f, "load: {m}"),
            CoreError::Unsupported(m) => write!(f, "unsupported: {m}"),
            CoreError::State(m) => write!(f, "state: {m}"),
        }
    }
}

impl std::error::Error for CoreError {}

impl From<std::io::Error> for CoreError {
    fn from(e: std::io::Error) -> Self {
        CoreError::Io(e)
    }
}

/// One addressable region a core exposes through `SET_MEMORY_MAPS`.
///
/// `ptr` is stored as an integer because the core owns the buffer for the
/// whole session; the frontend must never free or move it. `select` and
/// `disconnect` implement libretro's masked mapping rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryRegion {
    pub ptr: usize,
    pub offset: usize,
    pub start: usize,
    pub select: usize,
    pub disconnect: usize,
    pub len: usize,
}

pub trait RetroCore: Send {
    fn set_cheats(&mut self, _codes: &[String]) {}
    fn load(&mut self, rom: &Path) -> Result<(), CoreError>;
    fn run_frame(&mut self, input: ButtonMask);
    fn run_frame_linked(&mut self, p1: ButtonMask, _p2: ButtonMask) {
        self.run_frame(p1);
    }
    fn set_option(&mut self, _key: &str, _value: &str) {}
    fn set_frame_skip(&mut self, _skip: bool) {}
    /// Fixed canvas the frontend scales this core's frames from. Cores that
    /// need another size (e.g. a 256x224 Famicom canvas) override it; the
    /// core's own picture is still centred inside it.
    fn frame_size(&self) -> (u32, u32) {
        (GBA_W, GBA_H)
    }
    fn video_xrgb8888(&self) -> &[u8];
    fn take_audio(&mut self) -> Vec<i16>;
    fn serialize(&mut self) -> Result<Vec<u8>, CoreError>;
    fn unserialize(&mut self, data: &[u8]) -> Result<(), CoreError>;
    /// 主机工作内存（libretro `RETRO_MEMORY_SYSTEM_RAM`），用于成就求值。
    ///
    /// 返回的切片只在当前帧内有效：核心可能在 `run_frame`/`unserialize`
    /// 中重建缓冲区，调用方必须每帧重取，不能跨帧持有。
    fn system_ram(&self) -> Option<&[u8]> {
        None
    }
    /// Static address map exposed by the core (GBA EWRAM/IWRAM, ...).
    fn memory_regions(&self) -> Option<&[MemoryRegion]> {
        None
    }
    /// Read emulated memory at `address`; returns the number of bytes read.
    ///
    /// The default maps NES-style 2 KB work RAM through 0x1FFF, which keeps
    /// the FC test path working. Cores with SET_MEMORY_MAPS override this.
    fn read_memory(&self, address: u32, out: &mut [u8]) -> usize {
        let Some(ram) = self.system_ram() else {
            return 0;
        };
        let mut written = 0;
        for (offset, slot) in out.iter_mut().enumerate() {
            let address = address as usize + offset;
            let mapped = match address {
                0x0000..=0x07FF => address,
                0x0800..=0x1FFF => address & 0x07FF,
                _ => break,
            };
            let Some(byte) = ram.get(mapped) else {
                break;
            };
            *slot = *byte;
            written += 1;
        }
        written
    }
    fn save_ram(&self) -> Option<Vec<u8>>;
    fn load_save_ram(&mut self, data: &[u8]) -> Result<(), CoreError>;
    fn av_info(&self) -> AvInfo;
    fn rumble(&self) -> Rumble {
        Rumble::default()
    }
    fn net(&self) -> Link {
        Link::default()
    }
    fn start_link(&mut self, _client_id: u16) {}
    fn pump_link(&mut self) {}
    fn stop_link(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Recorder(Vec<ButtonMask>);

    impl RetroCore for Recorder {
        fn load(&mut self, _rom: &Path) -> Result<(), CoreError> {
            Ok(())
        }
        fn run_frame(&mut self, input: ButtonMask) {
            self.0.push(input);
        }
        fn video_xrgb8888(&self) -> &[u8] {
            &[]
        }
        fn take_audio(&mut self) -> Vec<i16> {
            Vec::new()
        }
        fn serialize(&mut self) -> Result<Vec<u8>, CoreError> {
            Ok(Vec::new())
        }
        fn unserialize(&mut self, _data: &[u8]) -> Result<(), CoreError> {
            Ok(())
        }
        fn save_ram(&self) -> Option<Vec<u8>> {
            None
        }
        fn load_save_ram(&mut self, _data: &[u8]) -> Result<(), CoreError> {
            Ok(())
        }
        fn av_info(&self) -> AvInfo {
            AvInfo {
                fps: 60.0,
                sample_rate: 48_000.0,
            }
        }
    }

    #[test]
    fn system_ram_is_optional_and_uses_the_libretro_id() {
        assert_eq!(crate::MEMORY_SYSTEM_RAM, 2);
        let core = Recorder::default();
        assert!(core.system_ram().is_none(), "the default is no system RAM");
    }

    #[test]
    fn a_core_without_link_mode_runs_player_1_alone() {
        let mut core = Recorder::default();
        core.run_frame_linked(ButtonMask(ButtonMask::A), ButtonMask(ButtonMask::B));
        assert_eq!(core.0, vec![ButtonMask(ButtonMask::A)]);
    }

    #[test]
    fn a_core_that_does_not_name_a_canvas_gets_the_gba_one() {
        assert_eq!(Recorder::default().frame_size(), (GBA_W, GBA_H));
    }
}
