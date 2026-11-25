//! 平台包激活门（B-lite）的护栏。
//!
//! 这里守的是产品约束本身，不是实现细节：**没装包就不识别**。每个用例都从
//! 「卡上有 Games/Arcade/a.zip、有 .so」这么一个"看起来什么都有"的目录出发，
//! 只改变 `installed.json` 一项，然后断言扫描器看不看得见街机。

use std::path::Path;

use slot_store::{
    available_options, available_platforms, platform_packs, scan, scan_quiet, Cart, Core, Platform,
    RECORD_REL,
};
use tempfile::TempDir;

/// 记录文件的路径与常量一致，避免测试自己写死一个拼错的路径。
const RECORD: &str = RECORD_REL;

fn root() -> TempDir {
    let d = tempfile::tempdir().unwrap();
    // 两个机种各放一个"游戏"。内容是假的 ZIP 头，绝不是合法 ROM：如果扫描器
    // 去读 ROM 头，标题就会变成这段字节里的乱码。
    for (platform, stem) in [(Platform::Arcade, "a"), (Platform::Neogeo, "b")] {
        let dir = d.path().join("Games").join(platform.dir_name());
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(format!("{stem}.zip")), b"PK\x03\x04 not a rom").unwrap();
    }
    d
}

/// 手工把某个核心拷进卡里（模拟"用户自己拷了 .so"）。
fn install_core(root: &Path, core: &str) {
    let dir = root.join("System/linkSlot/cores").join(core);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(format!(
            "{core}_libretro.{}",
            std::env::consts::DLL_EXTENSION
        )),
        b"\x7fELF pretend this is a libretro core",
    )
    .unwrap();
}

fn install_fbneo(root: &Path) {
    install_core(root, "fbneo");
}

/// 按**卡格式**（字面量 `.so`）放一个核心：设备上就是这个文件名，桌面上
/// 它不该被 `is_available` 认成"本机可加载的核心"。
fn install_device_core(root: &Path, core: &str) {
    let dir = root.join("System/linkSlot/cores").join(core);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(format!("{core}_libretro.so")),
        b"\x7fELF device-only core",
    )
    .unwrap();
}

fn write_record(root: &Path, json: &str) {
    let path = root.join(RECORD);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, json).unwrap();
}

fn platform_record(platforms: &[&str], cores: &[&str]) -> String {
    let platforms: Vec<String> = platforms.iter().map(|p| format!("\"{p}\"")).collect();
    let cores: Vec<String> = cores.iter().map(|c| format!("\"{c}\"")).collect();
    format!(
        "{{\"schema\":1,\"packs\":[{{\"id\":\"platform-arcade\",\"kind\":\"platform\",\"version\":\"1.0.0\",\"platforms\":[{}],\"cores\":[{}]}}]}}",
        platforms.join(","),
        cores.join(",")
    )
}

fn builtin_only() -> Vec<Platform> {
    Platform::BUILTIN.to_vec()
}

#[test]
fn no_pack_leaves_arcade_and_neogeo_dormant() {
    let d = root();
    // 用户自己拷了 FBNeo：核心在卡上，但没有包记录。
    install_fbneo(d.path());

    assert_eq!(available_platforms(d.path()), builtin_only());
    assert!(
        !available_platforms(d.path()).contains(&Platform::Arcade),
        "没有包记录时 Arcade 不该出现在扫描列表里"
    );

    let carts = scan_quiet(d.path()).unwrap();
    assert!(
        carts.is_empty(),
        "没有包记录时 Games/Arcade 与 Games/NeoGeo 都不该产生 Cart：{carts:?}"
    );
}

#[test]
fn manual_core_without_record_does_not_activate() {
    let d = root();
    install_fbneo(d.path());

    // 裸 .so 不是包：核心文件只是包的一部分，激活依据是记录 + 文件两者。
    assert!(!platform_packs::active(d.path(), Platform::Arcade));
    assert!(!platform_packs::active(d.path(), Platform::Neogeo));
    assert!(platform_packs::active_platforms(d.path()).is_empty());
}

#[test]
fn one_record_activates_both_platforms() {
    let d = root();
    install_fbneo(d.path());
    write_record(
        d.path(),
        &platform_record(&["arcade", "neogeo"], &["fbneo"]),
    );

    assert!(platform_packs::active(d.path(), Platform::Arcade));
    assert!(platform_packs::active(d.path(), Platform::Neogeo));
    assert_eq!(
        available_platforms(d.path()),
        [
            Platform::Gba,
            Platform::Gb,
            Platform::Gbc,
            Platform::Fc,
            Platform::Sfc,
            Platform::Md,
            Platform::Arcade,
            Platform::Neogeo,
        ]
        .to_vec()
    );

    let carts = scan_quiet(d.path()).unwrap();
    assert_eq!(carts.len(), 2, "一个包要同时点亮两个机种：{carts:?}");
    assert_eq!(carts[0].platform, Platform::Arcade);
    assert_eq!(carts[0].stem, "a");
    assert_eq!(carts[1].platform, Platform::Neogeo);
    assert_eq!(carts[1].stem, "b");

    // ZIP 不是卡带：扫描不该去解读 ROM 头。Phase 3 起街机的标题**来自 DAT**
    // （这里是"没有 DAT"的情形），所以标题等于文件名 stem，而不是 ZIP 字节里
    // 那段乱码——文件名兜底与"没读 ROM 头"这两件事同时成立。
    for cart in &carts {
        assert_eq!(
            cart.title, cart.stem,
            "{} 的标题不是文件名兜底，可能是读了 ZIP 的「头」",
            cart.stem
        );
        assert_eq!(cart.code, "");
    }
}

#[test]
fn a_zip_cart_never_reads_a_rom_header() {
    let d = root();
    install_fbneo(d.path());
    write_record(d.path(), &platform_record(&["arcade"], &["fbneo"]));

    // 直接用扫描结果手搓一个 Cart 再调 read_header：这是"标题只由文件名决定"
    // 的最后一个关口，绕过它就等于让扫描器相信 ZIP 里有 GBA 头。
    let carts = scan_quiet(d.path()).unwrap();
    assert_eq!(carts.len(), 1);
    let mut cart = Cart {
        title: "Street Fighter II".into(),
        code: String::new(),
        ..carts[0].clone()
    };
    let before = cart.title.clone();
    cart.read_header();
    assert_eq!(cart.title, before, "ZIP 的标题被 ROM 头读取改写了");
    assert_eq!(cart.code, "");
}

#[test]
fn missing_core_file_keeps_platforms_dormant() {
    let d = root();
    // 记录写进去了，但 .so 没拷成功（拔卡 / 空间不足）——半装不算激活。
    write_record(
        d.path(),
        &platform_record(&["arcade", "neogeo"], &["fbneo"]),
    );

    assert!(!platform_packs::active(d.path(), Platform::Arcade));
    assert!(!platform_packs::active(d.path(), Platform::Neogeo));
    assert_eq!(available_platforms(d.path()), builtin_only());
    assert!(scan_quiet(d.path()).unwrap().is_empty());
}

#[test]
fn an_empty_core_file_is_not_a_core() {
    let d = root();
    write_record(
        d.path(),
        &platform_record(&["arcade", "neogeo"], &["fbneo"]),
    );
    // 0 字节的 .so 是中断的拷贝，dlopen 必然失败：长度必须 > 0。
    let dir = d.path().join("System/linkSlot/cores/fbneo");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(format!(
            "fbneo_libretro.{}",
            std::env::consts::DLL_EXTENSION
        )),
        b"",
    )
    .unwrap();

    assert!(!platform_packs::active(d.path(), Platform::Arcade));
    assert!(!platform_packs::active(d.path(), Platform::Neogeo));
}

#[test]
fn record_listing_only_arcade_leaves_neogeo_dormant() {
    let d = root();
    install_fbneo(d.path());
    write_record(d.path(), &platform_record(&["arcade"], &["fbneo"]));

    assert!(platform_packs::active(d.path(), Platform::Arcade));
    assert!(
        !platform_packs::active(d.path(), Platform::Neogeo),
        "同一个核心能跑两个机种，但记录只点名了 arcade"
    );

    let carts = scan_quiet(d.path()).unwrap();
    assert_eq!(carts.len(), 1);
    assert_eq!(carts[0].platform, Platform::Arcade);
}

#[test]
fn a_core_that_cannot_run_the_platform_does_not_activate_it() {
    let d = root();
    // 记录写错了核心名：卡上只有 mGBA，而 mGBA 跑不了街机。若只看"文件存在"，
    // 平台会被点亮，而选择器里一颗核心都挑不出来——货架上就是一条点开必然
    // 报缺核心的街机条目。
    install_core(d.path(), "mgba");
    write_record(d.path(), &platform_record(&["arcade", "neogeo"], &["mgba"]));

    assert!(
        !platform_packs::active(d.path(), Platform::Arcade),
        "cores=[\"mgba\"] 点亮了 Arcade，但它跑不了这台机种"
    );
    assert!(!platform_packs::active(d.path(), Platform::Neogeo));
    assert_eq!(available_platforms(d.path()), builtin_only());
    assert!(
        scan_quiet(d.path()).unwrap().is_empty(),
        "激活门漏了，街机 ZIP 被扫成了 Cart"
    );
    // 与 QA 复现的那对矛盾对齐：没有"已激活但没有可选核心"的空转状态。
    assert!(available_options(d.path(), Platform::Arcade).is_empty());

    // 阳性对照：同一张卡，把记录换成 FBNeo 并把它放好，两台必须亮——否则
    // 上面的 false 可能只是夹具本身失效。
    install_fbneo(d.path());
    write_record(
        d.path(),
        &platform_record(&["arcade", "neogeo"], &["fbneo"]),
    );
    assert!(platform_packs::active(d.path(), Platform::Arcade));
    assert!(platform_packs::active(d.path(), Platform::Neogeo));
    assert_eq!(
        available_options(d.path(), Platform::Arcade),
        vec![Core::Fbneo]
    );
    assert_eq!(scan_quiet(d.path()).unwrap().len(), 2);
}

/// 同一条记录里既有跑不了的核心、也有跑得了的核心时，只要有一颗正确的就够：
/// 记录是"这个包提供了什么"，不该因为多写了一个核心名而整包失效。
#[test]
fn one_runnable_core_is_enough_within_the_same_record() {
    let d = root();
    install_core(d.path(), "mgba");
    install_fbneo(d.path());
    write_record(
        d.path(),
        &platform_record(&["arcade", "neogeo"], &["mgba", "fbneo_libretro"]),
    );

    assert!(platform_packs::active(d.path(), Platform::Arcade));
    assert!(platform_packs::active(d.path(), Platform::Neogeo));
    assert_eq!(scan_quiet(d.path()).unwrap().len(), 2);
}

#[test]
fn available_options_is_empty_until_the_platform_is_active() {
    let d = root();
    install_fbneo(d.path());

    assert!(
        available_options(d.path(), Platform::Arcade).is_empty(),
        "没有包时选择器不能回退到默认核心，否则激活门会被绕过"
    );
    assert!(available_options(d.path(), Platform::Neogeo).is_empty());

    write_record(
        d.path(),
        &platform_record(&["arcade", "neogeo"], &["fbneo"]),
    );
    assert_eq!(
        available_options(d.path(), Platform::Arcade),
        vec![Core::Fbneo]
    );
    assert_eq!(
        available_options(d.path(), Platform::Neogeo),
        vec![Core::Fbneo]
    );
}

#[test]
fn a_corrupt_record_file_activates_nothing() {
    let d = root();
    install_fbneo(d.path());
    write_record(d.path(), "{ this is not json ");

    assert!(!platform_packs::active(d.path(), Platform::Arcade));
    assert!(!platform_packs::active(d.path(), Platform::Neogeo));
    assert_eq!(available_platforms(d.path()), builtin_only());
    assert!(scan_quiet(d.path()).unwrap().is_empty());
    // 顺带确认 `scan`（会写日志的那条路径）也不会 panic，且报告的机种
    // 列表与真正扫过的一致。
    let carts = scan(d.path()).unwrap();
    assert!(carts.is_empty());
    let log = std::fs::read_to_string(d.path().join("linkSlot/Logs/slot-library.log")).unwrap();
    assert!(
        !log.contains("Games/Arcade"),
        "日志不该承诺扫描过 Games/Arcade：{log}"
    );
    assert!(log.contains("Games/GBA"), "日志漏了内置机种：{log}");
}

#[test]
fn a_record_without_platforms_or_cores_activates_nothing() {
    let d = root();
    install_fbneo(d.path());
    // Phase 2 之前手写的记录可能缺字段：缺字段的包一律当作没装。
    write_record(
        d.path(),
        "{\"packs\":[{\"id\":\"half-written\",\"kind\":\"platform\"}]}",
    );

    assert!(!platform_packs::active(d.path(), Platform::Arcade));
    assert!(!platform_packs::active(d.path(), Platform::Neogeo));
}

#[test]
fn only_platform_packs_activate_a_machine() {
    let d = root();
    install_fbneo(d.path());
    // 核心包 / 游戏包也会写记录，但它们不该点亮任何机种。
    write_record(
        d.path(),
        "{\"packs\":[{\"id\":\"fbneo-core\",\"kind\":\"core\",\"cores\":[\"fbneo\"]}]}",
    );

    assert!(!platform_packs::active(d.path(), Platform::Arcade));
    assert!(!platform_packs::active(d.path(), Platform::Neogeo));
}

#[test]
fn a_bare_array_record_file_is_understood() {
    let d = root();
    install_fbneo(d.path());
    // 两种外形都读：Phase 1 还没有安装器，格式只存在于设计文档里。
    write_record(
        d.path(),
        "[{\"id\":\"platform-arcade\",\"kind\":\"platform\",\"platforms\":[\"NeoGeo\"],\"cores\":[\"fbneo\"]}]",
    );

    assert!(platform_packs::active(d.path(), Platform::Neogeo));
    assert!(!platform_packs::active(d.path(), Platform::Arcade));
}

#[test]
fn builtin_platforms_are_always_scanned() {
    let d = root();
    std::fs::create_dir_all(d.path().join("Games/GBA")).unwrap();
    std::fs::write(d.path().join("Games/GBA/Emerald.gba"), b"AGB- fake").unwrap();

    // 没有任何包：内置机种照常工作，激活门只影响街机。
    let platforms = available_platforms(d.path());
    assert_eq!(platforms, builtin_only());
    let carts = scan_quiet(d.path()).unwrap();
    assert_eq!(carts.len(), 1);
    assert_eq!(carts[0].platform, Platform::Gba);
    assert_eq!(carts[0].stem, "Emerald");
}

/// 宿主语义回归：`Core::is_available` 只认宿主扩展名。
///
/// 平台包的 `.so` 是给设备（Linux/aarch64）用的。撤回全局放宽之后，桌面板不该
/// 因此把 Linux 核心当成"本机可加载的核心"——否则 GBA/SFC 的选择器会多出一颗
/// 点开就加载失败的芯片，那和街机没有任何关系。
#[test]
fn the_host_only_core_check_does_not_accept_a_device_so() {
    let d = root();
    // 字面量 `.so`，故意不用 DLL_EXTENSION：这就是设备卡上的文件名。
    install_device_core(d.path(), "gpsp");

    #[cfg(windows)]
    {
        assert!(
            !Core::Gpsp.is_available(d.path()),
            "桌面板把设备用的 .so 当成了可加载核心"
        );
        let options = available_options(d.path(), Platform::Gba);
        assert!(
            !options.contains(&Core::Gpsp),
            "GBA 选择器多出了桌面加载不了的 gpSP：{options:?}"
        );
        // 其它机种保持旧语义：全部缺失时回退到默认核心，UI 不为空。
        assert_eq!(options, vec![Core::Mgba]);
    }

    #[cfg(unix)]
    {
        // Linux 上 `.so` 就是宿主扩展名；macOS 是 `.dylib`，那里这颗 .so 同样
        // 不该算可用，所以只在扩展名一致时断言"相反"的一侧。
        if std::env::consts::DLL_EXTENSION == "so" {
            assert!(Core::Gpsp.is_available(d.path()));
            assert!(available_options(d.path(), Platform::Gba).contains(&Core::Gpsp));
        } else {
            assert!(!Core::Gpsp.is_available(d.path()));
        }
    }
}

/// 无论宿主是什么，平台包激活后街机选择器正好是 FBNeo。
///
/// 这条刻意用卡格式的 `.so`（不是 DLL_EXTENSION）：它证明桌面板上"包里的
/// 设备核心"仍然点亮平台，只是不混进内置机种的选择器。
#[test]
fn an_active_platform_pack_offers_fbneo_on_every_host() {
    let d = root();
    install_device_core(d.path(), "fbneo");
    write_record(
        d.path(),
        &platform_record(&["arcade", "neogeo"], &["fbneo"]),
    );

    assert!(
        platform_packs::active(d.path(), Platform::Arcade),
        "卡格式的 .so 没能点亮平台"
    );
    assert!(platform_packs::active(d.path(), Platform::Neogeo));
    assert_eq!(
        available_options(d.path(), Platform::Arcade),
        vec![Core::Fbneo]
    );
    assert_eq!(
        available_options(d.path(), Platform::Neogeo),
        vec![Core::Fbneo]
    );
}
