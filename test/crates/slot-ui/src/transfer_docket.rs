//! The cart-transfer card: a Console Flat "transfer link" manifest shown while
//! the cartridge stays in the slot.  The card itself is static; the percentage,
//! bar fill, per-item states, link arrow and data dot are drawn by the app.

use crate::plate::UndoFace;
use crate::switch_style as sw;

pub const DOCKET_W: u32 = 560;
pub const DOCKET_H: u32 = 344;

/// Where the card sits on the 720x480 screen.
pub const DOCKET_AT: (f32, f32) = (80.0, 70.0);
/// The game thumbnail frame (absolute screen coordinates) and the paste area.
pub const CARD_FRAME: (f32, f32, f32, f32) = (104.0, 168.0, 64.0, 38.0);
pub const CARD_INNER: (f32, f32, f32, f32) = (105.0, 169.0, 62.0, 36.0);
/// Dynamic text anchors (absolute screen coordinates).
pub const PERCENT_AT: (f32, f32) = (104.0, 248.0);
pub const BYTES_AT: (f32, f32) = (196.0, 270.0);
pub const BAR: (f32, f32, f32, f32) = (196.0, 260.0, 420.0, 4.0);
/// The link line's endpoints and the arrow centre.
pub const LINK_X0: f32 = 142.0;
pub const LINK_X1: f32 = 578.0;
pub const LINK_Y: f32 = 223.0;
pub const ARROW_CENTRE: (f32, f32) = (360.0, 223.0);
/// Manifest rows, absolute Y.
pub const ROW_Y: [f32; 4] = [312.0, 328.0, 344.0, 360.0];
pub const COL_L: f32 = 104.0;
pub const COL_R: f32 = 380.0;
pub const COL_L_RIGHT: f32 = 340.0;
pub const COL_R_RIGHT: f32 = 616.0;
pub const CHECK_PX: u32 = 10;
pub const CHECK_DY: f32 = 0.0;
/// The per-item mini bar runs from `x + 16` to `right - MINI_BAR_RIGHT`.
pub const MINI_BAR_DY: f32 = 14.0;
pub const MINI_BAR_RIGHT: f32 = 10.0;
/// The completion badge's centre, absolute.
pub const BADGE_CENTRE: (f32, f32) = (596.0, 140.0);
pub const BADGE_PX: u32 = 24;
pub const ARROW_PX: u32 = 8;

pub const PAPER: [u8; 4] = [0x26, 0x26, 0x2b, 255];
pub const PAPER_BORDER: [u8; 4] = [0x3a, 0x3a, 0x42, 255];
pub const PAPER_EDGE: [u8; 4] = [0x3a, 0x3a, 0x42, 255];
pub const PAPER_INK: [u8; 3] = [0xf4, 0xf4, 0xf1];
pub const PAPER_DIM: [u8; 3] = [0xa6, 0xa6, 0xad];
pub const PAPER_FAINT: [u8; 3] = [0x74, 0x74, 0x7c];
pub const HOST_DOT: [u8; 4] = [0x66, 0xe2, 0xba, 255];
pub const JOIN_DOT: [u8; 4] = [0x8a, 0xc8, 0xff, 255];

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

/// Everything the static card prints.  `reverse` is used by the app for the
/// link arrow / data dot; the card only needs the roles and the flow.
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
            if src.rgba[si + 3] == 0 {
                continue;
            }
            let di = ((dy as u32 * dst.w + dx as u32) * 4) as usize;
            dst.rgba[di..di + 4].copy_from_slice(&src.rgba[si..si + 4]);
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

fn dashed(face: &mut UndoFace, x0: f32, y: f32, width: f32, colour: [u8; 4]) {
    let mut x = x0;
    while x < x0 + width {
        let w = 7.0f32.min(x0 + width - x);
        sw::fill_round(face, x, y, w, 1.0, 0.0, colour);
        x += 12.0;
    }
}

fn point_segment(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let (dx, dy) = (bx - ax, by - ay);
    let len = dx * dx + dy * dy;
    if len <= f32::EPSILON {
        return ((px - ax) * (px - ax) + (py - ay) * (py - ay)).sqrt();
    }
    let t = (((px - ax) * dx + (py - ay) * dy) / len).clamp(0.0, 1.0);
    let qx = ax + dx * t;
    let qy = ay + dy * t;
    ((px - qx) * (px - qx) + (py - qy) * (py - qy)).sqrt()
}

fn stroke(face: &mut UndoFace, points: &[(f32, f32)], width: f32, colour: [u8; 3]) {
    for y in 0..face.h {
        for x in 0..face.w {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let mut distance = f32::INFINITY;
            for pair in points.windows(2) {
                distance = distance.min(point_segment(
                    px, py, pair[0].0, pair[0].1, pair[1].0, pair[1].1,
                ));
            }
            let alpha = ((width / 2.0 + 0.5 - distance).clamp(0.0, 1.0) * 255.0) as u8;
            if alpha > 0 {
                let at = ((y * face.w + x) * 4) as usize;
                let src = alpha as f32 / 255.0;
                let dst = face.rgba[at + 3] as f32 / 255.0;
                let out = src + dst * (1.0 - src);
                if out <= 0.0 {
                    continue;
                }
                for channel in 0..3 {
                    let a = colour[channel] as f32 / 255.0;
                    let b = face.rgba[at + channel] as f32 / 255.0;
                    face.rgba[at + channel] =
                        (((a * src + b * dst * (1.0 - src)) / out) * 255.0).round() as u8;
                }
                face.rgba[at + 3] = (out * 255.0).round() as u8;
            }
        }
    }
}

fn draw_device(face: &mut UndoFace, x: f32, host: bool) {
    let icon = sw::rounded_face(
        30,
        18,
        4.0,
        [0x1a, 0x1a, 0x1e, 255],
        Some([PAPER_INK[0], PAPER_INK[1], PAPER_INK[2], 255]),
    );
    blit(face, &icon, x as i32, 144);
    sw::fill_round(
        face,
        x + 5.0,
        150.0,
        15.0,
        1.0,
        0.0,
        [PAPER_INK[0], PAPER_INK[1], PAPER_INK[2], 255],
    );
    sw::fill_round(
        face,
        x + 5.0,
        154.0,
        11.0,
        1.0,
        0.0,
        [PAPER_DIM[0], PAPER_DIM[1], PAPER_DIM[2], 255],
    );
    dot(
        face,
        x + 25.0,
        147.0,
        2.5,
        if host { HOST_DOT } else { JOIN_DOT },
    );
}

/// The card, header and every line that does not change with the percentage.
pub fn docket_face(view: &DocketView) -> crate::CartFace {
    let mut face = blank(DOCKET_W, DOCKET_H);
    let card = sw::rounded_face(DOCKET_W, DOCKET_H, 12.0, PAPER, Some(PAPER_BORDER));
    blit(&mut face, &card, 0, 0);

    // Header: red spine, title, content chip, hairline.
    let accent = sw::accent();
    let accent4 = [accent[0], accent[1], accent[2], 255];
    sw::fill_round(&mut face, 24.0, 16.0, 3.0, 22.0, 1.5, accent4);
    crate::keyboard::draw_left(
        &mut face,
        view.title,
        20.0,
        10.0,
        400.0,
        36.0,
        12.0,
        26.0,
        PAPER_INK,
    );
    let chip = sw::rounded_face(80, 20, 10.0, sw::surface(), Some(PAPER_BORDER));
    blit(&mut face, &chip, 456, 16);
    crate::keyboard::draw_box(
        &mut face,
        view.content,
        9.0,
        8.0,
        456.0,
        16.0,
        80.0,
        20.0,
        PAPER_DIM,
    );
    sw::fill_round(&mut face, 24.0, 48.0, 512.0, 1.0, 0.0, PAPER_EDGE);

    // Status, role, and the waiting sub-line.
    dot(&mut face, 27.0, 70.0, 3.0, accent4);
    crate::keyboard::draw_left(
        &mut face,
        view.status,
        14.0,
        10.0,
        420.0,
        38.0,
        62.0,
        20.0,
        PAPER_INK,
    );
    if matches!(view.flow, DocketFlow::Waiting) {
        crate::keyboard::draw_left(
            &mut face,
            view.waiting_line,
            10.0,
            9.0,
            300.0,
            40.0,
            80.0,
            14.0,
            PAPER_DIM,
        );
    }

    // Game thumbnail frame (the app pastes the cart face inside) and name.
    let frame = sw::rounded_face(64, 38, 4.0, [0x1d, 0x1d, 0x22, 255], Some(PAPER_EDGE));
    blit(&mut face, &frame, 24, 98);
    crate::keyboard::draw_left(
        &mut face,
        view.game,
        18.0,
        10.0,
        420.0,
        100.0,
        96.0,
        24.0,
        PAPER_INK,
    );
    crate::keyboard::draw_left(
        &mut face,
        view.platform_line,
        10.5,
        9.0,
        420.0,
        100.0,
        120.0,
        16.0,
        PAPER_DIM,
    );

    // Two devices, one link, and the role dots.  The app paints the direction
    // arrow and the moving data dot on this line.
    draw_device(&mut face, 24.0, view.host);
    draw_device(&mut face, 506.0, !view.host);
    crate::keyboard::draw_text(&mut face, view.role_local, 9.5, 24.0, 166.0, PAPER_DIM);
    draw_right(&mut face, view.role_peer, 9.5, 536.0, 166.0, PAPER_DIM);
    if matches!(view.flow, DocketFlow::Waiting) {
        dashed(&mut face, 62.0, 153.0, 436.0, PAPER_EDGE);
    } else {
        sw::fill_round(&mut face, 62.0, 153.0, 436.0, 1.0, 0.0, PAPER_EDGE);
    }

    // Progress track; the app paints the red fill on top.
    if matches!(view.flow, DocketFlow::Waiting) {
        dashed(&mut face, 116.0, 191.5, 420.0, PAPER_EDGE);
    } else {
        sw::fill_round(&mut face, 116.0, 190.0, 420.0, 4.0, 2.0, PAPER_EDGE);
    }

    // Manifest: two explicit, equal-width groups.
    sw::fill_round(&mut face, 24.0, 234.0, 512.0, 1.0, 0.0, PAPER_EDGE);
    crate::keyboard::draw_text(&mut face, view.group_cart, 9.5, 24.0, 222.0, PAPER_FAINT);
    crate::keyboard::draw_text(&mut face, view.group_save, 9.5, 300.0, 222.0, PAPER_FAINT);

    let cart_note = if view.cart.is_empty() {
        if view.partial_note.is_empty() {
            Some(view.not_included)
        } else {
            Some(view.partial_note)
        }
    } else {
        None
    };
    let save_note = if view.save.is_empty() {
        Some(view.not_included)
    } else {
        None
    };
    let groups = [
        (view.cart, COL_L - DOCKET_AT.0, COL_L_RIGHT - DOCKET_AT.0, cart_note),
        (view.save, COL_R - DOCKET_AT.0, COL_R_RIGHT - DOCKET_AT.0, save_note),
    ];
    for (assets, x, right, note) in groups {
        if let Some(note) = note {
            crate::keyboard::draw_text(&mut face, note, 10.0, x + 16.0, 242.0, PAPER_FAINT);
            continue;
        }
        for (index, asset) in assets.iter().take(4).enumerate() {
            let y = 242.0 + index as f32 * 16.0;
            crate::keyboard::draw_left(
                &mut face,
                asset.label,
                11.0,
                9.0,
                right - (x + 16.0) - 52.0,
                x + 16.0,
                y - 1.0,
                16.0,
                PAPER_DIM,
            );
            draw_right(&mut face, asset.size, 9.5, right, y + 1.0, PAPER_FAINT);
        }
    }

    // Footer: the network/hint line and a small brand mark.
    crate::keyboard::draw_left(
        &mut face,
        view.footer_hint,
        9.0,
        8.0,
        430.0,
        24.0,
        320.0,
        12.0,
        PAPER_FAINT,
    );
    draw_right(&mut face, "LINKSLOT", 8.0, 536.0, 320.0, PAPER_FAINT);

    cart(face)
}

/// The white check on a completed manifest row.
pub fn check_face() -> crate::CartFace {
    let n = CHECK_PX;
    let mut face = blank(n, n);
    stroke(
        &mut face,
        &[(2.5, 5.2), (4.4, 7.2), (8.0, 3.2)],
        1.25,
        PAPER_INK,
    );
    cart(face)
}

/// The red completion badge: a filled red circle with a white check.
pub fn badge_face() -> crate::CartFace {
    let n = BADGE_PX;
    let mut face = blank(n, n);
    let accent = sw::accent();
    sw::fill_round(
        &mut face,
        0.0,
        0.0,
        n as f32,
        n as f32,
        n as f32 / 2.0,
        [accent[0], accent[1], accent[2], 255],
    );
    stroke(&mut face, &[(6.5, 12.5), (10.5, 17.0), (18.0, 7.5)], 1.6, PAPER_INK);
    cart(face)
}

/// The direction arrow on the link.  `reverse` points left.
pub fn arrow_face(reverse: bool) -> crate::CartFace {
    let n = ARROW_PX;
    let mut face = blank(n, n);
    let accent = sw::accent();
    let points = if reverse {
        [(6.0, 1.5), (2.5, 4.0), (6.0, 6.5)]
    } else {
        [(2.0, 1.5), (5.5, 4.0), (2.0, 6.5)]
    };
    stroke(&mut face, &points, 1.5, [accent[0], accent[1], accent[2]]);
    cart(face)
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
    fn the_docket_is_the_flat_transfer_card() {
        let face = docket_face(&view(&[], &[]));
        assert_eq!((face.w, face.h), (DOCKET_W, DOCKET_H));
        assert_eq!(alpha(&face, 0, 0), 0);
        let at = ((160 * face.w + 280) * 4) as usize;
        assert_eq!(&face.rgba[at..at + 3], &PAPER[..3]);
    }

    #[test]
    fn the_check_and_badge_have_ink() {
        let check = check_face();
        assert_eq!((check.w, check.h), (CHECK_PX, CHECK_PX));
        let inked = (0..check.w * check.h)
            .filter(|i| check.rgba[(*i as usize) * 4 + 3] > 0)
            .count();
        assert!(inked > 20, "the check has too little ink: {inked}");
        let badge = badge_face();
        assert_eq!((badge.w, badge.h), (BADGE_PX, BADGE_PX));
        assert!(badge.rgba.iter().any(|byte| *byte > 0));
        let right = arrow_face(false);
        let left = arrow_face(true);
        assert_ne!(right.rgba, left.rgba);
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
