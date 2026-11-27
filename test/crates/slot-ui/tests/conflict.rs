use slot_ui::conflict::{
    conflict_face, ConflictDirection, ConflictPage, ConflictUnitView, ConflictView,
};
use slot_ui::{switch_style as sw, Language, UndoFace};

fn unit(version: u32, locked: bool, reason: &str) -> ConflictUnitView {
    ConflictUnitView {
        cloud_line: format!("cloud v{version} 路 RG34XX-A 路 10-22 21:03 路 1 file"),
        local_line: format!("local v{} 路 device 路 10-22 21:12 路 1 file", version + 1),
        reason: reason.to_string(),
        locked,
    }
}

fn view(page: ConflictPage) -> ConflictView {
    let pages = if matches!(page, ConflictPage::All) {
        vec![ConflictPage::All, ConflictPage::State, ConflictPage::Sav]
    } else {
        vec![page]
    };
    ConflictView {
        pages,
        page,
        direction: ConflictDirection::Download,
        state: Some(unit(7, false, "cloud newer")),
        sav: Some(unit(9, false, "cloud differs")),
        locked_state: false,
        locked_sav: false,
        error: None,
        notice: None,
        syncing: false,
    }
}

fn pixel(face: &UndoFace, x: u32, y: u32) -> [u8; 4] {
    let at = ((y * face.w + x) * 4) as usize;
    [
        face.rgba[at],
        face.rgba[at + 1],
        face.rgba[at + 2],
        face.rgba[at + 3],
    ]
}

fn ink(face: &UndoFace) -> usize {
    face.rgba.chunks(4).filter(|p| p[3] > 0).count()
}

#[test]
fn the_conflict_face_is_opaque_and_full_screen() {
    for page in [ConflictPage::All, ConflictPage::State, ConflictPage::Sav] {
        let face = conflict_face(&view(page), Language::ZhCn);
        assert_eq!((face.w, face.h), (720, 480));
        assert_eq!(pixel(&face, 0, 0)[3], 255);
        assert_eq!(pixel(&face, 719, 479)[3], 255);
        assert!(ink(&face) > 1000);
    }
}

#[test]
fn the_dialog_lists_both_directions_with_the_selected_one_white() {
    let face = conflict_face(&view(ConflictPage::All), Language::ZhCn);
    let selected = pixel(&face, 120, 370);
    assert_eq!(
        [selected[0], selected[1], selected[2]],
        [
            sw::selected()[0],
            sw::selected()[1],
            sw::selected()[2]
        ],
        "the selected direction is not a white pill"
    );
    let other = pixel(&face, 600, 370);
    assert_ne!(
        [other[0], other[1], other[2]],
        [
            sw::selected()[0],
            sw::selected()[1],
            sw::selected()[2]
        ],
        "the second direction should stay visibly unselected"
    );
    let border = pixel(&face, 360, 64);
    assert_eq!(
        [border[0], border[1], border[2]],
        [
            sw::surface_border()[0],
            sw::surface_border()[1],
            sw::surface_border()[2]
        ],
        "the modal panel has no 1px border"
    );
}

#[test]
fn the_two_pages_and_the_all_page_render_differently() {
    let all = conflict_face(&view(ConflictPage::All), Language::ZhCn);
    let state = conflict_face(&view(ConflictPage::State), Language::ZhCn);
    let sav = conflict_face(&view(ConflictPage::Sav), Language::ZhCn);
    assert_ne!(all.rgba, state.rgba);
    assert_ne!(all.rgba, sav.rgba);
    assert_ne!(state.rgba, sav.rgba);
}

#[test]
fn a_locked_page_is_dimmed_and_says_so() {
    let mut locked = view(ConflictPage::State);
    locked.locked_state = true;
    let open = conflict_face(&view(ConflictPage::State), Language::ZhCn);
    let shut = conflict_face(&locked, Language::ZhCn);
    assert_ne!(open.rgba, shut.rgba);
}

#[test]
fn the_syncing_state_changes_the_action_line() {
    let mut syncing = view(ConflictPage::State);
    syncing.syncing = true;
    let idle = conflict_face(&view(ConflictPage::State), Language::ZhCn);
    let busy = conflict_face(&syncing, Language::ZhCn);
    assert_ne!(idle.rgba, busy.rgba);
}

#[test]
fn an_error_is_red_and_a_notice_is_drawn() {
    let mut failed = view(ConflictPage::State);
    failed.error = Some("upload failed".to_string());
    let face = conflict_face(&failed, Language::ZhCn);
    assert!(face.rgba.chunks(4).any(|p| {
        p[0] as i32 > p[1] as i32 + 40 && p[0] as i32 > p[2] as i32 + 40
    }));
}

