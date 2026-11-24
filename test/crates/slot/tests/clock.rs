mod common;

use common::{app_booting_at, tmp_root_with_carts, Clock};
use slot::app::{App, Phase};
use slot_input::{Action, Btn};
use slot_store::{read_slot_state, write_slot_state, SlotState};
use slot_ui::{ClockPicker, Field, QuickRow};
use tempfile::TempDir;

#[test]
fn a_fresh_card_opens_the_shelf_without_a_clock_screen() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let a = App::boot(d.path());
    assert!(
        !matches!(a.phase(), Phase::SetClock { .. }),
        "a fresh card stopped at {:?}",
        a.phase()
    );

    let again = App::boot(d.path());
    assert!(
        !matches!(again.phase(), Phase::SetClock { .. }),
        "the second boot stopped at {:?}",
        again.phase()
    );
}

#[test]
fn confirming_the_settings_clock_marks_the_timezone_choice() {
    let (d, mut a, _clock) = clock_screen();
    a.confirm_clock();
    assert!(
        slot::linkcfg::read_clock_settings(d.path()).timezone_explicit,
        "the settings picker should count as an explicit timezone choice"
    );
}

#[test]
fn a_new_device_falls_back_to_the_build_time() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let (_a, clock) = app_booting_at(d.path(), 0);
    assert_eq!(
        clock.get(),
        slot::build_info::build_epoch(),
        "a dead RTC did not fall back to the firmware build time"
    );
}

#[test]
fn a_valid_platform_clock_is_not_replaced_by_the_build_time() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let valid = 1_786_568_000;
    let (_a, clock) = app_booting_at(d.path(), valid);
    assert_eq!(clock.get(), valid, "a valid RTC was overwritten");
}

#[test]
fn a_seated_cart_is_restored_without_a_clock_screen() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    write_slot_state(
        d.path(),
        &SlotState {
            cart: Some("Emerald".into()),
            ..Default::default()
        },
    )
    .unwrap();
    let a = App::boot(d.path());
    assert!(
        !matches!(a.phase(), Phase::SetClock { .. }),
        "a seated cart was hidden behind the clock screen"
    );
}

#[test]
fn confirming_writes_the_picked_time_to_the_platform() {
    let (_d, mut a, clock) = clock_screen();
    a.apply(Action::GbaDown(Btn::Up));
    a.confirm_clock();
    assert_ne!(clock.get(), 0, "the platform clock was never set");
}

#[test]
fn a_field_wraps_rather_than_running_off_the_end() {
    let mut c = ClockPicker::from_secs(0);
    c.field(Field::Month);
    for _ in 0..13 {
        c.up();
    }
    assert!(
        (1..=12).contains(&c.month()),
        "month {} left the calendar",
        c.month()
    );
}

#[test]
fn february_29_is_reachable_in_a_leap_year_and_not_otherwise() {
    let mut c = ClockPicker::from_ymd(2028, 2, 28);
    c.field(Field::Day);
    c.up();
    assert_eq!(c.day(), 29, "2028 is a leap year");
    let mut c = ClockPicker::from_ymd(2027, 2, 28);
    c.field(Field::Day);
    c.up();
    assert_eq!(c.day(), 1, "2027 has no 29th of February");
}

#[test]
fn a_day_the_new_month_does_not_have_is_pulled_back() {
    let mut c = ClockPicker::from_ymd(2026, 1, 31);
    c.field(Field::Month);
    c.up();
    assert_eq!((c.month(), c.day()), (2, 28));
}

#[test]
fn the_picker_hands_back_utc_rather_than_what_was_typed() {
    let mut c = ClockPicker::from_ymd(2026, 8, 12);
    c.field(Field::Hour);
    for _ in 0..9 {
        c.up();
    }
    let typed = c.secs();
    c.field(Field::Offset);
    for _ in 0..10 {
        c.down();
    }
    assert_eq!(c.offset_min(), -300);
    assert_eq!(
        c.secs(),
        typed + 300 * 60,
        "09:00 local at UTC-5 is 14:00 UTC"
    );
}

#[test]
fn the_offset_steps_in_half_hours_because_real_zones_do() {
    let mut c = ClockPicker::from_ymd(2026, 8, 12);
    c.field(Field::Offset);
    c.up();
    assert_eq!(c.offset_min(), 30);
    c.up();
    assert_eq!(c.offset_min(), 60);
}

#[test]
fn the_offset_stops_at_the_ends_of_the_real_range() {
    let mut c = ClockPicker::from_ymd(2026, 8, 12);
    c.field(Field::Offset);
    for _ in 0..40 {
        c.up();
    }
    assert_eq!(c.offset_min(), 840, "walked past UTC+14");
    for _ in 0..80 {
        c.down();
    }
    assert_eq!(c.offset_min(), -720, "walked past UTC-12");
}

#[test]
fn the_offset_is_part_of_the_line_of_type() {
    let mut c = ClockPicker::from_ymd(2026, 8, 12);
    let before = c.text();
    c.field(Field::Offset);
    c.down();
    assert_ne!(c.text(), before);
    assert!(c.text().contains("-00:30"), "{}", c.text());
}

#[test]
fn confirming_persists_the_offset_and_sets_the_platform_to_utc() {
    let (d, mut a, clock) = clock_screen();
    for _ in 0..8 {
        a.apply(Action::GbaDown(Btn::Right));
    }
    for _ in 0..4 {
        a.apply(Action::GbaDown(Btn::Down));
    }
    a.confirm_clock();
    assert_eq!(read_slot_state(d.path()).utc_offset_min, -420);
    assert_eq!(
        clock.get(),
        AT + 120 * 60,
        "the platform was set to local rather than to utc"
    );
}

#[test]
fn the_wall_clock_is_local_rather_than_the_utc_the_card_keeps() {
    let d = tmp_root_with_carts(&["Emerald"]);
    write_slot_state(
        d.path(),
        &SlotState {
            clock_set: true,
            utc_offset_min: -300,
            ..SlotState::default()
        },
    )
    .unwrap();
    let (a, _clock) = app_booting_at(d.path(), 1_700_000_000);
    assert_eq!(a.wall_secs(), 1_700_000_000 - 300 * 60);
}

#[test]
fn a_clock_that_never_got_set_still_boots_to_the_shelf() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    write_slot_state(
        d.path(),
        &SlotState {
            clock_set: true,
            ..SlotState::default()
        },
    )
    .unwrap();
    let (a, _clock) = app_booting_at(d.path(), 0);
    assert!(
        !matches!(a.phase(), Phase::SetClock { .. }),
        "an unset clock blocked the shelf: {:?}",
        a.phase()
    );
}

#[test]
fn a_clock_that_looks_like_a_real_date_is_not_asked_for_again() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    write_slot_state(
        d.path(),
        &SlotState {
            clock_set: true,
            ..SlotState::default()
        },
    )
    .unwrap();
    let (a, _clock) = app_booting_at(d.path(), 1_786_568_000);
    assert!(
        !matches!(a.phase(), Phase::SetClock { .. }),
        "asked again for a clock that was already right"
    );
}

const AT: i64 = 1_786_568_022;
const OPEN_FOR: i64 = 100;

fn clock_screen() -> (TempDir, App, Clock) {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    write_slot_state(
        d.path(),
        &SlotState {
            clock_set: true,
            utc_offset_min: -300,
            ..SlotState::default()
        },
    )
    .unwrap();
    let (mut a, clock) = app_booting_at(d.path(), AT);
    a.apply(Action::QuickMenu);
    // 日期与时间 lives in 系统设置, so the row is two moves away.
    for _ in 0..QuickRow::SystemSettings.position() {
        a.apply(Action::GbaDown(Btn::Down));
    }
    a.apply(Action::GbaDown(Btn::A));
    for _ in 0..QuickRow::DateTime.position() {
        a.apply(Action::GbaDown(Btn::Down));
    }
    a.apply(Action::GbaDown(Btn::A));
    assert!(
        matches!(a.phase(), Phase::SetClock { .. }),
        "the settings clock was never reached: {:?}",
        a.phase()
    );
    (d, a, clock)
}

#[test]
fn confirming_the_clock_untouched_leaves_it_where_it_is() {
    let (_d, mut a, clock) = clock_screen();
    clock.advance(OPEN_FOR);
    a.apply(Action::GbaDown(Btn::A));
    assert_eq!(
        clock.get(),
        AT + OPEN_FOR,
        "an untouched confirm moved the clock by {} s",
        clock.get() - (AT + OPEN_FOR)
    );
}

#[test]
fn changing_the_minute_moves_the_clock_by_exactly_that_minute() {
    let (_d, mut a, clock) = clock_screen();
    for _ in 0..4 {
        a.apply(Action::GbaDown(Btn::Right));
    }
    assert_eq!(a.picker().expect("on the clock").cursor(), Field::Minute);
    a.apply(Action::GbaDown(Btn::Up));
    clock.advance(OPEN_FOR);
    a.apply(Action::GbaDown(Btn::A));
    assert_eq!(
        clock.get(),
        AT + OPEN_FOR + 60,
        "a minute on moved the clock by {} s",
        clock.get() - (AT + OPEN_FOR)
    );
}

#[test]
fn changing_only_the_offset_moves_utc_by_exactly_that_change() {
    let (d, mut a, clock) = clock_screen();
    let before = a.picker().expect("on the clock").offset_min();
    for _ in 0..5 {
        a.apply(Action::GbaDown(Btn::Right));
    }
    a.apply(Action::GbaDown(Btn::Down));
    clock.advance(OPEN_FOR);
    a.apply(Action::GbaDown(Btn::A));
    assert_eq!(
        clock.get(),
        AT + OPEN_FOR + 30 * 60,
        "half an hour west moved the clock by {} s",
        clock.get() - (AT + OPEN_FOR)
    );
    assert_eq!(
        i64::from(read_slot_state(d.path()).utc_offset_min),
        before - 30,
        "the offset was not saved"
    );
}
