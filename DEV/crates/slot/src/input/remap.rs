use slot_input::{Btn, RawEvent};
use slot_store::{ButtonLayout, Controls, Device, GameButton};

/// Remember the binding selected on press, so a menu or setting change cannot
/// strand a held shoulder. Multiple physical keys may share a logical button.
#[derive(Default)]
pub struct Remap {
    held: Vec<(Btn, Btn)>,
}

fn device_btn(device: Device) -> Btn {
    match device {
        Device::Up => Btn::Up,
        Device::Down => Btn::Down,
        Device::Left => Btn::Left,
        Device::Right => Btn::Right,
        Device::A => Btn::A,
        Device::B => Btn::B,
        Device::X => Btn::X,
        Device::Y => Btn::Y,
        Device::L1 => Btn::L1,
        Device::R1 => Btn::R1,
        Device::L2 => Btn::L2,
        Device::R2 => Btn::R2,
        Device::Select => Btn::Select,
        Device::Start => Btn::Start,
    }
}

fn game_btn(button: GameButton) -> Btn {
    match button {
        GameButton::Up => Btn::Up,
        GameButton::Down => Btn::Down,
        GameButton::Left => Btn::Left,
        GameButton::Right => Btn::Right,
        GameButton::A => Btn::A,
        GameButton::B => Btn::B,
        GameButton::Select => Btn::Select,
        GameButton::Start => Btn::Start,
        GameButton::L => Btn::L1,
        GameButton::R => Btn::R1,
        GameButton::X => Btn::X,
        GameButton::Y => Btn::Y,
        GameButton::TurboA => Btn::X,
        GameButton::TurboB => Btn::Y,
    }
}

/// Translate a physical key through the user's Controls file. Defaults are
/// identity, so an absent/old config keeps the old button behaviour.
///
/// `CONFIGURED` includes the Super Famicom / Mega Drive X/Y, so a hand-edited
/// swap (`x=y`, `y=x`) reaches the input layer.  A binding the user actually
/// changed wins over a default one, which keeps the older configs working:
/// e.g. `TurboA=Y` used to shadow the stock `Y=Y` row and still does.
fn controls_map(controls: &Controls, physical: Btn) -> Option<Btn> {
    let default = Controls::default();
    let matches = |button: GameButton, custom_only: bool| {
        let binding = controls.game(button)?;
        if binding.modifier.is_some() || device_btn(binding.button) != physical {
            return None;
        }
        if custom_only && Some(binding) == default.game(button) {
            return None;
        }
        Some(game_btn(button))
    };
    GameButton::CONFIGURED
        .iter()
        .copied()
        .find_map(|button| matches(button, true))
        .or_else(|| {
            GameButton::CONFIGURED
                .iter()
                .copied()
                .find_map(|button| matches(button, false))
        })
}

impl Remap {
    pub fn feed(&mut self, event: RawEvent, layout: ButtonLayout) -> Option<RawEvent> {
        self.feed_with(event, layout, &Controls::default())
    }

    pub fn feed_with(
        &mut self,
        event: RawEvent,
        layout: ButtonLayout,
        controls: &Controls,
    ) -> Option<RawEvent> {
        let (RawEvent::Down(key) | RawEvent::Up(key)) = event;
        let custom = if controls == &Controls::default() {
            None
        } else {
            controls_map(controls, key)
        };
        if custom.is_none() && !matches!(key, Btn::X | Btn::Y | Btn::L1 | Btn::R1 | Btn::L2 | Btn::R2)
        {
            return Some(event);
        }
        match event {
            RawEvent::Down(physical) => {
                if self.held.iter().any(|(p, _)| *p == physical) {
                    return None;
                }
                let logical = custom.unwrap_or(match (layout, physical) {
                    (ButtonLayout::SwapShoulders | ButtonLayout::Both, Btn::L1) => Btn::L2,
                    (ButtonLayout::SwapShoulders | ButtonLayout::Both, Btn::R1) => Btn::R2,
                    (ButtonLayout::SwapShoulders | ButtonLayout::Both, Btn::L2) => Btn::L1,
                    (ButtonLayout::SwapShoulders | ButtonLayout::Both, Btn::R2) => Btn::R1,
                    (ButtonLayout::XyShoulders | ButtonLayout::Both, Btn::X) => Btn::L1,
                    (ButtonLayout::XyShoulders | ButtonLayout::Both, Btn::Y) => Btn::R1,
                    _ => physical,
                });
                let already = self.held.iter().any(|(_, l)| *l == logical);
                self.held.push((physical, logical));
                (!already).then_some(RawEvent::Down(logical))
            }
            RawEvent::Up(physical) => {
                let i = self.held.iter().position(|(p, _)| *p == physical)?;
                let (_, logical) = self.held.remove(i);
                (!self.held.iter().any(|(_, l)| *l == logical)).then_some(RawEvent::Up(logical))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slot_store::Binding;

    #[test]
    fn a_custom_control_moves_the_physical_button() {
        let mut controls = Controls::default();
        controls.set_game(GameButton::A, Binding::plain(Device::B));
        let mut remap = Remap::default();
        assert_eq!(
            remap.feed_with(
                RawEvent::Down(Btn::B),
                ButtonLayout::Off,
                &controls
            ),
            Some(RawEvent::Down(Btn::A))
        );
        assert_eq!(
            remap.feed_with(RawEvent::Up(Btn::B), ButtonLayout::Off, &controls),
            Some(RawEvent::Up(Btn::A))
        );
    }

    #[test]
    fn a_swapped_x_and_y_reaches_the_super_famicom_buttons() {
        // The Super Famicom / Mega Drive can swap their extra face buttons
        // through controls.txt; the defaults leave them alone.
        let mut controls = Controls::default();
        controls.set_game(GameButton::X, Binding::plain(Device::Y));
        controls.set_game(GameButton::Y, Binding::plain(Device::X));
        let mut remap = Remap::default();
        assert_eq!(
            remap.feed_with(RawEvent::Down(Btn::Y), ButtonLayout::Off, &controls),
            Some(RawEvent::Down(Btn::X)),
            "physical Y did not become the console's X"
        );
        assert_eq!(
            remap.feed_with(RawEvent::Down(Btn::X), ButtonLayout::Off, &controls),
            Some(RawEvent::Down(Btn::Y)),
            "physical X did not become the console's Y"
        );
        assert_eq!(
            remap.feed_with(RawEvent::Up(Btn::Y), ButtonLayout::Off, &controls),
            Some(RawEvent::Up(Btn::X))
        );
        assert_eq!(
            remap.feed_with(RawEvent::Up(Btn::X), ButtonLayout::Off, &controls),
            Some(RawEvent::Up(Btn::Y))
        );
    }

    #[test]
    fn a_customised_turbo_alias_still_wins_over_the_stock_x_and_y() {
        // Older cards could bind 连发 A to the Y key.  The SFC's real X/Y rows
        // now default to Y, but an explicit turbo binding must keep working.
        let mut controls = Controls::default();
        controls.set_game(GameButton::TurboA, Binding::plain(Device::Y));
        let mut remap = Remap::default();
        assert_eq!(
            remap.feed_with(RawEvent::Down(Btn::Y), ButtonLayout::Off, &controls),
            Some(RawEvent::Down(Btn::X)),
            "the customised turbo A alias lost to the stock Y row"
        );
    }

    #[test]
    fn the_default_controls_keep_the_old_layout() {
        let mut remap = Remap::default();
        assert_eq!(
            remap.feed(RawEvent::Down(Btn::L1), ButtonLayout::SwapShoulders),
            Some(RawEvent::Down(Btn::L2))
        );
    }
}
