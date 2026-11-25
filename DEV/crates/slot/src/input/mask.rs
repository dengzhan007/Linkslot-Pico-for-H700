use slot_input::{Action, Btn};
use slot_retro::ButtonMask;

/// 按键布局。`Default` 是卡带的同名映射，`Arcade` 是 MAME 的六键顺序。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum PadLayout {
    #[default]
    Default,
    /// 街机 / NeoGeo（FBNeo）：B1..B6 见 `arcade::ARCADE_BUTTONS`。
    Arcade,
}

impl PadLayout {
    /// 这套布局的物理键顺序（B1..B6）。`Arcade` 直接返回那张常量表，
    /// `Default`（卡带）没有六键概念，返回空切片。
    ///
    /// 存在的意义：验收能断言"表 → 布局"没有第二次手写
    /// （`arcade_buttons() == PadLayout::Arcade.buttons()`）。
    pub fn buttons(self) -> &'static [Btn] {
        match self {
            PadLayout::Arcade => &crate::arcade::ARCADE_BUTTONS,
            PadLayout::Default => &[],
        }
    }
}

#[derive(Default)]
pub struct Pad {
    mask: u16,
}

impl Pad {
    /// 卡带布局（保持既有行为，旧调用点不用改）。
    pub fn apply(&mut self, action: Action) {
        self.apply_for(action, PadLayout::Default);
    }

    /// 按指定布局按下/松开。街机布局只表在 `arcade::arcade_pad`，这里不做第二份映射。
    pub fn apply_for(&mut self, action: Action, layout: PadLayout) {
        match action {
            Action::GbaDown(b) => {
                if let Some(bit) = mapped(b, layout) {
                    self.mask |= bit;
                }
            }
            Action::GbaUp(b) => {
                if let Some(bit) = mapped(b, layout) {
                    self.mask &= !bit;
                }
            }
            _ => {}
        }
    }

    pub fn mask(&self) -> ButtonMask {
        ButtonMask(self.mask)
    }

    pub fn clear(&mut self) {
        self.mask = 0;
    }
}

fn mapped(btn: Btn, layout: PadLayout) -> Option<u16> {
    match layout {
        PadLayout::Default => bit(btn),
        // 街机表只接管六键 + START/SELECT；方向键仍走卡带那张表
        // （`arcade_pad` 对方向返回 `None`，不是"街机不能用方向"）。
        PadLayout::Arcade => crate::arcade::arcade_pad(btn).or_else(|| bit(btn)),
    }
}

fn bit(btn: Btn) -> Option<u16> {
    Some(match btn {
        Btn::Up => ButtonMask::UP,
        Btn::Down => ButtonMask::DOWN,
        Btn::Left => ButtonMask::LEFT,
        Btn::Right => ButtonMask::RIGHT,
        Btn::A => ButtonMask::A,
        Btn::B => ButtonMask::B,
        Btn::X => ButtonMask::X,
        Btn::Y => ButtonMask::Y,
        Btn::L1 => ButtonMask::L,
        Btn::R1 => ButtonMask::R,
        Btn::Start => ButtonMask::START,
        Btn::Select => ButtonMask::SELECT,
        _ => return None,
    })
}
