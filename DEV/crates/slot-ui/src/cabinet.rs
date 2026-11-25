//! 街机柜与 JAMMA PCB 的绘制。
//!
//! 画布原语只有 `Draw::Rect / Tex / Turned`（没有圆角、没有渐变、没有描边），
//! 所以这里的柜子全部是矩形叠出来的：机身三层、marquee 背光、CRT 玻璃 + 四条
//! 1px 内框、投币门、控制面板、摇杆球头。
//!
//! 一条硬约束：**任何绘制都不能超出传入的 `rect`**。所有子矩形都经过 [`fit`]
//! 与父矩形求交，即使面板被压到 4px 高也不会画到界外（单测钉住）。
//! 这条约束的用处不只是"好看"：书架上的卡带槽位是浮点算出来的，柜子一旦越界
//! 就会盖到相邻卡带或状态栏上，而那种错在 720×480 上肉眼极难发现。

use std::ops::Range;

use slot_gfx::{Draw, TexId};

/// 一屏 PCB 的格数。
pub const BOARD_SLOTS: usize = 3;
/// PCB 之间的间距。
pub const BOARD_GAP: f32 = 6.0;

/// 轴对齐矩形（左上角 + 宽高）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }

    pub fn right(self) -> f32 {
        self.x + self.w
    }

    pub fn bottom(self) -> f32 {
        self.y + self.h
    }

    pub fn inset(self, d: f32) -> Rect {
        self.inset_each(d, d, d, d)
    }

    pub fn inset_each(self, left: f32, top: f32, right: f32, bottom: f32) -> Rect {
        Rect {
            x: self.x + left,
            y: self.y + top,
            w: (self.w - left - right).max(0.0),
            h: (self.h - top - bottom).max(0.0),
        }
    }

    /// 是否包含另一个矩形（`slack` 是像素容差）。
    pub fn contains(self, other: Rect, slack: f32) -> bool {
        other.x >= self.x - slack
            && other.y >= self.y - slack
            && other.right() <= self.right() + slack
            && other.bottom() <= self.bottom() + slack
    }

    /// 把子矩形裁进父矩形，保证不越界。
    pub fn fit(self, child: Rect) -> Rect {
        let x = child.x.max(self.x);
        let y = child.y.max(self.y);
        let right = child.right().min(self.right());
        let bottom = child.bottom().min(self.bottom());
        Rect::new(x, y, (right - x).max(0.0), (bottom - y).max(0.0))
    }

    fn centre_x(self, w: f32) -> f32 {
        self.x + (self.w - w) / 2.0
    }
}

/// 柜子/PCB 的配色与可选贴图。
#[derive(Clone, Copy, Debug, Default)]
pub struct Skin {
    /// 机种主色（`collection_style::platform(p).base`，已归一化到 0..1）。
    pub accent: [f32; 4],
    /// marquee 上的文字（选中板标题；没有就只画背光）。
    pub marquee: Option<TexId>,
    /// CRT 里的 attract 占位（没有就用标题顶替）。
    pub crt: Option<TexId>,
    /// 选中呼吸灯 0..1。
    pub glow: f32,
}

/// 三块一屏的 PCB 布局。
///
/// 槽宽恒定（`area` 三等分减间距），`window` 里有几块就摆几块：1 块居中、
/// 2 块分列、3 块铺满。滚动时**板子换格**（窗口起点变了，同一块板落到左边的
/// 格子里），而格子本身不动——这样选中块在首尾之外永远居中。
pub fn board_rects(area: Rect, window: Range<usize>, len: usize) -> Vec<(usize, Rect)> {
    if len == 0 || area.w <= 0.0 || area.h <= 0.0 {
        return Vec::new();
    }
    // 防御：窗口可能来自外部（`visible_window` 之外的地方也会构造），越出列表
    // 末尾的下标绝不能出现在结果里——调用方会拿它去索引 `boards`。
    let start = window.start.min(len);
    let end = window.end.min(len);
    let count = end.saturating_sub(start).min(BOARD_SLOTS);
    if count == 0 {
        return Vec::new();
    }
    // 板宽恒定、**浮点不取整**：`3*slot_w + 2*GAP == area.w`，三块严格铺满
    // （取整会在右侧留 1px 缝，同一块板在 1/2/3 块时宽度也会差一点点）。
    let slot_w = ((area.w - BOARD_GAP * (BOARD_SLOTS as f32 - 1.0)) / BOARD_SLOTS as f32).max(1.0);
    let total = slot_w * count as f32 + BOARD_GAP * (count.saturating_sub(1)) as f32;
    // 三块铺满时左对齐（余数留右）；只剩 1/2 块时整体居中。
    let start_x = if count == BOARD_SLOTS {
        area.x
    } else {
        area.centre_x(total)
    };
    (0..count)
        .map(|slot| {
            let x = start_x + slot as f32 * (slot_w + BOARD_GAP);
            let rect = Rect::new(x, area.y, slot_w, area.h);
            (start + slot, area.fit(rect))
        })
        .collect()
}

/// 面板里最大的 4:3 内接矩形（居中）。
pub fn crt_rect(panel: Rect) -> Rect {
    const ASPECT: f32 = 4.0 / 3.0;
    let inner = panel.inset(4.0);
    if inner.w <= 0.0 || inner.h <= 0.0 {
        return inner;
    }
    let (mut w, mut h) = (inner.w, inner.w / ASPECT);
    if h > inner.h {
        h = inner.h;
        w = h * ASPECT;
    }
    Rect::new(
        inner.centre_x(w),
        inner.y + (inner.h - h) / 2.0,
        w,
        h,
    )
}

fn fill(out: &mut Vec<Draw>, rect: Rect, colour: [f32; 4]) {
    if rect.w <= 0.0 || rect.h <= 0.0 {
        return;
    }
    out.push(Draw::Rect {
        x: rect.x,
        y: rect.y,
        w: rect.w,
        h: rect.h,
        colour,
    });
}

/// 在 `parent` 里画一块子矩形（自动裁进父矩形）。
fn fill_in(out: &mut Vec<Draw>, parent: Rect, child: Rect, colour: [f32; 4]) {
    fill(out, parent.fit(child), colour);
}

/// 四条窄矩形拼出的内框（对齐在 `rect` 内侧）。
fn frame_in(out: &mut Vec<Draw>, parent: Rect, rect: Rect, px: f32, colour: [f32; 4]) {
    if px <= 0.0 {
        return;
    }
    fill_in(out, parent, Rect::new(rect.x, rect.y, rect.w, px), colour);
    fill_in(
        out,
        parent,
        Rect::new(rect.x, rect.bottom() - px, rect.w, px),
        colour,
    );
    fill_in(out, parent, Rect::new(rect.x, rect.y, px, rect.h), colour);
    fill_in(
        out,
        parent,
        Rect::new(rect.right() - px, rect.y, px, rect.h),
        colour,
    );
}

fn tex_in(out: &mut Vec<Draw>, parent: Rect, rect: Rect, tex: TexId, alpha: f32) {
    let rect = parent.fit(rect);
    if rect.w <= 0.0 || rect.h <= 0.0 {
        return;
    }
    out.push(Draw::Tex {
        x: rect.x,
        y: rect.y,
        w: rect.w,
        h: rect.h,
        tex,
        alpha,
    });
}

/// 书架尺度的整台柜子：机身 + marquee 背光 + CRT + 投币门 + 控制面板。
pub fn draw_mini_cabinet(out: &mut Vec<Draw>, rect: Rect, skin: &Skin) {
    if rect.w <= 0.0 || rect.h <= 0.0 {
        return;
    }
    let accent = skin.accent;
    let glow = skin.glow.clamp(0.0, 1.0);

    // 机身三层：外框 → 机身 → 内阴影。
    fill(out, rect, [0.10, 0.11, 0.13, 1.0]);
    let body = rect.inset(2.0);
    fill_in(out, rect, body, [0.05, 0.055, 0.07, 1.0]);
    let shade = body.inset_each(3.0, 3.0, 1.0, 1.0);
    fill_in(out, rect, shade, [0.035, 0.04, 0.05, 1.0]);

    // marquee：低透明度外扩 + 内芯。
    let marquee = Rect::new(body.x, body.y, body.w, (body.h * 0.14).max(8.0));
    fill_in(
        out,
        rect,
        marquee.inset(-2.0),
        [accent[0] * 0.4, accent[1] * 0.4, accent[2] * 0.4, 0.45 + 0.35 * glow],
    );
    fill_in(
        out,
        rect,
        marquee,
        [accent[0] * 0.9, accent[1] * 0.9, accent[2] * 0.9, 0.9],
    );
    if let Some(tex) = skin.marquee {
        tex_in(out, rect, marquee.inset(2.0), tex, 0.95);
    }

    // CRT：深色玻璃 + 1px 内框。
    let screen = Rect::new(
        body.x + 5.0,
        marquee.bottom() + 4.0,
        body.w - 10.0,
        (body.h * 0.46).max(6.0),
    );
    fill_in(out, rect, screen, [0.015, 0.02, 0.025, 1.0]);
    frame_in(out, rect, screen, 1.0, [0.12, 0.13, 0.16, 1.0]);
    if let Some(tex) = skin.crt {
        tex_in(out, rect, crt_rect(screen), tex, 0.85);
    }

    // 投币门：门板 + 两条投币口 + 锁孔。
    let coin = Rect::new(
        body.x + 4.0,
        (screen.bottom() + 4.0).min(body.bottom() - body.h * 0.24),
        (body.w * 0.34).max(6.0),
        (body.h * 0.18).max(6.0),
    );
    fill_in(out, rect, coin, [0.09, 0.095, 0.11, 1.0]);
    for index in 0..2 {
        let slot = Rect::new(
            coin.x + coin.w * (0.18 + 0.38 * index as f32),
            coin.y + coin.h * 0.24,
            (coin.w * 0.12).max(1.5),
            (coin.h * 0.52).max(2.0),
        );
        fill_in(out, rect, slot, [0.02, 0.02, 0.03, 1.0]);
    }
    let lock = Rect::new(
        coin.x + coin.w * 0.82,
        coin.y + coin.h * 0.42,
        (coin.w * 0.10).max(1.5),
        (coin.w * 0.10).max(1.5),
    );
    fill_in(out, rect, lock, [accent[0], accent[1], accent[2], 0.9]);

    // 控制面板 + 按键排。
    let panel = Rect::new(
        body.x + 2.0,
        body.bottom() - (body.h * 0.16).max(5.0),
        body.w - 4.0,
        (body.h * 0.14).max(4.0),
    );
    fill_in(out, rect, panel, [0.10, 0.105, 0.12, 1.0]);
    for column in 0..3 {
        let button = Rect::new(
            panel.x + panel.w * (0.42 + 0.15 * column as f32),
            panel.y + panel.h * 0.28,
            (panel.w * 0.09).max(1.5),
            (panel.h * 0.42).max(1.5),
        );
        fill_in(out, rect, button, [accent[0] * 0.8, accent[1] * 0.8, accent[2] * 0.8, 1.0]);
    }
    // 摇杆：底座 + 杆 + 球头。
    let base = Rect::new(
        panel.x + panel.w * 0.12,
        panel.y + panel.h * 0.58,
        (panel.w * 0.16).max(2.0),
        (panel.h * 0.22).max(1.5),
    );
    fill_in(out, rect, base, [0.06, 0.065, 0.08, 1.0]);
    let rod = Rect::new(
        base.x + base.w * 0.4,
        panel.y + panel.h * 0.18,
        (base.w * 0.2).max(1.0),
        (panel.h * 0.5).max(1.0),
    );
    fill_in(out, rect, rod, [0.14, 0.15, 0.18, 1.0]);
    let ball = Rect::new(
        rod.x - (rod.w * 0.6).max(0.5),
        rod.y - (rod.w * 0.9).max(1.0),
        (rod.w * 2.2).max(1.5),
        (rod.w * 1.6).max(1.5),
    );
    fill_in(out, rect, ball, [accent[0], accent[1], accent[2], 1.0]);
}

/// 柜内预览的各块矩形。
///
/// 5a 要在 CRT 上叠消磁白光、在投币口上方画 token、在左下角写 `CREDIT`，
/// 这些覆盖层必须和柜子里的部件**同一个矩形**，所以把布局抽出来共享；
/// `draw_cabinet_preview` 自己也用这份，避免两处算两套坐标。
#[derive(Clone, Copy, Debug)]
pub struct PreviewParts {
    pub body: Rect,
    pub marquee: Rect,
    pub screen_panel: Rect,
    /// 4:3 内接矩形（真正"画面"的区域）。
    pub screen: Rect,
    pub coin: Rect,
    pub panel: Rect,
}

/// 按柜内预览的尺寸算各部件矩形（全部已裁进 `rect`）。
pub fn preview_parts(rect: Rect) -> PreviewParts {
    let body = rect.inset(3.0);
    let marquee = Rect::new(body.x + 10.0, body.y + 8.0, body.w - 20.0, 34.0);
    let screen_panel = Rect::new(
        body.x + 16.0,
        marquee.bottom() + 10.0,
        body.w - 32.0,
        (body.h * 0.52).max(20.0),
    );
    let coin_h = (body.h * 0.13).max(16.0);
    let coin = Rect::new(
        body.x + 16.0,
        body.bottom() - coin_h - (body.h * 0.22).max(28.0),
        (body.w * 0.30).max(30.0),
        coin_h,
    );
    let panel = Rect::new(
        body.x + 10.0,
        body.bottom() - (body.h * 0.12).max(20.0),
        body.w - 20.0,
        (body.h * 0.10).max(16.0),
    );
    PreviewParts {
        body: rect.fit(body),
        marquee: rect.fit(marquee),
        screen_panel: rect.fit(screen_panel),
        screen: rect.fit(crt_rect(screen_panel)),
        coin: rect.fit(coin),
        panel: rect.fit(panel),
    }
}

/// 主界面左侧的柜内预览：marquee（标题）+ 4:3 CRT + 几何文案 + 投币门 + 面板。
///
/// `buttons` 是按键数：Arcade 6 键两排 3+3，NeoGeo 4 键一排。
pub fn draw_cabinet_preview(
    out: &mut Vec<Draw>,
    rect: Rect,
    skin: &Skin,
    buttons: u8,
    title: Option<TexId>,
    geometry: Option<TexId>,
) {
    if rect.w <= 0.0 || rect.h <= 0.0 {
        return;
    }
    let accent = skin.accent;
    let glow = skin.glow.clamp(0.0, 1.0);

    let parts = preview_parts(rect);
    // 柜体：外框 + 机身 + 内阴影（三层，做出厚度）。
    fill(out, rect, [0.09, 0.10, 0.12, 1.0]);
    let body = rect.inset(3.0);
    fill_in(out, rect, body, [0.06, 0.065, 0.08, 1.0]);
    fill_in(
        out,
        rect,
        body.inset_each(6.0, 6.0, 3.0, 3.0),
        [0.04, 0.045, 0.055, 1.0],
    );

    // marquee 背光条 + 标题。
    let marquee = parts.marquee;
    fill_in(
        out,
        rect,
        marquee.inset(-4.0),
        [accent[0] * 0.45, accent[1] * 0.45, accent[2] * 0.45, 0.35 + 0.35 * glow],
    );
    fill_in(
        out,
        rect,
        marquee,
        [accent[0] * 0.85, accent[1] * 0.85, accent[2] * 0.85, 0.95],
    );
    frame_in(out, rect, marquee, 1.0, [0.05, 0.05, 0.07, 1.0]);
    if let Some(tex) = skin.marquee.or(title) {
        tex_in(out, rect, marquee.inset(6.0), tex, 1.0);
    }

    // CRT：暗玻璃 + 1px 内框 + attract 占位。
    let screen_panel = parts.screen_panel;
    fill_in(out, rect, screen_panel, [0.03, 0.035, 0.045, 1.0]);
    frame_in(out, rect, screen_panel, 1.0, [0.13, 0.14, 0.17, 1.0]);
    let screen = parts.screen;
    fill_in(out, rect, screen, [0.012, 0.016, 0.022, 1.0]);
    match skin.crt.or(title) {
        Some(tex) => tex_in(out, rect, screen.inset(8.0), tex, 0.9),
        None => {
            // 没有标题纹理时也给一块"有信号"的暗绿底，避免纯黑看起来像关机。
            fill_in(
                out,
                rect,
                screen.inset(8.0),
                [0.04, 0.10, 0.06, 0.7],
            );
        }
    }
    // 屏幕下方的几何文案（`304×224 · 2x` 之类）。
    if let Some(tex) = geometry {
        let line = Rect::new(screen.x, screen.bottom() + 4.0, screen.w, 12.0);
        tex_in(out, rect, line, tex, 0.8);
    }

    // 投币门：门板 + 两条投币口 + 锁孔。
    let coin = parts.coin;
    fill_in(out, rect, coin, [0.10, 0.105, 0.12, 1.0]);
    frame_in(out, rect, coin, 1.0, [0.16, 0.17, 0.20, 1.0]);
    for index in 0..2 {
        let slot = Rect::new(
            coin.x + coin.w * (0.16 + 0.34 * index as f32),
            coin.y + coin.h * 0.26,
            (coin.w * 0.10).max(3.0),
            (coin.h * 0.46).max(4.0),
        );
        fill_in(out, rect, slot, [0.02, 0.02, 0.03, 1.0]);
    }
    let lock = Rect::new(
        coin.x + coin.w * 0.80,
        coin.y + coin.h * 0.44,
        (coin.h * 0.28).max(3.0),
        (coin.h * 0.28).max(3.0),
    );
    fill_in(out, rect, lock, [accent[0], accent[1], accent[2], 0.95]);

    // 控制面板：摇杆 + 按键（Arcade 6 键 3+3，NeoGeo 4 键一排）。
    let panel = parts.panel;
    fill_in(out, rect, panel, [0.11, 0.115, 0.13, 1.0]);
    frame_in(out, rect, panel, 1.0, [0.17, 0.18, 0.21, 1.0]);

    let stick = Rect::new(
        panel.x + panel.w * 0.10,
        panel.y + panel.h * 0.30,
        (panel.w * 0.10).max(8.0),
        (panel.h * 0.55).max(6.0),
    );
    fill_in(out, rect, stick, [0.05, 0.055, 0.07, 1.0]);
    let rod = Rect::new(
        stick.x + stick.w * 0.42,
        panel.y + panel.h * 0.10,
        (stick.w * 0.16).max(2.0),
        (panel.h * 0.5).max(3.0),
    );
    fill_in(out, rect, rod, [0.16, 0.17, 0.20, 1.0]);
    let ball = Rect::new(
        rod.x - (rod.w * 0.7).max(1.0),
        rod.y - (rod.w * 1.1).max(2.0),
        (rod.w * 2.4).max(4.0),
        (rod.w * 1.8).max(3.0),
    );
    fill_in(out, rect, ball, [accent[0], accent[1], accent[2], 1.0]);

    let per_row = if buttons >= 6 { 3 } else { 2 };
    let rows = if buttons >= 6 { 2 } else { 1 };
    let button_w = (panel.w * 0.055).max(4.0);
    let button_h = (panel.h * 0.26).max(3.0);
    for row in 0..rows {
        for column in 0..per_row {
            let button = Rect::new(
                panel.x + panel.w * 0.40 + column as f32 * (button_w * 1.6),
                panel.y + panel.h * (0.16 + 0.40 * row as f32),
                button_w,
                button_h,
            );
            fill_in(out, rect, button, [accent[0] * 0.75, accent[1] * 0.75, accent[2] * 0.75, 1.0]);
            frame_in(out, rect, button, 1.0, [0.20, 0.21, 0.24, 1.0]);
        }
    }
}

/// 一块 JAMMA PCB：板身 + 顶部系统色条 + 金手指 + 两颗芯片 + 丝印框。
pub fn draw_pcb(
    out: &mut Vec<Draw>,
    rect: Rect,
    selected: bool,
    accent: [f32; 4],
    title: Option<TexId>,
    meta: Option<TexId>,
) {
    if rect.w <= 0.0 || rect.h <= 0.0 {
        return;
    }
    let board = if selected { rect.inset(1.0) } else { rect };

    // 选中：外框 2px accent + 一层低透明底光（都在 rect 内）。
    if selected {
        fill_in(
            out,
            rect,
            board,
            [accent[0] * 0.10, accent[1] * 0.10, accent[2] * 0.10, 0.6],
        );
        frame_in(out, rect, board, 2.0, accent);
    } else {
        fill_in(out, rect, board, [0.07, 0.075, 0.09, 1.0]);
        frame_in(out, rect, board, 1.0, [0.14, 0.15, 0.18, 1.0]);
    }

    // 板身（深绿黑）+ 丝印白框。
    let pcb = board.inset_each(6.0, 8.0, 6.0, 10.0);
    fill_in(out, rect, pcb, [0.05, 0.09, 0.07, 1.0]);
    frame_in(out, rect, pcb, 1.0, [0.55, 0.60, 0.55, 0.35]);

    // 顶部系统色条。
    let bar = Rect::new(pcb.x, pcb.y, pcb.w, 4.0);
    fill_in(out, rect, bar, [accent[0], accent[1], accent[2], 0.95]);

    // 两颗芯片 + 引脚。
    let chip_w = (pcb.w * 0.42).max(6.0);
    let chip_h = (pcb.h * 0.18).max(4.0);
    for (index, x_ratio) in [0.10f32, 0.46].iter().enumerate() {
        let chip = Rect::new(
            pcb.x + pcb.w * x_ratio,
            pcb.y + pcb.h * (0.24 + 0.26 * index as f32),
            chip_w,
            chip_h,
        );
        fill_in(out, rect, chip, [0.03, 0.03, 0.035, 1.0]);
        for pin in 0..4 {
            let pins = Rect::new(
                chip.x + chip.w * (0.12 + 0.22 * pin as f32),
                chip.y + chip.h,
                (chip.w * 0.06).max(1.0),
                (chip.h * 0.18).max(1.0),
            );
            fill_in(out, rect, pins, [0.35, 0.36, 0.38, 1.0]);
        }
    }

    // 金手指：底部等距竖条。
    let finger_h = (pcb.h * 0.16).max(4.0);
    let finger_y = pcb.bottom() - finger_h;
    for index in 0..10 {
        let finger = Rect::new(
            pcb.x + pcb.w * (0.06 + 0.088 * index as f32),
            finger_y,
            (pcb.w * 0.05).max(1.0),
            finger_h,
        );
        fill_in(out, rect, finger, [0.72, 0.58, 0.22, 1.0]);
    }

    // 贴纸标题 + 元信息。
    if let Some(tex) = title {
        let area = Rect::new(pcb.x + 4.0, pcb.y + 8.0, pcb.w - 8.0, (pcb.h * 0.14).max(8.0));
        tex_in(out, rect, area, tex, 1.0);
    }
    if let Some(tex) = meta {
        let area = Rect::new(
            pcb.x + 4.0,
            pcb.y + pcb.h * 0.72,
            pcb.w - 8.0,
            (pcb.h * 0.10).max(6.0),
        );
        tex_in(out, rect, area, tex, 0.8);
    }

    // 选中：底部一条 accent 条（"这块被选中"）。
    if selected {
        let strip = Rect::new(board.x + 4.0, board.bottom() - 6.0, board.w - 8.0, 4.0);
        fill_in(out, rect, strip, [accent[0], accent[1], accent[2], 1.0]);
    }
}

/// 把"可见窗口 → 若干块 PCB"这一步收进来。
///
/// 之前这段在 `draw_arcade` 里，测试只能读代码；挪到这里之后，"窗口里有几块就
/// 画几块、只有选中那块带 accent 外框"是可数的（QA 直接数 `Draw` 指令流）。
///
/// `titles` 按**窗口内槽位**给：`titles[slot]` 是那一格要贴的标题纹理，缺省为
/// `None`（非选中块不贴标题）。`index` 是当前选中板的**列表下标**，不是槽位——
/// 首尾两格（`index == 0` / `index == len-1`）正是"用窗口中心"会取错的地方。
pub fn draw_boards(
    out: &mut Vec<Draw>,
    area: Rect,
    window: Range<usize>,
    index: usize,
    len: usize,
    accent: [f32; 4],
    titles: &[Option<TexId>],
) {
    for (slot, (board, rect)) in board_rects(area, window, len).into_iter().enumerate() {
        let selected = board == index;
        let title = titles.get(slot).copied().flatten();
        draw_pcb(out, rect, selected, accent, title, None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skin() -> Skin {
        Skin {
            accent: [0.85, 0.45, 0.20, 1.0],
            marquee: Some(TexId::from_raw(7)),
            crt: Some(TexId::from_raw(9)),
            glow: 0.75,
        }
    }

    /// 与 `slot::arcade::visible_window` 同一条规则（这里不反向依赖 `slot`）。
    fn window_for(index: usize, len: usize) -> Range<usize> {
        let start = index.saturating_sub(1).min(len.saturating_sub(BOARD_SLOTS));
        start..(start + BOARD_SLOTS).min(len)
    }

    fn rects(draws: &[Draw]) -> Vec<Rect> {
        draws
            .iter()
            .filter_map(|draw| match draw {
                Draw::Rect { x, y, w, h, .. } => Some(Rect::new(*x, *y, *w, *h)),
                _ => None,
            })
            .collect()
    }

    fn assert_inside(draws: &[Draw], panel: Rect, what: &str) {
        for rect in rects(draws) {
            assert!(
                panel.contains(rect, 0.5),
                "{what}: 画到面板外了：{rect:?} 不在 {panel:?} 里"
            );
        }
    }

    #[test]
    fn every_cabinet_rect_stays_inside_its_panel() {
        let panel = Rect::new(20.0, 56.0, 310.0, 376.0);
        let mut draws = Vec::new();
        draw_mini_cabinet(&mut draws, Rect::new(160.0, 96.0, 120.0, 220.0), &skin());
        assert_inside(&draws, Rect::new(160.0, 96.0, 120.0, 220.0), "mini");

        let mut draws = Vec::new();
        draw_cabinet_preview(
            &mut draws,
            panel,
            &skin(),
            6,
            Some(TexId::from_raw(3)),
            Some(TexId::from_raw(4)),
        );
        assert_inside(&draws, panel, "preview");

        // 压到极小的面板也不能越界（书架上最窄的槽位）。
        let mut draws = Vec::new();
        draw_cabinet_preview(&mut draws, Rect::new(0.0, 0.0, 24.0, 40.0), &skin(), 4, None, None);
        assert_inside(&draws, Rect::new(0.0, 0.0, 24.0, 40.0), "preview-tiny");

        let mut draws = Vec::new();
        draw_pcb(
            &mut draws,
            Rect::new(348.0, 84.0, 110.0, 202.0),
            true,
            [0.9, 0.4, 0.2, 1.0],
            Some(TexId::from_raw(5)),
            Some(TexId::from_raw(6)),
        );
        assert_inside(
            &draws,
            Rect::new(348.0, 84.0, 110.0, 202.0),
            "pcb-selected",
        );
    }

    #[test]
    fn board_rects_centres_one_and_two_boards() {
        let area = Rect::new(348.0, 84.0, 364.0, 202.0);
        let three = board_rects(area, 0..3, 10);
        assert_eq!(three.len(), 3);
        let widths: Vec<f32> = three.iter().map(|(_, rect)| rect.w).collect();
        assert!(
            widths.windows(2).all(|pair| (pair[0] - pair[1]).abs() < 0.01),
            "三块必须等宽：{widths:?}"
        );
        assert!((three[0].1.x - area.x).abs() < 0.01, "三块要左对齐");
        // 浮点板宽：三块严格铺满，只允许浮点误差。
        assert!(
            (three[2].1.right() - area.right()).abs() < 0.5,
            "三块没有铺满：末块右边 {} vs {}",
            three[2].1.right(),
            area.right()
        );

        let one = board_rects(area, 4..5, 10);
        assert_eq!(one.len(), 1);
        let centre = area.x + area.w / 2.0;
        assert!(
            (one[0].1.x + one[0].1.w / 2.0 - centre).abs() < 0.01,
            "单块要居中：{:?}",
            one[0].1
        );

        let two = board_rects(area, 4..6, 10);
        assert_eq!(two.len(), 2);
        assert!(two[0].1.right() <= two[1].1.x, "两块不能重叠");
        let span = two[1].1.right() - two[0].1.x;
        assert!((span - (two[0].1.w * 2.0 + BOARD_GAP)).abs() < 0.01);
    }

    #[test]
    fn a_scrolled_window_moves_the_boards_left() {
        let area = Rect::new(348.0, 84.0, 364.0, 202.0);
        // 第 2 块板：窗口 0..3 时在第三格，窗口 2..5 时落到第一格。
        let first = board_rects(area, 0..3, 10);
        let second = board_rects(area, 2..5, 10);
        let before = first.iter().find(|(index, _)| *index == 2).expect("board 2").1;
        let after = second
            .iter()
            .find(|(index, _)| *index == 2)
            .expect("board 2")
            .1;
        assert!(
            after.x < before.x,
            "滚动后同一块板应当左移：{before:?} → {after:?}"
        );
        // 窗口内容也换了（第一格不再是 0 号板）。
        assert_eq!(first[0].0, 0);
        assert_eq!(second[0].0, 2);
    }

    /// 窗口越界（`start >= len` 或 `end > len`）不能吐出越界下标。
    ///
    /// 调用方会拿返回的下标去索引 `boards`，多一个 `len` 就是一次 panic。
    #[test]
    fn the_window_never_returns_a_slot_past_the_list() {
        let area = Rect::new(348.0, 84.0, 364.0, 202.0);
        assert!(board_rects(area, 10..13, 10).is_empty(), "整段越界应为空");
        assert!(board_rects(area, 0..3, 0).is_empty(), "空列表");
        let tail = board_rects(area, 8..12, 10);
        assert_eq!(tail.len(), 2, "只保留 8/9 两块：{tail:?}");
        assert!(tail.iter().all(|(index, _)| *index < 10));
        // 布局仍然在面板内。
        for (_, rect) in tail {
            assert!(area.contains(rect, 0.5));
        }
    }

    /// 选中态跟着 **下标** 走，不是"窗口中间那块"。
    ///
    /// 首尾两格是唯一能区分这两种实现的地方：`index = 0` 时窗口 0..3 的中间是 1，
    /// `index = 9` 时窗口 7..10 的中间是 8，都会取错。
    #[test]
    fn the_selection_follows_the_index_not_the_window_centre() {
        let area = Rect::new(348.0, 84.0, 364.0, 202.0);
        let accent = [0.9f32, 0.4, 0.2, 1.0];
        for index in [0usize, 4, 9] {
            let window = window_for(index, 10);
            let mut draws = Vec::new();
            draw_boards(&mut draws, area, window.clone(), index, 10, accent, &[]);
            // 选中块的 2px accent 外框 = 4 条（上下 h=2、左右 w=2，alpha 1.0）。
            let frames: Vec<Rect> = draws
                .iter()
                .filter_map(|draw| match draw {
                    Draw::Rect { x, y, w, h, colour }
                        if (*w - 2.0).abs() < 0.01 || (*h - 2.0).abs() < 0.01 =>
                    {
                        (colour[3] > 0.99
                            && (colour[0] - accent[0]).abs() < 0.01
                            && (colour[1] - accent[1]).abs() < 0.01
                            && (colour[2] - accent[2]).abs() < 0.01)
                            .then(|| Rect::new(*x, *y, *w, *h))
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(frames.len(), 4, "index={index} 应当只有一块带 2px 外框");
            let x = frames.iter().map(|rect| rect.x).fold(f32::MAX, f32::min);
            let right = frames.iter().map(|rect| rect.right()).fold(0.0, f32::max);
            let slot = board_rects(area, window.clone(), 10)
                .into_iter()
                .find(|(board, _)| *board == index)
                .expect("选中块必须在窗口里")
                .1;
            assert!(
                (x - slot.x).abs() < 1.5 && (right - slot.right()).abs() < 1.5,
                "index={index} 的 accent 框没落在它自己的槽位上：{x}..{right} vs {slot:?}"
            );
        }
    }

    #[test]
    fn the_selected_pcb_draws_more_than_an_unselected_one() {
        let area = Rect::new(348.0, 84.0, 110.0, 202.0);
        let accent = [0.9f32, 0.4, 0.2, 1.0];
        let mut plain = Vec::new();
        draw_pcb(&mut plain, area, false, accent, None, None);
        let mut selected = Vec::new();
        draw_pcb(&mut selected, area, true, accent, None, None);

        assert!(
            selected.len() > plain.len(),
            "选中态应当多画至少一条（发光/底条）：{} vs {}",
            selected.len(),
            plain.len()
        );
        let has_accent = selected.iter().any(|draw| match draw {
            Draw::Rect { colour, .. } => {
                (colour[0] - accent[0]).abs() < 0.01
                    && (colour[1] - accent[1]).abs() < 0.01
                    && (colour[2] - accent[2]).abs() < 0.01
            }
            _ => false,
        });
        assert!(has_accent, "选中态缺 accent 色条");
    }
}
