//! Phase 1 global achievements are a sidecar: they never enter the save
//! whitelist/meta, they sync even when the save command itself is a noop, and
//! a failure on the achievements file is a warning, not a save-sync error.

mod support;

use linkslot::achievements::Unlocks;
use linkslot::output::ResultKind;
use linkslot::sync::PushOptions;
use support::*;

fn write_local_unlocks(harness: &Harness, unlocks: &Unlocks) {
    let path = harness.root.join("linkSlot/Achievements/unlocks.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, unlocks.to_bytes().unwrap()).unwrap();
}

fn cloud_unlocks(harness: &Harness) -> Option<Unlocks> {
    let path = format!(
        "{}/achievements.json",
        harness.config.webdav().remote_root.trim_end_matches('/')
    );
    harness.dav.file(&path).and_then(|bytes| Unlocks::parse(&bytes))
}

fn unlocks(id: &str, at: u64) -> Unlocks {
    let mut value = Unlocks {
        schema: linkslot::achievements::SCHEMA,
        updated_at: at,
        ..Unlocks::default()
    };
    value.unlocks.insert(id.to_string(), at);
    value
}

#[test]
fn push_uploads_global_unlocks_as_a_sidecar() {
    let harness = Harness::new("ach-push");
    let unit = harness.unit();
    harness.seed_local(&unit, &[("resume.state", b"ach-resume".as_slice())]);
    write_local_unlocks(&harness, &unlocks("native:first-launch", 100));

    let result = harness.push(PushOptions::default());
    assert_eq!(result.outcome.result(), ResultKind::Ok, "{}", result.stderr);

    let cloud = cloud_unlocks(&harness).expect("achievements.json uploaded");
    assert_eq!(cloud.unlocks.get("native:first-launch"), Some(&100));

    // Never part of the save whitelist/meta.
    let meta = linkslot::meta::Meta::parse(
        &harness
            .dav
            .file(&harness.remote_meta_path(&unit))
            .expect("meta uploaded"),
    )
    .unwrap();
    assert!(!meta.files.contains_key("achievements.json"));
}

#[test]
fn per_game_unlocks_sync_to_the_game_folder() {
    let harness = Harness::new("ach-game");
    let unit = harness.unit();
    harness.seed_local(&unit, &[("resume.state", b"ach-game-resume".as_slice())]);
    let path = harness
        .root
        .join("linkSlot/Achievements/games/FC/deadbeef.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        &path,
        br#"{"schema":1,"platform":"FC","sha256":"deadbeef","unlocks":{"local:look-up":100},"updated_at":100}"#,
    )
    .unwrap();

    let result = harness.push(PushOptions::default());
    assert!(
        matches!(result.outcome.result(), ResultKind::Ok | ResultKind::Noop),
        "{}",
        result.stderr
    );
    let cloud_path = format!(
        "{}/FC/deadbeef/achievements.json",
        harness.config.webdav().remote_root.trim_end_matches('/')
    );
    let cloud = linkslot::achievements::GameUnlocks::parse(
        &harness.dav.file(&cloud_path).expect("game achievements uploaded"),
    )
    .expect("cloud game achievements parse");
    assert_eq!(cloud.unlocks.get("local:look-up"), Some(&100));
    assert_eq!(cloud.platform, "FC");
    assert_eq!(cloud.sha256, "deadbeef");
}

#[test]
fn an_empty_local_record_pulls_the_cloud_copy() {
    let harness = Harness::new("ach-restore");
    let unit = harness.unit();
    harness.seed_local(&unit, &[("resume.state", b"ach-restore".as_slice())]);
    let local_path = harness
        .root
        .join("linkSlot/Achievements/games/FC/deadbeef.json");
    std::fs::create_dir_all(local_path.parent().unwrap()).unwrap();
    std::fs::write(
        &local_path,
        br#"{"schema":1,"platform":"FC","sha256":"deadbeef","unlocks":{},"updated_at":0}"#,
    )
    .unwrap();
    let cloud_path = format!(
        "{}/FC/deadbeef/achievements.json",
        harness.config.webdav().remote_root.trim_end_matches('/')
    );
    harness.dav.insert_file(
        &cloud_path,
        br#"{"schema":1,"platform":"FC","sha256":"deadbeef","unlocks":{"local:look-up":100},"updated_at":100}"#,
    );

    let result = harness.push(PushOptions::default());
    assert!(
        matches!(result.outcome.result(), ResultKind::Ok | ResultKind::Noop),
        "{}",
        result.stderr
    );
    let local = linkslot::achievements::GameUnlocks::parse(&std::fs::read(&local_path).unwrap())
        .expect("local record parses");
    assert_eq!(local.unlocks.get("local:look-up"), Some(&100));
}

#[test]
fn a_failed_achievement_upload_keeps_the_save_result() {
    let harness = Harness::new("ach-fail");
    let unit = harness.unit();
    harness.seed_local(&unit, &[("resume.state", b"ach-fail-resume".as_slice())]);
    write_local_unlocks(&harness, &unlocks("native:first-launch", 100));
    harness
        .dav
        .fail_path("PUT", "achievements.json", 500, 1);

    let result = harness.push(PushOptions::default());
    assert!(
        matches!(result.outcome.result(), ResultKind::Ok | ResultKind::Noop),
        "achievements must not change the save result: {}",
        result.stderr
    );
    assert!(
        result.stderr.contains("成就同步失败"),
        "the failure is reported as a warning: {}",
        result.stderr
    );
    assert!(cloud_unlocks(&harness).is_none());

    // The next push retries and succeeds.
    let again = harness.push(PushOptions::default());
    assert!(
        matches!(again.outcome.result(), ResultKind::Ok | ResultKind::Noop),
        "{}",
        again.stderr
    );
    assert!(cloud_unlocks(&harness).is_some());
}
