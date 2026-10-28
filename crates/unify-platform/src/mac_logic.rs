use unify_proto::keymap::usage;
use unify_proto::MouseButton;

pub const FLAG_ALPHA_SHIFT: u64 = 0x0001_0000;
pub const FLAG_SHIFT: u64 = 0x0002_0000;
pub const FLAG_CONTROL: u64 = 0x0004_0000;
pub const FLAG_ALTERNATE: u64 = 0x0008_0000;
pub const FLAG_COMMAND: u64 = 0x0010_0000;

const DEVICE_LEFT_CONTROL: u64 = 0x0000_0001;
const DEVICE_LEFT_SHIFT: u64 = 0x0000_0002;
const DEVICE_RIGHT_SHIFT: u64 = 0x0000_0004;
const DEVICE_LEFT_COMMAND: u64 = 0x0000_0008;
const DEVICE_RIGHT_COMMAND: u64 = 0x0000_0010;
const DEVICE_LEFT_ALTERNATE: u64 = 0x0000_0020;
const DEVICE_RIGHT_ALTERNATE: u64 = 0x0000_0040;
const DEVICE_RIGHT_CONTROL: u64 = 0x0000_2000;

pub const KEYCODE_CAPS_LOCK: u16 = 0x39;
pub const WHEEL_UNITS_PER_POINT: i64 = 12;
pub const DOUBLE_CLICK_INTERVAL_MS: u64 = 500;
pub const DOUBLE_CLICK_DISTANCE_PX: i32 = 4;

struct ModifierInfo {
    keycode: u16,
    usage: u16,
    device_mask: u64,
    independent_mask: u64,
}

const MODIFIERS: &[ModifierInfo] = &[
    ModifierInfo { keycode: 0x38, usage: usage::LEFT_SHIFT, device_mask: DEVICE_LEFT_SHIFT, independent_mask: FLAG_SHIFT },
    ModifierInfo { keycode: 0x3C, usage: usage::RIGHT_SHIFT, device_mask: DEVICE_RIGHT_SHIFT, independent_mask: FLAG_SHIFT },
    ModifierInfo { keycode: 0x3B, usage: usage::LEFT_CTRL, device_mask: DEVICE_LEFT_CONTROL, independent_mask: FLAG_CONTROL },
    ModifierInfo { keycode: 0x3E, usage: usage::RIGHT_CTRL, device_mask: DEVICE_RIGHT_CONTROL, independent_mask: FLAG_CONTROL },
    ModifierInfo { keycode: 0x3A, usage: usage::LEFT_ALT, device_mask: DEVICE_LEFT_ALTERNATE, independent_mask: FLAG_ALTERNATE },
    ModifierInfo { keycode: 0x3D, usage: usage::RIGHT_ALT, device_mask: DEVICE_RIGHT_ALTERNATE, independent_mask: FLAG_ALTERNATE },
    ModifierInfo { keycode: 0x37, usage: usage::LEFT_GUI, device_mask: DEVICE_LEFT_COMMAND, independent_mask: FLAG_COMMAND },
    ModifierInfo { keycode: 0x36, usage: usage::RIGHT_GUI, device_mask: DEVICE_RIGHT_COMMAND, independent_mask: FLAG_COMMAND },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagsChange {
    Modifier { usage: u16, down: bool },
    CapsLockToggle,
    Ignored,
}

pub fn flags_change(keycode: u16, flags: u64) -> FlagsChange {
    if keycode == KEYCODE_CAPS_LOCK {
        return FlagsChange::CapsLockToggle;
    }
    match MODIFIERS.iter().find(|m| m.keycode == keycode) {
        Some(m) => FlagsChange::Modifier { usage: m.usage, down: flags & m.device_mask != 0 },
        None => FlagsChange::Ignored,
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ModifierFlags {
    device: u64,
    caps_lock: bool,
}

impl ModifierFlags {
    pub fn apply(&mut self, code: u16, down: bool) -> bool {
        let Some(m) = MODIFIERS.iter().find(|m| m.usage == code) else {
            return false;
        };
        if down {
            self.device |= m.device_mask;
        } else {
            self.device &= !m.device_mask;
        }
        true
    }

    pub fn toggle_caps_lock(&mut self) {
        self.caps_lock = !self.caps_lock;
    }

    pub fn bits(&self) -> u64 {
        let mut bits = self.device;
        for m in MODIFIERS {
            if self.device & m.device_mask != 0 {
                bits |= m.independent_mask;
            }
        }
        if self.caps_lock {
            bits |= FLAG_ALPHA_SHIFT;
        }
        bits
    }
}

pub fn wheel_from_mac(point_delta_axis1: i64, point_delta_axis2: i64) -> (i32, i32) {
    let dy = point_delta_axis1.saturating_mul(WHEEL_UNITS_PER_POINT).clamp(-(1 << 20), 1 << 20) as i32;
    let dx = point_delta_axis2.saturating_mul(-WHEEL_UNITS_PER_POINT).clamp(-(1 << 20), 1 << 20) as i32;
    (dx, dy)
}

#[derive(Debug, Default, Clone, Copy)]
pub struct WheelAccumulator {
    remainder_x: f64,
    remainder_y: f64,
}

impl WheelAccumulator {
    pub fn to_mac_points(&mut self, dx: i32, dy: i32) -> (i32, i32) {
        let unit = WHEEL_UNITS_PER_POINT as f64;
        let y = f64::from(dy) / unit + self.remainder_y;
        let x = -f64::from(dx) / unit + self.remainder_x;
        let whole_y = y.trunc();
        let whole_x = x.trunc();
        self.remainder_y = y - whole_y;
        self.remainder_x = x - whole_x;
        (whole_y as i32, whole_x as i32)
    }
}

pub fn other_button_number(button: MouseButton) -> i64 {
    match button {
        MouseButton::Left => 0,
        MouseButton::Right => 1,
        MouseButton::Middle => 2,
        MouseButton::Back => 3,
        MouseButton::Forward => 4,
    }
}

pub fn button_from_number(number: i64) -> Option<MouseButton> {
    match number {
        0 => Some(MouseButton::Left),
        1 => Some(MouseButton::Right),
        2 => Some(MouseButton::Middle),
        3 => Some(MouseButton::Back),
        4 => Some(MouseButton::Forward),
        _ => None,
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ClickCounter {
    last: Option<(MouseButton, u64, i32, i32)>,
    count: i64,
}

impl ClickCounter {
    pub fn on_down(&mut self, button: MouseButton, now_ms: u64, x: i32, y: i32) -> i64 {
        let continues = matches!(self.last, Some((last_button, at, lx, ly))
            if last_button == button
                && now_ms.saturating_sub(at) <= DOUBLE_CLICK_INTERVAL_MS
                && (lx - x).abs() <= DOUBLE_CLICK_DISTANCE_PX
                && (ly - y).abs() <= DOUBLE_CLICK_DISTANCE_PX);
        self.count = if continues { self.count + 1 } else { 1 };
        self.last = Some((button, now_ms, x, y));
        self.count
    }

    pub fn current(&self) -> i64 {
        self.count.max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifier_press_and_release_from_device_bits() {
        assert_eq!(flags_change(0x38, FLAG_SHIFT | DEVICE_LEFT_SHIFT), FlagsChange::Modifier { usage: usage::LEFT_SHIFT, down: true });
        assert_eq!(flags_change(0x38, FLAG_SHIFT | DEVICE_RIGHT_SHIFT), FlagsChange::Modifier { usage: usage::LEFT_SHIFT, down: false });
        assert_eq!(flags_change(0x36, DEVICE_RIGHT_COMMAND), FlagsChange::Modifier { usage: usage::RIGHT_GUI, down: true });
        assert_eq!(flags_change(0x39, 0), FlagsChange::CapsLockToggle);
        assert_eq!(flags_change(0x3F, 0), FlagsChange::Ignored);
    }

    #[test]
    fn injected_flags_keep_independent_bit_while_either_side_held() {
        let mut flags = ModifierFlags::default();
        flags.apply(usage::LEFT_SHIFT, true);
        flags.apply(usage::RIGHT_SHIFT, true);
        flags.apply(usage::LEFT_SHIFT, false);
        assert_ne!(flags.bits() & FLAG_SHIFT, 0);
        flags.apply(usage::RIGHT_SHIFT, false);
        assert_eq!(flags.bits() & FLAG_SHIFT, 0);
        assert!(!flags.apply(0x04, true));
        flags.toggle_caps_lock();
        assert_ne!(flags.bits() & FLAG_ALPHA_SHIFT, 0);
    }

    #[test]
    fn wheel_conversion_roundtrips_through_accumulator() {
        let (dx, dy) = wheel_from_mac(10, -2);
        assert_eq!((dx, dy), (24, 120));
        let mut acc = WheelAccumulator::default();
        assert_eq!(acc.to_mac_points(dx, dy), (10, -2));
    }

    #[test]
    fn accumulator_keeps_fractional_windows_deltas() {
        let mut acc = WheelAccumulator::default();
        let mut total = 0;
        for _ in 0..10 {
            total += acc.to_mac_points(0, 30).0;
        }
        assert_eq!(total, 25);
    }

    #[test]
    fn click_counter_detects_double_click_only_when_close_in_time_and_space() {
        let mut counter = ClickCounter::default();
        assert_eq!(counter.on_down(MouseButton::Left, 0, 10, 10), 1);
        assert_eq!(counter.on_down(MouseButton::Left, 200, 11, 10), 2);
        assert_eq!(counter.on_down(MouseButton::Left, 300, 11, 10), 3);
        assert_eq!(counter.on_down(MouseButton::Left, 1000, 11, 10), 1);
        assert_eq!(counter.on_down(MouseButton::Left, 1100, 40, 10), 1);
        assert_eq!(counter.on_down(MouseButton::Right, 1150, 40, 10), 1);
    }

    #[test]
    fn button_numbers_roundtrip() {
        for button in [MouseButton::Left, MouseButton::Right, MouseButton::Middle, MouseButton::Back, MouseButton::Forward] {
            assert_eq!(button_from_number(other_button_number(button)), Some(button));
        }
        assert_eq!(button_from_number(9), None);
    }
}
