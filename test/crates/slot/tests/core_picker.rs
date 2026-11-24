use slot::core_picker::{
    CorePicker, Outcome, Press, ThreeLayout, CLOSE_MS, HOP_MS, LIFT_MS, OPEN_MS, SLIDE_MS,
};
use slot_store::Core;
use slot_ui::Millis;

/// GBA 的双插槽：mGBA + gpSP。
fn gba_options() -> Vec<Core> {
    vec![Core::Mgba, Core::Gpsp]
}

fn opened(seat: Core, at: Millis) -> CorePicker {
    let mut p = CorePicker::open(seat, gba_options(), at);
    p.start(at);
    p
}

#[test]
fn it_opens_over_the_open_time_and_then_rests() {
    let p = opened(Core::Mgba, 1000);
    assert_eq!(p.openness(1000), 0.0);
    assert_eq!(p.openness(1000 + OPEN_MS / 4), 0.25);
    assert_eq!(p.openness(1000 + OPEN_MS), 1.0);
    assert_eq!(p.openness(1000 + 10 * OPEN_MS), 1.0);
}

#[test]
fn the_chip_starts_seated_in_the_carts_own_core() {
    let chip = opened(Core::Gpsp, 0).chip(0);
    assert_eq!(chip.seated, Some(Core::Gpsp));
    assert_eq!((chip.across, chip.lift, chip.tip), (1.0, 0.0, 0.0));
}

#[test]
fn right_from_mgba_hops_blank_and_lands_named_in_gpsp() {
    let mut p = opened(Core::Mgba, 0);
    assert_eq!(p.press(Press::Right, 500), Outcome::Nothing);
    assert_eq!(p.seat(), Core::Gpsp);

    let mid = p.chip(500 + HOP_MS / 2);
    assert_eq!(mid.seated, None, "the chip wears a name in flight");
    assert!((mid.across - 0.5).abs() < 0.01, "across {}", mid.across);
    assert!((mid.lift - 1.0).abs() < 0.01, "lift {}", mid.lift);
    assert!(mid.tip > 0.0, "a chip moving right should lean right");

    let landed = p.chip(500 + HOP_MS);
    assert_eq!(landed.seated, Some(Core::Gpsp));
    assert_eq!((landed.across, landed.lift), (1.0, 0.0));
}

#[test]
fn toward_the_socket_it_is_in_is_refused_and_only_shakes() {
    let mut p = opened(Core::Gpsp, 0);
    assert_eq!(p.press(Press::Right, 400), Outcome::Refused);
    assert_eq!(p.seat(), Core::Gpsp);
    assert_ne!(p.chip(400).shake, 0.0, "a refusal with no shake");
    assert_eq!(p.chip(400).seated, Some(Core::Gpsp));
    assert_eq!(p.chip(700).shake, 0.0, "the shake outlived its 300 ms");
}

#[test]
fn back_mid_hop_turns_the_chip_round_from_where_it_is() {
    let mut p = opened(Core::Mgba, 0);
    p.press(Press::Right, 400);
    let at = 400 + HOP_MS / 4;
    let before = p.chip(at).across;
    assert_eq!(p.press(Press::Left, at), Outcome::Nothing);
    assert_eq!(p.seat(), Core::Mgba);
    assert!(
        (p.chip(at).across - before).abs() < 0.02,
        "the chip jumped from {before} to {} as it turned",
        p.chip(at).across
    );
    assert_eq!(p.chip(at + HOP_MS).seated, Some(Core::Mgba));
}

#[test]
fn onward_mid_hop_does_nothing_and_is_not_a_refusal() {
    let mut p = opened(Core::Mgba, 0);
    p.press(Press::Right, 400);
    assert_eq!(p.press(Press::Right, 450), Outcome::Nothing);
    assert_eq!(p.chip(450).shake, 0.0);
    assert_eq!(p.seat(), Core::Gpsp);
}

#[test]
fn keep_writes_where_the_chip_is_heading_and_closes() {
    let mut p = opened(Core::Mgba, 0);
    p.press(Press::Right, 400);
    assert_eq!(p.press(Press::Keep, 450), Outcome::Write(Core::Gpsp));
    assert!(p.closing());
    assert!(!p.finished(450));
    assert!(p.finished(450 + CLOSE_MS));
}

#[test]
fn back_closes_without_a_write() {
    let mut p = opened(Core::Mgba, 0);
    assert_eq!(p.press(Press::Back, 400), Outcome::Nothing);
    assert!(p.closing());
    assert!(p.finished(400 + CLOSE_MS));
}

#[test]
fn a_close_during_the_lift_reverses_from_where_the_lid_had_got_to() {
    let mut p = opened(Core::Mgba, 0);
    let at = OPEN_MS / 2;
    let open = p.openness(at);
    p.press(Press::Back, at);
    assert!(
        (p.openness(at) - open).abs() < 1e-6,
        "the lid snapped as the close began"
    );
    assert!(
        p.finished(at + CLOSE_MS),
        "a partial close outlasted a whole one"
    );
}

#[test]
fn a_hop_started_soon_after_a_refusal_does_not_carry_its_shake() {
    let mut p = opened(Core::Gpsp, 0);
    assert_eq!(p.press(Press::Right, 0), Outcome::Refused);
    assert_eq!(p.press(Press::Left, 100), Outcome::Nothing);
    assert_eq!(p.chip(100).shake, 0.0, "the refusal rode along on the hop");
    assert_eq!(p.chip(150).shake, 0.0, "the refusal rode along on the hop");
}

#[test]
fn presses_during_the_close_do_nothing() {
    let mut p = opened(Core::Mgba, 0);
    p.press(Press::Back, 400);
    assert_eq!(p.press(Press::Right, 420), Outcome::Nothing);
    assert_eq!(p.press(Press::Keep, 430), Outcome::Nothing);
    assert_eq!(p.seat(), Core::Mgba);
}

#[test]
fn the_open_is_a_slide_then_a_lift_and_the_board_agrees() {
    assert_eq!(OPEN_MS, SLIDE_MS + LIFT_MS);
    assert!((slot_ui::SLIDE_SHARE - SLIDE_MS as f32 / OPEN_MS as f32).abs() < 1e-6);
}

#[test]
fn a_picker_waits_until_it_is_started() {
    let mut p = CorePicker::open(Core::Mgba, gba_options(), 100);
    assert!(p.waiting());
    assert_eq!(p.openness(5_000), 0.0);
    assert_eq!(p.waited(350), 250);
    p.start(400);
    assert!(!p.waiting());
    assert_eq!(p.openness(400), 0.0);
    assert_eq!(p.openness(400 + OPEN_MS), 1.0);
}

#[test]
fn a_second_start_keeps_the_first() {
    let mut p = CorePicker::open(Core::Mgba, gba_options(), 0);
    p.start(300);
    p.start(500);
    assert_eq!(p.openness(300 + OPEN_MS), 1.0);
}

#[test]
fn backing_out_while_waiting_finishes_at_once() {
    let mut p = CorePicker::open(Core::Mgba, gba_options(), 0);
    p.press(Press::Back, 50);
    assert!(p.finished(50));
}

#[test]
fn a_close_from_rest_takes_the_close_time() {
    let mut p = opened(Core::Mgba, 0);
    p.press(Press::Back, OPEN_MS);
    assert_eq!(p.openness(OPEN_MS + CLOSE_MS / 4), 0.75);
    assert!(p.finished(OPEN_MS + CLOSE_MS));
}

#[test]
fn the_arrows_do_nothing_while_the_picker_waits() {
    let mut p = CorePicker::open(Core::Mgba, gba_options(), 0);
    assert_eq!(p.press(Press::Right, 100), Outcome::Nothing);
    assert_eq!(p.seat(), Core::Mgba, "a hop started while the cart waited");
    let chip = p.chip(100);
    assert_eq!((chip.seated, chip.shake), (Some(Core::Mgba), 0.0));
    assert_eq!(
        p.press(Press::Left, 120),
        Outcome::Nothing,
        "toward its own socket while the cart waited was refused"
    );
    assert_eq!(p.chip(120).shake, 0.0);

    p.start(200);
    assert_eq!(p.press(Press::Right, 300), Outcome::Nothing);
    assert_eq!(
        p.seat(),
        Core::Gpsp,
        "the arrow did nothing once the cart had opened"
    );
    assert_eq!(
        p.chip(300 + HOP_MS / 2).seated,
        None,
        "no hop once the cart had opened"
    );
}

#[test]
fn a_single_core_picker_stays_on_its_one_socket() {
    let mut p = CorePicker::open(Core::Mgba, vec![Core::Mgba], 0);
    p.start(0);
    assert_eq!(p.press(Press::Left, 100), Outcome::Nothing);
    assert_eq!(p.press(Press::Right, 120), Outcome::Nothing);
    assert_eq!(p.seat(), Core::Mgba);
    let chip = p.chip(140);
    assert_eq!(chip.seated, Some(Core::Mgba));
    assert_eq!((chip.across, chip.shake), (0.5, 0.0));

    assert_eq!(p.press(Press::Keep, 200), Outcome::Nothing);
    assert!(p.closing());
}

#[test]
fn a_single_core_picker_can_close_without_writing() {
    let mut p = CorePicker::open(Core::Nes, vec![Core::Nes], 0);
    p.start(0);
    assert_eq!(p.press(Press::Back, 100), Outcome::Nothing);
    assert!(p.closing());
}

#[test]
fn a_sfc_board_can_hold_snes9x_and_the_optional_supafaust() {
    let mut p = CorePicker::open(Core::Snes9x, vec![Core::Snes9x, Core::Snes], 0);
    p.start(0);
    assert_eq!(p.chip(0).across, 0.0, "snes9x starts in the first socket");
    assert_eq!(p.press(Press::Right, 100), Outcome::Nothing);
    assert_eq!(p.seat(), Core::Snes);
    let landed = p.chip(100 + HOP_MS);
    assert_eq!(landed.seated, Some(Core::Snes));
    assert_eq!(landed.across, 1.0, "supafaust sits in the second socket");
    assert_eq!(p.press(Press::Keep, 400), Outcome::Write(Core::Snes));
}

#[test]
fn a_sfc_board_with_only_snes9x_is_an_single_information_board() {
    let p = CorePicker::open(Core::Snes9x, vec![Core::Snes9x], 0);
    assert!(p.is_single());
    let chip = p.chip(0);
    assert_eq!((chip.seated, chip.across), (Some(Core::Snes9x), 0.5));
}

#[test]
fn an_fc_board_can_hold_fceumm_and_the_optional_nestopia() {
    let mut p = CorePicker::open(Core::Nes, vec![Core::Nes, Core::Nestopia], 0);
    p.start(0);
    assert_eq!(p.chip(0).across, 0.0, "fceumm starts in the first socket");
    assert_eq!(p.press(Press::Right, 100), Outcome::Nothing);
    assert_eq!(p.seat(), Core::Nestopia);
    let landed = p.chip(100 + HOP_MS);
    assert_eq!(landed.seated, Some(Core::Nestopia));
    assert_eq!(landed.across, 1.0, "nestopia sits in the second socket");
    assert_eq!(p.press(Press::Keep, 400), Outcome::Write(Core::Nestopia));
}

#[test]
fn an_fc_board_with_only_fceumm_is_a_single_information_board() {
    let p = CorePicker::open(Core::Nes, vec![Core::Nes], 0);
    assert!(p.is_single());
    let chip = p.chip(0);
    assert_eq!((chip.seated, chip.across), (Some(Core::Nes), 0.5));
}

fn three(seat: Core, alt: Option<Core>) -> CorePicker {
    let mut p = CorePicker::open_three(
        seat,
        vec![Core::Mgba, Core::Gpsp, Core::Armgba],
        alt,
        0,
    );
    p.start(0);
    p
}

#[test]
fn a_third_gba_core_is_highlighted_in_the_tray_and_swaps_with_slot_two() {
    let mut p = three(Core::Mgba, Some(Core::Gpsp));
    assert!(!p.is_single());
    assert_eq!(
        p.layout(),
        Some(ThreeLayout {
            slot1: Core::Mgba,
            slot2: Core::Gpsp,
            tray: Core::Armgba,
        })
    );
    assert_eq!((p.selected_pos(), p.seat(), p.active()), (0, Core::Mgba, Core::Mgba));

    assert_eq!(p.press(Press::Right, 100), Outcome::Nothing);
    assert_eq!((p.selected_pos(), p.seat()), (1, Core::Gpsp));
    assert_eq!(p.active(), Core::Mgba, "the highlight does not run the core");
    assert_eq!(p.press(Press::Right, 200), Outcome::Nothing);
    assert_eq!((p.selected_pos(), p.seat()), (2, Core::Armgba));
    assert_eq!(p.active(), Core::Mgba);

    let at = 300;
    assert_eq!(p.press(Press::Keep, at), Outcome::Write(Core::Armgba));
    assert_eq!(
        p.layout(),
        Some(ThreeLayout {
            slot1: Core::Mgba,
            slot2: Core::Armgba,
            tray: Core::Gpsp,
        })
    );
    assert_eq!((p.selected_pos(), p.active()), (1, Core::Armgba));
    assert_eq!(p.swap_progress(at), Some(0.0));
    assert_eq!(p.swap_progress(at + HOP_MS / 2).map(|q| q > 0.0), Some(true));
    assert_eq!(p.swap_progress(at + HOP_MS), None);
    assert!(!p.finished(at + HOP_MS), "the page is still showing the swap");
    assert!(p.finished(at + HOP_MS + CLOSE_MS));
}

#[test]
fn opening_on_the_hot_core_keeps_mgba_in_slot_one() {
    let p = three(Core::Armgba, None);
    assert_eq!(
        p.layout(),
        Some(ThreeLayout {
            slot1: Core::Mgba,
            slot2: Core::Armgba,
            tray: Core::Gpsp,
        })
    );
    assert_eq!((p.selected_pos(), p.seat(), p.active()), (1, Core::Armgba, Core::Armgba));
}

#[test]
fn the_remembered_alternative_is_used_when_the_default_core_is_selected() {
    let p = three(Core::Mgba, Some(Core::Armgba));
    assert_eq!(
        p.layout(),
        Some(ThreeLayout {
            slot1: Core::Mgba,
            slot2: Core::Armgba,
            tray: Core::Gpsp,
        })
    );
    assert_eq!((p.selected_pos(), p.seat()), (0, Core::Mgba));

    let fallback = three(Core::Mgba, None);
    assert_eq!(
        fallback.layout().map(|layout| layout.slot2),
        Some(Core::Gpsp),
        "no remembered alternative should fall back to gpSP"
    );
}

#[test]
fn a_three_core_picker_refuses_the_edges_and_back_does_not_write() {
    let mut p = three(Core::Mgba, Some(Core::Gpsp));
    assert_eq!(p.press(Press::Left, 100), Outcome::Refused);
    assert_ne!(p.chip(100).shake, 0.0);
    assert_eq!(p.press(Press::Right, 200), Outcome::Nothing);
    assert_eq!(p.press(Press::Right, 300), Outcome::Nothing);
    assert_eq!(p.press(Press::Right, 400), Outcome::Refused);
    assert_eq!(p.press(Press::Back, 500), Outcome::Nothing);
    assert!(p.finished(500 + CLOSE_MS));
}

#[test]
fn selecting_a_chip_in_a_socket_writes_it_without_a_swap() {
    let mut p = three(Core::Mgba, Some(Core::Gpsp));
    assert_eq!(p.press(Press::Right, 100), Outcome::Nothing);
    assert_eq!(p.press(Press::Keep, 200), Outcome::Write(Core::Gpsp));
    assert_eq!(p.active(), Core::Gpsp);
    assert_eq!(p.swap_progress(200), None, "a socket chip does not swap");
    assert!(p.finished(200 + CLOSE_MS));
}
