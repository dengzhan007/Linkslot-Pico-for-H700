//! Phase 5b-1「START → 进 Playing → 退出回主界面」的**独立验收**（QA）。
//!
//! 只放测试代码，由 `lib.rs` 的 `#[cfg(test)] mod arcade_launch_qa;` 挂进来。
//!
//! 这一版能纯逻辑验的两块：
//! 1. `arcade::launch_plan(root, platform, stem, title)`（纯函数）——交给核心的
//!    `rom` / `system_dir` / `canvas`；
//! 2. `ArcadeScreen` 的 `core_missing` 标记——**只有成功的状态迁移才清**，
//!    被拒绝的 `insert(0)` / `start()` 不许把它抹掉（否则"缺核心红字"会一闪就没）。
//!
//! 只能在真机验的（本机 Windows + aarch64 `.so` 只会走 `MockCore`），本文件不假装测：
//! FBNeo 真实加载 / `av_info` 分辨率 / `BIOS/fbneo/neogeo.zip` 真被读 / 街机几何与
//! TATE / 设备上 `core_missing` 红字 / 真核心启动延迟与扣币的交互。

use std::path::Path;

use slot_store::Platform;

use crate::arcade::{
    launch_plan, ArcadeScreen, ArcadeStage, DEFAULT_BIOS_DIR, FBNEO_BIOS_DIR, INSERT_SECS, MAX_TICK,
};

/// 有板时传给 `insert` 的数量。
const BOARDS: usize = 2;

fn new_screen() -> ArcadeScreen {
    ArcadeScreen::new(Platform::Arcade)
}

/// 推进指定秒数；`tick` 把单帧夹到 `MAX_TICK`，必须分多帧喂。
fn tick_for(screen: &mut ArcadeScreen, secs: f32) {
    let mut left = secs;
    while left > 0.0 {
        let step = left.min(MAX_TICK);
        screen.tick(step);
        left -= step;
    }
}

// ---------------------------------------------------------------- ① launch_plan：路径

/// 交给核心的 `rom` 必须正好落在**扫描布局**上（`Games/<平台目录>/<stem>.zip`），
/// 否则核心会去打不开的文件，而 UI 侧一切正常——这种错在设备上只表现为黑屏。
#[test]
fn launch_plan_points_the_cart_at_the_scan_layout() {
    let root = Path::new("/card");
    let plan = launch_plan(root, Platform::Arcade, "mslug", "Metal Slug");

    assert_eq!(plan.cart.platform, Platform::Arcade);
    assert_eq!(plan.cart.stem, "mslug");
    assert_eq!(
        plan.cart.rom,
        root.join("Games/Arcade/mslug.zip"),
        "rom 没有指向 Games/<平台目录>/<stem>.zip"
    );
    assert_eq!(plan.cart.title, "Metal Slug");
    assert_eq!(plan.cart.code, "", "街机没有厂商码，不该被填上");
    assert!(plan.cart.label.is_none() && plan.cart.front.is_none());

    // NeoGeo 用同一个布局规则，只是平台目录不同。
    let neogeo = launch_plan(root, Platform::Neogeo, "mslug", "");
    assert_eq!(neogeo.cart.rom, root.join("Games/NeoGeo/mslug.zip"));
}

/// 标题为空（或全空白）时退回 `stem`；有标题时去掉首尾空白。
#[test]
fn launch_plan_falls_back_to_the_stem_for_a_blank_title() {
    let root = Path::new("/card");
    assert_eq!(launch_plan(root, Platform::Arcade, "sf2", "").cart.title, "sf2");
    assert_eq!(
        launch_plan(root, Platform::Arcade, "sf2", "   ").cart.title,
        "sf2",
        "全空白标题应退回 stem"
    );
    assert_eq!(
        launch_plan(root, Platform::Arcade, "sf2", "  Street Fighter II  ").cart.title,
        "Street Fighter II",
        "标题应当去掉首尾空白"
    );
}

// ---------------------------------------------------------------- ② launch_plan：BIOS 目录

/// `system_dir` **永远有值**：优先 `BIOS/fbneo`（扩展包把 BIOS 放这里），否则退回
/// `BIOS`。返回 `None` 会让核心自己去猜默认目录，缺 BIOS 时的报错就不再是"文件不存在"。
#[test]
fn launch_plan_always_gives_the_core_a_bios_dir() {
    let root = Path::new("/card");

    // 两个目录都不存在：仍然给默认的 `BIOS`，不是 `None`。
    let plan = launch_plan(root, Platform::Neogeo, "mslug", "Metal Slug");
    assert_eq!(
        plan.system_dir,
        Some(root.join(DEFAULT_BIOS_DIR)),
        "没有 BIOS 目录时也应当给出默认 BIOS 目录（让核心自己报缺）"
    );

    // 真实磁盘：造出 `BIOS/fbneo/`，应当改用它。
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join(FBNEO_BIOS_DIR)).unwrap();
    let plan = launch_plan(tmp.path(), Platform::Neogeo, "mslug", "Metal Slug");
    assert_eq!(
        plan.system_dir,
        Some(tmp.path().join(FBNEO_BIOS_DIR)),
        "BIOS/fbneo 存在时应当优先用它"
    );

    // `BIOS/fbneo` 是**文件**而不是目录时不该被当成目录用。
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("BIOS")).unwrap();
    std::fs::write(tmp.path().join(FBNEO_BIOS_DIR), b"not a dir").unwrap();
    let plan = launch_plan(tmp.path(), Platform::Neogeo, "mslug", "Metal Slug");
    assert_eq!(
        plan.system_dir,
        Some(tmp.path().join(DEFAULT_BIOS_DIR)),
        "BIOS/fbneo 是文件时应当退回 BIOS/"
    );
}

// ---------------------------------------------------------------- ③ launch_plan：canvas

/// canvas 取自机台表：Arcade 384×224、NeoGeo 320×224。
#[test]
fn launch_plan_uses_the_machine_table_canvas() {
    let root = Path::new("/card");
    assert_eq!(
        launch_plan(root, Platform::Arcade, "sf2", "").canvas,
        (384, 224)
    );
    assert_eq!(
        launch_plan(root, Platform::Neogeo, "mslug", "").canvas,
        (320, 224),
        "NeoGeo 的 canvas 应当取机台表的 320×224（4b 信息卡文案写 304 的是可视区）"
    );
}

// ---------------------------------------------------------------- ④ core_missing 只在成功迁移时清

/// `note_core_missing()` 之后的标记，只在**成功**的 `insert/start/back` 上清掉。
///
/// 被拒绝的操作（`insert(0)`、0 枚币且非 FREE PLAY 的 `start()`）不许抹掉它：
/// 否则"缺核心"的红字会在用户乱按一次键之后一闪就没，用户根本来不及看。
#[test]
fn core_missing_is_cleared_only_by_a_successful_transition() {
    let mut screen = new_screen();
    assert!(!screen.core_missing(), "初始不该是缺核心");

    // 被拒绝的 `insert(0)` 不能清标记。
    screen.note_core_missing();
    assert!(screen.core_missing());
    assert!(!screen.insert(0), "空板时 insert 应被拒绝");
    assert!(
        screen.core_missing(),
        "被拒绝的 insert(0) 不该清掉 core_missing"
    );

    // 成功的 insert 才清。
    assert!(screen.insert(BOARDS));
    assert!(!screen.core_missing(), "成功的 insert 应当清掉 core_missing");

    tick_for(&mut screen, INSERT_SECS + 0.001);
    assert_eq!(screen.stage(), ArcadeStage::Attract);

    // 被拒绝的 start（非 FREE PLAY 且 0 枚）不能清标记。
    screen.set_free_play(false);
    screen.note_core_missing();
    assert!(!screen.start(), "0 枚币且非 FREE PLAY 时不能开始");
    assert!(
        screen.core_missing(),
        "被拒绝的 start() 不该清掉 core_missing"
    );

    // 成功的 start 才清。
    assert!(screen.coin());
    screen.note_core_missing();
    assert!(screen.start());
    assert!(!screen.core_missing(), "成功的 start 应当清掉 core_missing");

    // back 也清（回主界面后不该还挂着上一局的缺核心红字）。
    screen.set_free_play(true);
    screen.note_core_missing();
    assert!(screen.core_missing());
    assert!(screen.back(), "Attract 下 back 应当成立");
    assert!(!screen.core_missing(), "back 回 Main 应当清掉 core_missing");
    assert_eq!(screen.stage(), ArcadeStage::Main);
}

/// 插板（`Inserting`）期间的 `note_core_missing()` 不该被 `tick` 清掉：
/// 缺核心是**这一局**的事实，要一直显示到用户回到 Main 或重新插板。
#[test]
fn core_missing_survives_ticks_and_stays_until_the_next_transition() {
    let mut screen = new_screen();
    assert!(screen.insert(BOARDS));
    screen.note_core_missing();
    assert!(screen.core_missing());

    // 走完插板动画、进入 Attract：tick 本身不该清标记。
    tick_for(&mut screen, INSERT_SECS + 0.001);
    assert_eq!(screen.stage(), ArcadeStage::Attract);
    assert!(
        screen.core_missing(),
        "tick 不该清掉 core_missing（缺核心是这一局的事实）"
    );

    // 再 tick 一大段也不清。
    tick_for(&mut screen, 5.0);
    assert!(screen.core_missing());
}
