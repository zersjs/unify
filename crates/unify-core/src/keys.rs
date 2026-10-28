use std::collections::BTreeSet;

use unify_proto::keymap::usage;
use unify_proto::{InputEvent, MouseButton, OsKind};

pub fn translate_usage(code: u16, sender: OsKind, receiver: OsKind, swap_command_control: bool) -> u16 {
    let crosses_mac_boundary = sender != receiver && (sender == OsKind::MacOs || receiver == OsKind::MacOs);
    if !(swap_command_control && crosses_mac_boundary) {
        return code;
    }
    match code {
        usage::LEFT_GUI => usage::LEFT_CTRL,
        usage::LEFT_CTRL => usage::LEFT_GUI,
        usage::RIGHT_GUI => usage::RIGHT_CTRL,
        usage::RIGHT_CTRL => usage::RIGHT_GUI,
        other => other,
    }
}

pub fn translate_event(event: InputEvent, sender: OsKind, receiver: OsKind, swap_command_control: bool, invert_scroll: bool) -> InputEvent {
    match event {
        InputEvent::Key { usage, down } => InputEvent::Key { usage: translate_usage(usage, sender, receiver, swap_command_control), down },
        InputEvent::Wheel { dx, dy } if invert_scroll => InputEvent::Wheel { dx: -dx, dy: -dy },
        other => other,
    }
}

fn button_rank(button: MouseButton) -> u8 {
    match button {
        MouseButton::Left => 0,
        MouseButton::Right => 1,
        MouseButton::Middle => 2,
        MouseButton::Back => 3,
        MouseButton::Forward => 4,
    }
}

fn button_from_rank(rank: u8) -> MouseButton {
    match rank {
        0 => MouseButton::Left,
        1 => MouseButton::Right,
        2 => MouseButton::Middle,
        3 => MouseButton::Back,
        _ => MouseButton::Forward,
    }
}

#[derive(Debug, Default)]
pub struct PressedState {
    keys: BTreeSet<u16>,
    buttons: BTreeSet<u8>,
}

impl PressedState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, event: &InputEvent) {
        match *event {
            InputEvent::Key { usage, down: true } => {
                self.keys.insert(usage);
            }
            InputEvent::Key { usage, down: false } => {
                self.keys.remove(&usage);
            }
            InputEvent::MouseButton { button, down: true } => {
                self.buttons.insert(button_rank(button));
            }
            InputEvent::MouseButton { button, down: false } => {
                self.buttons.remove(&button_rank(button));
            }
            _ => {}
        }
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty() && self.buttons.is_empty()
    }

    pub fn take_releases(&mut self) -> Vec<InputEvent> {
        let mut releases: Vec<InputEvent> = self
            .keys
            .iter()
            .rev()
            .map(|&usage| InputEvent::Key { usage, down: false })
            .collect();
        releases.extend(self.buttons.iter().map(|&rank| InputEvent::MouseButton { button: button_from_rank(rank), down: false }));
        self.keys.clear();
        self.buttons.clear();
        releases
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swaps_command_and_control_only_across_mac_boundary() {
        assert_eq!(translate_usage(usage::LEFT_GUI, OsKind::MacOs, OsKind::Windows, true), usage::LEFT_CTRL);
        assert_eq!(translate_usage(usage::LEFT_CTRL, OsKind::MacOs, OsKind::Windows, true), usage::LEFT_GUI);
        assert_eq!(translate_usage(usage::RIGHT_GUI, OsKind::Windows, OsKind::MacOs, true), usage::RIGHT_CTRL);
        assert_eq!(translate_usage(usage::LEFT_GUI, OsKind::MacOs, OsKind::Windows, false), usage::LEFT_GUI);
        assert_eq!(translate_usage(usage::LEFT_GUI, OsKind::Windows, OsKind::Windows, true), usage::LEFT_GUI);
        assert_eq!(translate_usage(usage::LEFT_ALT, OsKind::MacOs, OsKind::Windows, true), usage::LEFT_ALT);
        assert_eq!(translate_usage(0x06, OsKind::MacOs, OsKind::Windows, true), 0x06);
    }

    #[test]
    fn command_c_from_mac_becomes_ctrl_c_on_windows() {
        let down = translate_event(InputEvent::Key { usage: usage::LEFT_GUI, down: true }, OsKind::MacOs, OsKind::Windows, true, false);
        assert_eq!(down, InputEvent::Key { usage: usage::LEFT_CTRL, down: true });
    }

    #[test]
    fn invert_scroll_flips_both_axes() {
        let event = translate_event(InputEvent::Wheel { dx: 3, dy: -120 }, OsKind::MacOs, OsKind::Windows, false, true);
        assert_eq!(event, InputEvent::Wheel { dx: -3, dy: 120 });
    }

    #[test]
    fn pressed_state_releases_everything_held() {
        let mut state = PressedState::new();
        state.record(&InputEvent::Key { usage: usage::LEFT_SHIFT, down: true });
        state.record(&InputEvent::Key { usage: 0x04, down: true });
        state.record(&InputEvent::Key { usage: 0x04, down: true });
        state.record(&InputEvent::MouseButton { button: MouseButton::Left, down: true });
        state.record(&InputEvent::MouseButton { button: MouseButton::Back, down: true });
        state.record(&InputEvent::MouseButton { button: MouseButton::Back, down: false });
        let releases = state.take_releases();
        assert_eq!(
            releases,
            vec![
                InputEvent::Key { usage: usage::LEFT_SHIFT, down: false },
                InputEvent::Key { usage: 0x04, down: false },
                InputEvent::MouseButton { button: MouseButton::Left, down: false },
            ]
        );
        assert!(state.is_empty());
        assert!(state.take_releases().is_empty());
    }
}
