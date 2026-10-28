use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation, CGEventType, CGMouseButton, EventField, ScrollEventUnit};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::geometry::CGPoint;
use unify_proto::keymap::{is_modifier, usage, usage_to_mac_keycode};
use unify_proto::MouseButton;

use crate::mac_logic::{other_button_number, ClickCounter, ModifierFlags, WheelAccumulator, KEYCODE_CAPS_LOCK};
use crate::{CaptureShared, InjectEvent, Injector, PlatformError, INJECTION_MARKER};

struct InjectState {
    shared: Arc<CaptureShared>,
    source: CGEventSource,
    flags: ModifierFlags,
    buttons: u8,
    clicks: ClickCounter,
    wheel: WheelAccumulator,
    started: Instant,
    position: CGPoint,
}

fn button_bit(button: MouseButton) -> u8 {
    1 << other_button_number(button)
}

impl InjectState {
    fn mark_and_post(&self, event: &CGEvent) {
        event.set_integer_value_field(EventField::EVENT_SOURCE_USER_DATA, INJECTION_MARKER as i64);
        event.set_flags(CGEventFlags::from_bits_retain(self.flags.bits()));
        event.post(CGEventTapLocation::HID);
    }

    fn current_position(&self) -> CGPoint {
        CGEvent::new(self.source.clone()).map(|event| event.location()).unwrap_or(self.position)
    }

    fn key(&mut self, code: u16, down: bool) -> Result<(), PlatformError> {
        let Some(keycode) = usage_to_mac_keycode(code, self.shared.iso_keyboard()) else {
            return Ok(());
        };
        if code == usage::CAPS_LOCK {
            if !down {
                return Ok(());
            }
            self.flags.toggle_caps_lock();
            let event = CGEvent::new_keyboard_event(self.source.clone(), KEYCODE_CAPS_LOCK, true)
                .map_err(|_| PlatformError::Os(String::from("CGEventCreateKeyboardEvent failed")))?;
            event.set_type(CGEventType::FlagsChanged);
            self.mark_and_post(&event);
            return Ok(());
        }
        let event = CGEvent::new_keyboard_event(self.source.clone(), keycode, down)
            .map_err(|_| PlatformError::Os(String::from("CGEventCreateKeyboardEvent failed")))?;
        if is_modifier(code) {
            self.flags.apply(code, down);
            event.set_type(CGEventType::FlagsChanged);
        }
        self.mark_and_post(&event);
        Ok(())
    }

    fn move_to(&mut self, x: i32, y: i32, dx: i32, dy: i32) -> Result<(), PlatformError> {
        let point = CGPoint::new(f64::from(x), f64::from(y));
        let (kind, button) = if self.buttons & button_bit(MouseButton::Left) != 0 {
            (CGEventType::LeftMouseDragged, CGMouseButton::Left)
        } else if self.buttons & button_bit(MouseButton::Right) != 0 {
            (CGEventType::RightMouseDragged, CGMouseButton::Right)
        } else if self.buttons != 0 {
            (CGEventType::OtherMouseDragged, CGMouseButton::Center)
        } else {
            (CGEventType::MouseMoved, CGMouseButton::Left)
        };
        let event = CGEvent::new_mouse_event(self.source.clone(), kind, point, button)
            .map_err(|_| PlatformError::Os(String::from("CGEventCreateMouseEvent failed")))?;
        event.set_integer_value_field(EventField::MOUSE_EVENT_DELTA_X, i64::from(dx));
        event.set_integer_value_field(EventField::MOUSE_EVENT_DELTA_Y, i64::from(dy));
        self.mark_and_post(&event);
        self.position = point;
        Ok(())
    }

    fn button(&mut self, button: MouseButton, down: bool) -> Result<(), PlatformError> {
        let point = self.current_position();
        let (kind, cg_button) = match (button, down) {
            (MouseButton::Left, true) => (CGEventType::LeftMouseDown, CGMouseButton::Left),
            (MouseButton::Left, false) => (CGEventType::LeftMouseUp, CGMouseButton::Left),
            (MouseButton::Right, true) => (CGEventType::RightMouseDown, CGMouseButton::Right),
            (MouseButton::Right, false) => (CGEventType::RightMouseUp, CGMouseButton::Right),
            (_, true) => (CGEventType::OtherMouseDown, CGMouseButton::Center),
            (_, false) => (CGEventType::OtherMouseUp, CGMouseButton::Center),
        };
        let clicks = if down {
            let now = self.started.elapsed().as_millis() as u64;
            self.clicks.on_down(button, now, point.x as i32, point.y as i32)
        } else {
            self.clicks.current()
        };
        if down {
            self.buttons |= button_bit(button);
        } else {
            self.buttons &= !button_bit(button);
        }
        let event = CGEvent::new_mouse_event(self.source.clone(), kind, point, cg_button)
            .map_err(|_| PlatformError::Os(String::from("CGEventCreateMouseEvent failed")))?;
        event.set_integer_value_field(EventField::MOUSE_EVENT_CLICK_STATE, clicks);
        event.set_integer_value_field(EventField::MOUSE_EVENT_BUTTON_NUMBER, other_button_number(button));
        self.mark_and_post(&event);
        Ok(())
    }

    fn wheel(&mut self, dx: i32, dy: i32) -> Result<(), PlatformError> {
        let (vertical, horizontal) = self.wheel.to_mac_points(dx, dy);
        if vertical == 0 && horizontal == 0 {
            return Ok(());
        }
        let event = CGEvent::new_scroll_event(self.source.clone(), ScrollEventUnit::PIXEL, 2, vertical, horizontal, 0)
            .map_err(|_| PlatformError::Os(String::from("CGEventCreateScrollWheelEvent2 failed")))?;
        self.mark_and_post(&event);
        Ok(())
    }

    fn apply(&mut self, event: InjectEvent) -> Result<(), PlatformError> {
        match event {
            InjectEvent::Key { usage, down } => self.key(usage, down),
            InjectEvent::MoveTo { x, y, dx, dy } => self.move_to(x, y, dx, dy),
            InjectEvent::Button { button, down } => self.button(button, down),
            InjectEvent::Wheel { dx, dy } => self.wheel(dx, dy),
        }
    }
}

pub struct MacInjector {
    sender: Mutex<mpsc::Sender<InjectEvent>>,
}

impl MacInjector {
    pub fn start(shared: Arc<CaptureShared>) -> Result<Self, PlatformError> {
        let (sender, receiver) = mpsc::channel::<InjectEvent>();
        let (ready_tx, ready_rx) = mpsc::channel::<Result<(), PlatformError>>();
        std::thread::Builder::new()
            .name("unify-injector".into())
            .spawn(move || {
                let source = match CGEventSource::new(CGEventSourceStateID::HIDSystemState) {
                    Ok(source) => source,
                    Err(()) => {
                        let _ = ready_tx.send(Err(PlatformError::Os(String::from("CGEventSourceCreate failed"))));
                        return;
                    }
                };
                let mut state = InjectState {
                    shared,
                    source,
                    flags: ModifierFlags::default(),
                    buttons: 0,
                    clicks: ClickCounter::default(),
                    wheel: WheelAccumulator::default(),
                    started: Instant::now(),
                    position: CGPoint::new(0.0, 0.0),
                };
                let _ = ready_tx.send(Ok(()));
                while let Ok(event) = receiver.recv() {
                    if let Err(err) = state.apply(event) {
                        tracing::warn!(error = %err, "input injection failed");
                    }
                }
            })
            .map_err(|err| PlatformError::Os(err.to_string()))?;
        ready_rx.recv().map_err(|_| PlatformError::Os(String::from("injector thread exited during startup")))??;
        Ok(Self { sender: Mutex::new(sender) })
    }
}

impl Injector for MacInjector {
    fn inject(&self, event: InjectEvent) -> Result<(), PlatformError> {
        let guard = self.sender.lock().map_err(|_| PlatformError::NotRunning)?;
        guard.send(event).map_err(|_| PlatformError::NotRunning)
    }
}
