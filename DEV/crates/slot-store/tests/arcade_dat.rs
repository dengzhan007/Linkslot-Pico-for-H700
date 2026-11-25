//! Phase 3：街机 / NeoGeo 的 DAT 驱动的扫描行为。
//!
//! 每个用例都从"平台包已装好"出发（记录 + 核心 `.so` + 任意 DAT），因为街机的
//! 扫描本来就只在包激活时发生；DAT 只决定"标题 / 谁代表谁 / 谁是 BIOS"。

use std::path::Path;

use slot_store::{arcade_dat::Catalog, scan_quiet, Cart, Platform};
use tempfile::TempDir;

const RECORD: &str = "linkSlot/Config/packs/installed.json";
const PACK: &str = "arcade-neogeo";

fn new_card() -> TempDir {
    tempfile::tempdir().unwrap()
}

fn write_file(root: &Path, rel: &str, bytes: &[u8]) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

/// 装一个最小平台包：只有记录 + 核心 `.so`（DAT 由各用例按需写）。
fn install_pack(root: &Path, platforms: &[&str]) {
    write_file(
        root,
        "System/linkSlot/cores/fbneo/fbneo_libretro.so",
        b"\x7fELF fake fbneo",
    );
    let platforms: Vec<String> = platforms.iter().map(|name| format!("\"{name}\"")).collect();
    let record = format!(
        "[{{\"id\":\"{PACK}\",\"kind\":\"platform\",\"name\":\"Arcade & NeoGeo\",\
          \"version\":\"1.0.0\",\"installed_at\":1700000000,\
          \"platforms\":[{}],\"cores\":[\"fbneo\"],\"files\":[]}}]",
        platforms.join(",")
    );
    write_file(root, RECORD, record.as_bytes());
}

/// 写一份 DAT；`sets` 是 JSON 数组的**内容**（不含方括号），方便逐例拼。
fn write_dat(root: &Path, platform: &str, sets: &str) {
    let json = format!(
        "{{\"schema\":1,\"platform\":\"{platform}\",\"generated_from\":{{\"dat\":\"test\",\"version\":\"1\"}},\"sets\":[{sets}]}}"
    );
    write_file(
        root,
        &format!("System/linkSlot/extensions/platforms/{PACK}/metadata/{platform}.dat.json"),
        json.as_bytes(),
    );
}

fn add_zip(root: &Path, games_dir: &str, stem: &str) {
    write_file(
        root,
        &format!("Games/{games_dir}/{stem}.zip"),
        b"PK\x03\x04 not really a rom",
    );
}

fn scan(root: &Path) -> Vec<Cart> {
    scan_quiet(root).expect("scan")
}

fn titles(carts: &[Cart]) -> Vec<&str> {
    carts.iter().map(|cart| cart.title.as_str()).collect()
}

#[test]
fn parent_and_clone_show_only_the_parent() {
    let card = new_card();
    install_pack(card.path(), &["arcade", "neogeo"]);
    write_dat(
        card.path(),
        "arcade",
        r#"{"set":"sf2","title":"Street Fighter II: The World Warrior","system":"CPS-1","year":"1991","parent":"","is_bios":false},
           {"set":"sf2ce","title":"Street Fighter II': Champion Edition","system":"CPS-1","year":"1992","parent":"sf2","is_bios":false}"#,
    );
    add_zip(card.path(), "Arcade", "sf2");
    add_zip(card.path(), "Arcade", "sf2ce");

    let carts = scan(card.path());
    assert_eq!(carts.len(), 1, "clone 不该和母版一起占书架：{:?}", titles(&carts));
    assert_eq!(carts[0].stem, "sf2");
    assert_eq!(carts[0].title, "Street Fighter II: The World Warrior");
}

#[test]
fn a_clone_without_its_parent_still_shows() {
    let card = new_card();
    install_pack(card.path(), &["arcade", "neogeo"]);
    write_dat(
        card.path(),
        "arcade",
        r#"{"set":"sf2ce","title":"Champion Edition","system":"CPS-1","parent":"sf2","is_bios":false}"#,
    );
    add_zip(card.path(), "Arcade", "sf2ce");

    let carts = scan(card.path());
    assert_eq!(carts.len(), 1, "只装了 clone 的用户必须还能玩到它");
    assert_eq!(carts[0].title, "Champion Edition");
}

#[test]
fn bios_sets_are_not_listed() {
    let card = new_card();
    install_pack(card.path(), &["arcade", "neogeo"]);
    write_dat(
        card.path(),
        "neogeo",
        r#"{"set":"neogeo","title":"Neo Geo","system":"NEOGEO","parent":"","is_bios":true}"#,
    );
    add_zip(card.path(), "NeoGeo", "neogeo");

    assert!(
        scan(card.path()).is_empty(),
        "BIOS 集合不该出现在书架上"
    );
}

#[test]
fn unknown_sets_fall_back_to_the_file_name() {
    let card = new_card();
    install_pack(card.path(), &["arcade", "neogeo"]);
    write_dat(
        card.path(),
        "arcade",
        r#"{"set":"sf2","title":"Street Fighter II","parent":"","is_bios":false}"#,
    );
    add_zip(card.path(), "Arcade", "mystery");

    let carts = scan(card.path());
    assert_eq!(carts.len(), 1);
    assert_eq!(carts[0].title, "mystery", "DAT 里没有的 set 用文件名");
    assert_eq!(carts[0].code, "", "ZIP 没有 code");
}

#[test]
fn a_missing_or_corrupt_dat_only_costs_titles() {
    // 没有 DAT。
    let card = new_card();
    install_pack(card.path(), &["arcade", "neogeo"]);
    add_zip(card.path(), "Arcade", "sf2");
    add_zip(card.path(), "Arcade", "dino");
    let carts = scan(card.path());
    assert_eq!(carts.len(), 2, "没有 DAT 不该让游戏消失");
    assert_eq!(titles(&carts), vec!["dino", "sf2"]);

    // DAT 坏到解析不了。
    let card = new_card();
    install_pack(card.path(), &["arcade", "neogeo"]);
    write_file(
        card.path(),
        &format!("System/linkSlot/extensions/platforms/{PACK}/metadata/arcade.dat.json"),
        b"{ this is not json",
    );
    add_zip(card.path(), "Arcade", "sf2");
    add_zip(card.path(), "Arcade", "dino");
    let carts = scan(card.path());
    assert_eq!(carts.len(), 2, "坏 DAT 等价于没有 DAT");
    assert_eq!(titles(&carts), vec!["dino", "sf2"]);
}

#[test]
fn platform_dat_does_not_leak() {
    let card = new_card();
    install_pack(card.path(), &["arcade", "neogeo"]);
    // arcade 的 DAT 里故意放一条 neogeo 的 set 名。
    write_dat(
        card.path(),
        "arcade",
        r#"{"set":"mslug","title":"STOLEN FROM ARCADE","parent":"","is_bios":false}"#,
    );
    // neogeo 没有 DAT。
    add_zip(card.path(), "NeoGeo", "mslug");

    let carts = scan(card.path());
    let neogeo: Vec<&Cart> = carts
        .iter()
        .filter(|cart| cart.platform == Platform::Neogeo)
        .collect();
    assert_eq!(neogeo.len(), 1);
    assert_eq!(
        neogeo[0].title, "mslug",
        "NeoGeo 读了 Arcade 的 DAT：{:?}",
        titles(&carts)
    );
}

#[test]
fn no_pack_means_no_dat_and_no_carts() {
    // Phase 1 的回归：目录里同时有 DAT 和 zip，但一条平台包记录都没有。
    let card = new_card();
    write_dat(
        card.path(),
        "arcade",
        r#"{"set":"sf2","title":"Street Fighter II","parent":"","is_bios":false}"#,
    );
    add_zip(card.path(), "Arcade", "sf2");
    add_zip(card.path(), "NeoGeo", "mslug");

    assert!(
        scan(card.path()).is_empty(),
        "没装包时 DAT 也不能让街机出现"
    );
}

#[test]
fn labels_fall_back_from_stem_to_title() {
    let card = new_card();
    install_pack(card.path(), &["arcade", "neogeo"]);
    write_dat(
        card.path(),
        "arcade",
        r#"{"set":"sf2","title":"Street Fighter II","parent":"","is_bios":false},
           {"set":"dino","title":"Cadillacs and Dinosaurs","parent":"","is_bios":false}"#,
    );
    add_zip(card.path(), "Arcade", "sf2");
    add_zip(card.path(), "Arcade", "dino");
    // sf2 同时有两张贴纸：stem 那张必须赢。
    write_file(card.path(), "Labels/Arcade/sf2.png", b"stem label");
    write_file(card.path(), "Labels/Arcade/Street Fighter II.png", b"title label");
    // dino 只有按标题命名的那张。
    write_file(
        card.path(),
        "Labels/Arcade/Cadillacs and Dinosaurs.png",
        b"title only",
    );

    let carts = scan(card.path());
    let label_of = |stem: &str| {
        carts
            .iter()
            .find(|cart| cart.stem == stem)
            .and_then(|cart| cart.label.clone())
            .unwrap_or_else(|| panic!("{stem} 没有贴纸"))
    };
    assert!(
        label_of("sf2").ends_with("sf2.png"),
        "stem 贴纸应当优先：{:?}",
        label_of("sf2")
    );
    assert!(
        label_of("dino").ends_with("Cadillacs and Dinosaurs.png"),
        "没有 stem 贴纸时应当退回标题贴纸：{:?}",
        label_of("dino")
    );
}

#[test]
fn a_title_that_cannot_be_a_file_name_is_never_looked_up() {
    let card = new_card();
    install_pack(card.path(), &["arcade", "neogeo"]);
    // 冒号在 FAT/exFAT 上非法；这条标题不该被当成文件名去找贴纸。
    write_dat(
        card.path(),
        "arcade",
        r#"{"set":"sf2","title":"Street Fighter II: The World Warrior","parent":"","is_bios":false}"#,
    );
    add_zip(card.path(), "Arcade", "sf2");
    write_file(card.path(), "Labels/Arcade/sf2.png", b"stem label");

    let carts = scan(card.path());
    assert_eq!(carts.len(), 1);
    assert!(carts[0].label.as_ref().unwrap().ends_with("sf2.png"));
}

#[test]
fn load_rejects_absurd_entry_counts() {
    let card = new_card();
    install_pack(card.path(), &["arcade", "neogeo"]);
    // 200_001 条：超过上限就按损坏处理，不能把设备内存吃光。
    let mut sets = String::new();
    for index in 0..200_001 {
        if index > 0 {
            sets.push(',');
        }
        sets.push_str(&format!(r#"{{"set":"s{index}"}}"#));
    }
    write_dat(card.path(), "arcade", &sets);

    let catalog = Catalog::load(card.path(), Platform::Arcade);
    assert!(catalog.is_empty(), "200_001 条应当被当成损坏的 DAT");
}

#[test]
fn a_dat_without_a_pack_record_is_never_read() {
    // 记录缺失时连路径都推不出来：Catalog 必须是空的，而不是去猜目录。
    let card = new_card();
    write_dat(
        card.path(),
        "arcade",
        r#"{"set":"sf2","title":"Street Fighter II","parent":"","is_bios":false}"#,
    );
    assert!(Catalog::load(card.path(), Platform::Arcade).is_empty());
}


/// 本机基准：真实 DAT（8039 条）+ 1000 个 zip 的 `Catalog::load` 与扫描耗时。
///
/// 跑法（先用 `python build/fbneo_dat.py` 产出 `target/fbneo-dat/*.dat.json`）：
/// `cargo test -p slot-store --test arcade_dat -- --ignored --nocapture`
///
/// 断言只防"慢到不能用"；真实数字看 `println!`。忽略掉是因为它依赖本机产物。
#[test]
#[ignore = "needs target/fbneo-dat/arcade.dat.json from build/fbneo_dat.py"]
fn bench_a_real_dat_and_a_thousand_zips() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dat = manifest.join("../../target/fbneo-dat/arcade.dat.json");
    let Ok(dat_bytes) = std::fs::read(&dat) else {
        eprintln!("缺 {}，跳过基准", dat.display());
        return;
    };

    let card = new_card();
    install_pack(card.path(), &["arcade", "neogeo"]);
    write_file(
        card.path(),
        &format!("System/linkSlot/extensions/platforms/{PACK}/metadata/arcade.dat.json"),
        &dat_bytes,
    );

    let start = std::time::Instant::now();
    let catalog = Catalog::load(card.path(), Platform::Arcade);
    let load_ms = start.elapsed().as_secs_f64() * 1000.0;
    eprintln!("Catalog::load {} 条：{load_ms:.1} ms", catalog.len());
    assert!(catalog.len() > 8_000, "真实 DAT 应当有 8k+ 条");

    // 取 1000 个母版（parent 为空、非 BIOS），排序后写 zip，保证结果可复现。
    let mut parents: Vec<&str> = catalog
        .entries()
        .filter(|entry| entry.parent.is_empty() && !entry.is_bios)
        .map(|entry| entry.set.as_str())
        .collect();
    parents.sort_unstable();
    for name in parents.iter().take(1000) {
        add_zip(card.path(), "Arcade", name);
    }

    let start = std::time::Instant::now();
    let carts = scan(card.path());
    let scan_ms = start.elapsed().as_secs_f64() * 1000.0;
    eprintln!(
        "扫描 {} 个 zip（DAT {} 条）：{scan_ms:.1} ms → {} 个 Cart",
        parents.len().min(1000),
        catalog.len(),
        carts.len()
    );
    assert_eq!(carts.len(), parents.len().min(1000));
}

/// 子目录里的"按标题命名"贴纸必须能被找到（NIT-1：stem 能嵌套、title 不能）。
#[test]
fn a_title_sticker_in_a_subfolder_is_found() {
    let card = new_card();
    install_pack(card.path(), &["arcade", "neogeo"]);
    write_dat(
        card.path(),
        "arcade",
        r#"{"set":"a","title":"Metal Slug","parent":"","is_bios":false}"#,
    );
    // zip 在子目录里，贴纸放在同一个子目录、且用 DAT 标题命名。
    write_file(card.path(), "Games/Arcade/sub/a.zip", b"PK\x03\x04 rom");
    write_file(
        card.path(),
        "Labels/Arcade/sub/Metal Slug.png",
        b"title sticker",
    );

    let carts = scan(card.path());
    assert_eq!(carts.len(), 1);
    assert_eq!(carts[0].title, "Metal Slug");
    let label = carts[0].label.clone().expect("子目录里的标题贴纸应当命中");
    assert!(
        label.ends_with(Path::new("Labels/Arcade/sub/Metal Slug.png")),
        "命中的不是那张贴纸：{}",
        label.display()
    );
}

/// 同一子目录里 stem 与 title 两张贴纸并存：stem 胜（顺序的第 1 条）。
#[test]
fn a_stem_sticker_in_the_same_subfolder_wins() {
    let card = new_card();
    install_pack(card.path(), &["arcade", "neogeo"]);
    write_dat(
        card.path(),
        "arcade",
        r#"{"set":"a","title":"Metal Slug","parent":"","is_bios":false}"#,
    );
    write_file(card.path(), "Games/Arcade/sub/a.zip", b"PK\x03\x04 rom");
    write_file(card.path(), "Labels/Arcade/sub/a.png", b"stem sticker");
    write_file(
        card.path(),
        "Labels/Arcade/sub/Metal Slug.png",
        b"title sticker",
    );

    let carts = scan(card.path());
    assert_eq!(carts.len(), 1);
    let label = carts[0].label.clone().expect("子目录里的 stem 贴纸应当命中");
    assert!(
        label.ends_with(Path::new("Labels/Arcade/sub/a.png")),
        "同一子目录里 stem 应当优先：{}",
        label.display()
    );
}

/// 子目录里没有时，顶层仍然兜底（顺序的第 3、4 条）。
#[test]
fn top_level_stickers_still_work_for_roms_in_a_subfolder() {
    let card = new_card();
    install_pack(card.path(), &["arcade", "neogeo"]);
    write_dat(
        card.path(),
        "arcade",
        r#"{"set":"a","title":"Metal Slug","parent":"","is_bios":false}"#,
    );
    write_file(card.path(), "Games/Arcade/sub/a.zip", b"PK\x03\x04 rom");
    write_file(card.path(), "Labels/Arcade/Metal Slug.png", b"top sticker");

    let carts = scan(card.path());
    assert_eq!(carts.len(), 1);
    let label = carts[0].label.clone().expect("顶层标题贴纸应当兜底命中");
    assert!(
        label.ends_with(Path::new("Labels/Arcade/Metal Slug.png")),
        "{}",
        label.display()
    );
}

/// A 案的关键一格：**子目录里的 title 贴纸要赢过顶层的 stem 贴纸**。
///
/// 这条是"按文件夹分组"与"按名字种类分组"的唯一分水岭：
/// - 正确（同目录先试 stem 再试 title，两块都试完才退顶层）→ 命中子目录 title；
/// - 若只把 title 查找补成"nested 再 top"、外层仍是 `stem_label.or(title_label)`，
///   顶层 stem 会抢先命中 —— 本用例会红。
#[test]
fn a_subfolder_title_sticker_wins_over_a_top_level_stem_sticker() {
    let card = new_card();
    install_pack(card.path(), &["arcade", "neogeo"]);
    write_dat(
        card.path(),
        "arcade",
        r#"{"set":"a","title":"Metal Slug","parent":"","is_bios":false}"#,
    );
    write_file(card.path(), "Games/Arcade/sub/a.zip", b"PK\x03\x04 rom");
    // 子目录里只有 title 贴纸；顶层有一张更"该赢"的 stem 贴纸。
    write_file(
        card.path(),
        "Labels/Arcade/sub/Metal Slug.png",
        b"same folder title",
    );
    write_file(card.path(), "Labels/Arcade/a.png", b"top stem");

    let carts = scan(card.path());
    assert_eq!(carts.len(), 1);
    let label = carts[0].label.clone().expect("应当命中子目录里的标题贴纸");
    assert!(
        label.ends_with(Path::new("Labels/Arcade/sub/Metal Slug.png")),
        "顶层 stem 抢在了子目录 title 前面（查找没有按文件夹分组）：{}",
        label.display()
    );
}

/// 顶层内部仍然是 stem 优先、title 兜底（顺序的第 3、4 条）。
#[test]
fn a_top_level_stem_wins_over_a_top_level_title() {
    let card = new_card();
    install_pack(card.path(), &["arcade", "neogeo"]);
    write_dat(
        card.path(),
        "arcade",
        r#"{"set":"a","title":"Metal Slug","parent":"","is_bios":false}"#,
    );
    write_file(card.path(), "Games/Arcade/sub/a.zip", b"PK\x03\x04 rom");
    write_file(card.path(), "Labels/Arcade/a.png", b"top stem");
    write_file(card.path(), "Labels/Arcade/Metal Slug.png", b"top title");

    let carts = scan(card.path());
    assert_eq!(carts.len(), 1);
    let label = carts[0].label.clone().expect("顶层应当命中");
    assert!(
        label.ends_with(Path::new("Labels/Arcade/a.png")),
        "顶层应当 stem 优先：{}",
        label.display()
    );
}
