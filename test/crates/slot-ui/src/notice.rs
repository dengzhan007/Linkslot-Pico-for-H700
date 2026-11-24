//! A non-blocking notice: one card under the status bar that fades away.
//!
//! Used where a dialog would only ask the player to press a key: a dropped
//! peer, a failed cartridge transfer.  It never takes input and never covers
//! the game — five seconds and it is gone.

use slot_gfx::OUT_W;

use crate::keyboard as kb;
use crate::switch_style as sw;
use crate::plate::UndoFace;

pub const NOTICE_W: f32 = 420.0;
pub const NOTICE_H: f32 = 62.0;
/// Just under the 40 px status bar.
pub const NOTICE_TOP: f32 = 52.0;
/// How long a notice stays on screen, counting the fades.
pub const NOTICE_MS: u64 = 5000;
/// Fade in at the start and out at the end.
pub const NOTICE_FADE_MS: u64 = 300;
/// 成就徽章在卡片里的像素尺寸（正方形）。
pub const BADGE_PX: u32 = 44;

pub fn notice_rect() -> (f32, f32, f32, f32) {
    ((OUT_W as f32 - NOTICE_W) / 2.0, NOTICE_TOP, NOTICE_W, NOTICE_H)
}

/// 0..1 visibility for a notice that appeared at `shown` milliseconds.
pub fn notice_alpha(shown: u64, now: u64) -> f32 {
    let age = now.saturating_sub(shown);
    if age >= NOTICE_MS {
        return 0.0;
    }
    let fade_in = (age as f32 / NOTICE_FADE_MS as f32).min(1.0);
    let fade_out = ((NOTICE_MS - age) as f32 / NOTICE_FADE_MS as f32).min(1.0);
    fade_in.min(fade_out)
}

/// The card itself: transparent everywhere except the plate, so the caller can
/// blit it straight over the picture.
pub fn notice_face(title: &str, detail: &str, badge: Option<&[u8]>) -> UndoFace {
    let w = NOTICE_W as u32;
    let h = NOTICE_H as u32;
    let mut face = kb::blank(w, h, [0, 0, 0, 0]);
    let radius = 12.0;
    // Soft drop shadow, then the plate in the dialog's own colours.
    kb::fill(&mut face, 0.0, 3.0, NOTICE_W, NOTICE_H, [0, 0, 0, 80]);
    sw::fill_round(&mut face, 0.0, 0.0, NOTICE_W, NOTICE_H, radius, sw::surface_border());
    sw::fill_round(
        &mut face,
        1.0,
        1.0,
        NOTICE_W - 2.0,
        NOTICE_H - 2.0,
        (radius - 1.0).max(0.0),
        sw::surface(),
    );
    let has_badge = badge.is_some_and(|rgba| rgba.len() == (BADGE_PX * BADGE_PX * 4) as usize);
    // The same accent spine the modal dialogs carry.
    let accent = sw::accent();
    let spine_x = if has_badge { 6.0 } else { 14.0 };
    kb::fill(
        &mut face,
        spine_x,
        13.0,
        4.0,
        NOTICE_H - 26.0,
        [accent[0], accent[1], accent[2], 0xff],
    );
    if has_badge {
        if let Some(rgba) = badge {
            blit_rgba(&mut face, rgba, BADGE_PX, BADGE_PX, 12.0, 9.0);
        }
    }
    let text_x = if has_badge { 66.0 } else { 28.0 };
    let text_w = NOTICE_W - text_x - 16.0;
    kb::draw_left(
        &mut face,
        title,
        if has_badge { 16.0 } else { 17.0 },
        if has_badge { 11.0 } else { 12.0 },
        text_w,
        text_x,
        9.0,
        22.0,
        sw::ink(),
    );
    kb::draw_left(
        &mut face,
        detail,
        12.0,
        9.0,
        text_w,
        text_x,
        34.0,
        20.0,
        sw::ink_dim(),
    );
    face
}

fn blit_rgba(face: &mut UndoFace, src: &[u8], w: u32, h: u32, x: f32, y: f32) {
    let x = x.round() as i32;
    let y = y.round() as i32;
    for sy in 0..h as i32 {
        for sx in 0..w as i32 {
            let dx = x + sx;
            let dy = y + sy;
            if dx < 0 || dy < 0 || dx >= face.w as i32 || dy >= face.h as i32 {
                continue;
            }
            let si = ((sy as u32 * w + sx as u32) * 4) as usize;
            let di = ((dy as u32 * face.w + dx as u32) * 4) as usize;
            let a = src[si + 3] as f32 / 255.0;
            if a <= 0.0 {
                continue;
            }
            for c in 0..3 {
                face.rgba[di + c] =
                    (face.rgba[di + c] as f32 * (1.0 - a) + src[si + c] as f32 * a).round() as u8;
            }
            face.rgba[di + 3] = 255;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_card_starts_and_ends_invisible() {
        assert_eq!(notice_alpha(0, 0), 0.0, "it appeared at full alpha");
        assert_eq!(notice_alpha(0, NOTICE_MS), 0.0, "it outlived five seconds");
        assert_eq!(notice_alpha(0, 10_000), 0.0);
    }

    #[test]
    fn the_card_is_opaque_in_the_middle_of_its_life() {
        assert_eq!(notice_alpha(0, NOTICE_FADE_MS), 1.0);
        assert_eq!(notice_alpha(0, 2_500), 1.0);
        assert!(
            notice_alpha(0, NOTICE_MS - NOTICE_FADE_MS / 2) < 1.0,
            "the tail did not fade"
        );
    }

    #[test]
    fn the_face_is_a_rounded_card_with_clean_corners() {
        let face = notice_face("联机已断开", "对方已断开连接 · 可继续游戏", None);
        assert_eq!((face.w, face.h), (NOTICE_W as u32, NOTICE_H as u32));
        let at = |x: u32, y: u32| face.rgba[((y * face.w + x) * 4 + 3) as usize];
        assert_eq!(at(0, 0), 0, "the card corner is square");
        assert_eq!(at(face.w - 1, 0), 0, "the card corner is square");
        assert!(at(face.w / 2, face.h / 2) > 200, "the card has no plate");
        let opaque = face.rgba.chunks_exact(4).filter(|p| p[3] > 0).count();
        assert!(opaque > 5_000, "the card is nearly empty: {opaque}");
    }

    #[test]
    fn the_card_sits_under_the_status_bar() {
        let (_, y, _, h) = notice_rect();
        assert!(y >= 40.0, "the notice overlapped the status bar: {y}");
        assert!(y + h <= 480.0, "the notice ran off the screen");
    }
}
#[cfg(test)]
mod badge_tests {
    use super::*;

    fn pixel(face: &UndoFace, x: u32, y: u32) -> [u8; 4] {
        let at = ((y as usize * face.w as usize) + x as usize) * 4;
        [
            face.rgba[at],
            face.rgba[at + 1],
            face.rgba[at + 2],
            face.rgba[at + 3],
        ]
    }

    #[test]
    fn a_badge_is_drawn_into_the_card() {
        let badge = [250u8, 20, 250, 255].repeat((BADGE_PX * BADGE_PX) as usize);
        let with = notice_face("成就解锁", "初入丛林 · +5", Some(&badge));
        let plain = notice_face("成就解锁", "初入丛林 · +5", None);

        let badge_px = pixel(&with, 30, 20);
        let plain_px = pixel(&plain, 30, 20);
        let diff: i32 = (0..3)
            .map(|channel| (i32::from(badge_px[channel]) - i32::from(plain_px[channel])).abs())
            .sum();
        assert!(
            diff > 100,
            "the badge did not cover the plate: {badge_px:?} vs {plain_px:?}"
        );
        assert_eq!(badge_px[3], 255, "the badge left the card translucent");
    }
}
