//! The cloud-sync "sync bench" artwork: an opaque sheet, a docked cartridge,
//! a cream sticker card for the remote side, a short transfer lane and the
//! five-stage rail.  The app moves these finished faces around; nothing here
//! is rasterised per frame.

use crate::plate::UndoFace;
use crate::switch_style as sw;
use crate::sync_overlay::{
    cloud_face, StageDot, Travel, CHIP_H, CHIP_W, CLOUD_CARD_H, CLOUD_CARD_W, DIRECTION_PX, DOCK_H,
    DOCK_W, DOT_ACTIVE_PX, DOT_DONE_PX, DOT_PENDING_PX, LANE_EDGE, LANE_FILL, LANE_H, LANE_W,
    PACKET_H, PACKET_W, PAPER, PAPER_BORDER, SHEET_BORDER, SHEET_FILL, SHEET_H, SHEET_MASK,
    SHEET_W, STAMP_PX, STICKER_DIM, STICKER_INK,
};
use crate::text;
use crate::CartFace;

fn blank(w: u32, h: u32) -> UndoFace {
    UndoFace {
        rgba: vec![0u8; (w * h * 4) as usize],
        w,
        h,
    }
}

fn cart(face: UndoFace) -> CartFace {
    CartFace {
        rgba: face.rgba,
        w: face.w,
        h: face.h,
    }
}

fn accent() -> [u8; 4] {
    let a = sw::accent();
    [a[0], a[1], a[2], 255]
}

fn paper_ink() -> [u8; 4] {
    [STICKER_INK[0], STICKER_INK[1], STICKER_INK[2], 255]
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

fn small_text(value: &str, max_w: f32, px: f32, ink: [u8; 3]) -> UndoFace {
    let Some(font) = text::label_font() else {
        return blank(1, 1);
    };
    let layout = text::fit(font, value, max_w, 1, px, 5.0);
    let w = layout
        .lines
        .iter()
        .map(|line| text::line_width(font, line, layout.px, layout.tracking))
        .fold(0.0, f32::max)
        .ceil()
        .max(1.0) as u32;
    let h = font
        .horizontal_line_metrics(layout.px)
        .map_or((layout.px * 1.4).ceil(), |m| m.new_line_size.ceil())
        .max(1.0) as u32;
    let mut face = blank(w, h);
    text::draw_centred(&mut face.rgba, w, h, &layout, ink);
    face
}

fn dist_to_segment(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let (dx, dy) = (bx - ax, by - ay);
    let len2 = dx * dx + dy * dy;
    if len2 <= f32::EPSILON {
        return ((px - ax).powi(2) + (py - ay).powi(2)).sqrt();
    }
    let t = (((px - ax) * dx + (py - ay) * dy) / len2).clamp(0.0, 1.0);
    ((px - (ax + dx * t)).powi(2) + (py - (ay + dy * t)).powi(2)).sqrt()
}

/// Rounded-rectangle coverage for the translucent sheet.  `switch_style`'s
/// `rounded_face` paints an opaque border underneath the fill, which would
/// make the whole panel opaque; this composites a 1 px border and a
/// translucent fill in one pass instead.
fn rounded_coverage(px: f32, py: f32, x: f32, y: f32, w: f32, h: f32, r: f32) -> f32 {
    if w <= 0.0 || h <= 0.0 {
        return 0.0;
    }
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    if px < x || py < y || px >= x + w || py >= y + h {
        return 0.0;
    }
    let cx = px.clamp(x + r, x + w - r);
    let cy = py.clamp(y + r, y + h - r);
    let dx = px - cx;
    let dy = py - cy;
    (r - (dx * dx + dy * dy).sqrt() + 0.5).clamp(0.0, 1.0)
}

/// The charcoal sheet that holds the whole stage.  The red title bar and the
/// header rule are baked in; only the title itself is a separate text face.
pub fn stage_sheet_face() -> CartFace {
    let (w, h) = (SHEET_W, SHEET_H);
    let mut face = blank(w, h);
    let fill = SHEET_FILL;
    let border = SHEET_BORDER;
    let fill_a = fill[3] as f32 / 255.0;
    for py in 0..h {
        for px in 0..w {
            let outer = rounded_coverage(
                px as f32 + 0.5,
                py as f32 + 0.5,
                0.0,
                0.0,
                w as f32,
                h as f32,
                18.0,
            );
            if outer <= 0.0 {
                continue;
            }
            let inner = rounded_coverage(
                px as f32 + 0.5,
                py as f32 + 0.5,
                1.0,
                1.0,
                w as f32 - 2.0,
                h as f32 - 2.0,
                17.0,
            );
            let border_a = (outer - inner).clamp(0.0, 1.0);
            let fill_cover = inner * fill_a;
            let a = fill_cover + border_a * (1.0 - fill_cover);
            if a <= 0.0 {
                continue;
            }
            let i = ((py * w + px) * 4) as usize;
            for c in 0..3 {
                face.rgba[i + c] = ((fill[c] as f32 * fill_cover
                    + border[c] as f32 * border_a * (1.0 - fill_cover))
                    / a)
                    .round()
                    .clamp(0.0, 255.0) as u8;
            }
            face.rgba[i + 3] = (a * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    }
    sw::fill_round(&mut face, 20.0, 1.5, 616.0, 1.0, 0.5, [46, 46, 53, 255]);
    sw::fill_round(&mut face, 24.0, 22.0, 5.0, 30.0, 2.5, accent());
    sw::fill_round(&mut face, 24.0, 76.0, 608.0, 1.0, 0.5, [40, 40, 47, 255]);
    cart(face)
}

/// The little slot the game card sits in.  It is drawn after the card so the
/// bottom few pixels are swallowed by its lip, like the real cartridge slot.
pub fn dock_face() -> CartFace {
    let mut face = sw::rounded_face(
        DOCK_W,
        DOCK_H,
        4.0,
        [20, 20, 24, 255],
        Some([74, 74, 84, 255]),
    );
    sw::fill_round(&mut face, 2.0, 1.0, 228.0, 1.0, 0.5, [46, 46, 53, 255]);
    sw::fill_round(&mut face, 14.0, 7.0, 204.0, 3.0, 1.5, [11, 11, 15, 255]);
    sw::fill_round(&mut face, 128.0, 3.0, 3.0, 4.0, 1.0, [58, 58, 68, 255]);
    sw::fill_round(&mut face, 224.0, 3.0, 3.0, 4.0, 1.0, [58, 58, 68, 255]);
    cart(face)
}

/// The cream sticker card that stands for the cloud.  The word "云存档" is a
/// separate dark text face, because it is localised.
pub fn cloud_card_face() -> CartFace {
    let mut face = sw::rounded_face(
        CLOUD_CARD_W,
        CLOUD_CARD_H,
        12.0,
        sw::surface(),
        Some(sw::surface_border()),
    );
    sw::fill_round(&mut face, 14.0, 31.0, 148.0, 52.0, 3.0, [0, 0, 0, 80]);
    let paper = sw::rounded_face(148, 52, 3.0, PAPER, Some(PAPER_BORDER));
    blit(&mut face, &paper, 12, 29);
    sw::fill_round(&mut face, 16.0, 36.0, 3.0, 38.0, 1.5, accent());
    let tile = sw::rounded_face(36, 36, 6.0, [25, 26, 31, 255], Some(sw::surface_border()));
    blit(&mut face, &tile, 21, 37);
    let cloud = cloud_face(17.0);
    blit(
        &mut face,
        &UndoFace {
            rgba: cloud.rgba,
            w: cloud.w,
            h: cloud.h,
        },
        26,
        46,
    );
    let tag = small_text("CLOUD", 40.0, 7.0, STICKER_DIM);
    blit(&mut face, &tag, 67, 57);
    cart(face)
}

/// The neutral right-hand card for a manual mod that does not sync to a cloud.
pub fn target_card_face() -> CartFace {
    let mut face = sw::rounded_face(
        CLOUD_CARD_W,
        CLOUD_CARD_H,
        12.0,
        sw::surface(),
        Some(sw::surface_border()),
    );
    let tile = sw::rounded_face(56, 56, 10.0, [20, 20, 24, 255], Some([58, 58, 68, 255]));
    blit(&mut face, &tile, 58, 14);
    for i in 0..3 {
        sw::fill_round(
            &mut face,
            72.0,
            26.0 + i as f32 * 11.0,
            28.0,
            5.0,
            2.5,
            [95, 95, 106, 255],
        );
    }
    sw::fill_round(&mut face, 106.0, 61.0, 5.0, 5.0, 2.5, accent());
    let tag = small_text("MOD", 30.0, 7.0, [110, 110, 122]);
    let x = (CLOUD_CARD_W as i32 - tag.w as i32) / 2;
    blit(&mut face, &tag, x, 96);
    cart(face)
}

/// A soft contact shadow for the docked card.
pub fn shadow_face() -> CartFace {
    let (w, h) = (210u32, 22u32);
    let mut face = blank(w, h);
    for y in 0..h {
        for x in 0..w {
            let dx = (x as f32 + 0.5 - w as f32 / 2.0) / (w as f32 / 2.0);
            let dy = (y as f32 + 0.5 - h as f32 / 2.0) / (h as f32 / 2.0);
            let d = (dx * dx + dy * dy).max(0.0);
            let a = ((1.0 - d).max(0.0).powf(1.4) * 95.0) as u8;
            let i = ((y * w + x) * 4) as usize;
            face.rgba[i + 3] = a;
        }
    }
    cart(face)
}

/// The recessed transfer lane.
pub fn lane_face() -> CartFace {
    let mut face = blank(LANE_W, LANE_H);
    let slot = sw::rounded_face(LANE_W, 6, 3.0, LANE_FILL, Some(LANE_EDGE));
    blit(&mut face, &slot, 0, 2);
    sw::fill_round(&mut face, 4.0, 3.0, 134.0, 1.0, 0.5, [46, 46, 53, 255]);
    sw::fill_round(&mut face, 0.0, 1.0, 1.5, 8.0, 0.75, [74, 74, 84, 255]);
    sw::fill_round(&mut face, 140.5, 1.0, 1.5, 8.0, 0.75, [74, 74, 84, 255]);
    cart(face)
}

/// A cream paper slip carried along the lane.
pub fn packet_face() -> CartFace {
    let mut face = sw::rounded_face(PACKET_W, PACKET_H, 2.0, PAPER, Some(PAPER_BORDER));
    sw::fill_round(&mut face, 2.0, 4.0, 5.0, 1.0, 0.5, paper_ink());
    sw::fill_round(&mut face, 2.0, 8.0, 4.0, 1.0, 0.5, paper_ink());
    cart(face)
}

/// The approval stamp.  It is baked at an angle because `Draw::Tex` is
/// axis-aligned; the app only changes its size and alpha while it settles.
pub fn stamp_face() -> CartFace {
    let n = STAMP_PX;
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
                    let ring = (r - 23.0).abs() <= 1.5;
                    let inner = (r - 18.0).abs() <= 0.5;
                    let check = dist_to_segment(rx, ry, -8.0, 1.0, -2.0, 8.0) <= 1.7
                        || dist_to_segment(rx, ry, -2.0, 8.0, 10.0, -8.0) <= 1.7;
                    if ring || inner || check {
                        hits += 1;
                    }
                }
            }
            if hits == 0 {
                continue;
            }
            let i = ((y * n + x) * 4) as usize;
            face.rgba[i..i + 3].copy_from_slice(&ink[..3]);
            face.rgba[i + 3] = (hits * 255 / 16) as u8;
        }
    }
    cart(face)
}

/// A stage-rail dot.  `Active` carries the sheet-coloured ring so it cuts the
/// white completed line behind it.
pub fn stage_dot_face(kind: StageDot) -> CartFace {
    match kind {
        StageDot::Done => cart(sw::rounded_face(
            DOT_DONE_PX,
            DOT_DONE_PX,
            DOT_DONE_PX as f32 / 2.0,
            [255, 255, 255, 255],
            None,
        )),
        StageDot::Active => {
            let mut face = blank(DOT_ACTIVE_PX, DOT_ACTIVE_PX);
            sw::fill_round(
                &mut face,
                0.0,
                0.0,
                DOT_ACTIVE_PX as f32,
                DOT_ACTIVE_PX as f32,
                DOT_ACTIVE_PX as f32 / 2.0,
                SHEET_MASK,
            );
            sw::fill_round(&mut face, 3.0, 3.0, 13.0, 13.0, 6.5, accent());
            cart(face)
        }
        StageDot::Pending => cart(sw::rounded_face(
            DOT_PENDING_PX,
            DOT_PENDING_PX,
            DOT_PENDING_PX as f32 / 2.0,
            [0, 0, 0, 0],
            Some([90, 90, 100, 255]),
        )),
    }
}

/// The red disc with the white direction chevron.
pub fn direction_face(travel: Travel) -> CartFace {
    let n = DIRECTION_PX;
    let mut face = blank(n, n);
    let c = n as f32 / 2.0;
    let r = c - 0.5;
    let ink = accent();
    let sign = if matches!(travel, Travel::Right) {
        1.0
    } else {
        -1.0
    };
    for y in 0..n {
        for x in 0..n {
            let mut circle = 0u32;
            let mut chevron = 0u32;
            for sy in 0..4 {
                for sx in 0..4 {
                    let px = x as f32 + (sx as f32 + 0.5) / 4.0;
                    let py = y as f32 + (sy as f32 + 0.5) / 4.0;
                    let dx = px - c;
                    let dy = py - c;
                    if dx * dx + dy * dy <= r * r {
                        circle += 1;
                        let (ax, ay) = (-3.5 * sign, -5.0);
                        let (bx, by) = (2.5 * sign, 0.0);
                        let (cx, cy) = (-3.5 * sign, 5.0);
                        if dist_to_segment(dx, dy, ax, ay, bx, by) <= 1.1
                            || dist_to_segment(dx, dy, bx, by, cx, cy) <= 1.1
                        {
                            chevron += 1;
                        }
                    }
                }
            }
            if circle == 0 {
                continue;
            }
            let i = ((y * n + x) * 4) as usize;
            let w = chevron as u32;
            for ch in 0..3 {
                let under = ink[ch] as u32 * (circle - w) / circle;
                let over = 255u32 * w / circle;
                face.rgba[i + ch] = ((under + over) as u32).min(255) as u8;
            }
            face.rgba[i + 3] = (circle * 255 / 16) as u8;
        }
    }
    cart(face)
}

/// The hollow ring shown while the direction is not known yet.
pub fn wait_face() -> CartFace {
    let n = DIRECTION_PX;
    let mut face = blank(n, n);
    let c = n as f32 / 2.0;
    for y in 0..n {
        for x in 0..n {
            let mut hits = 0u32;
            for sy in 0..4 {
                for sx in 0..4 {
                    let px = x as f32 + (sx as f32 + 0.5) / 4.0;
                    let py = y as f32 + (sy as f32 + 0.5) / 4.0;
                    let r = ((px - c).powi(2) + (py - c).powi(2)).sqrt();
                    if (r - 13.0).abs() <= 1.5 || r <= 2.5 {
                        hits += 1;
                    }
                }
            }
            if hits == 0 {
                continue;
            }
            let i = ((y * n + x) * 4) as usize;
            face.rgba[i..i + 3].copy_from_slice(&[90, 90, 100]);
            face.rgba[i + 3] = (hits * 255 / 16) as u8;
        }
    }
    cart(face)
}

/// The small surface pill behind a manual mod's real percentage.
pub fn percent_chip_face() -> CartFace {
    cart(sw::rounded_face(
        CHIP_W,
        CHIP_H,
        8.0,
        sw::surface(),
        Some(sw::surface_border()),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alpha(face: &CartFace, x: u32, y: u32) -> u8 {
        face.rgba[((y * face.w + x) * 4 + 3) as usize]
    }

    #[test]
    fn the_sheet_has_rounded_corners_and_a_translucent_face() {
        let face = stage_sheet_face();
        assert_eq!((face.w, face.h), (SHEET_W, SHEET_H));
        assert_eq!(alpha(&face, 0, 0), 0);
        let centre = alpha(&face, SHEET_W / 2, SHEET_H / 2);
        assert!(
            (120..=170).contains(&centre),
            "the sheet should let the shelf ghost through: alpha {centre}"
        );
    }

    #[test]
    fn the_sticker_card_carries_the_cream_paper() {
        let face = cloud_card_face();
        let at = ((39 * face.w + 80) * 4) as usize;
        assert_eq!(&face.rgba[at..at + 3], &PAPER[..3]);
    }

    #[test]
    fn the_active_dot_has_a_ring_and_a_red_centre() {
        let face = stage_dot_face(StageDot::Active);
        assert_eq!((face.w, face.h), (DOT_ACTIVE_PX, DOT_ACTIVE_PX));
        assert_eq!(alpha(&face, 0, 0), 0);
        let centre = ((9 * face.w + 9) * 4) as usize;
        assert_eq!(&face.rgba[centre..centre + 3], &sw::accent()[..]);
    }

    #[test]
    fn the_stamp_is_mostly_transparent_with_red_ink() {
        let face = stamp_face();
        assert_eq!(alpha(&face, 0, 0), 0);
        let inked = (0..face.w * face.h)
            .filter(|i| face.rgba[(*i as usize) * 4 + 3] > 0)
            .count();
        assert!(inked > 300, "the stamp has too little ink: {inked}");
    }

    #[test]
    fn the_direction_face_points_where_it_says() {
        let right = direction_face(Travel::Right);
        let left = direction_face(Travel::Left);
        assert_eq!((right.w, right.h), (DIRECTION_PX, DIRECTION_PX));
        assert_eq!(alpha(&right, 0, 0), 0);
        assert_ne!(right.rgba, left.rgba);
    }

    #[test]
    fn the_packet_is_a_small_cream_slip() {
        let face = packet_face();
        assert_eq!((face.w, face.h), (PACKET_W, PACKET_H));
        let at = ((6 * face.w + 4) * 4) as usize;
        assert_eq!(alpha(&face, 4, 6), 255);
        assert_eq!(&face.rgba[at..at + 3], &PAPER[..3]);
    }
}
