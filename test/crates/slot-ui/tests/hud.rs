use slot_ui::{
    ff_badge, icon_box, icon_face, Badge, Draw, FfState, Hud, HudKind, Icon, LinkBadge, TexId,
    HUD_ICON_PX, LINK_HOST_INK, LINK_JOIN_INK, OUT_H, OUT_W,
};

fn bottom_edge(d: &Draw) -> Option<f32> {
    match *d {
        Draw::Rect { y, h, .. } | Draw::Tex { y, h, .. } | Draw::Turned { y, h, .. } => Some(y + h),
        Draw::Game | Draw::Shot { .. } => None,
    }
}

fn fill(kind: HudKind, value: u8) -> f32 {
    let mut h = Hud::new();
    h.show(kind, value, false, 0);
    let mut out = Vec::new();
    h.draw_bar_at(1.0, 36.0, &mut out);
    match out.last().expect("no bar in the list") {
        Draw::Rect { w, .. } => *w,
        _ => panic!("the bar must be a rect"),
    }
}

#[test]
fn hud_fades_after_1500ms() {
    let mut h = Hud::new();
    h.show(HudKind::Volume, 5, false, 1000);
    assert!(h.visible(2499));
    assert!(!h.visible(2500));
}

#[test]
fn nothing_shows_until_something_is_adjusted() {
    assert!(!Hud::new().visible(0));
}

#[test]
fn volume_reads_against_100_not_the_nine_step_scale() {
    let full = fill(HudKind::Brightness, 9);
    assert!(
        fill(HudKind::Volume, 9) < full * 0.2,
        "volume 9 filled the bar, so it is being read against the brightness scale"
    );
    assert_eq!(fill(HudKind::Volume, 100), full);
}

#[test]
fn the_hud_bar_sits_in_the_top_notification_stack() {
    let mut h = Hud::new();
    h.show(HudKind::Volume, 50, false, 0);
    let mut out = Vec::new();
    h.draw_bar_at(1.0, 36.0, &mut out);
    let lowest = out.iter().filter_map(bottom_edge).fold(0.0f32, f32::max);
    assert!(
        lowest < OUT_H as f32 * 0.3,
        "hud reaches {lowest}, expected the top stack"
    );
}

#[test]
fn the_hud_draws_an_independent_panel() {
    let mut h = Hud::new();
    h.show(HudKind::Brightness, 5, false, 0);
    let mut out = Vec::new();
    h.draw_bar_at(1.0, 36.0, &mut out);
    let panel = out.first().expect("nothing drawn");
    let Draw::Rect { w, h: ph, colour, .. } = panel else {
        panic!("first draw is not the panel")
    };
    assert_eq!(*w, 378.0, "the HUD is still a full-width bar");
    assert_eq!(*ph, 40.0, "the HUD panel is the wrong height");
    assert!(colour[3] > 0.6, "the panel is too transparent");
    assert!(
        !out.iter()
            .any(|d| matches!(d, Draw::Rect { w, .. } if *w == OUT_W as f32)),
        "the HUD still draws the full-width plate"
    );
}

#[test]
fn muted_volume_uses_the_muted_icon() {
    assert_eq!(HudKind::Volume.icon(0, false, false), Icon::VolumeZero);
    assert_eq!(HudKind::Volume.icon(0, true, false), Icon::VolumeMuted);
    assert_eq!(HudKind::Volume.icon(40, true, false), Icon::VolumeMuted);
    assert_eq!(HudKind::Volume.icon(40, false, false), Icon::Volume);
}

#[test]
fn the_rewind_bar_is_held_open_rather_than_fading() {
    let mut h = Hud::new();
    h.show(HudKind::Rewind, 80, false, 0);
    assert!(
        h.visible(60_000),
        "the rewind bar timed out while still held"
    );
    h.release_rewind();
    assert!(!h.visible(60_000));
}

#[test]
fn an_empty_rewind_buffer_still_draws_an_empty_bar() {
    let mut h = Hud::new();
    h.show(HudKind::Rewind, 0, false, 0);
    let mut out = Vec::new();
    h.draw_bar_at(1.0, 36.0, &mut out);
    assert!(
        out.len() >= 2,
        "the track disappeared when the buffer emptied"
    );
}

#[test]
fn releasing_rewind_leaves_a_level_bar_alone() {
    let mut h = Hud::new();
    h.show(HudKind::Volume, 50, false, 0);
    h.release_rewind();
    assert!(h.visible(0));
}

#[test]
fn held_and_latched_fast_forward_are_one_glyph_each_and_differ() {
    let held = ff_badge(FfState::Held).expect("held draws nothing");
    let latched = ff_badge(FfState::Latched).expect("latched draws nothing");
    assert_ne!(
        held, latched,
        "the latch is indistinguishable from the hold"
    );
    assert_eq!(ff_badge(FfState::Off), None);
}

#[test]
fn both_fast_forward_glyphs_share_one_box() {
    let held = icon_face(Icon::FastForward, 18.0, [255, 255, 255]);
    let latched = icon_face(Icon::FastForwardLatched, 18.0, [255, 255, 255]);
    assert_eq!((held.w, held.h), (latched.w, latched.h));
    assert_ne!(
        held.rgba, latched.rgba,
        "the two variants rasterise identically"
    );
}

#[test]
fn the_badge_outlives_the_bar_timer_without_a_plate() {
    let mut h = Hud::new();
    h.set_ff(FfState::Latched);
    let mut out = Vec::new();
    h.draw(&mut out);
    assert!(
        !out.iter()
            .any(|d| matches!(d, Draw::Rect { w, .. } if *w == OUT_W as f32)),
        "the badge dragged the full width plate up with it"
    );
    assert!(
        ff_badge(FfState::Latched).is_some(),
        "the badge itself went away"
    );
}

#[test]
fn the_badge_leaves_when_fast_forward_stops() {
    let mut h = Hud::new();
    h.set_ff(FfState::Held);
    h.set_ff(FfState::Off);
    let mut out = Vec::new();
    h.draw(&mut out);
    assert!(out.is_empty(), "the plate outlived the fast forward");
}

#[test]
fn the_bar_and_badge_draw_no_full_width_plate() {
    let full = |out: &Vec<Draw>| {
        out.iter()
            .filter(|d| matches!(d, Draw::Rect { w, .. } if *w == OUT_W as f32))
            .count()
    };

    let mut badge_only = Hud::new();
    badge_only.set_ff(FfState::Held);
    let mut out = Vec::new();
    badge_only.draw(&mut out);
    assert_eq!(full(&out), 0, "the badge is drawing a full width plate");

    let mut with_bar = Hud::new();
    with_bar.show(HudKind::Volume, 50, false, 0);
    let mut out = Vec::new();
    with_bar.draw_bar_at(1.0, 36.0, &mut out);
    assert_eq!(full(&out), 0, "the HUD bar is still full width");
}

fn badges() -> Vec<TexId> {
    (0..4).map(|i| TexId::from_raw(900 + i)).collect()
}

#[test]
fn the_link_badge_takes_the_fast_forward_corner_without_a_plate() {
    let mut h = Hud::new();
    h.set_link_faces(badges());
    h.set_link(LinkBadge::Hosting);
    let mut out = Vec::new();
    h.draw(&mut out);
    assert_eq!(out.len(), 1, "the badge came with company: {out:?}");
    let (w, _) = icon_box(HUD_ICON_PX);
    assert!(matches!(out[0], Draw::Tex { tex, x, .. }
        if tex == TexId::from_raw(900) && x == OUT_W as f32 - 12.0 - w as f32));
}

#[test]
fn each_link_badge_has_its_own_face_and_off_has_none() {
    assert_eq!(LinkBadge::Off.face_index(), None);
    for (i, b) in LinkBadge::FACES.iter().enumerate() {
        assert_eq!(b.face_index(), Some(i));
    }
    assert_eq!(LinkBadge::HostingLost.colour(), Some(LINK_HOST_INK));
    assert_eq!(LinkBadge::JoinedLost.colour(), Some(LINK_JOIN_INK));
    assert_eq!(LinkBadge::JoinedLost.badge(), Some(Badge::LinkBroken));
    assert_eq!(LinkBadge::Hosting.badge(), Some(Badge::Link));
}

#[test]
fn the_link_badge_outranks_fast_forward() {
    let mut h = Hud::new();
    h.set_icons((0..10).map(TexId::from_raw).collect());
    h.set_link_faces(badges());
    h.set_ff(FfState::Latched);
    h.set_link(LinkBadge::JoinedLost);
    let mut out = Vec::new();
    h.draw(&mut out);
    let texes: Vec<TexId> = out
        .iter()
        .filter_map(|d| match d {
            Draw::Tex { tex, .. } => Some(*tex),
            _ => None,
        })
        .collect();
    assert_eq!(texes, vec![TexId::from_raw(903)]);
}

#[test]
fn volume_on_headphones_shows_headphones() {
    assert_eq!(HudKind::Volume.icon(40, false, true), Icon::Headphones);
    assert_eq!(HudKind::Volume.icon(0, false, true), Icon::Headphones);
    assert_eq!(HudKind::Volume.icon(40, true, true), Icon::HeadphonesMuted);
    assert_eq!(HudKind::Brightness.icon(4, false, true), Icon::Brightness);
}
