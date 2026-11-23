//! Per-game playtime, stored inside the save unit:
//! `States/<platform>/<emulator>/<game>/playtime.json`.
//!
//! `seconds` is this device's cumulative counter (Slot increments it while a
//! core is running with the screen on). `total_seconds` and `devices` are the
//! last merged view written by linkSlot; the frontend only advances `seconds`
//! and `total_seconds`, leaving the device map to the sync tool. Keeping the
//! file inside the unit directory means a renamed ROM pulls the merged total
//! back with the save data, and linkSlot can sync it as one small companion
//! file without touching `meta.json`.
//!
//! v2 adds usage counters next to the duration: `plays` (this device, this
//! unit), `plays_devices` + `total_plays` (the merged cross-device view) and
//! `last_played_at` (UTC seconds, max across devices). They travel in the same
//! cloud `playtime.json`, so the long-press B card can show "played N times,
//! last ..." after a save sync.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const FILE_NAME: &str = "playtime.json";
pub const SCHEMA: u32 = 2;

fn schema_default() -> u32 {
    SCHEMA
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Playtime {
    #[serde(default = "schema_default")]
    pub schema: u32,
    #[serde(default)]
    pub seconds: u64,
    #[serde(default)]
    pub total_seconds: u64,
    #[serde(default)]
    pub devices: BTreeMap<String, u64>,
    /// This device's launch count for this unit.
    #[serde(default)]
    pub plays: u64,
    /// Per-device launch counts, written by linkSlot's merge.
    #[serde(default)]
    pub plays_devices: BTreeMap<String, u64>,
    /// Sum of `plays_devices` (all devices).
    #[serde(default)]
    pub total_plays: u64,
    /// UTC seconds of the most recent launch; 0 = never recorded.
    #[serde(default)]
    pub last_played_at: u64,
}

impl Playtime {
    pub fn is_empty(&self) -> bool {
        self.seconds == 0
            && self.total_seconds == 0
            && self.devices.is_empty()
            && self.plays == 0
            && self.plays_devices.is_empty()
            && self.total_plays == 0
            && self.last_played_at == 0
    }
}

pub fn path(unit_dir: &Path) -> PathBuf {
    unit_dir.join(FILE_NAME)
}

/// Human-readable duration for the shelf label. Zero returns `None` so the
/// caller can hide the label instead of printing a meaningless "0".
pub fn format_duration(seconds: u64) -> Option<String> {
    if seconds == 0 {
        return None;
    }
    let minutes = seconds / 60;
    if minutes == 0 {
        return Some("不足 1 分钟".to_string());
    }
    let hours = minutes / 60;
    let minutes = minutes % 60;
    Some(match (hours, minutes) {
        (0, minutes) => format!("{minutes} 分钟"),
        (hours, 0) => format!("{hours} 小时"),
        (hours, minutes) => format!("{hours} 小时 {minutes} 分"),
    })
}

/// Absolute `YYYY-MM-DD HH:MM:SS` in the device's local offset.
///
/// `offset_min` is the RTC/display offset (for example 480 for UTC+8). The
/// stored timestamp is always UTC seconds; only the text is local.
pub fn format_datetime(utc_secs: u64, offset_min: i32) -> Option<String> {
    if utc_secs == 0 {
        return None;
    }
    let local = utc_secs as i64 + i64::from(offset_min) * 60;
    let days = local.div_euclid(86_400);
    let rem = local.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    if !(1..=9999).contains(&year) {
        return None;
    }
    Some(format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02}",
        rem / 3_600,
        (rem % 3_600) / 60,
        rem % 60
    ))
}

/// Howard Hinnant's `civil_from_days`: days since 1970-01-01 → (year, month, day).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let year = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (year + i64::from(month <= 2), month, day)
}

/// Read the local file. Missing, unreadable or malformed files read as zero;
/// an unknown schema is ignored rather than partially interpreted. v1 (schema
/// 1) files are accepted and upgraded in memory: their new counters read as 0.
pub fn read(unit_dir: &Path) -> Playtime {
    let Ok(bytes) = std::fs::read(path(unit_dir)) else {
        return Playtime::default();
    };
    let Ok(playtime) = serde_json::from_slice::<Playtime>(&bytes) else {
        return Playtime::default();
    };
    if playtime.schema > SCHEMA {
        return Playtime::default();
    }
    playtime
}

/// Add `seconds` to this device's counters and write the file atomically.
/// The device map/total from other devices is preserved; linkSlot reconciles
/// the merged total on the next sync.
pub fn add(unit_dir: &Path, seconds: u64) -> io::Result<()> {
    if seconds == 0 {
        return Ok(());
    }
    let mut playtime = read(unit_dir);
    playtime.schema = SCHEMA;
    playtime.seconds = playtime.seconds.saturating_add(seconds);
    playtime.total_seconds = playtime.total_seconds.saturating_add(seconds);
    write(unit_dir, &playtime)
}

/// Record one launch: this device's `plays` +1, `last_played_at` = max(now).
/// Called once per fresh insertion (never for a resumed insertion).
pub fn record_start(unit_dir: &Path, utc_secs: u64) -> io::Result<()> {
    let mut playtime = read(unit_dir);
    playtime.schema = SCHEMA;
    playtime.plays = playtime.plays.saturating_add(1);
    playtime.total_plays = playtime
        .total_plays
        .saturating_add(1)
        .max(playtime.plays);
    if utc_secs > 0 {
        playtime.last_played_at = playtime.last_played_at.max(utc_secs);
    }
    write(unit_dir, &playtime)
}

/// Aggregate one game across every core's unit directory:
/// `<platform>/<core>/<stem>/playtime.json`.
///
/// The merged `total_plays` / `last_played_at` are preferred when present; a
/// game played with two cores therefore shows the larger count and the newer
/// timestamp instead of doubling the count.
pub fn game_stats(platform_dir: &Path, stem: &str) -> (u64, u64) {
    let Ok(entries) = std::fs::read_dir(platform_dir) else {
        return (0, 0);
    };
    let mut plays = 0u64;
    let mut last_played_at = 0u64;
    for entry in entries.flatten() {
        if !entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            continue;
        }
        let path = entry.path().join(stem).join(FILE_NAME);
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(playtime) = serde_json::from_slice::<Playtime>(&bytes) else {
            continue;
        };
        if playtime.schema > SCHEMA {
            continue;
        }
        plays = plays.max(playtime.total_plays.max(playtime.plays));
        last_played_at = last_played_at.max(playtime.last_played_at);
    }
    (plays, last_played_at)
}

pub fn write(unit_dir: &Path, playtime: &Playtime) -> io::Result<()> {
    std::fs::create_dir_all(unit_dir)?;
    let mut bytes = serde_json::to_vec_pretty(playtime).map_err(io::Error::other)?;
    bytes.push(b'\n');
    slot_store::atomic_write(&path(unit_dir), &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "slot-playtime-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn missing_and_bad_files_read_as_zero() {
        let dir = temp_dir();
        assert_eq!(read(&dir), Playtime::default());
        std::fs::write(path(&dir), b"{not json").unwrap();
        assert_eq!(read(&dir), Playtime::default());
        std::fs::write(path(&dir), br#"{"schema":9,"seconds":5}"#).unwrap();
        assert_eq!(read(&dir), Playtime::default());
    }

    #[test]
    fn add_accumulates_and_preserves_the_device_map() {
        let dir = temp_dir();
        let mut seeded = Playtime {
            schema: SCHEMA,
            seconds: 10,
            total_seconds: 30,
            devices: [("dev-a".to_string(), 10), ("dev-b".to_string(), 20)].into(),
            ..Playtime::default()
        };
        write(&dir, &seeded).unwrap();
        add(&dir, 5).unwrap();
        let got = read(&dir);
        assert_eq!(got.seconds, 15);
        assert_eq!(got.total_seconds, 35);
        assert_eq!(got.devices, seeded.devices);
        seeded.seconds = 15;
        seeded.total_seconds = 35;
        assert_eq!(got, seeded);
    }

    #[test]
    fn record_start_counts_and_stamps_the_launch() {
        let dir = temp_dir();
        record_start(&dir, 1_700_000_000).unwrap();
        let first = read(&dir);
        assert_eq!(first.plays, 1);
        assert_eq!(first.total_plays, 1);
        assert_eq!(first.last_played_at, 1_700_000_000);

        record_start(&dir, 1_700_000_500).unwrap();
        let second = read(&dir);
        assert_eq!(second.plays, 2);
        assert_eq!(second.total_plays, 2);
        assert_eq!(second.last_played_at, 1_700_000_500);

        // An older clock must not move the timestamp backwards.
        record_start(&dir, 1_700_000_100).unwrap();
        assert_eq!(read(&dir).last_played_at, 1_700_000_500);
    }

    #[test]
    fn a_schema_one_file_is_upgraded_and_starts_from_zero() {
        let dir = temp_dir();
        std::fs::write(
            path(&dir),
            br#"{"schema":1,"seconds":120,"total_seconds":400,"devices":{"dev-a":280}}"#,
        )
        .unwrap();
        let old = read(&dir);
        assert_eq!(old.seconds, 120);
        assert_eq!(old.plays, 0);
        record_start(&dir, 1_700_000_000).unwrap();
        let got = read(&dir);
        assert_eq!(got.schema, SCHEMA);
        assert_eq!(got.plays, 1);
        assert_eq!(got.seconds, 120, "record_start must keep the duration");
    }

    #[test]
    fn game_stats_takes_the_largest_count_and_newest_time() {
        let root = temp_dir();
        let write_core = |core: &str, plays: u64, total: u64, last: u64| {
            let dir = root.join(core).join("Game");
            std::fs::create_dir_all(&dir).unwrap();
            let playtime = Playtime {
                schema: SCHEMA,
                plays,
                total_plays: total,
                last_played_at: last,
                ..Playtime::default()
            };
            write(&dir, &playtime).unwrap();
        };
        write_core("mgba", 2, 2, 1_700_000_100);
        write_core("gpsp", 5, 9, 1_700_000_050);
        assert_eq!(game_stats(&root, "Game"), (9, 1_700_000_100));
        assert_eq!(game_stats(&root, "Missing"), (0, 0));
    }

    #[test]
    fn datetime_text_uses_the_local_offset_and_seconds() {
        assert_eq!(format_datetime(0, 480), None);
        assert_eq!(
            format_datetime(1, 480).as_deref(),
            Some("1970-01-01 08:00:01")
        );
        // 2024-02-29T00:00:00Z
        assert_eq!(
            format_datetime(1_709_164_800, 0).as_deref(),
            Some("2024-02-29 00:00:00")
        );
        assert_eq!(
            format_datetime(1_709_164_800, 480).as_deref(),
            Some("2024-02-29 08:00:00")
        );
    }

    #[test]
    fn duration_text_covers_the_shelf_boundaries() {
        assert_eq!(format_duration(0), None);
        assert_eq!(format_duration(1).as_deref(), Some("不足 1 分钟"));
        assert_eq!(format_duration(59).as_deref(), Some("不足 1 分钟"));
        assert_eq!(format_duration(60).as_deref(), Some("1 分钟"));
        assert_eq!(format_duration(3_599).as_deref(), Some("59 分钟"));
        assert_eq!(format_duration(3_600).as_deref(), Some("1 小时"));
        assert_eq!(format_duration(3_660).as_deref(), Some("1 小时 1 分"));
        assert_eq!(format_duration(90_061).as_deref(), Some("25 小时 1 分"));
    }
}
