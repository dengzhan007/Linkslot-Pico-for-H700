use slot_store::{Theme, ThemePreset};
use slot_ui::{set_theme, switch_style};

#[test]
fn the_paper_palette_reaches_the_shared_menu_surface() {
    set_theme(Theme::preset(ThemePreset::Paper));
    assert_eq!(switch_style::bg(), [0xef, 0xe9, 0xdc, 0xff]);
    assert_eq!(switch_style::surface(), [0xf8, 0xf3, 0xe7, 0xff]);
    assert_eq!(switch_style::accent(), [0x7a, 0x56, 0x20]);
}

#[test]
fn the_paper_palette_keeps_body_and_selected_text_readable() {
    let theme = Theme::preset(ThemePreset::Paper);
    let luminance = |c: [u8; 3]| 0.2126 * c[0] as f32 + 0.7152 * c[1] as f32 + 0.0722 * c[2] as f32;
    let bright = |a: [u8; 3], b: [u8; 3]| (luminance(a) - luminance(b)).abs();
    assert!(bright(theme.paper_ink, [theme.paper[0], theme.paper[1], theme.paper[2]]) > 80.0);
    assert!(
        bright(
            theme.ink_selected,
            [theme.selected[0], theme.selected[1], theme.selected[2]]
        ) > 80.0
    );
}

#[test]
fn the_warm_palette_keeps_selected_cards_readable() {
    let theme = Theme::preset(ThemePreset::Warm);
    let ink = theme.ink_selected;
    let selected = theme.selected;
    let luminance = |c: [u8; 3]| 0.2126 * c[0] as f32 + 0.7152 * c[1] as f32 + 0.0722 * c[2] as f32;
    assert!(
        (luminance(ink) - luminance([selected[0], selected[1], selected[2]])).abs() > 80.0,
        "selected text and card are too close"
    );
}
