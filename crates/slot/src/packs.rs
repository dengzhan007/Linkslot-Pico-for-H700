//! LSP 扩展包：校验、安装与卸载。
//!
//! 扩展包是一个 ZIP 容器（文件扩展名建议 `.lsp`），根目录必须有 `pack.json`，
//! 其余条目就是卡片上的目标路径，例如：
//!
//! ```text
//! pack.json
//! System/linkSlot/cores/supafaust/supafaust_libretro.so
//! ```
//!
//! v1 只支持 `kind = "core"`：只允许写入
//! `System/linkSlot/cores/<id>/<id>_libretro.so`，不会碰 `bin/`、`shell/`、
//! `Config/` 或用户数据。安装记录写在 `linkSlot/Config/packs/installed.json`。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::upgrade::parse_zip;

pub(crate) const MANIFEST: &str = "pack.json";
pub(crate) const KINDS: &[&str] = &["core", "achievements"];
pub(crate) const RECORD_DIR: &str = "linkSlot/Config/packs";
pub(crate) const RECORD_FILE: &str = "installed.json";
const STAGE_DIR: &str = "stage";
const MAX_FILES: usize = 512;
const MAX_FILE_BYTES: usize = 32 * 1024 * 1024;
const MAX_PAYLOAD_BYTES: usize = 64 * 1024 * 1024;
const CORE_PREFIX: [&str; 3] = ["System", "linkSlot", "cores"];
const ACHIEVEMENTS_PREFIX: [&str; 2] = ["linkSlot", "Achievements"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PackFile {
    pub path: String,
    pub size: usize,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PackMeta {
    pub schema: u32,
    pub kind: String,
    pub id: String,
    #[serde(default)]
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub min_app: String,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub source: String,
    pub files: Vec<PackFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct InstalledPack {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub version: String,
    pub installed_at: u64,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub source: String,
    /// Minimum linkSlot version from `pack.json`; empty for older records.
    #[serde(default)]
    pub min_app: String,
    pub files: Vec<String>,
}

/// 校验 `pack.json` 并返回解析结果（不读写卡片）。
pub(crate) fn parse_manifest(bytes: &[u8]) -> Result<PackMeta, String> {
    let meta: PackMeta =
        serde_json::from_slice(bytes).map_err(|error| format!("pack.json 格式错误：{error}"))?;
    if meta.schema != 1 {
        return Err("pack.json 的 schema 必须是 1".into());
    }
    if !KINDS.contains(&meta.kind.as_str()) {
        return Err(format!("不支持的扩展包类型：{}", meta.kind));
    }
    match meta.kind.as_str() {
        "core" => {
            if slot_model::core(&meta.id).is_none() {
                return Err(format!("未知的核心 id：{}", meta.id));
            }
        }
        "achievements" => validate_pack_id(&meta.id)?,
        _ => unreachable!("KINDS and the match must stay in sync"),
    }
    if meta.version.trim().is_empty() || meta.version.len() > 32 {
        return Err("version 不能为空且不超过 32 字符".into());
    }
    if !meta
        .version
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '+' | '-'))
    {
        return Err("version 只能包含字母、数字和 . _ + -".into());
    }
    if !meta.min_app.is_empty() {
        let Some(required) = parse_version(&meta.min_app) else {
            return Err(format!("min_app 格式错误：{}", meta.min_app));
        };
        let Some(current) = parse_version(slot_ui::PROJECT_VERSION) else {
            return Err("无法读取当前版本".into());
        };
        if required > current {
            return Err(format!(
                "扩展包要求 linkSlot v{}，当前 v{}",
                meta.min_app,
                slot_ui::PROJECT_VERSION
            ));
        }
    }
    if meta.files.is_empty() {
        return Err("pack.json 没有文件清单".into());
    }
    if meta.files.len() > MAX_FILES {
        return Err(format!("扩展包最多 {MAX_FILES} 个文件"));
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut total = 0usize;
    for file in &meta.files {
        pack_target(&meta.kind, &meta.id, &file.path)?;
        if !seen.insert(file.path.clone()) {
            return Err(format!("pack.json 重复文件：{}", file.path));
        }
        if file.size == 0 || file.size > MAX_FILE_BYTES {
            return Err(format!("文件大小不合法：{}", file.path));
        }
        normalize_sha(&file.sha256)?;
        total = total
            .checked_add(file.size)
            .ok_or_else(|| "扩展包总量越界".to_string())?;
    }
    if total > MAX_PAYLOAD_BYTES {
        return Err(format!(
            "扩展包解压超过 {} MiB",
            MAX_PAYLOAD_BYTES / (1024 * 1024)
        ));
    }
    Ok(meta)
}

/// 安装扩展包；同 id 的旧版本会被替换。
pub(crate) fn install(root: &Path, bytes: &[u8]) -> Result<InstalledPack, String> {
    let entries = parse_zip(bytes)?;
    let mut manifest_bytes = None;
    let mut payload = BTreeMap::<String, Vec<u8>>::new();
    for entry in entries {
        if entry.name == MANIFEST {
            if manifest_bytes.replace(entry.data).is_some() {
                return Err("扩展包包含重复的 pack.json".into());
            }
            continue;
        }
        if entry.name.ends_with('/') {
            continue;
        }
        let name = normalize_segments(&entry.name)?.join("/");
        if payload.insert(name.clone(), entry.data).is_some() {
            return Err(format!("扩展包包含重复文件：{name}"));
        }
    }
    let Some(manifest_bytes) = manifest_bytes else {
        return Err("扩展包缺少 pack.json".into());
    };
    let meta = parse_manifest(&manifest_bytes)?;
    for file in &meta.files {
        let Some(data) = payload.get(&file.path) else {
            return Err(format!("扩展包缺少文件：{}", file.path));
        };
        if data.len() != file.size {
            return Err(format!("文件大小不一致：{}", file.path));
        }
        if !digest_eq(&file.sha256, data) {
            return Err(format!("文件校验失败：{}", file.path));
        }
    }
    for name in payload.keys() {
        if !meta.files.iter().any(|file| &file.path == name) {
            return Err(format!("pack.json 未列出文件：{name}"));
        }
    }

    let stage = record_dir(root).join(STAGE_DIR).join(&meta.id);
    if stage.exists() {
        std::fs::remove_dir_all(&stage).map_err(|error| format!("清理旧暂存失败：{error}"))?;
    }
    std::fs::create_dir_all(&stage).map_err(|error| format!("创建暂存目录失败：{error}"))?;
    for file in &meta.files {
        let name = file
            .path
            .rsplit('/')
            .next()
            .ok_or_else(|| "路径缺少文件名".to_string())?;
        let temporary = stage.join(name);
        std::fs::write(&temporary, &payload[&file.path])
            .map_err(|error| format!("写入暂存文件失败：{error}"))?;
    }

    for file in &meta.files {
        let segments = normalize_segments(&file.path)?;
        let parent = ensure_plain_dirs(root, &segments[..segments.len() - 1])?;
        let target = parent.join(&segments[segments.len() - 1]);
        write_atomic(&target, &payload[&file.path])?;
    }
    let _ = std::fs::remove_dir_all(&stage);

    let mut packs = read_installed(root);
    packs.retain(|pack| pack.id != meta.id);
    let installed = InstalledPack {
        id: meta.id.clone(),
        kind: meta.kind.clone(),
        name: if meta.name.trim().is_empty() {
            meta.id.clone()
        } else {
            meta.name.clone()
        },
        version: meta.version.clone(),
        installed_at: now_secs(),
        license: meta.license.clone(),
        source: meta.source.clone(),
        min_app: meta.min_app.clone(),
        files: meta.files.iter().map(|file| file.path.clone()).collect(),
    };
    packs.push(installed.clone());
    packs.sort_by(|left, right| left.id.cmp(&right.id));
    write_installed(root, &packs)?;
    Ok(installed)
}

/// 卸载扩展包：只删除安装记录里列出的文件。
pub(crate) fn uninstall(root: &Path, id: &str) -> Result<InstalledPack, String> {
    let mut packs = read_installed(root);
    let index = packs
        .iter()
        .position(|pack| pack.id == id)
        .ok_or_else(|| format!("没有安装扩展包：{id}"))?;
    let pack = packs.remove(index);
    for path in &pack.files {
        pack_target(&pack.kind, &pack.id, path)?;
        let target = root.join(path);
        let metadata = match std::fs::symlink_metadata(&target) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(format!("无法读取 {}：{error}", target.display())),
        };
        if metadata.file_type().is_symlink() {
            return Err("不允许通过符号链接删除文件".into());
        }
        std::fs::remove_file(&target)
            .map_err(|error| format!("删除 {} 失败：{error}", target.display()))?;
    }
    if pack.kind == "core" {
        let dir = root
            .join(CORE_PREFIX[0])
            .join(CORE_PREFIX[1])
            .join(CORE_PREFIX[2])
            .join(&pack.id);
        let _ = std::fs::remove_dir(&dir);
    }
    write_installed(root, &packs)?;
    Ok(pack)
}

/// 已安装扩展包列表；记录损坏时当作空列表。
pub(crate) fn installed(root: &Path) -> Vec<InstalledPack> {
    read_installed(root)
}

fn record_dir(root: &Path) -> PathBuf {
    root.join(RECORD_DIR)
}

fn record_path(root: &Path) -> PathBuf {
    record_dir(root).join(RECORD_FILE)
}

fn read_installed(root: &Path) -> Vec<InstalledPack> {
    let Ok(bytes) = std::fs::read(record_path(root)) else {
        return Vec::new();
    };
    serde_json::from_slice(&bytes).unwrap_or_default()
}

fn write_installed(root: &Path, packs: &[InstalledPack]) -> Result<(), String> {
    let dir = record_dir(root);
    std::fs::create_dir_all(&dir).map_err(|error| format!("创建扩展包记录目录失败：{error}"))?;
    let bytes = serde_json::to_vec_pretty(packs)
        .map_err(|error| format!("序列化扩展包记录失败：{error}"))?;
    let temporary = dir.join("installed.json.tmp");
    std::fs::write(&temporary, bytes).map_err(|error| format!("写入扩展包记录失败：{error}"))?;
    std::fs::rename(&temporary, record_path(root))
        .map_err(|error| format!("提交扩展包记录失败：{error}"))
}

fn normalize_segments(raw: &str) -> Result<Vec<String>, String> {
    if raw.is_empty() || raw.starts_with('/') || raw.contains('\\') {
        return Err(format!("非法路径：{raw}"));
    }
    if raw.chars().any(char::is_control) || raw.contains(':') {
        return Err(format!("路径包含非法字符：{raw}"));
    }
    let mut segments = Vec::new();
    for segment in raw.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return Err(format!("路径包含非法目录名：{raw}"));
        }
        segments.push(segment.to_string());
    }
    if segments.len() > 8 {
        return Err("路径层级过深".into());
    }
    Ok(segments)
}

/// 核心扩展包只允许 `<id>_libretro.so` 落在 `System/linkSlot/cores/<id>/`。
fn core_target(id: &str, path: &str) -> Result<(), String> {
    let segments = normalize_segments(path)?;
    if segments.len() != 5
        || segments[0] != CORE_PREFIX[0]
        || segments[1] != CORE_PREFIX[1]
        || segments[2] != CORE_PREFIX[2]
        || segments[3] != id
    {
        return Err(format!(
            "核心包只允许 System/linkSlot/cores/{id}/{id}_libretro.so：{path}"
        ));
    }
    let expected = format!("{id}_libretro.so");
    if segments[4] != expected {
        return Err(format!("文件名必须是 {expected}：{path}"));
    }
    Ok(())
}

fn pack_target(kind: &str, id: &str, path: &str) -> Result<(), String> {
    match kind {
        "core" => core_target(id, path),
        "achievements" => achievements_target(path),
        _ => Err(format!("不支持的扩展包类型：{kind}")),
    }
}

fn achievements_target(path: &str) -> Result<(), String> {
    let segments = normalize_segments(path)?;
    if segments.len() == 5
        && segments[0] == ACHIEVEMENTS_PREFIX[0]
        && segments[1] == ACHIEVEMENTS_PREFIX[1]
        && segments[2] == "defs"
        && valid_platform(&segments[3])
        && segments[4].ends_with(".json")
        && segments[4].len() > 5
    {
        return Ok(());
    }
    if segments.len() == 6
        && segments[0] == ACHIEVEMENTS_PREFIX[0]
        && segments[1] == ACHIEVEMENTS_PREFIX[1]
        && segments[2] == "badges"
        && valid_platform(&segments[3])
        && segments[4].len() == 64
        && segments[4].chars().all(|c| c.is_ascii_hexdigit())
        && segments[5]
            .strip_suffix(".png")
            .is_some_and(|id| !id.is_empty() && id.len() <= 10 && id.chars().all(|c| c.is_ascii_digit()))
    {
        return Ok(());
    }
    Err(format!(
        "成就包只允许 linkSlot/Achievements/defs/<平台>/<名称>.json 或 linkSlot/Achievements/badges/<平台>/<sha256>/<ID>.png：{path}"
    ))
}

fn validate_pack_id(id: &str) -> Result<(), String> {
    if id.is_empty()
        || id.len() > 64
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    {
        return Err(format!("扩展包 id 不合法：{id}"));
    }
    Ok(())
}

fn valid_platform(platform: &str) -> bool {
    !platform.is_empty()
        && platform.len() <= 16
        && platform.chars().all(|c| c.is_ascii_alphanumeric())
}

fn ensure_plain_dirs(root: &Path, segments: &[String]) -> Result<PathBuf, String> {
    let mut current = root.to_path_buf();
    for segment in segments {
        current.push(segment);
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    return Err("不允许通过符号链接写入".into());
                }
                if !metadata.is_dir() {
                    return Err(format!("路径不是目录：{}", current.display()));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                std::fs::create_dir(&current)
                    .map_err(|error| format!("创建目录 {} 失败：{error}", current.display()))?;
            }
            Err(error) => return Err(format!("无法读取 {}：{error}", current.display())),
        }
    }
    Ok(current)
}

fn write_atomic(target: &Path, bytes: &[u8]) -> Result<(), String> {
    let temporary = target.with_extension("lsp-tmp");
    std::fs::write(&temporary, bytes)
        .map_err(|error| format!("写入 {} 失败：{error}", target.display()))?;
    std::fs::rename(&temporary, target)
        .map_err(|error| format!("提交 {} 失败：{error}", target.display()))
}

fn normalize_sha(raw: &str) -> Result<String, String> {
    let digest = raw.trim();
    let digest = digest.strip_prefix("sha256:").unwrap_or(digest);
    if digest.len() != 64 || !digest.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("sha256 格式错误：{raw}"));
    }
    Ok(digest.to_ascii_lowercase())
}

fn digest_eq(expected: &str, data: &[u8]) -> bool {
    let Ok(expected) = normalize_sha(expected) else {
        return false;
    };
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize()) == expected
}

fn parse_version(text: &str) -> Option<Vec<u64>> {
    let parts: Vec<&str> = text.split('.').collect();
    if parts.len() < 2 || parts.len() > 4 {
        return None;
    }
    parts
        .iter()
        .map(|part| {
            if part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
                None
            } else {
                part.parse().ok()
            }
        })
        .collect()
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn zip_store(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut central = Vec::new();
        for (name, data) in files {
            let offset = out.len();
            let crc = crc32fast::hash(data);
            out.extend_from_slice(b"PK\x03\x04");
            out.extend_from_slice(&20u16.to_le_bytes());
            out.extend_from_slice(&0x0800u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0x21u16.to_le_bytes());
            out.extend_from_slice(&crc.to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(data);
            central.extend_from_slice(b"PK\x01\x02");
            central.extend_from_slice(&20u16.to_le_bytes());
            central.extend_from_slice(&20u16.to_le_bytes());
            central.extend_from_slice(&0x0800u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0x21u16.to_le_bytes());
            central.extend_from_slice(&crc.to_le_bytes());
            central.extend_from_slice(&(data.len() as u32).to_le_bytes());
            central.extend_from_slice(&(data.len() as u32).to_le_bytes());
            central.extend_from_slice(&(name.len() as u16).to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u32.to_le_bytes());
            central.extend_from_slice(&(offset as u32).to_le_bytes());
            central.extend_from_slice(name.as_bytes());
        }
        let central_offset = out.len();
        out.extend_from_slice(&central);
        out.extend_from_slice(b"PK\x05\x06");
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&(files.len() as u16).to_le_bytes());
        out.extend_from_slice(&(files.len() as u16).to_le_bytes());
        out.extend_from_slice(&(central.len() as u32).to_le_bytes());
        out.extend_from_slice(&(central_offset as u32).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out
    }

    fn sha_hex(data: &[u8]) -> String {
        format!("{:x}", sha2::Sha256::digest(data))
    }

    fn manifest(meta: &serde_json::Value) -> String {
        serde_json::to_string(meta).unwrap()
    }

    fn core_meta(id: &str, version: &str, data: &[u8]) -> serde_json::Value {
        json!({
            "schema": 1,
            "kind": "core",
            "id": id,
            "name": "测试核心",
            "version": version,
            "files": [{
                "path": format!("System/linkSlot/cores/{id}/{id}_libretro.so"),
                "size": data.len(),
                "sha256": sha_hex(data),
            }],
        })
    }

    fn pack(meta: &serde_json::Value, files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut entries = vec![(MANIFEST, manifest(meta).into_bytes())];
        for (name, data) in files {
            entries.push((*name, (*data).to_vec()));
        }
        let borrowed: Vec<(&str, &[u8])> = entries
            .iter()
            .map(|(name, data)| (*name, data.as_slice()))
            .collect();
        zip_store(&borrowed)
    }

    #[test]
    fn installs_a_core_pack_and_records_it() {
        let root = tempfile::tempdir().unwrap();
        let data = b"fake core".as_slice();
        let meta = core_meta("supafaust", "1.0.0", data);
        let zip = pack(&meta, &[("System/linkSlot/cores/supafaust/supafaust_libretro.so", data)]);
        let installed = install(root.path(), &zip).unwrap();
        assert_eq!(installed.id, "supafaust");
        assert_eq!(installed.version, "1.0.0");
        assert!(root
            .path()
            .join("System/linkSlot/cores/supafaust/supafaust_libretro.so")
            .is_file());
        let listed = installed_packs(root.path());
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, "supafaust");
    }

    fn installed_packs(root: &Path) -> Vec<InstalledPack> {
        installed(root)
    }


    fn zip_deflate(files: &[(&str, &[u8])]) -> Vec<u8> {
        use std::io::Write;

        let mut buffer = std::io::Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut buffer);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            for (name, data) in files {
                writer.start_file(*name, options).unwrap();
                writer.write_all(data).unwrap();
            }
            writer.finish().unwrap();
        }
        buffer.into_inner()
    }

    #[test]
    fn installs_a_deflated_pack_like_the_python_builder() {
        let root = tempfile::tempdir().unwrap();
        let data = b"deflated core".as_slice();
        let meta = core_meta("supafaust", "1.0.0", data);
        let zip = zip_deflate(&[
            (MANIFEST, manifest(&meta).as_bytes()),
            (
                "System/linkSlot/cores/supafaust/supafaust_libretro.so",
                data,
            ),
        ]);
        let installed = install(root.path(), &zip).unwrap();
        assert_eq!(installed.id, "supafaust");
        assert!(root
            .path()
            .join("System/linkSlot/cores/supafaust/supafaust_libretro.so")
            .is_file());
    }
    #[test]
    fn rejects_unknown_kind_and_unknown_core() {
        let root = tempfile::tempdir().unwrap();
        let data = b"x";
        let mut meta = core_meta("supafaust", "1.0.0", data);
        meta["kind"] = json!("theme");
        let zip = pack(&meta, &[("System/linkSlot/cores/supafaust/supafaust_libretro.so", data)]);
        assert!(install(root.path(), &zip).is_err());

        let mut meta = core_meta("supafaust", "1.0.0", data);
        meta["id"] = json!("not-a-core");
        let zip = pack(&meta, &[("System/linkSlot/cores/not-a-core/not-a-core_libretro.so", data)]);
        assert!(install(root.path(), &zip).is_err());
    }

    #[test]
    fn rejects_paths_outside_the_core_directory() {
        let root = tempfile::tempdir().unwrap();
        let data = b"x";
        let mut meta = core_meta("supafaust", "1.0.0", data);
        meta["files"][0]["path"] = json!("System/linkSlot/bin/linkslot");
        let zip = pack(&meta, &[("System/linkSlot/bin/linkslot", data)]);
        assert!(install(root.path(), &zip).is_err());
    }

    #[test]
    fn rejects_hash_mismatch_and_extra_files() {
        let root = tempfile::tempdir().unwrap();
        let data = b"core";
        let mut meta = core_meta("supafaust", "1.0.0", data);
        meta["files"][0]["sha256"] = json!(sha_hex(b"other"));
        let zip = pack(&meta, &[("System/linkSlot/cores/supafaust/supafaust_libretro.so", data)]);
        assert!(install(root.path(), &zip).is_err());

        let meta = core_meta("supafaust", "1.0.0", data);
        let zip = pack(
            &meta,
            &[
                ("System/linkSlot/cores/supafaust/supafaust_libretro.so", data),
                ("System/linkSlot/cores/supafaust/extra.txt", b"x"),
            ],
        );
        assert!(install(root.path(), &zip).is_err());
    }

    #[test]
    fn installs_and_uninstalls_an_achievements_pack() {
        let root = tempfile::tempdir().unwrap();
        let definition = br#"{"schema":1,"game":{"platform":"FC","sha256":"deadbeef","title":"Test"},"achievements":[]}"#;
        let badge = b"fake badge png";
        let sha = sha_hex(b"rom");
        let definition_path = "linkSlot/Achievements/defs/FC/Test.json";
        let badge_path = format!("linkSlot/Achievements/badges/FC/{sha}/4266.png");
        let meta = json!({
            "schema": 1,
            "kind": "achievements",
            "id": "contra-1447",
            "name": "Test achievements",
            "version": "1.0.0",
            "files": [
                {
                    "path": definition_path,
                    "size": definition.len(),
                    "sha256": sha_hex(definition),
                },
                {
                    "path": badge_path,
                    "size": badge.len(),
                    "sha256": sha_hex(badge),
                },
            ],
        });
        let zip = pack(
            &meta,
            &[(definition_path, definition), (&badge_path, badge)],
        );
        let installed_pack = install(root.path(), &zip).unwrap();
        assert_eq!(installed_pack.kind, "achievements");
        assert!(root.path().join(definition_path).is_file());
        assert!(root.path().join(&badge_path).is_file());

        let removed = uninstall(root.path(), "contra-1447").unwrap();
        assert_eq!(removed.kind, "achievements");
        assert!(!root.path().join(definition_path).exists());
        assert!(!root.path().join(&badge_path).exists());
        assert!(installed(root.path()).is_empty());
    }

    #[test]
    fn achievements_packs_cannot_write_outside_their_directories() {
        let root = tempfile::tempdir().unwrap();
        let data = b"x";
        let mut meta = json!({
            "schema": 1,
            "kind": "achievements",
            "id": "contra-1447",
            "name": "Test",
            "version": "1.0.0",
            "files": [{
                "path": "linkSlot/Achievements/games/FC/deadbeef.json",
                "size": data.len(),
                "sha256": sha_hex(data),
            }],
        });
        let zip = pack(
            &meta,
            &[("linkSlot/Achievements/games/FC/deadbeef.json", data)],
        );
        assert!(install(root.path(), &zip).is_err(), "unlock records are not installable");

        meta["files"][0]["path"] = json!("linkSlot/Achievements/defs/FC/Test.json");
        let zip = pack(
            &meta,
            &[("linkSlot/Achievements/defs/FC/Test.json", data)],
        );
        assert!(install(root.path(), &zip).is_ok());
    }

    #[test]
    fn uninstall_removes_only_recorded_files() {
        let root = tempfile::tempdir().unwrap();
        let data = b"core";
        let meta = core_meta("supafaust", "1.0.0", data);
        let zip = pack(&meta, &[("System/linkSlot/cores/supafaust/supafaust_libretro.so", data)]);
        install(root.path(), &zip).unwrap();
        let keep = root.path().join("System/linkSlot/cores/supafaust/keep.txt");
        std::fs::write(&keep, b"user file").unwrap();

        let removed = uninstall(root.path(), "supafaust").unwrap();
        assert_eq!(removed.id, "supafaust");
        assert!(!root
            .path()
            .join("System/linkSlot/cores/supafaust/supafaust_libretro.so")
            .exists());
        assert!(keep.is_file(), "uninstall must not touch unrelated files");
        assert!(installed(root.path()).is_empty());
    }

    #[test]
    fn reinstalling_replaces_the_record() {
        let root = tempfile::tempdir().unwrap();
        let first = b"v1";
        let meta = core_meta("supafaust", "1.0.0", first);
        let zip = pack(&meta, &[("System/linkSlot/cores/supafaust/supafaust_libretro.so", first)]);
        install(root.path(), &zip).unwrap();

        let second = b"version two";
        let meta = core_meta("supafaust", "1.0.1", second);
        let zip = pack(&meta, &[("System/linkSlot/cores/supafaust/supafaust_libretro.so", second)]);
        install(root.path(), &zip).unwrap();

        let listed = installed(root.path());
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].version, "1.0.1");
        assert_eq!(
            std::fs::read(root.path().join("System/linkSlot/cores/supafaust/supafaust_libretro.so"))
                .unwrap(),
            second
        );
    }

    #[test]
    fn rejects_min_app_above_the_running_version() {
        let root = tempfile::tempdir().unwrap();
        let data = b"x";
        let mut meta = core_meta("supafaust", "1.0.0", data);
        meta["min_app"] = json!("99.0.0");
        let zip = pack(&meta, &[("System/linkSlot/cores/supafaust/supafaust_libretro.so", data)]);
        assert!(install(root.path(), &zip).is_err());
    }
}

    /// 本地验证用：先用 `build/pack_lsp.py` 生成 dist/packs/*.lsp，
    /// 再运行 `cargo test -p slot --lib packs:: -- --ignored`。
    #[test]
    #[ignore = "needs dist/packs/*.lsp built by build/pack_lsp.py"]
    fn a_generated_lsp_pack_installs() {
        let root = tempfile::tempdir().unwrap();
        let mut checked = 0;
        for (id, file) in [
            ("supafaust", "linkslot-pack-supafaust-v1.0.0.lsp"),
            ("nestopia", "linkslot-pack-nestopia-v1.0.0.lsp"),
        ] {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../dist/packs")
                .join(file);
            let Ok(bytes) = std::fs::read(&path) else {
                continue;
            };
            let installed = install(root.path(), &bytes).unwrap();
            assert_eq!(installed.id, id);
            assert_eq!(installed.version, "1.0.0");
            assert!(root
                .path()
                .join(format!("System/linkSlot/cores/{id}/{id}_libretro.so"))
                .is_file());
            checked += 1;
        }
        assert!(checked > 0, "build a pack with build/pack_lsp.py first");
    }