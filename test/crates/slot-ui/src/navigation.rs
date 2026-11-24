//! Overlay panels in the shared Nintendo-Switch-inspired style: the save-album
//! / utility panel, the game-title heading and the letter picker.  Only the
//! drawing changed; the pagination and A/B/left-right input logic is untouched.

use slot_i18n::Language;

use crate::draw::OUT_W;
use crate::keyboard as kb;
use crate::plate::UndoFace;
use crate::switch_style as sw;

const HEADER_H: f32 = 88.0;
const SIDE: f32 = 48.0;
const CARD_W: f32 = OUT_W as f32 - 2.0 * SIDE;
const LIST_TOP: f32 = 92.0;
const LIST_H: f32 = 280.0;
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

/// The folder name's teal, kept fixed so a theme change does not turn the
/// browse identity into something that reads as a warning or an accent.
const FOLDER_INK: [u8; 3] = [0x66, 0xe2, 0xba];
/// The right-hand status is deliberately brighter than the folder name but
/// cooler, so it leans back while staying readable.
const FOLDER_STATUS_INK: [u8; 3] = [0xc2, 0xcd, 0xde];
/// The transfer room marks which side it is with a dot: teal for the host,
/// blue for the joiner.  The mode name keeps the folder teal.
const TRANSFER_HOST_DOT: [u8; 3] = [0x66, 0xe2, 0xba];
const TRANSFER_JOIN_DOT: [u8; 3] = [0x8a, 0xc8, 0xff];

/// A folder-mode header: a compact single-line status card on top (folder icon,
/// folder name, then the platform / mode / position right-aligned), with the
/// selected game's name on a smaller chip underneath.
///
/// The face is 664×76, the same rectangle the normal navigation title uses, so
/// the frontend can cross-fade the two without moving the shelf.
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
    const CARD_H: f32 = 44.0;
    const PAD: f32 = 22.0;
    const ICON_W: f32 = 28.0;
    const GAP: f32 = 24.0;
    const MIN_CARD_W: f32 = 350.0;
    /// Leave the status bar's clock and battery corners alone even when a deep
    /// folder path has to be trimmed.
    const MAX_CARD_W: f32 = 520.0;
    let mut face = kb::blank(W as u32, H as u32, [0, 0, 0, 0]);

    let status = format!(
        "{} · {} {}/{}",
        platform,
        slot_i18n::t(lang, "device.app.folder.mode"),
        index,
        total
    );
    let status_px = 12.0;
    let status_w = kb::ink_width(&status, status_px);
    // Fit the folder name into the widest card first; the card itself then
    // shrinks towards its minimum so the short Chinese names stay compact and
    // keep the status-bar clock and battery clear.
    let max_name_w = (MAX_CARD_W - 2.0 * PAD - ICON_W - GAP - status_w).max(40.0);
    let (folder_line, folder_px) = kb::fit_line(folder, max_name_w, 20.0, 13.0);
    let folder_w = kb::ink_width(&folder_line, folder_px);
    let card_w = (PAD + ICON_W + folder_w + GAP + status_w + PAD)
        .clamp(MIN_CARD_W, MAX_CARD_W);
    let card_x = ((W - card_w) / 2.0).round();

    let surface = sw::surface();
    let border = sw::surface_border();
    sw::fill_round(
        &mut face,
        card_x,
        0.0,
        card_w,
        CARD_H,
        13.0,
        [border[0], border[1], border[2], 0xcc],
    );
    sw::fill_round(
        &mut face,
        card_x + 1.0,
        1.0,
        card_w - 2.0,
        CARD_H - 2.0,
        12.0,
        [surface[0], surface[1], surface[2], 0xe8],
    );

    // A folder outline, drawn from thin bars so it reads at any theme.
    let icon_x = card_x + PAD;
    let icon_y = 15.0;
    let ink = [FOLDER_INK[0], FOLDER_INK[1], FOLDER_INK[2], 0xff];
    kb::fill(&mut face, icon_x, icon_y, 8.0, 2.0, ink);
    kb::fill(&mut face, icon_x + 6.0, icon_y + 1.0, 2.0, 3.0, ink);
    kb::fill(&mut face, icon_x, icon_y + 3.0, 18.0, 2.0, ink);
    kb::fill(&mut face, icon_x, icon_y + 3.0, 2.0, 10.0, ink);
    kb::fill(&mut face, icon_x + 16.0, icon_y + 3.0, 2.0, 10.0, ink);
    kb::fill(&mut face, icon_x, icon_y + 11.0, 18.0, 2.0, ink);

    let status_x = card_x + card_w - PAD - status_w;
    let status_top = (CARD_H - kb::line_height(status_px)) / 2.0;
    kb::draw_text(
        &mut face,
        &status,
        status_px,
        status_x,
        status_top.max(0.0),
        FOLDER_STATUS_INK,
    );

    let name_x = icon_x + ICON_W;
    let folder_top = (CARD_H - kb::line_height(folder_px)) / 2.0;
    kb::draw_text(
        &mut face,
        &folder_line,
        folder_px,
        name_x,
        folder_top.max(0.0),
        FOLDER_INK,
    );

    if let Some(game) = game {
        let (game_line, game_px) = kb::fit_line(game, 540.0, 17.0, 12.0);
        let text_w = kb::ink_width(&game_line, game_px);
        let chip_w = (text_w + 90.0).clamp(150.0, 600.0);
        let chip_x = ((W - chip_w) / 2.0).round();
        sw::fill_round(
            &mut face,
            chip_x,
            47.0,
            chip_w,
            27.0,
            9.0,
            [border[0], border[1], border[2], 0xb8],
        );
        sw::fill_round(
            &mut face,
            chip_x + 1.0,
            48.0,
            chip_w - 2.0,
            25.0,
            8.0,
            [surface[0], surface[1], surface[2], 0xd0],
        );
        let top = 47.0 + (27.0 - kb::line_height(game_px)) / 2.0;
        kb::draw_text(
            &mut face,
            &game_line,
            game_px,
            chip_x + (chip_w - text_w) / 2.0,
            top.max(47.0),
            sw::ink(),
        );
    }
    face
}

/// A transfer-room header: the mode name and the current side (host / joiner)
/// on one compact card, with the selected cartridge's name and the content
/// choice on a chip underneath.  Same rectangle as [`folder_header`], so the
/// frontend can swap the two without moving the shelf.
pub fn transfer_header(
    host: bool,
    game: Option<&str>,
    content: usize,
    lang: Language,
) -> UndoFace {
    const W: f32 = 664.0;
    const H: f32 = 76.0;
    const CARD_H: f32 = 44.0;
    const PAD: f32 = 22.0;
    const ICON_W: f32 = 26.0;
    const GAP: f32 = 22.0;
    const MIN_CARD_W: f32 = 320.0;
    /// Same ceiling as the folder header: the status-bar clock and battery
    /// keep their corners.
    const MAX_CARD_W: f32 = 520.0;
    let mut face = kb::blank(W as u32, H as u32, [0, 0, 0, 0]);

    let role = slot_i18n::t(
        lang,
        if host {
            "device.app.transfer.role_host"
        } else {
            "device.app.transfer.role_join"
        },
    );
    let role_px = 13.0;
    let role_text_w = kb::ink_width(role, role_px);
    // The dot sits 6 px before the text and is followed by the same gap.
    let role_w = role_text_w + 12.0;
    let max_title_w = (MAX_CARD_W - 2.0 * PAD - ICON_W - GAP - role_w).max(60.0);
    let (title_line, title_px) = kb::fit_line(
        slot_i18n::t(lang, "device.app.transfer.title"),
        max_title_w,
        20.0,
        13.0,
    );
    let title_w = kb::ink_width(&title_line, title_px);
    let card_w = (PAD + ICON_W + title_w + GAP + role_w + PAD).clamp(MIN_CARD_W, MAX_CARD_W);
    let card_x = ((W - card_w) / 2.0).round();

    let surface = sw::surface();
    let border = sw::surface_border();
    sw::fill_round(
        &mut face,
        card_x,
        0.0,
        card_w,
        CARD_H,
        13.0,
        [border[0], border[1], border[2], 0xcc],
    );
    sw::fill_round(
        &mut face,
        card_x + 1.0,
        1.0,
        card_w - 2.0,
        CARD_H - 2.0,
        12.0,
        [surface[0], surface[1], surface[2], 0xe8],
    );

    // Exchange arrows: this device sends one card and receives another.
    let icon_x = card_x + PAD;
    let icon_y = 11.0;
    let ink = [FOLDER_INK[0], FOLDER_INK[1], FOLDER_INK[2], 0xff];
    kb::fill(&mut face, icon_x + 2.0, icon_y + 6.0, 15.0, 2.0, ink);
    kb::fill(&mut face, icon_x + 14.0, icon_y + 2.0, 2.0, 2.0, ink);
    kb::fill(&mut face, icon_x + 16.0, icon_y + 4.0, 2.0, 6.0, ink);
    kb::fill(&mut face, icon_x + 14.0, icon_y + 10.0, 2.0, 2.0, ink);
    kb::fill(&mut face, icon_x + 7.0, icon_y + 16.0, 15.0, 2.0, ink);
    kb::fill(&mut face, icon_x + 8.0, icon_y + 12.0, 2.0, 2.0, ink);
    kb::fill(&mut face, icon_x + 6.0, icon_y + 14.0, 2.0, 6.0, ink);
    kb::fill(&mut face, icon_x + 8.0, icon_y + 20.0, 2.0, 2.0, ink);

    let title_top = (CARD_H - kb::line_height(title_px)) / 2.0;
    kb::draw_text(
        &mut face,
        &title_line,
        title_px,
        icon_x + ICON_W,
        title_top.max(0.0),
        FOLDER_INK,
    );

    let role_x = card_x + card_w - PAD - role_text_w;
    let dot = if host {
        TRANSFER_HOST_DOT
    } else {
        TRANSFER_JOIN_DOT
    };
    sw::fill_round(
        &mut face,
        role_x - 12.0,
        (CARD_H - 6.0) / 2.0,
        6.0,
        6.0,
        3.0,
        [dot[0], dot[1], dot[2], 0xff],
    );
    let role_top = (CARD_H - kb::line_height(role_px)) / 2.0;
    kb::draw_text(
        &mut face,
        role,
        role_px,
        role_x,
        role_top.max(0.0),
        FOLDER_STATUS_INK,
    );

    // The chip under the card says what is selected *and* what A will send, so
    // a player who set 仅存档 an hour ago is not surprised by a save overwrite.
    let content_key = match content.min(2) {
        1 => "device.app.transfer.content_save",
        2 => "device.app.transfer.content_all",
        _ => "device.app.transfer.content_cart",
    };
    let content_label = slot_i18n::t(lang, content_key);
    // No selected cartridge, no chip: the room keeps the same empty strip it
    // had before the content choice existed.
    let chip_text = game.map(|game| format!("{game} · {content_label}"));
    if let Some(chip_text) = chip_text {
        let (game_line, game_px) = kb::fit_line(&chip_text, 540.0, 17.0, 12.0);
        let text_w = kb::ink_width(&game_line, game_px);
        let chip_w = (text_w + 90.0).clamp(150.0, 600.0);
        let chip_x = ((W - chip_w) / 2.0).round();
        sw::fill_round(
            &mut face,
            chip_x,
            47.0,
            chip_w,
            27.0,
            9.0,
            [border[0], border[1], border[2], 0xb8],
        );
        sw::fill_round(
            &mut face,
            chip_x + 1.0,
            48.0,
            chip_w - 2.0,
            25.0,
            8.0,
            [surface[0], surface[1], surface[2], 0xd0],
        );
        let top = 47.0 + (27.0 - kb::line_height(game_px)) / 2.0;
        kb::draw_text(
            &mut face,
            &game_line,
            game_px,
            chip_x + (chip_w - text_w) / 2.0,
            top.max(47.0),
            sw::ink(),
        );
    }
    face
}

/// The small "hold SELECT" prompt.  Only the card and its track are baked in;
/// the frontend draws the fill from [`folder_hold_bar`] every frame, so the bar
/// moves without re-uploading a texture.
pub fn folder_hold_prompt(lang: Language) -> UndoFace {
    let mut face = kb::blank(FOLDER_HOLD_W, FOLDER_HOLD_H, [0, 0, 0, 0]);
    let surface = sw::surface();
    let border = sw::surface_border();
    sw::fill_round(
        &mut face,
        0.0,
        0.0,
        FOLDER_HOLD_W as f32,
        FOLDER_HOLD_H as f32,
        14.0,
        [border[0], border[1], border[2], 0xcc],
    );
    sw::fill_round(
        &mut face,
        1.0,
        1.0,
        FOLDER_HOLD_W as f32 - 2.0,
        FOLDER_HOLD_H as f32 - 2.0,
        13.0,
        [surface[0], surface[1], surface[2], 0xf0],
    );
    kb::fill(
        &mut face,
        5.0,
        13.0,
        3.0,
        FOLDER_HOLD_H as f32 - 26.0,
        [FOLDER_INK[0], FOLDER_INK[1], FOLDER_INK[2], 0xff],
    );
    kb::draw_left(
        &mut face,
        slot_i18n::t(lang, "device.app.folder.hold"),
        15.0,
        11.0,
        FOLDER_HOLD_W as f32 - 32.0,
        18.0,
        6.0,
        22.0,
        FOLDER_INK,
    );
    let [bx, by, bw, bh] = folder_hold_bar();
    sw::fill_round(
        &mut face,
        bx,
        by,
        bw,
        bh,
        bh / 2.0,
        [0x0e, 0x10, 0x15, 0xff],
    );
    face
}

pub const FOLDER_HOLD_W: u32 = 288;
pub const FOLDER_HOLD_H: u32 = 58;
/// Where the hold prompt sits on the output screen: `[x, y]`.
pub const FOLDER_HOLD_ORIGIN: [f32; 2] = [216.0, 92.0];

/// The fill area of the hold prompt's bar, relative to the prompt face.
pub fn folder_hold_bar() -> [f32; 4] {
    [18.0, 40.0, 252.0, 8.0]
}

/// The progress bar's fill colour.
pub fn folder_hold_fill() -> [f32; 4] {
    [
        FOLDER_INK[0] as f32 / 255.0,
        FOLDER_INK[1] as f32 / 255.0,
        FOLDER_INK[2] as f32 / 255.0,
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
                sw::CARD_RADIUS,
                accent(),
            );
            sw::fill_round(
                &mut face,
                x + 2.0,
                y + 2.0,
                PICKER_CELL_W - 4.0,
                PICKER_CELL_H - 4.0,
                (sw::CARD_RADIUS - 2.0).max(0.0),
                sw::selected(),
            );
        } else if avail {
            card_at(&mut face, x, y, PICKER_CELL_W, PICKER_CELL_H, false);
        } else {
            sw::fill_round(
                &mut face,
                x,
                y,
                PICKER_CELL_W,
                PICKER_CELL_H,
                sw::CARD_RADIUS,
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

/// Cartridge-transfer matching menu: artwork on the left, the three choices on
/// the right. The frontend pastes the selected Host/Join artwork into the left
/// card, so this face deliberately leaves that area blank.
pub fn transfer_menu(mode: &str, selected: usize, lang: Language) -> UndoFace {
    let mut face = kb::blank(OUT_W, 480, sw::bg());
    kb::fill(&mut face, 0.0, 0.0, OUT_W as f32, HEADER_H, sw::bg_top());
    kb::fill(&mut face, SIDE, 16.0, 5.0, 30.0, accent());
    let ink = sw::ink();
    kb::draw_text(
        &mut face,
        slot_i18n::t(lang, "device.app.transfer.title"),
        28.0,
        64.0,
        12.0,
        [ink[0], ink[1], ink[2]],
    );

    // Mode chip at the top-right, separate from the heading.
    sw::fill_round(&mut face, 526.0, 16.0, 156.0, 32.0, 10.0, accent());
    sw::fill_round(&mut face, 528.0, 18.0, 152.0, 28.0, 8.0, sw::surface());
    let mode_ink = sw::accent();
    kb::draw_text(
        &mut face,
        mode,
        16.0,
        538.0,
        20.0,
        [mode_ink[0], mode_ink[1], mode_ink[2]],
    );

    // Left artwork card; the frontend fills it with the current role/mode art.
    card_at(&mut face, 40.0, 92.0, 280.0, 200.0, false);

    let rows = [
        ("device.app.transfer.create", "device.app.transfer.create_hint"),
        ("device.app.transfer.join", "device.app.transfer.join_hint"),
        ("device.app.common.back", "device.app.transfer.back_hint"),
    ];
    for (index, (title_key, hint_key)) in rows.iter().enumerate() {
        let y = 96.0 + index as f32 * 66.0;
        let selected = index == selected.min(2);
        card_at(&mut face, 340.0, y, 354.0, 58.0, selected);
        let ink = if selected { sw::ink_selected() } else { sw::ink() };
        let dim = if selected { sw::ink_selected() } else { sw::ink_dim() };
        kb::draw_text(
            &mut face,
            slot_i18n::t(lang, title_key),
            18.0,
            356.0,
            y + 9.0,
            [ink[0], ink[1], ink[2]],
        );
        kb::draw_text(
            &mut face,
            slot_i18n::t(lang, hint_key),
            11.0,
            356.0,
            y + 34.0,
            [dim[0], dim[1], dim[2]],
        );
    }

    let hints = [
        ("device.app.transfer.hint_nav", 48.0),
        ("device.app.transfer.hint_confirm", 172.0),
        ("device.app.transfer.select_mode", 286.0),
        ("device.app.transfer.hint_back", 470.0),
    ];
    for (key, x) in hints {
        let dim = sw::ink_dim();
        kb::draw_text(
            &mut face,
            slot_i18n::t(lang, key),
            11.0,
            x,
            HINT_Y + 8.0,
            [dim[0], dim[1], dim[2]],
        );
    }
    face
}

/// Cartridge-transfer content chooser: three radio rows under the room's
/// heading.  `selected` is the cursor, `current` the value already in force;
/// both are 0 = cart only, 1 = saves only, 2 = cart and saves.
pub fn transfer_content_menu(selected: usize, current: usize, lang: Language) -> UndoFace {
    const CARD_X: f32 = 56.0;
    const CARD_W: f32 = 608.0;
    const CARD_H: f32 = 78.0;
    const CARD_GAP: f32 = 9.0;
    const CARD_TOP: f32 = 108.0;
    const RADIO_X: f32 = 86.0;

    let mut face = kb::blank(OUT_W, 480, sw::bg());
    kb::fill(&mut face, 0.0, 0.0, OUT_W as f32, HEADER_H, sw::bg_top());
    kb::fill(&mut face, 48.0, 20.0, 5.0, 30.0, accent());
    let ink = sw::ink();
    kb::draw_text(
        &mut face,
        slot_i18n::t(lang, "device.app.transfer.content"),
        26.0,
        64.0,
        18.0,
        [ink[0], ink[1], ink[2]],
    );
    let dim = sw::ink_dim();
    kb::draw_text(
        &mut face,
        slot_i18n::t(lang, "device.app.transfer.content_hint"),
        12.0,
        66.0,
        56.0,
        [dim[0], dim[1], dim[2]],
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
    for (index, (title_key, hint_key)) in rows.iter().enumerate() {
        let y = CARD_TOP + index as f32 * (CARD_H + CARD_GAP);
        let lit = index == selected.min(2);
        card_at(&mut face, CARD_X, y, CARD_W, CARD_H, lit);

        // Radio dot: filled accent for the row that is in force, a ring for
        // the others.  The cursor is the white card, not the dot.
        let cy = y + CARD_H / 2.0;
        let dot_fill = if lit { sw::selected() } else { sw::surface() };
        if index == current.min(2) {
            sw::fill_round(&mut face, RADIO_X - 9.0, cy - 9.0, 18.0, 18.0, 9.0, accent());
            sw::fill_round(
                &mut face,
                RADIO_X - 3.5,
                cy - 3.5,
                7.0,
                7.0,
                3.5,
                [0xff, 0xff, 0xff, 0xff],
            );
        } else {
            sw::fill_round(
                &mut face,
                RADIO_X - 9.0,
                cy - 9.0,
                18.0,
                18.0,
                9.0,
                sw::surface_border(),
            );
            sw::fill_round(&mut face, RADIO_X - 7.0, cy - 7.0, 14.0, 14.0, 7.0, dot_fill);
        }

        let title_ink = if lit { sw::ink_selected() } else { sw::ink() };
        let hint_ink = if lit { sw::ink_selected() } else { sw::ink_dim() };
        kb::draw_text(
            &mut face,
            slot_i18n::t(lang, title_key),
            21.0,
            112.0,
            y + 13.0,
            title_ink,
        );
        kb::draw_text(
            &mut face,
            slot_i18n::t(lang, hint_key),
            12.5,
            112.0,
            y + 46.0,
            hint_ink,
        );

        if index == current.min(2) {
            let chip_w = 96.0;
            let chip_x = CARD_X + CARD_W - chip_w - 12.0;
            sw::fill_round(&mut face, chip_x, y + 14.0, chip_w, 24.0, 8.0, accent());
            let label = slot_i18n::t(lang, "device.app.transfer.content_current");
            let px = 12.0;
            let w = kb::ink_width(label, px);
            kb::draw_text(
                &mut face,
                label,
                px,
                chip_x + (chip_w - w) / 2.0,
                y + 19.0,
                [0xff, 0xff, 0xff],
            );
        }
    }

    let hints = [
        ("device.app.transfer.hint_nav", 56.0),
        ("device.app.transfer.hint_confirm", 148.0),
        ("device.app.transfer.hint_back", 228.0),
        ("device.app.transfer.content_close", 308.0),
    ];
    for (key, x) in hints {
        let dim = sw::ink_dim();
        kb::draw_text(
            &mut face,
            slot_i18n::t(lang, key),
            12.0,
            x,
            HINT_Y + 6.0,
            [dim[0], dim[1], dim[2]],
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
    // One rounded content area behind the rows.
    sw::fill_round(&mut face, SIDE, LIST_TOP, CARD_W, LIST_H, 14.0, sw::surface());
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
            12.0,
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
        12.0,
        10.0,
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
    sw::fill_round(&mut face, SIDE, ICON_TOP - 4.0, CARD_W, 278.0, 14.0, sw::surface());
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
                11.0,
                9.0,
                CARD_W - 230.0,
                icon_x + 70.0,
                y + 25.0,
                15.0,
                sub_ink,
            );
            kb::draw_box(
                &mut face,
                &row.points,
                13.0,
                10.0,
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
    sw::fill_round(&mut face, bx, by, bw, bh, bh / 2.0, sw::bg_top());
    face
}

/// The Switch heading band: charcoal ground, red accent bar and white title.
fn heading(face: &mut UndoFace, title: &str) {
    kb::fill(face, 0.0, 0.0, OUT_W as f32, HEADER_H, sw::bg_top());
    kb::fill(face, SIDE, 16.0, 5.0, 30.0, accent());
    kb::draw_box(
        face,
        title,
        28.0,
        15.0,
        64.0,
        12.0,
        OUT_W as f32 - 96.0,
        40.0,
        sw::ink(),
    );
}

fn accent() -> [u8; 4] {
    [sw::accent()[0], sw::accent()[1], sw::accent()[2], 0xff]
}

/// A Switch-style rounded card: white with a white rim when selected, charcoal
/// surface with a light rim otherwise.
fn card_at(face: &mut UndoFace, x: f32, y: f32, w: f32, h: f32, selected: bool) {
    let (rim, fill) = if selected {
        (sw::selected_border(), sw::selected())
    } else {
        (sw::surface_border(), sw::surface())
    };
    sw::fill_round(face, x, y, w, h, sw::CARD_RADIUS, rim);
    sw::fill_round(
        face,
        x + 1.0,
        y + 1.0,
        w - 2.0,
        h - 2.0,
        (sw::CARD_RADIUS - 1.0).max(0.0),
        fill,
    );
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
    fn the_folder_header_has_a_status_band_and_a_game_chip() {
        let face = folder_header("动作游戏", Some("阿拉丁"), "GBA", 2, 6, Language::ZhCn);
        assert_eq!((face.w, face.h), (664, 76));
        assert!(alpha(&face, 332, 20) > 0, "the status card is missing");
        assert_eq!(
            alpha(&face, 80, 20),
            0,
            "the status card covers the clock corner again"
        );
        assert!(alpha(&face, 332, 60) > 0, "the game chip is missing");
        // The game chip only appears when the game title is allowed.
        let bare = folder_header("动作游戏", None, "GBA", 2, 6, Language::ZhCn);
        assert_eq!(alpha(&bare, 332, 60), 0, "an empty game chip was drawn");
    }

    #[test]
    fn the_folder_header_status_changes_with_the_position() {
        let first = folder_header("动作", Some("A"), "GBA", 1, 4, Language::EnUs);
        let last = folder_header("动作", Some("A"), "GBA", 4, 4, Language::EnUs);
        assert_ne!(first.rgba, last.rgba, "the right-hand status did not change");
        assert!(
            first.rgba.iter().skip(3).step_by(4).any(|alpha| *alpha > 0),
            "the English header drew nothing"
        );
    }

    #[test]
    fn a_long_folder_name_is_trimmed_inside_the_header() {
        let face = folder_header(
            "Documents/Projects/Handheld/LinkSlot/Extremely Long Folder Name",
            Some("A"),
            "GBA",
            1,
            12,
            Language::EnUs,
        );
        // The card may grow but never spans the whole 664 px face, so the
        // status-bar clock and battery keep their corners.
        let row: Vec<u32> = (0..face.w).filter(|x| alpha(&face, *x, 30) > 0).collect();
        let (left, right) = (*row.first().unwrap(), *row.last().unwrap());
        assert!(
            right - left + 1 <= 520,
            "the folder card grew over the status bar: {left}..{right}"
        );
        assert!(right < 664, "the card reached the face edge");
    }

    #[test]
    fn the_folder_header_right_aligns_its_status_and_centres_the_game_name() {
        let face = folder_header("动作游戏", Some("阿拉丁"), "GBA", 2, 6, Language::ZhCn);
        let pixels = |y0: u32, y1: u32, colour: [u8; 3]| {
            (y0..y1)
                .flat_map(|y| (0..face.w).map(move |x| (x, y)))
                .filter(|(x, y)| rgb(&face, *x, *y) == colour)
                .collect::<Vec<(u32, u32)>>()
        };
        let card: Vec<u32> = (0..face.w).filter(|x| alpha(&face, *x, 30) > 0).collect();
        let (left, right) = (*card.first().unwrap(), *card.last().unwrap());
        assert!(
            right - left + 1 < 600,
            "the compact folder card is still full width: {left}..{right}"
        );
        assert!(
            ((left as f32 + right as f32) / 2.0 - 332.0).abs() < 2.0,
            "the folder card is not centred: {left}..{right}"
        );
        let status = pixels(0, 44, FOLDER_STATUS_INK);
        let status_right = status.iter().map(|(x, _)| *x).max().unwrap_or(0);
        assert!(
            status_right + 14 <= right && status_right + 40 >= right,
            "the status is not right-aligned to the card ({status_right} vs {right})"
        );
        let folder = pixels(0, 44, FOLDER_INK);
        assert!(
            folder.iter().any(|(x, _)| *x > left + 30 && *x < left + 260),
            "the folder name did not use the left part of the band"
        );
        let game = pixels(47, 76, sw::ink());
        let centre = game.iter().map(|(x, _)| *x as f32).sum::<f32>() / game.len() as f32;
        assert!(
            (centre - 332.0).abs() < 30.0,
            "the game name is centred at {centre}"
        );
    }

    #[test]
    fn the_transfer_header_names_the_side_and_keeps_the_cart_chip() {
        let host = transfer_header(true, Some("阿拉丁"), 0, Language::ZhCn);
        let joiner = transfer_header(false, Some("阿拉丁"), 0, Language::ZhCn);
        assert_eq!((host.w, host.h), (664, 76));
        assert!(alpha(&host, 332, 20) > 0, "the mode card is missing");
        assert_eq!(
            alpha(&host, 80, 20),
            0,
            "the transfer card covers the clock corner again"
        );
        assert!(alpha(&host, 332, 60) > 0, "the cart chip is missing");
        assert_ne!(host.rgba, joiner.rgba, "the side did not change the header");
        // The cart chip only appears when the game title is allowed.
        let bare = transfer_header(false, None, 0, Language::ZhCn);
        assert_eq!(alpha(&bare, 332, 60), 0, "an empty cart chip was drawn");
        // The longer English role must not push the card over the status bar.
        let english = transfer_header(false, Some("Emerald"), 0, Language::EnUs);
        let row: Vec<u32> = (0..english.w)
            .filter(|x| alpha(&english, *x, 30) > 0)
            .collect();
        let (left, right) = (*row.first().unwrap(), *row.last().unwrap());
        assert!(
            right - left < 520,
            "the transfer card grew over the status bar: {left}..{right}"
        );
        assert!(((left as f32 + right as f32) / 2.0 - 332.0).abs() < 2.0);
    }

    #[test]
    fn the_transfer_header_right_aligns_the_side() {
        let face = transfer_header(true, Some("阿拉丁"), 0, Language::ZhCn);
        let card: Vec<u32> = (0..face.w).filter(|x| alpha(&face, *x, 30) > 0).collect();
        let (left, right) = (*card.first().unwrap(), *card.last().unwrap());
        let half = left + (right - left) / 2;
        // The dot and the side word are the only bright ink in the right half.
        let last_ink = (half..=right)
            .rfind(|x| (10..34).any(|y| rgb(&face, *x, y).iter().any(|c| *c > 0x80)))
            .unwrap_or(0);
        assert!(
            right - last_ink <= 30,
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
        let mut count = 0;
        for y in y0..face.h {
            for x in 0..face.w {
                if rgb(face, x, y) == [0xf2, 0xf2, 0xf2] {
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
        assert_eq!(
            (rgb(&face, 0, 0), alpha(&face, 0, 0)),
            ([0x24, 0x24, 0x29], 255)
        );
        assert_eq!(rgb(&face, 50, 20), [0xe6, 0x00, 0x12], "the accent bar");
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
        assert_eq!(rgb(&top, 0, 0), [0x24, 0x24, 0x29]);
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
        assert_eq!(rgb(&first, 0, 0), [0x24, 0x24, 0x29]);
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
