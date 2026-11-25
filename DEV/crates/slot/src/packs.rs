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
//! 支持三种 `kind`：
//!
//! - `core`：只允许写入 `System/linkSlot/cores/<id>/<id>_libretro.so`；
//! - `achievements`：只允许写入 `linkSlot/Achievements/defs|badges/`；
//! - `platform`（schema 2）：平台扩展包，写一个核心 `.so` + 该包的 DAT/资产，
//!   并把"已激活哪些平台"记进安装记录。见 [`validate_platform_pack`]。
//!
//! 任何一种都不碰 `bin/`、`shell/`、`Config/` 或用户数据。安装记录写在
//! `linkSlot/Config/packs/installed.json`（裸数组）。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::upgrade::parse_zip_with_limits;

pub(crate) const MANIFEST: &str = "pack.json";
pub(crate) const KINDS: &[&str] = &["core", "achievements", "platform"];
pub(crate) const RECORD_DIR: &str = "linkSlot/Config/packs";
pub(crate) const RECORD_FILE: &str = "installed.json";
const STAGE_DIR: &str = "stage";
const MAX_FILES: usize = 512;
/// 单个文件上限。平台包要放 FBNeo（79.6 MiB 的 aarch64 `.so`），所以给到
/// 96 MiB：刚好够再长一点，又不会让一张 128 MiB 的卡上空间被无声吃掉。
const MAX_FILE_BYTES: usize = 96 * 1024 * 1024;
/// 解压后总量上限。96 MiB 的核心 + 几个 DAT/资产仍在其内；超过这个数说明
/// 要么是 zip 炸弹，要么是用户拿了别的发行版。
const MAX_PAYLOAD_BYTES: usize = 128 * 1024 * 1024;
const CORE_PREFIX: [&str; 3] = ["System", "linkSlot", "cores"];
const ACHIEVEMENTS_PREFIX: [&str; 2] = ["linkSlot", "Achievements"];
/// 平台包的 DAT / 资产目录（`System/linkSlot/extensions/platforms/<pack>/`）。
const PLATFORM_PREFIX: [&str; 4] = ["System", "linkSlot", "extensions", "platforms"];

/// 平台包格式的版本号。app 只认自己实现过的那一版模板，包里的
/// `extension_api` 不匹配就拒绝安装——否则会装上一个"装好了但没 UI"的包。
const PLATFORM_EXTENSION_API: u32 = 1;

/// 平台包里唯一被认识的扫描器模板（`Games/**/*.zip`，一 zip 一个 set）。
const SCANNER_ARCADE_ZIP: &str = "arcade_zip_v1";
/// 平台包里唯一被认识的 UI 模板（街机柜/PCB 架）。
const UI_ARCADE_CABINET: &str = "arcade_cabinet_v1";
/// 平台包里唯一被认识的输入模板（6 键 + COIN）。
const INPUT_ARCADE_6BUTTON: &str = "arcade_6button_v1";
/// 平台包允许的封面/铭牌扩展名。
const ASSET_EXTENSIONS: [&str; 4] = ["png", "jpg", "jpeg", "webp"];
/// 单张资产上限：铭牌/封面是贴图，不该比一张照片大。
const MAX_ASSET_BYTES: usize = 4 * 1024 * 1024;

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
    /// 平台包（`kind = "platform"`）格式版本；其它 kind 忽略。
    #[serde(default)]
    pub extension_api: u32,
    /// 平台包声明的机种。其它 kind 为空。
    #[serde(default)]
    pub platforms: Vec<PlatformDecl>,
    /// 平台包提供的核心。v1.1.0 只允许一条。
    #[serde(default)]
    pub cores: Vec<CoreDecl>,
    pub files: Vec<PackFile>,
}

/// 平台包里的一台机种。
///
/// 这些字段**不是**让包自带代码，而是让包证明"我要的是 app 已经实现过的那套
/// 模板"：每个值都必须等于 app 里硬编码的那一个，否则拒绝安装。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PlatformDecl {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub folder: String,
    #[serde(default)]
    pub rom_extensions: Vec<String>,
    #[serde(default)]
    pub scanner: String,
    #[serde(default)]
    pub ui: String,
    /// 柜子/PCB 的视觉 profile（`arcade` / `neogeo`），必须等于机种 id。
    #[serde(default)]
    pub ui_profile: String,
    #[serde(default)]
    pub input: String,
    #[serde(default)]
    pub core_ids: Vec<String>,
    #[serde(default)]
    pub default_core: String,
    #[serde(default)]
    pub bios: Vec<String>,
}

/// 平台包里的一个核心。`dylib` 是卡内文件名，必须与 core id 对应。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct CoreDecl {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub dylib: String,
    #[serde(default)]
    pub display: String,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub source: String,
    /// 该核心的 BIOS 目录（`BIOS/fbneo`）；包本身不带 BIOS。
    #[serde(default)]
    pub system_dir: String,
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
    /// 平台包激活的机种（规范小写 `arcade`/`neogeo`）。
    ///
    /// 必须**声明在结构体里**而不是靠 serde 忽略：安装核心包时记录会被
    /// 读进来、改一条、再整表写回，结构体没声明的字段会在这一轮里被静默丢掉，
    /// 平台记录就"被核心包冲掉了"（QA NOTE-2）。空表 = 非平台包。
    #[serde(default)]
    pub platforms: Vec<String>,
    /// 平台包提供的核心 id。同样必须声明，理由见上。
    #[serde(default)]
    pub cores: Vec<String>,
    pub files: Vec<String>,
}

/// 校验 `pack.json` 并返回解析结果（不读写卡片）。
pub(crate) fn parse_manifest(bytes: &[u8]) -> Result<PackMeta, String> {
    let meta: PackMeta =
        serde_json::from_slice(bytes).map_err(|error| format!("pack.json 格式错误：{error}"))?;
    if !KINDS.contains(&meta.kind.as_str()) {
        return Err(format!("不支持的扩展包类型：{}", meta.kind));
    }
    // 平台包是新格式（多文件 + 模板声明），所以用 schema 2；旧包仍是 1。
    // 老 app 见到 schema 2 会直接拒装，这正是我们要的向下兼容方向。
    let schema = if meta.kind == "platform" { 2 } else { 1 };
    if meta.schema != schema {
        return Err(format!("{} 包的 schema 必须是 {schema}", meta.kind));
    }
    match meta.kind.as_str() {
        "core" => {
            if slot_model::core(&meta.id).is_none() {
                return Err(format!("未知的核心 id：{}", meta.id));
            }
        }
        "achievements" => validate_pack_id(&meta.id)?,
        "platform" => validate_platform_pack(&meta)?,
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
    let (platforms, cores) = platform_lists(&meta);
    let mut has_core_file = false;
    let mut seen = std::collections::BTreeSet::new();
    let mut total = 0usize;
    for file in &meta.files {
        pack_target(&meta, &platforms, &cores, &file.path)?;
        if meta.kind == "platform" {
            match platform_file(&meta.id, &platforms, &cores, &file.path)? {
                PlatformFile::Core(_) => has_core_file = true,
                // 贴图是给人看的，不该比一张照片大；超了就是拿平台包当网盘。
                PlatformFile::Asset if file.size > MAX_ASSET_BYTES => {
                    return Err(format!(
                        "平台包资产超过 {} MiB：{}",
                        MAX_ASSET_BYTES / (1024 * 1024),
                        file.path
                    ));
                }
                PlatformFile::Asset | PlatformFile::Metadata(_) => {}
            }
        }
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
    if meta.kind == "platform" && !has_core_file {
        return Err("平台包必须带核心：缺少 System/linkSlot/cores/<core>/<core>_libretro.so".into());
    }
    Ok(meta)
}

/// 平台包声明的机种规范小写 id（`arcade`/`neogeo`），以及它提供的核心 id。
///
/// 路径白名单与安装记录都从这里取，保证"包里的目录名"和"记录里的名字"是同一
/// 份规范值；`meta.id` 的大小写不影响判断。
fn platform_lists(meta: &PackMeta) -> (Vec<String>, Vec<String>) {
    let platforms = meta
        .platforms
        .iter()
        .map(|decl| decl.id.trim().to_ascii_lowercase())
        .collect();
    let cores = meta
        .cores
        .iter()
        .map(|decl| decl.id.trim().to_string())
        .collect();
    (platforms, cores)
}

/// 平台包声明校验。
///
/// 平台包不是"带代码的插件"：`scanner` / `ui` / `input` / `ui_profile` 都是
/// app 里硬编码的模板 id，包只能**点名**自己用哪一套，不能定义新的。所以这里的
/// 规则是等号比较而不是"非空即可"——写错的模板名说明这个包是按别的 app 版本做
/// 的，装上只会得到一个点不开的货架，不如现在就拒。
fn validate_platform_pack(meta: &PackMeta) -> Result<(), String> {
    validate_pack_id(&meta.id)?;
    // 平台包必须写 min_app：旧 app 不认识 kind=platform，会把它当成坏包；
    // 更糟的是"装上了但没有 UI"。空 min_app 在旧包里是允许的，这里不行。
    if meta.min_app.trim().is_empty() {
        return Err("平台包必须声明 min_app（低版本 app 不能安装）".into());
    }
    if parse_version(&meta.min_app).is_none() {
        return Err(format!("min_app 格式错误：{}", meta.min_app));
    }
    if meta.extension_api != PLATFORM_EXTENSION_API {
        return Err(format!(
            "需要更新 linkSlot（extension_api={}，本版支持 {PLATFORM_EXTENSION_API}）",
            meta.extension_api
        ));
    }
    if meta.platforms.is_empty() {
        return Err("平台包必须声明至少一个平台".into());
    }
    // v1.1.0 的柜子/服务门只画一颗核心；多核心等 Phase 5 之后再放开。
    if meta.cores.len() != 1 {
        return Err(format!(
            "v{} 的平台包只支持一个核心（当前声明了 {} 个）",
            slot_ui::PROJECT_VERSION,
            meta.cores.len()
        ));
    }

    let core = &meta.cores[0];
    let core_id = core.id.trim();
    if core_id.is_empty() || slot_model::core(core_id).is_none() {
        return Err(format!("未知的核心 id：{core_id}"));
    }
    let expected_dylib = format!("{core_id}_libretro.so");
    if core.dylib.trim() != expected_dylib {
        return Err(format!(
            "核心 {core_id} 的 dylib 必须是 {expected_dylib}：{}",
            core.dylib
        ));
    }
    // 核心是原生代码，许可证必须由打包者写明（FBNeo 是 Non-commercial）。
    if core.license.trim().is_empty() {
        return Err(format!("核心 {core_id} 必须声明 license"));
    }
    let Some(parsed) = slot_store::Core::parse(core_id) else {
        return Err(format!("未知的核心 id：{core_id}"));
    };

    let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut declared: Vec<slot_model::Platform> = Vec::new();
    for decl in &meta.platforms {
        let platform = known_platform(&decl.id)?;
        let canonical = decl.id.trim().to_ascii_lowercase();
        if !seen.insert(canonical.clone()) {
            return Err(format!("平台 {canonical} 在 platforms 里重复声明"));
        }
        // 这条与激活门同源：核心跑不了这台机种，包阶段就该挡住，否则会出现
        // "平台点亮了、选择器里却没有核心"的货架。
        if !parsed.runs(platform) {
            return Err(format!(
                "核心 {core_id} 跑不了 {} 机种",
                platform.dir_name()
            ));
        }
        validate_platform_decl(meta, decl, platform)?;
        declared.push(platform);
    }
    Ok(())
}

/// 单台机种的声明逐项校验；报错时点名字段，打包者才知道改哪一行。
fn validate_platform_decl(
    meta: &PackMeta,
    decl: &PlatformDecl,
    platform: slot_model::Platform,
) -> Result<(), String> {
    let id = platform.dir_name().to_ascii_lowercase();
    let expected_folder = format!("Games/{}", platform.dir_name());
    if decl.folder.trim() != expected_folder {
        return Err(format!(
            "平台 {id} 的 folder 必须是 {expected_folder}：{}",
            decl.folder
        ));
    }
    if decl.rom_extensions.len() != 1 || !decl.rom_extensions[0].trim().eq_ignore_ascii_case("zip") {
        return Err(format!("平台 {id} 的 rom_extensions 必须是 [\"zip\"]"));
    }
    if decl.scanner.trim() != SCANNER_ARCADE_ZIP {
        return Err(format!(
            "平台 {id} 的 scanner 未知（{}），请更新 linkSlot",
            decl.scanner.trim()
        ));
    }
    if decl.ui.trim() != UI_ARCADE_CABINET {
        return Err(format!(
            "平台 {id} 的 ui 未知（{}），请更新 linkSlot",
            decl.ui.trim()
        ));
    }
    if decl.input.trim() != INPUT_ARCADE_6BUTTON {
        return Err(format!(
            "平台 {id} 的 input 未知（{}），请更新 linkSlot",
            decl.input.trim()
        ));
    }
    // ui_profile 决定柜子/PCB 的画法，必须与机种一一对应（arcade 走 4:3 拉伸，
    // neogeo 走 304×224 整数缩放）。
    if decl.ui_profile.trim().to_ascii_lowercase() != id {
        return Err(format!(
            "平台 {id} 的 ui_profile 必须是 {id}：{}",
            decl.ui_profile
        ));
    }
    if decl.core_ids.is_empty() {
        return Err(format!("平台 {id} 的 core_ids 不能为空"));
    }
    for core_id in &decl.core_ids {
        if !meta.cores.iter().any(|core| core.id.trim() == core_id.trim()) {
            return Err(format!(
                "平台 {id} 引用了未声明的核心：{}",
                core_id.trim()
            ));
        }
    }
    if !decl
        .core_ids
        .iter()
        .any(|core_id| core_id.trim() == decl.default_core.trim())
    {
        return Err(format!(
            "平台 {id} 的 default_core 必须在 core_ids 里：{}",
            decl.default_core.trim()
        ));
    }
    for bios in &decl.bios {
        if bios.trim() != "neogeo.zip" {
            return Err(format!("平台 {id} 不接受 BIOS 文件：{}", bios.trim()));
        }
    }
    // CPS 板子不需要 BIOS；只有 NeoGeo 允许声明 neogeo.zip。
    if platform != slot_model::Platform::Neogeo && !decl.bios.is_empty() {
        return Err(format!("平台 {id} 不允许声明 BIOS"));
    }
    Ok(())
}

/// 平台包只允许激活 app 里预置的两台机种。
fn known_platform(raw: &str) -> Result<slot_model::Platform, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "arcade" => Ok(slot_model::Platform::Arcade),
        "neogeo" => Ok(slot_model::Platform::Neogeo),
        _ => Err(format!("未知平台 {}，请更新 linkSlot", raw.trim())),
    }
}

/// 平台包允许写入的三类文件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlatformFile<'a> {
    /// 核心动态库；参数是核心 id。
    Core(&'a str),
    /// set → title 的 DAT；参数是平台 id。
    Metadata(&'a str),
    /// 封面 / 铭牌等静态贴图。
    Asset,
}

/// 平台包的路径白名单。
///
/// 三类，别的一律拒绝（`bin/`、`Config/`、别人的 pack 目录、别的核心目录）：
///
/// - `System/linkSlot/cores/<core>/<core>_libretro.so`
/// - `System/linkSlot/extensions/platforms/<pack_id>/metadata/<platform>.dat.json`
/// - `System/linkSlot/extensions/platforms/<pack_id>/assets/<1..4 段图片>`
fn platform_file<'a>(
    pack_id: &str,
    platforms: &'a [String],
    cores: &'a [String],
    path: &str,
) -> Result<PlatformFile<'a>, String> {
    let segments = normalize_segments(path)?;
    if segments.len() == 5 && segments[..3] == CORE_PREFIX {
        let core = &segments[3];
        let Some(id) = cores.iter().find(|declared| declared.eq_ignore_ascii_case(core)) else {
            return Err(format!("平台包不能写入未声明的核心目录：{path}"));
        };
        let expected = format!("{id}_libretro.so");
        if segments[4] != expected {
            return Err(format!("核心文件名必须是 {expected}：{path}"));
        }
        return Ok(PlatformFile::Core(id));
    }
    if segments.len() >= 6 && segments[..4] == PLATFORM_PREFIX {
        if segments[4] != pack_id {
            return Err(format!(
                "平台包只允许写入自己的目录 {}/{pack_id}/：{path}",
                PLATFORM_PREFIX.join("/")
            ));
        }
        if segments[5] == "metadata" && segments.len() == 7 {
            let file = &segments[6];
            let Some(name) = file.strip_suffix(".dat.json") else {
                return Err(format!("DAT 文件名必须是 <platform>.dat.json：{path}"));
            };
            let name = name.to_ascii_lowercase();
            let Some(id) = platforms.iter().find(|declared| **declared == name) else {
                return Err(format!("平台包不能写入未声明的平台 DAT：{path}"));
            };
            return Ok(PlatformFile::Metadata(id));
        }
        if segments[5] == "assets" && (7..=10).contains(&segments.len()) {
            let file = &segments[segments.len() - 1];
            let extension = file
                .rsplit_once('.')
                .map(|(_, extension)| extension.to_ascii_lowercase())
                .unwrap_or_default();
            if !ASSET_EXTENSIONS.contains(&extension.as_str()) {
                return Err(format!(
                    "平台包 assets 只接受 {}：{path}",
                    ASSET_EXTENSIONS.join("/")
                ));
            }
            return Ok(PlatformFile::Asset);
        }
    }
    Err(format!(
        "平台包只允许核心库、metadata/<platform>.dat.json 与 assets/ 下的图片：{path}"
    ))
}

fn platform_target(
    pack_id: &str,
    platforms: &[String],
    cores: &[String],
    path: &str,
) -> Result<(), String> {
    platform_file(pack_id, platforms, cores, path).map(|_| ())
}

/// 安装扩展包；同 id 的旧版本会被替换。
pub(crate) fn install(root: &Path, bytes: &[u8]) -> Result<InstalledPack, String> {
    // 扩展包的单文件上限是 [`MAX_FILE_BYTES`]（96 MiB，FBNeo 的 `.so` 是
    // 79.6 MiB），比升级包那条路径的 64 MiB 宽。ZIP 层如果还用自己那份旧上限，
    // 真实的平台包会在解压阶段就被拒，`parse_manifest` 的 96 MiB 门永远走不到。
    let entries = parse_zip_with_limits(bytes, MAX_FILE_BYTES, MAX_PAYLOAD_BYTES)?;
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
    let (platforms, cores) = platform_lists(&meta);
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
    // "一个平台只能属于一个包"必须在**落盘之前**判断：先写文件再拒绝会在卡上
    // 留一堆没有记录归属的 .so 和 DAT，卸载时再也找不到它们。位置放在完整性
    // 校验（大小/sha）之后：坏包应该先报"校验失败"，而不是先撞上占位冲突。
    if meta.kind == "platform" {
        ensure_platforms_are_free(root, &meta.id, &platforms)?;
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
    // 暂存目录的**父**目录也要收掉：每次都留一个空的 `packs/stage/` 会让用户
    // 以为装坏了东西。`remove_dir` 只删空目录，别的包正在暂存时自然失败。
    let _ = std::fs::remove_dir(record_dir(root).join(STAGE_DIR));

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
        platforms,
        cores,
        files: meta.files.iter().map(|file| file.path.clone()).collect(),
    };
    packs.push(installed.clone());
    packs.sort_by(|left, right| left.id.cmp(&right.id));
    write_installed(root, &packs)?;
    Ok(installed)
}

/// 一个平台只能属于一个包。
///
/// 两个包同时点亮 `arcade` 会让"卸载一个"变成"删掉另一个的核心"：两份记录都
/// 指向同一个 `.so`，谁先卸载谁把对方的文件删了。所以第二个包直接拒装，提示
/// 用户先卸载旧的。
fn ensure_platforms_are_free(root: &Path, id: &str, platforms: &[String]) -> Result<(), String> {
    for other in read_installed(root) {
        if other.id == id || other.kind != "platform" {
            continue;
        }
        if let Some(shared) = other
            .platforms
            .iter()
            .find(|held| platforms.iter().any(|wanted| wanted.eq_ignore_ascii_case(held)))
        {
            return Err(format!(
                "平台 {shared} 已由扩展包 {} 提供，请先卸载它",
                other.id
            ));
        }
    }
    Ok(())
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
        installed_target(&pack, path)?;
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
    prune_empty_dirs(root, &pack);
    write_installed(root, &packs)?;
    Ok(pack)
}

/// 清掉卸载后留下的空目录。
///
/// 只删**空**目录（`remove_dir` 对非空目录直接失败，我们忽略失败），所以用户
/// 自己往 `assets/` 里放的文件、或者另一个包共用的父目录都安然无恙。绝不靠近
/// `Games/`、`BIOS/`、`Saves/`、`States/`、`Cheats/` 或 `selected_core.txt`。
fn prune_empty_dirs(root: &Path, pack: &InstalledPack) {
    if pack.kind == "platform" {
        let base = root.join(PLATFORM_PREFIX[0]).join(PLATFORM_PREFIX[1]);
        let base = base.join(PLATFORM_PREFIX[2]).join(PLATFORM_PREFIX[3]);
        let own = base.join(&pack.id);
        // 资产可以有多层子目录，从最深处开始删。
        prune_empty_tree(&own.join("assets"));
        let _ = std::fs::remove_dir(own.join("metadata"));
        let _ = std::fs::remove_dir(&own);
        let _ = std::fs::remove_dir(&base);
        for core in &pack.cores {
            let _ = std::fs::remove_dir(root.join(CORE_PREFIX[0]).join(CORE_PREFIX[1]).join(CORE_PREFIX[2]).join(core));
        }
        return;
    }
    if pack.kind == "core" {
        let dir = root
            .join(CORE_PREFIX[0])
            .join(CORE_PREFIX[1])
            .join(CORE_PREFIX[2])
            .join(&pack.id);
        let _ = std::fs::remove_dir(&dir);
    }
}

/// 自底向上删空目录；非空目录保持原样。
fn prune_empty_tree(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            prune_empty_tree(&path);
        }
    }
    let _ = std::fs::remove_dir(dir);
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

/// 读安装记录。
///
/// 文件不存在 → 空表（首次安装）。文件在但**读不懂** → 先把它挪成
/// `installed.json.bad`，再返回空表：记录是整表覆盖写的，留着一个解析不了的
/// 文件在位置上，下一次装任何包都会把里面的平台/核心包记录永久冲掉；而用户
/// 什么都做错。旧证据不删，改名时加时间戳避免第二次损坏覆盖第一次。
fn read_installed(root: &Path) -> Vec<InstalledPack> {
    let path = record_path(root);
    let Ok(bytes) = std::fs::read(&path) else {
        return Vec::new();
    };
    match serde_json::from_slice(&bytes) {
        Ok(packs) => packs,
        Err(error) => {
            eprintln!(
                "packs: {} 解析失败（{error}），已备份为 .bad 后按空记录继续",
                path.display()
            );
            preserve_bad_record(&path);
            Vec::new()
        }
    }
}

/// 把损坏的记录文件改名留存；改名失败也继续（宁可丢证据，也不能卡住安装）。
fn preserve_bad_record(path: &Path) {
    // 先试最朴素的 `installed.json.bad`：它是最容易被用户找到、发给我们的名字。
    let primary = path.with_extension("json.bad");
    if !primary.exists() && std::fs::rename(path, &primary).is_ok() {
        return;
    }
    // 已经有旧证据了（或改不过去）：加时间戳，必要时再退让序号，绝不覆盖。
    let stamp = now_secs();
    for attempt in 0..100u32 {
        let candidate = if attempt == 0 {
            path.with_extension(format!("json.bad.{stamp}"))
        } else {
            path.with_extension(format!("json.bad.{stamp}.{attempt}"))
        };
        if candidate.exists() {
            continue;
        }
        let _ = std::fs::rename(path, &candidate);
        return;
    }
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

/// 按 kind 分派的路径白名单（安装 / 解包阶段）。
///
/// `platforms` / `cores` 是平台包的规范声明，其它 kind 传空切片即可。
fn pack_target(
    meta: &PackMeta,
    platforms: &[String],
    cores: &[String],
    path: &str,
) -> Result<(), String> {
    match meta.kind.as_str() {
        "core" => core_target(&meta.id, path),
        "achievements" => achievements_target(path),
        "platform" => platform_target(&meta.id, platforms, cores, path),
        _ => Err(format!("不支持的扩展包类型：{}", meta.kind)),
    }
}

/// 按 kind 分派的路径白名单（卸载阶段，记录里只有 id / platforms / cores）。
fn installed_target(pack: &InstalledPack, path: &str) -> Result<(), String> {
    match pack.kind.as_str() {
        "core" => core_target(&pack.id, path),
        "achievements" => achievements_target(path),
        "platform" => platform_target(&pack.id, &pack.platforms, &pack.cores, path),
        _ => Err(format!("不支持的扩展包类型：{}", pack.kind)),
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
pub(crate) mod tests {
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

    // ----- 平台包夹具 -----------------------------------------------------
    // 平台包只有一个核心（FBNeo）+ 两份 DAT，和 build/pack_lsp.py 打出来的
    // 形状一致；下面每个用例都从这里出发，只改一处再断言。

    const PACK: &str = "arcade-neogeo";
    const SO: &str = "System/linkSlot/cores/fbneo/fbneo_libretro.so";
    const DAT: &str = r#"{"schema":1,"sets":[]}"#;

    fn dat(platform: &str) -> String {
        dat_for(PACK, platform)
    }

    fn dat_for(pack_id: &str, platform: &str) -> String {
        format!("System/linkSlot/extensions/platforms/{pack_id}/metadata/{platform}.dat.json")
    }

    fn asset(name: &str) -> String {
        format!("System/linkSlot/extensions/platforms/{PACK}/assets/{name}")
    }

    fn fbneo_decl() -> serde_json::Value {
        json!({
            "id": "fbneo",
            "dylib": "fbneo_libretro.so",
            "display": "FBNeo",
            "license": "Non-commercial",
            "source": "https://github.com/libretro/FBNeo/tree/3f0de7fefbb7cc4b46e832647215cbe41eaf37b0",
            "system_dir": "BIOS/fbneo",
        })
    }

    /// 一台机种的合法声明；`arcade` 不需要 BIOS，`neogeo` 可以声明 neogeo.zip。
    fn platform_decl(id: &str) -> serde_json::Value {
        let (dir, profile, bios) = if id == "neogeo" {
            ("NeoGeo", "neogeo", vec!["neogeo.zip"])
        } else {
            ("Arcade", "arcade", Vec::new())
        };
        json!({
            "id": id,
            "name": id,
            "folder": format!("Games/{dir}"),
            "rom_extensions": ["zip"],
            "scanner": SCANNER_ARCADE_ZIP,
            "ui": UI_ARCADE_CABINET,
            "ui_profile": profile,
            "input": INPUT_ARCADE_6BUTTON,
            "core_ids": ["fbneo"],
            "default_core": "fbneo",
            "bios": bios,
        })
    }

    /// 一份合法的最小平台包：核心 `.so` + 两个平台的 DAT。
    fn platform_files(core: &[u8]) -> Vec<(String, Vec<u8>)> {
        platform_files_for(PACK, core)
    }

    fn platform_files_for(pack_id: &str, core: &[u8]) -> Vec<(String, Vec<u8>)> {
        vec![
            (SO.to_string(), core.to_vec()),
            (dat_for(pack_id, "arcade"), DAT.as_bytes().to_vec()),
            (dat_for(pack_id, "neogeo"), DAT.as_bytes().to_vec()),
        ]
    }

    fn meta_for(
        id: &str,
        version: &str,
        platforms: Vec<serde_json::Value>,
        cores: Vec<serde_json::Value>,
        files: &[(String, Vec<u8>)],
    ) -> serde_json::Value {
        json!({
            "schema": 2,
            "kind": "platform",
            "id": id,
            "name": "街机 & NeoGeo · FBNeo",
            "version": version,
            "min_app": "1.1.0",
            "extension_api": PLATFORM_EXTENSION_API,
            "platforms": platforms,
            "cores": cores,
            "files": files
                .iter()
                .map(|(path, data)| json!({
                    "path": path,
                    "size": data.len(),
                    "sha256": sha_hex(data),
                }))
                .collect::<Vec<_>>(),
        })
    }

    fn platform_meta(files: &[(String, Vec<u8>)]) -> serde_json::Value {
        meta_for(
            PACK,
            "1.0.0",
            vec![platform_decl("arcade"), platform_decl("neogeo")],
            vec![fbneo_decl()],
            files,
        )
    }

    fn build(meta: &serde_json::Value, files: &[(String, Vec<u8>)]) -> Vec<u8> {
        let borrowed: Vec<(&str, &[u8])> = files
            .iter()
            .map(|(path, data)| (path.as_str(), data.as_slice()))
            .collect();
        pack(meta, &borrowed)
    }

    pub(crate) fn platform_pack(core: &[u8]) -> Vec<u8> {
        let files = platform_files(core);
        build(&platform_meta(&files), &files)
    }

    /// 被拒的包不许在卡上留下任何痕迹（连目录都不该出现）。
    fn assert_nothing_written(root: &Path) {
        assert!(
            installed(root).is_empty(),
            "被拒绝的包写了安装记录：{:?}",
            installed(root)
        );
        for dir in ["System", "linkSlot"] {
            assert!(
                !root.join(dir).exists(),
                "被拒绝的包创建了 {dir}/，卸载时再也找不到这些文件"
            );
        }
    }

    fn installs_platform_pack(root: &Path) {
        let bytes = platform_pack(b"\x7fELF fake fbneo");
        install(root, &bytes).expect("a minimal platform pack installs");
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

    /// QA-1 回归：96 MiB 的单文件门必须真的**可到达**，而升级包的 64 MiB 门不变。
    ///
    /// 旧实现里 `install` 先走 `upgrade::parse_zip`，那一层还留着升级包的
    /// 64 MiB 单条目上限，于是 79.6 MiB 的真 FBNeo 会在解压阶段被拒，
    /// `parse_manifest` 的 96 MiB 门永远看不到。这里用 65 MiB（> 64 MiB 的升级包
    /// 上限、< 96 MiB 的扩展包上限）走 deflate 路径：压缩后包很小，只有解压后
    /// 才越过那条老上限。
    #[test]
    fn a_pack_entry_above_the_upgrade_cap_installs() {
        const MIB: usize = 1024 * 1024;
        let root = tempfile::tempdir().unwrap();
        let core = vec![0u8; 65 * MIB];
        let path = "System/linkSlot/cores/supafaust/supafaust_libretro.so";
        let meta = json!({
            "schema": 1,
            "kind": "core",
            "id": "supafaust",
            "name": "big core",
            "version": "1.0.0",
            "files": [{
                "path": path,
                "size": core.len(),
                "sha256": sha_hex(&core),
            }],
        });
        let zip = zip_deflate(&[(MANIFEST, manifest(&meta).as_bytes()), (path, &core)]);
        assert!(
            zip.len() < 64 * MIB,
            "夹具本身要能被请求体层放行（压缩后应远小于 64 MiB）"
        );

        // 扩展包入口：65 MiB 单文件在 96 MiB 门之内，必须装得进去。
        install(root.path(), &zip).expect("96 MiB 门之内的核心必须装得进去");
        assert_eq!(
            std::fs::metadata(root.path().join(path)).unwrap().len(),
            core.len() as u64
        );

        // 升级包入口：同一个 ZIP 仍然被 64 MiB 挡住——证明放宽只发生在
        // 扩展包这一条路径，升级包的语义与峰值内存没被动过。
        let error = crate::upgrade::parse_zip(&zip).expect_err("升级包入口不该被放宽");
        assert!(
            error.contains("64 MiB"),
            "升级包入口应当仍然报 64 MiB：{error}"
        );
    }

    /// 记录损坏时先备份，再按空表继续。
    ///
    /// 安装是"整表覆盖写"：如果坏文件原地留着，下一次装任何包都会把里面的
    /// 平台/核心包记录永久冲掉，而用户什么都没做错。所以损坏时必须留有证据。
    #[test]
    fn a_corrupt_record_is_preserved_before_any_install() {
        let root = tempfile::tempdir().unwrap();
        let record_dir = root.path().join(RECORD_DIR);
        std::fs::create_dir_all(&record_dir).unwrap();
        let broken = b"{ this is not json";
        std::fs::write(record_dir.join(RECORD_FILE), broken).unwrap();

        let data = b"fake supafaust".as_slice();
        let meta = core_meta("supafaust", "1.0.0", data);
        let zip = pack(
            &meta,
            &[("System/linkSlot/cores/supafaust/supafaust_libretro.so", data)],
        );
        install(root.path(), &zip).expect("坏记录不该挡住安装");

        // 原字节一字不差地留在 .bad 里。
        let backups: Vec<std::path::PathBuf> = std::fs::read_dir(&record_dir)
            .unwrap()
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("installed.json.bad"))
            })
            .collect();
        assert_eq!(backups.len(), 1, "没有留下损坏记录的备份：{backups:?}");
        assert_eq!(std::fs::read(&backups[0]).unwrap(), broken);

        // 新记录只有刚装的那一条，且是合法 JSON。
        let listed = installed(root.path());
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, "supafaust");
    }

    #[test]
    fn a_second_corrupt_record_does_not_overwrite_the_first_evidence() {
        let root = tempfile::tempdir().unwrap();
        let record_dir = root.path().join(RECORD_DIR);
        std::fs::create_dir_all(&record_dir).unwrap();
        let record = record_dir.join(RECORD_FILE);

        for broken in [b"{ first".as_slice(), b"{ second".as_slice()] {
            std::fs::write(&record, broken).unwrap();
            assert!(installed(root.path()).is_empty());
        }
        let backups: Vec<std::path::PathBuf> = std::fs::read_dir(&record_dir)
            .unwrap()
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("installed.json.bad"))
            })
            .collect();
        assert_eq!(backups.len(), 2, "第二次损坏覆盖了第一次的证据：{backups:?}");
    }

    // ----- 平台包：安装 / 记录 / 卸载 ------------------------------------

    #[test]
    fn installs_a_platform_pack_and_records_both_platforms() {
        let root = tempfile::tempdir().unwrap();
        let installed_pack = install(root.path(), &platform_pack(b"\x7fELF fake fbneo")).unwrap();

        assert_eq!(installed_pack.kind, "platform");
        assert_eq!(installed_pack.id, PACK);
        assert_eq!(installed_pack.platforms, vec!["arcade", "neogeo"]);
        assert_eq!(installed_pack.cores, vec!["fbneo"]);
        for path in [SO, &dat("arcade"), &dat("neogeo")] {
            assert!(root.path().join(path).is_file(), "{path} 没落地");
        }

        // 记录里存的是规范小写，激活门读的就是它。
        let listed = installed(root.path());
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].platforms, vec!["arcade", "neogeo"]);
        assert_eq!(listed[0].cores, vec!["fbneo"]);
        assert!(slot_store::platform_packs::active(
            root.path(),
            slot_store::Platform::Arcade
        ));
        assert!(slot_store::platform_packs::active(
            root.path(),
            slot_store::Platform::Neogeo
        ));
        let available = slot_store::platform_packs::available_platforms(root.path());
        assert!(available.contains(&slot_store::Platform::Arcade));
        assert!(available.contains(&slot_store::Platform::Neogeo));
    }

    #[test]
    fn uninstall_removes_only_pack_files_and_keeps_roms_bios_and_saves() {
        let root = tempfile::tempdir().unwrap();
        installs_platform_pack(root.path());
        // 用户数据：ROM、NeoGeo BIOS、存档。
        for (path, data) in [
            ("Games/Arcade/a.zip", b"PK\x03\x04 rom".as_slice()),
            ("Games/NeoGeo/b.zip", b"PK\x03\x04 rom".as_slice()),
            ("BIOS/fbneo/neogeo.zip", b"bios".as_slice()),
            ("Saves/Arcade/fbneo/a.sav", b"save".as_slice()),
        ] {
            let target = root.path().join(path);
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            std::fs::write(&target, data).unwrap();
        }

        uninstall(root.path(), PACK).unwrap();

        for path in [SO, &dat("arcade"), &dat("neogeo")] {
            assert!(!root.path().join(path).exists(), "{path} 没被删掉");
        }
        // 空目录要清掉，否则重装会被"目录已存在"的假象干扰。
        let base = root
            .path()
            .join("System/linkSlot/extensions/platforms")
            .join(PACK);
        assert!(!base.exists(), "扩展包目录没清理：{}", base.display());
        assert!(!root.path().join("System/linkSlot/cores/fbneo").exists());

        for path in [
            "Games/Arcade/a.zip",
            "Games/NeoGeo/b.zip",
            "BIOS/fbneo/neogeo.zip",
            "Saves/Arcade/fbneo/a.sav",
        ] {
            assert!(
                root.path().join(path).is_file(),
                "卸载删了用户数据：{path}"
            );
        }
        assert!(installed(root.path()).is_empty());
        assert!(!slot_store::platform_packs::active(
            root.path(),
            slot_store::Platform::Arcade
        ));
    }

    #[test]
    fn reinstalling_a_platform_pack_updates_the_record() {
        let root = tempfile::tempdir().unwrap();
        install(root.path(), &platform_pack(b"v1 core bytes")).unwrap();

        let files = platform_files(b"v2 core bytes, longer");
        let meta = meta_for(
            PACK,
            "1.0.1",
            vec![platform_decl("arcade"), platform_decl("neogeo")],
            vec![fbneo_decl()],
            &files,
        );
        let pack = build(&meta, &files);
        install(root.path(), &pack).unwrap();

        let listed = installed(root.path());
        assert_eq!(listed.len(), 1, "重装不该留下第二条记录");
        assert_eq!(listed[0].version, "1.0.1");
        assert_eq!(listed[0].platforms, vec!["arcade", "neogeo"]);
        assert_eq!(
            std::fs::read(root.path().join(SO)).unwrap(),
            b"v2 core bytes, longer"
        );
        assert!(slot_store::platform_packs::active(
            root.path(),
            slot_store::Platform::Arcade
        ));
    }

    /// QA NOTE-2 回归：装一个**核心包**不能把平台记录冲掉。
    ///
    /// 安装流程是"读记录 → 改一条 → 整表写回"。如果 `InstalledPack` 没把
    /// `platforms`/`cores` 声明成字段，平台记录就会在这一轮序列化里被静默丢掉，
    /// 街机从货架上消失，而且用户什么都没做错。
    #[test]
    fn a_core_pack_install_keeps_the_platform_record() {
        let root = tempfile::tempdir().unwrap();
        installs_platform_pack(root.path());

        let data = b"fake supafaust".as_slice();
        let meta = core_meta("supafaust", "1.0.0", data);
        let zip = pack(
            &meta,
            &[("System/linkSlot/cores/supafaust/supafaust_libretro.so", data)],
        );
        install(root.path(), &zip).unwrap();

        let listed = installed(root.path());
        assert_eq!(listed.len(), 2, "核心包把平台记录挤掉了：{listed:?}");
        let platform = listed
            .iter()
            .find(|pack| pack.id == PACK)
            .expect("平台记录不见了");
        assert_eq!(platform.platforms, vec!["arcade", "neogeo"]);
        assert_eq!(platform.cores, vec!["fbneo"]);
        // 盘上的 JSON 也必须还有这两个字段（结构体字段漏一个就会丢）。
        let raw = std::fs::read_to_string(root.path().join(RECORD_DIR).join(RECORD_FILE)).unwrap();
        assert!(raw.contains("\"platforms\""), "记录里没有 platforms：{raw}");
        assert!(raw.contains("\"cores\""), "记录里没有 cores：{raw}");
        assert!(slot_store::platform_packs::active(
            root.path(),
            slot_store::Platform::Arcade
        ));

        // 卸掉核心包，平台仍然活着。
        uninstall(root.path(), "supafaust").unwrap();
        assert_eq!(installed(root.path()).len(), 1);
        assert!(slot_store::platform_packs::active(
            root.path(),
            slot_store::Platform::Arcade
        ));
        assert!(slot_store::platform_packs::active(
            root.path(),
            slot_store::Platform::Neogeo
        ));
    }

    // ----- 平台包：拒绝矩阵 ----------------------------------------------

    #[test]
    fn rejects_unknown_or_duplicate_platforms() {
        let root = tempfile::tempdir().unwrap();
        let files = platform_files(b"core");

        let mut meta = meta_for(
            PACK,
            "1.0.0",
            vec![platform_decl("arcade"), platform_decl("snes")],
            vec![fbneo_decl()],
            &files,
        );
        meta["platforms"][1]["id"] = json!("snes");
        let zip = build(&meta, &files);
        let error = install(root.path(), &zip).unwrap_err();
        assert!(error.contains("未知平台"), "{error}");
        assert_nothing_written(root.path());

        let duplicate = meta_for(
            PACK,
            "1.0.0",
            vec![platform_decl("arcade"), platform_decl("arcade")],
            vec![fbneo_decl()],
            &files,
        );
        let zip = build(&duplicate, &files);
        let error = install(root.path(), &zip).unwrap_err();
        assert!(error.contains("重复"), "{error}");
        assert_nothing_written(root.path());
    }

    #[test]
    fn rejects_unknown_ui_scanner_input_and_profile_templates() {
        let files = platform_files(b"core");
        for (field, value, expected) in [
            ("scanner", json!("arcade_zip_v9"), "scanner"),
            ("ui", json!("arcade_cabinet_v9"), "ui"),
            ("input", json!("arcade_4button_v1"), "input"),
            ("ui_profile", json!("neogeo"), "ui_profile"),
        ] {
            let root = tempfile::tempdir().unwrap();
            let mut meta = platform_meta(&files);
            meta["platforms"][0][field] = value;
            let zip = build(&meta, &files);
            let error = install(root.path(), &zip).unwrap_err();
            assert!(error.contains(expected), "{field}: {error}");
            assert_nothing_written(root.path());
        }
    }

    #[test]
    fn rejects_a_platform_pack_meant_for_another_app_version() {
        let files = platform_files(b"core");

        // extension_api 不认识：将来加了新模板，旧 app 不能装。
        let root = tempfile::tempdir().unwrap();
        let mut meta = platform_meta(&files);
        meta["extension_api"] = json!(2);
        let zip = build(&meta, &files);
        let error = install(root.path(), &zip).unwrap_err();
        assert!(error.contains("extension_api"), "{error}");
        assert_nothing_written(root.path());

        // min_app 缺失：平台包必填。
        let root = tempfile::tempdir().unwrap();
        let mut meta = platform_meta(&files);
        meta["min_app"] = json!("");
        let zip = build(&meta, &files);
        let error = install(root.path(), &zip).unwrap_err();
        assert!(error.contains("min_app"), "{error}");
        assert_nothing_written(root.path());

        // min_app 高于当前版本。
        let root = tempfile::tempdir().unwrap();
        let mut meta = platform_meta(&files);
        meta["min_app"] = json!("99.0.0");
        let zip = build(&meta, &files);
        assert!(install(root.path(), &zip).is_err());
        assert_nothing_written(root.path());
    }

    #[test]
    fn rejects_a_core_that_cannot_run_the_platform() {
        let root = tempfile::tempdir().unwrap();
        // mGBA 是已知核心，但跑不了街机；包阶段就要挡住，否则会装出一个
        // "平台亮了、选择器里没有核心"的货架。
        let mut files = platform_files(b"core");
        files[0].0 = "System/linkSlot/cores/mgba/mgba_libretro.so".into();
        let mut core = fbneo_decl();
        core["id"] = json!("mgba");
        core["dylib"] = json!("mgba_libretro.so");
        let mut meta = meta_for(
            PACK,
            "1.0.0",
            vec![platform_decl("arcade"), platform_decl("neogeo")],
            vec![core],
            &files,
        );
        meta["platforms"][0]["core_ids"] = json!(["mgba"]);
        meta["platforms"][0]["default_core"] = json!("mgba");
        meta["platforms"][1]["core_ids"] = json!(["mgba"]);
        meta["platforms"][1]["default_core"] = json!("mgba");
        let zip = build(&meta, &files);
        let error = install(root.path(), &zip).unwrap_err();
        assert!(error.contains("跑不了"), "{error}");
        assert_nothing_written(root.path());
    }

    #[test]
    fn rejects_a_platform_pack_with_more_than_one_core() {
        let root = tempfile::tempdir().unwrap();
        let files = platform_files(b"core");
        let mut other = fbneo_decl();
        other["id"] = json!("mame2003_plus");
        other["dylib"] = json!("mame2003_plus_libretro.so");
        let meta = meta_for(
            PACK,
            "1.0.0",
            vec![platform_decl("arcade"), platform_decl("neogeo")],
            vec![fbneo_decl(), other],
            &files,
        );
        let zip = build(&meta, &files);
        let error = install(root.path(), &zip).unwrap_err();
        assert!(error.contains("一个核心"), "{error}");
        assert_nothing_written(root.path());
    }

    #[test]
    fn rejects_paths_outside_the_platform_whitelist() {
        let cases: [(&str, &str); 6] = [
            ("System/linkSlot/bin/linkslot", "bin/"),
            ("linkSlot/Config/x.json", "Config/"),
            ("System/linkSlot/extensions/platforms/arcade-neogeo/assets/run.sh", "assets"),
            (
                "System/linkSlot/extensions/platforms/arcade-neogeo/metadata/other.dat.json",
                "未声明",
            ),
            (
                "System/linkSlot/extensions/platforms/somebody-else/metadata/arcade.dat.json",
                "自己的目录",
            ),
            (
                "System/linkSlot/extensions/platforms/arcade-neogeo/assets/tool.exe",
                "assets",
            ),
        ];
        for (path, expected) in cases {
            let root = tempfile::tempdir().unwrap();
            let mut files = platform_files(b"core");
            files.push((path.to_string(), b"payload".to_vec()));
            let meta = platform_meta(&files);
            let zip = build(&meta, &files);
            let error = install(root.path(), &zip).unwrap_err();
            assert!(error.contains(expected), "{path}: {error}");
            assert_nothing_written(root.path());
        }
    }

    #[test]
    fn accepts_platform_assets_under_the_pack_directory() {
        let root = tempfile::tempdir().unwrap();
        let mut files = platform_files(b"core");
        files.push((asset("marquee/sf2.png"), b"png".to_vec()));
        let meta = platform_meta(&files);
        let zip = build(&meta, &files);
        install(root.path(), &zip).unwrap();
        assert!(root.path().join(asset("marquee/sf2.png")).is_file());
    }

    /// 仓库里那份 `packs/arcade-neogeo/pack.json` 模板必须真的能过校验。
    ///
    /// 它是 Phase 7 打包脚本的输入：模板和 app 的白名单一旦漂移（改了 folder
    /// 名、忘了 ui_profile、cores 写成两条），Phase 3 打出来的包会被我们自己的
    /// 安装器拒掉，而那时才发现已经太晚。
    #[test]
    fn the_shipped_arcade_template_validates() {
        let template = include_str!("../../../packs/arcade-neogeo/pack.json");
        let mut meta: serde_json::Value = serde_json::from_str(template).unwrap();
        // 模板故意留空 files（DAT 是 Phase 3 才生成），这里补一份最小清单，
        // 让校验走到路径白名单那一步。
        let files = platform_files(b"core");
        meta["files"] = platform_meta(&files)["files"].clone();
        let bytes = serde_json::to_vec(&meta).unwrap();
        let parsed = parse_manifest(&bytes).expect("模板必须能过校验");
        assert_eq!(parsed.kind, "platform");
        assert_eq!(parsed.platforms.len(), 2);
        assert_eq!(parsed.cores.len(), 1);
        assert_eq!(parsed.cores[0].license, "Non-commercial");
    }

    #[test]
    fn rejects_a_platform_pack_without_its_core_file() {
        let root = tempfile::tempdir().unwrap();
        let files = vec![
            (dat("arcade"), DAT.as_bytes().to_vec()),
            (dat("neogeo"), DAT.as_bytes().to_vec()),
        ];
        let meta = platform_meta(&files);
        let zip = build(&meta, &files);
        let error = install(root.path(), &zip).unwrap_err();
        assert!(error.contains("必须带核心"), "{error}");
        assert_nothing_written(root.path());
    }

    #[test]
    fn rejects_oversized_platform_payloads() {
        let files = platform_files(b"core");

        // 单文件 100 MiB > 96 MiB：只改声明，不真造大文件。
        let root = tempfile::tempdir().unwrap();
        let mut meta = platform_meta(&files);
        meta["files"][0]["size"] = json!(100 * 1024 * 1024);
        let zip = build(&meta, &files);
        assert!(install(root.path(), &zip).is_err());
        assert_nothing_written(root.path());

        // 两个 70 MiB 文件：单个合规，总量 140 MiB > 128 MiB。
        let root = tempfile::tempdir().unwrap();
        let mut meta = platform_meta(&files);
        for index in 0..2 {
            meta["files"][index]["size"] = json!(70 * 1024 * 1024);
        }
        let zip = build(&meta, &files);
        let error = install(root.path(), &zip).unwrap_err();
        assert!(error.contains("128 MiB"), "{error}");
        assert_nothing_written(root.path());
    }

    #[test]
    fn a_second_pack_claiming_the_same_platform_is_rejected() {
        let root = tempfile::tempdir().unwrap();
        installs_platform_pack(root.path());
        let before = installed(root.path());

        // 另一个 id，但同样点名 arcade：两个包会共用同一个 fbneo `.so`，
        // 卸载任一个都会删掉对方的文件。
        let files = vec![
            (SO.to_string(), b"other core bytes".to_vec()),
            (dat_for("arcade-solo", "arcade"), DAT.as_bytes().to_vec()),
        ];
        let meta = meta_for(
            "arcade-solo",
            "1.0.0",
            vec![platform_decl("arcade")],
            vec![fbneo_decl()],
            &files,
        );
        let zip = build(&meta, &files);
        let error = install(root.path(), &zip).unwrap_err();
        assert!(error.contains("已由扩展包"), "{error}");

        // 什么都没写：记录没变、核心还是原来那份、新包的目录不存在。
        assert_eq!(installed(root.path()), before);
        assert_eq!(
            std::fs::read(root.path().join(SO)).unwrap(),
            b"\x7fELF fake fbneo"
        );
        assert!(!root
            .path()
            .join("System/linkSlot/extensions/platforms/arcade-solo")
            .exists());
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
            ("armgba", "linkslot-pack-armgba-v1.0.0.lsp"),
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