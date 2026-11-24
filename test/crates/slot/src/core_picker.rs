//! 主板上的核心选择动画。
//!
//! 选择器是“选项驱动”的：打开时传入当前机种**实际可用**的核心列表
//! （`slot_store::available_options`），左右键在列表内切换，芯片落位到
//! 对应插槽。GBA 用它自己的两个插槽；装第三颗核心时多一个板下备用托盘
//! （mGBA / gpSP / armGBA）。SFC 复用 Famicom 板上的前两个插槽
//! （snes9x + 可选的 supafaust）。单核心机种列表只有 1 项：板面只作信息展示。
//!
//! 2 核及以下沿用旧版“一颗芯片在两个插槽之间 hop”；3 核改用“槽 1 + 槽 2 +
//! 备用托盘”的高亮选择：槽 1 固定 mGBA，槽 2 是热插槽，托盘只放备用芯片；
//! A 在托盘上时与槽 2 对调，运行核心始终在板载插槽。

use slot_store::Core;
use slot_ui::{ease, Millis, Refusal, CHIP_TIP};

pub const SLIDE_MS: Millis = 160;
pub const LIFT_MS: Millis = 260;
pub const OPEN_MS: Millis = SLIDE_MS + LIFT_MS;
pub const CLOSE_MS: Millis = 320;
pub const HOP_MS: Millis = 180;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Press {
    Left,
    Right,
    Keep,
    Back,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Outcome {
    Nothing,
    Refused,
    Write(Core),
}

/// 3 核选择器的固定三座位布局。
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct ThreeLayout {
    pub slot1: Core,
    pub slot2: Core,
    pub tray: Core,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Chip {
    /// 芯片在选项之间的归一化位置：0 = 第 1 项，1 = 最后一项（单核心恒为 0.5）。
    /// 2 项时 0/1 就是两个插槽；GBA 3 项时 0/0.5/1 分别是槽 1、槽 2、托盘位。
    /// 3 核模式下高亮位置由 `selected_pos` 表示，这个字段只用于兼容旧界面。
    pub across: f32,
    pub lift: f32,
    pub tip: f32,
    pub seated: Option<Core>,
    pub shake: f32,
}

#[derive(Copy, Clone)]
struct Hop {
    from: usize,
    to: usize,
    started: Millis,
}

#[derive(Copy, Clone)]
struct Swap {
    started: Millis,
}

#[derive(Copy, Clone)]
struct Close {
    started: Millis,
    from: f32,
}

#[derive(Clone)]
pub struct CorePicker {
    options: Vec<Core>,
    /// 2 核模式下选中的 options 下标；3 核模式下是选中核心的 options 下标。
    seat: usize,
    /// 正在/将要运行的核心 index（3 核模式下贴“运行”标签）。
    active: usize,
    /// 3 核布局；None 表示旧的 1/2 核单芯片模型。
    three: Option<ThreeLayout>,
    /// 3 核模式下高亮的位置：0 槽 1，1 槽 2，2 托盘。
    selected_pos: usize,
    opened: Millis,
    started: Option<Millis>,
    hop: Option<Hop>,
    swap: Option<Swap>,
    close: Option<Close>,
    refusal: Option<Refusal>,
}

impl CorePicker {
    pub fn open(seat: Core, options: Vec<Core>, now: Millis) -> Self {
        let seat = options.iter().position(|core| *core == seat).unwrap_or(0);
        let mut picker = CorePicker {
            options,
            seat,
            active: seat,
            three: None,
            selected_pos: 0,
            opened: now,
            started: None,
            hop: None,
            swap: None,
            close: None,
            refusal: None,
        };
        if picker.options.len() > 2 {
            picker.configure_three(None);
        }
        picker
    }

    /// 3 核选择器：`alt` 是上次热插槽里的替代核心（gpSP / armGBA）。
    pub fn open_three(seat: Core, options: Vec<Core>, alt: Option<Core>, now: Millis) -> Self {
        let mut picker = CorePicker::open(seat, options, now);
        if picker.three.is_some() {
            picker.configure_three(alt);
        }
        picker
    }

    fn configure_three(&mut self, alt: Option<Core>) {
        let opts = &self.options;
        let Some(mgba) = opts.iter().position(|core| *core == Core::Mgba) else {
            self.three = None;
            return;
        };
        let Some(gpsp) = opts.iter().position(|core| *core == Core::Gpsp) else {
            self.three = None;
            return;
        };
        let Some(armgba) = opts.iter().position(|core| *core == Core::Armgba) else {
            self.three = None;
            return;
        };
        let active = self.active;
        let (slot2, tray) = if active != mgba {
            let other = if active == gpsp { armgba } else { gpsp };
            (active, other)
        } else {
            let chosen = alt
                .and_then(|core| opts.iter().position(|candidate| *candidate == core))
                .filter(|index| *index == gpsp || *index == armgba)
                .unwrap_or(gpsp);
            let other = if chosen == gpsp { armgba } else { gpsp };
            (chosen, other)
        };
        self.three = Some(ThreeLayout {
            slot1: opts[mgba],
            slot2: opts[slot2],
            tray: opts[tray],
        });
        self.selected_pos = if active == mgba {
            0
        } else {
            1
        };
    }

    /// 可选项（与板面上的插槽一一对应）。
    pub fn options(&self) -> &[Core] {
        &self.options
    }

    /// 选中的核心（3 核模式下是高亮的那颗）。
    pub fn seat(&self) -> Core {
        match self.three {
            Some(layout) => match self.selected_pos {
                0 => layout.slot1,
                1 => layout.slot2,
                _ => layout.tray,
            },
            None => self.options[self.seat],
        }
    }

    /// 当前实际运行的核心（3 核模式的“运行”标签）。
    pub fn active(&self) -> Core {
        self.options[self.active]
    }

    pub fn is_three(&self) -> bool {
        self.three.is_some()
    }

    pub fn layout(&self) -> Option<ThreeLayout> {
        self.three
    }

    pub fn selected_pos(&self) -> usize {
        self.selected_pos
    }

    /// 换芯动画的进度（0..1）；只有按下托盘芯片后的 180ms 内有值。
    pub fn swap_progress(&self, now: Millis) -> Option<f32> {
        let swap = self.swap?;
        let q = now.saturating_sub(swap.started) as f32 / HOP_MS as f32;
        (q < 1.0).then_some(q)
    }

    pub fn start(&mut self, now: Millis) {
        self.started.get_or_insert(now);
    }

    pub fn waiting(&self) -> bool {
        self.started.is_none()
    }

    pub fn waited(&self, now: Millis) -> Millis {
        now.saturating_sub(self.opened)
    }

    pub fn is_single(&self) -> bool {
        self.options.len() <= 1
    }

    pub fn closing(&self) -> bool {
        self.close.is_some()
    }

    pub fn openness(&self, now: Millis) -> f32 {
        match (self.close, self.started) {
            (Some(Close { started, from }), _) => {
                (from - now.saturating_sub(started) as f32 / CLOSE_MS as f32).max(0.0)
            }
            (None, Some(started)) => (now.saturating_sub(started) as f32 / OPEN_MS as f32).min(1.0),
            (None, None) => 0.0,
        }
    }

    pub fn finished(&self, now: Millis) -> bool {
        self.close.is_some() && self.openness(now) <= 0.0
    }

    pub fn press(&mut self, press: Press, now: Millis) -> Outcome {
        if self.close.is_some() {
            return Outcome::Nothing;
        }
        if self.is_single() {
            return match press {
                // A single-core board is information, not a choice: closing
                // it must not rewrite the core the cart already uses.
                Press::Keep | Press::Back => {
                    self.begin_close(now);
                    Outcome::Nothing
                }
                Press::Left | Press::Right => Outcome::Nothing,
            };
        }
        if self.three.is_some() {
            return self.press_three(press, now);
        }
        let target = match press {
            Press::Left => self.seat.checked_sub(1),
            Press::Right => {
                if self.seat + 1 < self.options.len() {
                    Some(self.seat + 1)
                } else {
                    None
                }
            }
            Press::Keep => {
                self.active = self.seat;
                self.begin_close(now);
                return Outcome::Write(self.seat());
            }
            Press::Back => {
                self.begin_close(now);
                return Outcome::Nothing;
            }
        };
        if self.waiting() {
            return Outcome::Nothing;
        }
        if let Some(progress) = self.hop_progress(now) {
            // Mid-hop: pressing toward the slot we are already heading to is
            // ignored; pressing back turns the chip round from where it is.
            let Some(target) = target else {
                return Outcome::Nothing;
            };
            if target == self.seat {
                return Outcome::Nothing;
            }
            let done = ((1.0 - progress) * HOP_MS as f32) as Millis;
            self.hop = Some(Hop {
                from: self.seat,
                to: target,
                started: now.saturating_sub(done),
            });
            self.seat = target;
            self.active = target;
            self.refusal = None;
            return Outcome::Nothing;
        }
        let Some(target) = target else {
            // At the edge of the options: shake instead of moving.
            self.refusal = Some(Refusal::started(now));
            return Outcome::Refused;
        };
        self.hop = Some(Hop {
            from: self.seat,
            to: target,
            started: now,
        });
        self.seat = target;
        self.active = target;
        self.refusal = None;
        Outcome::Nothing
    }

    fn press_three(&mut self, press: Press, now: Millis) -> Outcome {
        // A swap owns the pad until the picker closes.
        if self.swap.is_some() {
            return Outcome::Nothing;
        }
        if self.waiting() {
            return Outcome::Nothing;
        }
        match press {
            Press::Left => {
                if self.selected_pos == 0 {
                    self.refusal = Some(Refusal::started(now));
                    Outcome::Refused
                } else {
                    self.selected_pos -= 1;
                    self.seat = self
                        .options
                        .iter()
                        .position(|core| *core == self.seat())
                        .unwrap_or(self.seat);
                    self.refusal = None;
                    Outcome::Nothing
                }
            }
            Press::Right => {
                if self.selected_pos >= 2 {
                    self.refusal = Some(Refusal::started(now));
                    Outcome::Refused
                } else {
                    self.selected_pos += 1;
                    self.seat = self
                        .options
                        .iter()
                        .position(|core| *core == self.seat())
                        .unwrap_or(self.seat);
                    self.refusal = None;
                    Outcome::Nothing
                }
            }
            Press::Keep => {
                let selected = self.seat();
                if self.selected_pos == 2 {
                    let Some(layout) = self.three else {
                        return Outcome::Nothing;
                    };
                    let swapped = ThreeLayout {
                        slot1: layout.slot1,
                        slot2: layout.tray,
                        tray: layout.slot2,
                    };
                    self.three = Some(swapped);
                    self.selected_pos = 1;
                    self.seat = self
                        .options
                        .iter()
                        .position(|core| *core == swapped.slot2)
                        .unwrap_or(self.seat);
                    self.active = self.seat;
                    self.swap = Some(Swap { started: now });
                    // Keep the swapped arrangement on screen for the 180ms
                    // animation, then fade the page out.
                    self.close = Some(Close {
                        started: now.saturating_add(HOP_MS),
                        from: 1.0,
                    });
                    Outcome::Write(swapped.slot2)
                } else {
                    self.active = self.seat;
                    self.begin_close(now);
                    Outcome::Write(selected)
                }
            }
            Press::Back => {
                self.begin_close(now);
                Outcome::Nothing
            }
        }
    }

    pub fn chip(&self, now: Millis) -> Chip {
        let shake = self.refusal.map_or(0.0, |r| r.offset(now));
        if self.three.is_some() {
            return Chip {
                across: self.selected_pos as f32 / 2.0,
                lift: 0.0,
                tip: 0.0,
                seated: Some(self.seat()),
                shake,
            };
        }
        if let Some((from, to, q)) = self.hop_state(now) {
            let arc = (std::f32::consts::PI * q).sin();
            let rightward = to > from;
            let lean = if rightward { 1.0 } else { -1.0 };
            return Chip {
                across: self.across_between(from, to, q),
                lift: arc,
                tip: lean * CHIP_TIP * arc,
                seated: None,
                shake,
            };
        }
        Chip {
            across: if self.is_single() {
                0.5
            } else {
                self.seat as f32 / (self.options.len() - 1) as f32
            },
            lift: 0.0,
            tip: 0.0,
            seated: Some(self.seat()),
            shake,
        }
    }

    fn across_between(&self, from: usize, to: usize, q: f32) -> f32 {
        let denom = (self.options.len() - 1).max(1) as f32;
        let f0 = from as f32 / denom;
        let f1 = to as f32 / denom;
        // 线性插值（ease 已对称）：向右 0→1 得 ease(q)，向左 1→0 得 1-ease(q)，
        // 与旧版两条分支等价；掉头时 q_new = 1 - q_old 正好保持连续。
        f0 + (f1 - f0) * ease(q)
    }

    fn begin_close(&mut self, now: Millis) {
        self.close = Some(Close {
            started: now,
            from: self.openness(now),
        });
    }

    fn hop_state(&self, now: Millis) -> Option<(usize, usize, f32)> {
        let hop = self.hop?;
        let q = now.saturating_sub(hop.started) as f32 / HOP_MS as f32;
        (q < 1.0).then_some((hop.from, hop.to, q))
    }

    fn hop_progress(&self, now: Millis) -> Option<f32> {
        self.hop_state(now).map(|(_, _, q)| q)
    }
}
