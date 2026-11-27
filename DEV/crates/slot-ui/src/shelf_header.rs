//! Shelf v2's header and centred game title.
//!
//! The header is deliberately a text face rather than a row of widgets: the
//! shelf rail changes with the library, but the frontend can upload one
//! texture and keep the draw list stable.

use crate::keyboard as kb;
use crate::plate::UndoFace;
use crate::switch_style as sw;

pub const HEADER_W: u32 = 664;
pub const HEADER_H: u32 = 60;
pub const SHELF_TITLE_W: u32 = 664;
pub const SHELF_TITLE_H: u32 = 58;

/// One shelf tab: the already-localised label and whether it is the active
/// rack.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RailItem {
    pub label: String,
    pub active: bool,
}

impl RailItem {
    pub fn new(label: impl Into<String>, active: bool) -> Self {
        Self {
            label: label.into(),
            active,
        }
    }
}

/// The top band: a brass rule and `游戏库`, the rail beneath it, and the item
/// count at the far right.  In folder mode `folder` replaces the rail.
pub fn header(title: &str, rail: &[RailItem], count: &str, folder: Option<&str>) -> UndoFace {
    let mut face = kb::blank(HEADER_W, HEADER_H, [0, 0, 0, 0]);
    if face.w == 0 {
        return face;
    }

    let accent = sw::accent();
    kb::fill(&mut face, 0.0, 2.0, 4.0, 16.0, [accent[0], accent[1], accent[2], 255]);
    kb::draw_box(&mut face, title, 18.0, 13.0, 12.0, 0.0, 360.0, 22.0, sw::ink());

    if !count.is_empty() {
        let w = kb::ink_width(count, 9.0);
        kb::draw_text(
            &mut face,
            count,
            9.0,
            HEADER_W as f32 - w,
            2.0,
            sw::ink_dim(),
        );
    }

    match folder {
        Some(folder) => {
            let (line, px) = kb::fit_line(folder, 600.0, 13.0, 10.0);
            kb::draw_text(&mut face, &line, px, 2.0, 30.0, sw::ink_dim());
        }
        None => {
            let mut x = 2.0;
            let mut active_underline: Option<(f32, f32)> = None;
            for item in rail {
                let (line, px) = kb::fit_line(&item.label, 88.0, 11.0, 9.0);
                let w = kb::ink_width(&line, px);
                if x + w > HEADER_W as f32 - 8.0 {
                    break;
                }
                let ink = if item.active { sw::ink() } else { sw::ink_dim() };
                kb::draw_text(&mut face, &line, px, x, 32.0, ink);
                if item.active {
                    active_underline = Some((x, w));
                }
                x += w + 16.0;
            }
            if let Some((x, w)) = active_underline {
                let accent = sw::accent();
                kb::fill(
                    &mut face,
                    x,
                    55.0,
                    w,
                    2.0,
                    [accent[0], accent[1], accent[2], 255],
                );
            }
        }
    }

    face
}

/// Console Flat's flat neighbour tile: 160×160, a 4px colour spine and a cream
/// platform slip.  No image, no filter, no shadow.
pub const FLAT_TILE_W: u32 = 160;
pub const FLAT_TILE_H: u32 = 160;

pub fn flat_side_tile_face(code: &str) -> UndoFace {
    let mut face = sw::rounded_face(
        FLAT_TILE_W,
        FLAT_TILE_H,
        12.0,
        [0x22, 0x22, 0x26, 255],
        Some([0x30, 0x30, 0x37, 255]),
    );
    kb::fill(&mut face, 0.0, 0.0, 4.0, 160.0, [0x4c, 0x3a, 0x34, 255]);
    kb::fill(&mut face, 22.0, 34.0, 116.0, 52.0, [0xf0, 0xe8, 0xdb, 255]);
    kb::draw_box(
        &mut face,
        code,
        7.0,
        4.0,
        32.0,
        46.0,
        104.0,
        28.0,
        [0x1f, 0x1f, 0x22],
    );
    face
}

pub const FLAT_SLOT_W: u32 = 360;
pub const FLAT_SLOT_H: u32 = 58;

pub fn flat_slot_face() -> UndoFace {
    let mut face = sw::rounded_face(
        FLAT_SLOT_W,
        FLAT_SLOT_H,
        7.0,
        [0x26, 0x26, 0x2b, 255],
        Some([0x3b, 0x3b, 0x43, 255]),
    );
    sw::fill_round(
        &mut face,
        8.0,
        7.0,
        344.0,
        27.0,
        4.0,
        [0x08, 0x08, 0x0a, 255],
    );
    sw::fill_round(
        &mut face,
        111.0,
        48.0,
        138.0,
        9.0,
        4.0,
        [0x10, 0x10, 0x13, 255],
    );
    face
}

pub fn flat_page_dot_face(on: bool) -> UndoFace {
    if on {
        sw::rounded_face(20, 6, 3.0, [0xf6, 0xf6, 0xf3, 255], None)
    } else {
        sw::rounded_face(6, 6, 3.0, [0x44, 0x44, 0x4b, 255], None)
    }
}

/// Legacy warm header: platform chip + `游戏库`, item count at the right.
/// No rail: the old design keeps the whole strip on the active rack.
pub const LEGACY_HEADER_W: u32 = 664;
pub const LEGACY_HEADER_H: u32 = 36;

pub fn legacy_header(platform: &str, title: &str, count: &str) -> UndoFace {
    let mut face = kb::blank(LEGACY_HEADER_W, LEGACY_HEADER_H, [0, 0, 0, 0]);
    let accent = sw::accent();
    kb::fill(&mut face, 0.0, 7.0, 4.0, 22.0, [accent[0], accent[1], accent[2], 255]);
    let surface = sw::surface();
    kb::fill(
        &mut face,
        16.0,
        6.0,
        54.0,
        24.0,
        [surface[0], surface[1], surface[2], 255],
    );
    kb::draw_box(
        &mut face,
        platform,
        9.0,
        5.0,
        16.0,
        6.0,
        54.0,
        24.0,
        accent,
    );
    kb::draw_box(
        &mut face,
        title,
        14.0,
        5.0,
        76.0,
        0.0,
        260.0,
        26.0,
        sw::ink(),
    );
    if !count.is_empty() {
        let w = kb::ink_width(count, 9.0);
        kb::draw_text(
            &mut face,
            count,
            9.0,
            LEGACY_HEADER_W as f32 - w,
            9.0,
            sw::ink_dim(),
        );
    }
    face
}

/// The warm shelf's SFC-style plate under the slot.
pub const DESIGN_PLATE_W: u32 = 620;
pub const DESIGN_PLATE_H: u32 = 52;

pub fn design_plate_face(code: &str, name: &str) -> UndoFace {
    let mut face = sw::rounded_face(
        DESIGN_PLATE_W,
        DESIGN_PLATE_H,
        12.0,
        sw::surface(),
        Some(sw::surface_border()),
    );
    let accent = sw::accent();
    kb::draw_box(
        &mut face,
        code,
        24.0,
        12.0,
        238.0,
        9.0,
        120.0,
        34.0,
        accent,
    );
    kb::fill(
        &mut face,
        358.0,
        16.0,
        1.0,
        20.0,
        [sw::surface_border()[0], sw::surface_border()[1], sw::surface_border()[2], 255],
    );
    kb::draw_box(
        &mut face,
        name,
        7.0,
        7.0,
        376.0,
        21.0,
        190.0,
        14.0,
        sw::ink_dim(),
    );
    face
}

/// The centred title above the selected cart: game name on top, platform and
/// playtime on the meta line.
pub fn game_title(title: &str, meta: &str) -> UndoFace {
    let mut face = kb::blank(SHELF_TITLE_W, SHELF_TITLE_H, [0, 0, 0, 0]);
    if title.is_empty() && meta.is_empty() {
        return face;
    }
    kb::draw_box(
        &mut face,
        title,
        26.0,
        14.0,
        12.0,
        0.0,
        SHELF_TITLE_W as f32 - 24.0,
        32.0,
        sw::ink(),
    );
    kb::draw_box(
        &mut face,
        meta,
        9.0,
        7.0,
        12.0,
        32.0,
        SHELF_TITLE_W as f32 - 24.0,
        18.0,
        sw::ink_dim(),
    );
    face
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inked(face: &UndoFace) -> bool {
        face.rgba.chunks(4).any(|px| px[3] > 0)
    }

    #[test]
    fn the_header_draws_a_title_rail_and_count() {
        let rail = [
            RailItem::new("最近", false),
            RailItem::new("GBA", true),
            RailItem::new("GB", false),
        ];
        let face = header("游戏库", &rail, "128 TITLES", None);
        assert_eq!((face.w, face.h), (HEADER_W, HEADER_H));
        assert!(inked(&face));
    }

    #[test]
    fn folder_mode_replaces_the_rail_with_a_breadcrumb() {
        let rail = [RailItem::new("GBA", true)];
        let shelf = header("游戏库", &rail, "1 TITLES", None);
        let folder = header("文件夹模式", &rail, "1 TITLES", Some("动作 / 角色扮演"));
        assert_ne!(shelf.rgba, folder.rgba);
        assert!(inked(&folder));
    }

    #[test]
    fn the_flat_tile_and_slot_have_the_console_flat_dimensions() {
        assert_eq!(
            (flat_side_tile_face("GBA").w, flat_side_tile_face("GBA").h),
            (FLAT_TILE_W, FLAT_TILE_H)
        );
        assert_eq!(
            (flat_slot_face().w, flat_slot_face().h),
            (FLAT_SLOT_W, FLAT_SLOT_H)
        );
        assert_eq!(
            (flat_page_dot_face(false).w, flat_page_dot_face(false).h),
            (6, 6)
        );
        assert_eq!(
            (flat_page_dot_face(true).w, flat_page_dot_face(true).h),
            (20, 6)
        );
    }

    #[test]
    fn the_legacy_header_carries_the_platform_library_and_count() {
        let face = legacy_header("GBA", "游戏库", "128 TITLES");
        assert_eq!((face.w, face.h), (LEGACY_HEADER_W, LEGACY_HEADER_H));
        assert!(inked(&face));
    }

    #[test]
    fn an_empty_title_face_is_transparent() {
        let face = game_title("", "");
        assert!(face.rgba.chunks(4).all(|px| px[3] == 0));
        assert!(inked(&game_title("银河战士 融合", "GBA · 12 小时")));
    }
}
