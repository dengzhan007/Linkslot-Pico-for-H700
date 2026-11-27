use slot_i18n::Language;

use crate::cart::CartFace;
use crate::plate::UndoFace;
use crate::switch_style;
use crate::text;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum PowerChoice {
    Restart,
    PowerOff,
    Repair,
}

impl PowerChoice {
    pub const ALL: [PowerChoice; 3] = [
        PowerChoice::Restart,
        PowerChoice::PowerOff,
        PowerChoice::Repair,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn text(self, lang: Language) -> &'static str {
        match self {
            PowerChoice::Restart => slot_i18n::t(lang, "device.app.power.choice.restart"),
            PowerChoice::PowerOff => slot_i18n::t(lang, "device.app.power.choice.power_off"),
            PowerChoice::Repair => slot_i18n::t(lang, "device.app.power.choice.repair"),
        }
    }

    pub fn glyph(self) -> char {
        match self {
            PowerChoice::Restart => '\u{f01e}',
            PowerChoice::PowerOff => '\u{f011}',
            PowerChoice::Repair => '\u{f0ad}',
        }
    }

    pub fn icon_text(self, lang: Language) -> &'static str {
        match self {
            PowerChoice::Restart => slot_i18n::t(lang, "device.app.power.choice.restart_short"),
            PowerChoice::PowerOff => slot_i18n::t(lang, "device.app.power.choice.power_off_short"),
            PowerChoice::Repair => slot_i18n::t(lang, "device.app.power.choice.repair_short"),
        }
    }
}

pub const POWER_CARD_W: u32 = 400;
pub const POWER_CARD_H: u32 = 48;
pub const POWER_PANEL_W: u32 = 440;
pub const POWER_PANEL_H: u32 = 292;

pub fn power_panel_face() -> UndoFace {
    switch_style::rounded_face(
        POWER_PANEL_W,
        POWER_PANEL_H,
        12.0,
        switch_style::surface(),
        Some(switch_style::surface_border()),
    )
}

pub fn power_card_face(selected: bool) -> UndoFace {
    if selected {
        switch_style::rounded_face(
            POWER_CARD_W,
            POWER_CARD_H,
            11.0,
            switch_style::selected(),
            None,
        )
    } else {
        UndoFace {
            rgba: vec![0u8; (POWER_CARD_W * POWER_CARD_H * 4) as usize],
            w: POWER_CARD_W,
            h: POWER_CARD_H,
        }
    }
}

pub fn power_label_face(choice: PowerChoice, lit: bool, lang: Language) -> UndoFace {
    let colour = if lit {
        switch_style::ink_selected()
    } else {
        switch_style::ink()
    };
    text_face(choice.text(lang), colour)
}

pub fn power_icon_face(choice: PowerChoice, lit: bool, _lang: Language) -> CartFace {
    let colour = if lit {
        switch_style::ink_selected()
    } else {
        switch_style::ink_dim()
    };
    let mut face = UndoFace {
        rgba: vec![0u8; (22 * 22 * 4) as usize],
        w: 22,
        h: 22,
    };
    let mut pen = |x: i32, y: i32| {
        if (0..22).contains(&x) && (0..22).contains(&y) {
            let at = ((y as u32 * 22 + x as u32) * 4) as usize;
            face.rgba[at..at + 3].copy_from_slice(&colour);
            face.rgba[at + 3] = 255;
        }
    };
    let line = |pen: &mut dyn FnMut(i32, i32), x0: i32, y0: i32, x1: i32, y1: i32| {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        let (mut x, mut y) = (x0, y0);
        loop {
            pen(x, y);
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    };
    let circle = |pen: &mut dyn FnMut(i32, i32), cx: i32, cy: i32, r: i32| {
        let mut x = r;
        let mut y = 0;
        let mut err = 1 - r;
        while x >= y {
            for (dx, dy) in [(x, y), (y, x), (-y, x), (-x, y), (-x, -y), (-y, -x), (y, -x), (x, -y)] {
                pen(cx + dx, cy + dy);
            }
            y += 1;
            if err < 0 {
                err += 2 * y + 1;
            } else {
                x -= 1;
                err += 2 * (y - x) + 1;
            }
        }
    };
    match choice {
        PowerChoice::Restart => {
            circle(&mut pen, 11, 11, 7);
            line(&mut pen, 11, 4, 11, 11);
            line(&mut pen, 7, 4, 11, 4);
        }
        PowerChoice::PowerOff => {
            line(&mut pen, 11, 3, 11, 11);
            let mut x = 7;
            let mut y = 4;
            while y <= 17 {
                line(&mut pen, x, y, x, y);
                x = (x + 1).min(15);
                y += 1;
            }
            line(&mut pen, 5, 7, 5, 13);
            line(&mut pen, 17, 7, 17, 13);
            line(&mut pen, 5, 13, 8, 17);
            line(&mut pen, 17, 13, 14, 17);
            line(&mut pen, 8, 17, 14, 17);
        }
        PowerChoice::Repair => {
            line(&mut pen, 5, 4, 17, 4);
            line(&mut pen, 5, 4, 5, 18);
            line(&mut pen, 17, 4, 17, 18);
            line(&mut pen, 5, 18, 17, 18);
            line(&mut pen, 8, 8, 14, 8);
            line(&mut pen, 8, 11, 14, 11);
            line(&mut pen, 8, 14, 12, 14);
        }
    }
    let face = crate::plate::upscale_nearest(&face, 30, 30);
    CartFace {
        rgba: face.rgba,
        w: face.w,
        h: face.h,
    }
}

pub fn power_title_face(lang: Language) -> UndoFace {
    switch_style::title_face(slot_i18n::t(lang, "device.app.power.title"), 26.0)
}

fn text_face(label: &str, colour: [u8; 3]) -> UndoFace {
    let Some(font) = text::label_font() else {
        return UndoFace {
            rgba: Vec::new(),
            w: 0,
            h: 0,
        };
    };
    let ink = text::line_width(font, label, MENU_PX, 0.0).ceil() as u32;
    let w = ink + 2 * MENU_PAD;
    let mut rgba = vec![0u8; (w * MENU_H * 4) as usize];
    let layout = text::fit(font, label, w as f32, 1, MENU_PX, MENU_MIN_PX);
    text::draw_centred(&mut rgba, w, MENU_H, &layout, colour);
    UndoFace { rgba, w, h: MENU_H }
}

pub(crate) const MENU_PX: f32 = 18.0;
const MENU_MIN_PX: f32 = 11.0;
pub(crate) const MENU_H: u32 = 28;
pub const MENU_PAD: u32 = 0;
pub(crate) const MENU_INK: [u8; 3] = [0xf6, 0xf4, 0xef];

pub fn menu_face(label: &str) -> UndoFace {
    text_face(label, MENU_INK)
}
