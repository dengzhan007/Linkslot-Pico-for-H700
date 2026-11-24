mod common;

use std::path::PathBuf;

use slot::app::App;
use slot_input::{Action, Btn};
use slot_store::{write_slot_state, Cart, Platform, Recent, SlotState};

fn cart(platform: Platform, stem: &str, folder: &str) -> Cart {
    let extension = platform.extensions()[0];
    let rom = if folder.is_empty() {
        PathBuf::from(format!("Games/{}/{stem}.{extension}", platform.dir_name()))
    } else {
        PathBuf::from(format!(
            "Games/{}/{folder}/{stem}.{extension}",
            platform.dir_name()
        ))
    };
    Cart {
        platform,
        stem: stem.to_string(),
        rom,
        label: None,
        title: stem.to_uppercase(),
        code: String::new(),
        shell: None,
        model: Default::default(),
        front: None,
    }
}

/// GBA: one root ROM plus three browse folders; GB adds a second platform.
fn library() -> Vec<Cart> {
    vec![
        cart(Platform::Gba, "阿拉丁", "动作"),
        cart(Platform::Gba, "龙珠", ""),
        cart(Platform::Gba, "合金弹头", "动作/横版"),
        cart(Platform::Gba, "黄金太阳", "角色扮演"),
        cart(Platform::Gb, "俄罗斯方块", "益智"),
    ]
}

fn app() -> App {
    App::new(library())
}

fn tap(app: &mut App, button: Btn) {
    app.apply(Action::GbaDown(button));
    app.apply(Action::GbaUp(button));
}

fn enter_folder_mode(app: &mut App) {
    app.apply(Action::GbaDown(Btn::Select));
    app.update(1.01);
    assert!(
        app.folder_mode(),
        "the SELECT hold did not enter folder mode"
    );
    // Let the 450 ms header slide finish before the next key.
    app.update(0.6);
}

#[test]
fn holding_select_for_a_second_enters_the_current_carts_folder() {
    let mut app = app();
    assert_eq!(app.selected_stem(), Some("阿拉丁"));
    assert!(!app.folder_mode());

    app.apply_at(Action::GbaDown(Btn::Select), 0);
    app.update(0.2);
    let hold = app.folder_hold_prompt().expect("no progress bar");
    assert!(hold.progress > 0.0 && hold.progress < 1.0);
    assert!(!app.folder_mode(), "a short hold changed the mode");
    app.apply_at(Action::GbaUp(Btn::Select), 200);
    app.update(0.04);
    let fading = app.folder_hold_prompt().expect("the released bar vanished");
    assert!(fading.alpha < 1.0, "the released bar did not fade");
    app.update(0.1);

    app.apply(Action::GbaDown(Btn::Select));
    app.update(1.01);
    let header = app.folder_header().expect("folder mode has no header");
    assert_eq!(header.folder, "动作");
    assert_eq!(header.platform, "GBA");
    assert_eq!(header.index, 1, "动作 is the first GBA folder");
    assert_eq!(header.total, 3, "动作/动作/横版/角色扮演");
    assert_eq!(header.game.as_deref(), Some("阿拉丁"));
}

#[test]
fn a_root_cart_enters_the_first_real_subfolder() {
    let mut app = App::new(vec![
        cart(Platform::Gba, "龙珠", ""),
        cart(Platform::Gba, "阿拉丁", "动作"),
    ]);
    // The root cart is selected first, so folder mode has to move into 动作.
    assert_eq!(app.selected_stem(), Some("龙珠"));
    enter_folder_mode(&mut app);
    let header = app.folder_header().expect("no folder header");
    assert_eq!(header.folder, "动作");
    assert_eq!(header.index, 1);
    assert_eq!(header.total, 1);
    assert_eq!(app.selected_stem(), Some("阿拉丁"));
}

#[test]
fn releasing_before_one_second_cancels_the_switch() {
    let mut app = app();
    app.apply_at(Action::GbaDown(Btn::Select), 0);
    app.update(0.6);
    app.apply_at(Action::GbaUp(Btn::Select), 600);
    app.update(0.6);
    assert!(!app.folder_mode(), "a 600 ms hold entered folder mode");
}

#[test]
fn select_plus_start_still_opens_the_core_picker_and_never_toggles_the_folder() {
    let d = common::tmp_root_with_carts(&["Root"]);
    let dir = d.path().join("Games/GBA/动作");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("Sub.gba"), common::gba_rom()).unwrap();
    let mut app = common::boot(d.path());
    app.apply_at(Action::GbaDown(Btn::Select), 0);
    app.update(0.5);
    app.apply_at(Action::GbaDown(Btn::Start), 500);
    app.update(0.6);
    assert!(!app.folder_mode(), "SELECT+START was read as a folder hold");
    assert!(
        app.core_picker().is_some(),
        "SELECT+START stopped opening the core picker"
    );
}

#[test]
fn shoulders_switch_folder_groups_and_not_platforms() {
    let mut app = app();
    enter_folder_mode(&mut app);
    assert_eq!(app.selected_stem(), Some("阿拉丁"));

    tap(&mut app, Btn::R1);
    assert_eq!(app.selected_stem(), Some("合金弹头"));
    assert_eq!(app.folder_header().unwrap().folder, "动作/横版");
    assert_eq!(app.shelf_platform_name(), Some("GBA"));

    tap(&mut app, Btn::L1);
    assert_eq!(app.selected_stem(), Some("阿拉丁"));
    assert_eq!(app.folder_header().unwrap().folder, "动作");
}

#[test]
fn the_gba_groups_finish_before_the_next_platform_begins() {
    let mut app = app();
    enter_folder_mode(&mut app);
    assert_eq!(app.folder_header().unwrap().folder, "动作");

    tap(&mut app, Btn::R1); // 动作/横版
    tap(&mut app, Btn::R1); // 角色扮演
    assert_eq!(app.folder_header().unwrap().platform, "GBA");

    tap(&mut app, Btn::R1); // GB / 益智 (GB has no other folder here)
    let header = app.folder_header().unwrap();
    assert_eq!(header.platform, "GB");
    assert_eq!(header.folder, "益智");
    assert_eq!(header.index, 1);
    assert_eq!(header.total, 1);

    tap(&mut app, Btn::R1); // wrap to GBA / 动作
    let header = app.folder_header().unwrap();
    assert_eq!(header.platform, "GBA");
    assert_eq!(header.folder, "动作");
}

#[test]
fn leaving_folder_mode_restores_platform_switching() {
    let mut app = app();
    enter_folder_mode(&mut app);
    app.apply(Action::GbaDown(Btn::Select));
    app.update(1.01);
    assert!(
        !app.folder_mode(),
        "the second hold did not leave folder mode"
    );
    app.update(0.2);

    tap(&mut app, Btn::R1);
    assert_eq!(
        app.shelf_platform_name(),
        Some("GB"),
        "R1 did not go back to switching platforms"
    );
}

#[test]
fn entering_favorites_leaves_folder_mode() {
    let mut app = app();
    enter_folder_mode(&mut app);
    app.apply(Action::FfStart); // favourite the selected cart
    assert!(app.folder_mode(), "favouriting left folder mode");
    app.apply(Action::RewindStart); // L2 jumps to Favorites
    assert!(!app.folder_mode(), "the Favorites rack kept folder mode");
    assert_eq!(app.shelf_platform_name(), Some("收藏"));
}

#[test]
fn folder_mode_opens_from_the_favorites_rack() {
    let mut app = app();
    app.apply(Action::FfStart); // favourite the selected GBA cart
    app.apply(Action::RewindStart); // L2 jumps to Favorites
    assert_eq!(app.shelf_platform_name(), Some("收藏"));
    assert_eq!(app.selected_stem(), Some("阿拉丁"));

    enter_folder_mode(&mut app);
    assert!(
        app.folder_mode(),
        "holding SELECT on Favorites did not open the folder"
    );
    let header = app.folder_header().expect("no folder header");
    assert_eq!(header.platform, "GBA");
    assert_eq!(header.folder, "动作");
    assert_eq!(app.selected_stem(), Some("阿拉丁"));
}

#[test]
fn search_is_named_after_the_folder() {
    let mut app = app();
    enter_folder_mode(&mut app);
    app.apply(Action::GbaDown(Btn::X));
    let keyboard = app.keyboard.as_ref().expect("search did not open");
    assert!(
        keyboard.title.contains("动作"),
        "the search title did not name the folder: {}",
        keyboard.title
    );
}

#[test]
fn a_platform_with_no_subfolders_refuses_folder_mode() {
    let mut app = App::new(vec![
        cart(Platform::Gba, "龙珠", ""),
        cart(Platform::Gba, "火影", ""),
    ]);
    app.apply(Action::GbaDown(Btn::Select));
    app.update(1.01);
    assert!(!app.folder_mode(), "root-only ROMs entered folder mode");
    assert_eq!(app.toast(), Some(slot_ui::Toast::FolderNone));
}

#[test]
fn a_booted_folder_mode_restores_the_path_and_selection() {
    let d = common::tmp_root_with_carts(&["龙珠"]);
    let dir = d.path().join("Games/GBA/动作");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("阿拉丁.gba"), vec![0u8; 0x100]).unwrap();
    write_slot_state(
        d.path(),
        &SlotState {
            clock_set: true,
            utc_offset_min: 0,
            shelf: Some("gba".into()),
            shelf_cart: Some(Recent {
                platform: Platform::Gba,
                stem: "阿拉丁".into(),
            }),
            folder_mode: true,
            folder_path: Some("动作".into()),
            ..Default::default()
        },
    )
    .unwrap();

    let app = App::boot(d.path());
    assert!(app.folder_mode(), "folder mode was not restored");
    assert_eq!(app.selected_stem(), Some("阿拉丁"));
    let header = app.folder_header().expect("no restored header");
    assert_eq!(header.folder, "动作");
    assert_eq!(header.index, 1, "动作 is the only folder");
    assert_eq!(header.total, 1);
}

#[test]
fn a_missing_saved_folder_falls_back_to_the_first_folder_and_says_so() {
    let d = common::tmp_root_with_carts(&["龙珠"]);
    let dir = d.path().join("Games/GBA/动作");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("阿拉丁.gba"), vec![0u8; 0x100]).unwrap();
    write_slot_state(
        d.path(),
        &SlotState {
            clock_set: true,
            utc_offset_min: 0,
            shelf: Some("gba".into()),
            folder_mode: true,
            folder_path: Some("已经删除".into()),
            ..Default::default()
        },
    )
    .unwrap();

    let app = App::boot(d.path());
    assert!(app.folder_mode(), "a missing path closed folder mode");
    assert_eq!(app.folder_header().unwrap().folder, "动作");
    assert_eq!(app.toast(), Some(slot_ui::Toast::FolderMissing));
}

#[test]
fn entering_and_switching_write_the_folder_to_the_state_file() {
    let d = common::tmp_root_with_carts(&["Root"]);
    let dir = d.path().join("Games/GBA/动作");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("阿拉丁.gba"), common::gba_rom()).unwrap();
    let dir = d.path().join("Games/GBA/动作/横版");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("合金弹头.gba"), common::gba_rom()).unwrap();

    let mut app = common::boot(d.path());
    assert_eq!(app.selected_stem(), Some("阿拉丁"));
    app.apply(Action::GbaDown(Btn::Select));
    app.update(1.01);
    app.update(0.6);
    let state = slot_store::read_slot_state(d.path());
    assert!(state.folder_mode);
    assert_eq!(state.folder_path.as_deref(), Some("动作"));

    app.apply(Action::GbaDown(Btn::R1));
    let state = slot_store::read_slot_state(d.path());
    assert_eq!(state.folder_path.as_deref(), Some("动作/横版"));

    app.apply(Action::GbaDown(Btn::Select));
    app.update(1.01);
    app.update(0.6);
    let state = slot_store::read_slot_state(d.path());
    assert!(!state.folder_mode, "leaving the mode was not persisted");
}

#[test]
fn ejecting_a_game_returns_to_the_same_folder_view() {
    let mut app = app();
    enter_folder_mode(&mut app);
    app.apply(Action::Insert);
    app.on_core_ready();
    for _ in 0..180 {
        app.update(1.0 / 60.0);
    }
    assert!(matches!(app.phase(), slot::app::Phase::Playing { .. }));
    app.apply(Action::Eject);
    for _ in 0..240 {
        app.update(1.0 / 60.0);
    }
    assert!(matches!(app.phase(), slot::app::Phase::Shelf));
    assert!(app.folder_mode(), "the folder view was lost on eject");
    assert_eq!(app.selected_stem(), Some("阿拉丁"));
}

#[test]
fn booting_into_a_library_without_folders_returns_to_normal_mode() {
    let d = common::tmp_root_with_carts(&["龙珠", "火影"]);
    write_slot_state(
        d.path(),
        &SlotState {
            clock_set: true,
            utc_offset_min: 0,
            shelf: Some("gba".into()),
            folder_mode: true,
            ..Default::default()
        },
    )
    .unwrap();
    let app = App::boot(d.path());
    assert!(!app.folder_mode(), "folder mode survived an empty catalog");
}

#[test]
fn resuming_a_running_game_does_not_forget_the_folder_mode() {
    let d = common::tmp_root_with_carts(&["龙珠"]);
    let dir = d.path().join("Games/GBA/动作");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("阿拉丁.gba"), common::gba_rom()).unwrap();
    write_slot_state(
        d.path(),
        &SlotState {
            clock_set: true,
            utc_offset_min: 0,
            cart: Some("阿拉丁".into()),
            cart_platform: Some(Platform::Gba),
            shelf: Some("gba".into()),
            folder_mode: true,
            folder_path: Some("动作".into()),
            ..Default::default()
        },
    )
    .unwrap();

    let app = App::boot(d.path());
    assert!(
        app.folder_mode(),
        "resuming a game wrote folder_mode=false before the restore"
    );
    assert_eq!(app.folder_header().unwrap().folder, "动作");
}
