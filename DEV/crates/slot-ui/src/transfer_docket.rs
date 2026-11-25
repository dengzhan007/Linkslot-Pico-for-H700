//! The cart-transfer "transfer docket": a cream paper manifest printed by the
//! console while the card stays in the slot.  The app moves this finished
//! face; the percentage, bar fill, check boxes and stamp are dynamic.
//!
//! The docket reports *real* byte progress.  Unlike the cloud-sync screen,
//! which only knows stages, this one has a legitimate percentage; the manifest
//! below it makes that percentage meaningful.

use crate::plate::UndoFace;
use crate::switch_style as sw;

pub const DOCKET_W: u32 = 560;
pub const DOCKET_H: u32 = 320;

/// Where the docket sits on the 720x480 screen.
pub const DOCKET_AT: (f32, f32) = (80.0, 88.0);
/// Where the cartridge thumbnail is pasted by the app.
pub const CARD_FRAME: (f32, f32, f32, f32) = (96.0, 162.0, 90.0, 51.0);
pub const CARD_INNER: (f32, f32, f32, f32) = (99.0, 165.0, 84.0, 47.0);
/// Dynamic text anchors (absolute screen coordinates).
pub const PERCENT_AT: (f32, f32) = (96.0, 240.0);
pub const BYTES_AT: (f32, f32) = (240.0, 264.0);
pub const BAR: (f32, f32, f32, f32) = (240.0, 252.0, 384.0, 6.0);
/// Manifest rows, absolute Y.
pub const ROW_Y: [f32; 4] = [310.0, 328.0, 346.0, 364.0];
pub const COL_L: f32 = 96.0;
pub const COL_R: f32 = 380.0;
pub const COL_L_RIGHT: f32 = 312.0;
pub const COL_R_RIGHT: f32 = 624.0;
pub const CHECK_PX: u32 = 10;
pub const CHECK_DY: f32 = 2.0;
/// The per-item mini bar runs from `x + 16` to `right - MINI_BAR_RIGHT`.
pub const MINI_BAR_DY: f32 = 15.0;
pub const MINI_BAR_RIGHT: f32 = 44.0;
/// The completion stamp's centre, absolute.
pub const STAMP_CENTRE: (f32, f32) = (600.0, 112.0);

pub const PAPER: [u8; 4] = [232, 226, 205, 255];
pub const PAPER_BORDER: [u8; 4] = [201, 195, 180, 255];
pub const PAPER_EDGE: [u8; 4] = [181, 174, 156, 255];
pub const PAPER_INK: [u8; 3] = [46, 41, 34];
pub const PAPER_DIM: [u8; 3] = [98, 92, 79];
pub const PAPER_FAINT: [u8; 3] = [102, 95, 81];
pub const HOST_DOT: [u8; 4] = [23, 107, 83, 255];
pub const JOIN_DOT: [u8; 4] = [40, 91, 148, 255];

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DocketFlow {
    Waiting,
    Sending,
    Receiving,
    Done,
}

/// One label/size pair in the manifest.  The label is already localised and
/// the size is already formatted, so this module stays i18n-free.
#[derive(Copy, Clone, Debug)]
pub struct DocketAsset<'a> {
    pub label: &'a str,
    pub size: &'a str,
}

/// Everything the static docket face prints.  `reverse` mirrors the arrow for
/// a receive; the local device always stays on the left.
pub struct DocketView<'a> {
    pub flow: DocketFlow,
    pub reverse: bool,
    pub host: bool,
    pub title: &'a str,
    pub status: &'a str,
    pub content: &'a str,
    pub game: &'a str,
    pub platform_line: &'a str,
    pub role_local: &'a str,
    pub role_peer: &'a str,
    pub group_cart: &'a str,
    pub group_save: &'a str,
    pub not_included: &'a str,
    pub partial_note: &'a str,
    pub waiting_line: &'a str,
    pub waiting_after: &'a str,
    pub footer_hint: &'a str,
    pub cart: &'a [DocketAsset<'a>],
    pub save: &'a [DocketAsset<'a>],
}

fn blank(w: u32, h: u32) -> UndoFace {
    UndoFace {
        rgba: vec![0u8; (w * h * 4) as usize],
        w,
        h,
    }
}

fn cart(face: UndoFace) -> crate::CartFace {
    crate::CartFace {
        rgba: face.rgba,
        w: face.w,
        h: face.h,
    }
}

fn accent() -> [u8; 4] {
    let a = sw::accent();
    [a[0], a[1], a[2], 255]
}

fn blit(dst: &mut UndoFace, src: &UndoFace, x: i32, y: i32) {
    for sy in 0..src.h as i32 {
        let dy = y + sy;
        if dy < 0 || dy >= dst.h as i32 {
            continue;
        }
        for sx in 0..src.w as i32 {
            let dx = x + sx;
            if dx < 0 || dx >= dst.w as i32 {
                continue;
            }
            let si = ((sy as u32 * src.w + sx as u32) * 4) as usize;
            let a = src.rgba[si + 3] as u32;
            if a == 0 {
                continue;
            }
            let di = ((dy as u32 * dst.w + dx as u32) * 4) as usize;
            let da = dst.rgba[di + 3] as u32;
            let out = a + da * (255 - a) / 255;
            if out == 0 {
                continue;
            }
            for c in 0..3 {
                dst.rgba[di + c] = ((src.rgba[si + c] as u32 * a
                    + dst.rgba[di + c] as u32 * da * (255 - a) / 255)
                    / out) as u8;
            }
            dst.rgba[di + 3] = out as u8;
        }
    }
}

fn ink_width(value: &str, px: f32) -> f32 {
    crate::keyboard::ink_width(value, px)
}

fn draw_right(face: &mut UndoFace, value: &str, px: f32, right: f32, top: f32, ink: [u8; 3]) {
    let w = ink_width(value, px);
    crate::keyboard::draw_text(face, value, px, (right - w).max(0.0), top, ink);
}

fn dot(face: &mut UndoFace, x: f32, y: f32, r: f32, colour: [u8; 4]) {
    // A radius of exactly half the box trips `f32::clamp`'s min>max check on
    // some values; shaving a hair off keeps the distance field well-formed.
    sw::fill_round(face, x - r, y - r, r * 2.0, r * 2.0, (r - 0.01).max(0.0), colour);
}

/// A soft contact shadow under the paper.  The face is larger than the docket
/// so the app can offset it without clipping.
pub fn shadow_face() -> crate::CartFace {
    let (w, h) = (DOCKET_W + 8, DOCKET_H + 8);
    let mut face = blank(w, h);
    for i in 0..7 {
        let inset = i as f32;
        let alpha = (16 - i * 2).max(0) as u8;
        sw::fill_round(
            &mut face,
            4.0 + inset,
            6.0 + inset,
            DOCKET_W as f32 - inset * 2.0,
            DOCKET_H as f32 - inset * 2.0,
            7.0,
            [0, 0, 0, alpha],
        );
    }
    cart(face)
}

/// The paper, header and every line that does not change with the percentage.
pub fn docket_face(view: &DocketView) -> crate::CartFace {
    let mut face = blank(DOCKET_W, DOCKET_H);
    let paper = sw::rounded_face(
        DOCKET_W,
        DOCKET_H,
        6.0,
        PAPER,
        Some(PAPER_BORDER),
    );
    blit(&mut face, &paper, 0, 0);
    sw::fill_round(&mut face, 2.0, 1.0, (DOCKET_W - 4) as f32, 1.0, 0.0, [245, 241, 226, 255]);
    sw::fill_round(&mut face, 8.0, 6.0, 1.0, (DOCKET_H - 12) as f32, 0.0, [238, 233, 217, 255]);
    sw::fill_round(
        &mut face,
        (DOCKET_W - 9) as f32,
        6.0,
        1.0,
        (DOCKET_H - 12) as f32,
        0.5,
        [238, 233, 217, 255],
    );

    // Header: accent bar, title, content pill, status.
    sw::fill_round(&mut face, 16.0, 16.0, 5.0, 26.0, 2.5, accent());
    crate::keyboard::draw_left(
        &mut face,
        view.title,
        26.0,
        18.0,
        400.0,
        30.0,
        8.0,
        32.0,
        PAPER_INK,
    );
    if !matches!(view.flow, DocketFlow::Done) {
        let pill = sw::rounded_face(90, 22, 6.0, [222, 216, 198, 255], Some(PAPER_BORDER));
        blit(&mut face, &pill, 454, 12);
        crate::keyboard::draw_box(
            &mut face,
            view.content,
            10.0,
            8.0,
            454.0,
            12.0,
            90.0,
            22.0,
            PAPER_INK,
        );
    }
    let status_dot = if matches!(view.flow, DocketFlow::Waiting) {
        PAPER_EDGE
    } else {
        accent()
    };
    dot(&mut face, 18.0, 59.0, 3.5, status_dot);
    crate::keyboard::draw_left(
        &mut face,
        view.status,
        16.0,
        12.0,
        420.0,
        28.0,
        48.0,
        20.0,
        PAPER_INK,
    );

    // Product line: cartridge thumbnail frame, game name, platform / total.
    let frame = sw::rounded_face(
        90,
        51,
        2.0,
        [PAPER_INK[0], PAPER_INK[1], PAPER_INK[2], 255],
        None,
    );
    blit(&mut face, &frame, 16, 74);
    crate::keyboard::draw_left(
        &mut face,
        view.game,
        18.0,
        13.0,
        340.0,
        118.0,
        68.0,
        24.0,
        PAPER_INK,
    );
    crate::keyboard::draw_left(
        &mut face,
        view.platform_line,
        11.0,
        10.0,
        340.0,
        118.0,
        94.0,
        16.0,
        PAPER_DIM,
    );

    // Local/peer identities and a compact arrow, not a full-width divider.
    let left_w = ink_width(view.role_local, 11.0);
    let right_w = ink_width(view.role_peer, 11.0);
    let arrow_w = 34.0;
    let gap = 14.0;
    let total = 6.0 + left_w + gap + arrow_w + gap + 6.0 + right_w;
    let x = (DOCKET_W as f32 - total) / 2.0;
    let cy = 136.0;
    dot(&mut face, x + 3.0, cy, 3.2, if view.host { HOST_DOT } else { JOIN_DOT });
    crate::keyboard::draw_text(
        &mut face,
        view.role_local,
        11.0,
        x + 12.0,
        cy - 7.0,
        PAPER_DIM,
    );
    let ax0 = x + 6.0 + left_w + gap;
    let ax1 = ax0 + arrow_w;
    let (start, end) = if view.reverse { (ax1, ax0) } else { (ax0, ax1) };
    sw::fill_round(
        &mut face,
        start.min(end),
        cy - 1.0,
        (end - start).abs().max(1.0),
        2.0,
        1.0,
        accent(),
    );
    let dir = if end > start { 1.0 } else { -1.0 };
    for i in 0..7 {
        let half = (7 - i) as f32 * 0.62;
        let bx = end - dir * (7 - i) as f32;
        sw::fill_round(
            &mut face,
            bx,
            cy - half,
            1.0,
            half * 2.0,
            0.0,
            accent(),
        );
    }
    let rx = x + 6.0 + left_w + gap + arrow_w + gap;
    dot(&mut face, rx + 3.0, cy, 3.2, if view.host { JOIN_DOT } else { HOST_DOT });
    crate::keyboard::draw_text(
        &mut face,
        view.role_peer,
        11.0,
        rx + 12.0,
        cy - 7.0,
        PAPER_DIM,
    );

    // Waiting state: no 0%.  A dashed slot plus one clear sentence.
    if matches!(view.flow, DocketFlow::Waiting) {
        crate::keyboard::draw_text(&mut face, view.waiting_line, 16.0, 16.0, 150.0, PAPER_INK);
        let mut x = 160.0;
        while x < 544.0 {
            sw::fill_round(&mut face, x, 176.0, 8.0, 6.0, 0.0, PAPER_EDGE);
            x += 14.0;
        }
        crate::keyboard::draw_left(
            &mut face,
            view.waiting_after,
            10.0,
            9.0,
            384.0,
            160.0,
            188.0,
            14.0,
            PAPER_FAINT,
        );
    } else {
        // The track is static; the app paints the red fill on top.
        sw::fill_round(&mut face, 160.0, 164.0, 384.0, 6.0, 3.0, [216, 208, 188, 255]);
        sw::fill_round(&mut face, 160.0, 164.0, 384.0, 1.0, 0.0, [201, 195, 180, 255]);
        sw::fill_round(&mut face, 160.0, 169.0, 384.0, 1.0, 0.0, [201, 195, 180, 255]);
    }

    // Manifest: two semantic columns.  A group that is not part of this
    // transfer still says so, so "仅存档" cannot be mistaken for a cart copy.
    crate::keyboard::draw_text(&mut face, view.group_cart, 10.0, 16.0, 204.0, PAPER_DIM);
    crate::keyboard::draw_text(&mut face, view.group_save, 10.0, 300.0, 204.0, PAPER_DIM);
    sw::fill_round(&mut face, 16.0, 216.0, 216.0, 1.0, 0.0, [201, 195, 180, 255]);
    sw::fill_round(&mut face, 300.0, 216.0, 244.0, 1.0, 0.0, [201, 195, 180, 255]);

    let rows = [
        (view.cart, 16.0, 232.0),
        (view.save, 300.0, 544.0),
    ];
    for (assets, x, right) in rows {
        if assets.is_empty() {
            crate::keyboard::draw_text(
                &mut face,
                view.not_included,
                11.0,
                x + 16.0,
                222.0,
                PAPER_FAINT,
            );
            continue;
        }
        for (index, asset) in assets.iter().take(4).enumerate() {
            let y = 222.0 + index as f32 * 18.0;
            crate::keyboard::draw_left(
                &mut face,
                asset.label,
                12.0,
                10.0,
                right - (x + 16.0) - 56.0,
                x + 16.0,
                y,
                16.0,
                PAPER_INK,
            );
            draw_right(&mut face, asset.size, 10.0, right, y + 2.0, PAPER_FAINT);
        }
    }
    if !view.partial_note.is_empty() && view.cart.len() < 4 {
        let row = if view.cart.is_empty() {
            1
        } else {
            view.cart.len().min(3)
        };
        let y = 222.0 + row as f32 * 18.0;
        crate::keyboard::draw_text(&mut face, view.partial_note, 9.0, 32.0, y, PAPER_FAINT);
    }

    // Footer: perforation, barcode, network hint, maker's mark.
    let mut x = 16.0;
    while x < 544.0 {
        sw::fill_round(&mut face, x, 302.0, 4.0, 1.0, 0.0, PAPER_EDGE);
        x += 8.0;
    }
    let bars = [2u32, 1, 2, 1, 3, 1, 1, 2, 3, 1, 2, 1, 3, 1, 1];
    let mut bx = 16.0;
    for width in bars {
        sw::fill_round(
            &mut face,
            bx,
            304.0,
            width as f32,
            12.0,
            0.0,
            [PAPER_INK[0], PAPER_INK[1], PAPER_INK[2], 255],
        );
        bx += width as f32 + 2.0;
    }
    crate::keyboard::draw_left(
        &mut face,
        view.footer_hint,
        9.0,
        8.0,
        430.0,
        92.0,
        304.0,
        12.0,
        PAPER_FAINT,
    );
    draw_right(&mut face, "LINKSLOT", 7.0, 544.0, 305.0, PAPER_FAINT);

    cart(face)
}

/// A tiny filled check.  The docket's static face leaves the checkbox empty;
/// the app draws this on completed rows.
pub fn check_face() -> crate::CartFace {
    let n = CHECK_PX;
    let mut face = blank(n, n);
    for y in 0..n {
        for x in 0..n {
            let mut red = 0u32;
            let mut white = 0u32;
            for sy in 0..4 {
                for sx in 0..4 {
                    let px = x as f32 + (sx as f32 + 0.5) / 4.0;
                    let py = y as f32 + (sy as f32 + 0.5) / 4.0;
                    let dx = (px - n as f32 / 2.0).abs() - (n as f32 / 2.0 - 2.0);
                    let dy = (py - n as f32 / 2.0).abs() - (n as f32 / 2.0 - 2.0);
                    if dx.max(0.0).hypot(dy.max(0.0)) <= 2.0 {
                        red += 1;
                        let a = (2.5, 5.2);
                        let b = (4.4, 7.2);
                        let c = (8.0, 3.2);
                        let near = |x0: f32, y0: f32, x1: f32, y1: f32| {
                            let (vx, vy) = (x1 - x0, y1 - y0);
                            let t = (((px - x0) * vx + (py - y0) * vy)
                                / (vx * vx + vy * vy).max(f32::EPSILON))
                                .clamp(0.0, 1.0);
                            ((px - (x0 + vx * t)).powi(2) + (py - (y0 + vy * t)).powi(2))
                                .sqrt()
                                <= 1.25
                        };
                        if near(a.0, a.1, b.0, b.1) || near(b.0, b.1, c.0, c.1) {
                            white += 1;
                        }
                    }
                }
            }
            if red == 0 {
                continue;
            }
            let at = ((y * n + x) * 4) as usize;
            let w = white as u32;
            for ch in 0..3 {
                let under = accent()[ch] as u32 * (red - w) / red;
                let over = 255u32 * w / red;
                face.rgba[at + ch] = (under + over).min(255) as u8;
            }
            face.rgba[at + 3] = (red * 255 / 16) as u8;
        }
    }
    cart(face)
}

/// The completion stamp: two rings and a check, baked at an angle because
/// `Draw::Tex` is axis-aligned.
pub fn stamp_face() -> crate::CartFace {
    let n = 40u32;
    let mut face = blank(n, n);
    let c = n as f32 / 2.0;
    let angle = 8.0_f32.to_radians();
    let (sin, cos) = angle.sin_cos();
    let ink = accent();
    for y in 0..n {
        for x in 0..n {
            let mut hits = 0u32;
            for sy in 0..4 {
                for sx in 0..4 {
                    let px = x as f32 + (sx as f32 + 0.5) / 4.0;
                    let py = y as f32 + (sy as f32 + 0.5) / 4.0;
                    let dx = px - c;
                    let dy = py - c;
                    let rx = dx * cos - dy * sin;
                    let ry = dx * sin + dy * cos;
                    let r = (rx * rx + ry * ry).sqrt();
                    let ring = (r - 17.5).abs() <= 1.3;
                    let inner = (r - 13.5).abs() <= 0.5;
                    let check = segment_dist(rx, ry, -7.0, 1.0, -2.0, 7.0) <= 1.5
                        || segment_dist(rx, ry, -2.0, 7.0, 8.0, -7.0) <= 1.5;
                    if ring || inner || check {
                        hits += 1;
                    }
                }
            }
            if hits == 0 {
                continue;
            }
            let at = ((y * n + x) * 4) as usize;
            face.rgba[at..at + 3].copy_from_slice(&ink[..3]);
            face.rgba[at + 3] = (hits * 255 / 16) as u8;
        }
    }
    cart(face)
}

fn segment_dist(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let (dx, dy) = (bx - ax, by - ay);
    let len2 = dx * dx + dy * dy;
    if len2 <= f32::EPSILON {
        return ((px - ax).powi(2) + (py - ay).powi(2)).sqrt();
    }
    let t = (((px - ax) * dx + (py - ay) * dy) / len2).clamp(0.0, 1.0);
    ((px - (ax + dx * t)).powi(2) + (py - (ay + dy * t)).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alpha(face: &crate::CartFace, x: u32, y: u32) -> u8 {
        face.rgba[((y * face.w + x) * 4 + 3) as usize]
    }

    fn asset<'a>(label: &'a str, size: &'a str) -> DocketAsset<'a> {
        DocketAsset { label, size }
    }

    fn view<'a>(cart: &'a [DocketAsset<'a>], save: &'a [DocketAsset<'a>]) -> DocketView<'a> {
        DocketView {
            flow: DocketFlow::Sending,
            reverse: false,
            host: true,
            title: "卡带传输",
            status: "正在发送到对方",
            content: "卡带＋存档",
            game: "马里奥赛车",
            platform_line: "GBA · 8.9 MB",
            role_local: "本机 · 主机",
            role_peer: "对方 · 副机",
            group_cart: "卡带",
            group_save: "存档",
            not_included: "不传输",
            partial_note: "",
            waiting_line: "等待对方确认…",
            waiting_after: "对方接受后开始传输",
            footer_hint: "同一 Wi-Fi · 传输中请勿退出",
            cart,
            save,
        }
    }

    #[test]
    fn the_docket_is_paper_sized() {
        let face = docket_face(&view(&[], &[]));
        assert_eq!((face.w, face.h), (DOCKET_W, DOCKET_H));
        assert_eq!(alpha(&face, 0, 0), 0);
        let at = ((160 * face.w + 280) * 4) as usize;
        assert_eq!(&face.rgba[at..at + 3], &PAPER[..3]);
    }

    #[test]
    fn the_check_has_red_and_white_ink() {
        let face = check_face();
        assert_eq!((face.w, face.h), (CHECK_PX, CHECK_PX));
        let inked = (0..face.w * face.h)
            .filter(|i| face.rgba[(*i as usize) * 4 + 3] > 0)
            .count();
        assert!(inked > 20, "the check has too little ink: {inked}");
    }

    #[test]
    fn the_stamp_is_a_ring_with_a_check() {
        let face = stamp_face();
        assert_eq!((face.w, face.h), (40, 40));
        assert_eq!(alpha(&face, 0, 0), 0);
        let inked = (0..face.w * face.h)
            .filter(|i| face.rgba[(*i as usize) * 4 + 3] > 0)
            .count();
        assert!(inked > 100, "the stamp has too little ink: {inked}");
    }

    #[test]
    fn seven_assets_do_not_overflow_the_manifest() {
        let cart = [
            asset("游戏本体", "4.0 MB"),
            asset("封面", "128 KB"),
            asset("介绍", "2 KB"),
            asset("视频", "4.2 MB"),
        ];
        let save = [
            asset("电池存档", "32 KB"),
            asset("即时存档", "512 KB"),
            asset("缩略图", "24 KB"),
        ];
        let face = docket_face(&view(&cart, &save));
        assert_eq!((face.w, face.h), (DOCKET_W, DOCKET_H));
    }

    #[test]
    fn the_english_docket_keeps_the_same_geometry() {
        let cart = [asset("Game", "4.0 MB"), asset("Cover", "128 KB")];
        let save = [asset("Battery save", "32 KB")];
        let face = docket_face(&DocketView {
            flow: DocketFlow::Sending,
            reverse: false,
            host: true,
            title: "Cartridge Transfer",
            status: "Sending to the other handheld",
            content: "Extra content",
            game: "Golden Sun: The Lost Age",
            platform_line: "GBA · 4.1 MB",
            role_local: "This device · Host",
            role_peer: "Other device · Joiner",
            group_cart: "Cartridge",
            group_save: "Saves",
            not_included: "Not transferred",
            partial_note: "Cartridge already on this device",
            waiting_line: "Waiting for confirmation…",
            waiting_after: "Starts after the other side accepts",
            footer_hint: "Same Wi-Fi · Do not exit during transfer",
            cart: &cart,
            save: &save,
        });
        assert_eq!((face.w, face.h), (DOCKET_W, DOCKET_H));
    }
}
