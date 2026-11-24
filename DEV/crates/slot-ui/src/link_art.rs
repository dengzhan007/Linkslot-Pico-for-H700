use slot_i18n::Language;

use crate::art::render_svg;
use crate::cart::CartFace;
use crate::text;

const PORT_SVG: &str = include_str!("../assets/link_port.svg");
const PLUG_HOST_SVG: &str = include_str!("../assets/link_plug_host.svg");
const PLUG_JOIN_SVG: &str = include_str!("../assets/link_plug_join.svg");
const ADAPTER_SVG: &str = include_str!("../assets/link_adapter.svg");
const ROUTER_SVG: &str = include_str!("../assets/link_router.svg");
const HOTSPOT_HOST_SVG: &str = include_str!("../assets/link_hotspot_host.svg");
const HOTSPOT_JOIN_SVG: &str = include_str!("../assets/link_hotspot_join.svg");
const WIFI_HOST_SVG: &str = include_str!("../assets/link_wifi_host.svg");
const WIFI_JOIN_SVG: &str = include_str!("../assets/link_wifi_join.svg");

pub const PORT_W: u32 = 720;
pub const PORT_H: u32 = 92;
pub const PORT_Y: f32 = 388.0;
pub const PLUG_W: u32 = 92;
pub const PLUG_H: u32 = 244;
pub const PLUG_TIP_X: f32 = 46.0;
pub const MODE_ART_W: u32 = 440;
pub const MODE_ART_H: u32 = 260;
pub const ROUTER_W: u32 = 360;
pub const ROUTER_H: u32 = 260;
pub const ADAPTER_W: u32 = 268;
pub const ADAPTER_H: u32 = 182;
pub const ADAPTER_BASE_X: f32 = 134.0;
pub const ADAPTER_BASE_Y: f32 = 144.0;
pub const ARCS: [(f32, f32, u32, u32); 3] = [
    (508.0, 290.0, 16, 56),
    (530.0, 274.0, 20, 88),
    (552.0, 258.0, 25, 120),
];
pub const CLICKS_X: f32 = 270.0;
pub const CLICKS_Y: f32 = 360.0;
pub const CLICKS_W: u32 = 180;
pub const CLICKS_H: u32 = 30;
pub const ARROW_W: u32 = 16;
pub const ARROW_H: u32 = 22;
pub const ARROW_LEFT_X: f32 = 268.0;
pub const ARROW_RIGHT_X: f32 = 436.0;
pub const ARROW_Y: f32 = 52.0;

const ADAPTER_SCALE: f32 = 1.75;
const LABEL_INK: [u8; 3] = [0xec, 0xee, 0xef];
const LOGO_INK: [u8; 3] = [0x7d, 0x85, 0x8e];
const ARC_INK: &str = "#f6f4ef";

pub struct LinkArt {
    pub port: CartFace,
    pub plug_host: CartFace,
    pub plug_join: CartFace,
    pub adapter: CartFace,
    pub router: CartFace,
    pub hotspot_host: CartFace,
    pub hotspot_join: CartFace,
    pub wifi_host: CartFace,
    pub wifi_join: CartFace,
    pub wifi_line_host: CartFace,
    pub wifi_line_join: CartFace,
    pub hotspot_arcs_cyan: [CartFace; 3],
    pub hotspot_arcs_amber: [CartFace; 3],
    pub hotspot_arcs_amber_solid: [CartFace; 3],
    pub ring_cyan: CartFace,
    pub ring_amber: CartFace,
    pub dot_cyan: CartFace,
    pub dot_amber: CartFace,
    pub check: CartFace,
    pub arcs_right: [CartFace; 3],
    pub arcs_left: [CartFace; 3],
    pub clicks: CartFace,
    pub arrow_left: CartFace,
    pub arrow_right: CartFace,
}

pub fn link_art(lang: Language) -> LinkArt {
    let arcs_right = [arc_face(0), arc_face(1), arc_face(2)];
    let arcs_left = [
        mirror(&arcs_right[0]),
        mirror(&arcs_right[1]),
        mirror(&arcs_right[2]),
    ];
    LinkArt {
        port: svg_face(PORT_SVG, PORT_W, PORT_H),
        plug_host: svg_face(PLUG_HOST_SVG, PLUG_W, PLUG_H),
        plug_join: svg_face(PLUG_JOIN_SVG, PLUG_W, PLUG_H),
        adapter: adapter_face(lang),
        router: svg_face(ROUTER_SVG, ROUTER_W, ROUTER_H),
        hotspot_host: svg_face(HOTSPOT_HOST_SVG, MODE_ART_W, MODE_ART_H),
        hotspot_join: svg_face(HOTSPOT_JOIN_SVG, MODE_ART_W, MODE_ART_H),
        wifi_host: svg_face(WIFI_HOST_SVG, MODE_ART_W, MODE_ART_H),
        wifi_join: svg_face(WIFI_JOIN_SVG, MODE_ART_W, MODE_ART_H),
        wifi_line_host: wifi_line_face("#4fc3f7", true),
        wifi_line_join: wifi_line_face("#f2b84b", false),
        hotspot_arcs_cyan: [
            hotspot_arc_face("#4fc3f7", true, 0, false),
            hotspot_arc_face("#4fc3f7", true, 1, false),
            hotspot_arc_face("#4fc3f7", true, 2, false),
        ],
        hotspot_arcs_amber: [
            hotspot_arc_face("#f2b84b", false, 0, true),
            hotspot_arc_face("#f2b84b", false, 1, true),
            hotspot_arc_face("#f2b84b", false, 2, true),
        ],
        hotspot_arcs_amber_solid: [
            hotspot_arc_face("#f2b84b", false, 0, false),
            hotspot_arc_face("#f2b84b", false, 1, false),
            hotspot_arc_face("#f2b84b", false, 2, false),
        ],
        ring_cyan: ring_face("#4fc3f7"),
        ring_amber: ring_face("#f2b84b"),
        dot_cyan: dot_face("#4fc3f7"),
        dot_amber: dot_face("#f2b84b"),
        check: check_face(),
        arcs_right,
        arcs_left,
        clicks: svg_face(
            &format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {CLICKS_W} {CLICKS_H}"><g stroke="{ARC_INK}" stroke-width="4" stroke-linecap="round"><path d="M34 20 L20 4"/><path d="M146 20 L160 4"/><path d="M26 26 L4 26"/><path d="M154 26 L176 26"/></g></svg>"#
            ),
            CLICKS_W,
            CLICKS_H,
        ),
        arrow_left: svg_face(&arrow_svg("M14 2 L2 11 L14 20 Z"), ARROW_W, ARROW_H),
        arrow_right: svg_face(&arrow_svg("M2 2 L14 11 L2 20 Z"), ARROW_W, ARROW_H),
    }
}


fn wifi_line_face(colour: &str, host: bool) -> CartFace {
    let (x1, y1, x2, y2) = if host {
        (108.0, 142.0, 158.0, 132.0)
    } else {
        (332.0, 142.0, 282.0, 132.0)
    };
    svg_face(
        &format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 440 260"><path d="M{x1} {y1} L{x2} {y2}" stroke="{colour}" stroke-width="8" stroke-linecap="round" opacity=".22"/><path d="M{x1} {y1} L{x2} {y2}" stroke="{colour}" stroke-width="3" stroke-linecap="round"/></svg>"#
        ),
        MODE_ART_W,
        MODE_ART_H,
    )
}

fn ring_face(colour: &str) -> CartFace {
    svg_face(
        &format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><circle cx="32" cy="32" r="28" fill="none" stroke="{colour}" stroke-width="3"/></svg>"#
        ),
        64,
        64,
    )
}

fn dot_face(colour: &str) -> CartFace {
    svg_face(
        &format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><circle cx="8" cy="8" r="5" fill="{colour}"/></svg>"#
        ),
        16,
        16,
    )
}

fn check_face() -> CartFace {
    svg_face(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 40 40"><circle cx="20" cy="20" r="18" fill="#40dc9f"/><path d="M11 21 L17 27 L30 13" fill="none" stroke="#0a281c" stroke-width="3.2" stroke-linecap="round" stroke-linejoin="round"/></svg>"##,
        40,
        40,
    )
}

fn hotspot_arc_face(colour: &str, host: bool, ring: usize, dashed: bool) -> CartFace {
    let (rx, ry) = [(68.0f32, 34.0f32), (120.0, 60.0), (172.0, 86.0)][ring];
    let (cx, cy) = if host { (110.0, 124.0) } else { (332.0, 124.0) };
    let a = 48.0f32.to_radians();
    let (dx, dy) = (rx * a.cos(), ry * a.sin());
    let dash = if dashed && !host { r#" stroke-dasharray="9 7""# } else { "" };
    let (x0, x1) = if host {
        (cx + dx, cx + dx)
    } else {
        (cx - dx, cx - dx)
    };
    let (y0, y1) = (cy - dy, cy + dy);
    let sweep = if host { 1 } else { 1 };
    svg_face(
        &format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 440 260"><path d="M{x0} {y0} A{rx} {ry} 0 0 {sweep} {x1} {y1}" fill="none" stroke="{colour}" stroke-width="3.5" stroke-linecap="round"{dash}/></svg>"#
        ),
        MODE_ART_W,
        MODE_ART_H,
    )
}

fn svg_face(svg: &str, w: u32, h: u32) -> CartFace {
    let rgba = render_svg(svg, w, h).unwrap_or_else(|| vec![0; (w * h * 4) as usize]);
    CartFace { rgba, w, h }
}

fn arrow_svg(path: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {ARROW_W} {ARROW_H}"><path d="{path}" fill="{ARC_INK}"/></svg>"#
    )
}

fn arc_face(ring: usize) -> CartFace {
    let (_, _, w, h) = ARCS[ring];
    let bulge = [16.0, 24.0, 34.0][ring];
    let path = format!("M4 4 Q{} {} 4 {}", 4.0 + bulge, h as f32 / 2.0, h - 4);
    svg_face(
        &format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}"><path d="{path}" fill="none" stroke="{ARC_INK}" stroke-width="5" stroke-linecap="round"/></svg>"#
        ),
        w,
        h,
    )
}

fn mirror(face: &CartFace) -> CartFace {
    let mut rgba = vec![0u8; face.rgba.len()];
    for y in 0..face.h {
        for x in 0..face.w {
            let s = ((y * face.w + x) * 4) as usize;
            let d = ((y * face.w + (face.w - 1 - x)) * 4) as usize;
            rgba[d..d + 4].copy_from_slice(&face.rgba[s..s + 4]);
        }
    }
    CartFace {
        rgba,
        w: face.w,
        h: face.h,
    }
}

#[derive(Copy, Clone)]
enum Align {
    Left,
    Centre,
    Right,
}

type LabelRow = (f32, f32, &'static str, f32, [u8; 3], Align);

fn adapter_face(lang: Language) -> CartFace {
    let mut face = svg_face(ADAPTER_SVG, ADAPTER_W, ADAPTER_H);
    let at = |x: f32, y: f32| {
        (
            ADAPTER_BASE_X + x * ADAPTER_SCALE,
            ADAPTER_BASE_Y + y * ADAPTER_SCALE,
        )
    };
    let s = ADAPTER_SCALE;
    let rows: [LabelRow; 6] = [
        (0.0, -58.9, "SLOT", 5.6 * s, LOGO_INK, Align::Centre),
        (
            0.0,
            -23.4,
            slot_i18n::t(lang, "device.app.link_art.adapter"),
            5.2 * s,
            LABEL_INK,
            Align::Centre,
        ),
        (
            -37.0,
            -15.8,
            slot_i18n::t(lang, "device.app.link_art.radio"),
            3.4 * s,
            LABEL_INK,
            Align::Left,
        ),
        (
            -37.0,
            -11.8,
            slot_i18n::t(lang, "device.app.link_art.model"),
            3.4 * s,
            LABEL_INK,
            Align::Left,
        ),
        (
            -37.0,
            -7.8,
            slot_i18n::t(lang, "device.app.link_art.made_in"),
            2.7 * s,
            LABEL_INK,
            Align::Left,
        ),
        (
            37.0,
            -7.8,
            "S/LOT-A-WA-USA",
            2.7 * s,
            LABEL_INK,
            Align::Right,
        ),
    ];
    for (x, baseline, line, px, ink, align) in rows {
        let (fx, fy) = at(x, baseline);
        stamp(&mut face, fx, fy, line, px, ink, align);
    }
    face
}

fn stamp(
    face: &mut CartFace,
    x: f32,
    baseline: f32,
    line: &str,
    px: f32,
    ink: [u8; 3],
    align: Align,
) {
    let Some(font) = text::label_font() else {
        return;
    };
    let layout = text::fit(font, line, f32::MAX, 1, px, px);
    let width = layout
        .lines
        .first()
        .map(|l| text::line_width(font, l, px, layout.tracking))
        .unwrap_or(0.0);
    let (bw, bh) = ((width.ceil() as u32).max(1), (px * 1.6).ceil() as u32);
    let cov = text::coverage(bw, bh, &layout);
    let left = match align {
        Align::Left => x,
        Align::Centre => x - bw as f32 / 2.0,
        Align::Right => x - bw as f32,
    }
    .round() as i32;
    let top = (baseline - px * 1.2).round() as i32;
    for row in 0..bh as i32 {
        for col in 0..bw as i32 {
            let a = cov[(row as u32 * bw + col as u32) as usize] as u32;
            let (dx, dy) = (left + col, top + row);
            if a == 0 || dx < 0 || dy < 0 || dx >= face.w as i32 || dy >= face.h as i32 {
                continue;
            }
            let d = ((dy as u32 * face.w + dx as u32) * 4) as usize;
            for (k, c) in ink.iter().enumerate() {
                face.rgba[d + k] =
                    ((*c as u32 * a + face.rgba[d + k] as u32 * (255 - a)) / 255) as u8;
            }
            face.rgba[d + 3] = face.rgba[d + 3].max(a as u8);
        }
    }
}
