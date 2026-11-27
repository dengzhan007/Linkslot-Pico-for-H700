use slot_gfx::{Draw, TexId};
use slot_model::Battery;

use crate::footer::Printed;
use crate::plate::HINT_H;

pub const BOLT_PX: f32 = 18.0;

pub const GAUGE_W: f32 = 20.0;
pub const GAUGE_H: f32 = 10.0;
const _: () = assert!(GAUGE_H < GAUGE_W);
const NUB_W: f32 = 2.0;
const NUB_H: f32 = 4.0;
pub const WALL: f32 = 1.2;
const RADIUS: f32 = 2.0;
const GAP: f32 = 7.0;

fn ink() -> [f32; 4] {
    let c = crate::switch_style::ink();
    [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0, 1.0]
}

/// Width `draw_gauge` occupies, including the charging bolt's slot, the gauge,
/// the nub, and the percentage when one is printed.
pub fn gauge_width(percent: Printed) -> f32 {
    let base = GAUGE_W + NUB_W;
    if percent.w > 0 {
        base + GAP + percent.w as f32
    } else {
        base
    }
}

pub fn draw_gauge(
    x: f32,
    y: f32,
    battery: Option<Battery>,
    percent: Printed,
    _bolt: Option<TexId>,
    out: &mut Vec<Draw>,
) {
    let Some(b) = battery else {
        return;
    };

    let rect = |x: f32, y: f32, w: f32, h: f32, out: &mut Vec<Draw>| {
        out.push(Draw::Rect {
            x,
            y,
            w,
            h,
            colour: ink(),
        });
    };

    let cx = x;

    // Rounded battery shell: the top/bottom rails are inset by the corner
    // radius, and the side rails by the same amount, leaving the corners
    // transparent instead of drawing a hard square.
    rect(cx + RADIUS, y, GAUGE_W - 2.0 * RADIUS, WALL, out);
    rect(
        cx + RADIUS,
        y + GAUGE_H - WALL,
        GAUGE_W - 2.0 * RADIUS,
        WALL,
        out,
    );
    rect(cx, y + RADIUS, WALL, GAUGE_H - 2.0 * RADIUS, out);
    rect(
        cx + GAUGE_W - WALL,
        y + RADIUS,
        WALL,
        GAUGE_H - 2.0 * RADIUS,
        out,
    );
    // One-pixel shoulder blocks soften the four corners.
    rect(cx + RADIUS - 1.0, y + 1.0, WALL, 1.0, out);
    rect(cx + GAUGE_W - RADIUS - WALL + 1.0, y + 1.0, WALL, 1.0, out);
    rect(cx + RADIUS - 1.0, y + GAUGE_H - 2.0, WALL, 1.0, out);
    rect(
        cx + GAUGE_W - RADIUS - WALL + 1.0,
        y + GAUGE_H - 2.0,
        WALL,
        1.0,
        out,
    );
    // The positive terminal: two small steps instead of a square tab.
    rect(
        cx + GAUGE_W,
        y + (GAUGE_H - NUB_H) / 2.0 + 1.0,
        NUB_W,
        NUB_H - 2.0,
        out,
    );

    let inner = GAUGE_W - 4.0 * WALL;
    let fill = inner * f32::from(b.percent.min(100)) / 100.0;
    if fill > 0.0 {
        rect(
            cx + 2.0 * WALL,
            y + 2.0 * WALL,
            fill,
            GAUGE_H - 4.0 * WALL,
            out,
        );
    }

    if percent.w > 0 {
        let px = cx + GAUGE_W + NUB_W + GAP;
        let py = y + (GAUGE_H - HINT_H as f32) / 2.0;
        out.push(match percent.face {
            Some(tex) => Draw::Tex {
                x: px,
                y: py,
                w: percent.w as f32,
                h: HINT_H as f32,
                tex,
                alpha: 1.0,
            },
            None => Draw::Rect {
                x: px,
                y: py,
                w: percent.w as f32,
                h: HINT_H as f32,
                colour: [1.0, 1.0, 1.0, 0.08],
            },
        });
    }
}
