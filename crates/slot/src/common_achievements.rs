//! Built-in per-ROM achievements.
//!
//! These are compiled into the app and are independent of achievement packs:
//! every ROM gets the same five goals, evaluated from that ROM's own
//! `playtime.json` (`plays` / `total_plays` and `seconds` / `total_seconds`).
//! Unlocks live in the normal per-game unlock file and therefore travel through
//! the existing cloud union sync.

use crate::cheevos::Rule;
use crate::playtime::Playtime;

pub const NUMERIC_BASE: u32 = 1_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Goal {
    FirstLaunch,
    PlaySeconds(u64),
    LaunchCount(u64),
}

pub struct Definition {
    pub id: &'static str,
    pub slug: &'static str,
    pub title_key: &'static str,
    pub desc_key: &'static str,
    pub points: u32,
    pub goal: Goal,
}

pub const DEFINITIONS: &[Definition] = &[
    Definition {
        id: "builtin:first-launch",
        slug: "first-launch",
        title_key: "device.app.achievement.common.first_launch",
        desc_key: "device.app.achievement.common.first_launch.desc",
        points: 5,
        goal: Goal::FirstLaunch,
    },
    Definition {
        id: "builtin:play-1h",
        slug: "play-1h",
        title_key: "device.app.achievement.common.play_1h",
        desc_key: "device.app.achievement.common.play_1h.desc",
        points: 10,
        goal: Goal::PlaySeconds(3_600),
    },
    Definition {
        id: "builtin:play-5h",
        slug: "play-5h",
        title_key: "device.app.achievement.common.play_5h",
        desc_key: "device.app.achievement.common.play_5h.desc",
        points: 20,
        goal: Goal::PlaySeconds(18_000),
    },
    Definition {
        id: "builtin:launch-10",
        slug: "launch-10",
        title_key: "device.app.achievement.common.launch_10",
        desc_key: "device.app.achievement.common.launch_10.desc",
        points: 5,
        goal: Goal::LaunchCount(10),
    },
    Definition {
        id: "builtin:launch-30",
        slug: "launch-30",
        title_key: "device.app.achievement.common.launch_30",
        desc_key: "device.app.achievement.common.launch_30.desc",
        points: 15,
        goal: Goal::LaunchCount(30),
    },
];

pub fn rules(lang: slot_i18n::Language) -> Vec<Rule> {
    DEFINITIONS
        .iter()
        .enumerate()
        .map(|(index, definition)| Rule {
            numeric: NUMERIC_BASE + index as u32,
            id: definition.id.to_string(),
            title: slot_i18n::t(lang, definition.title_key).to_string(),
            description: slot_i18n::t(lang, definition.desc_key).to_string(),
            points: definition.points,
            condition: String::new(),
        })
        .collect()
}

pub fn reached(definition: &Definition, playtime: &Playtime) -> bool {
    match definition.goal {
        Goal::FirstLaunch => playtime.plays.max(playtime.total_plays) >= 1,
        Goal::PlaySeconds(seconds) => playtime.seconds.max(playtime.total_seconds) >= seconds,
        Goal::LaunchCount(count) => playtime.plays.max(playtime.total_plays) >= count,
    }
}

pub fn badge_rgba(id: &str, size: u32) -> Option<Vec<u8>> {
    let definition = DEFINITIONS.iter().find(|definition| definition.id == id)?;
    let bytes: &[u8] = match (definition.slug, size) {
        ("first-launch", 40) => include_bytes!("assets/builtin-badges/first-launch-40.rgba"),
        ("first-launch", _) => include_bytes!("assets/builtin-badges/first-launch.rgba"),
        ("play-1h", 40) => include_bytes!("assets/builtin-badges/play-1h-40.rgba"),
        ("play-1h", _) => include_bytes!("assets/builtin-badges/play-1h.rgba"),
        ("play-5h", 40) => include_bytes!("assets/builtin-badges/play-5h-40.rgba"),
        ("play-5h", _) => include_bytes!("assets/builtin-badges/play-5h.rgba"),
        ("launch-10", 40) => include_bytes!("assets/builtin-badges/launch-10-40.rgba"),
        ("launch-10", _) => include_bytes!("assets/builtin-badges/launch-10.rgba"),
        ("launch-30", 40) => include_bytes!("assets/builtin-badges/launch-30-40.rgba"),
        ("launch-30", _) => include_bytes!("assets/builtin-badges/launch-30.rgba"),
        _ => return None,
    };
    Some(bytes.to_vec())
}

pub fn png(id: &str) -> Option<&'static [u8]> {
    let definition = DEFINITIONS.iter().find(|definition| definition.id == id)?;
    Some(match definition.slug {
        "first-launch" => include_bytes!("web/static/badges/first-launch.png"),
        "play-1h" => include_bytes!("web/static/badges/play-1h.png"),
        "play-5h" => include_bytes!("web/static/badges/play-5h.png"),
        "launch-10" => include_bytes!("web/static/badges/launch-10.png"),
        "launch-30" => include_bytes!("web/static/badges/launch-30.png"),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn playtime(seconds: u64, plays: u64, total_seconds: u64, total_plays: u64) -> Playtime {
        Playtime {
            seconds,
            plays,
            total_seconds,
            total_plays,
            ..Playtime::default()
        }
    }

    #[test]
    fn local_counts_unlock_before_cloud_merge() {
        let local = playtime(3_600, 10, 0, 0);
        assert!(reached(&DEFINITIONS[1], &local));
        assert!(reached(&DEFINITIONS[3], &local));
        assert!(!reached(&DEFINITIONS[2], &local));
        assert!(!reached(&DEFINITIONS[4], &local));
    }

    #[test]
    fn merged_counts_unlock_without_local_history() {
        let merged = playtime(0, 1, 18_000, 30);
        assert!(reached(&DEFINITIONS[0], &merged));
        assert!(reached(&DEFINITIONS[2], &merged));
        assert!(reached(&DEFINITIONS[4], &merged));
    }

    #[test]
    fn badges_are_rgba_at_both_sizes() {
        for definition in DEFINITIONS {
            assert_eq!(badge_rgba(definition.id, 40).map(|b| b.len()), Some(40 * 40 * 4));
            assert_eq!(badge_rgba(definition.id, 44).map(|b| b.len()), Some(44 * 44 * 4));
        }
        assert!(badge_rgba("builtin:missing", 40).is_none());
    }
}
