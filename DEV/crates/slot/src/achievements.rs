//! Phase 1 achievements: built-in global meta achievements.
//!
//! Definitions are compiled in; the only card data is the unlock set
//! (`linkSlot/Achievements/unlocks.json`) and a small aggregate cache
//! (`totals.json`) so boot does not rescan every `playtime.json` on FAT32.
//! Unlocks are unioned into the cloud by `linkslot achievements sync`; this
//! module only evaluates and writes locally.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const DIR: &str = "linkSlot/Achievements";
pub const UNLOCKS_FILE: &str = "unlocks.json";
pub const TOTALS_FILE: &str = "totals.json";
pub const SCHEMA: u32 = 1;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Unlocks {
    #[serde(default)]
    pub schema: u32,
    #[serde(default)]
    pub unlocks: BTreeMap<String, u64>,
    #[serde(default)]
    pub updated_at: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Totals {
    #[serde(default)]
    pub schema: u32,
    #[serde(default)]
    pub seconds: u64,
    #[serde(default)]
    pub plays: u64,
    #[serde(default)]
    pub games_played: u64,
    #[serde(default)]
    pub saves: u64,
    #[serde(default)]
    pub carts: u64,
    /// Number of platform racks with at least one cart (0..=6).
    #[serde(default)]
    pub platforms: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    FirstLaunch,
    PlaySeconds(u64),
    TotalPlays(u64),
    GamesPlayed(u64),
    CartCount(u64),
    SaveCount(u64),
    AllPlatforms,
}

pub struct Achievement {
    pub id: &'static str,
    pub name_key: &'static str,
    pub desc_key: &'static str,
    pub points: u32,
    pub rule: Rule,
}

pub const ACHIEVEMENTS: &[Achievement] = &[
    Achievement {
        id: "native:first-launch",
        name_key: "device.app.achievement.first_launch",
        desc_key: "device.app.achievement.first_launch.desc",
        points: 5,
        rule: Rule::FirstLaunch,
    },
    Achievement {
        id: "native:play-1h",
        name_key: "device.app.achievement.play_1h",
        desc_key: "device.app.achievement.play_1h.desc",
        points: 10,
        rule: Rule::PlaySeconds(3_600),
    },
    Achievement {
        id: "native:play-10h",
        name_key: "device.app.achievement.play_10h",
        desc_key: "device.app.achievement.play_10h.desc",
        points: 20,
        rule: Rule::PlaySeconds(36_000),
    },
    Achievement {
        id: "native:play-100h",
        name_key: "device.app.achievement.play_100h",
        desc_key: "device.app.achievement.play_100h.desc",
        points: 50,
        rule: Rule::PlaySeconds(360_000),
    },
    Achievement {
        id: "native:launch-10",
        name_key: "device.app.achievement.launch_10",
        desc_key: "device.app.achievement.launch_10.desc",
        points: 5,
        rule: Rule::TotalPlays(10),
    },
    Achievement {
        id: "native:launch-100",
        name_key: "device.app.achievement.launch_100",
        desc_key: "device.app.achievement.launch_100.desc",
        points: 20,
        rule: Rule::TotalPlays(100),
    },
    Achievement {
        id: "native:games-3",
        name_key: "device.app.achievement.games_3",
        desc_key: "device.app.achievement.games_3.desc",
        points: 5,
        rule: Rule::GamesPlayed(3),
    },
    Achievement {
        id: "native:games-5",
        name_key: "device.app.achievement.games_5",
        desc_key: "device.app.achievement.games_5.desc",
        points: 10,
        rule: Rule::GamesPlayed(5),
    },
    Achievement {
        id: "native:games-10",
        name_key: "device.app.achievement.games_10",
        desc_key: "device.app.achievement.games_10.desc",
        points: 20,
        rule: Rule::GamesPlayed(10),
    },
    Achievement {
        id: "native:carts-10",
        name_key: "device.app.achievement.carts_10",
        desc_key: "device.app.achievement.carts_10.desc",
        points: 5,
        rule: Rule::CartCount(10),
    },
    Achievement {
        id: "native:carts-50",
        name_key: "device.app.achievement.carts_50",
        desc_key: "device.app.achievement.carts_50.desc",
        points: 20,
        rule: Rule::CartCount(50),
    },
    Achievement {
        id: "native:carts-100",
        name_key: "device.app.achievement.carts_100",
        desc_key: "device.app.achievement.carts_100.desc",
        points: 50,
        rule: Rule::CartCount(100),
    },
    Achievement {
        id: "native:saves-1",
        name_key: "device.app.achievement.saves_1",
        desc_key: "device.app.achievement.saves_1.desc",
        points: 5,
        rule: Rule::SaveCount(1),
    },
    Achievement {
        id: "native:saves-10",
        name_key: "device.app.achievement.saves_10",
        desc_key: "device.app.achievement.saves_10.desc",
        points: 15,
        rule: Rule::SaveCount(10),
    },
    Achievement {
        id: "native:all-platforms",
        name_key: "device.app.achievement.all_platforms",
        desc_key: "device.app.achievement.all_platforms.desc",
        points: 30,
        rule: Rule::AllPlatforms,
    },
];

pub fn unlocks_path(root: &Path) -> PathBuf {
    root.join(DIR).join(UNLOCKS_FILE)
}

fn totals_path(root: &Path) -> PathBuf {
    root.join(DIR).join(TOTALS_FILE)
}

pub fn load_unlocks(root: &Path) -> Unlocks {
    let Ok(bytes) = std::fs::read(unlocks_path(root)) else {
        return Unlocks::default();
    };
    match serde_json::from_slice::<Unlocks>(&bytes) {
        Ok(unlocks) if unlocks.schema <= SCHEMA => unlocks,
        _ => Unlocks::default(),
    }
}

pub fn save_unlocks(root: &Path, unlocks: &Unlocks) -> std::io::Result<()> {
    let path = unlocks_path(root);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut bytes = serde_json::to_vec_pretty(unlocks).map_err(std::io::Error::other)?;
    bytes.push(b'\n');
    slot_store::atomic_write(&path, &bytes)
}

pub fn load_totals(root: &Path) -> Option<Totals> {
    let bytes = std::fs::read(totals_path(root)).ok()?;
    let totals = serde_json::from_slice::<Totals>(&bytes).ok()?;
    (totals.schema <= SCHEMA).then_some(totals)
}

pub fn save_totals(root: &Path, totals: &Totals) -> std::io::Result<()> {
    let path = totals_path(root);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut bytes = serde_json::to_vec_pretty(totals).map_err(std::io::Error::other)?;
    bytes.push(b'\n');
    slot_store::atomic_write(&path, &bytes)
}

/// Aggregate `total_seconds` / `total_plays` / game count across every core's
/// `playtime.json`. Used only on the first boot (or when the cache is missing).
pub fn scan_playtime(root: &Path) -> (u64, u64, u64) {
    let states = root.join("States");
    let Ok(platforms) = std::fs::read_dir(&states) else {
        return (0, 0, 0);
    };
    let mut seconds = 0u64;
    let mut plays = 0u64;
    let mut games = 0u64;
    for platform in platforms.flatten() {
        if !platform.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            continue;
        }
        let Ok(cores) = std::fs::read_dir(platform.path()) else {
            continue;
        };
        for core in cores.flatten() {
            if !core.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
                continue;
            }
            let Ok(games_dir) = std::fs::read_dir(core.path()) else {
                continue;
            };
            for game in games_dir.flatten() {
                if !game.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
                    continue;
                }
                let path = game.path().join(crate::playtime::FILE_NAME);
                let Ok(bytes) = std::fs::read(path) else {
                    continue;
                };
                let Ok(playtime) = serde_json::from_slice::<crate::playtime::Playtime>(&bytes)
                else {
                    continue;
                };
                if playtime.schema > crate::playtime::SCHEMA {
                    continue;
                }
                let game_seconds = playtime.total_seconds.max(playtime.seconds);
                let game_plays = playtime.total_plays.max(playtime.plays);
                if game_seconds > 0 || game_plays > 0 {
                    games = games.saturating_add(1);
                }
                seconds = seconds.saturating_add(game_seconds);
                plays = plays.saturating_add(game_plays);
            }
        }
    }
    (seconds, plays, games)
}

/// Count `.sav` / `.srm` files under `Saves/`.
pub fn count_saves(root: &Path) -> u64 {
    fn walk(dir: &Path, count: &mut u64) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                walk(&entry.path(), count);
            } else if kind.is_file()
                && entry
                    .path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("sav") || ext.eq_ignore_ascii_case("srm"))
            {
                *count = count.saturating_add(1);
            }
        }
    }
    let mut count = 0;
    walk(&root.join("Saves"), &mut count);
    count
}

/// Load the cached totals (scanning once if absent), then refresh the cheap
/// live fields from the current shelf and save counts.
pub fn refresh_totals(root: &Path, carts: u64, platforms: u8) -> Totals {
    let mut totals = load_totals(root).unwrap_or_else(|| {
        let (seconds, plays, games) = scan_playtime(root);
        Totals {
            schema: SCHEMA,
            seconds,
            plays,
            games_played: games,
            ..Totals::default()
        }
    });
    totals.schema = SCHEMA;
    totals.carts = carts;
    totals.platforms = platforms;
    totals.saves = count_saves(root);
    let _ = save_totals(root, &totals);
    totals
}

/// Apply one recorded launch: `plays` +1, the game counts once when it had no
/// counter before, and the unlock rules are re-evaluated by the caller.
pub fn add_launch(totals: &mut Totals, seconds_before: u64, plays_before: u64) {
    if seconds_before == 0 && plays_before == 0 {
        totals.games_played = totals.games_played.saturating_add(1);
    }
    totals.plays = totals.plays.saturating_add(1);
}

/// Apply newly flushed playtime for one game.
pub fn add_playtime(totals: &mut Totals, seconds_before: u64, seconds_now: u64) {
    totals.seconds = totals
        .seconds
        .saturating_add(seconds_now.saturating_sub(seconds_before));
}

/// Newly reachable achievements (not in `unlocks` yet).
pub fn evaluate(unlocks: &Unlocks, totals: &Totals) -> Vec<&'static Achievement> {
    ACHIEVEMENTS
        .iter()
        .filter(|achievement| {
            !unlocks.unlocks.contains_key(achievement.id) && rule_reached(&achievement.rule, totals)
        })
        .collect()
}

pub fn rule_reached(rule: &Rule, totals: &Totals) -> bool {
    let (value, target) = rule_progress(rule, totals);
    target > 0 && value >= target
}

pub fn rule_progress(rule: &Rule, totals: &Totals) -> (u64, u64) {
    match *rule {
        Rule::FirstLaunch => (u64::from(totals.plays > 0), 1),
        Rule::PlaySeconds(target) => (totals.seconds, target),
        Rule::TotalPlays(target) => (totals.plays, target),
        Rule::GamesPlayed(target) => (totals.games_played, target),
        Rule::CartCount(target) => (totals.carts, target),
        Rule::SaveCount(target) => (totals.saves, target),
        Rule::AllPlatforms => (u64::from(totals.platforms), 6),
    }
}

/// Unlock `ids` at `now` and write the file; returns the achievements that
/// were actually new (the caller shows a notice for each).
pub fn unlock(
    root: &Path,
    unlocks: &mut Unlocks,
    achievements: &[&'static Achievement],
    now: u64,
) -> std::io::Result<Vec<&'static Achievement>> {
    let mut newly = Vec::new();
    for achievement in achievements {
        if unlocks.unlocks.contains_key(achievement.id) {
            continue;
        }
        unlocks.unlocks.insert(achievement.id.to_string(), now);
        newly.push(*achievement);
    }
    if newly.is_empty() {
        return Ok(newly);
    }
    unlocks.schema = SCHEMA;
    unlocks.updated_at = unlocks.updated_at.max(now);
    save_unlocks(root, unlocks)?;
    Ok(newly)
}

pub fn unlocked_count(unlocks: &Unlocks) -> usize {
    ACHIEVEMENTS
        .iter()
        .filter(|achievement| unlocks.unlocks.contains_key(achievement.id))
        .count()
}

pub fn total_points(unlocks: &Unlocks) -> u32 {
    ACHIEVEMENTS
        .iter()
        .filter(|achievement| unlocks.unlocks.contains_key(achievement.id))
        .map(|achievement| achievement.points)
        .sum()
}

/// Detail-card line: `成就 3/15 · 累计 25 分`.
pub fn summary(lang: slot_i18n::Language, unlocks: &Unlocks) -> String {
    slot_i18n::t_args(
        lang,
        "device.app.achievement.summary",
        &[
            ("unlocked", unlocked_count(unlocks).to_string().as_str()),
            ("total", ACHIEVEMENTS.len().to_string().as_str()),
            ("points", total_points(unlocks).to_string().as_str()),
        ],
    )
}

pub fn unit_key(rule: &Rule) -> &'static str {
    match rule {
        Rule::PlaySeconds(_) => "device.app.achievement.unit_seconds",
        Rule::TotalPlays(_) => "device.app.achievement.unit_plays",
        Rule::GamesPlayed(_) => "device.app.achievement.unit_games",
        Rule::CartCount(_) => "device.app.achievement.unit_carts",
        Rule::SaveCount(_) => "device.app.achievement.unit_saves",
        Rule::AllPlatforms => "device.app.achievement.unit_platforms",
        Rule::FirstLaunch => "device.app.achievement.unit_first",
    }
}

fn rule_progress_text(lang: slot_i18n::Language, rule: &Rule, totals: &Totals) -> Option<String> {
    // Seconds make noisy progress text ("3599/3600 秒"); keep the duration
    // rows clean and only show counters for the other rules.
    if matches!(rule, Rule::PlaySeconds(_)) {
        return None;
    }
    let (value, target) = rule_progress(rule, totals);
    if value == 0 || value >= target {
        return None;
    }
    let unit = slot_i18n::t(lang, unit_key(rule));
    Some(format!("{value}/{target} {unit}"))
}

/// Plain rows for the utility list page. `√` is a GB2312 glyph, so it renders
/// with the shipped subset font.
pub fn rows(lang: slot_i18n::Language, unlocks: &Unlocks, totals: &Totals) -> Vec<String> {
    ACHIEVEMENTS
        .iter()
        .map(|achievement| {
            let unlocked = unlocks.unlocks.contains_key(achievement.id);
            let mark = if unlocked { "√" } else { "·" };
            let name = slot_i18n::t(lang, achievement.name_key);
            let points = format!("+{}", achievement.points);
            match rule_progress_text(lang, &achievement.rule, totals) {
                Some(progress) if !unlocked => {
                    format!("{mark} {name}  {progress}  {points}")
                }
                _ => format!("{mark} {name}  {points}"),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn totals(seconds: u64, plays: u64, games: u64, carts: u64, saves: u64) -> Totals {
        Totals {
            schema: SCHEMA,
            seconds,
            plays,
            games_played: games,
            saves,
            carts,
            platforms: 2,
        }
    }

    #[test]
    fn rule_boundaries_match() {
        let base = totals(3_599, 0, 2, 9, 0);
        assert!(!rule_reached(&Rule::FirstLaunch, &base));
        assert!(rule_reached(&Rule::FirstLaunch, &totals(0, 1, 0, 0, 0)));
        assert!(!rule_reached(&Rule::PlaySeconds(3_600), &base));
        assert!(rule_reached(&Rule::PlaySeconds(3_600), &totals(3_600, 0, 0, 0, 0)));
        assert!(!rule_reached(&Rule::TotalPlays(10), &base));
        assert!(rule_reached(&Rule::TotalPlays(10), &totals(0, 10, 0, 0, 0)));
        assert!(!rule_reached(&Rule::CartCount(10), &base));
        assert!(!rule_reached(&Rule::AllPlatforms, &base));
        assert!(!rule_reached(&Rule::SaveCount(1), &base));
        assert!(rule_reached(&Rule::SaveCount(1), &totals(0, 0, 0, 0, 1)));
    }

    #[test]
    fn unlock_only_adds_once_and_keeps_the_first_time() {
        let root = std::env::temp_dir().join(format!(
            "slot-achievements-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let mut unlocks = load_unlocks(&root);
        let first = &ACHIEVEMENTS[0];
        let newly = unlock(&root, &mut unlocks, &[first], 100).unwrap();
        assert_eq!(newly.len(), 1);
        assert_eq!(unlocks.unlocks.get(first.id), Some(&100));
        let again = unlock(&root, &mut unlocks, &[first], 200).unwrap();
        assert!(again.is_empty());
        assert_eq!(unlocks.unlocks.get(first.id), Some(&100));
        assert_eq!(load_unlocks(&root), unlocks);
    }

    #[test]
    fn add_launch_counts_the_first_game_and_plays() {
        let mut totals = totals(0, 0, 0, 0, 0);
        add_launch(&mut totals, 0, 0);
        assert_eq!(totals.plays, 1);
        assert_eq!(totals.games_played, 1);
        add_launch(&mut totals, 30, 1);
        assert_eq!(totals.plays, 2);
        assert_eq!(totals.games_played, 1);
        add_playtime(&mut totals, 30, 90);
        assert_eq!(totals.seconds, 60);
    }
}
