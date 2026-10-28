use std::collections::BTreeSet;

use unify_proto::keymap::{is_modifier, usage};
use unify_proto::MouseButton;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    PassThrough,
    Forward,
    Swallow,
    Hotkey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonAction {
    PassThrough,
    Forward,
}

fn modifier_bit(code: u16) -> u8 {
    1 << (code - usage::LEFT_CTRL)
}

fn button_bit(button: MouseButton) -> u8 {
    match button {
        MouseButton::Left => 1,
        MouseButton::Right => 2,
        MouseButton::Middle => 4,
        MouseButton::Back => 8,
        MouseButton::Forward => 16,
    }
}

#[derive(Debug)]
pub struct CaptureFilter {
    hotkey_enabled: bool,
    modifiers: u8,
    swallowing_space: bool,
    local_keys: BTreeSet<u16>,
    local_buttons: u8,
}

impl CaptureFilter {
    pub fn new(hotkey_enabled: bool) -> Self {
        Self { hotkey_enabled, modifiers: 0, swallowing_space: false, local_keys: BTreeSet::new(), local_buttons: 0 }
    }

    pub fn set_hotkey_enabled(&mut self, enabled: bool) {
        self.hotkey_enabled = enabled;
    }

    fn held(&self, left: u16, right: u16) -> bool {
        self.modifiers & (modifier_bit(left) | modifier_bit(right)) != 0
    }

    fn hotkey_chord_held(&self) -> bool {
        self.held(usage::LEFT_CTRL, usage::RIGHT_CTRL)
            && self.held(usage::LEFT_ALT, usage::RIGHT_ALT)
            && self.held(usage::LEFT_SHIFT, usage::RIGHT_SHIFT)
            && !self.held(usage::LEFT_GUI, usage::RIGHT_GUI)
    }

    pub fn on_key(&mut self, code: u16, down: bool, remote_active: bool) -> KeyAction {
        if is_modifier(code) {
            if down {
                self.modifiers |= modifier_bit(code);
            } else {
                self.modifiers &= !modifier_bit(code);
            }
        }
        if code == usage::SPACE {
            if self.swallowing_space {
                if !down {
                    self.swallowing_space = false;
                }
                return KeyAction::Swallow;
            }
            if down && self.hotkey_enabled && self.hotkey_chord_held() {
                self.swallowing_space = true;
                return KeyAction::Hotkey;
            }
        }
        if !remote_active {
            if down {
                self.local_keys.insert(code);
            } else {
                self.local_keys.remove(&code);
            }
            return KeyAction::PassThrough;
        }
        if !down && self.local_keys.remove(&code) {
            return KeyAction::PassThrough;
        }
        KeyAction::Forward
    }

    pub fn on_button(&mut self, button: MouseButton, down: bool, remote_active: bool) -> ButtonAction {
        let bit = button_bit(button);
        if !remote_active {
            if down {
                self.local_buttons |= bit;
            } else {
                self.local_buttons &= !bit;
            }
            return ButtonAction::PassThrough;
        }
        if !down && self.local_buttons & bit != 0 {
            self.local_buttons &= !bit;
            return ButtonAction::PassThrough;
        }
        ButtonAction::Forward
    }

    pub fn local_buttons_down(&self) -> bool {
        self.local_buttons != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press_chord(filter: &mut CaptureFilter, remote: bool) {
        filter.on_key(usage::LEFT_CTRL, true, remote);
        filter.on_key(usage::LEFT_ALT, true, remote);
        filter.on_key(usage::LEFT_SHIFT, true, remote);
    }

    #[test]
    fn local_keys_pass_through() {
        let mut filter = CaptureFilter::new(true);
        assert_eq!(filter.on_key(0x04, true, false), KeyAction::PassThrough);
        assert_eq!(filter.on_key(0x04, false, false), KeyAction::PassThrough);
    }

    #[test]
    fn remote_keys_are_forwarded() {
        let mut filter = CaptureFilter::new(true);
        assert_eq!(filter.on_key(0x04, true, true), KeyAction::Forward);
        assert_eq!(filter.on_key(0x04, false, true), KeyAction::Forward);
    }

    #[test]
    fn release_of_key_pressed_before_switch_goes_to_local_os() {
        let mut filter = CaptureFilter::new(true);
        filter.on_key(usage::LEFT_SHIFT, true, false);
        assert_eq!(filter.on_key(usage::LEFT_SHIFT, false, true), KeyAction::PassThrough);
        assert_eq!(filter.on_key(usage::LEFT_SHIFT, true, true), KeyAction::Forward);
        assert_eq!(filter.on_key(usage::LEFT_SHIFT, false, true), KeyAction::Forward);
    }

    #[test]
    fn hotkey_triggers_and_swallows_space_until_release() {
        let mut filter = CaptureFilter::new(true);
        press_chord(&mut filter, true);
        assert_eq!(filter.on_key(usage::SPACE, true, true), KeyAction::Hotkey);
        assert_eq!(filter.on_key(usage::SPACE, true, false), KeyAction::Swallow);
        assert_eq!(filter.on_key(usage::SPACE, false, false), KeyAction::Swallow);
        assert_eq!(filter.on_key(usage::SPACE, true, false), KeyAction::Hotkey);
        filter.on_key(usage::SPACE, false, false);
        filter.on_key(usage::LEFT_ALT, false, false);
        assert_eq!(filter.on_key(usage::SPACE, true, false), KeyAction::PassThrough);
    }

    #[test]
    fn hotkey_works_with_right_side_modifiers_and_not_with_gui() {
        let mut filter = CaptureFilter::new(true);
        filter.on_key(usage::RIGHT_CTRL, true, false);
        filter.on_key(usage::RIGHT_ALT, true, false);
        filter.on_key(usage::RIGHT_SHIFT, true, false);
        assert_eq!(filter.on_key(usage::SPACE, true, false), KeyAction::Hotkey);
        filter.on_key(usage::SPACE, false, false);
        filter.on_key(usage::LEFT_GUI, true, false);
        assert_eq!(filter.on_key(usage::SPACE, true, false), KeyAction::PassThrough);
    }

    #[test]
    fn hotkey_can_be_disabled() {
        let mut filter = CaptureFilter::new(false);
        press_chord(&mut filter, false);
        assert_eq!(filter.on_key(usage::SPACE, true, false), KeyAction::PassThrough);
    }

    #[test]
    fn partial_chord_does_not_trigger() {
        let mut filter = CaptureFilter::new(true);
        filter.on_key(usage::LEFT_CTRL, true, false);
        filter.on_key(usage::LEFT_ALT, true, false);
        assert_eq!(filter.on_key(usage::SPACE, true, false), KeyAction::PassThrough);
    }

    #[test]
    fn buttons_track_local_state_and_route_release() {
        let mut filter = CaptureFilter::new(true);
        assert_eq!(filter.on_button(MouseButton::Left, true, false), ButtonAction::PassThrough);
        assert!(filter.local_buttons_down());
        assert_eq!(filter.on_button(MouseButton::Left, false, true), ButtonAction::PassThrough);
        assert!(!filter.local_buttons_down());
        assert_eq!(filter.on_button(MouseButton::Right, true, true), ButtonAction::Forward);
        assert_eq!(filter.on_button(MouseButton::Right, false, true), ButtonAction::Forward);
    }
}
