//! Overlay panels in the shared Nintendo-Switch-inspired style: the save-album
//! / utility panel, the game-title heading and the letter picker.  Only the
//! drawing changed; the pagination and A/B/left-right input logic is untouched.

use slot_i18n::Language;

use crate::draw::OUT_W;
use crate::keyboard as kb;
use crate::plate::UndoFace;
use crate::switch_style as sw;

const SIDE: f32 = 24.0;
const CARD_W: f32 = OUT_W as f32 - 2.0 * SIDE;
const ROW_TOP: f32 = 104.0;
/// One prose card: a title line plus three wrapped lines of detail.
const NOTICE_CARD_H: f32 = 184.0;
const ROW_H: f32 = 40.0;
const ROW_GAP: f32 = 4.0;
const LIST_ROWS: usize = 6;
const EMPTY_CARD_H: f32 = 200.0;
const PAGE_Y: f32 = 392.0;
const HINT_Y: f32 = 424.0;
const HINT_H: f32 = 28.0;

const PICKER_COLS: usize = 7;
const PICKER_CELL_W: f32 = 72.0;
const PICKER_CELL_H: f32 = 56.0;
const PICKER_GAP: f32 = 10.0;
const PICKER_TOP: f32 = 110.0;

/// The transfer-room card's face size.  The frontend draws it at this size, and
/// the app reserves the same band above the shelf.
pub const TRANSFER_HEADER_W: f32 = 664.0;
pub const TRANSFER_HEADER_H: f32 = 76.0;

/// The game-title heading shown at the top of the navigation overlay: white
/// title type on a transparent face.  The shelf reuses this as an optional
/// title strip, so it deliberately carries no menu accent bar.
pub fn title(text: &str) -> UndoFace {
    let mut face = UndoFace {
        rgba: vec![0u8; (664 * 76 * 4) as usize],
        w: 664,
        h: 76,
    };
    kb::draw_box(&mut face, text, 32.0, 16.0, 30.0, 8.0, 610.0, 60.0, sw::ink());
    face
}

const FOLDER_MODE: [u8; 3] = [0xf2, 0xc8, 0x5b];
const HOST_MODE: [u8; 3] = [0x66, 0xe2, 0xba];
const JOIN_MODE: [u8; 3] = [0x8a, 0xc8, 0xff];

fn mode_rgba(mode: [u8; 3]) -> [u8; 4] {
    [mode[0], mode[1], mode[2], 0xff]
}

fn mode_tint(mode: [u8; 3]) -> [u8; 4] {
    let surface = sw::surface();
    [
        ((surface[0] as u16 * 84 + mode[0] as u16 * 16) / 100) as u8,
        ((surface[1] as u16 * 84 + mode[1] as u16 * 16) / 100) as u8,
        ((surface[2] as u16 * 84 + mode[2] as u16 * 16) / 100) as u8,
        0xff,
    ]
}

fn mode_tile(face: &mut UndoFace, x: f32, y: f32, mode: [u8; 3]) {
    sw::fill_round(face, x, y, 30.0, 30.0, 8.0, mode_rgba(mode));
    sw::fill_round(face, x + 1.0, y + 1.0, 28.0, 28.0, 7.0, mode_tint(mode));
}

fn mode_status(
    face: &mut UndoFace,
    card_x: f32,
    card_w: f32,
    label: &str,
    mode: [u8; 3],
    detail: &str,
) {
    let px = 14.0;
    let label_w = kb::ink_width(label, px);
    let detail_w = if detail.is_empty() {
        0.0
    } else {
        kb::ink_width(detail, px)
    };
    let gap = if detail.is_empty() { 0.0 } else { 8.0 };
    let total = 10.0 + 7.0 + label_w + gap + detail_w;
    let x = card_x + card_w - 18.0 - total;
    sw::fill_round(face, x, 17.0, 10.0, 10.0, 5.0, mode_rgba(mode));
    let top = (44.0 - kb::line_height(px)) / 2.0;
    kb::draw_text(face, label, px, x + 17.0, top.max(0.0), mode);
    if !detail.is_empty() {
        kb::draw_text(
            face,
            detail,
            px,
            x + 17.0 + label_w + gap,
            top.max(0.0),
            sw::ink_dim(),
        );
    }
}

fn header_game(face: &mut UndoFace, game: &str, suffix: Option<&str>, mode: [u8; 3]) {
    const TOP_H: f32 = 44.0;
    const ROW_H: f32 = 32.0;
    let (line, px) = kb::fit_line(game, 480.0, 18.0, 13.0);
    let line_w = kb::ink_width(&line, px);
    let suffix_px = 14.0;
    let suffix_w = suffix.map_or(0.0, |text| kb::ink_width(text, suffix_px));
    let gap = if suffix.is_some() { 8.0 } else { 0.0 };
    let total = 10.0 + line_w + gap + suffix_w;
    let x = ((664.0 - total) / 2.0).round();
    let top = TOP_H + ((ROW_H - kb::line_height(px)) / 2.0).max(0.0);
    sw::fill_round(
        face,
        x,
        TOP_H + (ROW_H - 5.0) / 2.0,
        5.0,
        5.0,
        2.5,
        mode_rgba(mode),
    );
    kb::draw_text(face, &line, px, x + 12.0, top.max(TOP_H), sw::ink());
    if let Some(suffix) = suffix {
        let suffix_top = TOP_H + ((ROW_H - kb::line_height(suffix_px)) / 2.0).max(0.0);
        kb::draw_text(
            face,
            suffix,
            suffix_px,
            x + 12.0 + line_w + gap,
            suffix_top.max(TOP_H),
            sw::ink_dim(),
        );
    }
}

fn folder_icon(face: &mut UndoFace, x: f32, y: f32, mode: [u8; 3]) {
    let ink = mode_rgba(mode);
    kb::fill(face, x, y, 7.0, 2.0, ink);
    kb::fill(face, x + 6.0, y + 1.0, 2.0, 3.0, ink);
    kb::fill(face, x, y + 3.0, 16.0, 2.0, ink);
    kb::fill(face, x, y + 3.0, 2.0, 9.0, ink);
    kb::fill(face, x + 14.0, y + 3.0, 2.0, 9.0, ink);
    kb::fill(face, x, y + 10.0, 16.0, 2.0, ink);
}

fn link_icon(face: &mut UndoFace, x: f32, y: f32, mode: [u8; 3]) {
    let ink = mode_rgba(mode);
    kb::fill(face, x + 2.0, y + 5.0, 14.0, 2.0, ink);
    kb::fill(face, x + 10.0, y + 1.0, 2.0, 8.0, ink);
    kb::fill(face, x + 14.0, y + 5.0, 2.0, 2.0, ink);
    kb::fill(face, x + 2.0, y + 13.0, 14.0, 2.0, ink);
    kb::fill(face, x + 8.0, y + 9.0, 2.0, 2.0, ink);
    kb::fill(face, x, y + 13.0, 2.0, 2.0, ink);
}

fn join_icon(face: &mut UndoFace, x: f32, y: f32, mode: [u8; 3]) {
    let ink = mode_rgba(mode);
    kb::fill(face, x, y + 7.0, 15.0, 2.0, ink);
    kb::fill(face, x + 10.0, y + 3.0, 2.0, 2.0, ink);
    kb::fill(face, x + 12.0, y + 5.0, 2.0, 6.0, ink);
    kb::fill(face, x + 10.0, y + 11.0, 2.0, 2.0, ink);
    kb::fill(face, x + 17.0, y + 3.0, 2.0, 10.0, ink);
}

/// A folder-mode header with a mode spine, coloured icon tile and status lamp.
pub fn folder_header(
    folder: &str,
    game: Option<&str>,
    platform: &str,
    index: usize,
    total: usize,
    lang: Language,
) -> UndoFace {
    const W: f32 = 664.0;
    const H: f32 = 76.0;
    const TOP_H: f32 = 44.0;
    let mut face = kb::blank(W as u32, H as u32, [0, 0, 0, 0]);
    sw::fill_round(&mut face, 0.0, 0.0, W, H, 11.0, sw::surface_border());
    sw::fill_round(&mut face, 1.0, 1.0, W - 2.0, H - 2.0, 10.0, sw::surface());
    kb::fill(&mut face, 0.0, 0.0, 3.0, H, mode_rgba(FOLDER_MODE));
    mode_tile(&mut face, 12.0, 7.0, FOLDER_MODE);
    folder_icon(&mut face, 19.0, 15.0, FOLDER_MODE);
    let (title, px) = kb::fit_line(folder, 400.0, 18.0, 13.0);
    kb::draw_text(
        &mut face,
        &title,
        px,
        54.0,
        ((TOP_H - kb::line_height(px)) / 2.0).max(0.0),
        sw::ink(),
    );
    let detail = format!("{platform} · {index}/{total}");
    mode_status(
        &mut face,
        0.0,
        W,
        slot_i18n::t(lang, "device.app.transfer.mode_folder"),
        FOLDER_MODE,
        &detail,
    );
    kb::fill(&mut face, 16.0, TOP_H, W - 32.0, 1.0, sw::surface_border());
    if let Some(game) = game {
        header_game(&mut face, game, None, FOLDER_MODE);
    }
    face
}

/// A transfer-room header; host and join are distinguished by their mode colour
/// and status lamp, while the panel itself stays neutral.
pub fn transfer_header(
    host: bool,
    game: Option<&str>,
    content: usize,
    lang: Language,
) -> UndoFace {
    const W: f32 = TRANSFER_HEADER_W;
    const H: f32 = TRANSFER_HEADER_H;
    const TOP_H: f32 = 44.0;
    let mut face = kb::blank(W as u32, H as u32, [0, 0, 0, 0]);
    let mode = if host { HOST_MODE } else { JOIN_MODE };
    let label = slot_i18n::t(
        lang,
        if host {
            "device.app.transfer.mode_host"
        } else {
            "device.app.transfer.mode_join"
        },
    );
    sw::fill_round(&mut face, 0.0, 0.0, W, H, 11.0, sw::surface_border());
    sw::fill_round(&mut face, 1.0, 1.0, W - 2.0, H - 2.0, 10.0, sw::surface());
    kb::fill(&mut face, 0.0, 0.0, 3.0, H, mode_rgba(mode));
    mode_tile(&mut face, 12.0, 7.0, mode);
    if host {
        link_icon(&mut face, 19.0, 13.0, mode);
    } else {
        join_icon(&mut face, 19.0, 13.0, mode);
    }
    let (title, px) = kb::fit_line(
        slot_i18n::t(lang, "device.app.transfer.title"),
        400.0,
        18.0,
        13.0,
    );
    kb::draw_text(
        &mut face,
        &title,
        px,
        54.0,
        ((TOP_H - kb::line_height(px)) / 2.0).max(0.0),
        sw::ink(),
    );
    mode_status(&mut face, 0.0, W, &label, mode, "");
    let content_key = match content.min(2) {
        1 => "device.app.transfer.content_save",
        2 => "device.app.transfer.content_all",
        _ => "device.app.transfer.content_cart",
    };
    let content_label = slot_i18n::t(lang, content_key);
    kb::fill(&mut face, 16.0, TOP_H, W - 32.0, 1.0, sw::surface_border());
    if let Some(game) = game {
        let suffix = format!("· {content_label}");
        header_game(&mut face, game, Some(&suffix), mode);
    }
    face
}

/// The small "hold SELECT" prompt.  Only the card and its track are baked in;
/// the frontend draws the fill from [`folder_hold_bar`] every frame, so the bar
/// moves without re-uploading a texture.
pub fn folder_hold_prompt(lang: Language) -> UndoFace {
    let mut face = kb::blank(FOLDER_HOLD_W, FOLDER_HOLD_H, [0, 0, 0, 0]);
    sw::fill_round(
        &mut face,
        0.0,
        0.0,
        FOLDER_HOLD_W as f32,
        FOLDER_HOLD_H as f32,
        12.0,
        sw::surface_border(),
    );
    sw::fill_round(
        &mut face,
        1.0,
        1.0,
        FOLDER_HOLD_W as f32 - 2.0,
        FOLDER_HOLD_H as f32 - 2.0,
        11.0,
        sw::surface(),
    );
    kb::fill(
        &mut face,
        0.0,
        0.0,
        3.0,
        FOLDER_HOLD_H as f32,
        mode_rgba(FOLDER_MODE),
    );
    mode_tile(&mut face, 12.0, 9.0, FOLDER_MODE);
    folder_icon(&mut face, 19.0, 17.0, FOLDER_MODE);
    let (label, px) = kb::fit_line(
        slot_i18n::t(lang, "device.app.folder.hold"),
        218.0,
        12.0,
        9.0,
    );
    kb::draw_text(&mut face, &label, px, 48.0, 17.0, sw::ink());
    let [bx, by, bw, bh] = folder_hold_bar();
    sw::fill_round(
        &mut face,
        bx,
        by,
        bw,
        bh,
        bh / 2.0,
        [0x3a, 0x3a, 0x42, 0xff],
    );
    face
}

pub const FOLDER_HOLD_W: u32 = 288;
pub const FOLDER_HOLD_H: u32 = 64;
/// Where the hold prompt sits on the output screen: `[x, y]`.
pub const FOLDER_HOLD_ORIGIN: [f32; 2] = [216.0, 120.0];

/// The fill area of the hold prompt's bar, relative to the prompt face.
pub fn folder_hold_bar() -> [f32; 4] {
    [18.0, 52.0, 252.0, 4.0]
}

/// The progress bar's fill colour.
pub fn folder_hold_fill() -> [f32; 4] {
    [
        FOLDER_MODE[0] as f32 / 255.0,
        FOLDER_MODE[1] as f32 / 255.0,
        FOLDER_MODE[2] as f32 / 255.0,
        1.0,
    ]
}


/// The letter picker: a 7×4 grid of rounded cells.  Available letters are light
/// on charcoal, unavailable ones are dimmer and the selection is a white cell
/// with a red rim and dark type.
pub fn picker(index: u8, available: [bool; 26], lang: Language) -> UndoFace {
    let mut face = kb::blank(OUT_W, 480, sw::bg());
    heading(&mut face, slot_i18n::t(lang, "device.quick.nav.picker_title"));
    let index = (index as usize).min(available.len().saturating_sub(1));
    let grid_w = PICKER_COLS as f32 * PICKER_CELL_W + (PICKER_COLS - 1) as f32 * PICKER_GAP;
    let left = ((OUT_W as f32 - grid_w) / 2.0).round();
    for (i, &avail) in available.iter().enumerate() {
        let col = i % PICKER_COLS;
        let row = i / PICKER_COLS;
        let x = left + col as f32 * (PICKER_CELL_W + PICKER_GAP);
        let y = PICKER_TOP + row as f32 * (PICKER_CELL_H + PICKER_GAP);
        let lit = i == index;
        if lit {
            sw::fill_round(
                &mut face,
                x,
                y,
                PICKER_CELL_W,
                PICKER_CELL_H,
                9.0,
                sw::selected(),
            );
            sw::fill_round(&mut face, x - 8.0, y + (PICKER_CELL_H - 24.0) / 2.0, 3.0, 24.0, 2.0, accent());
        } else if !avail {
            sw::fill_round(
                &mut face,
                x,
                y,
                PICKER_CELL_W,
                PICKER_CELL_H,
                9.0,
                [0x24, 0x24, 0x29, 0xff],
            );
        }
        let ink = if lit {
            sw::ink_selected()
        } else if avail {
            sw::ink()
        } else {
            [0x5a, 0x5a, 0x62]
        };
        let label = char::from(b'A' + i as u8).to_string();
        kb::draw_box(
            &mut face,
            &label,
            30.0,
            16.0,
            x,
            y,
            PICKER_CELL_W,
            PICKER_CELL_H,
            ink,
        );
    }
    kb::draw_box(
        &mut face,
        if available[index] {
            slot_i18n::t(lang, "device.quick.nav.picker_hint")
        } else {
            slot_i18n::t(lang, "device.quick.nav.picker_empty")
        },
        18.0,
        12.0,
        SIDE,
        HINT_Y,
        CARD_W,
        HINT_H,
        sw::ink_dim(),
    );
    face
}

/// A notice: the same heading and card family as [`panel`], but nothing is
/// selectable and there is one thing to press.
///
/// Use this wherever a dialog would ask the player to "confirm" something whose
/// every key already did the same thing — a menu there is just a keypress tax.
pub fn notice(title: &str, lines: &[String], lang: Language) -> UndoFace {
    let mut face = kb::blank(OUT_W, 480, sw::bg());
    heading(&mut face, title);
    let height = NOTICE_CARD_H.min(ROW_TOP + 200.0);
    card_at(&mut face, SIDE, ROW_TOP, CARD_W, height, false);
    let mut y = ROW_TOP + 18.0;
    for (i, line) in lines.iter().enumerate() {
        let ink = if i == 0 { sw::ink() } else { sw::ink_dim() };
        kb::draw_wrapped(
            &mut face,
            line,
            20.0,
            12.0,
            SIDE + 28.0,
            y,
            CARD_W - 56.0,
            34.0,
            2,
            ink,
        );
        y += 40.0;
    }
    kb::draw_box(
        &mut face,
        slot_i18n::t(lang, "device.quick.nav.notice_hint"),
        16.0,
        11.0,
        SIDE,
        HINT_Y,
        CARD_W,
        HINT_H,
        sw::ink_dim(),
    );
    face
}

/// The save-album / utility panel: a heading band, a rounded list area with one
/// white card for the selected row and the page / key hints at the bottom.
/// Six rows per page by default; the in-game menu asks for seven so it fits on
/// one screen.
/// Hotspot band picker: two large cards, with 5 GHz greyed out when the radio
/// cannot use it.  Kept beside the transfer menu because both are small
/// purpose-built overlays rather than generic lists.
pub fn band_menu(selected: usize, five_supported: bool, lang: Language) -> UndoFace {
    let mut face = kb::blank(OUT_W, 480, sw::bg());
    heading(&mut face, slot_i18n::t(lang, "device.app.link_band.title"));
    let ink = sw::ink();
    kb::draw_text(
        &mut face,
        slot_i18n::t(lang, "device.app.link_band.hint"),
        13.0,
        48.0,
        72.0,
        [sw::ink_dim()[0], sw::ink_dim()[1], sw::ink_dim()[2]],
    );

    let cards = [
        (
            slot_i18n::t(lang, "device.app.link_band.five"),
            slot_i18n::t(lang, "device.app.link_band.five_hint"),
            five_supported,
        ),
        (
            slot_i18n::t(lang, "device.app.link_band.two"),
            slot_i18n::t(lang, "device.app.link_band.two_hint"),
            true,
        ),
    ];
    for (i, (title, hint, enabled)) in cards.iter().enumerate() {
        let y = 112.0 + i as f32 * 116.0;
        let chosen = i == selected.min(1) && *enabled;
        if chosen {
            card_at(&mut face, 48.0, y, 624.0, 100.0, true);
        } else if *enabled {
            card_at(&mut face, 48.0, y, 624.0, 100.0, false);
        } else {
            sw::fill_round(&mut face, 48.0, y, 624.0, 100.0, 10.0, [0x1c, 0x1d, 0x26, 0xff]);
        }
        let mark = if *enabled { if chosen { "●" } else { "○" } } else { "×" };
        let mark_ink = if !*enabled {
            sw::ink_dim()
        } else if chosen {
            sw::ink_selected()
        } else {
            ink
        };
        kb::draw_text(
            &mut face,
            mark,
            22.0,
            70.0,
            y + 30.0,
            [mark_ink[0], mark_ink[1], mark_ink[2]],
        );
        let title_ink = if !*enabled {
            sw::ink_dim()
        } else if chosen {
            sw::ink_selected()
        } else {
            ink
        };
        kb::draw_text(
            &mut face,
            title,
            20.0,
            108.0,
            y + 24.0,
            [title_ink[0], title_ink[1], title_ink[2]],
        );
        let hint = if !*enabled {
            slot_i18n::t(lang, "device.app.link_band.unsupported")
        } else {
            hint
        };
        let hint_ink = if chosen { sw::ink_selected() } else { sw::ink_dim() };
        kb::draw_text(
            &mut face,
            hint,
            11.0,
            108.0,
            y + 58.0,
            [hint_ink[0], hint_ink[1], hint_ink[2]],
        );
    }

    let hint = [
        ("device.app.transfer.hint_nav", 48.0),
        ("device.app.transfer.hint_confirm", 180.0),
        ("device.app.transfer.hint_back", 330.0),
    ];
    for (key, x) in hint {
        kb::draw_text(
            &mut face,
            slot_i18n::t(lang, key),
            12.0,
            x,
            HINT_Y + 8.0,
            [sw::ink_dim()[0], sw::ink_dim()[1], sw::ink_dim()[2]],
        );
    }
    face
}


fn back_icon(face: &mut UndoFace, x: f32, y: f32, colour: [u8; 4]) {
    kb::fill(face, x, y + 7.0, 14.0, 2.0, colour);
    kb::fill(face, x + 2.0, y + 3.0, 2.0, 2.0, colour);
    kb::fill(face, x, y + 5.0, 2.0, 6.0, colour);
    kb::fill(face, x + 2.0, y + 11.0, 2.0, 2.0, colour);
}

fn clear_rect(face: &mut UndoFace, x: f32, y: f32, w: f32, h: f32) {
    let x0 = x.max(0.0) as u32;
    let y0 = y.max(0.0) as u32;
    let x1 = (x + w).min(face.w as f32) as u32;
    let y1 = (y + h).min(face.h as f32) as u32;
    for py in y0..y1 {
        for px in x0..x1 {
            let at = ((py * face.w + px) * 4) as usize;
            face.rgba[at..at + 4].copy_from_slice(&[0, 0, 0, 0]);
        }
    }
}

/// Cartridge-transfer matching menu: a two-device bridge on the left and the
/// three choices on the right. Host, join and idle each get a mode colour and
/// status lamp, but the panels stay neutral Console Flat surfaces.
pub fn transfer_menu(mode: &str, selected: usize, lang: Language) -> UndoFace {
    let mut face = kb::blank(OUT_W, 480, sw::bg());
    let selected = selected.min(2);
    let (mode_colour, status_key) = match selected {
        0 => (HOST_MODE, "device.app.transfer.create_hint"),
        1 => (JOIN_MODE, "device.app.transfer.join_hint"),
        _ => ([0xa6, 0xa6, 0xad], "device.app.transfer.back_hint"),
    };
    heading(&mut face, slot_i18n::t(lang, "device.app.transfer.title"));
    let (network, network_px) = kb::fit_line(mode, 170.0, 14.0, 12.0);
    let network_w = kb::ink_width(&network, network_px);
    let chip_w = network_w + 40.0;
    let chip_x = OUT_W as f32 - 32.0 - chip_w;
    sw::fill_round(&mut face, chip_x, 50.0, chip_w, 30.0, 8.0, sw::surface_border());
    sw::fill_round(
        &mut face,
        chip_x + 1.0,
        51.0,
        chip_w - 2.0,
        28.0,
        7.0,
        [0x20, 0x20, 0x24, 0xff],
    );
    sw::fill_round(
        &mut face,
        chip_x + 11.0,
        61.0,
        8.0,
        8.0,
        4.0,
        mode_rgba(mode_colour),
    );
    let network_top = 50.0 + (30.0 - kb::line_height(network_px)) / 2.0;
    kb::draw_text(
        &mut face,
        &network,
        network_px,
        chip_x + 26.0,
        network_top.max(50.0),
        sw::ink(),
    );

    // Left live-animation viewport.  The interior is transparent so the
    // frontend can draw the existing link animation underneath this face.
    sw::fill_round(&mut face, 24.0, 96.0, 340.0, 300.0, 12.0, sw::surface_border());
    sw::fill_round(&mut face, 25.0, 97.0, 338.0, 298.0, 11.0, sw::surface());
    let hero_key = match selected {
        0 => "device.app.transfer.create",
        1 => "device.app.transfer.join",
        _ => "device.app.common.back",
    };
    kb::draw_text(&mut face, "LIVE LINK / VIEWPORT", 9.0, 42.0, 112.0, sw::ink_dim());
    kb::draw_text(
        &mut face,
        slot_i18n::t(lang, hero_key),
        20.0,
        42.0,
        126.0,
        sw::ink(),
    );
    clear_rect(&mut face, 40.0, 164.0, 308.0, 194.0);
    let frame = sw::surface_border();
    kb::fill(&mut face, 40.0, 164.0, 308.0, 1.0, frame);
    kb::fill(&mut face, 40.0, 357.0, 308.0, 1.0, frame);
    kb::fill(&mut face, 40.0, 164.0, 1.0, 194.0, frame);
    kb::fill(&mut face, 347.0, 164.0, 1.0, 194.0, frame);
    sw::fill_round(
        &mut face,
        42.0,
        370.0,
        7.0,
        7.0,
        3.5,
        mode_rgba(mode_colour),
    );
    let (status, px) = kb::fit_line(slot_i18n::t(lang, status_key), 292.0, 11.0, 10.0);
    kb::draw_text(&mut face, &status, px, 57.0, 370.0, sw::ink_dim());

    // Right mode panel.
    sw::fill_round(&mut face, 380.0, 96.0, 316.0, 300.0, 12.0, sw::surface_border());
    sw::fill_round(&mut face, 381.0, 97.0, 314.0, 298.0, 11.0, sw::surface());
    kb::draw_text(&mut face, "MODE", 9.0, 400.0, 112.0, sw::ink_dim());
    kb::draw_text(
        &mut face,
        slot_i18n::t(lang, "device.app.transfer.mode_title"),
        20.0,
        400.0,
        126.0,
        sw::ink(),
    );

    let rows = [
        ("device.app.transfer.create", "device.app.transfer.create_hint"),
        ("device.app.transfer.join", "device.app.transfer.join_hint"),
        ("device.app.common.back", "device.app.transfer.back_hint"),
    ];
    for (index, (title_key, hint_key)) in rows.iter().enumerate() {
        let y = 174.0 + index as f32 * 54.0;
        let lit = index == selected;
        let row_ink = if lit { sw::ink_selected() } else { sw::ink() };
        let hint_ink = if lit { sw::ink_selected() } else { sw::ink_dim() };
        if lit {
            sw::fill_round(&mut face, 400.0, y, 276.0, 48.0, 11.0, sw::selected());
            sw::fill_round(
                &mut face,
                392.0,
                y + 12.0,
                3.0,
                24.0,
                2.0,
                accent(),
            );
        }
        let icon_colour = if lit {
            let ink = sw::ink_selected();
            [ink[0], ink[1], ink[2], 0xff]
        } else {
            mode_rgba(mode_colour)
        };
        match index {
            0 => {
                kb::fill(&mut face, 416.0, y + 15.0, 14.0, 2.0, icon_colour);
                kb::fill(&mut face, 428.0, y + 11.0, 2.0, 6.0, icon_colour);
                kb::fill(&mut face, 428.0, y + 22.0, 2.0, 2.0, icon_colour);
            }
            1 => {
                kb::fill(&mut face, 416.0, y + 22.0, 14.0, 2.0, icon_colour);
                kb::fill(&mut face, 426.0, y + 18.0, 2.0, 2.0, icon_colour);
                kb::fill(&mut face, 428.0, y + 20.0, 2.0, 6.0, icon_colour);
                kb::fill(&mut face, 426.0, y + 26.0, 2.0, 2.0, icon_colour);
            }
            _ => back_icon(&mut face, 416.0, y + 15.0, icon_colour),
        }
        kb::draw_text(
            &mut face,
            slot_i18n::t(lang, title_key),
            16.0,
            440.0,
            y + 5.0,
            row_ink,
        );
        kb::draw_text(
            &mut face,
            slot_i18n::t(lang, hint_key),
            10.0,
            440.0,
            y + 29.0,
            hint_ink,
        );
        if lit {
            let rights = match index {
                0 => slot_i18n::t(lang, "device.app.transfer.mode_host"),
                1 => slot_i18n::t(lang, "device.app.transfer.mode_join"),
                _ => "B",
            };
            kb::draw_text(&mut face, rights, 9.0, 632.0, y + 19.0, [0x55, 0x55, 0x5d]);
        }
    }
    let dim = sw::ink_dim();
    kb::draw_text(
        &mut face,
        slot_i18n::t(lang, "device.app.transfer.hint_back"),
        10.0,
        32.0,
        436.0,
        dim,
    );
    kb::draw_text(
        &mut face,
        slot_i18n::t(lang, "device.app.transfer.select_mode"),
        10.0,
        300.0,
        436.0,
        dim,
    );
    kb::draw_text(
        &mut face,
        slot_i18n::t(lang, "device.app.transfer.hint_confirm"),
        10.0,
        600.0,
        436.0,
        dim,
    );
    face
}

/// Cartridge-transfer content chooser: three flat rows with content icons.
/// `selected` is the cursor, `current` the value already in force; both are
/// 0 = cart only, 1 = saves only, 2 = cart and saves.
pub fn transfer_content_menu(selected: usize, current: usize, lang: Language) -> UndoFace {
    const ROW_X: f32 = 24.0;
    const ROW_W: f32 = 672.0;
    const ROW_H: f32 = 64.0;
    const ROW_GAP: f32 = 8.0;
    const ROW_TOP: f32 = 96.0;

    let mut face = kb::blank(OUT_W, 480, sw::bg());
    heading(&mut face, slot_i18n::t(lang, "device.app.transfer.content"));
    let dim = sw::ink_dim();
    kb::draw_text(
        &mut face,
        slot_i18n::t(lang, "device.app.transfer.content_hint"),
        11.0,
        32.0,
        82.0,
        dim,
    );

    let rows = [
        (
            "device.app.transfer.content_cart",
            "device.app.transfer.content_cart_hint",
        ),
        (
            "device.app.transfer.content_save",
            "device.app.transfer.content_save_hint",
        ),
        (
            "device.app.transfer.content_all",
            "device.app.transfer.content_all_hint",
        ),
    ];
    let current = current.min(2);
    for (index, (title_key, hint_key)) in rows.iter().enumerate() {
        let y = ROW_TOP + index as f32 * (ROW_H + ROW_GAP);
        let lit = index == selected.min(2);
        if lit {
            sw::fill_round(&mut face, ROW_X, y, ROW_W, ROW_H, 11.0, sw::selected());
            sw::fill_round(&mut face, ROW_X - 8.0, y + 20.0, 3.0, 24.0, 2.0, accent());
        }
        let title_ink = if lit { sw::ink_selected() } else { sw::ink() };
        let hint_ink = if lit { sw::ink_selected() } else { sw::ink_dim() };
        let icon_ink = if lit {
            sw::ink_selected()
        } else {
            sw::ink()
        };
        let icon_x = ROW_X + 16.0;
        let icon_y = y + 20.0;
        match index {
            0 => {
                // Cartridge
                kb::fill(&mut face, icon_x + 4.0, icon_y, 9.0, 2.0, [icon_ink[0], icon_ink[1], icon_ink[2], 0xff]);
                kb::fill(&mut face, icon_x + 2.0, icon_y + 2.0, 2.0, 12.0, [icon_ink[0], icon_ink[1], icon_ink[2], 0xff]);
                kb::fill(&mut face, icon_x + 13.0, icon_y + 2.0, 2.0, 12.0, [icon_ink[0], icon_ink[1], icon_ink[2], 0xff]);
                kb::fill(&mut face, icon_x + 4.0, icon_y + 12.0, 9.0, 2.0, [icon_ink[0], icon_ink[1], icon_ink[2], 0xff]);
            }
            1 => {
                // Save slot
                kb::fill(&mut face, icon_x + 3.0, icon_y, 10.0, 2.0, [icon_ink[0], icon_ink[1], icon_ink[2], 0xff]);
                kb::fill(&mut face, icon_x + 2.0, icon_y + 2.0, 2.0, 12.0, [icon_ink[0], icon_ink[1], icon_ink[2], 0xff]);
                kb::fill(&mut face, icon_x + 12.0, icon_y + 2.0, 2.0, 12.0, [icon_ink[0], icon_ink[1], icon_ink[2], 0xff]);
                kb::fill(&mut face, icon_x + 3.0, icon_y + 12.0, 10.0, 2.0, [icon_ink[0], icon_ink[1], icon_ink[2], 0xff]);
                kb::fill(&mut face, icon_x + 4.0, icon_y + 3.0, 6.0, 2.0, [icon_ink[0], icon_ink[1], icon_ink[2], 0xff]);
            }
            _ => {
                // Cartridge + save stack
                kb::fill(&mut face, icon_x + 2.0, icon_y, 12.0, 2.0, [icon_ink[0], icon_ink[1], icon_ink[2], 0xff]);
                kb::fill(&mut face, icon_x, icon_y + 2.0, 2.0, 6.0, [icon_ink[0], icon_ink[1], icon_ink[2], 0xff]);
                kb::fill(&mut face, icon_x + 14.0, icon_y + 2.0, 2.0, 6.0, [icon_ink[0], icon_ink[1], icon_ink[2], 0xff]);
                kb::fill(&mut face, icon_x + 2.0, icon_y + 8.0, 12.0, 2.0, [icon_ink[0], icon_ink[1], icon_ink[2], 0xff]);
                kb::fill(&mut face, icon_x + 2.0, icon_y + 12.0, 12.0, 2.0, [icon_ink[0], icon_ink[1], icon_ink[2], 0xff]);
            }
        }
        let (title, title_px) = kb::fit_line(
            slot_i18n::t(lang, title_key),
            470.0,
            18.0,
            13.0,
        );
        kb::draw_text(&mut face, &title, title_px, ROW_X + 52.0, y + 5.0, title_ink);
        let (hint, hint_px) = kb::fit_line(
            slot_i18n::t(lang, hint_key),
            470.0,
            12.0,
            10.0,
        );
        kb::draw_text(&mut face, &hint, hint_px, ROW_X + 52.0, y + 34.0, hint_ink);
        if index == current {
            let label = slot_i18n::t(lang, "device.app.transfer.content_current");
            let tag_px = 10.0;
            let tag_w = kb::ink_width(label, tag_px) + 14.0;
            let tag_x = ROW_X + ROW_W - 16.0 - tag_w;
            let border = if lit { [0xd0, 0xd0, 0xd2, 0xff] } else { sw::surface_border() };
            sw::fill_round(&mut face, tag_x, y + 22.0, tag_w, 20.0, 6.0, border);
            let fill = if lit { sw::selected() } else { sw::bg() };
            sw::fill_round(&mut face, tag_x + 1.0, y + 23.0, tag_w - 2.0, 18.0, 5.0, fill);
            kb::draw_text(
                &mut face,
                label,
                tag_px,
                tag_x + 7.0,
                y + 27.0,
                if lit { [0x55, 0x55, 0x5d] } else { sw::ink_dim() },
            );
        }
    }

    let hints = [
        ("device.app.transfer.hint_nav", 32.0),
        ("device.app.transfer.hint_confirm", 148.0),
        ("device.app.transfer.hint_back", 248.0),
        ("device.app.transfer.content_close", 328.0),
    ];
    for (key, x) in hints {
        kb::draw_text(
            &mut face,
            slot_i18n::t(lang, key),
            10.0,
            x,
            HINT_Y + 6.0,
            sw::ink_dim(),
        );
    }
    face
}

pub fn panel(title: &str, rows: &[String], selected: usize, lang: Language) -> UndoFace {
    panel_with_limit(title, rows, selected, LIST_ROWS, lang)
}

/// [`panel`] with an explicit page size.  Seven rows use a slightly shorter
/// row pitch so the last row still clears the page label and the key hints.
pub fn panel_with_limit(
    title: &str,
    rows: &[String],
    selected: usize,
    rows_per_page: usize,
    lang: Language,
) -> UndoFace {
    let mut face = kb::blank(OUT_W, 480, sw::bg());
    heading(&mut face, title);
    if rows.is_empty() {
        card_at(&mut face, SIDE, ROW_TOP, CARD_W, EMPTY_CARD_H, false);
        kb::draw_box(
            &mut face,
            slot_i18n::t(lang, "device.quick.nav.empty_states"),
            24.0,
            14.0,
            SIDE,
            ROW_TOP,
            CARD_W,
            EMPTY_CARD_H,
            sw::ink_dim(),
        );
        kb::draw_box(
            &mut face,
            slot_i18n::t(lang, "device.quick.nav.panel_back"),
            18.0,
            12.0,
            SIDE,
            HINT_Y,
            CARD_W,
            HINT_H,
            sw::ink_dim(),
        );
        return face;
    }
    let rows_per_page = rows_per_page.max(1);
    let compact = rows_per_page > LIST_ROWS;
    let (row_h, row_gap) = if compact {
        (34.0, 4.0)
    } else {
        (ROW_H, ROW_GAP)
    };
    let start = selected / rows_per_page * rows_per_page;
    for (i, text) in rows.iter().enumerate().skip(start).take(rows_per_page) {
        let row = i - start;
        let y = ROW_TOP + row as f32 * (row_h + row_gap);
        let lit = i == selected;
        if lit {
            card_at(&mut face, SIDE + 12.0, y, CARD_W - 24.0, row_h - 4.0, true);
        }
        // A row whose label says it is unavailable (SFC's link row today) is
        // drawn dim even though it can still be focused, so the player sees the
        // state before pressing it.
        // The suffix is the row's own marker, so it follows the language too.
        // WP2 supplies the English marker; until then an English row simply
        // keeps the normal ink, which is a shade, not a bug.
        let disabled = text.ends_with("（暂不支持）") || text.ends_with("(not supported)");
        let ink = if lit && !disabled {
            sw::ink_selected()
        } else if disabled {
            sw::ink_dim()
        } else {
            sw::ink()
        };
        kb::draw_left(
            &mut face,
            text,
            20.0,
            14.0,
            CARD_W - 72.0,
            SIDE + 36.0,
            y,
            row_h,
            ink,
        );
    }
    let pages = rows.len().max(1).div_ceil(rows_per_page);
    let page_label = if pages <= 1 {
        slot_i18n::t_args(
            lang,
            "device.quick.nav.panel_count",
            &[("count", &rows.len().to_string())],
        )
    } else {
        slot_i18n::t_args(
            lang,
            "device.quick.nav.panel_page",
            &[
                ("page", &(start / rows_per_page + 1).to_string()),
                ("pages", &pages.to_string()),
            ],
        )
    };
    kb::draw_box(
        &mut face,
        &page_label,
        16.0,
        11.0,
        SIDE,
        PAGE_Y,
        CARD_W,
        24.0,
        sw::ink_dim(),
    );
    kb::draw_box(
        &mut face,
        slot_i18n::t(lang, "device.quick.nav.panel_hint"),
        16.0,
        11.0,
        SIDE,
        HINT_Y,
        CARD_W,
        HINT_H,
        sw::ink_dim(),
    );
    face
}

/// One achievement row for the icon list. `badge` is a 40×40 RGBA image
/// already scaled by the caller; `None` falls back to a neutral tile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BadgeRow {
    pub title: String,
    pub subtitle: String,
    pub points: String,
    pub unlocked: bool,
    pub badge: Option<Vec<u8>>,
    /// Measured progress (value, target) when the definition has one.
    pub progress: Option<(u32, u32)>,
}

/// Achievement list: five larger rows with a badge, title, one-line
/// description and points. The page keeps the existing heading/footer style.
pub fn panel_badged(
    title: &str,
    game: &str,
    summary: &str,
    rows: &[BadgeRow],
    selected: usize,
    lang: Language,
) -> UndoFace {
    const ICON_TOP: f32 = 96.0;
    const ICON_H: f32 = 44.0;
    const ICON_GAP: f32 = 4.0;
    const ICON_ROWS: usize = 5;
    let mut face = kb::blank(OUT_W, 480, sw::bg());
    heading(&mut face, title);
    kb::draw_left(
        &mut face,
        game,
        13.0,
        10.0,
        CARD_W - 190.0,
        SIDE,
        57.0,
        17.0,
        sw::ink_dim(),
    );
    kb::draw_left(
        &mut face,
        summary,
        13.0,
        11.0,
        180.0,
        SIDE + CARD_W - 180.0,
        58.0,
        17.0,
        sw::ink_dim(),
    );
    let done = rows.iter().filter(|row| row.unlocked).count();
    let fraction = if rows.is_empty() { 0.0 } else {
        done as f32 / rows.len() as f32
    };
    let progress = [34u8, 160, 135, 255];
    let check_ink = [32u8, 150, 120];
    sw::fill_round(&mut face, SIDE, 80.0, CARD_W, 6.0, 3.0, sw::surface());
    if fraction > 0.0 {
        sw::fill_round(
            &mut face,
            SIDE,
            80.0,
            (CARD_W * fraction).max(6.0),
            6.0,
            3.0,
            progress,
        );
    }
    if rows.is_empty() {
        kb::draw_box(
            &mut face,
            slot_i18n::t(lang, "device.quick.nav.empty_states"),
            24.0,
            14.0,
            SIDE,
            ICON_TOP,
            CARD_W,
            EMPTY_CARD_H,
            sw::ink_dim(),
        );
    } else {
        let selected = selected.min(rows.len().saturating_sub(1));
        let start = selected / ICON_ROWS * ICON_ROWS;
        for (i, row) in rows.iter().enumerate().skip(start).take(ICON_ROWS) {
            let at = i - start;
            let y = ICON_TOP + at as f32 * (ICON_H + ICON_GAP);
            let lit = i == selected;
            if lit {
                card_at(&mut face, SIDE + 12.0, y, CARD_W - 24.0, ICON_H - 2.0, true);
            }
            let ink = if lit {
                sw::ink_selected()
            } else if row.unlocked {
                sw::ink()
            } else {
                sw::ink_dim()
            };
            let sub_ink = if lit { sw::ink_selected() } else { sw::ink_dim() };
            let icon_x = SIDE + 24.0;
            let icon_y = y + 2.0;
            if let Some(rgba) = row.badge.as_deref().filter(|rgba| rgba.len() == 40 * 40 * 4) {
                blit_rgba(&mut face, rgba, 40, 40, icon_x, icon_y, if row.unlocked { 1.0 } else { 0.38 });
            } else {
                sw::fill_round(
                    &mut face,
                    icon_x,
                    icon_y,
                    40.0,
                    40.0,
                    4.0,
                    if lit { sw::selected_border() } else { sw::surface_border() },
                );
            }
            let state = if row.unlocked { "√" } else { "○" };
            kb::draw_box(
                &mut face,
                state,
                15.0,
                10.0,
                icon_x + 44.0,
                y + 5.0,
                20.0,
                20.0,
                if lit && row.unlocked {
                    sw::ink_selected()
                } else if row.unlocked {
                    check_ink
                } else {
                    sw::ink_dim()
                },
            );
            kb::draw_left(
                &mut face,
                &row.title,
                16.0,
                11.0,
                CARD_W - 230.0,
                icon_x + 70.0,
                y + 3.0,
                20.0,
                ink,
            );
            kb::draw_left(
                &mut face,
                &row.subtitle,
                12.0,
                10.0,
                CARD_W - 230.0,
                icon_x + 70.0,
                y + 25.0,
                15.0,
                sub_ink,
            );
            kb::draw_box(
                &mut face,
                &row.points,
                14.0,
                11.0,
                SIDE + CARD_W - 70.0,
                y + 8.0,
                54.0,
                22.0,
                if row.unlocked { check_ink } else { sub_ink },
            );
            if let Some((measured, target)) = row.progress {
                if target > 0 {
                    let text = format!("{measured}/{target}");
                    kb::draw_left(
                        &mut face,
                        &text,
                        10.0,
                        8.0,
                        110.0,
                        SIDE + CARD_W - 140.0,
                        y + 27.0,
                        14.0,
                        sub_ink,
                    );
                }
            }
        }
    }
    let pages = rows.len().max(1).div_ceil(ICON_ROWS);
    let page_label = if pages <= 1 {
        slot_i18n::t_args(
            lang,
            "device.quick.nav.panel_count",
            &[("count", &rows.len().to_string())],
        )
    } else {
        slot_i18n::t_args(
            lang,
            "device.quick.nav.panel_page",
            &[
                ("page", &(selected / ICON_ROWS + 1).to_string()),
                ("pages", &pages.to_string()),
            ],
        )
    };
    kb::draw_box(&mut face, &page_label, 16.0, 11.0, SIDE, PAGE_Y, CARD_W, 24.0, sw::ink_dim());
    kb::draw_box(
        &mut face,
        slot_i18n::t(lang, "device.quick.nav.achievement_hint"),
        16.0,
        11.0,
        SIDE,
        HINT_Y,
        CARD_W,
        HINT_H,
        sw::ink_dim(),
    );
    face
}

fn blit_rgba(face: &mut UndoFace, src: &[u8], w: u32, h: u32, x: f32, y: f32, alpha: f32) {
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
            let a = (src[si + 3] as f32 / 255.0) * alpha;
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

/// Where the hold-to-leave prompt sits, in output pixels: `[x, y, w, h]`.
///
/// Low on the screen on purpose: it replaces the panel's footer hint instead of
/// covering a menu row, so the list the player is looking at stays readable
/// while the bar fills.
const HOLD_CARD: [f32; 4] = [140.0, 392.0, 440.0, 76.0];

/// The fill area of the prompt's bar, in output pixels: `[x, y, w, h]`.
pub const HOLD_BAR: [f32; 4] = [164.0, 450.0, 392.0, 10.0];

/// The colour the bar fills with.
pub fn hold_fill() -> [f32; 4] {
    let accent = sw::accent();
    [
        accent[0] as f32 / 255.0,
        accent[1] as f32 / 255.0,
        accent[2] as f32 / 255.0,
        1.0,
    ]
}

/// The "keep holding MENU to leave" prompt.
///
/// A full-screen face that is transparent everywhere except one card: the game
/// menu stays readable underneath. The bar is *not* baked in — the caller draws
/// it with [`HOLD_BAR`] and the progress it has — so this face is built once
/// and never rebuilt, and the bar moves at frame rate for free.
pub fn hold_card(lang: Language) -> UndoFace {
    let mut face = kb::blank(OUT_W, 480, [0, 0, 0, 0]);
    let [x, y, w, h] = HOLD_CARD;
    sw::fill_round(&mut face, x, y, w, h, 16.0, sw::surface());
    // An accent spine on the left, the way the page headings carry one: this
    // card is an action in progress, not another list.
    kb::fill(
        &mut face,
        x + 4.0,
        y + 14.0,
        4.0,
        h - 28.0,
        [sw::accent()[0], sw::accent()[1], sw::accent()[2], 0xff],
    );
    kb::draw_box(
        &mut face,
        slot_i18n::t(lang, "device.quick.nav.hold_title"),
        19.0,
        13.0,
        x + 24.0,
        y + 12.0,
        w - 48.0,
        24.0,
        sw::ink(),
    );
    kb::draw_box(
        &mut face,
        slot_i18n::t(lang, "device.quick.nav.hold_hint"),
        12.0,
        9.0,
        x + 24.0,
        y + 36.0,
        w - 48.0,
        18.0,
        sw::ink_dim(),
    );
    let [bx, by, bw, bh] = HOLD_BAR;
    sw::fill_round(&mut face, bx, by, bw, bh, bh / 2.0, sw::surface_border());
    face
}

/// The Switch heading band: charcoal ground, red accent bar and white title.
fn heading(face: &mut UndoFace, title: &str) {
    kb::fill(face, 28.0, 40.0, OUT_W as f32 - 56.0, 1.0, sw::surface_border());
    kb::draw_left(
        face,
        title,
        26.0,
        15.0,
        OUT_W as f32 - 64.0,
        32.0,
        48.0,
        34.0,
        sw::ink(),
    );
}

fn accent() -> [u8; 4] {
    [sw::accent()[0], sw::accent()[1], sw::accent()[2], 0xff]
}

/// A Switch-style rounded card: white with a white rim when selected, charcoal
/// surface with a light rim otherwise.
fn card_at(face: &mut UndoFace, x: f32, y: f32, w: f32, h: f32, selected: bool) {
    if selected {
        sw::fill_round(face, x, y, w, h, 11.0, sw::selected());
        sw::fill_round(face, x - 8.0, y + (h - 24.0) / 2.0, 3.0, 24.0, 2.0, accent());
    }
}

/// Translation keys for the help page's lines, in the order it shows them.
const HELP_KEYS: [&'static str; 36] = [
    "device.quick.help.shelf_pick",
    "device.quick.help.shelf_search",
    "device.quick.help.shelf_settings",
    "device.quick.help.shelf_network",
    "device.quick.help.shelf_core",
    "device.quick.help.shelf_restart",
    "device.quick.help.shelf_sync",
    "device.quick.help.sync_conflict",
    "device.quick.help.game_menu",
    "device.quick.help.game_exit",
    "device.quick.help.album_open",
    "device.quick.help.state_shortcuts",
    "device.quick.help.album_browse",
    "device.quick.help.album_delete",
    "device.quick.help.rewind",
    "device.quick.help.fast_forward",
    "device.quick.help.brightness",
    "device.quick.help.blue_light",
    "device.quick.help.shader",
    "device.quick.help.colour",
    "device.quick.help.palette",
    "device.quick.help.volume",
    "device.quick.help.mute",
    "device.quick.help.link",
    "device.quick.help.turbo",
    "device.quick.help.gb_video",
    "device.quick.help.sfc_video",
    "device.quick.help.power",
    "device.quick.help.lid",
    "device.quick.help.power_menu",
    "device.quick.help.chord",
    "device.quick.help.swap_shoulders",
    "device.quick.help.xy_shoulders",
    "device.quick.help.both_shoulders",
    "device.quick.help.cheats",
    "device.quick.help.cheats_default",
];

/// Actual input bindings in `lang`; shoulder combinations are stated in the
/// default layout.
///
/// Returned by value: an array of `&'static str` is `Copy`, so this costs the
/// same as handing out a slice while letting the language pick the text.
pub fn help(lang: Language) -> [&'static str; HELP_KEYS.len()] {
    HELP_KEYS.map(|key| slot_i18n::t(lang, key))
}
#[cfg(test)]
mod tests {
    use super::*;

    fn alpha(face: &UndoFace, x: u32, y: u32) -> u8 {
        face.rgba[((y * face.w + x) * 4 + 3) as usize]
    }

    fn rgb(face: &UndoFace, x: u32, y: u32) -> [u8; 3] {
        let i = ((y * face.w + x) * 4) as usize;
        [face.rgba[i], face.rgba[i + 1], face.rgba[i + 2]]
    }

    #[test]
    fn the_shelf_title_has_no_menu_accent_bar() {
        let face = title("宝可梦");
        assert_eq!(alpha(&face, 8, 30), 0, "the old red shelf bar came back");
        assert!(
            face.rgba.chunks(4).any(|pixel| pixel[3] > 0),
            "the title type disappeared"
        );
    }

    #[test]
    fn the_folder_header_is_full_width_with_mode_signals() {
        let face = folder_header("动作游戏", Some("阿拉丁"), "GBA", 2, 6, Language::ZhCn);
        assert_eq!((face.w, face.h), (664, 76));
        assert_eq!(alpha(&face, 0, 20), 255, "the card is not full width");
        assert_eq!(rgb(&face, 1, 20), FOLDER_MODE, "the folder signal spine");
        assert!(
            colour_count(&face, FOLDER_MODE) > 80,
            "the folder mode colour is too sparse"
        );
    }

    #[test]
    fn the_folder_header_status_changes_with_the_position() {
        let first = folder_header("动作", Some("A"), "GBA", 1, 4, Language::EnUs);
        let last = folder_header("动作", Some("A"), "GBA", 4, 4, Language::EnUs);
        assert_ne!(first.rgba, last.rgba, "the right-hand status did not change");
    }

    #[test]
    fn the_folder_header_game_line_only_appears_with_a_game() {
        let face = folder_header("动作游戏", Some("阿拉丁"), "GBA", 2, 6, Language::ZhCn);
        assert!(lower_ink(&face) > 0, "the game line is missing");
        let bare = folder_header("动作游戏", None, "GBA", 2, 6, Language::ZhCn);
        assert_eq!(lower_ink(&bare), 0, "an empty game line was drawn");
    }

    #[test]
    fn the_transfer_header_keeps_host_and_join_apart_with_colour() {
        let host = transfer_header(true, Some("阿拉丁"), 0, Language::ZhCn);
        let joiner = transfer_header(false, Some("阿拉丁"), 0, Language::ZhCn);
        assert_eq!((host.w, host.h), (664, 76));
        assert_eq!(rgb(&host, 1, 20), HOST_MODE, "the host spine");
        assert_eq!(rgb(&joiner, 1, 20), JOIN_MODE, "the join spine");
        assert_ne!(host.rgba, joiner.rgba);
        assert!(lower_ink(&host) > 0, "the game line is missing");
        let bare = transfer_header(false, None, 0, Language::ZhCn);
        assert_eq!(lower_ink(&bare), 0, "an empty game line was drawn");
        assert!(colour_count(&host, HOST_MODE) > 60);
        assert!(colour_count(&joiner, JOIN_MODE) > 60);
    }

    fn lower_ink(face: &UndoFace) -> usize {
        let ink = sw::ink();
        (44..76)
            .flat_map(|y| (0..face.w).map(move |x| (x, y)))
            .filter(|(x, y)| rgb(face, *x, *y) == ink)
            .count()
    }

    fn colour_count(face: &UndoFace, colour: [u8; 3]) -> usize {
        face.rgba
            .chunks_exact(4)
            .filter(|p| p[0] == colour[0] && p[1] == colour[1] && p[2] == colour[2])
            .count()
    }

    #[test]
    fn the_transfer_header_right_aligns_the_side() {
        let face = transfer_header(true, Some("阿拉丁"), 0, Language::ZhCn);
        let right = face.w - 1;
        let last_ink = (0..face.w)
            .rfind(|x| (10..34).any(|y| rgb(&face, *x, y).iter().any(|c| *c > 0x80)))
            .unwrap_or(0);
        assert!(
            right - last_ink <= 40,
            "the side is not right-aligned to the card ({last_ink} vs {right})"
        );
    }

    #[test]
    fn the_hold_prompt_has_a_track_for_the_live_fill() {
        let face = folder_hold_prompt(Language::ZhCn);
        assert_eq!((face.w, face.h), (FOLDER_HOLD_W, FOLDER_HOLD_H));
        let [x, y, w, h] = folder_hold_bar();
        assert_eq!(
            alpha(&face, (x + w / 2.0) as u32, (y + h / 2.0) as u32),
            255,
            "the track is open"
        );
        assert!(w > 0.0 && h > 0.0);
        assert!(
            face.rgba.chunks(4).any(|pixel| pixel[3] > 0),
            "the prompt drew nothing"
        );
    }

    fn white_pixels_below(face: &UndoFace, y0: u32) -> usize {
        let selected = sw::selected();
        let selected = [selected[0], selected[1], selected[2]];
        let mut count = 0;
        for y in y0..face.h {
            for x in 0..face.w {
                if rgb(face, x, y) == selected {
                    count += 1;
                }
            }
        }
        count
    }

    #[test]
    fn the_letter_picker_is_a_switch_face_with_one_white_selection() {
        let available = [true; 26];
        let face = picker(4, available, Language::ZhCn);
        assert_eq!((face.w, face.h), (720, 480));
        let bg = sw::bg();
        assert_eq!(
            (rgb(&face, 0, 0), alpha(&face, 0, 0)),
            ([bg[0], bg[1], bg[2]], 255)
        );
        let line = sw::surface_border();
        assert_eq!(
            rgb(&face, 100, 40),
            [line[0], line[1], line[2]],
            "the 1px header rule"
        );
        assert!(white_pixels_below(&face, PICKER_TOP as u32) > 0);
        let dim = [false; 26];
        assert!(
            white_pixels_below(&picker(0, dim, Language::ZhCn), PICKER_TOP as u32) > 0,
            "the selected cell is not white even when unavailable"
        );
    }

    #[test]
    fn the_panel_puts_the_selected_row_on_a_white_card() {
        let rows: Vec<String> = (0..8).map(|i| format!("即时存档 {i}")).collect();
        let top = panel("存档相册", &rows, 0, Language::ZhCn);
        let lower = panel("存档相册", &rows, 7, Language::ZhCn);
        assert_eq!((top.w, top.h), (720, 480));
        let bg = sw::bg_top();
        assert_eq!(rgb(&top, 0, 0), [bg[0], bg[1], bg[2]]);
        let card_row = |face: &UndoFace| white_pixels_below(face, ROW_TOP as u32);
        assert!(card_row(&top) > 0);
        assert!(card_row(&lower) > 0);
        assert_ne!(top.rgba, lower.rgba, "the paged selection did not move");
        let empty: [String; 0] = [];
        let blank = panel("存档相册", &empty, 0, Language::ZhCn);
        assert_eq!(blank.rgba.len(), (720 * 480 * 4) as usize);
    }

    #[test]
    fn the_content_chooser_marks_the_row_in_force_and_moves_the_cursor() {
        let first = transfer_content_menu(0, 0, Language::ZhCn);
        assert_eq!((first.w, first.h), (720, 480));
        let bg = sw::bg_top();
        assert_eq!(rgb(&first, 0, 0), [bg[0], bg[1], bg[2]]);
        assert!(
            white_pixels_below(&first, 88) > 0,
            "the cursor row is not on a white card"
        );
        let moved = transfer_content_menu(1, 0, Language::ZhCn);
        assert_ne!(first.rgba, moved.rgba, "the cursor did not move");
        let other_current = transfer_content_menu(0, 2, Language::ZhCn);
        assert_ne!(
            first.rgba, other_current.rgba,
            "the dot did not follow the choice in force"
        );
    }

    #[test]
    fn the_room_header_chip_names_the_content_choice() {
        let cart = transfer_header(true, Some("阿拉丁"), 0, Language::ZhCn);
        let saves = transfer_header(true, Some("阿拉丁"), 1, Language::ZhCn);
        let all = transfer_header(true, Some("阿拉丁"), 2, Language::ZhCn);
        assert_ne!(cart.rgba, saves.rgba, "the chip ignored 仅存档");
        assert_ne!(saves.rgba, all.rgba, "the chip ignored 卡带＋存档");
    }
}
