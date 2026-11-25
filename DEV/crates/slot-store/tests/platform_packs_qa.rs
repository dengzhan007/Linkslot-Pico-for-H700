//! Phase 1「街机 / NeoGeo 平台扩展包激活门」的**独立验收**（QA）。
//!
//! 与工程师的 `tests/platform_packs.rs` 互补：那个文件守实现者自己想到的路径，
//! 这个文件按验收单（1–10 条）逐条独立取证，每个用例都从一个"卡上看起来什么
//! 都有"的目录出发 —— `Games/Arcade/a.zip`、`Games/NeoGeo/b.zip`、
//! `System/linkSlot/cores/fbneo/fbneo_libretro.so` 一应俱全 —— 然后只改
//! `installed.json`（或它的某一行字节），断言扫描器看不看得见街机。
//!
//! 产品约束只有一句：**没装包就不识别**。所以下面所有 `assert` 都是在问同一个
//! 问题的不同侧面：包记录缺一个字节，街机就该彻底沉默，而内置六台不受影响。
//!
//! 用例编号与验收单一致（`qa01` … `qa10`），方便逐条对账。

use std::path::{Path, PathBuf};

use serde_json::json;
use slot_store::{
    available_options, available_platforms, platform_packs, scan, scan_quiet, Core, Platform,
    RECORD_REL,
};
use tempfile::TempDir;

/// 平台包里唯一的核心。
const FBNEO: &str = "fbneo";

// ---------------------------------------------------------------------------
// 夹具：一律自己造卡，不依赖真实 sdcard。
// ---------------------------------------------------------------------------

fn so_name(core: &str) -> String {
    format!("{core}_libretro.{}", std::env::consts::DLL_EXTENSION)
}

fn core_file(root: &Path, core: &str) -> PathBuf {
    root.join("System/linkSlot/cores")
        .join(core)
        .join(so_name(core))
}

fn write_file(path: &Path, bytes: &[u8]) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, bytes).unwrap();
}

/// 一张"游戏都在、核心也在"的卡，**只差 `installed.json`**。
fn card() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    // 内容是假 ZIP 头，不是合法 ROM：扫描器若去解 ZIP 只会得到乱码标题。
    write_file(
        &dir.path().join("Games/Arcade/a.zip"),
        b"PK\x03\x04 not a rom",
    );
    write_file(
        &dir.path().join("Games/NeoGeo/b.zip"),
        b"PK\x03\x04 not a rom",
    );
    dir
}

/// 用户手工把 FBNeo 拷进 `System/`（绕过安装器）。
fn hand_copy_core(root: &Path) {
    write_file(
        &core_file(root, FBNEO),
        b"\x7fELF pretend this is a libretro core",
    );
}

/// 装好核心 + 写好记录 = 一次完整的平台包安装。
fn install_pack(root: &Path, platforms: &[&str]) {
    hand_copy_core(root);
    put_record(root, &record(&[pack("platform", platforms, &[FBNEO])]));
}

fn put_record(root: &Path, text: &str) {
    write_file(&root.join(RECORD_REL), text.as_bytes());
}

fn remove_record(root: &Path) {
    std::fs::remove_file(root.join(RECORD_REL)).unwrap();
}

fn pack(kind: &str, platforms: &[&str], cores: &[&str]) -> serde_json::Value {
    json!({
        "id": "platform-arcade",
        "kind": kind,
        "version": "1.0.0",
        "platforms": platforms,
        "cores": cores,
    })
}

/// `{"packs":[...]}` 外形（Phase 2 记录文件最可能的形状）。
fn record(packs: &[serde_json::Value]) -> String {
    json!({ "schema": 1, "packs": packs }).to_string()
}

fn builtin() -> Vec<Platform> {
    Platform::BUILTIN.to_vec()
}

fn hosted_carts(root: &Path) -> Vec<String> {
    scan_quiet(root)
        .unwrap()
        .into_iter()
        .filter(|cart| platform_packs::is_hosted(cart.platform))
        .map(|cart| format!("{}/{}.zip", cart.platform.dir_name(), cart.stem))
        .collect()
}

// ---------------------------------------------------------------------------
// 验收点 1：没包不识别（核心在卡上也不算）。
// ---------------------------------------------------------------------------

#[test]
fn qa01_no_record_means_no_platform_even_with_the_core_on_the_card() {
    let card = card();
    hand_copy_core(card.path());

    assert_eq!(
        available_platforms(card.path()),
        builtin(),
        "没有 installed.json 时扫描列表必须只有内置六台"
    );
    assert_eq!(available_platforms(card.path()).len(), 6);
    assert!(!available_platforms(card.path()).contains(&Platform::Arcade));
    assert!(!available_platforms(card.path()).contains(&Platform::Neogeo));

    let carts = scan_quiet(card.path()).unwrap();
    assert_eq!(carts.len(), 0, "没装包时不该扫出任何 Cart：{carts:?}");
    assert_eq!(hosted_carts(card.path()).len(), 0, "Arcade/Neogeo 的 Cart 数必须是 0");
    // 目录确实存在、ZIP 确实在：沉默的原因只能是激活门，不是文件缺失。
    assert!(card.path().join("Games/Arcade/a.zip").is_file());
    assert!(card.path().join("Games/NeoGeo/b.zip").is_file());
}

#[test]
fn qa01_the_scan_report_does_not_promise_arcade_while_it_is_dormant() {
    let card = card();
    hand_copy_core(card.path());
    scan(card.path()).unwrap();
    let log = std::fs::read_to_string(card.path().join("linkSlot/Logs/slot-library.log")).unwrap();

    assert!(
        !log.contains("Arcade"),
        "报告承诺了没扫的目录，用户会以为丢游戏：\n{log}"
    );
    assert!(!log.contains("NeoGeo"));
    assert!(log.contains("Games/GBA"), "报告漏了内置机种：\n{log}");
    assert!(!log.contains(".zip"), "没装包时 .zip 不该出现在支持列表里：\n{log}");
}

// ---------------------------------------------------------------------------
// 验收点 2：裸 .so 不激活。
// ---------------------------------------------------------------------------

#[test]
fn qa02_a_hand_copied_core_activates_nothing() {
    let card = card();
    hand_copy_core(card.path());

    assert!(!platform_packs::active(card.path(), Platform::Arcade));
    assert!(!platform_packs::active(card.path(), Platform::Neogeo));
    assert!(platform_packs::active_platforms(card.path()).is_empty());
    // 核心文件真的在卡上且长度 > 0：false 不是因为"没找到文件"。
    assert!(core_file(card.path(), FBNEO).is_file());
    assert!(core_file(card.path(), FBNEO).metadata().unwrap().len() > 0);
    // 只有街机 / NeoGeo 才受激活门约束。
    assert!(platform_packs::is_hosted(Platform::Arcade));
    assert!(platform_packs::is_hosted(Platform::Neogeo));
    assert!(!platform_packs::is_hosted(Platform::Md));
    assert!(!platform_packs::active(card.path(), Platform::Md));
}

// ---------------------------------------------------------------------------
// 验收点 3：一个包点亮两个平台。
// ---------------------------------------------------------------------------

#[test]
fn qa03_one_pack_lights_both_platforms() {
    let card = card();
    install_pack(card.path(), &["arcade", "neogeo"]);

    assert!(platform_packs::active(card.path(), Platform::Arcade));
    assert!(platform_packs::active(card.path(), Platform::Neogeo));
    assert_eq!(
        platform_packs::active_platforms(card.path()),
        vec![Platform::Arcade, Platform::Neogeo]
    );
    let mut expected = builtin();
    expected.push(Platform::Arcade);
    expected.push(Platform::Neogeo);
    assert_eq!(available_platforms(card.path()), expected);

    let carts = scan_quiet(card.path()).unwrap();
    assert_eq!(carts.len(), 2, "一个包要同时点亮两个机种：{carts:?}");
    assert_eq!(carts[0].platform, Platform::Arcade);
    assert_eq!(carts[0].stem, "a");
    assert!(carts[0].rom.ends_with("Games/Arcade/a.zip") || carts[0].rom.ends_with("Games\\Arcade\\a.zip"));
    assert_eq!(carts[1].platform, Platform::Neogeo);
    assert_eq!(carts[1].stem, "b");
    // 记录被读懂了，不只是"没读到"。
    let records = platform_packs::records(card.path());
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].kind, "platform");
    assert_eq!(records[0].platforms, vec!["arcade", "neogeo"]);
    assert_eq!(records[0].cores, vec![FBNEO]);
}

// ---------------------------------------------------------------------------
// 验收点 4：半装不亮（记录写了但 .so 没落地）。
// ---------------------------------------------------------------------------

#[test]
fn qa04_a_missing_core_file_keeps_both_platforms_dark() {
    let card = card();
    // 记录写进去了，.so 没拷成功（拔卡 / 空间不足）。
    put_record(
        card.path(),
        &record(&[pack("platform", &["arcade", "neogeo"], &[FBNEO])]),
    );

    assert!(!platform_packs::active(card.path(), Platform::Arcade));
    assert!(!platform_packs::active(card.path(), Platform::Neogeo));
    assert_eq!(available_platforms(card.path()), builtin());
    assert_eq!(hosted_carts(card.path()).len(), 0);
    assert!(available_options(card.path(), Platform::Arcade).is_empty());
    assert!(available_options(card.path(), Platform::Neogeo).is_empty());
}

#[test]
fn qa04_a_zero_byte_core_keeps_both_platforms_dark() {
    let card = card();
    put_record(
        card.path(),
        &record(&[pack("platform", &["arcade", "neogeo"], &[FBNEO])]),
    );
    // 0 字节 = 中断的拷贝，dlopen 必然失败。
    write_file(&core_file(card.path(), FBNEO), b"");

    assert!(!platform_packs::active(card.path(), Platform::Arcade));
    assert!(!platform_packs::active(card.path(), Platform::Neogeo));
    assert_eq!(hosted_carts(card.path()).len(), 0);
}

#[test]
fn qa04_a_directory_named_like_a_core_is_not_a_core() {
    let card = card();
    put_record(
        card.path(),
        &record(&[pack("platform", &["arcade", "neogeo"], &[FBNEO])]),
    );
    // 同名目录：存在但不可加载，同样算半装。
    std::fs::create_dir_all(core_file(card.path(), FBNEO)).unwrap();

    assert!(!platform_packs::active(card.path(), Platform::Arcade));
    assert!(!platform_packs::active(card.path(), Platform::Neogeo));
}

/// 符号链接不算核心：悬空链接或指向卡外的链接都不该点亮平台。
/// Windows 上建符号链接需要开发者模式/管理员，整套测试不能因此变红。
#[cfg(unix)]
#[test]
fn qa04_a_symlinked_core_is_not_a_core() {
    let card = card();
    put_record(
        card.path(),
        &record(&[pack("platform", &["arcade", "neogeo"], &[FBNEO])]),
    );
    let outside = tempfile::tempdir().unwrap();
    let real = outside.path().join("fbneo_libretro.so");
    std::fs::write(&real, b"\x7fELF outside the card").unwrap();
    let link = core_file(card.path(), FBNEO);
    std::fs::create_dir_all(link.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(&real, &link).unwrap();

    assert!(
        !platform_packs::active(card.path(), Platform::Arcade),
        "符号链接指向卡外文件却点亮了平台：激活门被绕过"
    );
    assert!(!platform_packs::active(card.path(), Platform::Neogeo));
}

// ---------------------------------------------------------------------------
// 验收点 5：记录只点名一个平台。
// ---------------------------------------------------------------------------

#[test]
fn qa05_a_record_naming_only_arcade_leaves_neogeo_dark() {
    let card = card();
    install_pack(card.path(), &["arcade"]);

    assert!(platform_packs::active(card.path(), Platform::Arcade));
    assert!(
        !platform_packs::active(card.path(), Platform::Neogeo),
        "同一个核心能跑两个机种，但记录只点名了 arcade"
    );
    assert_eq!(
        platform_packs::active_platforms(card.path()),
        vec![Platform::Arcade]
    );
    let mut expected = builtin();
    expected.push(Platform::Arcade);
    assert_eq!(available_platforms(card.path()), expected);

    let carts = scan_quiet(card.path()).unwrap();
    assert_eq!(carts.len(), 1, "{carts:?}");
    assert_eq!(carts[0].platform, Platform::Arcade);
    assert_eq!(carts[0].stem, "a");
}

// ---------------------------------------------------------------------------
// 验收点 6：卸载即恢复沉默，且不删 ROM。
// ---------------------------------------------------------------------------

#[test]
fn qa06_uninstall_brings_back_the_silence_and_keeps_the_roms() {
    let card = card();
    install_pack(card.path(), &["arcade", "neogeo"]);
    assert_eq!(hosted_carts(card.path()).len(), 2, "装好之后必须看得见两台");

    // 卸载 = 删记录（.so 与 ROM 是否留在卡上由安装器决定，激活门只认记录）。
    remove_record(card.path());

    assert_eq!(
        available_platforms(card.path()),
        builtin(),
        "删掉记录后必须回到内置六台"
    );
    assert!(!platform_packs::active(card.path(), Platform::Arcade));
    assert!(!platform_packs::active(card.path(), Platform::Neogeo));
    assert_eq!(hosted_carts(card.path()).len(), 0, "卸载后不该再扫出街机卡带");

    // 卸载不删用户 ROM：识别与否是激活门的事，文件一个都不能少。
    assert!(
        card.path().join("Games/Arcade/a.zip").is_file(),
        "卸载把用户 ROM 删掉了"
    );
    assert!(card.path().join("Games/NeoGeo/b.zip").is_file());
    // 再装回来：同一个目录，不改任何文件，只补回记录。
    install_pack(card.path(), &["arcade", "neogeo"]);
    assert_eq!(hosted_carts(card.path()).len(), 2, "重装必须原样复活");
}

// ---------------------------------------------------------------------------
// 验收点 7：记录形状。
// ---------------------------------------------------------------------------

#[test]
fn qa07_the_packs_envelope_shape_activates() {
    let card = card();
    hand_copy_core(card.path());
    // {"packs":[...]} 外形。同时带上一堆 Phase 2 才会写的未知字段：
    // serde 忽略未声明键，多字段不该让记录读崩。
    put_record(
        card.path(),
        &json!({
            "schema": 1,
            "written_by": "installer 2.0",
            "packs": [{
                "id": "platform-arcade",
                "kind": "platform",
                "name": "Arcade / NeoGeo",
                "version": "1.0.0",
                "installed_at": 1_700_000_000u64,
                "platforms": ["arcade", "neogeo"],
                "cores": ["fbneo"],
                "files": ["System/linkSlot/cores/fbneo/fbneo_libretro.so"],
            }],
        })
        .to_string(),
    );

    assert!(platform_packs::active(card.path(), Platform::Arcade));
    assert!(platform_packs::active(card.path(), Platform::Neogeo));
    assert_eq!(hosted_carts(card.path()).len(), 2);
}

#[test]
fn qa07_a_bare_array_record_file_activates() {
    let card = card();
    hand_copy_core(card.path());
    // 裸数组外形（Phase 1 没有安装器，两种外形都得认）。
    put_record(
        card.path(),
        &json!([{
            "id": "platform-arcade",
            "kind": "platform",
            "platforms": ["NeoGeo"],
            "cores": ["fbneo"],
        }])
        .to_string(),
    );

    assert!(platform_packs::active(card.path(), Platform::Neogeo));
    assert!(!platform_packs::active(card.path(), Platform::Arcade));
}

#[test]
fn qa07_a_core_pack_record_never_lights_a_platform() {
    let card = card();
    hand_copy_core(card.path());
    // 核心包记录也可能带 platforms 字段（它本来就描述了核心能跑哪些机种）：
    // 只有 kind=="platform" 才允许点亮货架。
    put_record(
        card.path(),
        &record(&[
            pack("core", &["arcade", "neogeo"], &[FBNEO]),
            pack("achievements", &["arcade"], &[FBNEO]),
            pack("game", &["neogeo"], &[FBNEO]),
        ]),
    );

    assert!(!platform_packs::active(card.path(), Platform::Arcade));
    assert!(!platform_packs::active(card.path(), Platform::Neogeo));
    assert_eq!(available_platforms(card.path()), builtin());
    assert_eq!(hosted_carts(card.path()).len(), 0);
    // 记录本身是能读到的三条，只是类型不对。
    assert_eq!(platform_packs::records(card.path()).len(), 3);
}

#[test]
fn qa07_unreadable_records_activate_nothing_and_never_panic() {
    // 每一种"读不懂"的记录都必须等价于"没装包"，而不是清空游戏库或崩溃。
    let cases: [(&str, &str); 6] = [
        ("truncated", "{ \"packs\": [ { \"kind\": \"platform\", \"platf"),
        ("not json", "this is not json at all\n"),
        ("empty object", "{}"),
        ("empty array", "[]"),
        ("wrong packs type", "{\"packs\": {\"arcade\": \"yes\"}}"),
        ("null", "null"),
    ];
    for (name, text) in cases {
        let card = card();
        hand_copy_core(card.path());
        put_record(card.path(), text);

        assert!(
            !platform_packs::active(card.path(), Platform::Arcade),
            "{name} 形态的记录点亮了 Arcade"
        );
        assert!(!platform_packs::active(card.path(), Platform::Neogeo), "{name}");
        assert_eq!(
            available_platforms(card.path()),
            builtin(),
            "{name} 形态的记录污染了扫描列表"
        );
        assert_eq!(hosted_carts(card.path()).len(), 0, "{name}");
        // 内置机种照常工作：坏记录不能把整台机器的游戏库清空。
        write_file(&card.path().join("Games/GBA/Emerald.gba"), b"AGB- fake");
        let carts = scan_quiet(card.path()).unwrap();
        assert_eq!(carts.len(), 1, "{name} 让内置机种也失明了：{carts:?}");
        assert_eq!(carts[0].platform, Platform::Gba);
        // 会写日志的那条路径同样不能 panic。
        assert!(scan(card.path()).is_ok(), "{name} 让 scan() 出错了");
    }
}

#[test]
fn qa07_a_record_without_platforms_or_cores_activates_nothing() {
    let card = card();
    hand_copy_core(card.path());
    // Phase 2 之前手写的记录可能缺字段：缺一个都不算装好。
    put_record(
        card.path(),
        &record(&[
            json!({"id": "a", "kind": "platform"}),
            json!({"id": "b", "kind": "platform", "platforms": ["arcade"]}),
            json!({"id": "c", "kind": "platform", "cores": [FBNEO]}),
        ]),
    );

    assert!(!platform_packs::active(card.path(), Platform::Arcade));
    assert!(!platform_packs::active(card.path(), Platform::Neogeo));
}

#[test]
fn qa07_a_record_path_that_is_a_directory_activates_nothing() {
    // 读 `installed.json` 失败（这里是"同名目录"）只能退化成"没装包"。
    let card = card();
    hand_copy_core(card.path());
    std::fs::create_dir_all(card.path().join(RECORD_REL)).unwrap();

    assert!(!platform_packs::active(card.path(), Platform::Arcade));
    assert_eq!(available_platforms(card.path()), builtin());
    assert!(platform_packs::records(card.path()).is_empty());
}

// ---------------------------------------------------------------------------
// 验收点 8：路径安全 —— 记录里的核心名不能把检查指到 cores 目录外。
// ---------------------------------------------------------------------------

#[test]
fn qa08_core_names_cannot_escape_the_cores_directory() {
    let card = card();
    let root = card.path();
    // 先把每个"如果没做白名单就会命中的路径"真的造出来。这样 false 只能来自
    // 名字校验，而不是"文件恰好不存在"。
    let decoys: [&str; 4] = [
        // core="."      → cores/._libretro.<ext>
        "System/linkSlot/cores/._libretro.so",
        // core=".."     → System/linkSlot/.._libretro.<ext>
        "System/linkSlot/.._libretro.so",
        // core="a/b"    → cores/a/b/b_libretro.<ext>
        "System/linkSlot/cores/a/b/b_libretro.so",
        // core="../../etc" → <root>/etc_libretro.<ext>
        "etc_libretro.so",
    ];
    for rel in decoys {
        // 扩展名在 Windows 上是 .dll，两种都造一遍，省得条件编译散落在断言里。
        let path = root.join(rel);
        let with_dll = path.with_extension("dll");
        write_file(&path, b"\x7fELF decoy");
        write_file(&with_dll, b"\x7fELF decoy");
    }
    // 顺带造一个卡外的诱饵：core="../../../../pwned" 之类若被拼接就会命中这里。
    let outside = tempfile::tempdir().unwrap();
    write_file(&outside.path().join("etc_libretro.so"), b"\x7fELF decoy");

    let bad_names = [
        "../../etc",
        "..",
        ".",
        "a/b",
        "../fbneo",
        "/etc/passwd",
        "a\\b",
        "..\\fbneo",
        "",
        "   ",
    ];
    for name in bad_names {
        put_record(root, &record(&[pack("platform", &["arcade", "neogeo"], &[name])]));
        assert!(
            !platform_packs::active(root, Platform::Arcade),
            "cores=[{name:?}] 把 Arcade 点亮了（检查被指到 cores 目录外）"
        );
        assert!(
            !platform_packs::active(root, Platform::Neogeo),
            "cores=[{name:?}] 把 NeoGeo 点亮了"
        );
        assert_eq!(hosted_carts(root).len(), 0, "cores=[{name:?}] 让扫描器看见了街机");
    }

    // Windows 不允许文件名里出现 ':'，这个用例只造诱饵、不比不可用路径。
    put_record(root, &record(&[pack("platform", &["arcade"], &["a:b"])]));
    assert!(
        !platform_packs::active(root, Platform::Arcade),
        "cores=[\"a:b\"] 把 Arcade 点亮了（在 Linux 上 ':' 是合法文件名字符）"
    );

    // 阳性对照：同样的夹具，换成合法核心名必须点亮。否则上面全是假阴性。
    put_record(root, &record(&[pack("platform", &["arcade"], &[FBNEO])]));
    hand_copy_core(root);
    assert!(
        platform_packs::active(root, Platform::Arcade),
        "合法核心名都不亮，说明这个夹具本身失效了"
    );
}

// ---------------------------------------------------------------------------
// 验收点 9：ZIP 不读 ROM 头。
// ---------------------------------------------------------------------------

/// 一段在 GBA 与 GB 两个偏移上都写着可读标题的字节：只要扫描器/货架把 ZIP 当
/// 卡带去解析，`POKEMON EMER` 或 `TETRIS` 就会漏进 `Cart.title`。
fn header_soup() -> Vec<u8> {
    let mut bytes = vec![0u8; 0x200];
    bytes[0xA0..0xA0 + 12].copy_from_slice(b"POKEMON EMER"); // GBA title
    bytes[0xAC..0xB0].copy_from_slice(b"BPEE"); // GBA code
    bytes[0x134..0x134 + 11].copy_from_slice(b"TETRIS\0\0\0\0\0"); // GB title
    bytes
}

#[test]
fn qa09_an_arcade_zip_never_gets_a_rom_header_title() {
    let card = card();
    install_pack(card.path(), &["arcade", "neogeo"]);
    let zip = card.path().join("Games/Arcade/Header Soup.zip");
    write_file(&zip, &header_soup());

    let carts = scan_quiet(card.path()).unwrap();
    let mut arcade = carts
        .into_iter()
        .find(|cart| cart.rom == zip || cart.stem == "Header Soup")
        .expect("街机 ZIP 没被扫出来");
    assert_eq!(arcade.platform, Platform::Arcade);
    assert_eq!(arcade.stem, "Header Soup", "名字只由文件名决定");
    // Phase 3 起，街机的 `title` 是 DAT 的标题，缺 DAT 时退化成文件名 stem
    // （任务书 D-4）。没装 DAT 的卡上因此应当**恰好**等于文件名——这比旧契约
    // 的 `""` 更硬：夹具在 GBA 与 GB 两个偏移都写了可读标题，只要扫描器真的去
    // 解 ZIP 的"头"，这里就会是 `POKEMON EMER` 或 `TETRIS`。
    assert_eq!(
        arcade.title, "Header Soup",
        "没装 DAT 时标题应当等于文件名，而不是 ZIP 字节里的可读头"
    );
    assert!(
        arcade.title != "POKEMON EMER" && arcade.title != "TETRIS",
        "ZIP 字节里的 ROM 头漏进了标题：{:?}",
        arcade.title
    );
    assert_eq!(arcade.code, "");

    // 把标题预置成"文件名 stem"，再走一遍货架会调用的 `read_header`：
    // 这一关过了，才谈得上"标题保持文件名"。
    arcade.title = arcade.stem.clone();
    arcade.code = "ABCD".into();
    arcade.read_header();
    assert_eq!(
        arcade.title, "Header Soup",
        "ZIP 的标题被 ROM 头解析改写成 {:?}",
        arcade.title
    );
    assert_eq!(arcade.code, "ABCD", "ZIP 的 code 被 ROM 头解析改写了");

    // 同一段字节放进 Games/GBA 就是真卡带：这证明上面那句 assert 不是
    // 因为字节里根本没有可解析的头。
    let gba = card.path().join("Games/GBA/Header Soup.gba");
    write_file(&gba, &header_soup());
    let mut real_cart = scan_quiet(card.path())
        .unwrap()
        .into_iter()
        .find(|cart| cart.rom == gba)
        .expect("GBA 假卡带没被扫出来");
    real_cart.read_header();
    assert_eq!(real_cart.title, "POKEMON EMER", "阳性对照失效：GBA 头没被解析");
    assert_eq!(real_cart.code, "BPEE");
}

// ---------------------------------------------------------------------------
// 验收点 10：回归 —— 内置六台的行为与改动前一致。
// ---------------------------------------------------------------------------

/// 内置六台在"卡上什么都没有"时的选择器结果（改动前的语义：全部缺失时回退
/// 到机种默认核心，UI 不为空）。
#[test]
fn qa10_builtin_core_options_fall_back_to_the_default_core() {
    let card = tempfile::tempdir().unwrap();
    let cases = [
        (Platform::Gba, Core::Mgba),
        (Platform::Gb, Core::Mgba),
        (Platform::Gbc, Core::Mgba),
        (Platform::Fc, Core::Nes),
        (Platform::Sfc, Core::Snes9x),
        (Platform::Md, Core::Gpgx),
    ];
    for (platform, default) in cases {
        assert_eq!(
            available_options(card.path(), platform),
            vec![default],
            "{platform:?} 的默认核心回退变了"
        );
    }
}

#[test]
fn qa10_builtin_core_options_are_unchanged_and_never_offer_fbneo() {
    let card = card();
    // 极端情形：九颗核心全在卡上（包括 FBNeo），并且平台包已激活。
    for core in Core::EVERY {
        write_file(&core_file(card.path(), core.as_str()), b"\x7fELF core");
    }
    install_pack(card.path(), &["arcade", "neogeo"]);

    let cases: [(Platform, Vec<Core>); 6] = [
        (Platform::Gba, vec![Core::Mgba, Core::Gpsp, Core::Armgba]),
        (Platform::Gb, vec![Core::Mgba]),
        (Platform::Gbc, vec![Core::Mgba]),
        (Platform::Fc, vec![Core::Nes, Core::Nestopia]),
        (Platform::Sfc, vec![Core::Snes9x, Core::Snes]),
        (Platform::Md, vec![Core::Gpgx]),
    ];
    for (platform, expected) in cases {
        let options = available_options(card.path(), platform);
        assert_eq!(options, expected, "{platform:?} 的选择器被 FBNeo 污染了");
        assert!(
            !options.contains(&Core::Fbneo),
            "FBNeo 出现在 {platform:?} 的选择器里：{options:?}"
        );
    }
    // 街机自己：激活后正好是 FBNeo 一颗。
    assert_eq!(
        available_options(card.path(), Platform::Arcade),
        vec![Core::Fbneo]
    );
    assert_eq!(
        available_options(card.path(), Platform::Neogeo),
        vec![Core::Fbneo]
    );
}

#[test]
fn qa10_every_core_maps_one_to_one_with_the_model_table() {
    assert_eq!(
        Core::EVERY.len(),
        slot_model::CORES.len(),
        "Core::EVERY 与 slot_model::CORES 数量不一致"
    );
    for core in Core::EVERY {
        let spec = slot_model::core(core.id())
            .unwrap_or_else(|| panic!("{core:?} 的 id {:?} 不在 CORES 里", core.id()));
        assert_eq!(spec.id.as_str(), core.id(), "{core:?} 的 id 与表项不符");
        assert_eq!(core.spec().id.as_str(), core.id());
        assert_eq!(
            Core::parse(core.id()),
            Some(core),
            "{:?} 的 id 不能反解析回自己",
            core.id()
        );
        assert_eq!(core.index(), core as usize, "{core:?} 的 index 不是判别值");
    }
    for spec in slot_model::CORES {
        assert!(
            Core::parse(spec.id.as_str()).is_some(),
            "CORES 里的 {} 在 slot-store::Core 里没有对应变体",
            spec.id.as_str()
        );
    }
    // FBNeo 的能力：街机 + NeoGeo，且（暂无）别的机种。
    assert!(Core::Fbneo.runs(Platform::Arcade));
    assert!(Core::Fbneo.runs(Platform::Neogeo));
    assert!(!Core::Fbneo.runs(Platform::Md));
    assert_eq!(Core::options_for(Platform::Arcade), [Core::Fbneo].as_slice());
    assert_eq!(Core::options_for(Platform::Neogeo), [Core::Fbneo].as_slice());
    assert_eq!(Core::default_for(Platform::Arcade), Core::Fbneo);
    assert_eq!(Core::default_for(Platform::Neogeo), Core::Fbneo);
}

#[test]
fn qa10_builtin_is_the_first_six_of_all_and_the_arcade_slots_are_appended() {
    assert_eq!(Platform::ALL.len(), 8);
    assert_eq!(&Platform::ALL[..6], &Platform::BUILTIN[..]);
    assert_eq!(slot_model::MACHINES.len(), 8);
    for (i, platform) in Platform::ALL.iter().enumerate() {
        assert_eq!(platform.index(), i, "{platform:?} 的落盘槽位被重排了");
        assert_eq!(
            platform.machine().id.as_str(),
            slot_model::MACHINES[i].id.as_str()
        );
    }
    assert_eq!(Platform::Arcade.index(), 6);
    assert_eq!(Platform::Neogeo.index(), 7);
    assert_eq!(Platform::Arcade as usize, 6);
    assert_eq!(Platform::Neogeo as usize, 7);
    assert!(!Platform::BUILTIN.contains(&Platform::Arcade));
    assert!(!Platform::BUILTIN.contains(&Platform::Neogeo));

    assert_eq!(Platform::Arcade.dir_name(), "Arcade");
    assert_eq!(Platform::Neogeo.dir_name(), "NeoGeo");
    assert_eq!(Platform::Arcade.extensions(), &["zip"]);
    assert_eq!(Platform::Neogeo.extensions(), &["zip"]);
    assert!(Platform::Arcade.accepts(Path::new("sf2.zip")));
    assert!(!Platform::Arcade.accepts(Path::new("sf2.7z")));

    // 本阶段不做街机柜：美术资源必须沿用既有文件名，不能偷偷引入新素材。
    let gba = Platform::Gba.machine().art;
    for platform in [Platform::Arcade, Platform::Neogeo] {
        let art = platform.machine().art;
        assert_eq!(art.board, gba.board, "{platform:?} 引入了新的机种美术");
        assert_eq!(art.cart, gba.cart);
        assert_eq!(art.label, gba.label);
        assert_eq!(art.slot_back, gba.slot_back);
        assert!(platform.machine().quick.is_empty());
    }
}

#[test]
fn qa10_builtin_scanning_is_unchanged_when_a_pack_is_active() {
    let card = card();
    for (dir, stem) in [
        ("GBA", "Emerald.gba"),
        ("GB", "Tetris.gb"),
        ("GBC", "Tetris DX.gbc"),
        ("FC", "Contra.nes"),
        ("SFC", "Mario.sfc"),
        ("MD", "Sonic.md"),
    ] {
        write_file(&card.path().join("Games").join(dir).join(stem), b"rom");
    }
    // 没有包：六台照常（.zip 不算任何内置机种的 ROM）。
    let dormant: Vec<String> = scan_quiet(card.path())
        .unwrap()
        .into_iter()
        .map(|cart| format!("{}/{}", cart.platform.dir_name(), cart.stem))
        .collect();
    assert_eq!(dormant.len(), 6, "{dormant:?}");

    // 有包：内置六台一条不多一条不少，街机两台追加在后。
    install_pack(card.path(), &["arcade", "neogeo"]);
    let carts = scan_quiet(card.path()).unwrap();
    let names: Vec<String> = carts
        .iter()
        .map(|cart| format!("{}/{}", cart.platform.dir_name(), cart.stem))
        .collect();
    assert_eq!(carts.len(), 8, "{names:?}");
    assert_eq!(&names[..6], &dormant[..], "内置机种的扫描结果被改变了");
    assert_eq!(names[6], "Arcade/a");
    assert_eq!(names[7], "NeoGeo/b");
}

// ---------------------------------------------------------------------------
// 验收点 11（Phase 3 契约）：DAT 决定标题，缺 DAT 退化成文件名。
// ---------------------------------------------------------------------------

/// `qa09` 的另一半：它只证明"没 DAT 时标题不会被 ZIP 字节改写"，如果 DAT 整条
/// 链路坏掉（永远返回空表），`qa09` 依然会绿而"标题来自 DAT"这个功能已经死了。
/// 所以这里从反方向钉住：**有 DAT 时标题必须换成 DAT 的标题**，且 BIOS 集合
/// 不占书架 —— 两条都只用扫描结果判断，不碰内部类型。
#[test]
fn qa11_a_dat_title_replaces_the_file_name_and_bios_sets_do_not() {
    let dir = card();
    let root = dir.path();
    install_pack(root, &["arcade"]);
    // BIOS 与一个真游戏同目录：真实卡上 neogeo.zip 就是这么放的。
    write_file(&root.join("Games/Arcade/neogeo.zip"), b"PK\x03\x04 bios");
    // DAT 的位置由平台包记录推导：extensions/platforms/<记录 id>/metadata/<平台小写>.dat.json。
    let dat = root
        .join("System/linkSlot/extensions/platforms/platform-arcade/metadata/arcade.dat.json");
    write_file(
        &dat,
        json!({
            "schema": 1,
            "platform": "arcade",
            "sets": [
                {"set": "a", "title": "Alpha Attack", "system": "CPS-1", "year": "1991"},
                {"set": "neogeo", "title": "NeoGeo BIOS", "system": "NEOGEO", "is_bios": true},
            ],
        })
        .to_string()
        .as_bytes(),
    );

    let listed = |root: &Path| -> Vec<(String, String)> {
        scan_quiet(root)
            .unwrap()
            .into_iter()
            .filter(|cart| cart.platform == Platform::Arcade)
            .map(|cart| (cart.stem, cart.title))
            .collect()
    };

    // 有 DAT：标题来自 DAT；BIOS 不出现。
    assert_eq!(
        listed(root),
        vec![("a".to_string(), "Alpha Attack".to_string())],
        "DAT 标题没生效，或 BIOS 集合占了书架"
    );

    // 删掉 DAT：DAT 只做增强不做门 —— 游戏还在，标题退回文件名。
    std::fs::remove_file(&dat).unwrap();
    let after = listed(root);
    let alpha = after
        .iter()
        .find(|(stem, _)| stem == "a")
        .unwrap_or_else(|| panic!("缺 DAT 时游戏被吞了：{after:?}"));
    assert_eq!(
        alpha.1, "a",
        "缺 DAT 时标题没退回文件名，而是 {:?}",
        alpha.1
    );
}
