use std::path::{Path, PathBuf};

use crate::atomic::{atomic_write, sync_dir};
use crate::core::Core;
use crate::Platform;

pub const RING_MAX: usize = 10;

const STATE_EXT: &str = "state";
const THUMB_EXT: &str = "png";
const RESUME: &str = "resume";
const REFUSED: &str = "resume-refused";

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct StateEntry {
    pub stamp: String,
    pub state: PathBuf,
    pub thumb: PathBuf,
}

pub struct StateRing {
    dir: PathBuf,
}

/// 即时存档的目录：`States/<平台目录>/<核心>/<stem>/`（ring，不是单个文件）。
///
/// **只有一个入口，核心名必须由调用方给**：`slot_store::core_for` 允许用户为某张
/// 卡带换核心（core picker），换完核心后它的即时存档本来就落在
/// `States/<平台>/<新核心>/<stem>/`。如果这里偷偷用"平台默认核心"，换过核心的
/// 卡带会**找不到自己以前的即时存档**（读档列表突然变空、旧档变成孤儿文件）。
///
/// 也不提供"默认核心"的重载：同平台有多个核心时（SFC 的 snes9x / supafaust、
/// FC 的 fceumm / nestopia），"默认"与"实际"必然分叉，两个入口早晚有人挑错那个。
/// 调用方（`StateRing::new`、`arcade::save_paths`）显式把核心传进来。
pub fn state_dir(root: &Path, platform: Platform, core: Core, stem: &str) -> PathBuf {
    root.join("States")
        .join(platform.dir_name())
        .join(core.as_str())
        .join(stem)
}

impl StateRing {
    pub fn new(root: &Path, platform: Platform, core: Core, stem: &str) -> Self {
        StateRing {
            dir: state_dir(root, platform, core, stem),
        }
    }

    pub fn push(&self, state: &[u8], thumb_png: &[u8], stamp: &str) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        atomic_write(&self.path(stamp, THUMB_EXT), thumb_png)?;
        atomic_write(&self.path(stamp, STATE_EXT), state)?;
        self.evict()
    }

    pub fn list(&self) -> std::io::Result<Vec<StateEntry>> {
        let dir = match std::fs::read_dir(&self.dir) {
            Ok(d) => d,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };

        let mut entries = Vec::new();
        for entry in dir {
            let path = entry?.path();
            if path.extension().and_then(|e| e.to_str()) != Some(STATE_EXT) {
                continue;
            }
            let Some(stamp) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            if !is_stamp(stamp) {
                continue;
            }
            entries.push(StateEntry {
                thumb: self.path(stamp, THUMB_EXT),
                state: self.path(stamp, STATE_EXT),
                stamp: stamp.to_string(),
            });
        }
        entries.sort_by(|a, b| b.stamp.cmp(&a.stamp));
        Ok(entries)
    }

    pub fn read(&self, stamp: &str) -> std::io::Result<(Vec<u8>, Vec<u8>)> {
        let state = std::fs::read(self.stamped(stamp, STATE_EXT)?)?;
        let thumb = std::fs::read(self.path(stamp, THUMB_EXT)).unwrap_or_default();
        Ok((state, thumb))
    }

    pub fn remove(&self, stamp: &str) -> std::io::Result<()> {
        std::fs::remove_file(self.stamped(stamp, STATE_EXT)?)?;
        let _ = std::fs::remove_file(self.path(stamp, THUMB_EXT));
        Ok(())
    }

    pub fn write_resume(&self, state: &[u8]) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        atomic_write(&self.path(RESUME, STATE_EXT), state)
    }

    /// Save the thumbnail that belongs to `resume.state`.  The state reader
    /// deliberately ignores it, but the cover fallback treats it like any
    /// other state screenshot.
    pub fn write_resume_thumb(&self, thumb_png: &[u8]) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        atomic_write(&self.path(RESUME, THUMB_EXT), thumb_png)
    }

    pub fn read_resume(&self) -> std::io::Result<Option<Vec<u8>>> {
        match std::fs::read(self.path(RESUME, STATE_EXT)) {
            Ok(b) => Ok(Some(b)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn retire_resume(&self, stamp: &str) -> std::io::Result<Option<PathBuf>> {
        let from = self.path(RESUME, STATE_EXT);
        if !from.exists() {
            return Ok(None);
        }
        let to = self.free_refused(stamp);
        std::fs::rename(&from, &to)?;
        sync_dir(&to);
        Ok(Some(to))
    }

    fn free_refused(&self, stamp: &str) -> PathBuf {
        let base = format!("{REFUSED}-{stamp}");
        let first = self.path(&base, STATE_EXT);
        if !first.exists() {
            return first;
        }
        (2u32..)
            .map(|n| self.path(&format!("{base}-{n}"), STATE_EXT))
            .find(|p| !p.exists())
            .unwrap_or(first)
    }

    fn evict(&self) -> std::io::Result<()> {
        for old in self.list()?.into_iter().skip(RING_MAX) {
            std::fs::remove_file(&old.state)?;
            let _ = std::fs::remove_file(&old.thumb);
        }
        Ok(())
    }

    fn path(&self, stem: &str, ext: &str) -> PathBuf {
        self.dir.join(format!("{stem}.{ext}"))
    }

    fn stamped(&self, stamp: &str, ext: &str) -> std::io::Result<PathBuf> {
        if !is_stamp(stamp) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("not a state stamp: {stamp}"),
            ));
        }
        Ok(self.path(stamp, ext))
    }
}

fn is_stamp(s: &str) -> bool {
    const SHAPE: &[u8] = b"0000-00-00_00-00-00";
    s.len() == SHAPE.len()
        && s.bytes().zip(SHAPE).all(|(c, shape)| match shape {
            b'0' => c.is_ascii_digit(),
            _ => c == *shape,
        })
}
