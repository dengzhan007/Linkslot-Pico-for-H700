use slot_store::GbPalette;
use slot_ui::{
    alarm_rect, toast_box, toast_face, toast_rect, Draw, Hud, HudKind, Language, Toast, OUT_H,
    OUT_W, TOAST_TOP,
};

#[test]
fn saving_and_loading_say_which_one_happened() {
    assert_eq!(Toast::StateSaved.text(Language::ZhCn), "即时存档已保存");
    assert_eq!(Toast::StateLoaded.text(Language::ZhCn), "即时存档已读取");
}

#[test]
fn the_link_shortcut_on_the_wrong_core_says_to_switch() {
    assert_eq!(Toast::NeedsGpsp.text(Language::ZhCn), "请切换至 gpSP");
    let f = toast_face(Toast::NeedsGpsp, Language::ZhCn);
    assert!(f.rgba.chunks(4).any(|p| p[3] > 0), "the banner is blank");
}

#[test]
fn a_cart_gpsp_cannot_link_says_there_is_no_link() {
    assert_eq!(Toast::NoLink.text(Language::ZhCn), "不支持联机");
    let f = toast_face(Toast::NoLink, Language::ZhCn);
    assert!(f.rgba.chunks(4).any(|p| p[3] > 0), "the banner is blank");
}

#[test]
fn the_banner_says_what_happened_and_never_what_is_on_screen() {
    let all = Toast::all();
    assert_eq!(
        all[..25],
        [
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
        ],
        "a banner was added or dropped: every face is uploaded by its place in this list"
    );
    let fixed = Toast::all().len() - GbPalette::COUNT;
    assert_eq!(
        all[fixed..],
        GbPalette::all().map(Toast::Palette).collect::<Vec<_>>()[..],
        "the palette banners do not follow the fixed ones in palette order"
    );
    for (i, t) in all.iter().enumerate() {
        assert_eq!(t.index(), i, "{t:?} does not answer to its own place");
        let f = toast_face(*t, Language::ZhCn);
        assert!(
            f.rgba.chunks(4).any(|p| p[3] > 0),
            "{t:?} rastered to a blank banner"
        );
    }
}

#[test]
fn no_toast_is_shrunk_to_fit_its_box() {
    let rows = |t: Toast| {
        let f = toast_face(t, Language::ZhCn);
        let inked: Vec<usize> = (0..f.h as usize)
            .filter(|y| (0..f.w as usize).any(|x| f.rgba[(y * f.w as usize + x) * 4 + 3] > 0))
            .collect();
        let first = *inked.first().expect("the banner is blank");
        let last = *inked.last().expect("the banner is blank");
        (first, last)
    };
    let (top, bottom) = rows(Toast::StateSaved);
    for t in Toast::all()
        .into_iter()
        .filter(|t| !matches!(t, Toast::Palette(_)))
    {
        let (a, b) = rows(t);
        assert!(
            a.abs_diff(top) <= 1 && b.abs_diff(bottom) <= 1,
            "{t:?} sits on rows {a}..{b} where the others sit on {top}..{bottom}, so it was shrunk to fit"
        );
    }
}

#[test]
fn a_toast_fades_on_the_same_curve_as_the_bar() {
    let mut h = Hud::new();
    h.toast(Toast::StateSaved, 1_000);
    assert!(h.toast_visible(2_499));
    assert!(!h.toast_visible(2_500));
}

#[test]
fn saying_the_same_thing_twice_re_shows_it_rather_than_stacking() {
    let mut h = Hud::new();
    h.toast(Toast::StateSaved, 1_000);
    h.toast(Toast::StateSaved, 2_400);
    assert_eq!(h.said(3_400), Some(Toast::StateSaved), "it did not re-show");
    assert_eq!(h.said(3_900), None, "it never faded");
}

#[test]
fn a_second_banner_replaces_the_first() {
    let mut h = Hud::new();
    h.toast(Toast::StateSaved, 1_000);
    h.toast(Toast::LinkEnded, 1_100);
    assert_eq!(h.said(1_200), Some(Toast::LinkEnded));
}

#[test]
fn a_toast_is_centred() {
    let (x, _, w, _) = toast_rect(Toast::StateSaved);
    assert_eq!(x + w / 2.0, OUT_W as f32 / 2.0);
}

#[test]
fn a_toast_carries_its_own_halo() {
    let f = toast_face(Toast::StateSaved, Language::ZhCn);
    let dark = f
        .rgba
        .chunks(4)
        .any(|p| p[3] > 0 && p[0] < 0x40 && p[1] < 0x40 && p[2] < 0x40);
    assert!(dark, "there is nothing dark behind the type");
}

#[test]
fn a_toast_is_a_top_panel_without_a_full_width_plate() {
    let mut h = Hud::new();
    let face = slot_ui::TexId::from_raw(1);
    h.set_toasts(vec![face]);
    h.toast(Toast::StateSaved, 0);

    let (toast, alpha) = h.toast_view(0).expect("the toast disappeared");
    assert_eq!(toast, Toast::StateSaved);
    assert_eq!(alpha, 1.0);
    assert_eq!(h.toast_face(toast), Some(face), "the face was not kept");
    let (x, y, w, th) = toast_rect(Toast::StateSaved);
    assert_eq!(
        y, TOAST_TOP,
        "the toast did not take the row under the status bar"
    );
    assert!(
        y + th < OUT_H as f32 * 0.2,
        "the toast at {y} is not in the top row"
    );
    assert_eq!(x + w / 2.0, OUT_W as f32 / 2.0);
}

#[test]
fn a_toast_and_the_hud_bar_live_in_separate_bands() {
    let mut h = Hud::new();
    h.show(HudKind::Volume, 50, false, 0);
    h.toast(Toast::StateSaved, 0);

    assert!(h.toast_view(0).is_some(), "the toast was dropped");
    assert!(h.bar_alpha(0).is_some(), "the bar disappeared");
    let (_, y, _, _) = toast_rect(Toast::StateSaved);
    assert!(y < OUT_H as f32 / 4.0, "the toast left the top row");

    // The stack sits the bar under the toast row; the HUD just blits at the
    // row the caller hands it.
    let bar_y = y + toast_box(Toast::StateSaved).1 as f32 + 8.0;
    let mut out = Vec::new();
    h.draw_bar_at(h.bar_alpha(0).expect("no bar"), bar_y, &mut out);
    assert!(
        out.iter()
            .any(|d| matches!(d, Draw::Rect { y, .. } if (*y - bar_y).abs() < 0.01)),
        "the bar did not take the row it was given"
    );
}

#[test]
fn a_palette_banner_names_the_palette_without_its_boot_combo() {
    let p = GbPalette::parse("GBC 深绿 →A").unwrap();
    assert_eq!(Toast::Palette(p).text(Language::ZhCn), "GBC 深绿");
}

fn alarm_panel(h: &Hud) -> Option<(f32, f32, [f32; 4])> {
    if !h.alarmed() {
        return None;
    }
    let (_, at, want_w, want_h) = alarm_rect();
    let mut out = Vec::new();
    h.draw_alarm_at(at, &mut out);
    out.iter().rev().find_map(|d| match d {
        Draw::Rect {
            y, w, h, colour, ..
        } if (*w - want_w).abs() < 0.01 && (*h - want_h).abs() < 0.01 => {
            Some((*y, *h, *colour))
        }
        _ => None,
    })
}

#[test]
fn the_card_alarm_stays_up_until_it_is_cleared() {
    let mut h = Hud::new();
    h.set_alarm(true, 1_000);
    assert!(
        alarm_panel(&h).is_some(),
        "the alarm went away by itself"
    );
    h.set_alarm(false, 3_600_001);
    assert!(!h.alarmed(), "the alarm outlived a write that landed");
}

#[test]
fn the_card_alarm_is_a_flat_top_panel() {
    let mut h = Hud::new();
    h.set_alarm(true, 0);
    let (y, bh, _) = alarm_panel(&h).expect("no alarm panel");
    assert!(
        y + bh < OUT_H as f32 * 0.25,
        "the alarm panel is not in the top notification stack"
    );
}

#[test]
fn a_second_failure_does_not_restart_the_panel() {
    let mut h = Hud::new();
    h.set_alarm(true, 0);
    h.set_alarm(true, 250);
    let mut fresh = Hud::new();
    fresh.set_alarm(true, 0);
    assert_eq!(alarm_panel(&h), alarm_panel(&fresh));
}

#[test]
fn the_card_alarm_text_is_large_and_inside_its_band() {
    let (x, y, w, h) = alarm_rect();
    assert_eq!(
        (w, h),
        (slot_ui::notice::NOTICE_W, slot_ui::notice::NOTICE_H)
    );
    assert!(
        y + h < OUT_H as f32 * 0.25,
        "the alarm text is not in the top notification stack"
    );
    assert_eq!(x + w / 2.0, OUT_W as f32 / 2.0);
}

#[test]
fn the_card_alarm_says_what_failed_and_what_it_costs() {
    assert_eq!(
        slot_i18n::t(Language::ZhCn, "device.toast.card_unwritable"),
        "无法写入存储卡"
    );
    assert_eq!(
        slot_i18n::t(Language::ZhCn, "device.toast.card_unwritable.detail"),
        "当前进度未能保存"
    );
}

#[test]
fn the_card_alarm_is_a_large_two_line_notice() {
    let f = slot_ui::notice::notice_face(
        "无法写入存储卡",
        "当前进度未能保存",
        None,
        Language::ZhCn,
    );
    assert_eq!(
        (f.w, f.h),
        (
            slot_ui::notice::NOTICE_W as u32,
            slot_ui::notice::NOTICE_H as u32
        )
    );
    assert!(f.rgba.iter().any(|a| *a > 0));
}

#[test]
fn the_result_and_status_use_the_small_notification_shell() {
    let result = slot_ui::small_result_face("已拉取版本 7");
    let status = slot_ui::small_status_face("正在同步");
    assert_eq!((result.w, result.h), (378, 40), "the result is not the small shell");
    assert_eq!((status.w, status.h), (378, 40), "the status is not the small shell");
    assert!(result.rgba.iter().any(|a| *a > 0), "the result drew nothing");
    assert!(status.rgba.iter().any(|a| *a > 0), "the status drew nothing");
}