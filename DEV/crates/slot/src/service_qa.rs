//! Phase 6a「服务门」逻辑层的**独立验收**（QA）。
//!
//! 只放测试代码，由 `lib.rs` 的 `#[cfg(test)] mod service_qa;` 挂进来。
//! 覆盖 `crate::service` 的纯面（不碰 `Draw`、不起 `App`）：
//!
//! 1. `Config/arcade.ini` 的路径与"坏文件退默认"；
//! 2. `records_text`/`records_from` 往返，**未知键连 section 一起保留**（重写不许抹掉别人的字段）；
//! 3. `bump_launch` **只动这一块板**（计数与时间都按 set）；
//! 4. `door_opens` 只在 PCB 架（`Main`）；
//! 5. `dirty` **只有一个来源**（`Overview` 的 FREE PLAY），翻页/移焦点/只读页按 A 都不置；
//! 6. 关闭流程：干净直接关、脏了进确认（保存 / 放弃 / 取消三支）。
//!
//! **只能代码审查/真机**（本文件不假装覆盖）：`Toast::ServiceBusy` 的可见性、
//! 门的实际绘制（`draw_service`）、真卡上 `Config/arcade.ini` 的读写、
//! FREE PLAY 保存后跨重启生效。

use std::collections::BTreeMap;
use std::path::Path;

use slot_store::Platform;

use crate::arcade::ArcadeStage;
use crate::service::{
    bump_launch, door_opens, read_records, records_from, records_path, records_text, write_records,
    Records, ServiceConfirm, ServicePage, ServiceState, SetRecord, CONFIRM_CHOICES, SERVICE_PAGES,
};

fn temp_root() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir")
}

fn state(records: Records) -> ServiceState {
    ServiceState::open(records, Platform::Neogeo, "mslug".to_string())
}

// ---------------------------------------------------------------- ① 路径与默认值

/// `Config/arcade.ini` 在卡上的位置。
#[test]
fn the_records_file_lives_under_config() {
    let root = Path::new("/card");
    assert_eq!(
        records_path(root),
        root.join("Config/arcade.ini"),
        "记录文件应当是 <root>/Config/arcade.ini"
    );
}

/// 没有文件、或是垃圾内容时，退回默认值而不是报错（门必须能开）。
#[test]
fn a_missing_or_unparsable_file_falls_back_to_defaults() {
    let dir = temp_root();
    let empty = read_records(dir.path());
    assert!(empty.sets.is_empty(), "没有文件时不该有记录");
    assert!(empty.free_play, "街机默认 FREE PLAY 打开");
    assert!(empty.extras.is_empty());

    // 垃圾内容：坏行忽略，不 panic。
    std::fs::create_dir_all(dir.path().join("Config")).unwrap();
    std::fs::write(dir.path().join("Config/arcade.ini"), "这不是 ini\n\u{0}\u{0}\n").unwrap();
    let broken = read_records(dir.path());
    assert!(broken.free_play, "坏文件也要退回默认 FREE PLAY");
    assert!(broken.sets.is_empty(), "坏文件不该解析出记录");
}

// ---------------------------------------------------------------- ② 往返与未知键

/// 写出去再读回来，已知字段必须一致（`extras` 按 section 归组后顺序可能变，
/// 所以比较时排序）。
#[test]
fn the_records_survive_a_round_trip() {
    let mut records = Records {
        sets: BTreeMap::new(),
        free_play: false,
        extras: vec![
            ("nvram".to_string(), "foo".to_string(), "bar".to_string()),
            ("settings".to_string(), "future_key".to_string(), "1".to_string()),
        ],
    };
    records.sets.insert(
        "mslug".to_string(),
        SetRecord {
            launches: 7,
            last_played: Some("2026-11-24_20-41-03".to_string()),
        },
    );

    let text = records_text(&records);
    assert!(text.starts_with("[records]\n"), "第一个 section 应当是 [records]");
    assert!(text.contains("[settings]\n"), "必须有 [settings] section");
    assert!(text.contains("free_play = 0"), "free_play 应当写成 0");

    let back = records_from(&text);
    assert_eq!(back.free_play, false);
    assert_eq!(back.sets, records.sets, "set 记录应当原样读回");

    let mut got = back.extras.clone();
    got.sort();
    let mut want = records.extras.clone();
    want.sort();
    assert_eq!(got, want, "未知键（含 section）应当原样保留");
}

/// **未知键必须连 section 一起保留**：Phase 7 往这个文件加字段后，旧版本重写不能抹掉。
#[test]
fn unknown_keys_keep_their_section_across_a_rewrite() {
    let dir = temp_root();
    std::fs::create_dir_all(dir.path().join("Config")).unwrap();
    std::fs::write(
        dir.path().join("Config/arcade.ini"),
        "[records]\nmslug.launches = 3\n\n[settings]\nfree_play = 1\nfuture_key = 1\n\n[nvram]\nfoo = bar\n",
    )
    .unwrap();

    // 只改 free_play 再写回：未知键与它们的 section 都必须还在。
    let mut records = read_records(dir.path());
    assert_eq!(records.set("mslug").launches, 3);
    records.free_play = false;
    write_records(dir.path(), &records).unwrap();

    let text = std::fs::read_to_string(dir.path().join("Config/arcade.ini")).unwrap();
    assert!(text.contains("future_key = 1"), "未知键 future_key 被抹掉了：\n{text}");
    assert!(text.contains("foo = bar"), "未知键 foo 被抹掉了：\n{text}");
    assert!(
        text.contains("[nvram]\nfoo = bar"),
        "foo 的 section 归属丢了（不在 [nvram] 下）：\n{text}"
    );
    assert!(text.contains("free_play = 0"), "改动没写进文件：\n{text}");
    assert!(text.contains("mslug.launches = 3"), "旧记录被丢了：\n{text}");
}

/// 调用方只带了 `free_play`（`extras` 为空）时，`write_records` 也必须先读旧文件再合并。
#[test]
fn writing_a_bare_record_set_does_not_flush_the_disk_extras() {
    let dir = temp_root();
    std::fs::create_dir_all(dir.path().join("Config")).unwrap();
    std::fs::write(
        dir.path().join("Config/arcade.ini"),
        "[records]\nsf2.launches = 9\n\n[settings]\nfree_play = 1\nfuture_key = 1\n",
    )
    .unwrap();

    let bare = Records {
        sets: BTreeMap::new(),
        free_play: false,
        extras: Vec::new(),
    };
    write_records(dir.path(), &bare).unwrap();

    let text = std::fs::read_to_string(dir.path().join("Config/arcade.ini")).unwrap();
    assert!(
        text.contains("future_key = 1"),
        "只改 free_play 时把磁盘上的未知键冲掉了：\n{text}"
    );
}

// ---------------------------------------------------------------- ③ 开局计数按 set

/// `bump_launch` 只动被点的那一块板：计数各算各的，时间只更新对应 set。
#[test]
fn bump_launch_touches_only_the_set_it_is_given() {
    let mut records = Records::default();
    bump_launch(&mut records, "mslug", "2026-11-24_20-00-00");
    bump_launch(&mut records, "mslug", "2026-11-24_20-05-00");
    bump_launch(&mut records, "sf2", "2026-11-24_20-10-00");

    assert_eq!(records.set("mslug").launches, 2);
    assert_eq!(
        records.set("mslug").last_played.as_deref(),
        Some("2026-11-24_20-05-00"),
        "mslug 的时间应当是最新那次"
    );
    assert_eq!(records.set("sf2").launches, 1);
    assert_eq!(
        records.set("sf2").last_played.as_deref(),
        Some("2026-11-24_20-10-00")
    );

    // 没碰过的 set 读出来是零值，且不会被写进 map。
    assert_eq!(records.set("dino").launches, 0);
    assert!(records.set("dino").last_played.is_none());
    assert_eq!(records.sets.len(), 2, "只应有被 bump 过的两个 set");
}

// ---------------------------------------------------------------- ④ 门只在 PCB 架能开

/// 只有主界面（PCB 架）能开门：插板动画中、attract 中都不行。
#[test]
fn the_door_opens_only_from_the_pcb_rack() {
    assert!(door_opens(ArcadeStage::Main), "Main 下应当能开服务门");
    assert!(!door_opens(ArcadeStage::Inserting), "插板动画中不该开门");
    assert!(!door_opens(ArcadeStage::Attract), "attract 中不该开门");
}

// ---------------------------------------------------------------- ⑤ dirty 只有一个来源

/// 翻页、移焦点、在只读页按 A 都不许置 `dirty`；只有 `Overview` 的 FREE PLAY 会。
///
/// `dirty` 一旦被别处置真，用户会被无关操作弹"要保存吗"，而保存的其实什么都没改。
#[test]
fn only_the_free_play_row_dirties_the_door() {
    let mut door = state(Records::default());
    assert!(!door.dirty);

    // 把所有页面翻一遍、每页都移焦点、只读页按 A。
    for _ in 0..SERVICE_PAGES.len() {
        assert!(!door.dirty, "翻到 {:?} 时 dirty 变真了", door.page);
        door.move_focus(1);
        door.move_focus(-1);
        if door.page.is_read_only() {
            let leave = door.activate();
            assert!(!leave, "只读页按 A 不该要求离开：{:?}", door.page);
            assert!(!door.dirty, "只读页 {:?} 按 A 把 dirty 置真了", door.page);
        }
        door.next_page();
    }

    // 回 Overview 翻 FREE PLAY：这才是唯一会脏的操作。
    while door.page != ServicePage::Overview {
        door.prev_page();
    }
    let before = door.free_play();
    let leave = door.activate();
    assert!(!leave, "Overview 按 A 只是翻开关，不该离开");
    assert_ne!(door.free_play(), before, "FREE PLAY 没被翻转");
    assert!(door.dirty, "翻了 FREE PLAY 却没置 dirty");
}

/// 只读页没有可执行行；`Overview` 与 `Board` 各一行。
#[test]
fn only_the_editable_pages_have_rows() {
    let mut door = state(Records::default());
    for page in SERVICE_PAGES {
        door.page = page;
        let expected = match page {
            ServicePage::Overview | ServicePage::Board => 1,
            _ => 0,
        };
        assert_eq!(door.rows(), expected, "{page:?} 的行数不对");
        assert_eq!(
            page.is_read_only(),
            expected == 0,
            "{page:?} 的只读标记与行数不一致"
        );
    }
}

/// 页面顺序固定且双向环绕。
#[test]
fn the_pages_follow_the_declared_order_and_wrap() {
    let expected = [
        ServicePage::Overview,
        ServicePage::Board,
        ServicePage::Bios,
        ServicePage::Core,
        ServicePage::Dip,
    ];
    assert_eq!(SERVICE_PAGES, expected, "SERVICE_PAGES 的顺序变了");

    let mut door = state(Records::default());
    for page in expected {
        assert_eq!(door.page, page);
        door.next_page();
    }
    assert_eq!(door.page, ServicePage::Overview, "最后一页往后应回到 Overview");

    door.prev_page();
    assert_eq!(door.page, ServicePage::Dip, "Overview 往前应回到 Dip");
}

// ---------------------------------------------------------------- ⑥ 关闭与确认

/// 干净时按 B 直接关；脏了先进确认，三支（保存 / 放弃 / 取消）行为要分明。
#[test]
fn closing_with_changes_asks_before_saving() {
    let mut door = state(Records::default());
    assert!(door.request_close(), "没有改动时应当直接关");

    // 弄脏。
    assert!(!door.activate(), "Overview 的 A 只是翻开关");
    assert!(door.dirty);
    assert!(!door.request_close(), "有改动时不该直接关");
    assert_eq!(
        door.confirm,
        Some(ServiceConfirm::SaveAndClose),
        "确认框默认应当停在「保存并关闭」"
    );

    // 取消：留在门里，改动还在。
    door.move_confirm(-1); // SaveAndClose -> Cancel（往回一格）
    assert_eq!(door.confirm, Some(ServiceConfirm::Cancel));
    assert_eq!(door.resolve_confirm(), None, "取消应当返回 None（留在门里）");
    assert!(door.dirty, "取消之后 dirty 不该被清");
    assert!(door.confirm.is_none(), "确认框应当已收起");

    // 放弃：不保存、清 dirty。
    assert!(!door.request_close());
    door.move_confirm(1); // SaveAndClose -> Discard（顺序是 SaveAndClose, Discard, Cancel）
    assert_eq!(door.confirm, Some(ServiceConfirm::Discard));
    assert_eq!(door.resolve_confirm(), Some(false), "放弃应当返回 Some(false)");
    assert!(!door.dirty, "放弃之后 dirty 应当清掉");

    // 保存：返回 Some(true)。
    assert!(!door.activate());
    assert!(door.dirty);
    assert!(!door.request_close());
    assert_eq!(
        door.resolve_confirm(),
        Some(true),
        "默认项（SaveAndClose）应当返回 Some(true)"
    );
    assert_eq!(CONFIRM_CHOICES.len(), 3);
}

/// 保存后重读，FREE PLAY 的新值必须落到磁盘（临时目录，真卡读写属真机项）。
#[test]
fn saving_writes_the_free_play_value() {
    let dir = temp_root();
    let mut door = state(Records::default());
    assert!(door.free_play(), "默认 FREE PLAY 打开");
    assert!(!door.activate());
    assert!(!door.free_play(), "翻过之后应当关闭");
    door.save(dir.path());

    let from_disk = read_records(dir.path());
    assert!(!from_disk.free_play, "保存后磁盘上仍是旧值");
    assert!(!door.dirty, "save() 之后 dirty 应当清掉");

    // reload 会把内存里的值退回磁盘。
    let mut door = state(Records::default());
    assert!(!door.activate());
    door.reload(dir.path());
    assert!(!door.free_play(), "reload 应当读回磁盘上的 false");
    assert!(!door.dirty);
}

// ---------------------------------------------------------------- ⑦ 自研 section-aware 解析器（恶意输入）

/// 重复 section、同一 key 出现两次：后面的值生效，且不 panic。
#[test]
fn the_parser_takes_the_last_value_when_a_key_repeats() {
    let records = records_from(
        "[records]\nmslug.launches = 3\n\n[records]\nmslug.launches = 5\n\n[settings]\nfree_play = 1\nfree_play = 0\n",
    );
    assert_eq!(
        records.set("mslug").launches,
        5,
        "同 key 重复出现时应当以后者为准"
    );
    assert!(!records.free_play, "free_play 重复时也应以后者为准");
}

/// 值里带 `=`：只按**第一个** `=` 切分，值原样保留（URL、base64、表达式都会被写坏）。
#[test]
fn a_value_containing_equals_is_kept_whole() {
    let records = records_from("[nvram]\ncmd = a=b=c\n");
    let extra = records
        .extras
        .iter()
        .find(|(_, key, _)| key == "cmd")
        .expect("cmd 应当被保留");
    assert_eq!(extra.2, "a=b=c", "值里的 = 被切掉了：{extra:?}");
    assert_eq!(extra.0, "nvram");
}

/// 只有 `[records]` 没有 `[settings]`：`free_play` 保持默认 `true`，不 panic。
#[test]
fn a_file_without_the_settings_section_keeps_the_default() {
    let records = records_from("[records]\nmslug.launches = 2\n");
    assert_eq!(records.set("mslug").launches, 2);
    assert!(records.free_play, "缺 [settings] 时应当保持默认 FREE PLAY");
}

/// CRLF、空行、`#` 与 `;` 注释混在一起也能读（真卡上的文件可能被别的编辑器改过）。
#[test]
fn the_parser_survives_crlf_blank_lines_and_comments() {
    let text = "# 头部注释\r\n\r\n[records]\r\n; 分号注释\r\nmslug.launches = 4\r\n\r\n[settings]\r\nfree_play = 0\r\n";
    let records = records_from(text);
    assert_eq!(records.set("mslug").launches, 4, "CRLF 下没读到记录");
    assert!(!records.free_play, "CRLF 下没读到 free_play");
    assert!(
        records.extras.is_empty(),
        "注释/空行不该被当成未知键：{:?}",
        records.extras
    );
}

/// 坏行（没有 `=`、非数字计数、空值的时间）一律忽略或退零值，不 panic、不影响其它键。
#[test]
fn malformed_lines_are_ignored_without_losing_the_rest() {
    let records = records_from(
        "[records]\n这行没有等号\nmslug.launches = 很多\nsf2.launches = 2\nsf2.last_played = \n[settings]\nfree_play = 1\n孤立的一行\n",
    );
    assert_eq!(
        records.set("mslug").launches, 0,
        "非数字的计数应当退成 0 而不是 panic"
    );
    assert_eq!(records.set("sf2").launches, 2, "坏行不该影响其它记录");
    assert!(
        records.set("sf2").last_played.is_none(),
        "空的时间戳应当读成 None"
    );
    assert!(records.free_play);
}

/// 超长行（真卡上可能是被写坏的二进制）不许 panic，也不许把后面的键吞掉。
#[test]
fn a_very_long_line_does_not_break_the_parser() {
    let long = "x".repeat(200_000);
    let text = format!("[nvram]\nblob = {long}\n[records]\nmslug.launches = 1\n");
    let records = records_from(&text);
    assert_eq!(
        records.set("mslug").launches, 1,
        "超长行之后的键没被读到"
    );
    assert!(
        records
            .extras
            .iter()
            .any(|(section, key, value)| section == "nvram" && key == "blob" && value.len() == 200_000),
        "超长值应当被原样保留（或至少存在），实际 extras 里没有它"
    );
}

/// `[records]` 下的未知键也要带走 section（重写时不能把它塞进 `[settings]`）。
#[test]
fn an_unknown_key_inside_records_keeps_that_section() {
    let records = records_from("[records]\nweird = 1\nmslug.launches = 2\n");
    let extra = records
        .extras
        .iter()
        .find(|(_, key, _)| key == "weird")
        .expect("weird 应当进 extras");
    assert_eq!(extra.0, "records", "未知键的 section 归属错了：{extra:?}");

    // 重写后仍然能被读回来（不会因为 section 名重复而丢）。
    let again = records_from(&records_text(&records));
    assert!(
        again.extras.iter().any(|(_, key, _)| key == "weird"),
        "重写一轮之后未知键丢了"
    );
    assert_eq!(again.set("mslug").launches, 2);
}

/// 同一 set 的 `.launches` 与 `.last_played` 分两行出现时都必须读到同一个 set。
#[test]
fn launches_and_last_played_land_on_the_same_set() {
    let records = records_from(
        "[records]\nmslug.last_played = 2026-11-24_20-41-03\nmslug.launches = 6\n",
    );
    assert_eq!(records.set("mslug").launches, 6);
    assert_eq!(
        records.set("mslug").last_played.as_deref(),
        Some("2026-11-24_20-41-03")
    );
    assert_eq!(records.sets.len(), 1, "两个键不该拆成两个 set");
}
