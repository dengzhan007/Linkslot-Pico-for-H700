//! Phase 4b「柜子 / PCB / 信息卡绘制」的**几何层独立验收**（QA）。
//!
//! 只放测试代码，由 `lib.rs` 的 `#[cfg(test)] mod cabinet_qa;` 挂进来。
//! 像素/版式与设计稿的比对归 lead（宿主机构建里截图）；这里只验**几何**：
//!
//! 1. 任何绘制都不超出传入面板（`cabinet.rs` 文档自称的硬约束，独立再验一次）；
//! 2. 1/2/3 块的居中与铺满、三种数量板宽一致、余数留右侧；
//! 3. 滚动时"同一块板换格"而"格子本身不动"；
//! 4. 选中那块带 2px accent 框、非选中一块都没有。
//!
//! 两处**只能代码审查**（如实标注，不硬写成断言）：
//! - "1/2/3 块真的被画出来"这一步的循环在 `App::draw_arcade` 里（`draw_boards`
//!   尚未抽成 `cabinet.rs` 的函数），我验的是它依赖的 `board_rects` 与 `draw_pcb`；
//! - 信息卡/marquee 的文字内容（`ArcadeView` 的字符串）由 `app.rs` 组装，
//!   属于 UI 文案，不在几何范围。

use slot_gfx::{Draw, TexId};

use crate::cabinet::{
    board_rects, draw_boards, draw_cabinet_preview, draw_mini_cabinet, draw_pcb, Rect, Skin,
    BOARD_GAP, BOARD_SLOTS,
};

// ---------------------------------------------------------------- 夹具

/// 一块普通面板（主界面中间区域）。
fn area() -> Rect {
    Rect::new(40.0, 30.0, 640.0, 360.0)
}

/// 契约的板宽：三等分减两个间距。
///
/// 实现取 `floor`（三块左对齐、余数留右侧），lead 的文字契约一度写的是浮点。
/// 两种只差 <1px，所以这里按 `floor` 算但断言带 1px 容差 —— 两条路都能过，
/// 判的是"等宽 / 左对齐 / 间距 / 不出面板"这些性质，不是某个取整写法。
fn board_width(area: Rect) -> f32 {
    ((area.w - BOARD_GAP * (BOARD_SLOTS as f32 - 1.0)) / BOARD_SLOTS as f32).floor()
}

/// 与 `slot::arcade::visible_window` 同一条规则（`slot-ui` 不能反向依赖 `slot`）。
fn window_for(index: usize, len: usize) -> std::ops::Range<usize> {
    let start = index
        .saturating_sub(1)
        .min(len.saturating_sub(BOARD_SLOTS));
    start..(start + BOARD_SLOTS).min(len)
}

fn rect_of(draw: &Draw) -> Option<Rect> {
    match draw {
        Draw::Rect { x, y, w, h, .. }
        | Draw::Tex { x, y, w, h, .. }
        | Draw::Turned { x, y, w, h, .. } => Some(Rect::new(*x, *y, *w, *h)),
        _ => None,
    }
}

fn colour_of(draw: &Draw) -> Option<[f32; 4]> {
    match draw {
        Draw::Rect { colour, .. } => Some(*colour),
        _ => None,
    }
}

/// 调用方如何从 `Vec<(usize, Rect)>` 里取"当前块"——首尾两格正是容易取错的地方。
fn current<'a>(slots: &'a [(usize, Rect)], index: usize) -> Option<&'a Rect> {
    slots.iter().find(|(at, _)| *at == index).map(|(_, rect)| rect)
}

// ---------------------------------------------------------------- ① 落在面板内

/// 任意窗口大小 × 任意起点：所有板位都落在面板内，且不返回越界下标。
#[test]
fn every_board_slot_stays_inside_the_area() {
    let panel = area();
    for len in [0usize, 1, 2, 3, 10] {
        for start in 0..=len {
            for width in 0..=BOARD_SLOTS {
                let window = start..(start + width).min(len);
                for (index, rect) in board_rects(panel, window.clone(), len) {
                    assert!(
                        index < len.max(1),
                        "len={len} window={window:?} 返回了越界下标 {index}"
                    );
                    assert!(
                        panel.contains(rect, 0.5),
                        "len={len} window={window:?} index={index} 越界：{rect:?} 不在 {panel:?} 内"
                    );
                    assert!(rect.w > 0.0 && rect.h > 0.0, "零面积板位：{rect:?}");
                }
            }
        }
    }
}

// ---------------------------------------------------------------- ② 等宽 / 铺满 / 居中

#[test]
fn the_three_slots_share_a_width_and_fill_the_row() {
    let panel = area();
    let width = board_width(panel);
    let three = board_rects(panel, 0..3, 3);
    assert_eq!(three.len(), 3);

    for (slot, (index, rect)) in three.iter().enumerate() {
        assert_eq!(*index, slot, "窗口起点为 0 时第 {slot} 个槽位的下标应等于槽位号");
        assert!(
            (rect.w - width).abs() < 1.01,
            "第 {slot} 块宽 {} != 期望 {width}（允许 1px 取整差）",
            rect.w
        );
        let expected_x = panel.x + slot as f32 * (width + BOARD_GAP);
        assert!(
            (rect.x - expected_x).abs() < 1.01,
            "第 {slot} 块 x={} != {expected_x}（三块左对齐排布）",
            rect.x
        );
        assert!(
            (rect.y - panel.y).abs() < 0.01 && (rect.h - panel.h).abs() < 0.01,
            "第 {slot} 块的垂直几何与面板不一致"
        );
        if slot > 0 {
            let gap = rect.x - three[slot - 1].1.right();
            assert!((gap - BOARD_GAP).abs() < 0.01, "间距 {gap} != {BOARD_GAP}");
        }
    }

    // 末块不出面板；余量（取整误差）不超过 1px。
    let remainder = panel.right() - three[2].1.right();
    assert!(
        (-0.01..1.01).contains(&remainder),
        "末块右侧余量 {remainder} 不在 [0,1]（应当铺满或只差取整的 1px）"
    );

    // 1/2 块时板宽不变（不能"块少了就变宽"）。
    for (window, count) in [(0..1usize, 1usize), (0..2, 2)] {
        let slots = board_rects(panel, window.clone(), 3);
        assert_eq!(slots.len(), count, "window={window:?}");
        for (index, rect) in &slots {
            assert!(
                (rect.w - width).abs() < 1.01,
                "window={window:?} index={index} 板宽 {} 与三块时的 {width} 不一致",
                rect.w
            );
        }
    }
}

#[test]
fn one_and_two_boards_are_centred() {
    let panel = area();

    let one = board_rects(panel, 0..1, 1);
    assert_eq!(one.len(), 1);
    let centre = one[0].1.x + one[0].1.w / 2.0;
    assert!(
        (centre - (panel.x + panel.w / 2.0)).abs() < 0.5,
        "单块没有水平居中：中心 {centre} vs {}",
        panel.x + panel.w / 2.0
    );

    let two = board_rects(panel, 0..2, 2);
    assert_eq!(two.len(), 2);
    let left_margin = two[0].1.x - panel.x;
    let right_margin = panel.right() - two[1].1.right();
    assert!(
        (left_margin - right_margin).abs() < 1.01,
        "两块左右留白不等：{left_margin} vs {right_margin}"
    );
    assert!(
        (two[1].1.x - two[0].1.right() - BOARD_GAP).abs() < 0.01,
        "两块间距不是 BOARD_GAP"
    );
}

/// 垂直方向：1/2/3 块的 `y`/`h` 一致（否则滚动时会上下跳）。
#[test]
fn vertical_geometry_is_independent_of_the_board_count() {
    let panel = area();
    let baseline = board_rects(panel, 0..1, 1)[0].1;
    for window in [0..2usize, 0..3] {
        for (index, rect) in board_rects(panel, window.clone(), 3) {
            assert!(
                (rect.y - baseline.y).abs() < 0.01
                    && (rect.h - baseline.h).abs() < 0.01,
                "window={window:?} index={index} 垂直几何变了：{rect:?} vs {baseline:?}"
            );
        }
    }
}

// ---------------------------------------------------------------- ③ 滚动

/// 滚动时"同一块板换格"而"格子本身不动"。
///
/// 这条把两件事分开：只看"布局序列不相等"会把"选中移动"也算成滚动。槽位是固定
/// 的（首元素 x 本来就该一样），变的是"哪个下标落在哪个槽位"。
#[test]
fn scrolling_moves_the_same_board_to_another_slot() {
    let panel = area();
    let first = board_rects(panel, 0..3, 10);
    let second = board_rects(panel, 2..5, 10);
    assert_eq!(first.len(), 3);
    assert_eq!(second.len(), 3);

    for slot in 0..3 {
        assert!(
            (first[slot].1.x - second[slot].1.x).abs() < 0.01,
            "滚动后槽位 {slot} 的 x 变了（格子不该动）"
        );
    }

    let x2_first = current(&first, 2).expect("第一个窗口应含 index 2").x;
    let x2_second = current(&second, 2).expect("第二个窗口应含 index 2").x;
    assert!(
        x2_second < x2_first - 1.0,
        "同一块板向左滚动后位置没变小（{x2_first} → {x2_second}）"
    );

    // 注意：比较的是 `(下标, 矩形)` **配对**，不是纯矩形序列 —— 槽位本身是固定的，
    // 纯矩形序列在两次滚动间必须相同（这正是"格子不动"）。变的是下标与槽位的对应。
    assert_ne!(
        first, second,
        "两次布局的 (下标, 矩形) 配对完全相同（滚动没有改变板与槽位的对应）"
    );
}

/// 空输入一律返回空（不 panic、不给零面积矩形）。
#[test]
fn degenerate_inputs_yield_nothing() {
    let panel = area();
    assert!(board_rects(panel, 0..0, 0).is_empty(), "len=0");
    assert!(board_rects(panel, 0..0, 5).is_empty(), "空窗口");
    assert!(board_rects(panel, 3..3, 5).is_empty(), "起点=终点的窗口");
    assert!(
        board_rects(Rect::new(0.0, 0.0, 0.0, 100.0), 0..3, 3).is_empty(),
        "零宽面板"
    );
    assert!(
        board_rects(Rect::new(0.0, 0.0, 100.0, 0.0), 0..3, 3).is_empty(),
        "零高面板"
    );
}

// ---------------------------------------------------------------- ④ 当前块（首/尾）

/// 当前块必须能由**下标比对**取到，首尾两格尤其要测。
///
/// `index = 0` 的窗口是 `0..3`（窗口中间是 1）、`index = 9` 的窗口是 `7..10`
/// （窗口中间是 8）：任何"用窗口中间那块当选中"的实现都会在这两格指错块。
#[test]
fn the_current_board_is_found_by_index_even_at_both_ends() {
    let panel = area();
    let len = 10;
    for (index, window, slot) in [
        (0usize, 0usize..3usize, 0usize), // 首格 → 槽位 0
        (1, 0..3, 1),
        (5, 4..7, 1),  // 中间：正好落在窗口中间
        (9, 7..10, 2), // 尾格 → 槽位 2（不是窗口中间的 1）
    ] {
        let slots = board_rects(panel, window.clone(), len);
        assert_eq!(slots.len(), window.len().min(BOARD_SLOTS));
        let found = slots
            .iter()
            .position(|(at, _)| *at == index)
            .unwrap_or_else(|| panic!("window={window:?} 里没有当前块 index={index}：{slots:?}"));
        assert_eq!(
            found, slot,
            "index={index} 在 window={window:?} 里应落在槽位 {slot}，实际 {found}"
        );
        assert!(
            (slots[found].1.x - slots[slot].1.x).abs() < 0.01,
            "取到的当前块 x 与槽位 x 不一致"
        );
    }
}

// ---------------------------------------------------------------- ⑤ 选中态

/// 选中态的 PCB 必须有一条 2px 的 accent 外框，非选中态一条都没有。
///
/// 判据不能只是"出现过 accent 颜色"：两种状态都会用 accent 画顶部系统色条，
/// 但那条 alpha 是 0.95。可判定的差异是"颜色**恰好**等于 accent（alpha 1.0）
/// 且厚度 2px"，且外框由四条矩形组成（上下 h=2、左右 w=2）。
#[test]
fn only_the_selected_pcb_paints_a_two_pixel_accent_frame() {
    let accent = [0.20f32, 0.62, 0.90, 1.0];
    let rect = Rect::new(10.0, 20.0, 200.0, 140.0);

    let accent_frame = |out: &[Draw]| {
        out.iter()
            .filter(|draw| {
                colour_of(draw) == Some(accent)
                    && rect_of(draw).is_some_and(|r| {
                        (r.h - 2.0).abs() < 0.01 || (r.w - 2.0).abs() < 0.01
                    })
            })
            .count()
    };

    let mut selected = Vec::new();
    draw_pcb(&mut selected, rect, true, accent, None, None);
    let mut idle = Vec::new();
    draw_pcb(&mut idle, rect, false, accent, None, None);

    assert_eq!(
        accent_frame(&selected),
        4,
        "选中态的 2px accent 外框应由 4 条矩形组成（上下 + 左右）"
    );
    assert_eq!(
        accent_frame(&idle),
        0,
        "非选中态不该有 2px accent 外框（选中/非选中不可区分）"
    );
    assert!(
        selected.len() > idle.len(),
        "选中态 {} 条绘制没有比非选中态 {} 条更多",
        selected.len(),
        idle.len()
    );

    // 两种状态下所有绘制都必须在 PCB 内。
    for (label, out) in [("selected", &selected), ("idle", &idle)] {
        for draw in out.iter() {
            let Some(child) = rect_of(draw) else { continue };
            assert!(
                rect.contains(child, 0.5),
                "{label} 有一条绘制越出 PCB：{child:?} 不在 {rect:?} 内"
            );
        }
    }
}

// ---------------------------------------------------------------- ⑥ 柜体不越界

/// 柜内预览与书架上的 mini 柜子：任何绘制都不得超出传入面板。
///
/// `cabinet.rs` 的模块文档把这条写成硬约束（"单测钉住"）；这里独立再验一次，
/// 因为越界的后果是"盖到相邻卡带或状态栏"，720×480 上肉眼极难发现。
#[test]
fn the_cabinets_never_paint_outside_their_panel() {
    let panel = area();
    let skin = Skin::default();

    for (label, out) in [
        ("preview/6", {
            let mut out = Vec::new();
            draw_cabinet_preview(&mut out, panel, &skin, 6, None, None);
            out
        }),
        ("preview/4", {
            let mut out = Vec::new();
            draw_cabinet_preview(&mut out, panel, &skin, 4, None, None);
            out
        }),
        ("mini", {
            let mut out = Vec::new();
            draw_mini_cabinet(&mut out, panel, &skin);
            out
        }),
    ] {
        assert!(!out.is_empty(), "{label} 一条绘制都没有");
        for draw in &out {
            let Some(child) = rect_of(draw) else { continue };
            assert!(
                panel.contains(child, 0.5),
                "{label} 越界：{child:?} 不在 {panel:?} 内"
            );
        }
    }
}

/// 面板被压到极小时也不许越界（文档承诺"压到 4px 高也不会画到界外"）。
///
/// `24×40` 是书架上最窄的柜子槽位。
#[test]
fn tiny_panels_are_clipped_rather_than_overflowing() {
    let skin = Skin::default();
    let tiny = [
        Rect::new(0.0, 0.0, 4.0, 4.0),
        Rect::new(0.0, 0.0, 1.0, 200.0),
        Rect::new(0.0, 0.0, 200.0, 1.0),
        Rect::new(270.0, 150.0, 24.0, 40.0),
    ];
    for panel in tiny {
        let mut out = Vec::new();
        draw_cabinet_preview(&mut out, panel, &skin, 6, None, None);
        for draw in &out {
            let Some(child) = rect_of(draw) else { continue };
            assert!(
                panel.contains(child, 0.5),
                "极小面板 {panel:?} 下越界：{child:?}"
            );
        }

        let mut out = Vec::new();
        draw_mini_cabinet(&mut out, panel, &skin);
        for draw in &out {
            let Some(child) = rect_of(draw) else { continue };
            assert!(
                panel.contains(child, 0.5),
                "极小面板 {panel:?} 下 mini 柜越界：{child:?}"
            );
        }

        let mut out = Vec::new();
        draw_pcb(&mut out, panel, true, [0.2, 0.5, 0.8, 1.0], None, None);
        for draw in &out {
            let Some(child) = rect_of(draw) else { continue };
            assert!(
                panel.contains(child, 0.5),
                "极小面板 {panel:?} 下 PCB 越界：{child:?}"
            );
        }
    }
}

// ---------------------------------------------------------------- ⑦ 越界窗口（防御）

/// 窗口越过 `len` 时不许返回 `index >= len` 的槽位。
///
/// 生产路径 `visible_window` 永远给不出这种窗口，所以这条是**防御性**的：
/// `len` 是 `boards.len()`，一旦返回越界下标，调用方 `boards[index]` 会 panic。
/// 期望实现：把 `window.end` 裁到 `len`（然后由 `count == 0` 走空返回）。
#[test]
fn a_window_that_runs_past_the_end_is_clamped() {
    let panel = area();
    for (window, len) in [(8usize..12usize, 10usize), (10..13, 10), (9..12, 10)] {
        let slots = board_rects(panel, window.clone(), len);
        for (index, rect) in &slots {
            assert!(
                *index < len,
                "window={window:?} len={len} 返回了越界下标 {index}（rect={rect:?}）"
            );
        }
    }
    assert!(
        board_rects(panel, 10..13, 10).is_empty(),
        "完全越过 len 的窗口应当返回空"
    );
}

// 未使用告警抑制：`TexId` 只在 `titles` 类签名里出现，留着便于将来加标题断言。
const _: Option<TexId> = None;

// ---------------------------------------------------------------- ⑧ draw_boards（1/2/3 块真的被画出来）

/// 1/2/3 块都真的被画出来：指令数随块数单调增加，且全部落在面板内。
///
/// 这条是 `draw_boards` 落地之后补的（早前它只在 `draw_arcade` 里循环，归代码
/// 审查）。它同时验证"窗口里有几块就画几块"，以及 `titles` 比窗口短时不会 panic。
#[test]
fn one_two_and_three_boards_are_all_painted() {
    let panel = area();
    let accent = [0.2f32, 0.5, 0.8, 1.0];
    let mut counts = Vec::new();
    for (window, index) in [
        (0usize..1usize, 0usize),
        (0..2, 0),
        (0..3, 1),
    ] {
        let titles: Vec<Option<TexId>> = (0..window.len()).map(|_| None).collect();
        let mut out = Vec::new();
        draw_boards(&mut out, panel, window.clone(), index, 3, accent, &titles);
        assert!(!out.is_empty(), "window={window:?} 一条绘制都没有");
        for draw in &out {
            let Some(child) = rect_of(draw) else { continue };
            assert!(
                panel.contains(child, 0.5),
                "window={window:?} 越界：{child:?}"
            );
        }
        counts.push((window.len(), out.len()));
    }
    assert!(
        counts[0].1 < counts[1].1 && counts[1].1 < counts[2].1,
        "指令数没有随块数增加：{counts:?}"
    );

    // `titles` 比窗口短（甚至空）不许 panic：`draw_boards` 用的是 `titles.get(slot)`。
    let mut out = Vec::new();
    draw_boards(&mut out, panel, 0..3, 0, 3, accent, &[]);
    assert!(!out.is_empty(), "空 titles 时一条绘制都没有");
}

/// 选中块必须由**下标**决定，首/中/尾三处都只有一块带 4 条 2px accent 框。
///
/// `index=0` 的窗口是 `0..3`（窗口中间是 1）、`index=9` 的窗口是 `7..10`（中间是 8）：
/// 任何"用窗口中间那块当选中"的实现，在这两格都会红。
#[test]
fn only_the_selected_board_gets_the_accent_frame_at_both_ends() {
    let panel = area();
    let accent = [0.20f32, 0.62, 0.90, 1.0];
    let len = 10;

    for index in [0usize, 4, 5, 9] {
        let window = window_for(index, len);
        let titles: Vec<Option<TexId>> = (0..window.len()).map(|_| None).collect();
        let mut out = Vec::new();
        draw_boards(&mut out, panel, window.clone(), index, len, accent, &titles);

        let frames = out
            .iter()
            .filter(|draw| {
                colour_of(draw) == Some(accent)
                    && rect_of(draw).is_some_and(|r| {
                        (r.h - 2.0).abs() < 0.01 || (r.w - 2.0).abs() < 0.01
                    })
            })
            .count();
        assert_eq!(
            frames, 4,
            "index={index} window={window:?} 的 2px accent 框应为 4 条（只有选中那块），实际 {frames}"
        );

        // 反证：外框的包围盒必须落在 index 自己的槽位里，而不是"窗口中间那块"。
        let boxes: Vec<Rect> = out
            .iter()
            .filter(|draw| {
                colour_of(draw) == Some(accent)
                    && rect_of(draw).is_some_and(|r| {
                        (r.h - 2.0).abs() < 0.01 || (r.w - 2.0).abs() < 0.01
                    })
            })
            .filter_map(rect_of)
            .collect();
        let left = boxes.iter().map(|r| r.x).fold(f32::INFINITY, f32::min);
        let right = boxes.iter().map(|r| r.right()).fold(f32::NEG_INFINITY, f32::max);
        let own = current(&board_rects(panel, window.clone(), len), index)
            .copied()
            .expect("当前块应在窗口里");
        // 选中块的 PCB 会先 `inset(1.0)` 再画 2px 外框，所以外框比槽位每边内缩 1px；
        // **中心不变**，而"取错块"会让中心偏出半个槽位，所以按中心断言。
        let box_centre = (left + right) / 2.0;
        let own_centre = (own.x + own.right()) / 2.0;
        assert!(
            (box_centre - own_centre).abs() < 0.51,
            "index={index} 的 accent 框中心 {box_centre} 与自己的槽位中心 {own_centre} 不符\
             （像是按窗口中间取块）"
        );
        assert!(
            left >= own.x - 0.51 && right <= own.right() + 0.51,
            "index={index} 的 accent 框 [{left},{right}] 越出自己的槽位 [{},{}]",
            own.x,
            own.right()
        );
    }
}
