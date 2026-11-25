//! Phase 4a「货架柜子 + 街机主界面状态机」的**独立验收**（QA）。
//!
//! 只放测试代码，由 `lib.rs` 的 `#[cfg(test)] mod arcade_qa;` 挂进来。夹具全部
//! 自己造：`Cart` 手搓、`Catalog` 用 `from_entries`、货架用 `shelves_of` 现搭，
//! 不共用工程师 `arcade.rs` / `app::arcade_tests` 里的夹具与断言顺序。
//!
//! 覆盖的是**逻辑层**（柜子出现条件、书架环、3 块窗口、筛选、BIOS、按键语义）。
//! 像素/布局/绘制不在本文件内（由 lead 在宿主机构建里截图比对设计稿）。
//!
//! 有两处注定只能"代码审查"而不是断言，这里如实标出来，不假装测过：
//! - 柜子上 **LEFT/RIGHT 的物理键映射**（`L1/R1 → switch_shelf`）住在
//!   `App::handle` 的私有分支里；本文件测的是它依赖的 `ring_step` + 可选性谓词。
//! - **`Phase::Arcade` 里 B 只改 phase、不动 `shelf_at`/`shelf().index`** 由
//!   `App::exit_arcade`（`app.rs:3726`）实现，本文件只能验证
//!   `ArcadeScreen::key` 的语义位；父层不动光标这一点见报告里的代码审查结论。

use std::path::{Path, PathBuf};

use slot_store::arcade_dat::{Catalog, Entry};
use slot_store::{Cart, Platform, SlotState};
use slot_ui::Shelf;

use crate::app::{reconcile_shelf_at, ring_step, shelves_of, Rack};
use crate::arcade::{
    bios_state, visible_window, ArcadeAction, ArcadeFilter, ArcadeKey, ArcadeScreen, BiosState,
    WINDOW,
};

// ---------------------------------------------------------------- 夹具

fn cart(platform: Platform, stem: &str) -> Cart {
    Cart {
        platform,
        stem: stem.to_string(),
        rom: PathBuf::from(format!("Games/{}/{stem}.zip", platform.dir_name())),
        label: None,
        title: stem.to_string(),
        code: String::new(),
        shell: None,
        model: Default::default(),
        front: None,
    }
}

fn entry(set: &str, system: &str) -> Entry {
    Entry {
        set: set.to_string(),
        title: set.to_uppercase(),
        system: system.to_string(),
        year: String::new(),
        players: 1,
        orientation: Default::default(),
        parent: String::new(),
        is_bios: false,
    }
}

fn catalog(platform: Platform, entries: Vec<Entry>) -> Catalog {
    Catalog::from_entries(platform, entries)
}

/// 与 `App::next_shelf` 里那条谓词同形：空架子不可选，柜子恒可选。
fn selectable_of(shelves: &[(Rack, Shelf)]) -> Vec<bool> {
    shelves
        .iter()
        .map(|(rack, shelf)| !shelf.carts.is_empty() || matches!(rack, Rack::Cabinet(_)))
        .collect()
}

fn write_file(path: &Path, bytes: &[u8]) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, bytes).unwrap();
}

const HOSTED: [Platform; 2] = [Platform::Arcade, Platform::Neogeo];

// ---------------------------------------------------------------- ① 柜子只随包装出现

/// 没装包 → 8 个架子且**没有**柜子；装包 → 10 个，柜子追加在末尾 8、9。
#[test]
fn cabinets_exist_only_while_the_pack_is_active() {
    let state = SlotState::default();
    let dormant = shelves_of(vec![cart(Platform::Gba, "emerald")], &state, &[]);
    assert_eq!(dormant.len(), 8, "内置 6 台 + 最近 + 收藏");
    assert!(
        !dormant.iter().any(|(rack, _)| matches!(rack, Rack::Cabinet(_))),
        "没装包就长出了柜子"
    );

    let active = shelves_of(vec![cart(Platform::Gba, "emerald")], &state, &HOSTED);
    assert_eq!(active.len(), 10, "内置 8 + 两台柜子");
    assert_eq!(active[8].0, Rack::Cabinet(Platform::Arcade));
    assert_eq!(active[9].0, Rack::Cabinet(Platform::Neogeo));
    assert!(active[8].1.carts.is_empty(), "柜子不是卡带架，架子必须恒空");
    assert!(active[9].1.carts.is_empty());
}

/// 街机卡带不该被塞进任何架子（柜子恒空，板子由 `ArcadeScreen` 从 carts 自己算）。
#[test]
fn arcade_carts_never_land_in_a_shelf_rack() {
    let state = SlotState::default();
    let carts = vec![
        cart(Platform::Gba, "emerald"),
        cart(Platform::Arcade, "sf2"),
        cart(Platform::Neogeo, "mslug"),
    ];
    let shelves = shelves_of(carts, &state, &HOSTED);
    let total: usize = shelves.iter().map(|(_, shelf)| shelf.carts.len()).sum();
    assert_eq!(total, 1, "只有 GBA 那张卡进架子");
    assert_eq!(shelves[2].1.carts.len(), 1, "GBA 架：index 0/1 是最近与收藏");
    for (rack, shelf) in &shelves {
        assert!(
            shelf
                .carts
                .iter()
                .all(|cart| cart.platform != Platform::Arcade && cart.platform != Platform::Neogeo),
            "{rack:?} 里混进了街机卡带"
        );
    }
}

/// 空柜子也必须能被环选中，否则装包后用户永远走不到它。
#[test]
fn a_cabinet_is_selectable_even_with_no_games() {
    let state = SlotState::default();
    let shelves = shelves_of(Vec::new(), &state, &HOSTED);
    let selectable = selectable_of(&shelves);
    assert_eq!(selectable.len(), 10);
    assert!(selectable[8] && selectable[9], "柜子不可选＝装了包也进不去");
    assert!(
        !selectable[2..8].iter().any(|bit| *bit),
        "空的内置架不该可选"
    );
}

// ---------------------------------------------------------------- ② 卸载回落（重点）

#[test]
fn reconcile_keeps_the_cursor_where_it_is_when_still_selectable() {
    let shelves = vec![
        (Rack::Recents, Shelf::new(Vec::new())),
        (Rack::Favorites, Shelf::new(Vec::new())),
        (Rack::Of(Platform::Gba), Shelf::new(vec![cart(Platform::Gba, "a")])),
        (Rack::Cabinet(Platform::Arcade), Shelf::new(Vec::new())),
    ];
    assert_eq!(reconcile_shelf_at(&shelves, 2), 2);
    assert_eq!(reconcile_shelf_at(&shelves, 3), 3, "柜子可选，光标不动");
    assert_eq!(reconcile_shelf_at(&shelves, 0), 2, "最近不可选，往**前**找");
}

#[test]
fn reconcile_prefers_the_nearest_selectable_rack_before_the_cursor() {
    let shelves = vec![
        (Rack::Of(Platform::Gba), Shelf::new(vec![cart(Platform::Gba, "a")])),
        (Rack::Of(Platform::Gb), Shelf::new(Vec::new())),
        (Rack::Of(Platform::Gbc), Shelf::new(vec![cart(Platform::Gbc, "b")])),
        (Rack::Of(Platform::Fc), Shelf::new(Vec::new())),
    ];
    assert_eq!(reconcile_shelf_at(&shelves, 3), 2, "先往前找最近的");
    assert_eq!(reconcile_shelf_at(&shelves, 1), 0);
}

#[test]
fn reconcile_looks_forward_when_nothing_before_the_cursor_is_selectable() {
    let shelves = vec![
        (Rack::Recents, Shelf::new(Vec::new())),
        (Rack::Favorites, Shelf::new(Vec::new())),
        (Rack::Of(Platform::Gba), Shelf::new(vec![cart(Platform::Gba, "a")])),
    ];
    assert_eq!(reconcile_shelf_at(&shelves, 0), 2, "前面没有就往后找");
    assert_eq!(reconcile_shelf_at(&shelves, 1), 2);
}

#[test]
fn reconcile_returns_zero_when_nothing_is_selectable() {
    let shelves = vec![
        (Rack::Recents, Shelf::new(Vec::new())),
        (Rack::Of(Platform::Gba), Shelf::new(Vec::new())),
        (Rack::Cabinet(Platform::Arcade), Shelf::new(Vec::new())),
    ];
    // 注意柜子是可选的，把柜子也换成空架子才构造得出"全不可选"。
    let shelves: Vec<(Rack, Shelf)> = shelves
        .into_iter()
        .map(|(rack, shelf)| match rack {
            Rack::Cabinet(_) => (Rack::Of(Platform::Gba), shelf),
            other => (other, shelf),
        })
        .collect();
    assert_eq!(reconcile_shelf_at(&shelves, 2), 0);
}

/// 你点名要重点测的那条：光标停在柜子上时卸载包。
///
/// 卸载后货架从 10 变 8，旧下标 8/9 直接越界——这条路径必须不 panic，且落回
/// 一个**可选**的架子。
#[test]
fn unloading_a_pack_while_the_cursor_sits_on_the_cabinet_does_not_panic() {
    let state = SlotState::default();
    let carts = vec![cart(Platform::Gba, "emerald")];
    let active = shelves_of(carts.clone(), &state, &HOSTED);
    assert_eq!(active.len(), 10);

    let home = reconcile_shelf_at(&active, 9);
    assert_eq!(home, 9, "卸载前光标就停在 NeoGeo 柜子上（前提）");

    let dormant = shelves_of(carts.clone(), &state, &[]);
    assert_eq!(dormant.len(), 8, "卸载后柜子消失");
    for stale in [8usize, 9] {
        let landed = reconcile_shelf_at(&dormant, stale);
        assert_eq!(landed, 2, "越界下标 {stale} 应落回 GBA 架（index 2）");
        assert!(!dormant[landed].1.carts.is_empty(), "落到了不可选的架子");
    }

    // 空卡（连 GBA 都没有）：全不可选 → 0，仍然不 panic。
    let empty = shelves_of(Vec::new(), &state, &[]);
    assert_eq!(reconcile_shelf_at(&empty, 9), 0);
    assert_eq!(reconcile_shelf_at(&empty, 99), 0);
}

#[test]
fn reconcile_tolerates_an_index_far_past_the_end() {
    let state = SlotState::default();
    let shelves = shelves_of(vec![cart(Platform::Gba, "a")], &state, &[]);
    assert_eq!(reconcile_shelf_at(&shelves, 99), 2);
}

// ---------------------------------------------------------------- ③ 书架环

/// 环要真的能走到柜子：从最后一台内置架往右是 Arcade 柜，再往右是 NeoGeo 柜。
#[test]
fn the_shelf_ring_reaches_the_cabinets() {
    let state = SlotState::default();
    let shelves = shelves_of(vec![cart(Platform::Gba, "a")], &state, &HOSTED);
    let selectable = selectable_of(&shelves);
    let len = shelves.len();

    // Md 在 index 7；往右第一次落到 Arcade 柜（8），再往右是 NeoGeo 柜（9）。
    assert_eq!(ring_step(len, 7, 1, &selectable), Some(8), "环走不到 Arcade 柜");
    assert_eq!(ring_step(len, 8, 1, &selectable), Some(9), "环走不到 NeoGeo 柜");
    assert_eq!(ring_step(len, 9, 1, &selectable), Some(2), "从柜子继续往前应绕回 GBA");
    // 往回走：期望值从夹具现算（"8 之前最近的可选架"），不硬编码 index，免得
    // 夹具一改就变成"断言在描述夹具"而不是描述行为。
    let before_8 = (0..8)
        .rev()
        .find(|index| selectable[*index])
        .expect("夹具前提：8 之前必须有可选架");
    assert_eq!(before_8, 2, "夹具前提：只有 GBA 可选");
    assert_eq!(ring_step(len, 8, -1, &selectable), Some(before_8));
}

/// `by = ±len` 是"整圈"：**起点可选时**必然回到起点。
///
/// （不可选起点不参与这条性质：它是"多走几步"的普通情形，见
/// [`each_step_moves_to_the_next_selectable_rack`]。）
#[test]
fn a_full_turn_of_the_ring_returns_to_the_start() {
    let state = SlotState::default();
    let shelves = shelves_of(vec![cart(Platform::Gba, "a")], &state, &HOSTED);
    let selectable = selectable_of(&shelves);
    let len = shelves.len();
    // 2 = GBA、8/9 = 两台柜子，三个可选起点都试一遍。
    for at in [2usize, 8, 9] {
        assert_eq!(
            ring_step(len, at, len as i32, &selectable),
            Some(at),
            "+{len} 步应回到 {at}"
        );
        assert_eq!(
            ring_step(len, at, -(len as i32), &selectable),
            Some(at),
            "-{len} 步应回到 {at}"
        );
    }
}

/// `by` 的绝对值是"走几步"，每步都单独判可选——不是"一次跳 |by| 格"。
///
/// 这条区分很容易被后来的"优化"抹平：把 `at + by * step` 写成一次取模，
/// `by = 2` 就会变成"跳过两个位置"，从而踩到不可选的架子上去。
#[test]
fn each_step_moves_to_the_next_selectable_rack() {
    // 0 可选、1 不可选、2 可选、3 可选、4 不可选（绕回 0）。
    let selectable = [true, false, true, true, false];
    let len = selectable.len();

    assert_eq!(ring_step(len, 0, 1, &selectable), Some(2), "单步要跳过不可选的 1");
    assert_eq!(
        ring_step(len, 0, 2, &selectable),
        Some(3),
        "by=2 是两次单步（0→2→3），不是跳过两个位置（0→2 之后停在 2）"
    );
    assert_eq!(
        ring_step(len, 0, 3, &selectable),
        Some(0),
        "三次单步（0→2→3→0）应绕回起点"
    );
    assert_eq!(
        ring_step(len, 3, -2, &selectable),
        Some(0),
        "反向两次单步（3→2→0）"
    );
}

/// `by == 0`：停在原地，但只在"原位可选"时成立。
#[test]
fn a_zero_step_keeps_the_cursor_only_when_it_is_selectable() {
    let selectable = [false, true, false, true];
    assert_eq!(ring_step(4, 1, 0, &selectable), Some(1), "可选起点上原地");
    assert_eq!(ring_step(4, 3, 0, &selectable), Some(3));
    assert_eq!(ring_step(4, 0, 0, &selectable), None, "不可选起点上不该原地");
    assert_eq!(ring_step(4, 2, 0, &selectable), None);
}

/// 越界下标直接拒绝，不用取模把它"折"回合法范围。
///
/// 取模会让"调用方传了坏下标"这件事被静默掩盖：界面看起来正常，实际光标在
/// 一个不该出现的位置上。
#[test]
fn an_out_of_range_index_is_rejected_instead_of_wrapped() {
    let selectable = [true; 4];
    assert_eq!(ring_step(4, 4, 1, &selectable), None, "at == len");
    assert_eq!(ring_step(4, 9, -1, &selectable), None, "at 远大于 len");
    assert_eq!(ring_step(4, 9, 0, &selectable), None, "by=0 也不能掩盖越界");
}

#[test]
fn the_ring_cannot_move_when_nothing_is_selectable() {
    assert_eq!(ring_step(0, 0, 1, &[]), None, "空货架");
    assert_eq!(ring_step(8, 0, 1, &[false; 8]), None, "全不可选");
    assert_eq!(
        ring_step(10, 0, 1, &[true; 3]),
        None,
        "selectable 长度与 len 不一致时宁可不动"
    );
    assert_eq!(ring_step(1, 0, 1, &[true]), Some(0), "只有一个架子时原地");
    assert_eq!(ring_step(1, 0, 1, &[false]), None);
}

// ---------------------------------------------------------------- ④ 3 块窗口

#[test]
fn the_three_block_window_clamps_at_both_ends() {
    assert_eq!(visible_window(0, 0), 0..0, "空列表");
    assert_eq!(WINDOW, 3);

    for (len, cases) in [
        (1usize, vec![(0usize, 0..1)]),
        (2, vec![(0, 0..2), (1, 0..2)]),
        (
            3,
            vec![(0, 0..3), (1, 0..3), (2, 0..3)],
        ),
        (
            10,
            vec![(0, 0..3), (1, 0..3), (2, 1..4), (5, 4..7), (9, 7..10)],
        ),
    ] {
        for (index, expected) in cases {
            assert_eq!(
                visible_window(index, len),
                expected,
                "len={len} index={index}"
            );
        }
    }

    // 越界下标也要夹住，不能返回越过 len 的区间。
    for (index, expected) in [(99usize, 7..10), (3, 2..5)] {
        assert_eq!(visible_window(index, 10), expected, "index={index}");
    }
}

// ---------------------------------------------------------------- ⑤ 筛选 / 板列表

fn arcade_fixture() -> (Vec<Cart>, Catalog) {
    let carts = vec![
        cart(Platform::Arcade, "sf2"),
        cart(Platform::Arcade, "ssf2"),
        cart(Platform::Arcade, "dino"),
        cart(Platform::Neogeo, "mslug"),
    ];
    let catalog = catalog(
        Platform::Arcade,
        vec![
            entry("sf2", "CPS-1"),
            entry("ssf2", "CPS-2"),
            entry("dino", "CPS-1"),
        ],
    );
    (carts, catalog)
}

#[test]
fn boards_follow_the_filter_and_keep_scan_order() {
    let (carts, catalog) = arcade_fixture();
    let mut screen = ArcadeScreen::new(Platform::Arcade);

    let all: Vec<&str> = screen
        .boards(&catalog, &carts)
        .iter()
        .map(|cart| cart.stem.as_str())
        .collect();
    assert_eq!(all, vec!["sf2", "ssf2", "dino"], "保持 scan 顺序，且只看本平台");

    screen.filter = ArcadeFilter::System("CPS-1".to_string());
    let cps1: Vec<&str> = screen
        .boards(&catalog, &carts)
        .iter()
        .map(|cart| cart.stem.as_str())
        .collect();
    assert_eq!(cps1, vec!["sf2", "dino"]);
    assert_eq!(screen.current(&catalog, &carts).map(|c| c.stem.as_str()), Some("sf2"));
}

#[test]
fn cycling_the_filter_returns_to_the_first_board() {
    let (carts, catalog) = arcade_fixture();
    let systems = ArcadeScreen::new(Platform::Arcade).systems(&catalog, &carts);
    assert_eq!(systems, vec!["CPS-1".to_string(), "CPS-2".to_string()]);

    let mut screen = ArcadeScreen::new(Platform::Arcade);
    screen.index = 2;

    screen.cycle_filter(&systems);
    assert_eq!(screen.filter, ArcadeFilter::System("CPS-1".to_string()));
    assert_eq!(screen.index, 0, "换筛选后必须回到第一块");
    screen.cycle_filter(&systems);
    assert_eq!(screen.filter, ArcadeFilter::System("CPS-2".to_string()));
    screen.cycle_filter(&systems);
    assert_eq!(screen.filter, ArcadeFilter::All, "最后一项再切回全部");
    assert_eq!(screen.index, 0);
}

#[test]
fn an_empty_filter_yields_no_boards_and_never_panics() {
    let (carts, catalog) = arcade_fixture();
    let mut screen = ArcadeScreen::new(Platform::Arcade);
    screen.filter = ArcadeFilter::System("NO-SUCH-BOARD".to_string());

    assert!(screen.boards(&catalog, &carts).is_empty(), "空筛选不该有板");
    assert!(screen.current(&catalog, &carts).is_none());
    screen.select_next(0);
    assert_eq!(screen.index, 0, "空列表选中项必须夹回 0");
    screen.select_prev(0);
    assert_eq!(screen.index, 0);
    screen.set_index(7, 0);
    assert_eq!(screen.index, 0, "列表变短后不能越界");
}

#[test]
fn systems_rank_by_count_then_name_and_stop_at_five() {
    let stems = [
        ("a", "CPS-1"),
        ("b", "CPS-1"),
        ("c", "CPS-1"),
        ("d", "CPS-2"),
        ("e", "CPS-2"),
        ("f", "NEOGEO"),
        ("g", "NEOGEO"),
        ("h", "KONAMI"),
        ("i", "SEGA"),
        ("j", "TAITO"),
    ];
    let carts: Vec<Cart> = stems
        .iter()
        .map(|(stem, _)| cart(Platform::Arcade, stem))
        .collect();
    let catalog = catalog(
        Platform::Arcade,
        stems
            .iter()
            .map(|(stem, system)| entry(stem, system))
            .collect(),
    );

    let screen = ArcadeScreen::new(Platform::Arcade);
    assert_eq!(
        screen.systems(&catalog, &carts),
        vec![
            "CPS-1".to_string(),
            "CPS-2".to_string(),
            "NEOGEO".to_string(),
            "KONAMI".to_string(),
            "SEGA".to_string(),
        ],
        "按出现次数降序、同频按名字升序，且最多 5 个（TAITO 被截掉）"
    );
}

// ---------------------------------------------------------------- ⑥ BIOS 两个路径

#[test]
fn bios_state_reads_both_paths_and_rejects_non_files() {
    // 什么都没放 → Missing
    let empty = tempfile::tempdir().unwrap();
    assert_eq!(bios_state(empty.path(), Platform::Neogeo), BiosState::Missing);

    // 路径一：BIOS/fbneo/neogeo.zip，普通文件且有内容 → Present
    let first = tempfile::tempdir().unwrap();
    write_file(&first.path().join("BIOS/fbneo/neogeo.zip"), b"PK\x03\x04 bios");
    assert_eq!(bios_state(first.path(), Platform::Neogeo), BiosState::Present);

    // 路径二：BIOS/neogeo.zip → 同样算 Present
    let second = tempfile::tempdir().unwrap();
    write_file(&second.path().join("BIOS/neogeo.zip"), b"PK\x03\x04 bios");
    assert_eq!(bios_state(second.path(), Platform::Neogeo), BiosState::Present);

    // 0 字节的空壳不算
    let blank = tempfile::tempdir().unwrap();
    write_file(&blank.path().join("BIOS/neogeo.zip"), b"");
    assert_eq!(bios_state(blank.path(), Platform::Neogeo), BiosState::Missing);

    // 目录不算
    let folder = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(folder.path().join("BIOS/fbneo/neogeo.zip")).unwrap();
    assert_eq!(bios_state(folder.path(), Platform::Neogeo), BiosState::Missing);

    // 符号链接不算（悬空链接最容易被当成"有"）
    #[cfg(unix)]
    {
        let linked = tempfile::tempdir().unwrap();
        write_file(&linked.path().join("BIOS/real.zip"), b"PK\x03\x04 bios");
        std::os::unix::fs::symlink(
            linked.path().join("BIOS/real.zip"),
            linked.path().join("BIOS/neogeo.zip"),
        )
        .unwrap();
        assert_eq!(
            bios_state(linked.path(), Platform::Neogeo),
            BiosState::Missing,
            "符号链接被当成了真 BIOS"
        );
    }
}

/// Arcade 永远不需要 BIOS；即便卡上放着一个真的 BIOS 也不该变成 Missing。
#[test]
fn arcade_never_needs_a_bios() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(bios_state(dir.path(), Platform::Arcade), BiosState::NotRequired);
    write_file(&dir.path().join("BIOS/neogeo.zip"), b"PK\x03\x04 bios");
    assert_eq!(bios_state(dir.path(), Platform::Arcade), BiosState::NotRequired);
    // 内置机种同样不需要
    assert_eq!(bios_state(dir.path(), Platform::Gba), BiosState::NotRequired);
}

// ---------------------------------------------------------------- ⑦ 按键语义（不穿透）

/// 只有 LEFT/RIGHT 能移动选中项；其余键只返回语义动作，不得改 `index`/`filter`。
#[test]
fn only_left_and_right_move_the_selection() {
    let systems = vec!["CPS-1".to_string(), "CPS-2".to_string()];
    let boards = 3;

    let mut screen = ArcadeScreen::new(Platform::Arcade);
    assert_eq!(screen.key(ArcadeKey::Right, boards, &systems), ArcadeAction::None);
    assert_eq!(screen.index, 1);
    assert_eq!(screen.key(ArcadeKey::Right, boards, &systems), ArcadeAction::None);
    assert_eq!(screen.index, 2);
    assert_eq!(screen.key(ArcadeKey::Right, boards, &systems), ArcadeAction::None);
    assert_eq!(screen.index, 0, "3 块板要循环回第一块");
    assert_eq!(screen.key(ArcadeKey::Left, boards, &systems), ArcadeAction::None);
    assert_eq!(screen.index, 2, "往回也要循环");

    for (key, expected) in [
        (ArcadeKey::Favorite, ArcadeAction::Favorite),
        (ArcadeKey::Back, ArcadeAction::Back),
        (ArcadeKey::Insert, ArcadeAction::Insert),
        (ArcadeKey::Service, ArcadeAction::None),
    ] {
        let mut untouched = ArcadeScreen::new(Platform::Arcade);
        untouched.index = 1;
        untouched.filter = ArcadeFilter::System("CPS-2".to_string());
        let before = untouched.clone();
        assert_eq!(untouched.key(key, boards, &systems), expected, "{key:?}");
        assert_eq!(
            untouched.index, before.index,
            "{key:?} 移动了选中项（输入穿透到板列表）"
        );
        assert_eq!(
            untouched.filter, before.filter,
            "{key:?} 改了筛选（输入穿透到筛选条）"
        );
    }
}

/// X（Filter）既切筛选、又把选中项归零——两条必须同时成立。
#[test]
fn the_filter_key_also_resets_the_selection() {
    let systems = vec!["CPS-1".to_string(), "CPS-2".to_string()];
    let mut screen = ArcadeScreen::new(Platform::Arcade);
    screen.index = 2;

    assert_eq!(screen.key(ArcadeKey::Filter, 3, &systems), ArcadeAction::None);
    assert_eq!(screen.filter, ArcadeFilter::System("CPS-1".to_string()));
    assert_eq!(screen.index, 0, "换筛选后选中项必须归零");
}
