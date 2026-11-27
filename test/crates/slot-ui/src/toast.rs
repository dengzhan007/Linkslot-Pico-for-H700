use slot_gfx::OUT_W;
use slot_i18n::Language;
use slot_store::GbPalette;

use crate::hud::HUD_INK;
use crate::quick_menu::palette_name;
use crate::switch_style as sw;
use crate::text;
use crate::plate::UndoFace;
use crate::CartFace;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Toast {
    StateSaved,
    StateLoaded,
    NeedsGpsp,
    NoLink,
    /// The selected core (armGBA) has no linkslot-compatible link.
    CoreNoLink,
    /// The extension core disappeared while the picker was open.
    CoreMissing,
    /// An extension pack was installed/removed through the Web page.
    PacksChanged,
    LinkEnded,
    ColourOn,
    ColourOff,
    ShaderOff,
    ShaderLcd3x,
    ShaderGrid,
    ShaderDot,
    ShaderSimpletex,
    ShaderFourPass,
    FavoriteOn,
    FavoriteOff,
    RepairClean,
    RepairDone,
    RepairFailed,
    LogDebug,
    LogDebugFailed,
    Palette(GbPalette),
    Duplicate,
    UpgradeNoPackage,
    UpgradeStarted,
    UpgradeFailed,
    /// A peer joined the game this device started.
    LinkJoined,
    /// This device started a game and the peer has not joined yet.
    LinkWaiting,
    /// Every shortcut went back to its factory chord.
    ShortcutsReset,
    /// The joiner's clock was set from the host's on connect.
    ClockSynced,
    /// The joiner accepted an invite and is switching carts.
    Following,
    /// The host restarted the shared cart; the joiner jumped back with it.
    HostRestarted,
    /// Cartridge transfer mode messages.
    CartTransferWaiting,
    CartTransferReady,
    CartTransferSending,
    CartTransferReceiving,
    CartTransferDone,
    CartTransferFailed,
    CartTransferDuplicate,
    CartTransferAlreadyOwned,
    CartDeleted,
    CartDeleteFailed,
    /// The clock was corrected from an SNTP server after Wi-Fi connected.
    NetworkTimeSynced,
    /// The selected game has no readable TXT guide.
    NoGuide,
    /// A theme was saved; the palette is rebuilt on the next boot.
    ThemeSaved,
    /// Folder browsing was asked for on a platform with no subdirectories.
    FolderNone,
    /// The saved folder no longer exists; the shelf fell back to 全部.
    FolderMissing,
}

impl Toast {
    const FIXED: [Toast; 48] = [
        Toast::StateSaved,
        Toast::StateLoaded,
        Toast::NeedsGpsp,
        Toast::NoLink,
        Toast::CoreNoLink,
        Toast::CoreMissing,
        Toast::PacksChanged,
        Toast::LinkEnded,
        Toast::ColourOn,
        Toast::ColourOff,
        Toast::ShaderOff,
        Toast::ShaderLcd3x,
        Toast::ShaderGrid,
        Toast::ShaderDot,
        Toast::ShaderSimpletex,
        Toast::ShaderFourPass,
        Toast::FavoriteOn,
        Toast::FavoriteOff,
        Toast::RepairClean,
        Toast::RepairDone,
        Toast::RepairFailed,
        Toast::LogDebug,
        Toast::LogDebugFailed,
        Toast::Duplicate,
        Toast::UpgradeNoPackage,
        Toast::UpgradeStarted,
        Toast::UpgradeFailed,
        Toast::LinkJoined,
        Toast::LinkWaiting,
        Toast::ShortcutsReset,
        Toast::ClockSynced,
        Toast::Following,
        Toast::HostRestarted,
        Toast::CartTransferWaiting,
        Toast::CartTransferReady,
        Toast::CartTransferSending,
        Toast::CartTransferReceiving,
        Toast::CartTransferDone,
        Toast::CartTransferFailed,
        Toast::CartTransferDuplicate,
        Toast::CartTransferAlreadyOwned,
        Toast::CartDeleted,
        Toast::CartDeleteFailed,
        Toast::NetworkTimeSynced,
        Toast::NoGuide,
        Toast::ThemeSaved,
        Toast::FolderNone,
        Toast::FolderMissing,
    ];

    pub fn all() -> Vec<Toast> {
        Self::FIXED
            .into_iter()
            .chain(GbPalette::all().map(Toast::Palette))
            .collect()
    }

    pub fn index(self) -> usize {
        match self {
            Toast::Palette(p) => Self::FIXED.len() + p.index(),
            fixed => Self::FIXED.iter().position(|t| *t == fixed).unwrap_or(0),
        }
    }

    pub fn text(self, lang: Language) -> &'static str {
        match self {
            Toast::Palette(p) => palette_name(p, lang),
            Toast::StateSaved => slot_i18n::t(lang, "device.toast.state_saved"),
            Toast::StateLoaded => slot_i18n::t(lang, "device.toast.state_loaded"),
            Toast::NeedsGpsp => slot_i18n::t(lang, "device.toast.needs_gpsp"),
            Toast::NoLink => slot_i18n::t(lang, "device.toast.no_link"),
            Toast::CoreNoLink => slot_i18n::t(lang, "device.toast.core_no_link"),
            Toast::CoreMissing => slot_i18n::t(lang, "device.toast.core_missing"),
            Toast::PacksChanged => slot_i18n::t(lang, "device.toast.packs_changed"),
            Toast::LinkEnded => slot_i18n::t(lang, "device.toast.link_ended"),
            Toast::ColourOn => slot_i18n::t(lang, "device.toast.colour_on"),
            Toast::ColourOff => slot_i18n::t(lang, "device.toast.colour_off"),
            Toast::ShaderOff => slot_i18n::t(lang, "device.toast.shader_off"),
            Toast::ShaderLcd3x => slot_i18n::t(lang, "device.toast.shader_lcd3x"),
            Toast::ShaderGrid => slot_i18n::t(lang, "device.toast.shader_grid"),
            Toast::ShaderDot => slot_i18n::t(lang, "device.toast.shader_dot"),
            Toast::ShaderSimpletex => slot_i18n::t(lang, "device.toast.shader_simpletex"),
            Toast::ShaderFourPass => slot_i18n::t(lang, "device.toast.shader_four_pass"),
            Toast::FavoriteOn => slot_i18n::t(lang, "device.toast.favorite_on"),
            Toast::FavoriteOff => slot_i18n::t(lang, "device.toast.favorite_off"),
            Toast::RepairClean => slot_i18n::t(lang, "device.toast.repair_clean"),
            Toast::RepairDone => slot_i18n::t(lang, "device.toast.repair_done"),
            Toast::RepairFailed => slot_i18n::t(lang, "device.toast.repair_failed"),
            Toast::LogDebug => slot_i18n::t(lang, "device.toast.log_debug"),
            Toast::LogDebugFailed => slot_i18n::t(lang, "device.toast.log_debug_failed"),
            Toast::Duplicate => slot_i18n::t(lang, "device.toast.duplicate"),
            Toast::UpgradeNoPackage => slot_i18n::t(lang, "device.toast.upgrade_no_package"),
            Toast::UpgradeStarted => slot_i18n::t(lang, "device.toast.upgrade_started"),
            Toast::UpgradeFailed => slot_i18n::t(lang, "device.toast.upgrade_failed"),
            Toast::LinkJoined => slot_i18n::t(lang, "device.toast.link_joined"),
            Toast::LinkWaiting => slot_i18n::t(lang, "device.toast.link_waiting"),
            Toast::ShortcutsReset => slot_i18n::t(lang, "device.toast.shortcuts_reset"),
            Toast::ClockSynced => slot_i18n::t(lang, "device.toast.clock_synced"),
            Toast::Following => slot_i18n::t(lang, "device.toast.following"),
            Toast::HostRestarted => slot_i18n::t(lang, "device.toast.host_restarted"),
            Toast::CartTransferWaiting => slot_i18n::t(lang, "device.toast.cart_transfer_waiting"),
            Toast::CartTransferReady => slot_i18n::t(lang, "device.toast.cart_transfer_ready"),
            Toast::CartTransferSending => slot_i18n::t(lang, "device.toast.cart_transfer_sending"),
            Toast::CartTransferReceiving => slot_i18n::t(lang, "device.toast.cart_transfer_receiving"),
            Toast::CartTransferDone => slot_i18n::t(lang, "device.toast.cart_transfer_done"),
            Toast::CartTransferFailed => slot_i18n::t(lang, "device.toast.cart_transfer_failed"),
            Toast::CartTransferDuplicate => slot_i18n::t(lang, "device.toast.cart_transfer_duplicate"),
            Toast::CartTransferAlreadyOwned => slot_i18n::t(lang, "device.toast.cart_transfer_already_owned"),
            Toast::CartDeleted => slot_i18n::t(lang, "device.toast.cart_deleted"),
            Toast::CartDeleteFailed => slot_i18n::t(lang, "device.toast.cart_delete_failed"),
            Toast::NetworkTimeSynced => slot_i18n::t(lang, "device.toast.network_time_synced"),
            Toast::NoGuide => slot_i18n::t(lang, "device.toast.no_guide"),
            Toast::ThemeSaved => slot_i18n::t(lang, "device.toast.theme_saved"),
            Toast::FolderNone => slot_i18n::t(lang, "device.toast.folder_none"),
            Toast::FolderMissing => slot_i18n::t(lang, "device.toast.folder_missing"),
        }
    }
}

const TOAST_W: u32 = 360;
const TOAST_H: u32 = 22;
const TOAST_PX: f32 = 14.0;
const TOAST_MIN_PX: f32 = 11.0;

/// A toast sits directly under the 36 px clock/battery strip: all notifications
/// and the volume/brightness/rewind bar share this top stack.  No notice is
/// left floating over the bottom key legend.
pub const TOAST_TOP: f32 = 36.0;

struct Setting {
    w: u32,
    h: u32,
    px: f32,
    min_px: f32,
}

fn setting(_toast: Toast) -> Setting {
    Setting {
        w: TOAST_W,
        h: TOAST_H,
        px: TOAST_PX,
        min_px: TOAST_MIN_PX,
    }
}

pub fn toast_rect(toast: Toast) -> (f32, f32, f32, f32) {
    let (w, h) = toast_box(toast);
    let (w, h) = (w as f32, h as f32);
    ((OUT_W as f32 - w) / 2.0, TOAST_TOP, w, h)
}

/// The storage-card alarm: a persistent large notification in the top stack.
pub fn alarm_rect() -> (f32, f32, f32, f32) {
    let (w, h) = (crate::notice::NOTICE_W, crate::notice::NOTICE_H);
    ((OUT_W as f32 - w) / 2.0, TOAST_TOP, w, h)
}

const TOAST_PAD: u32 = 9;

pub fn toast_box(toast: Toast) -> (u32, u32) {
    let s = setting(toast);
    (s.w + 2 * TOAST_PAD, s.h + 2 * TOAST_PAD)
}

pub fn toast_face(toast: Toast, lang: Language) -> CartFace {
    let Some(font) = text::label_font() else {
        return CartFace {
            rgba: Vec::new(),
            w: 0,
            h: 0,
        };
    };
    let s = setting(toast);
    let (w, h) = toast_box(toast);
    let mut face = UndoFace { rgba: vec![0u8; (w * h * 4) as usize], w, h };
    sw::fill_round(&mut face, 0.0, 0.0, w as f32, h as f32, 10.0, sw::surface_border());
    sw::fill_round(&mut face, 1.0, 1.0, w as f32 - 2.0, h as f32 - 2.0, 9.0, sw::surface());
    let pad = TOAST_PAD;
    let icon_x = 14.0;
    let icon_y = (h as f32 - 18.0) / 2.0;
    let white = [HUD_INK[0], HUD_INK[1], HUD_INK[2], 255];
    for (dx, dy) in [(2.0, 9.0), (4.0, 11.0), (6.0, 13.0), (8.0, 11.0), (10.0, 9.0), (12.0, 7.0)] {
        kb_fill(&mut face, icon_x + dx, icon_y + dy, 2.0, 2.0, white);
    }
    let text_x = 34.0;
    let text_w = w.saturating_sub(46);
    let layout = text::fit(font, toast.text(lang), text_w as f32, 1, s.px, s.min_px);
    let mut title = UndoFace { rgba: vec![0u8; (text_w * s.h * 4) as usize], w: text_w, h: s.h };
    text::draw_centred(&mut title.rgba, text_w, s.h, &layout, HUD_INK);
    blit_face(&mut face, &title, text_x as u32, pad);
    let (face_w, face_h) = (face.w, face.h);
    CartFace { rgba: face.rgba, w: face_w, h: face_h }
}

/// A sync result on the small 378x40 notification shell: the same card as the
/// transient toasts, with a check and the dynamic result line.
pub fn small_result_face(text: &str) -> CartFace {
    small_face(text, SmallGlyph::Check)
}

/// The persistent "mod is running" status on the small shell: a cartridge and
/// the current stage line.
pub fn small_status_face(text: &str) -> CartFace {
    small_face(text, SmallGlyph::Cart)
}

enum SmallGlyph {
    Check,
    Cart,
}

fn small_face(text: &str, glyph: SmallGlyph) -> CartFace {
    let Some(font) = text::label_font() else {
        return CartFace {
            rgba: Vec::new(),
            w: 0,
            h: 0,
        };
    };
    let (w, h) = (crate::toast::toast_box(Toast::StateSaved).0, crate::toast::toast_box(Toast::StateSaved).1);
    let mut face = UndoFace { rgba: vec![0u8; (w * h * 4) as usize], w, h };
    sw::fill_round(&mut face, 0.0, 0.0, w as f32, h as f32, 10.0, sw::surface_border());
    sw::fill_round(&mut face, 1.0, 1.0, w as f32 - 2.0, h as f32 - 2.0, 9.0, sw::surface());
    let white = [HUD_INK[0], HUD_INK[1], HUD_INK[2], 255];
    // The small shell's left marker, shared with the HUD control bar.
    kb_fill(&mut face, 8.0, 12.0, 3.0, 24.0, white);
    match glyph {
        SmallGlyph::Check => {
            for (dx, dy) in [(2.0, 9.0), (4.0, 11.0), (6.0, 13.0), (8.0, 11.0), (10.0, 9.0), (12.0, 7.0)] {
                kb_fill(&mut face, 14.0 + dx, 11.0 + dy, 2.0, 2.0, white);
            }
        }
        SmallGlyph::Cart => {
            // A tiny cartridge outline: body, label window, bottom notch.
            kb_fill(&mut face, 14.0, 11.0, 12.0, 2.0, white);
            kb_fill(&mut face, 14.0, 11.0, 2.0, 16.0, white);
            kb_fill(&mut face, 24.0, 11.0, 2.0, 16.0, white);
            kb_fill(&mut face, 14.0, 25.0, 12.0, 2.0, white);
            kb_fill(&mut face, 17.0, 15.0, 6.0, 2.0, white);
        }
    }
    let text_w = w.saturating_sub(46);
    let layout = text::fit(font, text, text_w as f32, 1, TOAST_PX, TOAST_MIN_PX);
    let mut title = UndoFace { rgba: vec![0u8; (text_w * 22 * 4) as usize], w: text_w, h: 22 };
    text::draw_centred(&mut title.rgba, text_w, 22, &layout, HUD_INK);
    blit_face(&mut face, &title, 34, TOAST_PAD);
    let (face_w, face_h) = (face.w, face.h);
    CartFace { rgba: face.rgba, w: face_w, h: face_h }
}

fn kb_fill(face: &mut UndoFace, x: f32, y: f32, w: f32, h: f32, colour: [u8; 4]) {    for py in y as u32..(y + h) as u32 {
        for px in x as u32..(x + w) as u32 {
            if px < face.w && py < face.h {
                let at = ((py * face.w + px) * 4) as usize;
                face.rgba[at..at + 4].copy_from_slice(&colour);
            }
        }
    }
}

fn blit_face(dst: &mut UndoFace, src: &UndoFace, x: u32, y: u32) {
    for sy in 0..src.h {
        for sx in 0..src.w {
            let di = (((y + sy) * dst.w + x + sx) * 4) as usize;
            let si = ((sy * src.w + sx) * 4) as usize;
            let a = src.rgba[si + 3] as u32;
            if a == 0 {
                continue;
            }
            for c in 0..3 {
                dst.rgba[di + c] = ((src.rgba[si + c] as u32 * a
                    + dst.rgba[di + c] as u32 * (255 - a)
                    + 127)
                    / 255) as u8;
            }
            dst.rgba[di + 3] = 255;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ink_width(toast: Toast) -> u32 {
        let font = text::label_font().expect("label font");
        let s = setting(toast);
        let layout = text::fit(font, toast.text(Language::ZhCn), s.w as f32, 1, s.px, s.min_px);
        let cov = text::coverage(s.w, s.h, &layout);
        let cols: Vec<u32> = (0..s.w)
            .filter(|x| (0..s.h).any(|y| cov[(y * s.w + x) as usize] > 0))
            .collect();
        match (cols.first(), cols.last()) {
            (Some(a), Some(b)) => b - a + 1,
            _ => 0,
        }
    }

    fn ink_rows(face: &CartFace) -> (u32, u32) {
        let mut first = None;
        let mut last = 0;
        for y in 0..face.h {
            let inked = (0..face.w).any(|x| face.rgba[((y * face.w + x) * 4 + 3) as usize] > 0);
            if inked {
                first.get_or_insert(y);
                last = y;
            }
        }
        (first.unwrap_or(0), last)
    }

    fn ink_height_at(toast: Toast, px: f32) -> u32 {
        let font = text::label_font().expect("label font");
        let layout = text::fit(font, toast.text(Language::ZhCn), TOAST_W as f32, 1, px, px);
        let cov = text::coverage(TOAST_W, TOAST_H, &layout);
        let rows: Vec<u32> = (0..TOAST_H)
            .filter(|y| (0..TOAST_W).any(|x| cov[(y * TOAST_W + x) as usize] > 0))
            .collect();
        match (rows.first(), rows.last()) {
            (Some(a), Some(b)) => b - a + 1,
            _ => 0,
        }
    }

    #[test]
    fn the_ended_line_is_rastered_the_size_the_others_are() {
        let ended = toast_face(Toast::LinkEnded, Language::ZhCn);
        let saved = toast_face(Toast::StateSaved, Language::ZhCn);
        assert_eq!(
            (ended.w, ended.h),
            (saved.w, saved.h),
            "one box holds every banner"
        );
        let (top, bottom) = ink_rows(&ended);
        assert!(bottom > top, "the line rastered to nothing at all");

        let shipped = ink_height_at(Toast::LinkEnded, TOAST_PX);
        let shrunk = ink_height_at(Toast::LinkEnded, TOAST_MIN_PX);
        assert!(
            shipped > shrunk,
            "the shipped line is no taller than the {TOAST_MIN_PX} px fallback: {shipped} against {shrunk}"
        );
        let reference = ink_height_at(Toast::StateSaved, TOAST_PX);
        assert!(
            shipped.abs_diff(reference) <= 1,
            "LINK ENDED is {shipped} px of ink where STATE SAVED is {reference}"
        );
    }

    #[test]
    fn every_toast_is_set_at_full_size() {
        let font = text::label_font().expect("label font");
        for t in Toast::all() {
            let s = setting(t);
            let layout = text::fit(font, t.text(Language::ZhCn), s.w as f32, 1, s.px, s.min_px);
            assert_eq!(
                layout.px,
                s.px,
                "{:?} ({:?}) was shrunk to {} px to fit {}",
                t,
                t.text(Language::ZhCn),
                layout.px,
                s.w
            );
            assert_eq!(layout.lines.len(), 1, "{t:?} wrapped onto a second line");
            let w = ink_width(t);
            assert!(w <= s.w, "{:?} is {w} px wide in a {} px box", t, s.w);
        }
    }
}
