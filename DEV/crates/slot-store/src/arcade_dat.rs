//! 街机 / NeoGeo 的 DAT（set → 标题 / 机板 / 年份 / parent / BIOS）。
//!
//! DAT 由平台包携带，落在
//! `System/linkSlot/extensions/platforms/<pack_id>/metadata/<平台小写>.dat.json`
//! ——与 `packs` 的安装白名单是同一份路径规则，所以只有合法装进来的包才会有它。
//!
//! **DAT 只做增强，不做门**：文件缺失、JSON 坏、字段类型不对，一律退化成空表
//! 并记一行日志，扫描结果只损失"标题/机板"这些装饰，zip 仍然照常出现在书架上。
//! 反过来也一样：一个 set 不在 DAT 里，不是"这不是游戏"，只是"我们不知道它叫什么"。

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use slot_model::Platform;

/// 平台包内 DAT 的目录（`System/linkSlot/extensions/platforms/<pack>/metadata`）。
const PLATFORM_EXTENSIONS: [&str; 4] = ["System", "linkSlot", "extensions", "platforms"];

/// 一张 8.7k 条的 DAT 才 1 MB；20 万条已经远超任何已知 DAT，按损坏处理——免得
/// 一张坏文件把设备内存吃光。
const MAX_SETS: usize = 200_000;

/// set 名就是 zip 名，超过这个长度的必然是脏数据（真实 FBNeo 最长 60 上下）。
const MAX_SET_NAME: usize = 128;

/// 街机柜的屏幕方向（Phase 4 的 TATE 布局用）。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Orientation {
    #[default]
    Horizontal,
    Vertical,
}

impl Orientation {
    /// 解析 DAT 里的写法；不认识的一律当作横屏。
    pub fn parse(raw: &str) -> Orientation {
        if raw.trim().eq_ignore_ascii_case("vertical") {
            Orientation::Vertical
        } else {
            Orientation::Horizontal
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Orientation::Horizontal => "horizontal",
            Orientation::Vertical => "vertical",
        }
    }
}

/// 一个 set（一套 ROM）的元数据。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// zip 名（不含扩展名），也是 [`Catalog::get`] 的键。
    pub set: String,
    /// 展示名；DAT 里为空时由调用方回退到 set 名。
    pub title: String,
    /// 机板名（`CPS-1` / `NEOGEO` / `Konami` …）。
    pub system: String,
    /// 年份原样字符串（DAT 里本来就是 `1991` / `199?` 这种）。
    pub year: String,
    pub players: u8,
    pub orientation: Orientation,
    /// 该 set 的父版本（FBNeo 的 `cloneof`）；空 = 它自己就是母版。
    pub parent: String,
    /// BIOS 集合：不进书架，但运行时可能被其它 set 依赖。
    pub is_bios: bool,
}

/// 一张平台 DAT 的只读视图。
#[derive(Clone, Debug)]
pub struct Catalog {
    entries: HashMap<String, Entry>,
    platform: Platform,
}

impl Catalog {
    /// 空表。没装包、没 DAT、DAT 坏了都用它。
    pub fn empty(platform: Platform) -> Catalog {
        Catalog {
            entries: HashMap::new(),
            platform,
        }
    }

    /// 用一组条目直接建表。
    ///
    /// 给测试与将来的导入器用（`load` 只认卡上的 DAT 文件）；同名 set 后者覆盖
    /// 前者，与 `HashMap::insert` 的语义一致。
    pub fn from_entries(platform: Platform, entries: Vec<Entry>) -> Catalog {
        let mut map = HashMap::with_capacity(entries.len());
        for entry in entries {
            map.insert(entry.set.clone(), entry);
        }
        Catalog {
            entries: map,
            platform,
        }
    }

    /// 读某平台的 DAT；任何问题都退化成空表（不 panic）。
    ///
    /// **降级时会打一条带结论的警告**（每次加载一条）：DAT 只做增强，所以缺了
    /// 它不阻止扫描——但用户会看到"标题变成文件名、BIOS 集合混进书架"，那必须
    /// 在日志里明说，否则只会被当成 bug 报上来。DAT 正常时一行都不打。
    pub fn load(root: &Path, platform: Platform) -> Catalog {
        let Some(path) = dat_path(root, platform) else {
            // 连平台包记录都没有：这不是"降级"，是这台机种压根没被启用。
            return Catalog::empty(platform);
        };
        match read_catalog(&path, platform) {
            Ok(catalog) => catalog,
            Err(reason) => {
                eprintln!("{}", degraded_message(platform, &path, &reason));
                Catalog::empty(platform)
            }
        }
    }

    pub fn platform(&self) -> Platform {
        self.platform
    }

    /// 按 set 名查（大小写敏感：DAT 的 set 名就是 zip 名）。
    pub fn get(&self, set: &str) -> Option<&Entry> {
        self.entries.get(set)
    }

    /// 某个母版的所有子版本。Phase 4 的"版本"子菜单用它。
    ///
    /// 不做索引：一张 DAT 最多几千个 clone，而调用点是"打开某个游戏的子菜单"
    /// 这种人工操作；为它多存一份 parent → Vec 的表不值得。
    pub fn clones_of<'a>(&'a self, parent: &'a str) -> impl Iterator<Item = &'a Entry> + 'a {
        self.entries
            .values()
            .filter(move |entry| !parent.is_empty() && entry.parent == parent)
    }

    /// 全部条目，**顺序不保证**（HashMap）。导入/诊断/基准用，需要稳定顺序的
    /// 调用方自己排序。
    pub fn entries(&self) -> impl Iterator<Item = &Entry> {
        self.entries.values()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// 读文件 + 解析；失败时把"为什么"拼成一句人话，供降级警告使用。
fn read_catalog(path: &Path, platform: Platform) -> Result<Catalog, String> {
    let bytes = std::fs::read(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            "文件不存在".to_string()
        } else {
            format!("读取失败：{error}")
        }
    })?;
    parse(&bytes, platform)
}

/// 降级警告的正文。
///
/// 独立出来的理由：它是本模块对用户的**唯一交代**，值得被单测钉住格式——
/// 一旦改文案改到"看不出后果"，排障的人只会看到一只沉默的空书架。
fn degraded_message(platform: Platform, path: &Path, reason: &str) -> String {
    format!(
        "slot: {}: DAT 缺失或损坏（{}）：{reason}；标题回退文件名，BIOS/clone 过滤已降级",
        platform.dir_name().to_ascii_lowercase(),
        path.display(),
    )
}

/// 该平台 DAT 的卡内路径：从已安装的平台包记录推导。
///
/// 只认 `kind=="platform"` 且 `platforms[]` 点名了这台机种的记录——与激活门
/// 同一份数据源，不会出现"扫描说激活了、DAT 却来自另一个包"的情况。
fn dat_path(root: &Path, platform: Platform) -> Option<PathBuf> {
    let name = platform.dir_name();
    let record = crate::platform_packs::records(root)
        .into_iter()
        .find(|record| {
            record.kind.trim().eq_ignore_ascii_case("platform")
                && record
                    .platforms
                    .iter()
                    .any(|declared| declared.trim().eq_ignore_ascii_case(name))
        })?;
    let file = format!("{}.dat.json", name.to_ascii_lowercase());
    let base = root.join(PLATFORM_EXTENSIONS[0]);
    let base = base.join(PLATFORM_EXTENSIONS[1]);
    Some(
        base.join(PLATFORM_EXTENSIONS[2])
            .join(PLATFORM_EXTENSIONS[3])
            .join(record.id)
            .join("metadata")
            .join(file),
    )
}

#[derive(Deserialize)]
struct DatFile {
    #[serde(default)]
    #[allow(dead_code)]
    schema: u32,
    #[serde(default)]
    #[allow(dead_code)]
    platform: String,
    #[serde(default)]
    sets: Vec<DatSet>,
}

#[derive(Deserialize)]
struct DatSet {
    set: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    system: String,
    #[serde(default)]
    year: String,
    #[serde(default)]
    players: u8,
    #[serde(default)]
    orientation: String,
    #[serde(default)]
    parent: String,
    #[serde(default)]
    is_bios: bool,
}

fn parse(bytes: &[u8], platform: Platform) -> Result<Catalog, String> {
    let file: DatFile = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    if file.sets.len() > MAX_SETS {
        return Err(format!(
            "sets 条目过多（{} > {MAX_SETS}），按损坏处理",
            file.sets.len()
        ));
    }
    let mut entries = HashMap::with_capacity(file.sets.len());
    for set in file.sets {
        let name = set.set.trim();
        if name.is_empty() || name.len() > MAX_SET_NAME {
            continue;
        }
        let title = if set.title.trim().is_empty() {
            name.to_string()
        } else {
            set.title.trim().to_string()
        };
        entries.insert(
            name.to_string(),
            Entry {
                set: name.to_string(),
                title,
                system: set.system.trim().to_string(),
                year: set.year.trim().to_string(),
                // 缺省 1 人：FBNeo 的 DAT 根本没有 players 字段，写 0 会让 UI 画出
                // "0 人游戏"这种明显错误。
                players: if set.players == 0 { 1 } else { set.players },
                orientation: Orientation::parse(&set.orientation),
                parent: set.parent.trim().to_string(),
                is_bios: set.is_bios,
            },
        );
    }
    Ok(Catalog {
        entries,
        platform,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog(json: &str) -> Catalog {
        parse(json.as_bytes(), Platform::Arcade).expect("valid dat")
    }

    #[test]
    fn parses_the_fields_a_shelf_needs() {
        let catalog = catalog(
            r#"{"schema":1,"platform":"arcade","sets":[
                {"set":"sf2","title":"Street Fighter II","system":"CPS-1","year":"1991","parent":"","is_bios":false},
                {"set":"neogeo","title":"Neo Geo","system":"NEOGEO","year":"1990","parent":"","is_bios":true}
            ]}"#,
        );
        assert_eq!(catalog.len(), 2);
        let sf2 = catalog.get("sf2").expect("sf2");
        assert_eq!(sf2.title, "Street Fighter II");
        assert_eq!(sf2.system, "CPS-1");
        assert_eq!(sf2.year, "1991");
        assert_eq!(sf2.players, 1, "缺省 1 人");
        assert_eq!(sf2.orientation, Orientation::Horizontal);
        assert!(catalog.get("neogeo").expect("bios").is_bios);
    }

    #[test]
    fn an_empty_title_falls_back_to_the_set_name() {
        let catalog = catalog(r#"{"sets":[{"set":"sf2","title":"  "}]}"#);
        assert_eq!(catalog.get("sf2").unwrap().title, "sf2");
    }

    #[test]
    fn players_and_orientation_are_read_when_present() {
        let catalog = catalog(
            r#"{"sets":[{"set":"ddp","players":2,"orientation":"Vertical"}]}"#,
        );
        let entry = catalog.get("ddp").unwrap();
        assert_eq!(entry.players, 2);
        assert_eq!(entry.orientation, Orientation::Vertical);
    }

    #[test]
    fn broken_entries_are_skipped_not_fatal() {
        let catalog = catalog(
            r#"{"sets":[
                {"set":"","title":"nameless"},
                {"set":"sf2","title":"ok"}
            ]}"#,
        );
        assert_eq!(catalog.len(), 1);
        assert!(catalog.get("sf2").is_some());
    }

    #[test]
    fn a_wrong_type_degrades_to_an_empty_catalog() {
        let error = parse(
            br#"{"sets":[{"set":"sf2","players":"two"}]}"#,
            Platform::Arcade,
        );
        assert!(error.is_err(), "类型不对必须整体判坏，而不是猜一个值");
    }

    #[test]
    fn clones_point_at_their_parent() {
        let catalog = catalog(
            r#"{"sets":[
                {"set":"sf2","title":"SF2"},
                {"set":"sf2ce","title":"SF2 CE","parent":"sf2"},
                {"set":"sf2hf","title":"SF2 HF","parent":"sf2"}
            ]}"#,
        );
        let mut clones: Vec<&str> = catalog.clones_of("sf2").map(|e| e.set.as_str()).collect();
        clones.sort_unstable();
        assert_eq!(clones, vec!["sf2ce", "sf2hf"]);
        assert_eq!(catalog.clones_of("sf2ce").count(), 0);
        assert_eq!(catalog.clones_of("").count(), 0, "空 parent 不该匹配任何东西");
    }

    /// 降级日志是这个模块对用户的唯一交代：必须一眼看出"后果是什么"。
    #[test]
    fn the_degraded_warning_says_what_is_lost() {
        let message = degraded_message(
            Platform::Neogeo,
            Path::new("System/linkSlot/extensions/platforms/arcade-neogeo/metadata/neogeo.dat.json"),
            "文件不存在",
        );
        assert!(message.starts_with("slot: neogeo: "), "{message}");
        assert!(message.contains("DAT 缺失或损坏"), "{message}");
        assert!(message.contains("neogeo.dat.json"), "缺路径：{message}");
        assert!(message.contains("文件不存在"), "缺原因：{message}");
        assert!(message.contains("标题回退文件名"), "没写后果：{message}");
        assert!(message.contains("BIOS/clone 过滤已降级"), "没写后果：{message}");
    }

    /// 缺 DAT 不阻止扫描：空表就是"没有增强"，不是"没有游戏"。
    #[test]
    fn a_missing_dat_loads_as_an_empty_catalog_without_panicking() {
        let root = tempfile::tempdir().unwrap();
        // 有平台包记录（否则连路径都推不出来），但没有 DAT 文件。
        std::fs::create_dir_all(root.path().join("linkSlot/Config/packs")).unwrap();
        std::fs::write(
            root.path().join("linkSlot/Config/packs/installed.json"),
            br#"[{"id":"arcade-neogeo","kind":"platform","platforms":["arcade","neogeo"],"cores":["fbneo"]}]"#,
        )
        .unwrap();

        let catalog = Catalog::load(root.path(), Platform::Arcade);
        assert!(catalog.is_empty());
        assert_eq!(catalog.platform(), Platform::Arcade);
    }
}
