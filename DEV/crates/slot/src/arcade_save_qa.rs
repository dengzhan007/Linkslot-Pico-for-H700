//! Phase 5b-2「街机存档路径 + 六键映射」的**独立验收**（QA）。
//!
//! 只放测试代码，由 `lib.rs` 的 `#[cfg(test)] mod arcade_save_qa;` 挂进来。
//!
//! 契约（lead 定稿，显式核心版）：
//! - `save_paths(root, platform, core, stem) -> SavePaths { sram, state_dir }`；
//!   `sram` = `Saves/<平台目录>/<stem>.sav`（文件），`state_dir` =
//!   `States/<平台目录>/<核心>/<stem>/`（**ring 目录**，一个目录放多个 slot）。
//!   两者都调生产同一段逻辑（`persist::sav_path` / `slot_store::state_dir`）。
//! - 核心**必须显式传**：同平台多核心时"默认"与"实际"会分叉，换过核心的卡带
//!   若合并目录就会读不到旧档 —— 所以这里也断言"换核心 → 换目录"。
//! - 六键顺序 `B1..B6 = B, Y, A, X, L1, R1`；`ARCADE_BUTTONS` 是唯一来源，
//!   `PadLayout::Arcade` 直接返回它。
//!
//! 为什么值得单测：街机与卡带同名 stem 一旦路径重叠就会**互相覆盖存档**；
//! 六键顺序错会让"拳/跳"对调 —— 两者在设备上都不容易被发现。

use std::path::{Path, PathBuf};

use slot_input::Btn;
use slot_retro::ButtonMask;
use slot_store::{Core, Platform};

use crate::arcade::{arcade_buttons, arcade_pad, save_paths, ARCADE_BUTTONS};
use crate::input::PadLayout;

/// 每台机种"该用哪个核心"，供用例显式传入（与生产 `StateRing::new` 同一口径）。
fn core_of(platform: Platform) -> Core {
    match platform {
        Platform::Arcade | Platform::Neogeo => Core::Fbneo,
        _ => Core::Mgba,
    }
}

/// 把路径规整成用 `/` 分隔的字符串（Windows 上是 `\`）。
fn shown(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

// ---------------------------------------------------------------- ① 存档文件

/// SRAM 落在 `Saves/<平台目录>/<stem>.sav`。
#[test]
fn the_sram_file_sits_under_its_own_platform() {
    let root = Path::new("/card");
    let arcade = save_paths(root, Platform::Arcade, Core::Fbneo, "mslug");
    assert_eq!(shown(&arcade.sram), "/card/Saves/Arcade/mslug.sav");
    assert_eq!(
        arcade.sram.extension().and_then(|e| e.to_str()),
        Some("sav"),
        "SRAM 应当是 .sav 文件"
    );

    let neogeo = save_paths(root, Platform::Neogeo, Core::Fbneo, "mslug");
    assert_eq!(shown(&neogeo.sram), "/card/Saves/NeoGeo/mslug.sav");
}

// ---------------------------------------------------------------- ② 快照是 ring 目录

/// 即时存档在 `States/<平台目录>/<核心>/<stem>/` —— 是**目录**（ring），不是单文件。
#[test]
fn the_state_ring_is_a_directory_between_the_core_and_the_stem() {
    let root = Path::new("/card");
    let arcade = save_paths(root, Platform::Arcade, Core::Fbneo, "mslug");
    assert_eq!(
        shown(&arcade.state_dir),
        "/card/States/Arcade/fbneo/mslug",
        "街机快照目录应当是 States/<平台>/<核心>/<stem>/"
    );
    assert!(
        arcade.state_dir.extension().is_none(),
        "快照应是目录而不是带扩展名的文件：{:?}",
        arcade.state_dir
    );

    // 卡带侧同样带核心名；目录名用 `dir_name()`（GBA 是大写），不写死。
    let gba = save_paths(root, Platform::Gba, Core::Mgba, "mslug");
    assert_eq!(
        shown(&gba.state_dir),
        format!(
            "/card/States/{}/{}/mslug",
            Platform::Gba.dir_name(),
            Core::Mgba.as_str()
        )
    );
    assert_ne!(
        shown(&gba.state_dir),
        shown(&arcade.state_dir),
        "同一 stem 的街机与 GBA 快照目录相同（会互相覆盖）"
    );
}

/// 换核心必须换目录：否则换过核心的卡带会读到别的核心写的档（或"旧档消失"）。
#[test]
fn a_different_core_moves_the_state_directory() {
    let root = Path::new("/card");
    let mgba = save_paths(root, Platform::Gba, Core::Mgba, "emerald");
    let gpsp = save_paths(root, Platform::Gba, Core::Gpsp, "emerald");
    assert_ne!(
        mgba.state_dir, gpsp.state_dir,
        "同一张卡换核心后快照目录没变（旧档会被别的核心覆盖/读错）"
    );
    // SRAM 与核心无关：换核心仍读同一份 .sav（既有行为）。
    assert_eq!(
        mgba.sram, gpsp.sram,
        "SRAM 不该随核心改变（它是卡带存档，不是即时存档）"
    );
}

// ---------------------------------------------------------------- ③ 跨平台不撞名

/// 同一个 stem 在所有平台上，SRAM 与快照路径都不得重复——这是"街机不覆盖卡带存档"的最小证据。
#[test]
fn the_same_stem_never_shares_a_path_across_platforms() {
    let root = Path::new("/card");
    let platforms = [
        Platform::Arcade,
        Platform::Neogeo,
        Platform::Gba,
        Platform::Gb,
        Platform::Gbc,
        Platform::Fc,
        Platform::Sfc,
        Platform::Md,
    ];
    let mut srams: Vec<PathBuf> = Vec::new();
    let mut states: Vec<PathBuf> = Vec::new();
    for platform in platforms {
        let paths = save_paths(root, platform, core_of(platform), "mslug");
        assert!(
            !srams.contains(&paths.sram),
            "{platform:?} 的 SRAM 路径与前面某个平台重复：{}",
            shown(&paths.sram)
        );
        assert!(
            !states.contains(&paths.state_dir),
            "{platform:?} 的快照路径与前面某个平台重复：{}",
            shown(&paths.state_dir)
        );
        srams.push(paths.sram);
        states.push(paths.state_dir);
    }
    assert_eq!(srams.len(), platforms.len());
    assert_eq!(states.len(), platforms.len());
}

// ---------------------------------------------------------------- ④ stem 原样保留

/// stem 里的空格与大写原样保留：normalize 之后"界面能列出、存档却读不到"。
#[test]
fn the_paths_keep_the_stem_verbatim() {
    let root = Path::new("/card");
    let paths = save_paths(root, Platform::Arcade, Core::Fbneo, "Metal Slug 3");
    assert_eq!(shown(&paths.sram), "/card/Saves/Arcade/Metal Slug 3.sav");
    assert_eq!(
        shown(&paths.state_dir),
        "/card/States/Arcade/fbneo/Metal Slug 3"
    );

    let upper = save_paths(root, Platform::Arcade, Core::Fbneo, "MSLUG");
    assert_eq!(
        shown(&upper.sram),
        "/card/Saves/Arcade/MSLUG.sav",
        "大写 stem 不该被改写成小写"
    );
}

// ---------------------------------------------------------------- ⑤ 不出现双分隔符

/// `root` 带尾斜杠时不许拼出 `//`（`Path::join` 天然满足，手工 `format!` 会露）。
#[test]
fn a_trailing_slash_on_the_root_does_not_produce_double_separators() {
    for root in [Path::new("/card/"), Path::new("/card")] {
        let paths = save_paths(root, Platform::Arcade, Core::Fbneo, "mslug");
        let sram = shown(&paths.sram);
        let state = shown(&paths.state_dir);
        assert!(!sram.contains("//"), "SRAM 路径出现 //：{sram}");
        assert!(!state.contains("//"), "快照路径出现 //：{state}");
    }
}

// ---------------------------------------------------------------- ⑥ 单一来源

/// `save_paths().state_dir` 必须与 `slot_store::state_dir` 完全一致（同一段生产逻辑，
/// 不许出现第二份 `join("States")`）。
#[test]
fn the_save_paths_delegate_to_the_single_production_helper() {
    let root = Path::new("/card");
    for platform in [Platform::Arcade, Platform::Neogeo, Platform::Gba] {
        let core = core_of(platform);
        assert_eq!(
            save_paths(root, platform, core, "mslug").state_dir,
            slot_store::state_dir(root, platform, core, "mslug"),
            "{platform:?} 的 state_dir 与 slot_store::state_dir 不一致（出现了第二份实现）"
        );
    }
}

// ---------------------------------------------------------------- ⑦ 六键顺序

/// `B1..B6 = B, Y, A, X, L1, R1`：顺序错会把"拳"和"跳"对调。
#[test]
fn the_six_arcade_buttons_are_in_panel_order() {
    assert_eq!(
        ARCADE_BUTTONS,
        [Btn::B, Btn::Y, Btn::A, Btn::X, Btn::L1, Btn::R1],
        "街机 6 键的顺序应与面板 B1..B6 一致"
    );

    let mut seen = Vec::new();
    for button in ARCADE_BUTTONS {
        assert!(
            !seen.contains(&button),
            "{button:?} 在 6 键表里出现了两次（两个物理键映射到同一功能）"
        );
        seen.push(button);
    }
    assert_eq!(seen.len(), 6, "街机应当恰好 6 个功能键");

    for forbidden in [Btn::Start, Btn::Select] {
        assert!(
            !ARCADE_BUTTONS.contains(&forbidden),
            "{forbidden:?} 不该出现在 6 键表里"
        );
    }
}

/// 每个面板键映射到**各自**的掩码位；START/SELECT 单独可用；L2/R2 为 `None`。
#[test]
fn the_arcade_pad_maps_each_panel_button_to_its_own_bit() {
    let expected = [
        ButtonMask::B,
        ButtonMask::Y,
        ButtonMask::A,
        ButtonMask::X,
        ButtonMask::L,
        ButtonMask::R,
    ];
    let mut masks: Vec<u16> = Vec::new();
    for (panel, mask) in ARCADE_BUTTONS.iter().zip(expected) {
        let got = arcade_pad(*panel);
        assert_eq!(
            got,
            Some(mask),
            "{panel:?} 应当映射到掩码 {mask:#x}（面板 B{}）",
            masks.len() + 1
        );
        let got = got.expect("上面已断言 Some");
        assert!(
            !masks.contains(&got),
            "两个面板键映射到了同一个掩码位 {got:#x}（会同时触发两个功能）"
        );
        masks.push(got);
    }

    assert_eq!(arcade_pad(Btn::Start), Some(ButtonMask::START));
    assert_eq!(arcade_pad(Btn::Select), Some(ButtonMask::SELECT));

    // 街机布局里没有作用的键返回 None；方向键由 `Pad` 回退到卡带表处理，
    // 所以这里的 None 不代表"街机不能走方向"。
    for inert in [Btn::L2, Btn::R2] {
        assert_eq!(arcade_pad(inert), None, "{inert:?} 在街机布局里应当没有作用");
    }
}

/// `arcade_buttons()` 与 `PadLayout::Arcade` 都读同一张常量表（布局不许二次手写）。
#[test]
fn the_layout_and_the_helper_read_the_same_table() {
    assert_eq!(
        arcade_buttons(),
        ARCADE_BUTTONS,
        "arcade_buttons() 与 ARCADE_BUTTONS 不是同一份数据"
    );
    assert_eq!(
        PadLayout::Arcade.buttons(),
        &ARCADE_BUTTONS[..],
        "PadLayout::Arcade 的键序与 ARCADE_BUTTONS 不一致（绘制/输入会与文档脱节）"
    );
    assert_eq!(PadLayout::Arcade.buttons().len(), 6);
    assert!(
        PadLayout::Default.buttons().is_empty(),
        "卡带布局不该有自己的六键表"
    );
}
