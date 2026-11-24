//! Global achievement unlocks (`linkSlot/Achievements/unlocks.json`).
//!
//! Phase 1 achievements are global ("played 10 hours", "owns 100 carts"), so
//! they are not tied to a sync unit: one small file on the card and one copy in
//! the cloud root. The merge is a pure union: unlock ids are only ever added,
//! `at` keeps the earliest timestamp and `updated_at` the newest. Every failure
//! is a warning; achievements never change the save-sync result.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::error::{fs_error, LinkError, Result};
use crate::output::Reporter;
use crate::store;
use crate::sync::Ctx;
use crate::webdav::DavClient;

pub const SCHEMA: u32 = 1;
/// 云端文件在同步期间可能被另一台设备改写；重新读取并重新合并的次数。
const MERGE_ATTEMPTS: usize = 3;
pub const LOCAL_REL: &str = "linkSlot/Achievements/unlocks.json";
pub const CLOUD_FILE: &str = "achievements.json";

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Unlocks {
    #[serde(default)]
    pub schema: u32,
    /// Achievement id -> UTC seconds of the earliest unlock.
    #[serde(default)]
    pub unlocks: BTreeMap<String, u64>,
    #[serde(default)]
    pub updated_at: u64,
}

impl Unlocks {
    pub fn parse(bytes: &[u8]) -> Option<Unlocks> {
        serde_json::from_slice(bytes).ok()
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut bytes = serde_json::to_vec_pretty(self)
            .map_err(|err| LinkError::fs(format!("unlocks.json 生成失败：{err}")))?;
        bytes.push(b'\n');
        Ok(bytes)
    }

    pub fn is_empty(&self) -> bool {
        self.unlocks.is_empty() && self.updated_at == 0
    }
}

pub fn local_path(root: &Path) -> PathBuf {
    root.join(LOCAL_REL)
}

/// Read the local file; missing/corrupt/unknown schema reads as empty.
pub fn read_local(root: &Path) -> Unlocks {
    let Ok(bytes) = std::fs::read(local_path(root)) else {
        return Unlocks::default();
    };
    match Unlocks::parse(&bytes) {
        Some(unlocks) if unlocks.schema <= SCHEMA => unlocks,
        _ => Unlocks::default(),
    }
}

pub fn write_local(root: &Path, unlocks: &Unlocks) -> Result<()> {
    let path = local_path(root);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|err| fs_error(format!("无法创建目录 {}", dir.display()), err))?;
    }
    store::atomic_write(&path, &unlocks.to_bytes()?)
}

fn cloud_bytes(unlocks: &Unlocks) -> Result<Vec<u8>> {
    let cloud = Unlocks {
        schema: SCHEMA,
        unlocks: unlocks.unlocks.clone(),
        updated_at: unlocks.updated_at,
    };
    cloud.to_bytes()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Merged {
    pub local: Unlocks,
    pub cloud: Unlocks,
    pub local_changed: bool,
    pub cloud_changed: bool,
}

/// Union merge: ids are only added, `at` keeps the earliest unlock time and
/// `updated_at` the newest value.
pub fn merge(local: &Unlocks, cloud: &Unlocks) -> Merged {
    let mut unlocks = local.unlocks.clone();
    for (id, at) in &cloud.unlocks {
        unlocks
            .entry(id.clone())
            .and_modify(|current| *current = (*current).min(*at))
            .or_insert(*at);
    }
    let updated_at = local.updated_at.max(cloud.updated_at);
    let merged = Unlocks {
        schema: SCHEMA,
        unlocks,
        updated_at,
    };
    // An empty pair must stay a no-op (no zeroed file is ever created).
    let touched = !merged.unlocks.is_empty() || merged.updated_at != 0;
    Merged {
        local: merged.clone(),
        cloud: merged.clone(),
        local_changed: touched
            && (local.unlocks != merged.unlocks
                || local.updated_at != merged.updated_at
                || local.schema != SCHEMA),
        cloud_changed: touched
            && (cloud.unlocks != merged.unlocks
                || cloud.updated_at != merged.updated_at
                || cloud.schema != SCHEMA),
    }
}

/// Best-effort global unlock sync after a save sync. Never returns an error:
/// a failure is only a warning (offline, no config, WebDAV down, ...).
pub fn sync_global(ctx: &Ctx, config: &Config, rep: &mut Reporter) {
    if let Err(error) = try_sync(ctx, config, rep) {
        rep.warn(format!("成就同步失败（不影响存档结果）：{error}"));
    }
}

fn try_sync(ctx: &Ctx, config: &Config, rep: &mut Reporter) -> Result<()> {
    let local = read_local(&ctx.root);
    let client = DavClient::new(config, rep)?;
    let mut cloud = match read_global_cloud(&client, rep)? {
        Some(cloud) => cloud,
        None => return Ok(()),
    };
    if local.is_empty() && cloud.is_empty() {
        return Ok(());
    }

    // Re-read just before the PUT: another device may have written since our
    // first GET, and a blind overwrite would drop its unlocks until the next
    // sync.  Merge again instead; this narrows the lost-update window to the
    // GET/PUT gap.
    let mut merged = merge(&local, &cloud);
    let mut raced = false;
    let mut wrote = false;
    for _ in 0..MERGE_ATTEMPTS {
        if !merged.cloud_changed {
            break;
        }
        let latest = match read_global_cloud(&client, rep)? {
            Some(cloud) => cloud,
            None => return Ok(()),
        };
        if latest == cloud {
            client.put(rep, &[CLOUD_FILE], &cloud_bytes(&merged.cloud)?)?;
            wrote = true;
            break;
        }
        raced = true;
        cloud = latest;
        merged = merge(&local, &cloud);
    }

    if merged.local_changed {
        write_local(&ctx.root, &merged.local)?;
    }
    if merged.cloud_changed && !wrote {
        client.put(rep, &[CLOUD_FILE], &cloud_bytes(&merged.cloud)?)?;
    }
    if raced {
        rep.warn("云端成就文件在同步期间被其他设备修改，已重新合并");
    }
    Ok(())
}

fn read_global_cloud(client: &DavClient, rep: &mut Reporter) -> Result<Option<Unlocks>> {
    match client.get(rep, &[CLOUD_FILE])? {
        None => Ok(Some(Unlocks::default())),
        Some(bytes) => match Unlocks::parse(&bytes) {
            Some(unlocks) if unlocks.schema == SCHEMA => Ok(Some(unlocks)),
            Some(_) => {
                rep.warn("云端 achievements.json 版本不支持，跳过成就同步（不覆盖云端数据）");
                Ok(None)
            }
            None => {
                rep.warn("云端 achievements.json 已损坏，跳过成就同步（不覆盖云端数据）");
                Ok(None)
            }
        },
    }
}

/// Local status for `achievements status [--json]`.
pub fn status(root: &Path) -> Unlocks {
    read_local(root)
}

// ---------------------------------------------------------------------------
// Phase 2: per-game unlocks
// ---------------------------------------------------------------------------

/// Per-game unlock files written by the device:
/// `linkSlot/Achievements/games/<platform>/<sha256>.json`.
pub const GAMES_REL: &str = "linkSlot/Achievements/games";

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameUnlocks {
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

impl GameUnlocks {
    pub fn parse(bytes: &[u8]) -> Option<GameUnlocks> {
        serde_json::from_slice(bytes).ok()
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut bytes = serde_json::to_vec_pretty(self)
            .map_err(|err| LinkError::fs(format!("游戏成就文件生成失败：{err}")))?;
        bytes.push(b'\n');
        Ok(bytes)
    }

    pub fn is_empty(&self) -> bool {
        self.unlocks.is_empty() && self.updated_at == 0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct MergedGame {
    local: GameUnlocks,
    cloud: GameUnlocks,
    local_changed: bool,
    cloud_changed: bool,
}

fn merge_game(local: &GameUnlocks, cloud: &GameUnlocks) -> MergedGame {
    let mut unlocks = local.unlocks.clone();
    for (id, at) in &cloud.unlocks {
        unlocks
            .entry(id.clone())
            .and_modify(|current| *current = (*current).min(*at))
            .or_insert(*at);
    }
    let merged = GameUnlocks {
        schema: SCHEMA,
        platform: local.platform.clone(),
        sha256: local.sha256.clone(),
        unlocks,
        updated_at: local.updated_at.max(cloud.updated_at),
    };
    let touched = !merged.unlocks.is_empty() || merged.updated_at != 0;
    MergedGame {
        local: merged.clone(),
        cloud: merged.clone(),
        local_changed: touched
            && (local.unlocks != merged.unlocks
                || local.updated_at != merged.updated_at
                || local.schema != SCHEMA),
        cloud_changed: touched
            && (cloud.unlocks != merged.unlocks
                || cloud.updated_at != merged.updated_at
                || cloud.schema != SCHEMA),
    }
}

fn game_files(root: &Path) -> Vec<(String, String, PathBuf)> {
    let base = root.join(GAMES_REL);
    let Ok(platforms) = std::fs::read_dir(&base) else {
        return Vec::new();
    };
    let mut files = Vec::new();
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
            let Some(sha) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            files.push((platform_name.clone(), sha.to_string(), path));
        }
    }
    files
}

fn write_game(path: &Path, unlocks: &GameUnlocks) -> Result<()> {
    store::atomic_write(path, &unlocks.to_bytes()?)
}

/// Best-effort sync of every per-game unlock file. Failures on one game are
/// warnings and do not stop the others.
pub fn sync_games(ctx: &Ctx, config: &Config, rep: &mut Reporter) {
    let files = game_files(&ctx.root);
    if files.is_empty() {
        return;
    }
    let client = match DavClient::new(config, rep) {
        Ok(client) => client,
        Err(error) => {
            rep.warn(format!("游戏成就同步失败（不影响存档结果）：{error}"));
            return;
        }
    };
    for (platform, sha, path) in files {
        if let Err(error) = sync_one_game(ctx, &client, &platform, &sha, &path, rep) {
            rep.warn(format!("游戏成就 {platform}/{sha} 同步失败：{error}"));
        }
    }
}

fn sync_one_game(
    ctx: &Ctx,
    client: &DavClient,
    platform: &str,
    sha: &str,
    path: &Path,
    rep: &mut Reporter,
) -> Result<()> {
    let bytes = std::fs::read(path)
        .map_err(|error| LinkError::fs(format!("读取 {} 失败：{error}", path.display())))?;
    let local = match GameUnlocks::parse(&bytes) {
        Some(unlocks) if unlocks.schema <= SCHEMA => unlocks,
        Some(_) => {
            rep.warn(format!("{platform}/{sha} 成就文件版本不支持，跳过"));
            return Ok(());
        }
        None => {
            rep.warn(format!("{platform}/{sha} 成就文件损坏，跳过"));
            return Ok(());
        }
    };
    let rel = [platform, sha, CLOUD_FILE];
    let mut cloud = match read_game_cloud(client, &rel, &local, platform, sha, rep)? {
        Some(cloud) => cloud,
        None => return Ok(()),
    };
    let mut cloud_new = BTreeSet::new();
    collect_cloud_new(&local, &cloud, &mut cloud_new);

    let mut merged = merge_game(&local, &cloud);
    let mut raced = false;
    let mut wrote = false;
    for _ in 0..MERGE_ATTEMPTS {
        if !merged.cloud_changed {
            break;
        }
        let latest = match read_game_cloud(client, &rel, &local, platform, sha, rep)? {
            Some(cloud) => cloud,
            None => return Ok(()),
        };
        if latest == cloud {
            client.ensure_dir(rep, &rel[..2])?;
            client.put(rep, &rel, &merged.cloud.to_bytes()?)?;
            wrote = true;
            break;
        }
        raced = true;
        cloud = latest;
        collect_cloud_new(&local, &cloud, &mut cloud_new);
        merged = merge_game(&local, &cloud);
    }

    if merged.local_changed {
        write_game(path, &merged.local)?;
    }
    if merged.cloud_changed && !wrote {
        client.ensure_dir(rep, &rel[..2])?;
        client.put(rep, &rel, &merged.cloud.to_bytes()?)?;
    }
    if !cloud_new.is_empty() {
        if let Err(error) = record_cloud_merged(&ctx.root, platform, sha, &cloud_new) {
            rep.warn(format!("写入云端成就提示失败（不影响结果）：{error}"));
        }
    }
    if raced {
        rep.warn(format!(
            "云端 {platform}/{sha} 成就在同步期间被其他设备修改，已重新合并"
        ));
    }
    Ok(())
}

/// 云端已有、本机尚无的成就 id：同步完成后由掌机提示「云端成就」。
pub const CLOUD_MERGED_FILE: &str = "linkSlot/Achievements/cloud-merged.json";

fn collect_cloud_new(local: &GameUnlocks, cloud: &GameUnlocks, out: &mut BTreeSet<String>) {
    for id in cloud.unlocks.keys() {
        if !local.unlocks.contains_key(id) {
            out.insert(id.clone());
        }
    }
}

fn record_cloud_merged(
    root: &Path,
    platform: &str,
    sha: &str,
    ids: &BTreeSet<String>,
) -> Result<()> {
    let path = root.join(CLOUD_MERGED_FILE);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| LinkError::fs(format!("创建云端成就提示目录失败：{error}")))?;
    }
    let mut doc: serde_json::Value = std::fs::read(&path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_else(|| serde_json::json!({"schema": 1, "entries": []}));
    doc["schema"] = serde_json::json!(1);
    if !doc.get("entries").map(|value| value.is_array()).unwrap_or(false) {
        doc["entries"] = serde_json::json!([]);
    }
    let entries = doc["entries"].as_array_mut().expect("entries array");
    let matching = entries.iter_mut().find(|entry| {
        entry.get("platform").and_then(|value| value.as_str()) == Some(platform)
            && entry.get("sha256").and_then(|value| value.as_str()) == Some(sha)
    });
    if let Some(entry) = matching {
        if !entry.get("ids").map(|value| value.is_array()).unwrap_or(false) {
            entry["ids"] = serde_json::json!([]);
        }
        let list = entry["ids"].as_array_mut().expect("ids array");
        for id in ids {
            if !list.iter().any(|value| value.as_str() == Some(id)) {
                list.push(serde_json::json!(id));
            }
        }
    } else {
        entries.push(serde_json::json!({
            "platform": platform,
            "sha256": sha,
            "ids": ids.iter().collect::<Vec<_>>(),
        }));
    }
    let bytes = serde_json::to_vec_pretty(&doc)
        .map_err(|error| LinkError::fs(format!("成就提示序列化失败：{error}")))?;
    store::atomic_write(&path, &bytes)
}

fn read_game_cloud(
    client: &DavClient,
    rel: &[&str],
    local: &GameUnlocks,
    platform: &str,
    sha: &str,
    rep: &mut Reporter,
) -> Result<Option<GameUnlocks>> {
    match client.get(rep, rel)? {
        None => Ok(Some(GameUnlocks {
            schema: SCHEMA,
            platform: local.platform.clone(),
            sha256: local.sha256.clone(),
            ..GameUnlocks::default()
        })),
        Some(bytes) => match GameUnlocks::parse(&bytes) {
            Some(unlocks) if unlocks.schema == SCHEMA => Ok(Some(unlocks)),
            Some(_) => {
                rep.warn(format!("云端 {platform}/{sha} 成就版本不支持，跳过（不覆盖）"));
                Ok(None)
            }
            None => {
                rep.warn(format!("云端 {platform}/{sha} 成就已损坏，跳过（不覆盖）"));
                Ok(None)
            }
        },
    }
}

/// Global + per-game unlock sync, called after the save sync.
pub fn sync_all(ctx: &Ctx, config: &Config, rep: &mut Reporter) {
    sync_global(ctx, config, rep);
    sync_games(ctx, config, rep);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unlocks(entries: &[(&str, u64)], updated_at: u64) -> Unlocks {
        Unlocks {
            schema: SCHEMA,
            unlocks: entries
                .iter()
                .map(|(id, at)| ((*id).to_string(), *at))
                .collect(),
            updated_at,
        }
    }

    #[test]
    fn merge_is_a_union_and_keeps_the_earliest_time() {
        let local = unlocks(&[("native:a", 300), ("native:b", 100)], 400);
        let cloud = unlocks(&[("native:b", 200), ("native:c", 50)], 350);
        let merged = merge(&local, &cloud);
        assert_eq!(merged.local.unlocks.get("native:a"), Some(&300));
        assert_eq!(merged.local.unlocks.get("native:b"), Some(&100));
        assert_eq!(merged.local.unlocks.get("native:c"), Some(&50));
        assert_eq!(merged.local.updated_at, 400);
        assert_eq!(merged.cloud, merged.local);
        assert!(merged.local_changed);
        assert!(merged.cloud_changed);
    }

    #[test]
    fn a_second_merge_round_keeps_a_concurrent_write() {
        let game = |entries: &[(&str, u64)], updated_at: u64| GameUnlocks {
            schema: SCHEMA,
            platform: "FC".into(),
            sha256: "a".repeat(64),
            unlocks: entries
                .iter()
                .map(|(id, at)| ((*id).to_string(), *at))
                .collect(),
            updated_at,
        };
        let local = game(&[("ra:1", 100)], 100);
        let first_cloud = game(&[("ra:2", 200)], 200);
        let merged = merge_game(&local, &first_cloud);

        // Another device wrote ra:3 after our GET but before our PUT.
        let later_cloud = game(&[("ra:2", 200), ("ra:3", 300)], 300);
        let merged = merge_game(&merged.local, &later_cloud);
        assert_eq!(merged.local.unlocks.get("ra:1"), Some(&100));
        assert_eq!(merged.local.unlocks.get("ra:2"), Some(&200));
        assert_eq!(merged.local.unlocks.get("ra:3"), Some(&300));
    }

    #[test]
    fn merging_an_empty_pair_is_a_noop() {
        let empty = Unlocks::default();
        let merged = merge(&empty, &empty);
        assert!(!merged.local_changed);
        assert!(!merged.cloud_changed);
        assert!(merged.local.unlocks.is_empty());
    }

    #[test]
    fn an_already_merged_pair_is_not_rewritten() {
        let local = unlocks(&[("native:a", 100)], 100);
        let merged = merge(&local, &local);
        assert!(!merged.local_changed);
        assert!(!merged.cloud_changed);
    }

    fn temp_root() -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "linkslot-achievements-{}",
            crate::util::unique_suffix()
        ));
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn cloud_merged_marker_accumulates_and_deduplicates() {
        let root = temp_root();
        let sha = "a".repeat(64);
        let mut first = BTreeSet::new();
        first.insert("ra:1".to_string());
        record_cloud_merged(&root, "FC", &sha, &first).unwrap();

        let mut second = BTreeSet::new();
        second.insert("ra:1".to_string());
        second.insert("ra:2".to_string());
        record_cloud_merged(&root, "FC", &sha, &second).unwrap();

        let doc: serde_json::Value =
            serde_json::from_slice(&std::fs::read(root.join(CLOUD_MERGED_FILE)).unwrap()).unwrap();
        let ids = doc["entries"][0]["ids"].as_array().unwrap();
        assert_eq!(ids.len(), 2);
        assert!(ids.iter().any(|id| id == "ra:1"));
        assert!(ids.iter().any(|id| id == "ra:2"));
    }

    #[test]
    fn local_files_roundtrip_and_tolerate_corruption() {
        let root = temp_root();
        assert!(read_local(&root).is_empty());
        let value = unlocks(&[("native:a", 100)], 100);
        write_local(&root, &value).unwrap();
        assert_eq!(read_local(&root), value);
        std::fs::write(local_path(&root), b"not json").unwrap();
        assert!(read_local(&root).is_empty());
        let mut future = value.clone();
        future.schema = SCHEMA + 1;
        write_local(&root, &future).unwrap();
        assert!(read_local(&root).is_empty());
    }
}