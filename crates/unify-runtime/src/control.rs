use std::sync::Arc;
use std::time::Duration;

use tokio::time::Instant;
use unify_core::geometry::ENTRY_INSET_PX;
use unify_core::keys::translate_event;
use unify_core::switcher::{CursorStep, EdgeDecision, RemoteCursor};
use unify_platform::{CaptureEvent, InjectEvent, PlatformError};
use unify_proto::{EntryPoint, InputEvent, Message, OsKind, RefuseReason, Side};

use crate::actor::{Actor, Control, Event};
use crate::status::CaptureState;

const REFUSAL_BACKOFF: Duration = Duration::from_secs(3);
const PEER_REFUSAL_CODES: [&str; 3] = ["peer_sharing_disabled", "peer_permission_missing", "peer_unavailable"];

impl Actor {
    fn peer_os(&self) -> OsKind {
        self.config.peer.as_ref().map_or(OsKind::Other, |peer| peer.os)
    }

    fn session_ready(&self) -> bool {
        self.active.is_some()
    }

    fn can_control(&self) -> bool {
        self.config.settings.input_sharing
            && self.session_ready()
            && self.control == Control::Local
            && !self.refused_until.is_some_and(|until| Instant::now() < until)
    }

    pub fn sync_capture(&mut self) {
        let wanted = self.config.settings.input_sharing;
        if !wanted {
            if let Some(capture) = self.capture.take() {
                capture.stop();
            }
            self.capture_shared.leave_remote();
            self.capture_state = CaptureState::Disabled;
            self.dirty = true;
            return;
        }
        if self.capture.is_some() {
            return;
        }
        let events = self.events_tx.clone();
        let sink = Arc::new(move |event: CaptureEvent| {
            let _ = events.send(Event::Capture(event));
        });
        match self.platform.start_capture(self.capture_shared.clone(), sink) {
            Ok(handle) => {
                self.capture = Some(handle);
                self.capture_state = CaptureState::Running;
            }
            Err(PlatformError::PermissionDenied(which)) => {
                tracing::warn!(permission = which, "input capture needs permission");
                self.capture_state = CaptureState::PermissionRequired;
            }
            Err(err) => {
                tracing::error!(error = %err, "input capture failed to start");
                self.capture_state = CaptureState::Failed;
                self.set_error("capture_failed");
            }
        }
        self.dirty = true;
    }

    fn ensure_injector(&mut self) -> Result<(), RefuseReason> {
        if self.injector.is_some() {
            return Ok(());
        }
        match self.platform.injector(self.capture_shared.clone()) {
            Ok(injector) => {
                self.injector = Some(injector);
                Ok(())
            }
            Err(PlatformError::PermissionDenied(_)) => {
                self.set_error("inject_permission");
                Err(RefuseReason::PermissionMissing)
            }
            Err(err) => {
                tracing::error!(error = %err, "input injector unavailable");
                self.set_error("inject_failed");
                Err(RefuseReason::Unavailable)
            }
        }
    }

    fn inject(&mut self, event: InjectEvent) {
        let Some(injector) = self.injector.as_ref() else {
            return;
        };
        if let Err(err) = injector.inject(event) {
            tracing::warn!(error = %err, "injection rejected by the operating system");
            self.set_error("inject_failed");
        }
    }

    pub fn on_capture(&mut self, event: CaptureEvent) {
        match event {
            CaptureEvent::LocalMotion { x, y } => self.on_local_motion(x, y),
            CaptureEvent::Input(input) => {
                if self.control == Control::Controlling {
                    let translated = translate_event(
                        input,
                        self.os,
                        self.peer_os(),
                        self.config.settings.swap_command_control,
                        self.config.settings.invert_scroll,
                    );
                    if self.send_active(Message::Input(translated)) {
                        self.stats.input_events_sent += 1;
                    }
                }
            }
            CaptureEvent::Hotkey => self.on_hotkey(),
            CaptureEvent::TapReenabled => tracing::warn!("event tap was disabled by the system and has been re-enabled"),
        }
    }

    fn on_local_motion(&mut self, x: i32, y: i32) {
        if !self.can_control() {
            return;
        }
        let Some(desktop) = self.desktop.as_ref() else {
            return;
        };
        let now = self.started.elapsed().as_millis() as u64;
        let can_switch = !self.capture_shared.local_buttons_down();
        if let EdgeDecision::Switch { ratio } = self.switcher.on_motion(desktop, x, y, now, can_switch) {
            let entry = EntryPoint::Edge { side: self.config.settings.peer_side.opposite(), ratio };
            self.start_controlling(entry, (x, y));
        }
    }

    fn park_point(&self, exit: (i32, i32)) -> (i32, i32) {
        match self.desktop.as_ref() {
            Some(desktop) if !self.platform.parks_cursor_at_exit() => desktop.center(),
            _ => self.point_inside_edge(exit),
        }
    }

    fn point_inside_edge(&self, exit: (i32, i32)) -> (i32, i32) {
        let Some(desktop) = self.desktop.as_ref() else {
            return exit;
        };
        let (x, y) = desktop.clamp(exit.0, exit.1);
        let inset = ENTRY_INSET_PX * 2;
        match self.config.settings.peer_side {
            Side::Right => (x - inset, y),
            Side::Left => (x + inset, y),
            Side::Top => (x, y + inset),
            Side::Bottom => (x, y - inset),
        }
    }

    fn start_controlling(&mut self, entry: EntryPoint, exit: (i32, i32)) {
        if !self.send_active(Message::Enter(entry)) {
            return;
        }
        if self.last_error.as_ref().is_some_and(|error| PEER_REFUSAL_CODES.contains(&error.code)) {
            self.last_error = None;
        }
        self.exit_point = Some(exit);
        let park = self.park_point(exit);
        self.capture_shared.enter_remote(park);
        if let Err(err) = self.platform.warp_cursor(park.0, park.1) {
            tracing::debug!(error = %err, "could not park cursor");
        }
        self.control = Control::Controlling;
        self.dirty = true;
        tracing::debug!("control moved to peer");
    }

    fn return_local(&mut self, warp_to: Option<(i32, i32)>) {
        self.capture_shared.leave_remote();
        if let Some((x, y)) = warp_to {
            if let Err(err) = self.platform.warp_cursor(x, y) {
                tracing::debug!(error = %err, "could not restore cursor");
            }
        }
        let now = self.started.elapsed().as_millis() as u64;
        self.switcher.on_returned(now);
        self.control = Control::Local;
        self.dirty = true;
    }

    fn release_injected(&mut self) {
        for event in self.injected.take_releases() {
            let inject = match event {
                InputEvent::Key { usage, down } => InjectEvent::Key { usage, down },
                InputEvent::MouseButton { button, down } => InjectEvent::Button { button, down },
                _ => continue,
            };
            self.inject(inject);
        }
        self.remote_cursor = None;
    }

    fn on_hotkey(&mut self) {
        match self.control {
            Control::Local => {
                if self.can_control() {
                    let exit = self.platform.cursor_position().unwrap_or((0, 0));
                    self.start_controlling(EntryPoint::Center, exit);
                }
            }
            Control::Controlling => {
                self.send_active(Message::Release);
                self.return_local(None);
            }
            Control::Controlled => {
                self.send_active(Message::Release);
                self.release_injected();
                self.control = Control::Local;
                self.dirty = true;
            }
        }
    }

    pub fn release_control_everywhere(&mut self) {
        match self.control {
            Control::Local => {}
            Control::Controlling => {
                self.send_active(Message::Release);
                self.return_local(None);
            }
            Control::Controlled => {
                self.send_active(Message::Release);
                self.release_injected();
                self.control = Control::Local;
                self.dirty = true;
            }
        }
    }

    pub fn on_session_lost(&mut self) {
        match self.control {
            Control::Local => {}
            Control::Controlling => self.return_local(None),
            Control::Controlled => {
                self.release_injected();
                self.control = Control::Local;
            }
        }
        self.dirty = true;
    }

    pub fn on_remote_enter(&mut self, entry: EntryPoint) {
        if self.control == Control::Controlling {
            if self.identity.device_id() < self.config.peer.as_ref().map_or("", |p| p.device_id.as_str()) {
                return;
            }
            self.return_local(None);
        }
        if !self.config.settings.input_sharing {
            self.send_active(Message::Refuse { reason: RefuseReason::SharingDisabled });
            return;
        }
        if let Err(reason) = self.ensure_injector() {
            self.send_active(Message::Refuse { reason });
            return;
        }
        self.refresh_desktop();
        let Some(desktop) = self.desktop.as_ref() else {
            self.send_active(Message::Refuse { reason: RefuseReason::Unavailable });
            return;
        };
        let cursor = RemoteCursor::enter(desktop, entry, self.config.settings.peer_side);
        let (x, y) = cursor.position();
        self.remote_cursor = Some(cursor);
        self.control = Control::Controlled;
        self.inject(InjectEvent::MoveTo { x, y, dx: 0, dy: 0 });
        self.dirty = true;
    }

    pub fn on_remote_leave(&mut self, _side: Side, ratio: u16) {
        if self.control != Control::Controlling {
            return;
        }
        let warp_to = self.desktop.as_ref().map(|desktop| {
            desktop.entry_point(EntryPoint::Edge { side: self.config.settings.peer_side, ratio })
        });
        self.return_local(warp_to);
    }

    pub fn on_remote_release(&mut self) {
        match self.control {
            Control::Controlling => {
                self.return_local(None);
                self.set_error("peer_released_control");
            }
            Control::Controlled => {
                self.release_injected();
                self.control = Control::Local;
                self.dirty = true;
            }
            Control::Local => {}
        }
    }

    pub fn on_remote_refuse(&mut self, reason: RefuseReason) {
        if self.control != Control::Controlling {
            return;
        }
        let restore = self.exit_point.take().map(|exit| self.point_inside_edge(exit));
        self.return_local(restore);
        self.refused_until = Some(Instant::now() + REFUSAL_BACKOFF);
        tracing::info!(?reason, "peer refused control");
        self.set_error(match reason {
            RefuseReason::SharingDisabled => "peer_sharing_disabled",
            RefuseReason::PermissionMissing => "peer_permission_missing",
            RefuseReason::Unavailable => "peer_unavailable",
        });
    }

    pub fn on_remote_input(&mut self, event: InputEvent) {
        if self.control != Control::Controlled {
            return;
        }
        self.stats.input_events_received += 1;
        match event {
            InputEvent::Key { usage, down } => {
                self.injected.record(&event);
                self.inject(InjectEvent::Key { usage, down });
            }
            InputEvent::MouseButton { button, down } => {
                self.injected.record(&event);
                self.inject(InjectEvent::Button { button, down });
            }
            InputEvent::Wheel { dx, dy } => self.inject(InjectEvent::Wheel { dx, dy }),
            InputEvent::MouseMove { dx, dy } => {
                let actual = self.platform.cursor_position();
                let (Some(desktop), Some(cursor)) = (self.desktop.as_ref(), self.remote_cursor.as_mut()) else {
                    return;
                };
                match cursor.step(desktop, actual, dx, dy) {
                    CursorStep::Move { x, y } => self.inject(InjectEvent::MoveTo { x, y, dx, dy }),
                    CursorStep::Leave { side, ratio } => {
                        self.send_active(Message::Leave { side, ratio });
                        self.release_injected();
                        self.control = Control::Local;
                        self.dirty = true;
                    }
                }
            }
        }
    }

    pub fn on_session_ready(&mut self) {
        self.flush_pending_clipboard();
    }
}
