use std::net::{SocketAddr, UdpSocket as StdUdpSocket};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::net::UdpSocket;
use tokio::task::JoinHandle;
use tokio::time::Instant;
use unify_core::keys::translate_event;
use unify_platform::viewer_logic::stream_size;
use unify_platform::{EncodedFrame, InjectEvent, PlatformError, ScreenCapture, ScreenSettings, Viewer, ViewerEvent};
use unify_proto::{ratio_to_f64, Message, RefuseReason};
use unify_stream::{derive_stream_key, packetize, ClockSync, Direction, FrameAssembler, Header, LatencyWindow, Opener, Sealer};

use crate::actor::{now_us, Actor, Control, Event};
use crate::status::{StreamState, StreamStatus};
use crate::RuntimeError;

const TARGET_FPS: u16 = 60;
const MIN_BITRATE_BPS: u64 = 8_000_000;
const MAX_BITRATE_BPS: u64 = 60_000_000;
const BITS_PER_PIXEL_TENTHS: u64 = 1;
const KEYFRAME_REQUEST_INTERVAL: Duration = Duration::from_millis(250);
const LATENCY_SAMPLES: usize = 240;
const RECEIVE_BUFFER_LEN: usize = 2_048;

pub(crate) type SharedViewer = Arc<Mutex<Option<Box<dyn Viewer>>>>;

pub(crate) struct ViewerSession {
    id: u64,
    key: [u8; 32],
    socket: Arc<UdpSocket>,
    receiver: Option<JoinHandle<()>>,
    viewer: SharedViewer,
    size: (u32, u32),
    clock: ClockSync,
    latency: LatencyWindow,
    presented: u64,
    fps_mark: (Instant, u64),
    fps: u32,
    dropped: Arc<AtomicU64>,
    last_keyframe_request: Option<Instant>,
}

pub(crate) struct SharingSession {
    id: u64,
    capture: Option<Box<dyn ScreenCapture>>,
    size: (u32, u32),
    last_pointer: Option<(i32, i32)>,
}

pub(crate) type SharingResult = Result<(Box<dyn ScreenCapture>, (u32, u32)), PlatformError>;

fn bitrate_for(size: (u32, u32), fps: u16) -> u32 {
    let raw = u64::from(size.0) * u64::from(size.1) * u64::from(fps) * BITS_PER_PIXEL_TENTHS / 10;
    raw.clamp(MIN_BITRATE_BPS, MAX_BITRATE_BPS) as u32
}

fn frame_sender(socket: StdUdpSocket, key: &[u8; 32]) -> impl Fn(EncodedFrame) + Send + Sync + 'static {
    let state = Mutex::new((Sealer::new(key, Direction::Video), 1u32, Vec::with_capacity(RECEIVE_BUFFER_LEN)));
    move |frame: EncodedFrame| {
        let Ok(mut guard) = state.lock() else {
            return;
        };
        let (sealer, next_frame, packet) = &mut *guard;
        let frame_id = *next_frame;
        *next_frame = next_frame.wrapping_add(1);
        let Ok(chunks) = packetize(&frame.data) else {
            tracing::warn!(bytes = frame.data.len(), "encoded frame too large to send");
            return;
        };
        for chunk in chunks {
            let header = Header {
                keyframe: frame.keyframe,
                sequence: 0,
                frame_id,
                chunk_index: chunk.index,
                chunk_count: chunk.count,
                capture_us: frame.capture_us,
            };
            if sealer.seal(header, chunk.data, packet).is_err() {
                return;
            }
            if let Err(err) = socket.send(packet) {
                tracing::debug!(error = %err, "video packet not sent");
            }
        }
    }
}

impl Actor {
    fn active_binding(&self) -> Option<([u8; 32], std::net::IpAddr)> {
        let conn = self.active.and_then(|id| self.conns.get(&id))?;
        Some((conn.info.channel_binding, conn.info.remote_addr.ip()))
    }

    pub fn streaming(&self) -> bool {
        self.viewing.is_some() || self.sharing.is_some()
    }

    pub async fn start_viewer(&mut self) -> Result<(), RuntimeError> {
        if self.streaming() {
            return Ok(());
        }
        let (binding, _) = self.active_binding().ok_or(RuntimeError::new("not_connected"))?;
        let size = self.desktop.as_ref().map(|desktop| desktop.primary()).ok_or(RuntimeError::new("stream_failed"))?;
        let socket = UdpSocket::bind(SocketAddr::new(self.bind_ip, 0)).await.map_err(|_| RuntimeError::new("stream_failed"))?;
        let port = socket.local_addr().map_err(|_| RuntimeError::new("stream_failed"))?.port();
        let id = rand::random::<u64>();
        self.viewing = Some(ViewerSession {
            id,
            key: derive_stream_key(&binding, id),
            socket: Arc::new(socket),
            receiver: None,
            viewer: Arc::new(Mutex::new(None)),
            size: (0, 0),
            clock: ClockSync::new(),
            latency: LatencyWindow::new(LATENCY_SAMPLES),
            presented: 0,
            fps_mark: (Instant::now(), 0),
            fps: 0,
            dropped: Arc::new(AtomicU64::new(0)),
            last_keyframe_request: None,
        });
        self.send_active(Message::StreamRequest {
            session_id: id,
            udp_port: port,
            width: size.width.max(1) as u32,
            height: size.height.max(1) as u32,
            fps: TARGET_FPS,
        });
        self.dirty = true;
        Ok(())
    }

    pub fn stop_viewer(&mut self, notify_peer: bool) {
        let Some(mut session) = self.viewing.take() else {
            return;
        };
        if notify_peer {
            self.send_active(Message::StreamStop { session_id: session.id });
        }
        if let Some(receiver) = session.receiver.take() {
            receiver.abort();
        }
        let viewer = session.viewer.lock().ok().and_then(|mut guard| guard.take());
        if let Some(viewer) = viewer {
            tokio::task::spawn_blocking(move || viewer.stop());
        }
        self.dirty = true;
    }

    pub fn on_stream_accepted(&mut self, session_id: u64, width: u32, height: u32) {
        if !self.viewing.as_ref().is_some_and(|session| session.id == session_id) {
            return;
        }
        let events = self.events_tx.clone();
        let sink = Arc::new(move |event: ViewerEvent| {
            let _ = events.send(Event::Viewer(event));
        });
        let title = self.config.peer.as_ref().map_or_else(|| String::from("Unify"), |peer| format!("Unify — {}", peer.device_name));
        let viewer = match self.platform.start_viewer(&title, (width, height), sink) {
            Ok(viewer) => viewer,
            Err(err) => {
                tracing::error!(error = %err, "viewer could not start");
                self.set_error("stream_failed");
                self.stop_viewer(true);
                return;
            }
        };
        let Some(session) = self.viewing.as_mut() else {
            return;
        };
        session.size = (width, height);
        if let Ok(mut guard) = session.viewer.lock() {
            *guard = Some(viewer);
        }
        let socket = session.socket.clone();
        let shared = session.viewer.clone();
        let dropped = session.dropped.clone();
        let key = session.key;
        let events = self.events_tx.clone();
        session.receiver = Some(tokio::spawn(async move {
            let mut opener = Opener::new(&key, Direction::Video);
            let mut assembler = FrameAssembler::new();
            let mut buffer = [0u8; RECEIVE_BUFFER_LEN];
            loop {
                let Ok((length, _)) = socket.recv_from(&mut buffer).await else {
                    return;
                };
                let Ok((header, payload)) = opener.open(&mut buffer[..length]) else {
                    continue;
                };
                let output = assembler.push(header, payload);
                dropped.store(assembler.dropped_frames(), Ordering::Relaxed);
                let mut need_keyframe = output.request_keyframe;
                if let Some(frame) = output.frame {
                    let encoded = EncodedFrame { data: frame.data, keyframe: frame.keyframe, capture_us: frame.capture_us };
                    let accepted = shared.lock().ok().and_then(|guard| guard.as_ref().map(|viewer| viewer.submit(encoded)));
                    need_keyframe |= accepted == Some(false);
                }
                if need_keyframe && events.send(Event::StreamKeyframeNeeded).is_err() {
                    return;
                }
            }
        }));
        self.dirty = true;
    }

    pub fn on_stream_refused(&mut self, session_id: u64, reason: RefuseReason) {
        if !self.viewing.as_ref().is_some_and(|session| session.id == session_id) {
            return;
        }
        self.stop_viewer(false);
        self.set_error(match reason {
            RefuseReason::SharingDisabled => "peer_sharing_disabled",
            RefuseReason::PermissionMissing => "peer_permission_missing",
            RefuseReason::ScreenPermissionMissing => "peer_screen_permission",
            RefuseReason::Unavailable => "peer_unavailable",
        });
    }

    pub fn request_stream_keyframe(&mut self) {
        let now = Instant::now();
        let Some(session) = self.viewing.as_mut() else {
            return;
        };
        if session.last_keyframe_request.is_some_and(|last| now.duration_since(last) < KEYFRAME_REQUEST_INTERVAL) {
            return;
        }
        session.last_keyframe_request = Some(now);
        let session_id = session.id;
        self.send_active(Message::StreamKeyframe { session_id });
    }

    pub fn on_viewer_event(&mut self, event: ViewerEvent) {
        if self.viewing.is_none() {
            return;
        }
        match event {
            ViewerEvent::Input(input) => {
                let translated = translate_event(
                    input,
                    self.os,
                    self.config.peer.as_ref().map_or(unify_proto::OsKind::Other, |peer| peer.os),
                    self.config.settings.swap_command_control,
                    self.config.settings.invert_scroll,
                );
                self.send_active(Message::Input(translated));
                self.stats.input_events_sent += 1;
            }
            ViewerEvent::Pointer { x, y } => {
                self.send_active(Message::ViewerPointer { x, y });
            }
            ViewerEvent::FocusLost => {
                self.send_active(Message::ViewerRelease);
            }
            ViewerEvent::KeyframeNeeded => self.request_stream_keyframe(),
            ViewerEvent::Presented { capture_us, presented_us } => {
                if let Some(session) = self.viewing.as_mut() {
                    session.presented += 1;
                    if let Some(local_capture) = session.clock.to_local_us(capture_us) {
                        session.latency.add(presented_us.saturating_sub(local_capture).min(u64::from(u32::MAX)) as u32);
                    }
                }
            }
            ViewerEvent::Closed => self.stop_viewer(true),
            ViewerEvent::Failed => {
                self.set_error("stream_failed");
                self.stop_viewer(true);
            }
        }
    }

    pub fn on_stream_request(&mut self, session_id: u64, udp_port: u16, width: u32, height: u32, fps: u16) {
        if self.streaming() {
            self.send_active(Message::StreamRefused { session_id, reason: RefuseReason::Unavailable });
            return;
        }
        if !self.config.settings.input_sharing {
            self.send_active(Message::StreamRefused { session_id, reason: RefuseReason::SharingDisabled });
            return;
        }
        if let Err(reason) = self.ensure_injector() {
            self.send_active(Message::StreamRefused { session_id, reason });
            return;
        }
        let Some((binding, peer_ip)) = self.active_binding() else {
            return;
        };
        self.sharing = Some(SharingSession { id: session_id, capture: None, size: (0, 0), last_pointer: None });
        self.dirty = true;
        let key = derive_stream_key(&binding, session_id);
        let platform = self.platform.clone();
        let events = self.events_tx.clone();
        let target = (width.clamp(320, 7_680), height.clamp(200, 4_320));
        let fps = fps.clamp(15, 120);
        tokio::task::spawn_blocking(move || {
            let result = (|| -> SharingResult {
                let pixels = platform.match_display_mode(target.0, target.1)?;
                let size = stream_size(pixels, target);
                let socket = StdUdpSocket::bind(SocketAddr::new(unspecified_for(peer_ip), 0)).map_err(|err| PlatformError::Os(err.to_string()))?;
                socket.connect(SocketAddr::new(peer_ip, udp_port)).map_err(|err| PlatformError::Os(err.to_string()))?;
                let settings = ScreenSettings { width: size.0, height: size.1, fps: u32::from(fps), bitrate_bps: bitrate_for(size, fps) };
                let capture = platform.start_screen_capture(settings, Arc::new(frame_sender(socket, &key)))?;
                Ok((capture, size))
            })();
            let _ = events.send(Event::SharingStarted { session_id, result });
        });
    }

    pub fn on_sharing_started(&mut self, session_id: u64, result: SharingResult) {
        let current = self.sharing.as_ref().is_some_and(|session| session.id == session_id);
        match result {
            Ok((capture, size)) if current => {
                if let Some(session) = self.sharing.as_mut() {
                    session.capture = Some(capture);
                    session.size = size;
                }
                self.control = Control::Controlled;
                self.send_active(Message::StreamAccepted { session_id, width: size.0, height: size.1 });
            }
            Ok((capture, _)) => {
                tokio::task::spawn_blocking(move || capture.stop());
            }
            Err(err) => {
                tracing::warn!(error = %err, "screen sharing could not start");
                self.sharing = None;
                self.platform.restore_display_mode();
                let reason = match err {
                    PlatformError::PermissionDenied(_) => RefuseReason::ScreenPermissionMissing,
                    _ => RefuseReason::Unavailable,
                };
                self.send_active(Message::StreamRefused { session_id, reason });
            }
        }
        self.dirty = true;
    }

    pub fn stop_sharing(&mut self) {
        let Some(mut session) = self.sharing.take() else {
            return;
        };
        self.release_viewer_input();
        self.control = Control::Local;
        let capture = session.capture.take();
        let platform = self.platform.clone();
        tokio::task::spawn_blocking(move || {
            if let Some(capture) = capture {
                capture.stop();
            }
            platform.restore_display_mode();
        });
        self.dirty = true;
    }

    pub fn on_stream_stop(&mut self, session_id: u64) {
        if self.sharing.as_ref().is_some_and(|session| session.id == session_id) {
            self.stop_sharing();
        }
        if self.viewing.as_ref().is_some_and(|session| session.id == session_id) {
            self.stop_viewer(false);
        }
    }

    pub fn on_stream_keyframe(&mut self, session_id: u64) {
        if let Some(capture) = self.sharing.as_ref().filter(|session| session.id == session_id).and_then(|session| session.capture.as_ref()) {
            capture.request_keyframe();
        }
    }

    pub fn on_viewer_pointer(&mut self, x: u16, y: u16) {
        let Some(display) = self.desktop.as_ref().map(|desktop| desktop.primary()) else {
            return;
        };
        let Some(session) = self.sharing.as_mut().filter(|session| session.capture.is_some()) else {
            return;
        };
        let px = display.x + (ratio_to_f64(x) * f64::from(display.width - 1)).round() as i32;
        let py = display.y + (ratio_to_f64(y) * f64::from(display.height - 1)).round() as i32;
        let (lx, ly) = session.last_pointer.unwrap_or((px, py));
        session.last_pointer = Some((px, py));
        self.inject_event(InjectEvent::MoveTo { x: px, y: py, dx: px - lx, dy: py - ly });
    }

    pub fn on_stream_clock_pong(&mut self, sent_us: u64, peer_us: u64) {
        let received = now_us();
        if let Some(session) = self.viewing.as_mut() {
            session.clock.add_sample(sent_us, peer_us, received);
        }
    }

    pub fn stream_tick(&mut self, now: Instant) {
        let Some(session) = self.viewing.as_mut() else {
            return;
        };
        let elapsed = now.duration_since(session.fps_mark.0).as_secs_f64();
        if elapsed >= 1.0 {
            session.fps = ((session.presented - session.fps_mark.1) as f64 / elapsed).round() as u32;
            session.fps_mark = (now, session.presented);
        }
        self.send_active(Message::StreamClockPing { sent_us: now_us() });
    }

    pub fn stop_streams(&mut self, notify_peer: bool) {
        self.stop_viewer(notify_peer);
        self.stop_sharing();
    }

    pub fn stream_status(&self) -> StreamStatus {
        if let Some(session) = &self.viewing {
            let state = if session.receiver.is_some() { StreamState::Viewing } else { StreamState::Starting };
            return StreamStatus {
                state,
                width: session.size.0,
                height: session.size.1,
                fps: session.fps,
                latency_p50_us: session.latency.percentile(50),
                latency_p99_us: session.latency.percentile(99),
                dropped_frames: session.dropped.load(Ordering::Relaxed),
            };
        }
        if let Some(session) = &self.sharing {
            let state = if session.capture.is_some() { StreamState::Sharing } else { StreamState::Starting };
            return StreamStatus { state, width: session.size.0, height: session.size.1, fps: 0, latency_p50_us: None, latency_p99_us: None, dropped_frames: 0 };
        }
        StreamStatus { state: StreamState::Idle, width: 0, height: 0, fps: 0, latency_p50_us: None, latency_p99_us: None, dropped_frames: 0 }
    }
}

fn unspecified_for(peer: std::net::IpAddr) -> std::net::IpAddr {
    match peer {
        std::net::IpAddr::V4(_) => std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
        std::net::IpAddr::V6(_) => std::net::IpAddr::V6(std::net::Ipv6Addr::UNSPECIFIED),
    }
}
