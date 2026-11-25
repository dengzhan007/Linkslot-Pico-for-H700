//! 平台扩展包的**激活门**。
//!
//! 产品约束是"没装包就不识别"：`Games/Arcade` 与 `Games/NeoGeo` 里即便有 ZIP，
//! 只要卡上没有合法的 `kind=platform` 包记录，扫描器就完全不该看见它们。这条
//! 规则集中在这里，`scan` 只问 [`available_platforms`]，别的模块不再重复判断。
//!
//! 记录文件由 Phase 2 的安装器写入，本阶段只读；读不懂就当"没有包"处理，
//! 因为一个损坏的 JSON 绝不能把整台机器的游戏库清空。
//!
//! 激活需要同时满足两件事：
//! 1. 记录里 `kind == "platform"` 且 `platforms[]` 点名了该机种；
//! 2. 记录里 `cores[]` 至少有一个核心**真的**落在卡上（普通文件、长度 > 0）。
//!
//! 第 2 条是防"半装"：包记录写进去了但 `.so` 拷贝没完成（拔卡、空间不足）时，
//! 平台保持沉默，而不是在货架上放一堆点开就报缺核心的游戏。

use std::path::Path;

use serde::Deserialize;
use slot_model::Platform;

/// 已安装包记录（相对卡根）。
pub const RECORD_REL: &str = "linkSlot/Config/packs/installed.json";

/// 唯一能激活机种的包类型。
const KIND_PLATFORM: &str = "platform";

/// 需要平台包才能出现的机种。
///
/// 顺序即 [`active_platforms`] 的输出顺序，与 `Platform::BUILTIN` 之后的追加
/// 顺序一致。
pub const HOSTED: [Platform; 2] = [Platform::Arcade, Platform::Neogeo];

/// 一条已安装包记录。
///
/// 只声明本阶段用得到的字段。Phase 2 会往同一个 JSON 对象里写 `version`、
/// `files` 等，serde 默认忽略未声明的键，因此上游加字段不会把这里读崩。
#[derive(Clone, Debug, Default, Deserialize)]
pub struct PackRecord {
    /// 包类型（`"platform"` / `"core"` / `"game"` …）。缺省视为未知类型。
    #[serde(default)]
    pub kind: String,
    /// 该包激活的机种，用机种目录名书写（`"arcade"` / `"NeoGeo"` 都认）。
    #[serde(default)]
    pub platforms: Vec<String>,
    /// 该包提供的核心 id（`"fbneo"`、`"mgba"` …）。
    #[serde(default)]
    pub cores: Vec<String>,
    /// 包标识，供诊断与 Phase 2 卸载使用。
    #[serde(default)]
    pub id: String,
}

/// 记录文件的两种可能外形。
///
/// Phase 1 还没有安装器，格式只在设计文档里约定；这里同时接受裸数组与
/// `{"packs": [...]}`，以免 Phase 2 选错外形时整份记录被静默丢弃。
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Envelope {
    List(Vec<PackRecord>),
    Object {
        #[serde(default)]
        packs: Vec<PackRecord>,
    },
}

/// 读取全部包记录。文件缺失、损坏、形状不认识时返回空表（不 panic）。
pub fn records(root: &Path) -> Vec<PackRecord> {
    let path = root.join(RECORD_REL);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    match serde_json::from_str::<Envelope>(&text) {
        Ok(Envelope::List(list)) => list,
        Ok(Envelope::Object { packs }) => packs,
        Err(e) => {
            // 记一行日志就够了：损坏的记录等价于"没装包"，游戏库照常可用。
            eprintln!("slot: packs: {RECORD_REL}: {e}");
            Vec::new()
        }
    }
}

/// 该机种是否属于"要装包才会出现"的一类。
pub fn is_hosted(platform: Platform) -> bool {
    HOSTED.contains(&platform)
}

/// 平台是否已被某个包激活。
pub fn active(root: &Path, platform: Platform) -> bool {
    if !is_hosted(platform) {
        return false;
    }
    let dir = platform.dir_name();
    records(root).iter().any(|record| {
        record.kind.trim().eq_ignore_ascii_case(KIND_PLATFORM)
            && record
                .platforms
                .iter()
                .any(|name| name.trim().eq_ignore_ascii_case(dir))
            // 光"卡上有这个 .so"不够：它还得真的跑得了这台机种。否则一份
            // 写错核心名的记录（例如 cores=["mgba"]）会点亮街机，而选择器
            // 里又挑不出任何核心——货架上就会出现点开必然报缺核心的条目。
            //
            // 找文件用解析后的规范名（`dylib`），而不是记录里的原字符串：
            // 记录允许写 `"fbneo_libretro"` 这种 libretro 风格的名字，用它去
            // 拼路径会去找 `cores/fbneo_libretro/fbneo_libretro_libretro.so`，
            // 把一份合法记录误判成半装。
            && record.cores.iter().any(|core| {
                crate::Core::parse(core).is_some_and(|parsed| {
                    parsed.runs(platform) && core_present(root, parsed.as_str())
                })
            })
    })
}

/// 已激活的平台包机种（顺序固定，便于测试与日志）。
pub fn active_platforms(root: &Path) -> Vec<Platform> {
    HOSTED
        .into_iter()
        .filter(|platform| active(root, *platform))
        .collect()
}

/// 扫描器应该遍历的机种：内置六台 + 已激活的包机种。
///
/// 这是"没包不扫描"的唯一关口，`scan::collect` 只调用这里。
pub fn available_platforms(root: &Path) -> Vec<Platform> {
    let mut out: Vec<Platform> = Platform::BUILTIN.to_vec();
    out.extend(active_platforms(root));
    out
}

/// 核心二进制是否真的在卡上。
///
/// 记录来自卡上的 JSON，因此核心名先做白名单校验再拼路径：一个带 `/` 或 `..`
/// 的名字不该能把检查指到卡外去。
///
/// 两条候选路径：宿主扩展名优先（桌面板上自己构建/安装的核心），再试规范的
/// `.so`（设备的卡格式）。**设备上两者是同一个文件**；桌面板上包里的 `.so` 仍
/// 应点亮平台——它是给设备用的包，装到卡上就该生效。注意这与
/// `Core::is_available` 刻意不同：那里只认宿主扩展名，否则 GBA/SFC 的选择器
/// 会把设备用的 Linux `.so` 当成桌面可加载的核心。
fn core_present(root: &Path, core: &str) -> bool {
    let core = core.trim();
    if core.is_empty()
        || core.contains('/')
        || core.contains('\\')
        || core.contains(':')
        || core == "."
        || core == ".."
    {
        return false;
    }
    let dir = root.join(slot_model::layout::SYSTEM_CORES).join(core);
    [
        dir.join(format!(
            "{core}_libretro.{}",
            std::env::consts::DLL_EXTENSION
        )),
        dir.join(format!("{core}_libretro.so")),
    ]
    .iter()
    .any(|file| {
        // `symlink_metadata`：符号链接本身不算核心。一个悬空链接若算数，平台
        // 就会被点亮，而运行时 dlopen 必然失败。
        let Ok(meta) = std::fs::symlink_metadata(file) else {
            return false;
        };
        meta.file_type().is_file() && meta.len() > 0
    })
}
