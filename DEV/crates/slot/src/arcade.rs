//! 街机 / NeoGeo 主界面的状态（纯逻辑，不碰 `Draw`，可以脱离窗口单测）。
//!
//! 这一层只回答四个问题：**哪些板**（平台 + 筛选）、**选第几块**、**筛选条有哪些
//! 项**、**BIOS 在不在**。柜子怎么画、PCB 怎么摆属于 `slot-ui`，与这里无关。

use std::collections::BTreeMap;
use std::ops::Range;
use std::path::{Path, PathBuf};

use slot_store::arcade_dat::Catalog;
use slot_input::Btn;
use slot_store::{Cart, Platform};

/// 筛选条最多显示几个机板（`全部` 之外）；再多就挤不下顶栏了。
pub const MAX_SYSTEMS: usize = 5;

/// 一屏同时可见的 PCB 块数。
pub const WINDOW: usize = 3;

/// 插板动画时长（秒）：PCB 从右侧架位滑进 JAMMA 槽 + CRT 消磁。
pub const INSERT_SECS: f32 = 0.180;

/// 投币动画时长（秒）：token 从投币口上方落进机器。
pub const COIN_SECS: f32 = 0.240;

/// 最多累计几个 credit（面板上是两位数，9 是设计稿的上限）。
pub const MAX_CREDITS: u8 = 9;

/// `tick` 的 `dt` 上限。掉帧后一帧就可能跑完整个插板动画，插值会"跳"过去；
/// 夹住上限保证动画至少跨几帧。
pub const MAX_TICK: f32 = 0.05;

/// 阶段的**无载荷拷贝**：给 `ArcadeView` 当签名用。
///
/// `Stage` 里带着 `t` / `coin_t` / `blink` 这些每帧变量，放进视图会让"视图变了
/// 才重建纹理"每帧都成立；这里只留类别。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ArcadeStage {
    #[default]
    Main,
    Inserting,
    Attract,
}

/// 街机主界面的阶段。
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Stage {
    /// 浏览 PCB 架。
    Main,
    /// 插板动画中（`t` 秒，0..[`INSERT_SECS`]）。
    Inserting { t: f32 },
    /// 机台待机：CRT 显示 attract，等投币 / START。
    Attract {
        credits: u8,
        /// 投币 token 的上浮计时，`> 0` 时画 token。
        coin_t: f32,
        /// `INSERT COIN` 的闪烁相位（秒，取模 2.0；`< 0.5` 时亮）。
        blink: f32,
    },
}

impl Stage {
    pub fn kind(&self) -> ArcadeStage {
        match self {
            Stage::Main => ArcadeStage::Main,
            Stage::Inserting { .. } => ArcadeStage::Inserting,
            Stage::Attract { .. } => ArcadeStage::Attract,
        }
    }
}

/// 主界面的筛选：全部，或某个机板（`CPS-2` / `NEOGEO` …）。
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ArcadeFilter {
    All,
    System(String),
}

impl ArcadeFilter {
    /// 顶栏 chip 上显示的文字；`None` 表示"按语言取 `全部`"。
    pub fn system(&self) -> Option<&str> {
        match self {
            ArcadeFilter::All => None,
            ArcadeFilter::System(name) => Some(name.as_str()),
        }
    }
}

/// 启动一套街机板所需要的一切（纯数据，`session` 拿它去开核心）。
///
/// 抽成纯函数的原因：真实核心在街机机上跑不了（宿主是 Windows，核心是
/// aarch64 Linux `.so`），"路径 / BIOS 目录 / canvas 选对了"只能用纯函数验，
/// 真实启动留给 Phase 7 真机。
#[derive(Clone, Debug, PartialEq)]
pub struct LaunchPlan {
    /// 交给核心的 cart：`rom` 指向 `Games/<平台>/<stem>.zip`。
    pub cart: Cart,
    /// 覆盖给核心的 system/BIOS 目录；`None` = 用默认的 `BIOS/`。
    pub system_dir: Option<PathBuf>,
    /// 载入前告知核心的帧尺寸。真实尺寸由 `av_info` 在 load 后给（Phase 7 真机核）。
    pub canvas: (u32, u32),
}

/// 街机包把 BIOS 放在 `BIOS/fbneo/`（`neogeo.zip` 等），模拟器默认目录是
/// `BIOS/`。目录在就用前者，否则退回后者（**总要给一个**，让核心自己报缺）。
pub const FBNEO_BIOS_DIR: &str = "BIOS/fbneo";
/// 模拟器的默认 system 目录。
pub const DEFAULT_BIOS_DIR: &str = "BIOS";

pub fn launch_plan(root: &Path, platform: Platform, stem: &str, title: &str) -> LaunchPlan {
    // `metadata()` 对**文件**也返回 Ok：卡上若有个叫 `BIOS/fbneo` 的文件
    // （手滑重命名、包解压出同名文件），`system_dir` 就会指到那个文件上，
    // 核心找不到 BIOS。契约说的是"`BIOS/fbneo/` **目录**存在"，所以用 `is_dir()`。
    let folder = if root.join(FBNEO_BIOS_DIR).is_dir() {
        FBNEO_BIOS_DIR
    } else {
        DEFAULT_BIOS_DIR
    };
    let title = if title.trim().is_empty() {
        stem.to_string()
    } else {
        title.trim().to_string()
    };
    LaunchPlan {
        cart: Cart {
            platform,
            stem: stem.to_string(),
            rom: root.join("Games").join(platform.dir_name()).join(format!("{stem}.zip")),
            label: None,
            title,
            code: String::new(),
            shell: None,
            model: Default::default(),
            front: None,
        },
        system_dir: Some(root.join(folder)),
        // 街机的 canvas 固定在 `slot_model` 的机台表里（Arcade 384×224、
        // NeoGeo 320×224）；真实尺寸由 `av_info` 在 load 之后覆盖。
        canvas: platform.frame(),
    }
}

/// 街机存档的两条路径。
///
/// **不自己拼路径**：`sram` 来自 `persist::sav_path`、`state_dir` 来自
/// `slot_store::state_dir`——两者都是生产在用的同一段逻辑，避免"测试断言的路径"
/// 与"生产写的路径"漂移。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavePaths {
    /// `<root>/Saves/<平台目录>/<stem>.sav`
    pub sram: PathBuf,
    /// `<root>/States/<平台目录>/<核心>/<stem>/`（ring 目录）
    pub state_dir: PathBuf,
}

/// 核心名**显式传入**（街机侧调用点写 `Core::default_for(platform)`）：不藏进函数里，
/// 是因为同平台多核心时"默认"与"实际"会分叉（见 `slot_store::state_dir` 的注释）。
///
/// 生产侧真正读写用的是 `persist::sav_path` / `App::ring()`，这个函数是给验收
/// 用例与 Phase 6 UI 的只读视图，所以非测试构建里没人调它。
#[allow(dead_code)]
pub(crate) fn save_paths(
    root: &Path,
    platform: Platform,
    core: slot_store::Core,
    stem: &str,
) -> SavePaths {
    SavePaths {
        sram: crate::persist::sav_path(root, platform, stem),
        state_dir: slot_store::state_dir(root, platform, core, stem),
    }
}

/// 六键表（B1..B6）。与 [`ARCADE_BUTTONS`] 同一个来源，给验收与 Phase 6 用。
pub fn arcade_buttons() -> [Btn; 6] {
    ARCADE_BUTTONS
}

/// MAME 习惯的六键顺序：B1..B6。
///
/// 这是**唯一来源**：`arcade_pad` 与绘制/文档都读它。顺序错了会把"拳"和"跳"
/// 对调，而且只看代码几乎发现不了——所以它是一条可以被断言的常量。
pub const ARCADE_BUTTONS: [Btn; 6] = [
    Btn::B,  // B1 轻拳
    Btn::Y,  // B2 重拳
    Btn::A,  // B3 轻脚
    Btn::X,  // B4 重脚
    Btn::L1, // B5
    Btn::R1, // B6
];

/// 街机按键表：六键 + START + SELECT（SELECT 就是投币）。
///
/// `None` = 这个物理键在街机布局里没有作用（L2/R2 等）。
pub fn arcade_pad(btn: Btn) -> Option<u16> {
    use slot_retro::ButtonMask;
    // 位置由表决定：`B1..B6` 分别落到 libretro 的 B/Y/A/X/L/R。
    // 这里**没有第二份 `[Btn::B, Btn::Y, …]` 字面量**——顺序只存在于
    // `ARCADE_BUTTONS`，测试断言它也等于 `PadLayout::Arcade.buttons()`。
    const BITS: [u16; 6] = [
        ButtonMask::B,
        ButtonMask::Y,
        ButtonMask::A,
        ButtonMask::X,
        ButtonMask::L,
        ButtonMask::R,
    ];
    if let Some(slot) = ARCADE_BUTTONS.iter().position(|candidate| *candidate == btn) {
        return Some(BITS[slot]);
    }
    Some(match btn {
        Btn::Start => ButtonMask::START,
        Btn::Select => ButtonMask::SELECT,
        _ => return None,
    })
}

/// 主界面的按键。与平台按键的映射放在 `app.rs`，这里只关心语义。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ArcadeKey {
    Left,
    Right,
    /// X：切换筛选。
    Filter,
    /// Y：切换当前板的收藏。
    Favorite,
    /// B：返回书架。
    Back,
    /// A / START / MENU：本阶段无动作。
    Insert,
    Service,
}

/// 一次按键产生的语义结果。`app.rs` 据此改 `Phase` 或调收藏 API。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ArcadeAction {
    None,
    /// A：本阶段 no-op，但保留语义位给 Phase 5 的"插板"。
    Insert,
    /// Y：收藏当前板。
    Favorite,
    /// B：回书架。
    Back,
}

/// NeoGeo 的 BIOS 状态（Arcade 永远不需要）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BiosState {
    NotRequired,
    Present,
    Missing,
}

/// BIOS 只看这两条路径（与设计稿一致）；任一是"普通文件且长度 > 0"就算在。
pub const BIOS_PATHS: [&str; 2] = ["BIOS/fbneo/neogeo.zip", "BIOS/neogeo.zip"];

/// 判定某个机种此刻的 BIOS 状态。
pub fn bios_state(root: &Path, platform: Platform) -> BiosState {
    if platform != Platform::Neogeo {
        return BiosState::NotRequired;
    }
    for rel in BIOS_PATHS {
        let path = root.join(rel);
        // `symlink_metadata`：目录、符号链接、0 字节的空壳都不算"有 BIOS"。
        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        if meta.file_type().is_file() && meta.len() > 0 {
            return BiosState::Present;
        }
    }
    BiosState::Missing
}

/// 3 块一屏的窗口：当前板尽量居中，首尾各留空位，永不越界。
pub fn visible_window(index: usize, len: usize) -> Range<usize> {
    if len == 0 {
        return 0..0;
    }
    let start = index.saturating_sub(1).min(len.saturating_sub(WINDOW));
    start..(start + WINDOW).min(len)
}

/// 主界面状态机。
///
/// 不做 `Eq`：`Stage::Inserting{t}` / `Attract{coin_t, blink}` 里是浮点，只比较
/// 相等性（`PartialEq`）就够，`Eq` 会逼着把浮点换成定点，得不偿失。
#[derive(Clone, Debug, PartialEq)]
pub struct ArcadeScreen {
    pub platform: Platform,
    pub filter: ArcadeFilter,
    /// 当前选中的板，是该平台 + 当前筛选后的下标。
    pub index: usize,
    pub stage: Stage,
    /// FREE PLAY（默认开）：不投币也能 START；投币仍可听声音。
    pub free_play: bool,
    /// 上一次 START 因为核心缺失失败；CRT 上会挂一行红字。
    /// `insert()` / `start()` 成功 / `back()` 都会清掉。
    pub core_missing: bool,
}

impl ArcadeScreen {
    pub fn new(platform: Platform) -> ArcadeScreen {
        ArcadeScreen {
            platform,
            filter: ArcadeFilter::All,
            index: 0,
            stage: Stage::Main,
            free_play: true,
            core_missing: false,
        }
    }

    /// 该平台 + 该筛选下的板子，保持 `carts` 原有顺序（scan 已按 set 名排好）。
    pub fn boards<'a>(&self, catalog: &Catalog, carts: &'a [Cart]) -> Vec<&'a Cart> {
        carts
            .iter()
            .filter(|cart| cart.platform == self.platform)
            .filter(|cart| match &self.filter {
                ArcadeFilter::All => true,
                ArcadeFilter::System(system) => catalog
                    .get(&cart.stem)
                    .or_else(|| catalog.get(&cart.stem.to_ascii_lowercase()))
                    .is_some_and(|entry| entry.system == *system),
            })
            .collect()
    }

    /// 筛选条上的机板名：按出现次数降序，同频按名字升序（结果稳定），最多
    /// [`MAX_SYSTEMS`] 个。`全部` 不在这里——它永远是第一项，由调用方加。
    pub fn systems(&self, catalog: &Catalog, carts: &[Cart]) -> Vec<String> {
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for cart in carts.iter().filter(|cart| cart.platform == self.platform) {
            let entry = catalog
                .get(&cart.stem)
                .or_else(|| catalog.get(&cart.stem.to_ascii_lowercase()));
            if let Some(entry) = entry {
                if !entry.system.is_empty() {
                    *counts.entry(entry.system.as_str()).or_default() += 1;
                }
            }
        }
        let mut ranked: Vec<(&str, usize)> = counts.into_iter().collect();
        ranked.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(right.0)));
        ranked
            .into_iter()
            .take(MAX_SYSTEMS)
            .map(|(system, _)| system.to_string())
            .collect()
    }

    /// 循环下一块；空列表保持 0。**只有 `Main` 能换板**（插板中与 attract 里
    /// 机台状态已定，翻板没有意义，也免得 attract 时选中项被误改）。
    pub fn select_next(&mut self, len: usize) {
        if self.stage.kind() != ArcadeStage::Main {
            return;
        }
        if len == 0 {
            self.index = 0;
        } else {
            self.index = (self.index + 1) % len;
        }
    }

    pub fn select_prev(&mut self, len: usize) {
        if self.stage.kind() != ArcadeStage::Main {
            return;
        }
        if len == 0 {
            self.index = 0;
        } else {
            self.index = (self.index + len - 1) % len;
        }
    }

    /// 夹到 `0..len`；列表变短后（换筛选、库刷新）不会越界。
    pub fn set_index(&mut self, index: usize, len: usize) {
        self.index = if len == 0 { 0 } else { index.min(len - 1) };
    }

    /// `全部 → 各机板 → 全部`；切换后回到第一块。
    pub fn cycle_filter(&mut self, systems: &[String]) {
        if self.stage.kind() != ArcadeStage::Main {
            return;
        }
        let next = match &self.filter {
            ArcadeFilter::All => systems
                .first()
                .map(|system| ArcadeFilter::System(system.clone()))
                .unwrap_or(ArcadeFilter::All),
            ArcadeFilter::System(current) => {
                let position = systems.iter().position(|system| system == current);
                match position {
                    Some(index) if index + 1 < systems.len() => {
                        ArcadeFilter::System(systems[index + 1].clone())
                    }
                    // 已是最后一项（或该项已消失）→ 回到全部。
                    _ => ArcadeFilter::All,
                }
            }
        };
        self.filter = next;
        self.index = 0;
    }

    /// A：把选中的板插进 JAMMA 槽。只有 Main 且列表非空时成立。
    pub fn insert(&mut self, boards: usize) -> bool {
        if self.stage.kind() != ArcadeStage::Main || boards == 0 {
            return false;
        }
        self.core_missing = false;
        self.stage = Stage::Inserting { t: 0.0 };
        true
    }

    /// 推进动画。`dt` 是秒，且夹在 [`MAX_TICK`] 以内。
    pub fn tick(&mut self, dt: f32) {
        let dt = dt.clamp(0.0, MAX_TICK);
        match &mut self.stage {
            Stage::Main => {}
            Stage::Inserting { t } => {
                *t += dt;
                if *t >= INSERT_SECS {
                    // 进 attract 时把上一轮的 token / 闪烁相位清零，避免残留。
                    self.stage = Stage::Attract {
                        credits: 0,
                        coin_t: 0.0,
                        blink: 0.0,
                    };
                }
            }
            Stage::Attract { coin_t, blink, .. } => {
                *blink = (*blink + dt) % 2.0;
                *coin_t = (*coin_t - dt).max(0.0);
            }
        }
    }

    /// SELECT：投币。
    ///
    /// 规则（与 QA 的边界用例一致）：
    /// - 只在 `Attract` 生效；
    /// - **限流**：上一枚的 `coin_t` 还在倒数（`> 0`）时拒绝——`coin_t <= 0`
    ///   才接受，接受后重新填满 `COIN_SECS`；所以 `0.240` 恰好可以投第二枚；
    /// - **满员**：`credits == MAX_CREDITS` 时返回 `false`，且**不动 `coin_t`**
    ///   （不占掉限流窗口，玩家松手再按也不算"刚投过"）；
    /// - FREE PLAY 下照常接受、照常计数（设计稿：仍可投币听声音，只是 START
    ///   不消耗）。
    pub fn coin(&mut self) -> bool {
        match &mut self.stage {
            Stage::Attract { credits, coin_t, .. } => {
                if *credits >= MAX_CREDITS {
                    return false;
                }
                if *coin_t > 0.0 {
                    return false;
                }
                *credits += 1;
                *coin_t = COIN_SECS;
                true
            }
            _ => false,
        }
    }

    /// START：开始游戏。只在 Attract 生效；FREE PLAY 或有 credit 时成立。
    ///
    /// **不在这里启动核心**（那是 5b）：这里只回答"能不能开"以及扣不扣币。
    pub fn start(&mut self) -> bool {
        let free = self.free_play;
        match &mut self.stage {
            Stage::Attract { credits, .. } => {
                if free {
                    self.core_missing = false;
                    return true;
                }
                if *credits > 0 {
                    *credits -= 1;
                    self.core_missing = false;
                    return true;
                }
                false
            }
            _ => false,
        }
    }

    /// B：从 attract 回到 PCB 架（插板动画期间不允许）。
    pub fn back(&mut self) -> bool {
        match self.stage {
            Stage::Attract { .. } => {
                self.core_missing = false;
                self.stage = Stage::Main;
                true
            }
            _ => false,
        }
    }

    pub fn stage(&self) -> ArcadeStage {
        self.stage.kind()
    }

    /// 现在能不能接受输入。
    ///
    /// 这是"插板动画期间所有输入无效"的**唯一闸门**：调用方（`app`）与验收都看
    /// 它，不必逐键列举。`Main`/`Attract` 为真，`Inserting` 为假。
    pub fn accepts_input(&self) -> bool {
        !self.is_busy()
    }

    pub fn free_play(&self) -> bool {
        self.free_play
    }

    /// 开关 FREE PLAY。关掉之后 `start()` 才会扣币（Phase 6 的服务门也用这里）。
    pub fn set_free_play(&mut self, on: bool) {
        self.free_play = on;
    }

    /// 上一次启动是不是因为核心缺失失败的。
    ///
    /// 字段仍是 `pub`（QA 的用例正在直接读它）；这个方法给 Phase 6 的服务门用，
    /// 将来字段要改名/改私有，调用方不必跟着动。
    pub fn core_missing(&self) -> bool {
        self.core_missing
    }

    /// 记一次"核心缺失"。`app` 在 `on_core_failed()`（且来源是街机）里调。
    pub fn note_core_missing(&mut self) {
        self.core_missing = true;
    }

    /// 已投的币数。只有 `Attract` 里有意义，其它阶段恒 0。
    pub fn credits(&self) -> u8 {
        match self.stage {
            Stage::Attract { credits, .. } => credits,
            _ => 0,
        }
    }

    /// 当前筛选项下标（`0` = 全部），给视图/顶栏 chip 用。
    pub fn filter_index(&self, systems: &[String]) -> usize {
        match self.filter.system() {
            None => 0,
            Some(system) => systems
                .iter()
                .position(|name| name == system)
                .map(|index| index + 1)
                .unwrap_or(0),
        }
    }

    /// 当前筛选下第 `index` 块板（没有就 `None`）。
    pub fn current<'a>(&self, catalog: &Catalog, carts: &'a [Cart]) -> Option<&'a Cart> {
        self.boards(catalog, carts).get(self.index).copied()
    }

    /// 插板动画期间不接受任何输入（选中、筛选、投币、返回全部 no-op）。
    ///
    /// 写在状态机里而不是只写在 `app` 里：这是 `ArcadeScreen` 自己的不变量，
    /// 谁调用都该成立。
    pub fn is_busy(&self) -> bool {
        matches!(self.stage, Stage::Inserting { .. })
    }

    /// 一块按键，返回语义动作；**只在这个结构上改状态**，不碰 `Phase`。
    ///
    /// `boards` 是当前筛选下的板数（`boards().len()`），`systems` 来自
    /// [`ArcadeScreen::systems`]，两者都由调用方先算好——这样这里可以脱离卡片
    /// 与 `Draw` 单测。
    pub fn key(
        &mut self,
        key: ArcadeKey,
        boards: usize,
        systems: &[String],
    ) -> ArcadeAction {
        // 只有 Main 会走这张表；Inserting / Attract 的键由 `app` 直接分派到
        // `insert/coin/start/back`，不经这里。
        if self.stage.kind() != ArcadeStage::Main {
            return ArcadeAction::None;
        }
        match key {
            ArcadeKey::Left => {
                self.select_prev(boards);
                ArcadeAction::None
            }
            ArcadeKey::Right => {
                self.select_next(boards);
                ArcadeAction::None
            }
            ArcadeKey::Filter => {
                self.cycle_filter(systems);
                ArcadeAction::None
            }
            ArcadeKey::Favorite => ArcadeAction::Favorite,
            ArcadeKey::Back => ArcadeAction::Back,
            ArcadeKey::Insert => ArcadeAction::Insert,
            ArcadeKey::Service => ArcadeAction::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slot_store::arcade_dat::Entry;

    fn cart(platform: Platform, stem: &str) -> Cart {
        Cart {
            platform,
            stem: stem.to_string(),
            rom: std::path::PathBuf::from(format!("Games/{}/{stem}.zip", platform.dir_name())),
            label: None,
            title: stem.to_string(),
            code: String::new(),
            shell: None,
            model: Default::default(),
            front: None,
        }
    }

    fn entry(set: &str, system: &str) -> Entry {
        Entry {
            set: set.to_string(),
            title: set.to_uppercase(),
            system: system.to_string(),
            year: String::new(),
            players: 1,
            orientation: Default::default(),
            parent: String::new(),
            is_bios: false,
        }
    }

    fn catalog(platform: Platform, entries: Vec<Entry>) -> Catalog {
        Catalog::from_entries(platform, entries)
    }

    fn arcade_carts() -> Vec<Cart> {
        vec![
            cart(Platform::Arcade, "sf2"),
            cart(Platform::Arcade, "ssf2"),
            cart(Platform::Arcade, "dino"),
            cart(Platform::Neogeo, "mslug"),
        ]
    }

    fn arcade_catalog() -> Catalog {
        catalog(
            Platform::Arcade,
            vec![
                entry("sf2", "CPS-1"),
                entry("ssf2", "CPS-2"),
                entry("dino", "CPS-1"),
            ],
        )
    }

    #[test]
    fn boards_follow_the_filter() {
        let carts = arcade_carts();
        let catalog = arcade_catalog();
        let mut screen = ArcadeScreen::new(Platform::Arcade);

        let all: Vec<&str> = screen
            .boards(&catalog, &carts)
            .iter()
            .map(|cart| cart.stem.as_str())
            .collect();
        assert_eq!(all, vec!["sf2", "ssf2", "dino"], "全部要按 scan 顺序");

        screen.filter = ArcadeFilter::System("CPS-2".into());
        let cps2: Vec<&str> = screen
            .boards(&catalog, &carts)
            .iter()
            .map(|cart| cart.stem.as_str())
            .collect();
        assert_eq!(cps2, vec!["ssf2"]);

        // 换平台：看不到 Arcade 的板。
        let mut neogeo = ArcadeScreen::new(Platform::Neogeo);
        assert_eq!(neogeo.boards(&catalog, &carts).len(), 1);
        assert_eq!(neogeo.boards(&catalog, &carts)[0].stem, "mslug");
        neogeo.filter = ArcadeFilter::System("CPS-2".into());
        assert!(neogeo.boards(&catalog, &carts).is_empty());
    }

    #[test]
    fn selection_wraps_at_both_ends() {
        let mut screen = ArcadeScreen::new(Platform::Arcade);
        screen.select_prev(3);
        assert_eq!(screen.index, 2, "从第一块往回走要绕到末尾");
        screen.select_next(3);
        assert_eq!(screen.index, 0, "从末尾往前走要绕回第一块");
        screen.select_next(3);
        assert_eq!(screen.index, 1);
        // 空列表不 panic，下标保持 0。
        screen.set_index(5, 0);
        assert_eq!(screen.index, 0);
        screen.select_next(0);
        assert_eq!(screen.index, 0);
        screen.select_prev(0);
        assert_eq!(screen.index, 0);
        // 列表变短时夹紧。
        screen.set_index(9, 4);
        assert_eq!(screen.index, 3);
    }

    #[test]
    fn cycling_filters_returns_to_all() {
        let carts = arcade_carts();
        let catalog = arcade_catalog();
        let mut screen = ArcadeScreen::new(Platform::Arcade);
        let systems = screen.systems(&catalog, &carts);
        assert_eq!(systems, vec!["CPS-1", "CPS-2"], "按频次降序");

        screen.index = 2;
        screen.cycle_filter(&systems);
        assert_eq!(screen.filter, ArcadeFilter::System("CPS-1".into()));
        assert_eq!(screen.index, 0, "切筛选要回到第一块");

        screen.cycle_filter(&systems);
        assert_eq!(screen.filter, ArcadeFilter::System("CPS-2".into()));

        screen.cycle_filter(&systems);
        assert_eq!(screen.filter, ArcadeFilter::All, "转一圈回到全部");

        // 没有任何 system 时，切换保持 All（不会卡在空筛选里）。
        screen.cycle_filter(&[]);
        assert_eq!(screen.filter, ArcadeFilter::All);
    }

    #[test]
    fn a_missing_dat_keeps_every_board_under_all() {
        let carts = arcade_carts();
        let catalog = Catalog::empty(Platform::Arcade);
        let mut screen = ArcadeScreen::new(Platform::Arcade);

        assert_eq!(screen.boards(&catalog, &carts).len(), 3);
        assert!(
            screen.systems(&catalog, &carts).is_empty(),
            "没有 DAT 就没有筛选条"
        );
        // 空筛选条下切筛选：仍然 All，板子不丢。
        screen.cycle_filter(&screen.systems(&catalog, &carts));
        assert_eq!(screen.filter, ArcadeFilter::All);
        assert_eq!(screen.boards(&catalog, &carts).len(), 3);
    }

    #[test]
    fn neogeo_bios_is_detected_in_both_locations() {
        let card = tempfile::tempdir().unwrap();
        let root = card.path();
        assert_eq!(bios_state(root, Platform::Neogeo), BiosState::Missing);
        assert_eq!(
            bios_state(root, Platform::Arcade),
            BiosState::NotRequired,
            "Arcade 不需要 BIOS"
        );

        // 0 字节的空壳不算。
        std::fs::create_dir_all(root.join("BIOS/neogeo")).unwrap();
        std::fs::write(root.join("BIOS/fbneo/neogeo.zip").join("x"), b"").ok();
        std::fs::create_dir_all(root.join("BIOS/fbneo")).unwrap();
        std::fs::write(root.join("BIOS/fbneo/neogeo.zip"), b"").unwrap();
        assert_eq!(bios_state(root, Platform::Neogeo), BiosState::Missing);

        // 标准位置。
        std::fs::write(root.join("BIOS/fbneo/neogeo.zip"), b"PK\x03\x04 bios").unwrap();
        assert_eq!(bios_state(root, Platform::Neogeo), BiosState::Present);

        // 老位置（直接放 BIOS/ 下）也要认。
        std::fs::remove_file(root.join("BIOS/fbneo/neogeo.zip")).unwrap();
        std::fs::write(root.join("BIOS/neogeo.zip"), b"PK\x03\x04 bios").unwrap();
        assert_eq!(bios_state(root, Platform::Neogeo), BiosState::Present);
    }

    #[test]
    fn the_window_keeps_three_boards_in_view() {
        assert_eq!(visible_window(0, 0), 0..0, "空列表");
        assert_eq!(visible_window(0, 1), 0..1, "只有一块时不越界");
        assert_eq!(visible_window(1, 2), 0..2);
        assert_eq!(visible_window(0, 10), 0..3, "第一块：左边留空");
        assert_eq!(visible_window(1, 10), 0..3, "第二块仍然从 0 起");
        assert_eq!(visible_window(4, 10), 3..6, "中间：选中块居中");
        assert_eq!(visible_window(9, 10), 7..10, "最后一块：右边留空");
    }

    #[test]
    fn keys_map_to_actions_without_touching_the_other_screen() {
        let systems = vec!["CPS-1".to_string(), "CPS-2".to_string()];
        let mut screen = ArcadeScreen::new(Platform::Arcade);
        screen.index = 1;

        assert_eq!(screen.key(ArcadeKey::Left, 3, &systems), ArcadeAction::None);
        assert_eq!(screen.index, 0);
        assert_eq!(screen.key(ArcadeKey::Right, 3, &systems), ArcadeAction::None);
        assert_eq!(screen.index, 1);
        assert_eq!(screen.key(ArcadeKey::Favorite, 3, &systems), ArcadeAction::Favorite);
        assert_eq!(screen.key(ArcadeKey::Back, 3, &systems), ArcadeAction::Back);
        assert_eq!(screen.key(ArcadeKey::Insert, 3, &systems), ArcadeAction::Insert);
        assert_eq!(
            screen.key(ArcadeKey::Service, 3, &systems),
            ArcadeAction::None,
            "START 本阶段 no-op"
        );
        // A / START / Y / B 都不改选中项与筛选。
        assert_eq!(screen.index, 1);
        assert_eq!(screen.filter, ArcadeFilter::All);

        assert_eq!(screen.key(ArcadeKey::Filter, 3, &systems), ArcadeAction::None);
        assert_eq!(screen.filter, ArcadeFilter::System("CPS-1".into()));
        assert_eq!(screen.index, 0);
    }

    // ---------------------------------------------------------------- 启动计划（5b-1）

    #[test]
    fn the_launch_plan_points_at_the_zip_in_the_platform_folder() {
        let card = tempfile::tempdir().unwrap();
        let root = card.path();
        for platform in [Platform::Arcade, Platform::Neogeo] {
            let plan = launch_plan(root, platform, "mslug", "Metal Slug");
            assert_eq!(plan.cart.platform, platform);
            assert_eq!(plan.cart.stem, "mslug");
            assert_eq!(plan.cart.title, "Metal Slug");
            assert_eq!(plan.cart.code, "", "ZIP 没有卡带代码");
            assert_eq!(
                plan.cart.rom,
                root.join("Games").join(platform.dir_name()).join("mslug.zip"),
                "ROM 必须指向该平台目录下的 zip"
            );
        }
        // 没有标题时退回 stem。
        let plan = launch_plan(root, Platform::Arcade, "sf2", "  ");
        assert_eq!(plan.cart.title, "sf2");
    }

    #[test]
    fn a_fbneo_pack_puts_the_bios_folder_first() {
        let card = tempfile::tempdir().unwrap();
        let root = card.path();

        // 只有 BIOS/neogeo.zip（老布局）→ 退回 BIOS/。
        std::fs::create_dir_all(root.join("BIOS")).unwrap();
        std::fs::write(root.join("BIOS/neogeo.zip"), b"PK").unwrap();
        assert_eq!(
            launch_plan(root, Platform::Neogeo, "mslug", "").system_dir,
            Some(root.join("BIOS"))
        );

        // 什么都没有 → 仍然给 BIOS/（让核心自己报缺，而不是我们不传）。
        std::fs::remove_file(root.join("BIOS/neogeo.zip")).unwrap();
        assert_eq!(
            launch_plan(root, Platform::Neogeo, "mslug", "").system_dir,
            Some(root.join("BIOS"))
        );

        // `BIOS/fbneo` 是个**文件**（不是目录）→ 仍然退回 BIOS/。
        std::fs::write(root.join("BIOS/fbneo"), b"not a dir").unwrap();
        assert_eq!(
            launch_plan(root, Platform::Neogeo, "mslug", "").system_dir,
            Some(root.join("BIOS")),
            "同名文件不能当 BIOS 目录"
        );
        std::fs::remove_file(root.join("BIOS/fbneo")).unwrap();

        // 有 BIOS/fbneo/neogeo.zip → 优先 BIOS/fbneo/。
        std::fs::create_dir_all(root.join("BIOS/fbneo")).unwrap();
        std::fs::write(root.join("BIOS/fbneo/neogeo.zip"), b"PK").unwrap();
        assert_eq!(
            launch_plan(root, Platform::Neogeo, "mslug", "").system_dir,
            Some(root.join("BIOS/fbneo"))
        );
        assert_eq!(
            launch_plan(root, Platform::Arcade, "sf2", "").system_dir,
            Some(root.join("BIOS/fbneo")),
            "街机与 NeoGeo 共用 FBNeo 的 BIOS 目录"
        );
    }

    #[test]
    fn arcade_and_neogeo_get_their_own_canvas() {
        let card = tempfile::tempdir().unwrap();
        let root = card.path();
        // 与 `slot_model` 的机台表一致；真实尺寸由 `av_info` 在 load 后给。
        assert_eq!(
            launch_plan(root, Platform::Arcade, "sf2", "").canvas,
            (384, 224)
        );
        assert_eq!(
            launch_plan(root, Platform::Neogeo, "mslug", "").canvas,
            (320, 224)
        );
    }

    #[test]
    fn core_missing_is_recorded_and_cleared_by_the_next_action() {
        let mut screen = attracted();
        screen.note_core_missing();
        assert!(screen.core_missing, "失败要留一行红字");
        // START 成功（FREE PLAY）就清掉。
        assert!(screen.start());
        assert!(!screen.core_missing, "重新开一次应当清掉");

        screen.note_core_missing();
        assert!(screen.back(), "回 Main");
        assert!(!screen.core_missing, "back() 也要清");
    }

    // ---------------------------------------------------------------- 存档 / 按键（5b-2）

    #[test]
    fn arcade_saves_live_under_their_own_platform() {
        let card = tempfile::tempdir().unwrap();
        let root = card.path();
        let arcade = save_paths(root, Platform::Arcade, slot_store::Core::default_for(Platform::Arcade), "sf2");
        let neogeo = save_paths(root, Platform::Neogeo, slot_store::Core::default_for(Platform::Neogeo), "sf2");
        let gba = save_paths(root, Platform::Gba, slot_store::Core::default_for(Platform::Gba), "sf2");

        // 目录名来自 `platform.dir_name()`，与 `Games/` 一致。
        assert_eq!(arcade.sram, root.join("Saves/Arcade/sf2.sav"));
        assert_eq!(neogeo.sram, root.join("Saves/NeoGeo/sf2.sav"));
        assert_eq!(gba.sram, root.join("Saves/GBA/sf2.sav"));
        // 同一个 stem、三个平台，互不覆盖。
        assert_ne!(arcade.sram, neogeo.sram);
        assert_ne!(arcade.sram, gba.sram, "街机存档不能落进卡带目录");
        // 即时存档是 ring 目录：`States/<平台>/<核心>/<stem>/`。
        assert_eq!(arcade.state_dir, root.join("States/Arcade/fbneo/sf2"));
        assert_eq!(gba.state_dir, root.join("States/GBA/mgba/sf2"));
        assert_ne!(arcade.state_dir, gba.state_dir);
    }

    #[test]
    fn the_arcade_pad_maps_six_buttons_and_the_coin_key() {
        // 顺序就是 B1..B6；顺序错了会把"拳"和"跳"对调。
        assert_eq!(
            ARCADE_BUTTONS,
            [Btn::B, Btn::Y, Btn::A, Btn::X, Btn::L1, Btn::R1]
        );
        use slot_retro::ButtonMask;
        assert_eq!(arcade_pad(Btn::B), Some(ButtonMask::B), "B1");
        assert_eq!(arcade_pad(Btn::Y), Some(ButtonMask::Y), "B2");
        assert_eq!(arcade_pad(Btn::A), Some(ButtonMask::A), "B3");
        assert_eq!(arcade_pad(Btn::X), Some(ButtonMask::X), "B4");
        assert_eq!(arcade_pad(Btn::L1), Some(ButtonMask::L), "B5");
        assert_eq!(arcade_pad(Btn::R1), Some(ButtonMask::R), "B6");
        assert_eq!(arcade_pad(Btn::Start), Some(ButtonMask::START));
        assert_eq!(arcade_pad(Btn::Select), Some(ButtonMask::SELECT), "投币");
        // 方向键不在六键表里，但仍是方向。
        assert_eq!(arcade_pad(Btn::Up), None, "方向走另一条分支");
        assert_eq!(arcade_pad(Btn::Menu), None, "MENU 不该被当成游戏键");
    }

    /// 六键顺序只有一份：常量表 == 布局的 `buttons()`。
    #[test]
    fn the_button_table_and_the_layout_share_one_source() {
        use crate::input::PadLayout;
        assert_eq!(arcade_buttons(), ARCADE_BUTTONS);
        assert_eq!(PadLayout::Arcade.buttons(), &ARCADE_BUTTONS[..]);
        assert!(PadLayout::Default.buttons().is_empty(), "卡带没有六键概念");
        // 恰好 6 项、无重复。
        assert_eq!(ARCADE_BUTTONS.len(), 6);
        for (index, btn) in ARCADE_BUTTONS.iter().enumerate() {
            assert!(
                !ARCADE_BUTTONS[index + 1..].contains(btn),
                "{btn:?} 在六键表里出现了两次"
            );
        }
    }

    #[test]
    fn a_cartridge_cart_keeps_the_old_mapping() {
        use crate::input::PadLayout;
        // 方向键：两种布局都必须能走（街机表本身不接管方向，`Pad` 会退回卡带表）。
        let mut default = crate::input::Pad::default();
        default.apply_for(slot_input::Action::GbaDown(Btn::Up), PadLayout::Default);
        let mut arcade = crate::input::Pad::default();
        arcade.apply_for(slot_input::Action::GbaDown(Btn::Up), PadLayout::Arcade);
        assert_eq!(default.mask(), arcade.mask(), "方向键在街机布局里不能丢");
        assert_ne!(arcade.mask(), slot_retro::ButtonMask(0));

        // 六键表只声明那 8 个键：方向与 L2/R2 都不归它管。
        assert_eq!(arcade_pad(Btn::Up), None, "方向走另一条分支");
        assert_eq!(arcade_pad(Btn::L2), None);
        assert_eq!(arcade_pad(Btn::R2), None);

        // 卡带布局不受街机表影响：同一颗键在两张表里落到同一位。
        for btn in [Btn::A, Btn::B, Btn::X, Btn::Y, Btn::L1, Btn::R1, Btn::Start] {
            let mut default = crate::input::Pad::default();
            default.apply_for(slot_input::Action::GbaDown(btn), PadLayout::Default);
            let mut arcade = crate::input::Pad::default();
            arcade.apply_for(slot_input::Action::GbaDown(btn), PadLayout::Arcade);
            assert_eq!(
                default.mask(),
                arcade.mask(),
                "{btn:?} 在两种布局里应当落到同一位"
            );
            assert_ne!(default.mask(), slot_retro::ButtonMask(0));
        }
    }

    #[test]
    fn loading_state_uses_the_same_path_it_saved() {
        let card = tempfile::tempdir().unwrap();
        let root = card.path();
        // 写一份 SRAM 到"生产会读的那个位置"，再用同一个纯函数算回来。
        let plan = save_paths(root, Platform::Arcade, slot_store::Core::default_for(Platform::Arcade), "dino");
        std::fs::create_dir_all(plan.sram.parent().unwrap()).unwrap();
        std::fs::write(&plan.sram, b"SRAM").unwrap();
        assert_eq!(
            crate::persist::read_sav(root, Platform::Arcade, "dino"),
            Some(b"SRAM".to_vec()),
            "读档路径必须与 `save_paths` 算出的写档路径一致"
        );
        // 即时存档目录也走同一个函数。
        assert_eq!(
            save_paths(root, Platform::Arcade, slot_store::Core::default_for(Platform::Arcade), "dino")
                .state_dir,
            slot_store::state_dir(root, Platform::Arcade, slot_store::Core::Fbneo, "dino")
        );
    }

    // ---------------------------------------------------------------- 阶段机（5a）

    /// 走完插板动画：`insert` → 按 60fps 的节奏 `tick` 到 attract。
    ///
    /// 注意 `tick` 会夹 `dt` 上限（[`MAX_TICK`]），所以不能一次 `tick(0.19)`
    /// 就跳过去——那正是"掉帧不能一帧跑完动画"的保证。
    fn attracted() -> ArcadeScreen {
        let mut screen = ArcadeScreen::new(Platform::Arcade);
        assert!(screen.insert(1));
        for _ in 0..12 {
            screen.tick(1.0 / 60.0);
            if screen.stage() == ArcadeStage::Attract {
                return screen;
            }
        }
        panic!("12 帧（200ms）还没进 attract");
    }

    #[test]
    fn inserting_takes_180ms_and_then_attracts() {
        let mut screen = ArcadeScreen::new(Platform::Arcade);
        assert!(screen.insert(3), "Main + 有板 → 可以插");
        assert_eq!(screen.stage(), ArcadeStage::Inserting);

        // dt 被夹到 MAX_TICK：一帧 0.1s 只推进 0.05s。
        screen.tick(0.1);
        assert_eq!(screen.stage(), ArcadeStage::Inserting, "50ms 还不够");
        match screen.stage {
            Stage::Inserting { t } => assert!((t - MAX_TICK).abs() < 1e-6, "dt 没夹上限：{t}"),
            _ => panic!("still inserting"),
        }

        // 再推 3 帧（共 200ms ≥ 180ms）→ attract。
        screen.tick(0.05);
        screen.tick(0.05);
        assert_eq!(screen.stage(), ArcadeStage::Inserting, "150ms 仍在插板");
        screen.tick(0.05);
        assert_eq!(screen.stage(), ArcadeStage::Attract, "200ms 之后进 attract");
        match screen.stage {
            Stage::Attract {
                credits,
                coin_t,
                blink,
            } => {
                assert_eq!(credits, 0);
                assert_eq!(coin_t, 0.0);
                assert_eq!(blink, 0.0);
            }
            _ => panic!("attract"),
        }

        // 没有板的列表插不进去。
        let mut idle = ArcadeScreen::new(Platform::Arcade);
        assert!(!idle.insert(0));
        assert_eq!(idle.stage(), ArcadeStage::Main);

        // 掉帧也要夹住：一帧给 5 秒也只推进 MAX_TICK。
        let mut screen = ArcadeScreen::new(Platform::Arcade);
        screen.insert(1);
        screen.tick(5.0);
        match screen.stage {
            Stage::Inserting { t } => assert!(t <= MAX_TICK + 1e-6, "dt 没夹上限：{t}"),
            _ => panic!("一帧不该跑完整个动画"),
        }
    }

    #[test]
    fn no_input_is_accepted_while_inserting() {
        let mut screen = ArcadeScreen::new(Platform::Arcade);
        screen.index = 1;
        let systems = vec!["CPS-1".to_string()];
        screen.insert(3);

        assert!(!screen.coin(), "插板时不能投币");
        assert!(!screen.start(), "插板时不能开始");
        assert!(!screen.back(), "插板时不能返回");
        screen.select_next(3);
        assert_eq!(screen.index, 1, "插板时不能换板");
        screen.cycle_filter(&systems);
        assert_eq!(screen.filter, ArcadeFilter::All, "插板时不能换筛选");
        assert_eq!(
            screen.key(ArcadeKey::Filter, 3, &systems),
            ArcadeAction::None,
            "插板时按键表整体失效"
        );
        assert_eq!(screen.index, 1);
        assert_eq!(screen.stage(), ArcadeStage::Inserting);
    }

    /// 投一枚币并把限流窗口走完（`coin()` 只有在 `coin_t <= 0` 时才接受）。
    fn coin_after_window(screen: &mut ArcadeScreen) -> bool {
        for _ in 0..8 {
            screen.tick(COIN_SECS.max(MAX_TICK) / 8.0);
        }
        screen.coin()
    }

    #[test]
    fn coins_stop_at_nine() {
        let mut screen = attracted();
        for index in 0..MAX_CREDITS {
            assert!(coin_after_window(&mut screen), "第 {} 枚应当被接受", index + 1);
        }
        match screen.stage {
            Stage::Attract { credits, coin_t, .. } => {
                assert_eq!(credits, MAX_CREDITS);
                assert_eq!(coin_t, COIN_SECS, "每次被接受的投币都要重放 token 动画");
            }
            _ => panic!("attract"),
        }
        // 满员之后的三枚：拒绝，且不动 coin_t（不占限流窗口）。
        for _ in 0..3 {
            assert!(!coin_after_window(&mut screen), "满 9 枚后应当拒绝");
        }
        match screen.stage {
            Stage::Attract { credits, coin_t, .. } => {
                assert_eq!(credits, MAX_CREDITS);
                assert_eq!(coin_t, 0.0, "被拒的投币不该重置限流窗口");
            }
            _ => panic!("attract"),
        }
        // Main 里投币无效。
        let mut main = ArcadeScreen::new(Platform::Arcade);
        assert!(!main.coin());
    }

    #[test]
    fn free_play_starts_without_a_credit() {
        let mut screen = attracted();
        assert!(screen.free_play(), "默认 FREE PLAY");
        assert!(screen.start(), "FREE PLAY 下不投币也能开");
        match screen.stage {
            Stage::Attract { credits, .. } => assert_eq!(credits, 0, "FREE PLAY 不扣币"),
            _ => panic!("attract"),
        }
        assert_eq!(screen.stage(), ArcadeStage::Attract, "5a 不离开 attract");
    }

    #[test]
    fn a_credit_is_spent_when_the_coin_slot_is_on() {
        let mut screen = attracted();
        screen.free_play = false;
        assert!(!screen.start(), "没币不能开");
        assert_eq!(screen.stage(), ArcadeStage::Attract, "失败要留在 attract");

        assert!(coin_after_window(&mut screen));
        assert!(coin_after_window(&mut screen));
        match screen.stage {
            Stage::Attract { credits, .. } => assert_eq!(credits, 2),
            _ => panic!("attract"),
        }
        assert!(screen.start(), "有币就能开");
        match screen.stage {
            Stage::Attract { credits, .. } => assert_eq!(credits, 1, "START 扣一枚"),
            _ => panic!("attract"),
        }
    }

    #[test]
    fn back_returns_to_main_and_clears_the_stage() {
        let mut screen = attracted();
        coin_after_window(&mut screen);
        coin_after_window(&mut screen);
        assert!(screen.back());
        assert_eq!(screen.stage(), ArcadeStage::Main);
        // 再插板一次：credits 从 0 起，不带上一轮的币与 token。
        assert!(screen.insert(3));
        for _ in 0..12 {
            screen.tick(1.0 / 60.0);
        }
        match screen.stage {
            Stage::Attract { credits, coin_t, .. } => {
                assert_eq!(credits, 0);
                assert_eq!(coin_t, 0.0);
            }
            _ => panic!("attract"),
        }
        // Main 里 back() 无效。
        let mut main = ArcadeScreen::new(Platform::Arcade);
        assert!(!main.back());
    }

    /// 三个公开入口：闸门 / FREE PLAY 开关 / 币数读取。
    #[test]
    fn the_three_accessors_report_the_stage_faithfully() {
        let mut screen = ArcadeScreen::new(Platform::Arcade);
        assert!(screen.accepts_input(), "Main 收输入");
        assert_eq!(screen.credits(), 0);
        assert!(screen.free_play(), "默认 FREE PLAY");

        screen.insert(2);
        assert!(!screen.accepts_input(), "插板期间不收输入");
        assert!(screen.is_busy(), "两套闸门必须同向");
        for _ in 0..12 {
            screen.tick(1.0 / 60.0);
        }
        assert_eq!(screen.stage(), ArcadeStage::Attract);
        assert!(screen.accepts_input(), "attract 收输入");

        screen.set_free_play(false);
        assert!(!screen.free_play(), "set_free_play 生效");
        assert_eq!(screen.credits(), 0);
        coin_after_window(&mut screen);
        coin_after_window(&mut screen);
        assert_eq!(screen.credits(), 2, "credits() 能读到 Attract 的币数");
        screen.back();
        assert_eq!(screen.credits(), 0, "回 Main 后币数归 0");
    }

    #[test]
    fn an_empty_filter_list_shows_no_boards_and_does_not_panic() {
        let carts = arcade_carts();
        let catalog = arcade_catalog();
        let mut screen = ArcadeScreen::new(Platform::Arcade);
        screen.filter = ArcadeFilter::System("CAVE".into());
        assert!(screen.boards(&catalog, &carts).is_empty());
        assert!(screen.current(&catalog, &carts).is_none());
        screen.select_next(0);
        assert_eq!(screen.index, 0);
        assert_eq!(visible_window(0, 0), 0..0);
    }
}
