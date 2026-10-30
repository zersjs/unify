use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::JoinHandle;
use tokio::time::{Instant, MissedTickBehavior};
use unify_core::backoff::Backoff;
use unify_core::clipboard::{ClipboardAssembler, ClipboardSync};
use unify_core::config::{ConfigFile, ConfigStore, LoadOutcome, MacKeyboardLayout, SettingsPatch};
use unify_core::geometry::Desktop;
use unify_core::keys::PressedState;
use unify_core::switcher::{EdgeSwitchConfig, EdgeSwitcher, RemoteCursor};
use unify_core::{PING_INTERVAL_MS, SLEEP_GAP_MS};
use unify_net::{DiscoveredDevice, Discovery, DiscoveryEvent, PeerInfo, SessionStream};
use unify_platform::{CaptureEvent, CaptureShared, ClipboardText, Injector, PermissionState, Platform, StopHandle, ViewerEvent};
use unify_proto::{Message, OsKind};
use unify_security::{DeviceIdentity, IdentityOutcome, IdentityStore};

use crate::pairing_flow::PairingActivity;
use crate::session::{Conn, ConnId, Direction, Purpose};
use crate::status::*;
use crate::stream_flow::{SharingResult, SharingSession, ViewerSession};
use crate::{Command, RuntimeError, RuntimeHandle, RuntimeOptions, APP_VERSION};

pub(crate) const DESKTOP_REFRESH_MS: u64 = 2_000;
pub(crate) const PERMISSION_REFRESH_MS: u64 = 3_000;

pub(crate) fn now_us() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_micros() as u64).unwrap_or(0)
}

pub(crate) fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

pub(crate) enum Event {
    Capture(CaptureEvent),
    ClipboardChanged,
    ClipboardRead { result: Result<Option<ClipboardText>, String>, force: bool },
    ClipboardWritten { result: Result<(), String> },
    ClipboardSent { ok: bool },
    Discovery(DiscoveryEvent),
    TcpAccepted(TcpStream),
    TlsReady { direction: Direction, purpose: Purpose, stream: Box<SessionStream>, info: PeerInfo, dialed: Option<String> },
    HandshakeFailed { purpose: Purpose, outbound: bool, error: String },
    ConnMessage { conn: ConnId, message: Message },
    ConnClosed { conn: ConnId, reason: String },
    ListenerFailed(String),
    Viewer(ViewerEvent),
    StreamKeyframeNeeded,
    SharingStarted { session_id: u64, result: SharingResult },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Control {
    Local,
    Controlling,
    Controlled,
}

pub(crate) struct Actor {
    pub platform: Arc<dyn Platform>,
    pub store: ConfigStore,
    pub config: ConfigFile,
    pub identities: IdentityStore,
    pub identity: DeviceIdentity,
    pub server_config: Arc<rustls::ServerConfig>,
    pub os: OsKind,
    pub bind_ip: std::net::IpAddr,
    pub port_override: Option<u16>,
    pub listen_port: u16,
    pub discovery_allowed: bool,
    pub events_tx: mpsc::UnboundedSender<Event>,
    pub status_tx: watch::Sender<StatusSnapshot>,
    pub dirty: bool,

    pub listener: Option<JoinHandle<()>>,
    pub discovery: Option<Discovery>,
    pub discovered: HashMap<String, DiscoveredDevice>,

    pub conns: HashMap<ConnId, Conn>,
    pub next_conn_id: ConnId,
    pub active: Option<ConnId>,
    pub pending_handshakes: usize,
    pub dial_in_flight: bool,
    pub next_dial_at: Option<Instant>,
    pub backoff: Backoff,
    pub ever_connected: bool,
    pub connection_since_ms: u64,
    pub rtt_ms: Option<u32>,

    pub pairing: Option<PairingActivity>,

    pub capture_shared: Arc<CaptureShared>,
    pub capture: Option<Box<dyn StopHandle>>,
    pub capture_state: CaptureState,
    pub injector: Option<Arc<dyn Injector>>,
    pub desktop: Option<Desktop>,
    pub last_desktop_refresh: Instant,
    pub switcher: EdgeSwitcher,
    pub control: Control,
    pub exit_point: Option<(i32, i32)>,
    pub refused_until: Option<Instant>,
    pub viewing: Option<ViewerSession>,
    pub sharing: Option<SharingSession>,
    pub remote_cursor: Option<RemoteCursor>,
    pub injected: PressedState,
    pub started: Instant,

    pub clipboard_watch: Option<Box<dyn StopHandle>>,
    pub clipboard_sync: ClipboardSync,
    pub assembler: ClipboardAssembler,
    pub pending_clipboard: Option<String>,
    pub next_transfer_id: u64,
    pub clipboard_activity: ClipboardActivity,
    pub last_transfer_ms: Option<u64>,
    pub last_direction: Option<TransferDirection>,
    pub last_skip: Option<&'static str>,
    pub clipboard_read_in_flight: bool,
    pub clipboard_reread: Option<bool>,

    pub permissions: unify_platform::PermissionReport,
    pub last_permission_check: Instant,
    pub last_error: Option<ErrorStatus>,
    pub stats: Stats,
    pub last_tick: Instant,
}

impl Actor {
    pub fn spawn(options: RuntimeOptions) -> Result<RuntimeHandle, RuntimeError> {
        let store = ConfigStore::new(&options.config_dir);
        let (config, outcome) =
            store.load_or_create(&options.platform.default_device_name()).map_err(|_| RuntimeError::new("config_load_failed"))?;
        if let LoadOutcome::RecoveredFromCorruption { backup } = &outcome {
            tracing::warn!(backup = %backup.display(), "configuration was corrupt; defaults restored");
        }
        let identities = IdentityStore::new(&options.config_dir, options.secrets.clone(), &options.profile);
        let (identity, identity_outcome) = identities.load_or_create().map_err(|err| {
            tracing::error!(error = %err, "identity store unavailable");
            RuntimeError::new("identity_store_failed")
        })?;
        let mut config = config;
        if identity_outcome == IdentityOutcome::Regenerated && config.peer.is_some() {
            tracing::warn!("device identity regenerated; previous pairing is no longer valid");
            config.peer = None;
            store.save(&config).map_err(|_| RuntimeError::new("config_save_failed"))?;
        }
        let server_config = unify_security::tls::server_config(&identity).map_err(|_| RuntimeError::new("tls_config_failed"))?;
        let platform = options.platform.clone();
        let iso = resolve_iso(config.settings.mac_keyboard_layout, platform.as_ref());
        let capture_shared = CaptureShared::new(config.settings.switch_hotkey, iso);
        let (events_tx, events_rx) = mpsc::unbounded_channel();
        let (commands_tx, commands_rx) = mpsc::unbounded_channel();
        let switcher = EdgeSwitcher::new(switch_config(&config.settings));
        let now = Instant::now();
        let mut actor = Actor {
            os: platform.os(),
            permissions: platform.permissions(),
            platform,
            store,
            config,
            identities,
            identity,
            server_config,
            bind_ip: options.bind_ip,
            port_override: options.listen_port_override,
            listen_port: 0,
            discovery_allowed: options.discovery_enabled,
            events_tx,
            status_tx: watch::channel(placeholder_snapshot()).0,
            dirty: true,
            listener: None,
            discovery: None,
            discovered: HashMap::new(),
            conns: HashMap::new(),
            next_conn_id: 1,
            active: None,
            pending_handshakes: 0,
            dial_in_flight: false,
            next_dial_at: None,
            backoff: Backoff::new(500, 10_000),
            ever_connected: false,
            connection_since_ms: now_ms(),
            rtt_ms: None,
            pairing: None,
            capture_shared,
            capture: None,
            capture_state: CaptureState::Disabled,
            injector: None,
            desktop: None,
            last_desktop_refresh: now,
            switcher,
            control: Control::Local,
            exit_point: None,
            refused_until: None,
            viewing: None,
            sharing: None,
            remote_cursor: None,
            injected: PressedState::new(),
            started: now,
            clipboard_watch: None,
            clipboard_sync: ClipboardSync::new(),
            assembler: ClipboardAssembler::new(),
            pending_clipboard: None,
            next_transfer_id: 1,
            clipboard_activity: ClipboardActivity::Idle,
            last_transfer_ms: None,
            last_direction: None,
            last_skip: None,
            clipboard_read_in_flight: false,
            clipboard_reread: None,
            last_permission_check: now,
            last_error: None,
            stats: Stats::default(),
            last_tick: now,
        };
        let initial = actor.snapshot();
        let (status_tx, status_rx) = watch::channel(initial);
        actor.status_tx = status_tx;
        tokio::spawn(actor.run(commands_rx, events_rx));
        Ok(RuntimeHandle { commands: commands_tx, status: status_rx })
    }

    async fn run(mut self, mut commands: mpsc::UnboundedReceiver<Command>, mut events: mpsc::UnboundedReceiver<Event>) {
        self.refresh_desktop();
        self.start_listener().await;
        self.restart_discovery();
        self.sync_capture();
        self.sync_clipboard_watch();
        if self.config.peer.is_some() {
            self.next_dial_at = Some(Instant::now());
        }
        let mut tick = tokio::time::interval(Duration::from_millis(PING_INTERVAL_MS));
        tick.set_missed_tick_behavior(MissedTickBehavior::Delay);
        let mut shutdown_reply: Option<oneshot::Sender<()>> = None;
        loop {
            let dial_at = self.next_dial_at.filter(|_| self.should_dial());
            tokio::select! {
                command = commands.recv() => match command {
                    Some(Command::Shutdown(reply)) => {
                        shutdown_reply = Some(reply);
                        break;
                    }
                    Some(command) => self.on_command(command).await,
                    None => break,
                },
                Some(event) = events.recv() => self.on_event(event),
                _ = tick.tick() => self.on_tick(),
                _ = sleep_until_opt(dial_at), if dial_at.is_some() => self.dial_session(),
            }
            self.publish();
        }
        self.shutdown().await;
        if let Some(reply) = shutdown_reply {
            let _ = reply.send(());
        }
    }

    pub fn should_dial(&self) -> bool {
        self.config.peer.is_some() && self.active.is_none() && !self.dial_in_flight
    }

    async fn on_command(&mut self, command: Command) {
        self.dirty = true;
        match command {
            Command::UpdateSettings(patch, reply) => {
                let result = self.update_settings(patch).await;
                let _ = reply.send(result);
            }
            Command::StartPairingHost(reply) => {
                let _ = reply.send(self.start_pairing_host());
            }
            Command::CancelPairing => self.cancel_pairing(),
            Command::JoinPairing { address, code, reply } => {
                let _ = reply.send(self.join_pairing(&address, &code));
            }
            Command::ApprovePairing { accept } => self.approve_pairing(accept),
            Command::RemovePeer(reply) => {
                let _ = reply.send(self.remove_peer());
            }
            Command::SetManualAddress(address, reply) => {
                let _ = reply.send(self.set_manual_address(address));
            }
            Command::ReconnectNow => {
                self.backoff.reset();
                if self.active.is_none() {
                    self.next_dial_at = Some(Instant::now());
                }
            }
            Command::ResyncClipboard => self.request_clipboard_read(true),
            Command::ReleaseControl => self.release_control_everywhere(),
            Command::RequestPermissions => {
                self.platform.request_permissions();
                self.refresh_permissions();
            }
            Command::ResetPermissions => {
                if let Err(err) = self.platform.reset_permissions() {
                    tracing::warn!(error = %err, "permission reset failed");
                    self.set_error("permission_reset_failed");
                }
                self.refresh_permissions();
            }
            Command::OpenPermissionSettings(kind) => {
                if self.platform.open_permission_settings(kind).is_err() {
                    self.set_error("permission_settings_unavailable");
                }
            }
            Command::ResetIdentity(reply) => {
                let _ = reply.send(self.reset_identity());
            }
            Command::StartViewer(reply) => {
                let _ = reply.send(self.start_viewer().await);
            }
            Command::StopViewer => self.stop_viewer(true),
            Command::ClearError => self.last_error = None,
            Command::Shutdown(_) => {}
        }
    }

    fn on_event(&mut self, event: Event) {
        match event {
            Event::Capture(capture) => self.on_capture(capture),
            Event::ClipboardChanged => self.request_clipboard_read(false),
            Event::ClipboardRead { result, force } => self.on_clipboard_read(result, force),
            Event::ClipboardWritten { result } => self.on_clipboard_written(result),
            Event::ClipboardSent { ok } => self.on_clipboard_sent(ok),
            Event::Discovery(event) => self.on_discovery(event),
            Event::TcpAccepted(stream) => self.on_tcp_accepted(stream),
            Event::TlsReady { direction, purpose, stream, info, dialed } => self.on_tls_ready(direction, purpose, *stream, info, dialed),
            Event::HandshakeFailed { purpose, outbound, error } => self.on_handshake_failed(purpose, outbound, &error),
            Event::ConnMessage { conn, message } => self.on_conn_message(conn, message),
            Event::ConnClosed { conn, reason } => self.on_conn_closed(conn, &reason),
            Event::Viewer(event) => self.on_viewer_event(event),
            Event::StreamKeyframeNeeded => self.request_stream_keyframe(),
            Event::SharingStarted { session_id, result } => self.on_sharing_started(session_id, result),
            Event::ListenerFailed(error) => {
                tracing::error!(error = %error, "listener stopped");
                self.listener = None;
                self.set_error("listen_failed");
            }
        }
    }

    fn on_tick(&mut self) {
        let now = Instant::now();
        let gap = now.duration_since(self.last_tick).as_millis() as u64;
        self.last_tick = now;
        if gap > SLEEP_GAP_MS {
            tracing::info!(gap_ms = gap, "clock gap detected; assuming system sleep and resetting connections");
            self.on_wake();
        }
        if now.duration_since(self.last_desktop_refresh).as_millis() as u64 >= DESKTOP_REFRESH_MS {
            self.refresh_desktop();
        }
        if now.duration_since(self.last_permission_check).as_millis() as u64 >= PERMISSION_REFRESH_MS {
            self.refresh_permissions();
        }
        self.heartbeat(now);
        self.stream_tick(now);
        self.expire_pairing(now);
        self.dirty = true;
    }

    fn on_wake(&mut self) {
        if let Some(active) = self.active {
            self.close_conn(active, "system_wake", None);
        }
        self.backoff.reset();
        self.next_dial_at = Some(Instant::now());
        self.refresh_desktop();
    }

    pub fn refresh_desktop(&mut self) {
        self.last_desktop_refresh = Instant::now();
        match self.platform.desktop() {
            Ok(desktop) => self.desktop = Some(desktop),
            Err(err) => tracing::warn!(error = %err, "display layout unavailable"),
        }
    }

    pub fn refresh_permissions(&mut self) {
        self.last_permission_check = Instant::now();
        let report = self.platform.permissions();
        if report != self.permissions {
            self.permissions = report;
            self.dirty = true;
        }
        if self.capture_state == CaptureState::PermissionRequired {
            self.sync_capture();
        }
    }

    pub fn set_error(&mut self, code: &'static str) {
        tracing::info!(code, "user-facing error");
        self.last_error = Some(ErrorStatus { code, at_ms: now_ms() });
        self.dirty = true;
    }

    pub fn save_config(&mut self) -> bool {
        match self.store.save(&self.config) {
            Ok(()) => true,
            Err(err) => {
                tracing::error!(error = %err, "failed to persist configuration");
                self.set_error("config_save_failed");
                false
            }
        }
    }

    async fn update_settings(&mut self, patch: SettingsPatch) -> Result<(), RuntimeError> {
        let previous = self.config.settings.clone();
        let next = patch.apply(&previous, now_ms());
        if next == previous {
            return Ok(());
        }
        self.config.settings = next.clone();
        if !self.save_config() {
            self.config.settings = previous;
            return Err(RuntimeError::new("config_save_failed"));
        }
        self.capture_shared.set_hotkey_enabled(next.switch_hotkey);
        self.capture_shared.set_iso_keyboard(resolve_iso(next.mac_keyboard_layout, self.platform.as_ref()));
        self.switcher.reconfigure(switch_config(&next));
        if previous.input_sharing != next.input_sharing {
            if !next.input_sharing {
                self.release_control_everywhere();
            }
            self.sync_capture();
        }
        if previous.clipboard_sync != next.clipboard_sync {
            self.sync_clipboard_watch();
        }
        if previous.peer_side != next.peer_side {
            self.send_layout_claim();
        }
        if previous.listen_port != next.listen_port && self.port_override.is_none() {
            self.start_listener().await;
        }
        if previous.listen_port != next.listen_port || previous.device_name != next.device_name || previous.discovery != next.discovery {
            self.restart_discovery();
        }
        Ok(())
    }

    fn reset_identity(&mut self) -> Result<(), RuntimeError> {
        let identity = self.identities.reset(&self.identity).map_err(|_| RuntimeError::new("identity_store_failed"))?;
        let server_config = unify_security::tls::server_config(&identity).map_err(|_| RuntimeError::new("tls_config_failed"))?;
        self.release_control_everywhere();
        let ids: Vec<ConnId> = self.conns.keys().copied().collect();
        for id in ids {
            self.close_conn(id, "identity_reset", Some(unify_proto::GoodbyeReason::Unpaired));
        }
        self.identity = identity;
        self.server_config = server_config;
        self.config.peer = None;
        self.pairing = None;
        self.save_config();
        self.restart_discovery();
        Ok(())
    }

    pub fn restart_discovery(&mut self) {
        if let Some(discovery) = self.discovery.take() {
            discovery.stop();
        }
        self.discovered.clear();
        if !self.discovery_allowed || !self.config.settings.discovery || self.listen_port == 0 {
            return;
        }
        let (tx, mut rx) = mpsc::unbounded_channel();
        let advertisement = unify_net::Advertisement {
            device_id: self.identity.device_id().to_owned(),
            device_name: self.config.settings.device_name.clone(),
            os: self.os,
            port: self.listen_port,
            fingerprint_prefix: self.identity.fingerprint().to_hex()[..16].to_owned(),
        };
        match Discovery::start(advertisement, tx) {
            Ok(discovery) => {
                self.discovery = Some(discovery);
                let events = self.events_tx.clone();
                tokio::spawn(async move {
                    while let Some(event) = rx.recv().await {
                        if events.send(Event::Discovery(event)).is_err() {
                            break;
                        }
                    }
                });
            }
            Err(err) => {
                tracing::warn!(error = %err, "mDNS discovery unavailable");
                self.set_error("discovery_failed");
            }
        }
    }

    fn on_discovery(&mut self, event: DiscoveryEvent) {
        self.dirty = true;
        match event {
            DiscoveryEvent::Found(device) => {
                let is_peer = self.config.peer.as_ref().is_some_and(|peer| peer.device_id == device.device_id);
                self.discovered.insert(device.device_id.clone(), device);
                if is_peer && self.active.is_none() && !self.dial_in_flight {
                    self.backoff.reset();
                    self.next_dial_at = Some(Instant::now());
                }
            }
            DiscoveryEvent::Lost { device_id } => {
                self.discovered.remove(&device_id);
            }
        }
    }

    async fn shutdown(&mut self) {
        self.stop_streams(true);
        self.release_control_everywhere();
        let ids: Vec<ConnId> = self.conns.keys().copied().collect();
        for id in ids {
            self.close_conn(id, "shutdown", Some(unify_proto::GoodbyeReason::Shutdown));
        }
        if let Some(capture) = self.capture.take() {
            tokio::task::spawn_blocking(move || capture.stop()).await.ok();
        }
        self.capture_shared.leave_remote();
        if let Some(watch) = self.clipboard_watch.take() {
            tokio::task::spawn_blocking(move || watch.stop()).await.ok();
        }
        if let Some(discovery) = self.discovery.take() {
            discovery.stop();
        }
        if let Some(listener) = self.listener.take() {
            listener.abort();
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
    }

    pub fn publish(&mut self) {
        if !self.dirty {
            return;
        }
        self.dirty = false;
        let snapshot = self.snapshot();
        self.status_tx.send_if_modified(|current| {
            if *current == snapshot {
                false
            } else {
                *current = snapshot;
                true
            }
        });
    }

    fn connection_state(&self) -> ConnectionState {
        if self.config.peer.is_none() {
            return ConnectionState::NotPaired;
        }
        if self.active.is_some_and(|id| self.conns.get(&id).is_some_and(Conn::is_established)) {
            return ConnectionState::Connected;
        }
        if self.dial_in_flight {
            return if self.ever_connected || self.backoff.attempt() > 0 { ConnectionState::Reconnecting } else { ConnectionState::Connecting };
        }
        if self.ever_connected {
            ConnectionState::Reconnecting
        } else {
            ConnectionState::Waiting
        }
    }

    pub fn snapshot(&self) -> StatusSnapshot {
        let peer = self.config.peer.as_ref().map(|peer| PeerStatus {
            id: peer.device_id.clone(),
            name: peer.device_name.clone(),
            os: peer.os.as_str(),
            fingerprint: unify_security::Fingerprint::from_hex(&peer.fingerprint).map(|fp| fp.short()).unwrap_or_default(),
            paired_at: peer.paired_at,
            manual_address: peer.manual_address.clone(),
            last_address: peer.last_address.clone(),
        });
        let next_retry_ms = self.next_dial_at.filter(|_| self.active.is_none()).map(|at| {
            let remaining = at.saturating_duration_since(Instant::now()).as_millis() as u64;
            now_ms() + remaining
        });
        let remote_address = self.active.and_then(|id| self.conns.get(&id)).map(|conn| conn.info.remote_addr.to_string());
        let mut discovered: Vec<DiscoveredStatus> = self
            .discovered
            .values()
            .map(|device| DiscoveredStatus {
                id: device.device_id.clone(),
                name: device.device_name.clone(),
                os: device.os.as_str(),
                address: device.addresses.first().map(SocketAddr::to_string).unwrap_or_default(),
                is_paired_peer: self.config.peer.as_ref().is_some_and(|peer| peer.device_id == device.device_id),
            })
            .collect();
        discovered.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
        StatusSnapshot {
            device: DeviceStatus {
                id: self.identity.device_id().to_owned(),
                name: self.config.settings.device_name.clone(),
                os: self.os.as_str(),
                fingerprint: self.identity.fingerprint().short(),
                port: self.listen_port,
                secret_backend: self.identities.secret_backend(),
                config_path: self.store.path().display().to_string(),
            },
            peer,
            connection: ConnectionStatus {
                state: self.connection_state(),
                attempt: self.backoff.attempt(),
                rtt_ms: if self.active.is_some() { self.rtt_ms } else { None },
                since_ms: self.connection_since_ms,
                remote_address,
                next_retry_ms,
            },
            control: match self.control {
                Control::Local => ControlState::Local,
                Control::Controlling => ControlState::Controlling,
                Control::Controlled => ControlState::Controlled,
            },
            capture: self.capture_state,
            stream: self.stream_status(),
            clipboard: ClipboardStatus {
                activity: self.clipboard_activity,
                last_transfer_ms: self.last_transfer_ms,
                last_direction: self.last_direction,
                last_skip: self.last_skip,
            },
            permissions: PermissionsStatus {
                accessibility: permission_str(self.permissions.accessibility),
                input_monitoring: permission_str(self.permissions.input_monitoring),
                local_network: permission_str(self.permissions.local_network),
            },
            pairing: self.pairing.as_ref().map(PairingActivity::status),
            discovered,
            last_error: self.last_error.clone(),
            stats: self.stats.clone(),
            settings: self.config.settings.clone(),
            iso_keyboard: self.capture_shared.iso_keyboard(),
            displays: self.desktop.as_ref().map(|d| d.displays().to_vec()).unwrap_or_default(),
            app_version: APP_VERSION,
        }
    }
}

pub(crate) fn permission_str(state: PermissionState) -> &'static str {
    match state {
        PermissionState::Granted => "granted",
        PermissionState::Denied => "denied",
        PermissionState::NotRequired => "not_required",
        PermissionState::Unknown => "unknown",
    }
}

pub(crate) fn switch_config(settings: &unify_core::config::Settings) -> EdgeSwitchConfig {
    EdgeSwitchConfig {
        side: settings.peer_side,
        enabled: settings.edge_switch,
        dwell_ms: u64::from(settings.edge_dwell_ms),
        corner_guard_px: settings.corner_guard_px as i32,
    }
}

fn resolve_iso(layout: MacKeyboardLayout, platform: &dyn Platform) -> bool {
    match layout {
        MacKeyboardLayout::Iso => true,
        MacKeyboardLayout::Ansi => false,
        MacKeyboardLayout::Auto => platform.detect_iso_keyboard(),
    }
}

async fn sleep_until_opt(at: Option<Instant>) {
    match at {
        Some(at) => tokio::time::sleep_until(at).await,
        None => std::future::pending::<()>().await,
    }
}

fn placeholder_snapshot() -> StatusSnapshot {
    StatusSnapshot {
        device: DeviceStatus {
            id: String::new(),
            name: String::new(),
            os: "other",
            fingerprint: String::new(),
            port: 0,
            secret_backend: "",
            config_path: String::new(),
        },
        peer: None,
        connection: ConnectionStatus { state: ConnectionState::NotPaired, attempt: 0, rtt_ms: None, since_ms: 0, remote_address: None, next_retry_ms: None },
        control: ControlState::Local,
        capture: CaptureState::Disabled,
        stream: StreamStatus { state: StreamState::Idle, width: 0, height: 0, fps: 0, latency_p50_us: None, latency_p99_us: None, dropped_frames: 0 },
        clipboard: ClipboardStatus { activity: ClipboardActivity::Idle, last_transfer_ms: None, last_direction: None, last_skip: None },
        permissions: PermissionsStatus { accessibility: "unknown", input_monitoring: "unknown", local_network: "unknown" },
        pairing: None,
        discovered: Vec::new(),
        last_error: None,
        stats: Stats::default(),
        settings: unify_core::config::Settings::default(),
        iso_keyboard: false,
        displays: Vec::new(),
        app_version: APP_VERSION,
    }
}
