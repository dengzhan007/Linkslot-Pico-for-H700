//! Phase 2 per-game achievement definitions and local unlock records.
//!
//! Definitions are user-supplied data under
//! `linkSlot/Achievements/defs/<platform>/<stem>.json`; they are never shipped
//! with the card image. The engine field keeps the door open for native rules,
//! but phase 2 only evaluates `rcheevos` condition strings.
//!
//! Unlocks live under `linkSlot/Achievements/games/<platform>/<sha256>.json`.
//! The file is keyed by the ROM hash, not the stem, so renaming a ROM keeps its
//! progress; a different dump starts a separate record.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const SCHEMA: u32 = 1;
pub const DEFS_DIR: &str = "linkSlot/Achievements/defs";
pub const GAMES_DIR: &str = "linkSlot/Achievements/games";
pub const ENGINE_RCHEEVOS: &str = "rcheevos";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameInfo {
    #[serde(default)]
    pub platform: String,
    #[serde(default)]
    pub sha256: String,
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AchievementDef {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub points: u32,
    #[serde(default)]
    pub engine: String,
    #[serde(default)]
    pub condition: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameDefinition {
    #[serde(default)]
    pub schema: u32,
    pub game: GameInfo,
    #[serde(default)]
    pub achievements: Vec<AchievementDef>,
}

/// One active condition, already validated and assigned a numeric id for
/// rcheevos (which uses `u32` ids internally).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rule {
    pub numeric: u32,
    pub id: String,
    pub title: String,
    pub description: String,
    pub points: u32,
    pub condition: String,
}

#[derive(Clone, Debug)]
pub struct GameSet {
    pub game: GameInfo,
    pub rules: Vec<Rule>,
}

impl GameSet {
    pub fn rule(&self, numeric: u32) -> Option<&Rule> {
        self.rules.iter().find(|rule| rule.numeric == numeric)
    }
}

/// 成就徽章 PNG 的卡内路径（RA 成就的 `<id>.png`）。
pub fn badge_path(root: &Path, game: &GameInfo, rule: &Rule) -> Option<PathBuf> {
    let id = rule.id.rsplit(':').next()?;
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if game.platform.is_empty() || game.sha256.is_empty() {
        return None;
    }
    Some(
        root.join("linkSlot/Achievements/badges")
            .join(&game.platform)
            .join(&game.sha256)
            .join(format!("{id}.png")),
    )
}

/// 已缩放到 `size x size` 的 RGBA 徽章；没有徽章时返回 None。
pub fn badge_rgba(root: &Path, game: &GameInfo, rule: &Rule, size: u32) -> Option<Vec<u8>> {
    let path = badge_path(root, game, rule)?;
    slot_ui::contain(&path, size, size)
}

pub fn defs_dir(root: &Path, platform: &str) -> PathBuf {
    root.join(DEFS_DIR).join(platform)
}

pub fn def_path(root: &Path, platform: &str, stem: &str) -> PathBuf {
    defs_dir(root, platform).join(format!("{stem}.json"))
}

/// Load and validate the definition for a game stem. Missing files, unknown
/// schemas and unsupported engines are reported as `None`/`Err`; the caller
/// shows "no achievements" rather than guessing.
pub fn load_for_stem(root: &Path, platform: &str, stem: &str) -> Result<Option<GameSet>, String> {
    load_path(root, platform, &def_path(root, platform, stem))
}

/// Load the definition for the actual ROM content.
///
/// The filename is only a hint: every `.json` in the platform directory is
/// checked for a matching `game.sha256`, so renaming a ROM (or the definition
/// file) does not lose its achievements. The stem-named file is tried first
/// and an old definition with an empty sha256 is still accepted as a fallback.
/// A stem-named definition whose sha256 disagrees with the live ROM is refused,
/// so two different dumps cannot accidentally share a name-based definition.
pub fn load_for_game(
    root: &Path,
    platform: &str,
    sha256: &str,
    stem: &str,
) -> Result<Option<GameSet>, String> {
    let wanted = sha256.trim().to_ascii_lowercase();
    let exact = def_path(root, platform, stem);
    let mut paths = Vec::new();
    if !wanted.is_empty() {
        if exact.is_file() {
            paths.push(exact.clone());
        }
        if let Ok(entries) = std::fs::read_dir(defs_dir(root, platform)) {
            let mut rest: Vec<PathBuf> = entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
                .filter(|path| path != &exact)
                .collect();
            rest.sort();
            paths.extend(rest);
        }
        for path in paths {
            let Ok(Some(set)) = load_path(root, platform, &path) else {
                continue;
            };
            if !set.game.sha256.is_empty() && set.game.sha256.eq_ignore_ascii_case(&wanted) {
                return Ok(Some(set));
            }
        }
    }
    match load_path(root, platform, &exact)? {
        Some(set)
            if wanted.is_empty()
                || set.game.sha256.is_empty()
                || set.game.sha256.eq_ignore_ascii_case(&wanted) =>
        {
            Ok(Some(set))
        }
        _ => Ok(None),
    }
}

/// Load a definition whose `game.sha256` matches, independent of filename.
pub fn load_for_sha(
    root: &Path,
    platform: &str,
    sha256: &str,
) -> Result<Option<GameSet>, String> {
    let wanted = sha256.trim().to_ascii_lowercase();
    if wanted.is_empty() {
        return Ok(None);
    }
    let entries = match std::fs::read_dir(defs_dir(root, platform)) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("读取成就定义目录失败：{error}")),
    };
    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect();
    paths.sort();
    for path in paths {
        let Ok(Some(set)) = load_path(root, platform, &path) else {
            continue;
        };
        if !set.game.sha256.is_empty() && set.game.sha256.eq_ignore_ascii_case(&wanted) {
            return Ok(Some(set));
        }
    }
    Ok(None)
}

fn load_path(
    _root: &Path,
    platform: &str,
    path: &Path,
) -> Result<Option<GameSet>, String> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("读取成就定义失败 {}：{error}", path.display())),
    };
    let definition: GameDefinition = serde_json::from_slice(&bytes)
        .map_err(|error| format!("成就定义不是合法 JSON {}：{error}", path.display()))?;
    if definition.schema > SCHEMA {
        return Err(format!(
            "成就定义版本 {} 高于本机支持的 {}",
            definition.schema, SCHEMA
        ));
    }
    if !definition.game.platform.is_empty() && !definition.game.platform.eq_ignore_ascii_case(platform) {
        return Err(format!(
            "成就定义平台 {} 与当前平台 {platform} 不一致",
            definition.game.platform
        ));
    }

    let mut rules = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for achievement in definition.achievements {
        if achievement.engine != ENGINE_RCHEEVOS {
            continue;
        }
        if achievement.id.trim().is_empty()
            || achievement.condition.trim().is_empty()
            || !seen.insert(achievement.id.clone())
        {
            continue;
        }
        rules.push(Rule {
            numeric: rules.len() as u32 + 1,
            id: achievement.id,
            title: achievement.title,
            description: achievement.description,
            points: achievement.points,
            condition: achievement.condition,
        });
    }
    if rules.is_empty() {
        return Ok(None);
    }
    Ok(Some(GameSet {
        game: definition.game,
        rules,
    }))
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Unlocks {
    #[serde(default)]
    pub schema: u32,
    #[serde(default)]
    pub platform: String,
    #[serde(default)]
    pub sha256: String,
    #[serde(default)]
    pub unlocks: BTreeMap<String, u64>,
    #[serde(default)]
    pub updated_at: u64,
}

pub fn unlock_path(root: &Path, platform: &str, sha256: &str) -> PathBuf {
    root.join(GAMES_DIR).join(platform).join(format!("{sha256}.json"))
}

pub fn load_unlocks(root: &Path, platform: &str, sha256: &str) -> Unlocks {
    let path = unlock_path(root, platform, sha256);
    let Ok(bytes) = std::fs::read(path) else {
        return Unlocks::default();
    };
    match serde_json::from_slice::<Unlocks>(&bytes) {
        Ok(unlocks) if unlocks.schema <= SCHEMA => unlocks,
        _ => Unlocks::default(),
    }
}

pub fn save_unlocks(root: &Path, unlocks: &Unlocks) -> std::io::Result<()> {
    let path = unlock_path(root, &unlocks.platform, &unlocks.sha256);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut bytes = serde_json::to_vec_pretty(unlocks).map_err(std::io::Error::other)?;
    bytes.push(b'\n');
    slot_store::atomic_write(&path, &bytes)
}

/// Create an empty record when a definition is installed, so the next sync can
/// pull a cloud copy even before the first unlock on this device.
pub fn ensure_record(root: &Path, game: &GameInfo) -> std::io::Result<()> {
    if game.platform.trim().is_empty() || game.sha256.trim().is_empty() {
        return Ok(());
    }
    let path = unlock_path(root, &game.platform, &game.sha256);
    if path.is_file() {
        return Ok(());
    }
    save_unlocks(
        root,
        &Unlocks {
            schema: SCHEMA,
            platform: game.platform.clone(),
            sha256: game.sha256.clone(),
            ..Unlocks::default()
        },
    )
}

/// Add unlocks for `ids`, keeping the earliest timestamp, and persist. Returns
/// only the ids that were not already present.
pub fn record(
    root: &Path,
    game: &GameInfo,
    ids: &[String],
    now: u64,
) -> std::io::Result<Vec<String>> {
    if game.sha256.trim().is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "achievement definition has no sha256",
        ));
    }
    let mut unlocks = load_unlocks(root, &game.platform, &game.sha256);
    unlocks.schema = SCHEMA;
    unlocks.platform = game.platform.clone();
    unlocks.sha256 = game.sha256.clone();
    let mut newly = Vec::new();
    for id in ids {
        if unlocks.unlocks.contains_key(id) {
            continue;
        }
        unlocks.unlocks.insert(id.clone(), now);
        newly.push(id.clone());
    }
    if newly.is_empty() {
        return Ok(newly);
    }
    unlocks.updated_at = unlocks.updated_at.max(now);
    save_unlocks(root, &unlocks)?;
    Ok(newly)
}

/// One installed definition, for the Web game picker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstalledGame {
    pub platform: String,
    pub stem: String,
    pub title: String,
    pub sha256: String,
    pub total: u32,
    pub unlocked: u32,
    pub points: u32,
}

/// Scan `linkSlot/Achievements/defs/<platform>/*.json` for the Web picker.
/// Invalid or future-schema files are skipped; definitions are user data, so a
/// broken file must not hide the rest.
pub fn installed_games(root: &Path) -> Vec<InstalledGame> {
    let base = root.join(DEFS_DIR);
    let Ok(platforms) = std::fs::read_dir(&base) else {
        return Vec::new();
    };
    let mut games = Vec::new();
    for platform in platforms.flatten() {
        if !platform.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            continue;
        }
        let Some(platform_name) = platform.file_name().to_str().map(str::to_string) else {
            continue;
        };
        let Ok(entries) = std::fs::read_dir(platform.path()) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            let Ok(bytes) = std::fs::read(&path) else {
                continue;
            };
            let Ok(definition) = serde_json::from_slice::<GameDefinition>(&bytes) else {
                continue;
            };
            if definition.schema > SCHEMA {
                continue;
            }
            let platform = if definition.game.platform.is_empty() {
                platform_name.clone()
            } else {
                definition.game.platform.clone()
            };
            let title = definition
                .game
                .title
                .clone()
                .unwrap_or_else(|| stem.to_string());
            let mut ids = std::collections::HashSet::new();
            let mut total = 0u32;
            for achievement in &definition.achievements {
                if achievement.engine == ENGINE_RCHEEVOS
                    && !achievement.id.trim().is_empty()
                    && !achievement.condition.trim().is_empty()
                    && ids.insert(achievement.id.clone())
                {
                    total += 1;
                }
            }
            let unlocks = load_unlocks(root, &platform, &definition.game.sha256);
            let mut unlocked = 0u32;
            let mut points = 0u32;
            for achievement in &definition.achievements {
                if achievement.engine != ENGINE_RCHEEVOS || !ids.contains(&achievement.id) {
                    continue;
                }
                if unlocks.unlocks.contains_key(&achievement.id) {
                    unlocked += 1;
                    points += achievement.points;
                }
            }
            games.push(InstalledGame {
                platform,
                stem: stem.to_string(),
                title,
                sha256: definition.game.sha256,
                total,
                unlocked,
                points,
            });
        }
    }
    games.sort_by(|a, b| {
        a.platform
            .cmp(&b.platform)
            .then_with(|| a.title.cmp(&b.title))
    });
    // The sha256 is the real game identity; duplicate definition files for the
    // same dump show up once in the Web picker.
    games.dedup_by(|a, b| {
        a.platform == b.platform
            && !a.sha256.is_empty()
            && a.sha256.eq_ignore_ascii_case(&b.sha256)
    });
    games
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "slot-cheevos-{name}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    fn fixture() -> GameDefinition {
        GameDefinition {
            schema: SCHEMA,
            game: GameInfo {
                platform: "FC".into(),
                sha256: "abc".into(),
                title: Some("魂斗罗".into()),
            },
            achievements: vec![
                AchievementDef {
                    id: "local:look-up".into(),
                    title: "测试：向上看".into(),
                    description: String::new(),
                    points: 1,
                    engine: "rcheevos".into(),
                    condition: "0xH00bc=1".into(),
                },
                AchievementDef {
                    id: "native:ignored".into(),
                    title: "旧规则".into(),
                    description: String::new(),
                    points: 0,
                    engine: "native".into(),
                    condition: String::new(),
                },
            ],
        }
    }

    #[test]
    fn loads_rcheevos_rules_and_skips_other_engines() {
        let root = temp_root("load");
        let path = def_path(&root, "FC", "Contra");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, serde_json::to_vec(&fixture()).unwrap()).unwrap();
        let set = load_for_stem(&root, "FC", "Contra").unwrap().unwrap();
        assert_eq!(set.rules.len(), 1);
        assert_eq!(set.rules[0].numeric, 1);
        assert_eq!(set.rules[0].id, "local:look-up");
        assert_eq!(set.rules[0].condition, "0xH00bc=1");
    }

    #[test]
    fn missing_definition_is_none() {
        let root = temp_root("missing");
        assert!(load_for_stem(&root, "FC", "NotThere").unwrap().is_none());
    }

    #[test]
    fn a_renamed_definition_is_found_by_sha() {
        let root = temp_root("renamed");
        let path = defs_dir(&root, "FC").join("renamed.json");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, serde_json::to_vec(&fixture()).unwrap()).unwrap();
        let set = load_for_game(&root, "FC", "abc", "Contra")
            .unwrap()
            .unwrap();
        assert_eq!(set.rules[0].id, "local:look-up");
    }

    #[test]
    fn a_stem_definition_with_a_different_sha_is_rejected() {
        let root = temp_root("wrong-sha");
        let path = def_path(&root, "FC", "Contra");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, serde_json::to_vec(&fixture()).unwrap()).unwrap();
        assert!(load_for_game(&root, "FC", "def", "Contra")
            .unwrap()
            .is_none());
    }

    #[test]
    fn a_legacy_definition_without_sha_still_uses_the_stem() {
        let root = temp_root("legacy");
        let mut definition = fixture();
        definition.game.sha256.clear();
        let path = def_path(&root, "FC", "Contra");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, serde_json::to_vec(&definition).unwrap()).unwrap();
        assert!(load_for_game(&root, "FC", "def", "Contra")
            .unwrap()
            .is_some());
    }

    #[test]
    fn unlocks_are_keyed_by_sha_and_keep_the_first_time() {
        let root = temp_root("unlock");
        let game = GameInfo {
            platform: "FC".into(),
            sha256: "deadbeef".into(),
            title: None,
        };
        let ids = vec!["local:look-up".to_string()];
        let first = record(&root, &game, &ids, 100).unwrap();
        assert_eq!(first, ids);
        let again = record(&root, &game, &ids, 200).unwrap();
        assert!(again.is_empty());
        let loaded = load_unlocks(&root, "FC", "deadbeef");
        assert_eq!(loaded.unlocks.get("local:look-up"), Some(&100));
        assert_eq!(loaded.updated_at, 100);
    }

    #[test]
    fn a_different_sha_has_separate_progress() {
        let root = temp_root("separate");
        let a = GameInfo {
            platform: "FC".into(),
            sha256: "aaa".into(),
            title: None,
        };
        record(&root, &a, &["x".into()], 1).unwrap();
        assert!(load_unlocks(&root, "FC", "bbb").unlocks.is_empty());
    }

    #[test]
    fn ensure_record_creates_an_empty_cloud_restore_target() {
        let root = temp_root("ensure");
        let game = GameInfo {
            platform: "FC".into(),
            sha256: "abc".into(),
            title: None,
        };
        ensure_record(&root, &game).unwrap();
        let unlocks = load_unlocks(&root, "FC", "abc");
        assert!(unlocks.unlocks.is_empty());
        assert_eq!(unlocks.sha256, "abc");
        assert!(unlock_path(&root, "FC", "abc").is_file());
    }
}
