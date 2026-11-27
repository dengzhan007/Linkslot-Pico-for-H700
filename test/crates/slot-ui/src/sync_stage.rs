//! Sync Ring faces: the flat 520×400 card, the five-stage ring, and the small
//! glyphs that sit inside it (cloud / check / arrow / seal / head / scan /
//! target).  Everything is rasterised once at boot; the app only moves the
//! finished faces and changes the ring face as the stage advances.

use crate::plate::UndoFace;
use crate::switch_style as sw;
use crate::CartFace;

/// The card that holds the whole animation.
pub const CARD_W: u32 = 520;
pub const CARD_H: u32 = 400;
/// The ring face has an 8 px margin so the 2 px stroke and its antialiasing
/// stay inside the texture.
pub const RING_PX: u32 = 192;
pub const RING_R: f32 = 88.0;
pub const RING_STROKE: f32 = 2.0;
/// The angular gap between two stage segments.
pub const RING_GAP_DEG: f32 = 6.0;
pub const SEGMENTS: usize = 5;
pub const SEGMENT_DEG: f32 = 72.0;
pub const SEAL_PX: u32 = 36;
pub const CHECK_W: u32 = 44;
pub const CHECK_H: u32 = 32;
pub const HEAD_PX: u32 = 7;
pub const TARGET_PX: u32 = 48;

fn ink() -> [u8; 3] {
    sw::ink()
}

fn accent() -> [u8; 3] {
    sw::accent()
}

fn track() -> [u8; 3] {
    let border = sw::surface_border();
    [border[0], border[1], border[2]]
}

fn cart(face: UndoFace) -> CartFace {
    CartFace {
        rgba: face.rgba,
        w: face.w,
        h: face.h,
    }
}

fn blank(w: u32, h: u32) -> UndoFace {
    UndoFace {
        rgba: vec![0u8; (w * h * 4) as usize],
        w,
        h,
    }
}

/// Coverage of a 1 px-ish stroke at one pixel: 1 inside, fading over one pixel
/// at the edge so the small glyphs do not look like staircases.
fn coverage(distance: f32, width: f32) -> u8 {
    ((width / 2.0 + 0.5 - distance).clamp(0.0, 1.0) * 255.0).round() as u8
}

fn put(rgba: &mut [u8], w: u32, h: u32, x: i32, y: i32, colour: [u8; 3], alpha: u8) {
    if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
        return;
    }
    let at = ((y as u32 * w + x as u32) * 4) as usize;
    if rgba[at + 3] >= alpha {
        return;
    }
    rgba[at..at + 3].copy_from_slice(&colour);
    rgba[at + 3] = alpha;
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

fn stroke(
    face: &mut UndoFace,
    points: &[(f32, f32)],
    width: f32,
    colour: [u8; 3],
) {
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
            let alpha = coverage(distance, width);
            if alpha > 0 {
                let at = ((y * face.w + x) * 4) as usize;
                if face.rgba[at + 3] < alpha {
                    face.rgba[at..at + 3].copy_from_slice(&colour);
                    face.rgba[at + 3] = alpha;
                }
            }
        }
    }
}

fn circle_outline(face: &mut UndoFace, cx: f32, cy: f32, radius: f32, width: f32, colour: [u8; 3]) {
    for y in 0..face.h {
        for x in 0..face.w {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let distance = ((px - cx) * (px - cx) + (py - cy) * (py - cy)).sqrt();
            let alpha = coverage((distance - radius).abs(), width);
            if alpha > 0 {
                let at = ((y * face.w + x) * 4) as usize;
                if face.rgba[at + 3] < alpha {
                    face.rgba[at..at + 3].copy_from_slice(&colour);
                    face.rgba[at + 3] = alpha;
                }
            }
        }
    }
}

/// The running sheet: the same 12 px card as the other flat popups.
pub fn sync_card_face() -> CartFace {
    cart(sw::rounded_face(
        CARD_W,
        CARD_H,
        12.0,
        sw::surface(),
        Some(sw::surface_border()),
    ))
}

/// The five-stage ring.  `done` is the number of completed stages (0–5); the
/// next segment is red, the later ones are the track colour.  At `done = 5`
/// the five gaps close into one continuous white ring.
pub fn sync_ring_face(done: usize) -> CartFace {
    let size = RING_PX;
    let centre = size as f32 / 2.0;
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    let samples = 4u32;
    let total = (samples * samples) as u32;
    let done = done.min(SEGMENTS);
    for y in 0..size {
        for x in 0..size {
            let mut hits = 0u32;
            let mut colour = [0u8; 3];
            for sy in 0..samples {
                for sx in 0..samples {
                    let px = x as f32 + (sx as f32 + 0.5) / samples as f32;
                    let py = y as f32 + (sy as f32 + 0.5) / samples as f32;
                    let dx = px - centre;
                    let dy = py - centre;
                    let distance = (dx * dx + dy * dy).sqrt();
                    if (distance - RING_R).abs() > RING_STROKE / 2.0 + 0.5 {
                        continue;
                    }
                    if done < SEGMENTS {
                        // PIL/image coordinates: 0° at 3 o'clock, growing
                        // clockwise because y points down.  The first segment
                        // starts at 12 o'clock.
                        let angle = dy.atan2(dx).to_degrees().rem_euclid(360.0);
                        let offset = (angle - 270.0).rem_euclid(360.0);
                        let segment = (offset / SEGMENT_DEG).floor() as usize;
                        let within = offset - segment as f32 * SEGMENT_DEG;
                        if within < RING_GAP_DEG / 2.0
                            || within > SEGMENT_DEG - RING_GAP_DEG / 2.0
                        {
                            continue;
                        }
                        colour = if segment < done {
                            ink()
                        } else if segment == done {
                            accent()
                        } else {
                            track()
                        };
                    } else {
                        colour = ink();
                    }
                    hits += 1;
                }
            }
            if hits == 0 {
                continue;
            }
            let at = ((y * size + x) * 4) as usize;
            rgba[at..at + 3].copy_from_slice(&colour);
            rgba[at + 3] = (hits * 255 / total) as u8;
        }
    }
    CartFace {
        rgba,
        w: size,
        h: size,
    }
}

/// The white completion check that replaces the cloud.
pub fn sync_check_face() -> CartFace {
    let mut face = blank(CHECK_W, CHECK_H);
    stroke(
        &mut face,
        &[(6.0, 16.0), (17.0, 26.0), (38.0, 5.0)],
        3.0,
        ink(),
    );
    cart(face)
}

/// The red completion seal: two thin rings and a small white check.
pub fn sync_seal_face() -> CartFace {
    let mut face = blank(SEAL_PX, SEAL_PX);
    let centre = SEAL_PX as f32 / 2.0;
    circle_outline(&mut face, centre, centre, 15.0, 2.0, accent());
    circle_outline(&mut face, centre, centre, 12.0, 1.0, accent());
    stroke(&mut face, &[(11.0, 18.0), (16.0, 23.0), (25.0, 12.0)], 2.0, ink());
    cart(face)
}

/// The red dot that travels along the current segment.
pub fn sync_head_face() -> CartFace {
    let mut face = blank(HEAD_PX, HEAD_PX);
    let a = accent();
    sw::fill_round(
        &mut face,
        0.0,
        0.0,
        HEAD_PX as f32,
        HEAD_PX as f32,
        HEAD_PX as f32 / 2.0,
        [a[0], a[1], a[2], 255],
    );
    cart(face)
}

/// The verify scan: one 1 px radial line from the centre to the inner edge of
/// the ring, with a small dot at its tip.  Drawn at the design's 42° angle.
pub fn sync_scan_face() -> CartFace {
    let size = RING_PX;
    let centre = size as f32 / 2.0;
    let angle = 42.0f32.to_radians();
    let ex = centre + (RING_R - 8.0) * angle.cos();
    let ey = centre + (RING_R - 8.0) * angle.sin();
    let mut face = blank(size, size);
    stroke(&mut face, &[(centre, centre), (ex, ey)], 1.0, ink());
    let a = ink();
    for y in 0..size as i32 {
        for x in 0..size as i32 {
            let dx = x as f32 + 0.5 - ex;
            let dy = y as f32 + 0.5 - ey;
            let alpha = coverage((dx * dx + dy * dy).sqrt(), 4.0);
            if alpha > 0 {
                put(&mut face.rgba, size, size, x, y, a, alpha);
            }
        }
    }
    cart(face)
}

/// The `show_cloud = false` centre glyph: a thin reticle with a red core, used
/// by manual mods whose target is a network endpoint rather than a cloud.
pub fn sync_target_face() -> CartFace {
    let mut face = blank(TARGET_PX, TARGET_PX);
    let centre = TARGET_PX as f32 / 2.0;
    let radius = 14.0;
    circle_outline(&mut face, centre, centre, radius, 1.0, ink());
    stroke(&mut face, &[(centre - radius, centre), (centre + radius, centre)], 1.0, ink());
    stroke(&mut face, &[(centre, centre - radius), (centre, centre + radius)], 1.0, ink());
    let a = accent();
    sw::fill_round(&mut face, centre - 2.5, centre - 2.5, 5.0, 5.0, 2.5, [a[0], a[1], a[2], 255]);
    cart(face)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alpha(face: &CartFace, x: u32, y: u32) -> u8 {
        face.rgba[((y * face.w + x) * 4 + 3) as usize]
    }

    fn rgb(face: &CartFace, x: u32, y: u32) -> [u8; 3] {
        let at = ((y * face.w + x) * 4) as usize;
        [face.rgba[at], face.rgba[at + 1], face.rgba[at + 2]]
    }

    #[test]
    fn the_five_stage_ring_starts_red_and_closes_white() {
        let first = sync_ring_face(0);
        assert_eq!((first.w, first.h), (RING_PX, RING_PX));
        // The middle of segment 0 is 36° clockwise from 12 o'clock; the exact
        // 12 o'clock line is the inter-segment gap.
        let angle = 36.0f32.to_radians();
        let x = (RING_PX as f32 / 2.0 + RING_R * angle.sin()) as u32;
        let y = (RING_PX as f32 / 2.0 - RING_R * angle.cos()) as u32;
        assert_eq!(rgb(&first, x, y), accent());
        let second = sync_ring_face(1);
        assert_eq!(rgb(&second, x, y), ink());
        let last = sync_ring_face(5);
        assert_eq!(rgb(&last, x, y), ink());
        // The centre and the outside stay transparent.
        assert_eq!(alpha(&last, RING_PX / 2, RING_PX / 2), 0);
        assert_eq!(alpha(&last, 1, 1), 0);
    }

    #[test]
    fn the_completion_ring_has_no_stage_gaps() {
        let closed = sync_ring_face(5);
        // 12 o'clock is a gap in the running ring but solid once closed.
        let top = (RING_PX as f32 / 2.0 - RING_R + 0.5) as u32;
        let running = sync_ring_face(2);
        assert_eq!(alpha(&running, RING_PX / 2, top), 0);
        assert!(alpha(&closed, RING_PX / 2, top) > 200);
    }

    #[test]
    fn the_finish_marks_have_the_project_colours() {
        let check = sync_check_face();
        assert!(check.rgba.iter().any(|byte| *byte > 0));
        let seal = sync_seal_face();
        assert!(seal.rgba.iter().any(|byte| *byte > 0));
        let head = sync_head_face();
        assert_eq!((head.w, head.h), (HEAD_PX, HEAD_PX));
        let centre = (HEAD_PX * HEAD_PX / 2) as usize * 4;
        assert_eq!(&head.rgba[centre..centre + 3], &accent()[..]);
        let target = sync_target_face();
        assert_eq!((target.w, target.h), (TARGET_PX, TARGET_PX));
    }

    #[test]
    fn the_card_is_the_flat_running_sheet() {
        let card = sync_card_face();
        assert_eq!((card.w, card.h), (CARD_W, CARD_H));
        assert!(alpha(&card, CARD_W / 2, CARD_H / 2) > 0);
    }
}
