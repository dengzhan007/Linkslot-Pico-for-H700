use slot_ui::{
    ease, Draw, Millis, TexId, ADAPTER_BASE_X, ADAPTER_BASE_Y, ARCS, ARROW_LEFT_X, ARROW_RIGHT_X,
    ARROW_Y, CLICKS_X, CLICKS_Y, OUT_W, PLUG_H, PLUG_TIP_X, PORT_Y,
};

use crate::app::{GameMenu, LinkRow};
use crate::link_start::LinkTransport;
use crate::link_kind::LinkKind;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Sprite {
    pub tex: TexId,
    pub w: u32,
    pub h: u32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct LinkSprites {
    pub port: Sprite,
    pub plug_host: Sprite,
    pub plug_join: Sprite,
    pub adapter: Sprite,
    pub router: Sprite,
    pub hotspot_host: Sprite,
    pub hotspot_join: Sprite,
    pub wifi_host: Sprite,
    pub wifi_join: Sprite,
    pub wifi_line_host: Sprite,
    pub wifi_line_join: Sprite,
    pub hotspot_arcs_cyan: [Sprite; 3],
    pub hotspot_arcs_amber: [Sprite; 3],
    pub hotspot_arcs_amber_solid: [Sprite; 3],
    pub ring_cyan: Sprite,
    pub ring_amber: Sprite,
    pub dot_cyan: Sprite,
    pub dot_amber: Sprite,
    pub check: Sprite,
    pub arcs_right: [Sprite; 3],
    pub arcs_left: [Sprite; 3],
    pub clicks: Sprite,
    pub arrow_left: Sprite,
    pub arrow_right: Sprite,
}

const PICK_TIP: f32 = 330.0;
const SEATED_TIP: f32 = 419.0;
const FAILED_TIP: f32 = 276.0;
const FAILED_TURN_DEG: f32 = 14.0;
const FAILED_ALPHA: f32 = 0.45;
const DROP_MS: Millis = 200;
const SEAT_MS: Millis = 160;
const LIFT_MS: Millis = 250;
const UNPLUG_MS: Millis = 260;
const PICK_BASE: f32 = 336.0;
const ARC_MS: f32 = 1200.0;
const ARC_STAGGER_MS: f32 = 300.0;
const ARROW_ALPHA: f32 = 0.7;

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    if t >= 1.0 {
        b
    } else {
        a + (b - a) * t
    }
}

fn eased(now: Millis, since: Millis, ms: Millis) -> f32 {
    ease((now.saturating_sub(since) as f32 / ms as f32).clamp(0.0, 1.0))
}

fn working_tip(since: Millis, now: Millis) -> f32 {
    lerp(PICK_TIP, SEATED_TIP, eased(now, since, DROP_MS))
}

fn working_base(since: Millis, now: Millis) -> f32 {
    lerp(PICK_BASE, PORT_Y, eased(now, since, DROP_MS))
}

fn working_arcs(since: Millis, now: Millis) -> [f32; 3] {
    let t = now.saturating_sub(since);
    let mut out = [0.0; 3];
    if t < DROP_MS {
        return out;
    }
    for (i, a) in out.iter_mut().enumerate() {
        let u = (t - DROP_MS) as f32 - ARC_STAGGER_MS * i as f32;
        if u >= 0.0 {
            let phase = (u / ARC_MS).fract();
            *a = 1.0 - (2.0 * phase - 1.0).abs();
        }
    }
    out
}

pub fn plug_tip(menu: GameMenu, now: Millis) -> f32 {
    match menu {
        GameMenu::Pick(_) => PICK_TIP,
        GameMenu::Working { since, .. } => working_tip(since, now),
        GameMenu::Linked { worked, since, .. } => lerp(
            working_tip(worked, since),
            SEATED_TIP,
            eased(now, since, SEAT_MS),
        ),
        GameMenu::Failed { worked, since, .. } => lerp(
            working_tip(worked, since),
            FAILED_TIP,
            eased(now, since, LIFT_MS),
        ),
        GameMenu::Unplug { since, .. } => lerp(SEATED_TIP, PICK_TIP, eased(now, since, UNPLUG_MS)),
    }
}

pub fn plug_turn(menu: GameMenu, now: Millis) -> f32 {
    match menu {
        GameMenu::Failed { since, .. } => FAILED_TURN_DEG.to_radians() * eased(now, since, LIFT_MS),
        _ => 0.0,
    }
}

pub fn art_alpha(menu: GameMenu, now: Millis) -> f32 {
    match menu {
        GameMenu::Failed { since, .. } => lerp(1.0, FAILED_ALPHA, eased(now, since, LIFT_MS)),
        _ => 1.0,
    }
}

pub fn adapter_base(menu: GameMenu, now: Millis) -> f32 {
    match menu {
        GameMenu::Pick(_) => PICK_BASE,
        GameMenu::Working { since, .. } => working_base(since, now),
        GameMenu::Linked { worked, since, .. } => lerp(
            working_base(worked, since),
            PORT_Y,
            eased(now, since, SEAT_MS),
        ),
        GameMenu::Failed { worked, since, .. } => working_base(worked, since),
        GameMenu::Unplug { since, .. } => lerp(PORT_Y, PICK_BASE, eased(now, since, UNPLUG_MS)),
    }
}

pub fn arc_alphas(menu: GameMenu, now: Millis) -> [f32; 3] {
    match menu {
        GameMenu::Pick(_) => [0.0; 3],
        GameMenu::Working { since, .. } => working_arcs(since, now),
        GameMenu::Linked { worked, since, .. } => {
            let from = working_arcs(worked, since);
            let t = eased(now, since, SEAT_MS);
            [
                lerp(from[0], 1.0, t),
                lerp(from[1], 1.0, t),
                lerp(from[2], 1.0, t),
            ]
        }
        GameMenu::Failed { worked, since, .. } => {
            let from = working_arcs(worked, since);
            let fade = 1.0 - eased(now, since, LIFT_MS);
            [from[0] * fade, from[1] * fade, from[2] * fade]
        }
        GameMenu::Unplug { since, .. } => {
            let fade = 1.0 - eased(now, since, UNPLUG_MS);
            [fade; 3]
        }
    }
}

pub fn clicks_alpha(menu: GameMenu, now: Millis) -> f32 {
    match menu {
        GameMenu::Linked { since, .. } if now.saturating_sub(since) >= SEAT_MS => 1.0,
        _ => 0.0,
    }
}

fn role_of(menu: GameMenu) -> LinkRow {
    match menu {
        GameMenu::Pick(role)
        | GameMenu::Working { role, .. }
        | GameMenu::Linked { role, .. }
        | GameMenu::Failed { role, .. }
        | GameMenu::Unplug { role, .. } => role,
    }
}

fn tex(out: &mut Vec<Draw>, s: Sprite, x: f32, y: f32, alpha: f32) {
    out.push(Draw::Tex {
        x: x.round(),
        y: y.round(),
        w: s.w as f32,
        h: s.h as f32,
        tex: s.tex,
        alpha,
    });
}

pub fn draw_link_art_transport(
    menu: GameMenu,
    kind: LinkKind,
    transport: LinkTransport,
    single_machine: bool,
    now: Millis,
    s: &LinkSprites,
    out: &mut Vec<Draw>,
) {
    let host = role_of(menu) == LinkRow::Host;
    let alpha = art_alpha(menu, now);
    let centre = OUT_W as f32 / 2.0;
    if single_machine {
        let art = match (transport, host) {
            (LinkTransport::Hotspot, true) => s.hotspot_host,
            (LinkTransport::Hotspot, false) => s.hotspot_join,
            (LinkTransport::Wifi, true) => s.wifi_host,
            (LinkTransport::Wifi, false) => s.wifi_join,
        };
        // The GB composition is already centred inside its 440px source:
        // the router is at x=220 and the two handhelds balance around it.
        let left = centre - 176.0;
        out.push(Draw::Tex {
            x: left.round(),
            y: 136.0,
            w: 352.0,
            h: 208.0,
            tex: art.tex,
            alpha,
        });
        draw_single_machine_animation(menu, transport, host, now, s, out);
        if matches!(menu, GameMenu::Pick(_)) {
            tex(out, s.arrow_left, ARROW_LEFT_X, ARROW_Y, ARROW_ALPHA);
            tex(out, s.arrow_right, ARROW_RIGHT_X, ARROW_Y, ARROW_ALPHA);
        }
        return;
    }
    // GBA only has cable and wireless adapter; these never use the FC art.
    match kind {
        LinkKind::Cable => {
            let tip = plug_tip(menu, now);
            let plug = if host { s.plug_host } else { s.plug_join };
            let (x, y) = ((centre - PLUG_TIP_X).round(), (tip - PLUG_H as f32).round());
            let turn = plug_turn(menu, now);
            if turn == 0.0 {
                tex(out, plug, x, y, alpha);
            } else {
                out.push(Draw::Turned {
                    x,
                    y,
                    w: plug.w as f32,
                    h: plug.h as f32,
                    tex: plug.tex,
                    alpha,
                    turn,
                });
            }
            tex(out, s.port, 0.0, PORT_Y, 1.0);
            let clicks = clicks_alpha(menu, now);
            if clicks > 0.0 {
                tex(out, s.clicks, CLICKS_X, CLICKS_Y, clicks);
            }
        }
        LinkKind::Wireless => {
            let base = adapter_base(menu, now);
            tex(
                out,
                s.adapter,
                centre - ADAPTER_BASE_X,
                base - ADAPTER_BASE_Y,
                alpha,
            );
            tex(out, s.port, 0.0, PORT_Y, 1.0);
            let dy = base - PORT_Y;
            for (i, a) in arc_alphas(menu, now).into_iter().enumerate() {
                if a <= 0.0 {
                    continue;
                }
                let (x, y, w, _) = ARCS[i];
                tex(out, s.arcs_right[i], x, y + dy, a);
                tex(out, s.arcs_left[i], OUT_W as f32 - x - w as f32, y + dy, a);
            }
        }
    }
    if matches!(menu, GameMenu::Pick(_)) {
        tex(out, s.arrow_left, ARROW_LEFT_X, ARROW_Y, ARROW_ALPHA);
        tex(out, s.arrow_right, ARROW_RIGHT_X, ARROW_Y, ARROW_ALPHA);
    }
}

const ART_X: f32 = 160.0;
const ART_Y: f32 = 136.0;
const ART_W: f32 = 352.0;
const ART_H: f32 = 208.0;
const ART_SCALE: f32 = 0.8;
const PULSE_MS: u64 = 180;
const HANDSHAKE_MS: u64 = 520;

fn art_point(x: f32, y: f32) -> (f32, f32) {
    (ART_X + x * ART_SCALE, ART_Y + y * ART_SCALE)
}

fn art_tex(out: &mut Vec<Draw>, s: Sprite, x: f32, y: f32, w: f32, h: f32, alpha: f32) {
    if alpha <= 0.0 {
        return;
    }
    out.push(Draw::Tex {
        x: x.round(),
        y: y.round(),
        w,
        h,
        tex: s.tex,
        alpha: alpha.clamp(0.0, 1.0),
    });
}

fn art_ring(out: &mut Vec<Draw>, s: Sprite, x: f32, y: f32, diameter: f32, alpha: f32) {
    let (cx, cy) = art_point(x, y);
    let d = diameter * ART_SCALE;
    art_tex(out, s, cx - d / 2.0, cy - d / 2.0, d, d, alpha);
}

fn art_dot(out: &mut Vec<Draw>, s: Sprite, x: f32, y: f32, alpha: f32) {
    let (cx, cy) = art_point(x, y);
    let d = 12.0 * ART_SCALE;
    art_tex(out, s, cx - d / 2.0, cy - d / 2.0, d, d, alpha);
}

fn art_check(out: &mut Vec<Draw>, s: Sprite, x: f32, y: f32, alpha: f32) {
    let (cx, cy) = art_point(x, y);
    let d = 34.0 * ART_SCALE;
    art_tex(out, s, cx - d / 2.0, cy - d / 2.0, d, d, alpha);
}

fn draw_single_machine_animation(
    menu: GameMenu,
    transport: LinkTransport,
    host: bool,
    now: Millis,
    s: &LinkSprites,
    out: &mut Vec<Draw>,
) {
    let wifi = transport == LinkTransport::Wifi;
    let (origin_x, origin_y) = if wifi {
        if host { (108.0, 142.0) } else { (332.0, 142.0) }
    } else if host {
        (110.0, 124.0)
    } else {
        (332.0, 124.0)
    };
    let (screen_x, screen_y) = if host { (63.0, 51.0) } else { (376.0, 51.0) };
    let ring = if host { s.ring_cyan } else { s.ring_amber };
    let dot = if host { s.dot_cyan } else { s.dot_amber };
    let (check_x, check_y) = if host { (386.0, 92.0) } else { (412.0, 70.0) };

    let selection = |out: &mut Vec<Draw>, t: f32| {
        let pulse = 30.0 + (t * 10.0).sin() * 7.0;
        art_ring(out, ring, origin_x, origin_y, pulse, 0.30 + (t * 10.0).sin() * 0.10);
        art_ring(out, ring, screen_x, screen_y, 48.0 + (t * 10.0).sin() * 4.0, 0.16);
    };

    match menu {
        GameMenu::Pick(_) => {
            selection(out, now as f32 / 620.0);
        }
        GameMenu::Working { since, .. } => {
            let elapsed = now.saturating_sub(since);
            if elapsed < PULSE_MS {
                selection(out, elapsed as f32 / 55.0);
            } else if elapsed < HANDSHAKE_MS {
                let t = ((elapsed - PULSE_MS) as f32
                    / (HANDSHAKE_MS - PULSE_MS) as f32)
                    .clamp(0.0, 1.0);
                selection(out, elapsed as f32 / 620.0);
                if wifi {
                    let end = if host { (158.0, 132.0) } else { (282.0, 132.0) };
                    // No connection line during handshake: the travelling
                    // packets alone show the attempt.  The line locks only
                    // after the peer is linked.
                    for i in 0..5 {
                        let u = (t * 1.5 - i as f32 * 0.17).clamp(0.0, 1.0);
                        if u > 0.0 && u < 1.0 {
                            art_dot(
                                out,
                                dot,
                                origin_x + (end.0 - origin_x) * u,
                                origin_y + (end.1 - origin_y) * u,
                                0.85,
                            );
                        }
                    }
                } else {
                    let arcs = if host {
                        &s.hotspot_arcs_cyan
                    } else {
                        &s.hotspot_arcs_amber
                    };
                    for (i, arc) in arcs.iter().enumerate() {
                        let a = ((t - i as f32 * 0.16) * 2.8).clamp(0.0, 1.0) * 0.75;
                        art_tex(out, *arc, ART_X, ART_Y, ART_W, ART_H, a);
                    }
                }
            } else if wifi {
                // Matching: the handshake line is gone; only breaths remain.
                let pulse = (now as f32 / 520.0).sin() * 0.5 + 0.5;
                art_ring(out, ring, 220.0, 126.0, 74.0 + pulse * 16.0, 0.22 + pulse * 0.16);
                art_ring(out, ring, 220.0, 126.0, 112.0 + pulse * 8.0, 0.10);
                art_ring(out, ring, origin_x, origin_y, 34.0, 0.20 + pulse * 0.12);
                for i in 0..4 {
                    let on = ((now / 240 + i as u64) % 4) as f32;
                    art_dot(out, dot, 183.0 + i as f32 * 15.0, 129.0, 0.12 + on * 0.18);
                }
            } else {
                let arcs = if host {
                    &s.hotspot_arcs_cyan
                } else {
                    &s.hotspot_arcs_amber
                };
                for (i, arc) in arcs.iter().enumerate() {
                    let pulse = (now as f32 / 520.0 + i as f32 * 0.8).sin() * 0.5 + 0.5;
                    art_tex(out, *arc, ART_X, ART_Y, ART_W, ART_H, 0.22 + pulse * 0.25);
                }
                let pulse = (now as f32 / 520.0).sin() * 0.5 + 0.5;
                art_ring(out, ring, origin_x, origin_y, 36.0 + pulse * 10.0, 0.20);
            }
        }
        GameMenu::Linked { .. } => {
            if wifi {
                let line = if host { s.wifi_line_host } else { s.wifi_line_join };
                art_tex(out, line, ART_X, ART_Y, ART_W, ART_H, 0.95);
                art_ring(out, ring, 220.0, 126.0, 76.0, 0.18);
            } else {
                let arcs = if host {
                    &s.hotspot_arcs_cyan
                } else {
                    &s.hotspot_arcs_amber_solid
                };
                for arc in arcs {
                    art_tex(out, *arc, ART_X, ART_Y, ART_W, ART_H, 0.85);
                }
            }
            art_check(out, s.check, check_x, check_y, 1.0);
        }
        _ => {}
    }
}

/// Compatibility wrapper: existing callers/tests draw the hotspot artwork.
pub fn draw_link_art(
    menu: GameMenu,
    kind: LinkKind,
    now: Millis,
    s: &LinkSprites,
    out: &mut Vec<Draw>,
) {
    draw_link_art_transport(menu, kind, LinkTransport::Hotspot, false, now, s, out);
}