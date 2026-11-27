//! Full-screen guide pages: a scrollable text viewport with a position bar,
//! and single-screen EPUB images.
//!
//! A TXT guide is one long stream of wrapped lines, not a stack of fixed
//! pages: the app owns the line offset, so a D-pad press moves one line and
//! L/R move a whole screen.  The bar in the right margin is the reader's only
//! position cue and is drawn into the same face, keeping the texture cache in
//! the frontend keyed by one scroll offset.

use slot_i18n::Language;

use crate::draw::{OUT_H, OUT_W};
use crate::keyboard as kb;
use crate::plate::UndoFace;
use crate::switch_style as sw;
use crate::text;

pub const TEXT_PX: f32 = 18.0;
pub const TEXT_W: f32 = 624.0;

const TEXT_X: f32 = 48.0;
const TEXT_TOP: f32 = 92.0;
/// The footer sits at `OUT_H - 30`; leave a comfortable gap above it.
const TEXT_BOTTOM: f32 = 438.0;
const LINE_GAP: f32 = 5.0;

/// The scroll bar lives in the right margin (the text ends at 672).
const BAR_X: f32 = 684.0;
const BAR_W: f32 = 4.0;
/// A short guide still gets a thumb tall enough to read as a position cue.
const BAR_MIN_H: f32 = 20.0;

const IMAGE_X: f32 = 24.0;
const IMAGE_Y: f32 = 88.0;
const IMAGE_W: f32 = 672.0;
const IMAGE_H: f32 = 342.0;

fn line_step() -> f32 {
    text::line_height(TEXT_PX) + LINE_GAP
}

/// Lines that fit in the reader's text viewport.  Derived from the installed
/// face's metrics rather than pinned to a constant: a theme pack can replace
/// the font, and the old fixed count overflowed the panel with the bundled
/// font, printing the last lines under the footer.
pub fn visible_lines() -> usize {
    (((TEXT_BOTTOM - TEXT_TOP) / line_step()).floor() as usize).max(1)
}

#[allow(clippy::too_many_arguments)]
pub fn page_face(
    title: &str,
    lines: &[String],
    scroll: usize,
    max_scroll: usize,
    page: usize,
    pages: usize,
    lang: Language,
) -> UndoFace {
    let mut face = kb::blank(OUT_W, OUT_H, sw::bg());
    kb::fill(&mut face, 28.0, 40.0, OUT_W as f32 - 56.0, 1.0, [0x3a, 0x3a, 0x42, 0xff]);

    let (heading, px) = kb::fit_line(title, TEXT_W, 26.0, 15.0);
    kb::draw_text(&mut face, &heading, px, 32.0, 48.0, sw::ink());

    let line_h = line_step();
    for (index, line) in lines.iter().take(visible_lines()).enumerate() {
        let top = TEXT_TOP + index as f32 * line_h;
        kb::draw_text(&mut face, line, TEXT_PX, TEXT_X, top, sw::ink());
    }
    scroll_bar(&mut face, scroll, max_scroll, line_h);

    let footer = slot_i18n::t_args(
        lang,
        "device.app.guide.footer",
        &[
            ("page", page.saturating_add(1).to_string().as_str()),
            ("pages", pages.max(1).to_string().as_str()),
        ],
    );
    let (footer, px) = kb::fit_line(&footer, TEXT_W, 14.0, 11.0);
    kb::draw_text(&mut face, &footer, px, TEXT_X, OUT_H as f32 - 30.0, sw::ink_dim());
    face
}

/// The right-margin track and thumb.  `scroll` is the first visible line and
/// `max_scroll` the last reachable one, so the thumb always fills a fraction
/// of the track that matches the fraction of the guide on screen.
fn scroll_bar(face: &mut UndoFace, scroll: usize, max_scroll: usize, line_h: f32) {
    if max_scroll == 0 {
        return;
    }
    let visible = visible_lines();
    let track_h = visible as f32 * line_h;
    let document = (max_scroll + visible) as f32;
    let thumb_h = (track_h * visible as f32 / document)
        .max(BAR_MIN_H)
        .min(track_h);
    let progress = scroll.min(max_scroll) as f32 / max_scroll as f32;
    let thumb_y = TEXT_TOP + (track_h - thumb_h) * progress;

    let track = sw::surface();
    let thumb = sw::ink_dim();
    kb::fill(face, BAR_X, TEXT_TOP, BAR_W, track_h, track);
    kb::fill(
        face,
        BAR_X - 1.0,
        thumb_y,
        BAR_W + 2.0,
        thumb_h,
        [thumb[0], thumb[1], thumb[2], 0xff],
    );
}

/// Full-screen image page used by EPUB guides.  The image is contained inside
/// the content area; it is never cropped or stretched.
pub fn image_face(
    title: &str,
    rgba: &[u8],
    width: u32,
    height: u32,
    page: usize,
    pages: usize,
    lang: Language,
) -> UndoFace {
    let mut face = kb::blank(OUT_W, OUT_H, sw::bg());
    kb::fill(&mut face, 28.0, 40.0, OUT_W as f32 - 56.0, 1.0, [0x3a, 0x3a, 0x42, 0xff]);
    kb::fill(&mut face, IMAGE_X, IMAGE_Y, IMAGE_W, IMAGE_H, [0x0b, 0x0b, 0x10, 0xff]);

    let (heading, px) = kb::fit_line(title, 600.0, 26.0, 15.0);
    kb::draw_text(&mut face, &heading, px, 32.0, 48.0, sw::ink());

    if width > 0 && height > 0 && rgba.len() >= (width as usize * height as usize * 4) {
        let scale = (IMAGE_W / width as f32).min(IMAGE_H / height as f32);
        let draw_w = ((width as f32 * scale).round() as u32).clamp(1, IMAGE_W as u32);
        let draw_h = ((height as f32 * scale).round() as u32).clamp(1, IMAGE_H as u32);
        let left = IMAGE_X + (IMAGE_W - draw_w as f32) / 2.0;
        let top = IMAGE_Y + (IMAGE_H - draw_h as f32) / 2.0;
        kb::fill(&mut face, left - 2.0, top - 2.0, draw_w as f32 + 4.0, 2.0, [0x3a, 0x3a, 0x42, 0xff]);
        kb::fill(&mut face, left - 2.0, top + draw_h as f32, draw_w as f32 + 4.0, 2.0, [0x3a, 0x3a, 0x42, 0xff]);
        for y in 0..draw_h {
            let sy = ((y as f32 + 0.5) / scale).min(height as f32 - 1.0) as u32;
            for x in 0..draw_w {
                let sx = ((x as f32 + 0.5) / scale).min(width as f32 - 1.0) as u32;
                let src = ((sy * width + sx) * 4) as usize;
                let dst = (((top as u32 + y) * OUT_W + left as u32 + x) * 4) as usize;
                face.rgba[dst..dst + 4].copy_from_slice(&rgba[src..src + 4]);
            }
        }
    }

    let footer = slot_i18n::t_args(
        lang,
        "device.app.guide.image_footer",
        &[
            ("page", page.saturating_add(1).to_string().as_str()),
            ("pages", pages.max(1).to_string().as_str()),
        ],
    );
    let (footer, px) = kb::fit_line(&footer, TEXT_W, 14.0, 11.0);
    kb::draw_text(&mut face, &footer, px, 48.0, OUT_H as f32 - 30.0, sw::ink_dim());
    face
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel(face: &UndoFace, x: u32, y: u32) -> [u8; 3] {
        let at = ((y * OUT_W + x) * 4) as usize;
        [face.rgba[at], face.rgba[at + 1], face.rgba[at + 2]]
    }

    #[test]
    fn an_image_page_fits_the_picture_inside_the_content_box() {
        let rgba = vec![255u8; 4 * 4];
        let face = image_face("测试图片", &rgba, 2, 2, 0, 2, Language::ZhCn);
        assert_eq!((face.w, face.h), (OUT_W, OUT_H));
        assert!(face.rgba.chunks(4).any(|pixel| pixel[0] > 0x80));
    }

    #[test]
    fn a_guide_page_is_full_screen_and_has_ink() {
        let face = page_face(
            "测试攻略",
            &["第一行".to_string(), "第二行".to_string()],
            0,
            0,
            0,
            3,
            Language::ZhCn,
        );
        assert_eq!((face.w, face.h), (OUT_W, OUT_H));
        assert!(face.rgba.chunks(4).any(|pixel| pixel[0] > 0x80));
    }

    #[test]
    fn a_scrollable_page_draws_the_bar_and_moves_the_thumb_with_the_scroll() {
        let visible = visible_lines();
        let lines: Vec<String> = (0..30).map(|line| format!("第 {line} 行")).collect();
        let max = lines.len() - visible;
        let top = page_face("攻略", &lines[..visible], 0, max, 0, 2, Language::ZhCn);
        let bottom = page_face("攻略", &lines[max..], max, max, 1, 2, Language::ZhCn);
        let plain = page_face("攻略", &lines[..visible], 0, 0, 0, 1, Language::ZhCn);

        let bar = sw::surface();
        let thumb = sw::ink_dim();
        let track_y = (TEXT_TOP + 8.0) as u32;
        let foot_y = (TEXT_TOP + visible as f32 * line_step() - 14.0) as u32;

        assert_eq!(pixel(&plain, BAR_X as u32, track_y), [sw::bg()[0], sw::bg()[1], sw::bg()[2]]);
        assert_eq!(
            pixel(&top, BAR_X as u32, track_y),
            [thumb[0], thumb[1], thumb[2]],
            "a fresh page does not show the thumb at the top"
        );
        assert_eq!(pixel(&top, BAR_X as u32, foot_y), [bar[0], bar[1], bar[2]]);
        assert_eq!(
            pixel(&bottom, BAR_X as u32, foot_y),
            [thumb[0], thumb[1], thumb[2]],
            "the thumb did not follow the scroll to the bottom"
        );
        assert_eq!(
            pixel(&bottom, BAR_X as u32, track_y),
            [bar[0], bar[1], bar[2]],
            "the thumb did not leave the top"
        );
    }

    #[test]
    fn the_footer_fits_without_truncation_in_every_language() {
        for lang in Language::ALL {
            for key in ["device.app.guide.footer", "device.app.guide.image_footer"] {
                let footer = slot_i18n::t_args(
                    lang,
                    key,
                    &[("page", "999"), ("pages", "999")],
                );
                let (fit, _) = kb::fit_line(&footer, TEXT_W, 14.0, 11.0);
                assert_eq!(fit, footer, "{lang:?} {key} was truncated: {footer}");
            }
        }
    }

    #[test]
    fn the_text_viewport_fits_between_the_rule_and_the_footer() {
        let visible = visible_lines();
        let last_top = TEXT_TOP + (visible - 1) as f32 * line_step();
        assert!(
            last_top + text::line_height(TEXT_PX) <= TEXT_BOTTOM,
            "{visible} lines overflow the text viewport"
        );
        let next_top = TEXT_TOP + visible as f32 * line_step();
        assert!(
            next_top + text::line_height(TEXT_PX) > TEXT_BOTTOM,
            "the viewport wastes room on a line that would fit"
        );
    }
}
