//! Phase 3「DAT + parent/clone/BIOS/标签」的**独立验收**（QA 草稿）。
//!
//! 与工程师的 `tests/arcade_dat.rs` 互补：那个文件守 `Catalog` 自己的解析边界
//! （字段、类型、上限），这个文件从**一张卡**出发，只问"货架上到底出现什么"：
//! 哪一条该被 clone/BIOS 规则吞掉、标题从哪来、贴纸找哪张、跨平台会不会串。
//!
//! 夹具一律自己造：卡上只有 `installed.json` + 核心 + `Games/<机种>/<stem>.zip`
//! + 自己写的 DAT。`*.zip` 的内容无关紧要（扫描器不解析 ZIP），所以是几个字节
//! 的假头；贴纸只是空文件（`listing()` 只列目录，不解码 PNG）。

use std::path::{Path, PathBuf};

use serde_json::json;
use slot_store::{platform_packs, scan_quiet, Platform};
use tempfile::TempDir;

const FBNEO: &str = "fbneo";
/// 记录里的包 id；DAT 目录名由它决定（`.../platforms/<id>/metadata/...`）。
const PACK: &str = "platform-arcade";

// ---------------------------------------------------------------------------
// 夹具
// ---------------------------------------------------------------------------

fn so_name(core: &str) -> String {
    format!("{core}_libretro.{}", std::env::consts::DLL_EXTENSION)
}

fn write_file(path: &Path, bytes: &[u8]) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, bytes).unwrap();
}

/// 装一个平台包记录 + 把核心放到卡上（激活门的两个条件）。
fn install(root: &Path, platforms: &[&str]) {
    let core = root
        .join("System/linkSlot/cores")
        .join(FBNEO)
        .join(so_name(FBNEO));
    write_file(&core, b"\x7fELF pretend this is a libretro core");
    let record = json!({
        "schema": 1,
        "packs": [{
            "id": PACK,
            "kind": "platform",
            "version": "1.0.0",
            "platforms": platforms,
            "cores": [FBNEO],
        }],
    });
    write_file(
        &root.join(slot_store::RECORD_REL),
        record.to_string().as_bytes(),
    );
}

/// 卡上的一个 ROM（内容与扫描无关，只要扩展名是 zip）。
fn rom(root: &Path, platform_dir: &str, stem: &str) {
    write_file(
        &root.join("Games").join(platform_dir).join(format!("{stem}.zip")),
        b"PK\x03\x04 qa fixture",
    );
}

/// 平台包携带的 DAT：`.../platforms/<pack>/metadata/<平台小写>.dat.json`。
fn dat(root: &Path, platform_dir: &str, sets: serde_json::Value) -> PathBuf {
    let path = root
        .join("System/linkSlot/extensions/platforms")
        .join(PACK)
        .join("metadata")
        .join(format!("{}.dat.json", platform_dir.to_ascii_lowercase()));
    write_file(
        &path,
        json!({"schema": 1, "platform": platform_dir.to_ascii_lowercase(), "sets": sets})
            .to_string()
            .as_bytes(),
    );
    path
}

fn sticker(root: &Path, platform_dir: &str, file: &str) {
    write_file(
        &root.join("Labels").join(platform_dir).join(file),
        b"\x89PNG qa fixture",
    );
}

/// 某个机种在货架上的 `(stem, title)`，按扫描顺序。
fn listed(root: &Path, platform: Platform) -> Vec<(String, String)> {
    scan_quiet(root)
        .unwrap()
        .into_iter()
        .filter(|cart| cart.platform == platform)
        .map(|cart| (cart.stem, cart.title))
        .collect()
}

fn label_of(root: &Path, platform: Platform, stem: &str) -> Option<String> {
    scan_quiet(root)
        .unwrap()
        .into_iter()
        .find(|cart| cart.platform == platform && cart.stem == stem)
        .and_then(|cart| cart.label)
        .and_then(|path| path.file_name()?.to_str().map(str::to_string))
}

/// 一张卡：装了 arcade + neogeo 包，`Games/` 下还没放任何 ROM。
fn card(platforms: &[&str]) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    install(dir.path(), platforms);
    dir
}

// ---------------------------------------------------------------------------
// ① parent / clone
// ---------------------------------------------------------------------------

/// 母版在卡上时，clone 不该单独占一个货架位（由母版代表）；标题取母版的 DAT。
#[test]
fn a_clone_whose_parent_is_on_the_card_is_hidden() {
    let dir = card(&["arcade"]);
    let root = dir.path();
    rom(root, "Arcade", "mslug");
    rom(root, "Arcade", "mslug2");
    dat(
        root,
        "Arcade",
        json!([
            {"set": "mslug", "title": "Metal Slug"},
            {"set": "mslug2", "title": "Metal Slug 2", "parent": "mslug"},
        ]),
    );

    assert_eq!(
        listed(root, Platform::Arcade),
        vec![("mslug".to_string(), "Metal Slug".to_string())],
        "母版在卡上，clone 仍占了货架位"
    );
}

/// 只拷了 clone（母版不在卡上）时，它必须出现——不能被静默吞掉。
#[test]
fn an_orphan_clone_still_appears_with_its_own_title() {
    let dir = card(&["arcade"]);
    let root = dir.path();
    rom(root, "Arcade", "mslugx");
    dat(
        root,
        "Arcade",
        json!([
            {"set": "mslugx", "title": "Metal Slug X", "parent": "mslug3"},
            {"set": "mslug3", "title": "Metal Slug 3"},
        ]),
    );

    assert_eq!(
        listed(root, Platform::Arcade),
        vec![("mslugx".to_string(), "Metal Slug X".to_string())],
        "母版不在卡上时 clone 被吞了"
    );
}

/// 母版名的大小写不该影响判定：DAT 写 `MSLUG`、卡上是 `mslug.zip`。
#[test]
fn the_parent_match_ignores_ascii_case() {
    let dir = card(&["arcade"]);
    let root = dir.path();
    rom(root, "Arcade", "mslug");
    rom(root, "Arcade", "mslug2");
    dat(
        root,
        "Arcade",
        json!([
            {"set": "mslug", "title": "Metal Slug"},
            {"set": "mslug2", "title": "Metal Slug 2", "parent": "MSLUG"},
        ]),
    );

    assert_eq!(
        listed(root, Platform::Arcade),
        vec![("mslug".to_string(), "Metal Slug".to_string())],
        "parent 的大小写不一致就让 clone 漏上了货架"
    );
}

// ---------------------------------------------------------------------------
// ② 孤儿 clone 的提醒：每平台最多一条
// ---------------------------------------------------------------------------

/// 子进程里的工作用例：造 3 个孤儿 clone，扫描一次。
///
/// 单独跑也成立（断言三个都在）；父用例会以子进程方式重入它，用来抓 stderr。
#[test]
fn qa2_orphan_warning_child_scan() {
    let dir = card(&["arcade"]);
    let root = dir.path();
    for stem in ["alpha", "beta", "gamma"] {
        rom(root, "Arcade", stem);
    }
    dat(
        root,
        "Arcade",
        json!([
            {"set": "alpha", "title": "Alpha", "parent": "ghost"},
            {"set": "beta", "title": "Beta", "parent": "ghost"},
            {"set": "gamma", "title": "Gamma", "parent": "ghost"},
        ]),
    );

    let names: Vec<String> = listed(root, Platform::Arcade)
        .into_iter()
        .map(|(stem, _)| stem)
        .collect();
    assert_eq!(names, vec!["alpha", "beta", "gamma"], "孤儿 clone 被吞了");
}

/// 三个孤儿 clone 只能换来越多一条提醒：日志刷屏是可用性问题。
///
/// `eprintln` 写在进程的 stderr 上，进程内抓不到（Rust 测试框架只捕获 stdout），
/// 所以这里把测试二进制**自己**再起一次，只跑 [`qa2_orphan_warning_child_scan`]
/// 并抓它的 stderr。断言是"**最多**一条"（不是"正好一条"）：文案改动不该让这条
/// 用例变红，而"真的刷了 N 行"必须红。
#[test]
fn the_orphan_warning_is_printed_at_most_once_per_platform() {
    let Ok(exe) = std::env::current_exe() else {
        eprintln!("SKIP the_orphan_warning_is_printed_at_most_once_per_platform: 拿不到测试二进制路径");
        return;
    };
    let output = std::process::Command::new(&exe)
        .args([
            "--exact",
            "qa2_orphan_warning_child_scan",
            "--nocapture",
            "--test-threads=1",
        ])
        .output();
    let Ok(output) = output else {
        eprintln!("SKIP the_orphan_warning_is_printed_at_most_once_per_platform: 无法重入测试二进制");
        return;
    };
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "子进程里的扫描用例失败：\n{stderr}"
    );
    let warnings = stderr
        .lines()
        .filter(|line| line.contains("slot: scan:") && line.contains("clone"))
        .count();
    assert!(
        warnings <= 1,
        "3 个孤儿 clone 换来了 {warnings} 行提醒（应该最多 1 行）：\n{stderr}"
    );
}

// ---------------------------------------------------------------------------
// ③ 贴纸回退：`<stem>.png` 优先，其次 `<title>.png`
// ---------------------------------------------------------------------------

#[test]
fn a_title_named_sticker_is_found_when_there_is_no_stem_sticker() {
    let dir = card(&["arcade"]);
    let root = dir.path();
    rom(root, "Arcade", "sf2");
    dat(root, "Arcade", json!([{"set": "sf2", "title": "Street Fighter II"}]));
    sticker(root, "Arcade", "Street Fighter II.png");

    assert_eq!(
        label_of(root, Platform::Arcade, "sf2").as_deref(),
        Some("Street Fighter II.png"),
        "按标题命名的贴纸没被找到"
    );

    // 两套并存时以文件名为准：一卡千条时这是唯一可预测的规则。
    sticker(root, "Arcade", "sf2.png");
    assert_eq!(
        label_of(root, Platform::Arcade, "sf2").as_deref(),
        Some("sf2.png"),
        "stem 贴纸应当优先于 title 贴纸"
    );
}

/// 标题里有 `:` `/` 时不去按标题找贴纸（FAT/exFAT 上是非法文件名）。
///
/// 这条只断言"不会指向一个不存在的路径"：`Street Fighter II: The World Warrior`
/// 在 Windows 上根本造不出来，所以只能证明"有 stem 贴纸时仍然用 stem 贴纸"。
#[test]
fn a_title_with_illegal_characters_falls_back_to_the_stem_sticker() {
    let dir = card(&["arcade"]);
    let root = dir.path();
    rom(root, "Arcade", "sf2ce");
    dat(
        root,
        "Arcade",
        json!([{"set": "sf2ce", "title": "Street Fighter II: The World Warrior"}]),
    );
    sticker(root, "Arcade", "sf2ce.png");

    assert_eq!(
        label_of(root, Platform::Arcade, "sf2ce").as_deref(),
        Some("sf2ce.png"),
        "带冒号的标题把贴纸查找带偏了"
    );
}

// ---------------------------------------------------------------------------
// ④ DAT 上限：20 万条当损坏，但扫描照常
// ---------------------------------------------------------------------------

fn dat_with_sets(root: &Path, platform_dir: &str, total: usize, first_title: &str) -> PathBuf {
    let mut text = String::from("{\"schema\":1,\"sets\":[");
    text.push_str(&format!("{{\"set\":\"a\",\"title\":\"{first_title}\"}}"));
    for index in 1..total {
        text.push_str(&format!(",\n{{\"set\":\"filler{index:06}\",\"title\":\"Filler {index}\"}}"));
    }
    text.push_str("]}");
    let path = root
        .join("System/linkSlot/extensions/platforms")
        .join(PACK)
        .join("metadata")
        .join(format!("{}.dat.json", platform_dir.to_ascii_lowercase()));
    write_file(&path, text.as_bytes());
    path
}

/// 200 000 条（上限本身）仍然生效：标题来自 DAT。
#[test]
fn a_dat_at_the_200_000_limit_is_still_used() {
    let dir = card(&["arcade"]);
    let root = dir.path();
    rom(root, "Arcade", "a");
    dat_with_sets(root, "Arcade", 200_000, "Alpha Attack");

    assert_eq!(
        listed(root, Platform::Arcade),
        vec![("a".to_string(), "Alpha Attack".to_string())],
        "恰好 200000 条的 DAT 被当成损坏了"
    );
}

/// 200 001 条（多一条）按损坏处理：DAT 被忽略，但**扫描照常**、标题回退、不 panic。
#[test]
fn a_dat_one_over_the_limit_degrades_without_breaking_the_scan() {
    let dir = card(&["arcade"]);
    let root = dir.path();
    rom(root, "Arcade", "a");
    dat_with_sets(root, "Arcade", 200_001, "Alpha Attack");

    assert!(
        platform_packs::active(root, Platform::Arcade),
        "DAT 太大把平台也弄没了（DAT 不该是门）"
    );
    assert_eq!(
        listed(root, Platform::Arcade),
        vec![("a".to_string(), "a".to_string())],
        "超限 DAT 应当被忽略、标题回退成文件名，而不是丢游戏或 panic"
    );
}

// ---------------------------------------------------------------------------
// ⑤ 跨平台不串
// ---------------------------------------------------------------------------

#[test]
fn an_arcade_dat_never_titles_a_neogeo_cart() {
    let dir = card(&["arcade", "neogeo"]);
    let root = dir.path();
    // 两个机种目录里有同名 zip；只有 arcade 的 DAT 认识它。
    rom(root, "Arcade", "b");
    rom(root, "NeoGeo", "b");
    dat(root, "Arcade", json!([{"set": "b", "title": "Arcade B"}]));

    assert_eq!(
        listed(root, Platform::Arcade),
        vec![("b".to_string(), "Arcade B".to_string())]
    );
    assert_eq!(
        listed(root, Platform::Neogeo),
        vec![("b".to_string(), "b".to_string())],
        "NeoGeo 的卡用上了 arcade DAT 的标题"
    );
}

// ---------------------------------------------------------------------------
// ⑥ 已记录的降级：包已激活但 DAT 缺失时，BIOS 会作为普通游戏出现
// ---------------------------------------------------------------------------

/// **这是已知降级，不是期望行为**：DAT 只做增强不做门（`arcade_dat.rs` 文档）。
///
/// 所以"包已装、DAT 丢了"时，`neogeo.zip` 这种 BIOS 集合会像普通游戏一样上货架，
/// 标题回退成文件名。这条用例存在的意义是：把降级**钉死成不崩溃**，任何人把
/// BIOS 过滤改成"读不到 DAT 就整目录静默"，这里会红。
#[test]
fn a_missing_dat_lets_bios_zips_through_as_a_known_degradation() {
    let dir = card(&["arcade"]);
    let root = dir.path();
    rom(root, "Arcade", "a");
    rom(root, "Arcade", "neogeo"); // BIOS 集合，与游戏同目录（真实卡就是这样）
    // 故意不写 DAT。

    let shelf = listed(root, Platform::Arcade);
    assert!(
        shelf.contains(&("a".to_string(), "a".to_string())),
        "缺 DAT 时游戏被吞了：{shelf:?}"
    );
    assert!(
        shelf.contains(&("neogeo".to_string(), "neogeo".to_string())),
        "降级行为变了：缺 DAT 时 BIOS 集合的既有表现是像普通游戏一样出现：{shelf:?}"
    );
}

// ---------------------------------------------------------------------------
// ⑦ NIT-1 回归：贴纸"先按 zip 所在目录就近，同目录内 stem 优先，再退顶层"。
// ---------------------------------------------------------------------------

/// 契约（lead 定稿 A 案，`scan.rs` 里四条注释与之一致）：
/// 1. `Labels/<dir>/<相对目录>/<stem>.png`
/// 2. `Labels/<dir>/<相对目录>/<title>.png`
/// 3. `Labels/<dir>/<stem>.png`
/// 4. `Labels/<dir>/<title>.png`
///
/// 旧实现里 title 贴纸**只查顶层**、stem 贴纸才走子目录，所以第 ① 级会命中顶层
/// `a.png`（同一张卡上"把贴纸放进子目录"的意图被静默无视）。逐级断言的意义是：
/// 任何"只改了一半"的实现（例如只把 title 查找改成 nested、却保留
/// `stem_label.or(title_label)` 的外层组合）都会精确红在某一级，而不是只报"贴纸不对"。
#[test]
fn a_sticker_beside_the_rom_wins_over_the_top_level_one() {
    let dir = card(&["arcade"]);
    let root = dir.path();
    // ROM 在子目录里：贴纸的"相对目录"就是 sub。
    write_file(
        &root.join("Games/Arcade/sub/a.zip"),
        b"PK\x03\x04 qa fixture",
    );
    dat(root, "Arcade", json!([{"set": "a", "title": "Metal Slug"}]));

    let sub_title = root.join("Labels/Arcade/sub/Metal Slug.png");
    let sub_stem = root.join("Labels/Arcade/sub/a.png");
    let top_stem = root.join("Labels/Arcade/a.png");
    let top_title = root.join("Labels/Arcade/Metal Slug.png");

    // ① 子目录 title（第 2 级）对顶层 stem（第 3 级）：就近优先 → 子目录那张赢。
    write_file(&sub_title, b"\x89PNG");
    write_file(&top_stem, b"\x89PNG");
    assert_eq!(
        label_of(root, Platform::Arcade, "a").as_deref(),
        Some("Metal Slug.png"),
        "顶层 stem 贴纸抢走了子目录 title 贴纸的位置（旧实现的错法）"
    );

    // ② 删掉子目录那张 → 顶层 stem 兜底（第 3 级）。
    std::fs::remove_file(&sub_title).unwrap();
    assert_eq!(
        label_of(root, Platform::Arcade, "a").as_deref(),
        Some("a.png"),
        "子目录没有贴纸时，顶层 stem 兜底失效"
    );

    // ③ 同一目录内 stem 优先（第 1 级 > 第 2 级）。
    write_file(&sub_title, b"\x89PNG");
    write_file(&sub_stem, b"\x89PNG");
    assert_eq!(
        label_of(root, Platform::Arcade, "a").as_deref(),
        Some("a.png"),
        "同一目录内 title 贴纸抢走了 stem 贴纸的位置"
    );

    // ④ 子目录整层删空 → 顶层；顶层内 stem 仍优先（第 3 级 > 第 4 级）。
    std::fs::remove_file(&sub_stem).unwrap();
    std::fs::remove_file(&sub_title).unwrap();
    write_file(&top_title, b"\x89PNG");
    assert_eq!(
        label_of(root, Platform::Arcade, "a").as_deref(),
        Some("a.png"),
        "顶层 stem 与顶层 title 并存时，stem 应当优先"
    );

    // ⑤ 只剩顶层 title → 第 4 级兜底。
    std::fs::remove_file(&top_stem).unwrap();
    assert_eq!(
        label_of(root, Platform::Arcade, "a").as_deref(),
        Some("Metal Slug.png"),
        "顶层 title 兜底失效"
    );
}
