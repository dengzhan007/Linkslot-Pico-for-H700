//! 服务门（Service Door）：只读页 + FREE PLAY 开关 + 本机记录。
//!
//! 设计原则是**不做假东西**：libretro 的 FBNeo `fixed_options: &[]`、也不暴露
//! DIP，所以 DIP 页与核心页只显示"该核心不提供"，不画可点控件；机台账本在核心
//! 的 NVRAM 里没有解析接口，所以我们只记**自己看到的**（开局次数 / 上次开局时间）。
//!
//! 本机记录写在 `Config/arcade.ini`（INI，不是 JSON——它与 `state.ini` 那种卡带
//! 状态分开，避免"插着街机板改了卡带状态"）。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use slot_store::Platform;

use crate::arcade::{bios_state, ArcadeStage, BiosState};

/// 记录文件（相对卡根）。
pub const ARCADE_INI: &str = "Config/arcade.ini";
/// `[settings]` 下唯一的可编辑项。
pub const FREE_PLAY_KEY: &str = "free_play";
/// 开局次数后缀：`<set>.launches`。
pub const LAUNCHES_SUFFIX: &str = ".launches";
/// 上次开局时间后缀：`<set>.last_played`。
pub const LAST_PLAYED_SUFFIX: &str = ".last_played";

/// 门的五页。`Overview` 与 `Board` 有可执行项，其余三页只读。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ServicePage {
    Overview,
    Board,
    Bios,
    Core,
    Dip,
}

pub const SERVICE_PAGES: [ServicePage; 5] = [
    ServicePage::Overview,
    ServicePage::Board,
    ServicePage::Bios,
    ServicePage::Core,
    ServicePage::Dip,
];

impl ServicePage {
    pub fn next(self) -> ServicePage {
        let at = SERVICE_PAGES.iter().position(|page| *page == self).unwrap_or(0);
        SERVICE_PAGES[(at + 1) % SERVICE_PAGES.len()]
    }

    pub fn prev(self) -> ServicePage {
        let at = SERVICE_PAGES.iter().position(|page| *page == self).unwrap_or(0);
        SERVICE_PAGES[(at + SERVICE_PAGES.len() - 1) % SERVICE_PAGES.len()]
    }

    /// 这一页有没有可编辑/可执行的项。`Dip` / `Core` 恒为假。
    pub fn is_read_only(self) -> bool {
        matches!(self, ServicePage::Bios | ServicePage::Core | ServicePage::Dip)
    }

    /// 翻译键（标题行）。
    pub fn title_key(self) -> &'static str {
        match self {
            ServicePage::Overview => "device.app.arcade.service.overview",
            ServicePage::Board => "device.app.arcade.service.board",
            ServicePage::Bios => "device.app.arcade.service.bios",
            ServicePage::Core => "device.app.arcade.service.core",
            ServicePage::Dip => "device.app.arcade.service.dip",
        }
    }
}

/// `dirty` 时按 B 弹出的三选一。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ServiceConfirm {
    SaveAndClose,
    Discard,
    Cancel,
}

pub const CONFIRM_CHOICES: [ServiceConfirm; 3] = [
    ServiceConfirm::SaveAndClose,
    ServiceConfirm::Discard,
    ServiceConfirm::Cancel,
];

/// 一块板的本机记录。
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct SetRecord {
    pub launches: u32,
    pub last_played: Option<String>,
}

/// `Config/arcade.ini` 的全部内容。
///
/// `extras` 保存**我们不认识的**键（连它所在的 section 一起）：Phase 7 若往这个
/// 文件里加字段，旧版本重写时不能把它们抹掉。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Records {
    pub sets: BTreeMap<String, SetRecord>,
    pub free_play: bool,
    /// `(section, key, value)`，原样保留。
    pub extras: Vec<(String, String, String)>,
}

impl Default for Records {
    fn default() -> Records {
        Records {
            sets: BTreeMap::new(),
            // 街机默认 FREE PLAY（与 `ArcadeScreen::free_play` 的默认一致）。
            free_play: true,
            extras: Vec::new(),
        }
    }
}

impl Records {
    /// 某块板的记录（没有就是零值，不写进 map）。
    pub fn set(&self, set: &str) -> SetRecord {
        self.sets.get(set).cloned().unwrap_or_default()
    }
}

pub fn records_path(root: &Path) -> PathBuf {
    root.join(ARCADE_INI)
}

fn atomic_write(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("ini.tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path)
}

/// 逐行解析，**带 section**（`ini::parse` 会丢掉 section，而我们要原样保留未知键）。
fn parse_ini(text: &str) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    let mut section = String::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')) {
            section = name.trim().to_string();
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        out.push((
            section.clone(),
            key.trim().to_string(),
            value.trim().to_string(),
        ));
    }
    out
}

pub fn read_records(root: &Path) -> Records {
    let text = std::fs::read_to_string(records_path(root)).unwrap_or_default();
    records_from(&text)
}

/// 从文本解析记录（坏行直接忽略，不 panic、不阻塞开门）。
pub fn records_from(text: &str) -> Records {
    let mut records = Records::default();
    for (section, key, value) in parse_ini(text) {
        if section == "settings" && key == FREE_PLAY_KEY {
            records.free_play = value != "0";
            continue;
        }
        if section == "records" {
            if let Some(set) = key.strip_suffix(LAUNCHES_SUFFIX) {
                records.sets.entry(set.to_string()).or_default().launches =
                    value.parse().unwrap_or(0);
                continue;
            }
            if let Some(set) = key.strip_suffix(LAST_PLAYED_SUFFIX) {
                records.sets.entry(set.to_string()).or_default().last_played =
                    Some(value.clone()).filter(|v| !v.is_empty());
                continue;
            }
        }
        records.extras.push((section, key, value));
    }
    records
}

/// 整文件重写：`[records]` / `[settings]` 由我们生成，其余键原样带到它们**原来的
/// section**（`extras`），不会因为重写而丢字段。
pub fn records_text(records: &Records) -> String {
    let mut out = String::from("[records]\n");
    for (set, record) in &records.sets {
        out.push_str(&format!("{set}{LAUNCHES_SUFFIX} = {}\n", record.launches));
        if let Some(stamp) = &record.last_played {
            out.push_str(&format!("{set}{LAST_PLAYED_SUFFIX} = {stamp}\n"));
        }
    }
    out.push_str("\n[settings]\n");
    out.push_str(&format!(
        "{FREE_PLAY_KEY} = {}\n",
        if records.free_play { 1 } else { 0 }
    ));
    // 未知键按原 section 归组追加（同 section 连续出现，文件不会来回跳）。
    let sections: Vec<String> = {
        let mut names: Vec<String> = records
            .extras
            .iter()
            .map(|(section, _, _)| section.clone())
            .collect();
        names.sort();
        names.dedup();
        names
    };
    for section in sections {
        if !section.is_empty() {
            out.push_str(&format!("\n[{section}]\n"));
        }
        for (at, key, value) in &records.extras {
            if *at == section {
                out.push_str(&format!("{key} = {value}\n"));
            }
        }
    }
    out
}

pub fn write_records(root: &Path, records: &Records) -> std::io::Result<()> {
    // 先读旧文件：调用方传进来的 `Records` 若来自别处（例如只改了 free_play），
    // 也不该把磁盘上的未知键冲掉。
    let mut merged = read_records(root);
    merged.sets = records.sets.clone();
    merged.free_play = records.free_play;
    if !records.extras.is_empty() {
        merged.extras = records.extras.clone();
    }
    atomic_write(&records_path(root), &records_text(&merged))
}

/// 记一次开局：只动这一块板。
pub fn bump_launch(records: &mut Records, set: &str, now: &str) {
    let entry = records.sets.entry(set.to_string()).or_default();
    entry.launches = entry.launches.saturating_add(1);
    entry.last_played = Some(now.to_string());
}

/// 门能不能开：只有主界面（PCB 架）能开。
pub fn door_opens(stage: ArcadeStage) -> bool {
    stage == ArcadeStage::Main
}

/// 门的运行态。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ServiceState {
    pub page: ServicePage,
    /// 有未保存改动（只有 FREE PLAY 会置真）。
    pub dirty: bool,
    /// 当前页的焦点行。
    pub focus: usize,
    /// `dirty` 时按 B 弹出的确认；`None` = 没在确认。
    pub confirm: Option<ServiceConfirm>,
    pub records: Records,
    pub set: String,
    pub platform: Platform,
}

impl ServiceState {
    pub fn open(records: Records, platform: Platform, set: String) -> ServiceState {
        ServiceState {
            page: ServicePage::Overview,
            dirty: false,
            focus: 0,
            confirm: None,
            records,
            set,
            platform,
        }
    }

    /// 当前 FREE PLAY 值（编辑中的值就是 `records.free_play`）。
    pub fn free_play(&self) -> bool {
        self.records.free_play
    }

    pub fn next_page(&mut self) {
        self.page = self.page.next();
        self.focus = 0;
    }

    pub fn prev_page(&mut self) {
        self.page = self.page.prev();
        self.focus = 0;
    }

    pub fn move_focus(&mut self, delta: i32) {
        let rows = self.rows();
        if rows == 0 {
            self.focus = 0;
            return;
        }
        let rows = rows as i32;
        self.focus = (((self.focus as i32 + delta) % rows + rows) % rows) as usize;
    }

    /// 当前页的可执行行数（只读页为 0）。
    pub fn rows(&self) -> usize {
        match self.page {
            ServicePage::Overview => 1, // FREE PLAY
            ServicePage::Board => 1,    // 退板
            _ => 0,
        }
    }

    /// A：执行焦点行。返回 `true` 表示"要离开门"（退板）。
    pub fn activate(&mut self) -> bool {
        match self.page {
            // 唯一可编辑项：FREE PLAY。翻转即 dirty。
            ServicePage::Overview => {
                self.records.free_play = !self.records.free_play;
                self.dirty = true;
                false
            }
            // 退板：交给 app 关掉门并回 PCB 架。
            ServicePage::Board => true,
            // 只读页没有可执行项。
            ServicePage::Bios | ServicePage::Core | ServicePage::Dip => false,
        }
    }

    /// B：没有改动就直接关；有改动先进确认。
    pub fn request_close(&mut self) -> bool {
        if self.confirm.is_some() {
            return false;
        }
        if self.dirty {
            self.confirm = Some(ServiceConfirm::SaveAndClose);
            self.focus = 0;
            return false;
        }
        true
    }

    pub fn move_confirm(&mut self, delta: i32) {
        if self.confirm.is_none() {
            return;
        }
        let at = self.confirm_at();
        let next = (((at as i32 + delta) % CONFIRM_CHOICES.len() as i32
            + CONFIRM_CHOICES.len() as i32)
            % CONFIRM_CHOICES.len() as i32) as usize;
        self.confirm = Some(CONFIRM_CHOICES[next]);
    }

    fn confirm_at(&self) -> usize {
        self.confirm
            .and_then(|choice| CONFIRM_CHOICES.iter().position(|c| *c == choice))
            .unwrap_or(0)
    }

    /// 确认框里按 A。返回 `Some(true/false)` = 是否保存后关闭；`None` = 留在门里。
    pub fn resolve_confirm(&mut self) -> Option<bool> {
        match self.confirm.take()? {
            ServiceConfirm::SaveAndClose => Some(true),
            ServiceConfirm::Discard => {
                // 放弃改动：把内存里的值退回磁盘。
                self.dirty = false;
                Some(false)
            }
            ServiceConfirm::Cancel => None,
        }
    }

    /// 保存（`SaveAndClose` 用）：失败只打日志，不阻塞关门。
    pub fn save(&mut self, root: &Path) {
        if let Err(error) = write_records(root, &self.records) {
            eprintln!("slot: arcade records: {error}");
        }
        self.dirty = false;
    }

    /// 存档被外部改了（Web 刷新）时重读记录。
    pub fn reload(&mut self, root: &Path) {
        self.records = read_records(root);
        self.dirty = false;
    }

    /// 当前板在 BIOS 上的状态（Bios 页与 Overview 用）。
    pub fn bios(&self, root: &Path) -> BiosState {
        bios_state(root, self.platform)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> ServiceState {
        ServiceState::open(Records::default(), Platform::Neogeo, "mslug".to_string())
    }

    #[test]
    fn the_door_opens_only_from_the_main_screen() {
        assert!(door_opens(ArcadeStage::Main));
        assert!(!door_opens(ArcadeStage::Attract), "机台运行时不能开门");
        assert!(!door_opens(ArcadeStage::Inserting), "插板动画期间不能开门");
    }

    #[test]
    fn closing_with_unsaved_changes_asks_first() {
        let mut clean = state();
        assert!(clean.request_close(), "没有改动 → 直接关");
        assert_eq!(clean.confirm, None);

        let mut dirty = state();
        dirty.activate(); // 翻 FREE PLAY
        assert!(dirty.dirty);
        assert!(!dirty.request_close(), "有改动 → 先进确认");
        assert_eq!(dirty.confirm, Some(ServiceConfirm::SaveAndClose));

        // 默认停在"保存并关闭"；移到"取消"再按 A → 留在门里、不改 dirty。
        assert_eq!(dirty.confirm, Some(ServiceConfirm::SaveAndClose));
        dirty.move_confirm(2);
        assert_eq!(dirty.confirm, Some(ServiceConfirm::Cancel));
        assert_eq!(dirty.resolve_confirm(), None);
        assert!(dirty.dirty, "取消不该清 dirty");
        assert_eq!(dirty.confirm, None, "取消后关掉确认框");

        // 放弃 → 也不保存，但退出确认并清 dirty。
        dirty.request_close();
        dirty.move_confirm(1);
        assert_eq!(dirty.confirm, Some(ServiceConfirm::Discard));
        assert_eq!(dirty.resolve_confirm(), Some(false));
        assert!(!dirty.dirty);

        // 保存并关闭 → Some(true)。
        let mut save = state();
        save.activate();
        save.request_close();
        assert_eq!(save.resolve_confirm(), Some(true));
    }

    #[test]
    fn the_free_play_toggle_is_the_only_dirty_source() {
        let mut door = state();
        door.next_page();
        door.next_page();
        door.prev_page();
        door.move_focus(1);
        door.move_confirm(1);
        assert!(!door.dirty, "翻页 / 移焦点不许置 dirty");

        // 只读页按 A 也不置 dirty。
        door.page = ServicePage::Dip;
        door.activate();
        door.page = ServicePage::Core;
        door.activate();
        assert!(!door.dirty);

        // 唯一的 dirty 源：FREE PLAY。
        door.page = ServicePage::Overview;
        door.activate();
        assert!(door.dirty);
        assert!(!door.free_play(), "默认 FREE PLAY 打开，翻一次就关掉");
    }

    #[test]
    fn records_round_trip_through_arcade_ini() {
        let card = tempfile::tempdir().unwrap();
        let root = card.path();
        let mut records = Records::default();
        records.free_play = false;
        bump_launch(&mut records, "mslug", "2026-11-24T20:41:03");
        bump_launch(&mut records, "sf2", "2026-11-24T20:45:00");
        write_records(root, &records).unwrap();

        // 文件是 INI，带两个 section。
        let text = std::fs::read_to_string(records_path(root)).unwrap();
        assert!(text.contains("[records]"));
        assert!(text.contains("[settings]"));
        assert!(text.contains("mslug.launches = 1"));

        let read = read_records(root);
        assert_eq!(read, records, "写→读必须完全相等");

        // 坏文件：当空记录，不 panic。
        std::fs::write(records_path(root), b"\x00\xffnot an ini").unwrap();
        let broken = read_records(root);
        assert!(broken.sets.is_empty());
        assert!(broken.free_play, "坏文件退回默认（FREE PLAY 开）");
    }

    #[test]
    fn a_launch_bumps_the_counter_and_the_timestamp() {
        let mut records = Records::default();
        assert_eq!(records.set("mslug").launches, 0);
        bump_launch(&mut records, "mslug", "2026-11-24T20:41:03");
        let record = records.set("mslug");
        assert_eq!(record.launches, 1);
        assert_eq!(record.last_played.as_deref(), Some("2026-11-24T20:41:03"));
    }

    #[test]
    fn a_launch_is_recorded_per_set() {
        let mut records = Records::default();
        bump_launch(&mut records, "mslug", "2026-11-24T20:41:03");
        bump_launch(&mut records, "mslug", "2026-11-24T21:02:00");
        bump_launch(&mut records, "sf2", "2026-11-24T21:10:00");

        assert_eq!(records.set("mslug").launches, 2);
        assert_eq!(
            records.set("mslug").last_played.as_deref(),
            Some("2026-11-24T21:02:00"),
            "只更新该 set 的时间"
        );
        assert_eq!(records.set("sf2").launches, 1);
        assert_eq!(
            records.set("sf2").last_played.as_deref(),
            Some("2026-11-24T21:10:00")
        );
        assert_eq!(records.set("dino").launches, 0, "没开过的板不凭空出现");
    }

    #[test]
    fn dip_and_core_pages_are_read_only() {
        let mut door = state();
        for page in [ServicePage::Dip, ServicePage::Core, ServicePage::Bios] {
            door.page = page;
            assert!(page.is_read_only(), "{page:?} 应当只读");
            assert_eq!(door.rows(), 0, "{page:?} 不该有可执行行");
            door.activate();
            assert!(!door.dirty, "{page:?} 的任何操作都不该置 dirty");
        }
        // 有可执行项的两页不是只读。
        assert!(!ServicePage::Overview.is_read_only());
        assert!(!ServicePage::Board.is_read_only());
    }

    #[test]
    fn writing_records_keeps_unknown_keys() {
        let card = tempfile::tempdir().unwrap();
        let root = card.path();
        std::fs::create_dir_all(root.join("Config")).unwrap();
        std::fs::write(
            records_path(root),
            b"[settings]\nfree_play = 1\nfuture_key = 1\n[nvram]\nfoo = bar\n",
        )
        .unwrap();

        let mut records = read_records(root);
        assert!(records.free_play);
        bump_launch(&mut records, "mslug", "2026-11-24T20:41:03");
        records.free_play = false;
        write_records(root, &records).unwrap();

        let text = std::fs::read_to_string(records_path(root)).unwrap();
        assert!(text.contains("future_key = 1"), "未知键不能丢：\n{text}");
        assert!(text.contains("foo = bar"), "未知 section 也要保留：\n{text}");
        assert!(text.contains("mslug.launches = 1"));
        assert!(text.contains("free_play = 0"));

        // 再读一遍：未知键仍然在。
        let again = read_records(root);
        assert_eq!(again.set("mslug").launches, 1);
        assert!(!again.free_play);
        assert!(again
            .extras
            .iter()
            .any(|(_, key, value)| key == "future_key" && value == "1"));
    }
}
