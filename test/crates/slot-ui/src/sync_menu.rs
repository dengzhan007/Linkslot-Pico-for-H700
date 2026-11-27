//! The on-device WebDAV "sync method" screen, drawn in the same charcoal /
//! white-card / red-accent language as the Wi-Fi and Quick menus.
//!
//! The switch action is intentionally not available until the app reports a
//! successful connection test for the highlighted profile.  That gate lives in
//! the app; this module only renders `can_switch` and `test_ready`.

use slot_gfx::{Draw, OUT_H, OUT_W};
use slot_i18n::Language;

use crate::keyboard as kb;
use crate::plate::UndoFace;
use crate::switch_style as sw;

pub struct SyncMenuRow<'a> {
    pub name: &'a str,
    pub detail: &'a str,
    pub active: bool,
    pub password_set: bool,
}

pub struct SyncMenuView<'a> {
    pub rows: &'a [SyncMenuRow<'a>],
    pub selected: usize,
    /// True when the highlighted profile is a change that may be tested.
    pub test_ready: bool,
    pub testing: bool,
    /// True when the current profile is already the active one.
    pub active: bool,
    pub can_switch: bool,
    pub status: Option<&'a str>,
    pub error: Option<&'a str>,
    /// First use: no WebDAV profile has been filled in yet.  The page tells the
    /// user to start the Web service instead of showing an empty list.
    pub needs_setup: bool,
}

const SIDE: f32 = 24.0;
const CARD_W: f32 = OUT_W as f32 - 2.0 * SIDE;
const STATUS_TOP: f32 = 96.0;
const STATUS_H: f32 = 40.0;
const LIST_TOP: f32 = 144.0;
const LIST_ROWS: usize = 4;
const ROW_H: f32 = 42.0;
const ROW_GAP: f32 = 4.0;
const TEST_TOP: f32 = 336.0;
const SWITCH_TOP: f32 = 386.0;
const ACTION_H: f32 = 40.0;

pub fn sync_menu_face(view: &SyncMenuView, lang: Language) -> UndoFace {
    let mut face = UndoFace {
        rgba: vec![0u8; (OUT_W * OUT_H * 4) as usize],
        w: OUT_W,
        h: OUT_H,
    };
    fill(&mut face, 0.0, 0.0, OUT_W as f32, OUT_H as f32, sw::bg());
    fill(&mut face, 28.0, 40.0, OUT_W as f32 - 56.0, 1.0, sw::surface_border());
    kb::draw_left(
        &mut face,
        slot_i18n::t(lang, "device.quick.sync.title"),
        26.0,
        15.0,
        500.0,
        32.0,
        48.0,
        34.0,
        sw::ink(),
    );

    if view.needs_setup {
        kb::draw_left(
            &mut face,
            slot_i18n::t(lang, "device.quick.sync.empty"),
            18.0,
            12.0,
            CARD_W - 36.0,
            SIDE + 18.0,
            LIST_TOP + 16.0,
            30.0,
            sw::ink(),
        );
        kb::draw_left(
            &mut face,
            slot_i18n::t(lang, "device.quick.sync.step_web"),
            13.0,
            10.0,
            CARD_W - 36.0,
            SIDE + 18.0,
            LIST_TOP + 56.0,
            24.0,
            sw::ink(),
        );
        kb::draw_left(
            &mut face,
            slot_i18n::t(lang, "device.quick.sync.step_wifi"),
            11.0,
            9.0,
            CARD_W - 36.0,
            SIDE + 18.0,
            LIST_TOP + 92.0,
            20.0,
            sw::ink_dim(),
        );
        kb::draw_left(
            &mut face,
            slot_i18n::t(lang, "device.quick.sync.step_open"),
            11.0,
            9.0,
            CARD_W - 36.0,
            SIDE + 18.0,
            LIST_TOP + 114.0,
            20.0,
            sw::ink_dim(),
        );
        action_card(&mut face, lang, TEST_TOP, "A", slot_i18n::t(lang, "device.quick.sync.start_web"), true, false);
        kb::draw_box(
            &mut face,
            slot_i18n::t(lang, "device.quick.sync.start_hint"),
            12.0,
            9.0,
            SIDE,
            418.0,
            CARD_W,
            24.0,
            sw::ink_dim(),
        );
        return face;
    }

    // Status / error card.
    let (status, ink) = if let Some(error) = view.error {
        (error, accent_ink())
    } else if let Some(status) = view.status {
        (status, sw::ink())
    } else if view.testing {
        (slot_i18n::t(lang, "device.quick.sync.testing"), sw::ink())
    } else if view.test_ready {
        (slot_i18n::t(lang, "device.quick.sync.press_test"), sw::ink_dim())
    } else {
        (slot_i18n::t(lang, "device.quick.sync.test_other"), sw::ink_dim())
    };
    kb::draw_left(
        &mut face,
        status,
        10.0,
        8.0,
        CARD_W,
        SIDE,
        STATUS_TOP,
        STATUS_H,
        ink,
    );

    if view.rows.is_empty() {
        kb::draw_box(
            &mut face,
            slot_i18n::t(lang, "device.quick.sync.need_webdav"),
            15.0,
            11.0,
            SIDE,
            LIST_TOP,
            CARD_W,
            120.0,
            sw::ink_dim(),
        );
    } else {
        let start = if view.rows.len() > LIST_ROWS {
            view.selected
                .saturating_sub(LIST_ROWS - 1)
                .min(view.rows.len() - LIST_ROWS)
        } else {
            0
        };
        for (at, row) in view.rows.iter().enumerate().skip(start).take(LIST_ROWS) {
            let y = LIST_TOP + (at - start) as f32 * (ROW_H + ROW_GAP);
            let selected = at == view.selected;
            card(&mut face, y, ROW_H, selected);
            let row_ink = if selected { sw::ink_selected() } else { sw::ink() };
            let dim = if selected {
                [0x55, 0x55, 0x5d]
            } else {
                sw::ink_dim()
            };
            kb::draw_left(
                &mut face,
                row.name,
                17.0,
                13.0,
                260.0,
                SIDE + 16.0,
                y,
                26.0,
                row_ink,
            );
            kb::draw_left(
                &mut face,
                row.detail,
                11.0,
                10.0,
                330.0,
                SIDE + 16.0,
                y + 26.0,
                15.0,
                dim,
            );
            if row.active {
                kb::draw_left(
                    &mut face,
                    slot_i18n::t(lang, "device.quick.sync.current"),
                    15.0,
                    12.0,
                    52.0,
                    SIDE + CARD_W - 112.0,
                    y,
                    ROW_H,
                    row_ink,
                );
            }
            let lock = if row.password_set { slot_i18n::t(lang, "device.quick.sync.password_set") } else { slot_i18n::t(lang, "device.quick.sync.no_password") };
            kb::draw_left(
                &mut face,
                lock,
                12.0,
                10.0,
                92.0,
                SIDE + CARD_W - 174.0,
                y,
                ROW_H,
                dim,
            );
        }
    }

    // The two actions are deliberately giant, high-contrast cards: the test
    // action lights up for a changed profile, the switch stays grey until the
    // app has a successful test token for that exact profile.
    action_card(
        &mut face,
        lang,
        TEST_TOP,
        "A",
        if view.testing {
            slot_i18n::t(lang, "device.quick.sync.testing_short")
        } else {
            slot_i18n::t(lang, "device.quick.sync.test")
        },
        view.test_ready || view.testing,
        false,
    );
    let switch_text = if view.active {
        slot_i18n::t(lang, "device.quick.sync.active")
    } else {
        slot_i18n::t(lang, "device.quick.sync.switch")
    };
    action_card(
        &mut face,
        lang,
        SWITCH_TOP,
        "X",
        switch_text,
        view.can_switch,
        true,
    );

    kb::draw_box(
        &mut face,
        slot_i18n::t(lang, "device.quick.sync.hint"),
        12.0,
        9.0,
        SIDE,
        436.0,
        CARD_W,
        20.0,
        sw::ink_dim(),
    );
    face
}

fn card(face: &mut UndoFace, y: f32, h: f32, selected: bool) {
    if !selected {
        return;
    }
    sw::fill_round(face, SIDE, y, CARD_W, h, 11.0, sw::selected());
    sw::fill_round(face, SIDE - 8.0, y + (h - 24.0) / 2.0, 3.0, 24.0, 2.0, accent());
}

#[allow(clippy::too_many_arguments)]
fn action_card(
    face: &mut UndoFace,
    lang: Language,
    y: f32,
    key: &str,
    label: &str,
    enabled: bool,
    switch: bool,
) {
    let (fill, border) = if enabled && switch {
        (sw::selected(), None)
    } else if enabled {
        ([0x00, 0x00, 0x00, 0x00], Some(sw::surface_border()))
    } else {
        ([0x1a, 0x1a, 0x1f, 0xff], Some([0x32, 0x32, 0x38, 0xbf]))
    };
    let action = sw::rounded_face(CARD_W as u32, ACTION_H as u32, 10.0, fill, border);
    blit(face, &action, SIDE, y);
    if enabled && switch {
        sw::fill_round(face, SIDE - 8.0, y + 8.0, 3.0, 24.0, 2.0, accent());
    }
    let ink = if enabled && switch {
        sw::ink_selected()
    } else if enabled {
        sw::ink()
    } else {
        [0x66, 0x66, 0x70]
    };
    let key_ink = if enabled { sw::accent() } else { [0x55, 0x55, 0x5d] };
    kb::draw_left(face, key, 13.0, 10.0, 20.0, SIDE + 16.0, y, ACTION_H, key_ink);
    kb::draw_left(
        face,
        label,
        15.0,
        12.0,
        CARD_W - 70.0,
        SIDE + 46.0,
        y,
        ACTION_H,
        ink,
    );
    if !enabled {
        kb::draw_left(
            face,
            slot_i18n::t(lang, "device.quick.sync.locked"),
            11.0,
            9.0,
            38.0,
            SIDE + CARD_W - 70.0,
            y,
            ACTION_H,
            [0x55, 0x55, 0x5d],
        );
    }
}

fn accent() -> [u8; 4] {
    [sw::accent()[0], sw::accent()[1], sw::accent()[2], 0xff]
}

fn accent_ink() -> [u8; 3] {
    [sw::accent()[0], sw::accent()[1], sw::accent()[2]]
}

fn fill(face: &mut UndoFace, x: f32, y: f32, w: f32, h: f32, colour: [u8; 4]) {
    let x0 = x.round().max(0.0) as u32;
    let y0 = y.round().max(0.0) as u32;
    let x1 = (x + w).round().min(face.w as f32) as u32;
    let y1 = (y + h).round().min(face.h as f32) as u32;
    for py in y0..y1 {
        for px in x0..x1 {
            let at = ((py * face.w + px) * 4) as usize;
            face.rgba[at..at + 4].copy_from_slice(&colour);
        }
    }
}

fn blit(dst: &mut UndoFace, src: &UndoFace, x: f32, y: f32) {
    let (x, y) = (x.round() as u32, y.round() as u32);
    for row in 0..src.h {
        for col in 0..src.w {
            let dx = x + col;
            let dy = y + row;
            if dx >= dst.w || dy >= dst.h {
                continue;
            }
            let from = ((row * src.w + col) * 4) as usize;
            let to = ((dy * dst.w + dx) * 4) as usize;
            let sa = src.rgba[from + 3] as f32 / 255.0;
            if sa >= 1.0 {
                dst.rgba[to..to + 4].copy_from_slice(&src.rgba[from..from + 4]);
                continue;
            }
            if sa <= 0.0 {
                continue;
            }
            let da = dst.rgba[to + 3] as f32 / 255.0;
            let out_a = sa + da * (1.0 - sa);
            if out_a <= 0.0 {
                continue;
            }
            for channel in 0..3 {
                let src_c = src.rgba[from + channel] as f32 / 255.0;
                let dst_c = dst.rgba[to + channel] as f32 / 255.0;
                dst.rgba[to + channel] =
                    (((src_c * sa + dst_c * da * (1.0 - sa)) / out_a) * 255.0)
                        .round()
                        .clamp(0.0, 255.0) as u8;
            }
            dst.rgba[to + 3] = (out_a * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    }
}

/// A tiny draw-list helper for callers that want to lay the panel over an
/// existing frame without uploading a texture (currently the frontend uploads
/// the full face, but tests use this to assert the dimensions).
pub fn draw_sync_menu(tex: Option<slot_gfx::TexId>, out: &mut Vec<Draw>) {
    let Some(tex) = tex else {
        return;
    };
    out.push(Draw::Tex {
        x: 0.0,
        y: 0.0,
        w: OUT_W as f32,
        h: OUT_H as f32,
        tex,
        alpha: 1.0,
    });
}
