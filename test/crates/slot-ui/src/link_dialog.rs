//! Small modal dialogs: link-session confirmations and destructive-action
//! confirmations.  They are drawn over a dimmed picture, so the alpha channel
//! is part of the texture rather than a solid full-screen face.

use slot_i18n::Language;

use crate::keyboard as kb;
use crate::plate::UndoFace;
use crate::switch_style as sw;
use slot_gfx::{OUT_H, OUT_W};

const DIALOG_W: f32 = 560.0;
const DIALOG_H: f32 = 260.0;

fn face(title: &str, heading: &str, body: &str, hint: &str) -> UndoFace {
    let mut face = kb::blank(OUT_W, OUT_H, [0, 0, 0, 0]);
    kb::fill(
        &mut face,
        0.0,
        0.0,
        OUT_W as f32,
        OUT_H as f32,
        [0, 0, 0, 168],
    );

    let x = ((OUT_W as f32 - DIALOG_W) / 2.0).round();
    let y = ((OUT_H as f32 - DIALOG_H) / 2.0).round();
    sw::fill_round(
        &mut face,
        x,
        y,
        DIALOG_W,
        DIALOG_H,
        sw::DIALOG_RADIUS,
        sw::surface_border(),
    );
    sw::fill_round(
        &mut face,
        x + 1.0,
        y + 1.0,
        DIALOG_W - 2.0,
        DIALOG_H - 2.0,
        (sw::DIALOG_RADIUS - 1.0).max(0.0),
        sw::surface(),
    );
    kb::draw_left(
        &mut face,
        title,
        26.0,
        14.0,
        DIALOG_W - 64.0,
        x + 32.0,
        y + 20.0,
        44.0,
        sw::ink(),
    );
    kb::draw_left(
        &mut face,
        heading,
        20.0,
        12.0,
        DIALOG_W - 96.0,
        x + 48.0,
        y + 74.0,
        30.0,
        sw::ink(),
    );
    kb::draw_wrapped(
        &mut face,
        body,
        16.0,
        11.0,
        x + 48.0,
        y + 112.0,
        DIALOG_W - 96.0,
        58.0,
        3,
        sw::ink_dim(),
    );
    kb::draw_box(
        &mut face,
        hint,
        16.0,
        11.0,
        x + 40.0,
        y + 206.0,
        DIALOG_W - 80.0,
        30.0,
        sw::ink_dim(),
    );
    face
}

/// Confirm ending a live link from a long M press.
pub fn end_link(lang: Language) -> UndoFace {
    face(
        slot_i18n::t(lang, "device.app.link.ending_title"),
        slot_i18n::t(lang, "device.app.link.ending_question"),
        slot_i18n::t(lang, "device.app.link.ending_body"),
        slot_i18n::t(lang, "device.app.link.ending_hint"),
    )
}

/// The second confirmation before a cartridge is deleted.
pub fn cart_delete(lang: Language) -> UndoFace {
    face(
        slot_i18n::t(lang, "device.app.confirm.delete_again_title"),
        slot_i18n::t(lang, "device.app.confirm.delete_again_heading"),
        slot_i18n::t(lang, "device.app.confirm.delete_again_body"),
        slot_i18n::t(lang, "device.app.confirm.delete_again_hint"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_end_link_dialog_is_full_size_and_has_a_card() {
        let confirm = end_link(Language::ZhCn);
        assert_eq!((confirm.w, confirm.h), (OUT_W, OUT_H));
        assert!(
            confirm
                .rgba
                .chunks(4)
                .any(|pixel| pixel[0] > 48 && pixel[1] > 48 && pixel[2] > 48),
            "the end-link dialog has no card pixels"
        );
        // Disconnects no longer own a dialog; they are a notice.
    }

    #[test]
    fn the_cart_delete_dialog_is_full_size_and_has_a_card() {
        let confirm = cart_delete(Language::ZhCn);
        assert_eq!((confirm.w, confirm.h), (OUT_W, OUT_H));
        assert!(
            confirm
                .rgba
                .chunks(4)
                .any(|pixel| pixel[0] > 48 && pixel[1] > 48 && pixel[2] > 48),
            "the delete dialog has no card pixels"
        );
    }
}
