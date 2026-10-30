use std::net::SocketAddr;
use std::time::Duration;

use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;
use tokio::time::Instant;
use unify_core::config::{normalize_address, DEFAULT_PORT};
use unify_core::{CONNECT_TIMEOUT_MS, HANDSHAKE_TIMEOUT_MS, PEER_TIMEOUT_MS};
use unify_net::{accept_tls, connect_tls, read_message, spawn_writer, Outbound, PeerInfo, ReadError, SendError, SessionStream};
use unify_proto::{GoodbyeReason, Hello, LayoutClaim, Message, PROTOCOL_VERSION};
use unify_security::pairing::{PairingExchange, PairingKey, Transcript};
use unify_security::tls::{client_config, ServerExpectation};
use unify_security::Fingerprint;

use crate::actor::{now_ms, Actor, Event};
use crate::APP_VERSION;

pub(crate) type ConnId = u64;

const MAX_PENDING_HANDSHAKES: usize = 8;
const HELLO_TIMEOUT_MS: u64 = 10_000;
const PAIRING_STEP_TIMEOUT_MS: u64 = 75_000;
const WRITER_DRAIN_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Direction {
    Inbound,
    Outbound,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Purpose {
    Session,
    Pairing { code: String },
    Accept,
}

pub(crate) enum ConnState {
    AwaitHello,
    Established { peer_hello: Hello },
    PairInitiatorAwaitReply { exchange: Option<PairingExchange> },
    PairInitiatorAwaitConfirm { key: PairingKey, transcript: Transcript, identity: unify_proto::PairIdentity },
    PairInitiatorAwaitResult { identity: unify_proto::PairIdentity },
    PairResponderAwaitConfirm { key: PairingKey, transcript: Transcript, identity: unify_proto::PairIdentity },
    PairResponderAwaitApproval,
}

pub(crate) struct Conn {
    pub direction: Direction,
    pub info: PeerInfo,
    pub outbound: Outbound,
    pub reader: JoinHandle<()>,
    pub writer: JoinHandle<Result<(), std::io::Error>>,
    pub state: ConnState,
    pub opened_at: Instant,
    pub last_rx: Instant,
    pub dialed: Option<String>,
}

impl Conn {
    pub fn is_established(&self) -> bool {
        matches!(self.state, ConnState::Established { .. })
    }

    pub fn is_pairing(&self) -> bool {
        !matches!(self.state, ConnState::AwaitHello | ConnState::Established { .. })
    }

    pub fn client_id<'a>(&'a self, own_id: &'a str) -> &'a str {
        match (&self.direction, &self.state) {
            (Direction::Outbound, _) => own_id,
            (Direction::Inbound, ConnState::Established { peer_hello }) => &peer_hello.device_id,
            (Direction::Inbound, _) => "",
        }
    }
}

impl Actor {
    pub async fn start_listener(&mut self) {
        if let Some(listener) = self.listener.take() {
            listener.abort();
        }
        let port = self.port_override.unwrap_or(self.config.settings.listen_port);
        let listener = match TcpListener::bind(SocketAddr::new(self.bind_ip, port)).await {
            Ok(listener) => listener,
            Err(err) => {
                tracing::error!(port, error = %err, "cannot listen for connections");
                self.listen_port = 0;
                self.set_error("listen_failed");
                return;
            }
        };
        self.listen_port = listener.local_addr().map(|addr| addr.port()).unwrap_or(port);
        tracing::info!(port = self.listen_port, "listening for peers");
        let events = self.events_tx.clone();
        self.listener = Some(tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, _)) => {
                        if events.send(Event::TcpAccepted(stream)).is_err() {
                            break;
                        }
                    }
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::ConnectionAborted || err.kind() == std::io::ErrorKind::ConnectionReset {
                            continue;
                        }
                        let _ = events.send(Event::ListenerFailed(err.to_string()));
                        break;
                    }
                }
            }
        }));
        self.dirty = true;
    }

    pub fn on_tcp_accepted(&mut self, stream: TcpStream) {
        if self.pending_handshakes >= MAX_PENDING_HANDSHAKES {
            self.stats.rejected_connections += 1;
            return;
        }
        self.pending_handshakes += 1;
        let config = self.server_config.clone();
        let events = self.events_tx.clone();
        tokio::spawn(async move {
            let result = accept_tls(stream, config, Duration::from_millis(HANDSHAKE_TIMEOUT_MS)).await;
            let event = match result {
                Ok((stream, info)) => {
                    Event::TlsReady { direction: Direction::Inbound, purpose: Purpose::Accept, stream: Box::new(stream), info, dialed: None }
                }
                Err(err) => Event::HandshakeFailed { purpose: Purpose::Accept, outbound: false, error: err.to_string() },
            };
            let _ = events.send(event);
        });
    }

    fn dial_candidates(&self) -> Vec<String> {
        let Some(peer) = self.config.peer.as_ref() else {
            return Vec::new();
        };
        let mut candidates: Vec<String> = Vec::new();
        if let Some(manual) = &peer.manual_address {
            candidates.push(manual.clone());
        }
        if let Some(device) = self.discovered.get(&peer.device_id) {
            candidates.extend(device.addresses.iter().map(SocketAddr::to_string));
        }
        if let Some(last) = &peer.last_address {
            candidates.push(last.clone());
        }
        let mut seen = std::collections::HashSet::new();
        candidates.retain(|candidate| seen.insert(candidate.clone()));
        candidates
    }

    pub fn dial_session(&mut self) {
        self.next_dial_at = None;
        if !self.should_dial() {
            return;
        }
        let Some(peer) = self.config.peer.as_ref() else {
            return;
        };
        let Some(fingerprint) = Fingerprint::from_hex(&peer.fingerprint) else {
            self.set_error("peer_record_invalid");
            return;
        };
        let candidates = self.dial_candidates();
        if candidates.is_empty() {
            self.schedule_retry();
            return;
        }
        let config = match client_config(&self.identity, ServerExpectation::Pinned(fingerprint)) {
            Ok(config) => config,
            Err(_) => {
                self.set_error("tls_config_failed");
                return;
            }
        };
        self.dial_in_flight = true;
        self.dirty = true;
        self.spawn_dial(candidates, config, Purpose::Session);
    }

    pub fn spawn_dial(&self, candidates: Vec<String>, config: std::sync::Arc<rustls::ClientConfig>, purpose: Purpose) {
        let events = self.events_tx.clone();
        tokio::spawn(async move {
            let mut last_error = String::from("no_candidates");
            for candidate in candidates {
                match connect_tls(&candidate, config.clone(), Duration::from_millis(CONNECT_TIMEOUT_MS), Duration::from_millis(HANDSHAKE_TIMEOUT_MS)).await {
                    Ok((stream, info)) => {
                        let _ = events.send(Event::TlsReady {
                            direction: Direction::Outbound,
                            purpose,
                            stream: Box::new(stream),
                            info,
                            dialed: Some(candidate),
                        });
                        return;
                    }
                    Err(err) => last_error = err.to_string(),
                }
            }
            let _ = events.send(Event::HandshakeFailed { purpose, outbound: true, error: last_error });
        });
    }

    pub fn schedule_retry(&mut self) {
        let delay = self.backoff.next_delay(rand::random::<f64>());
        self.next_dial_at = Some(Instant::now() + delay);
        self.dirty = true;
    }

    pub fn on_handshake_failed(&mut self, purpose: Purpose, outbound: bool, error: &str) {
        self.dirty = true;
        if !outbound {
            self.pending_handshakes = self.pending_handshakes.saturating_sub(1);
            self.stats.rejected_connections += 1;
            tracing::info!(error = %error, "inbound handshake failed");
            return;
        }
        match purpose {
            Purpose::Session => {
                self.dial_in_flight = false;
                tracing::debug!(error = %error, attempt = self.backoff.attempt(), "dial failed");
                if self.backoff.attempt() >= 2 {
                    self.set_error("peer_unreachable");
                }
                self.schedule_retry();
            }
            Purpose::Pairing { .. } => self.pairing_failed("pairing_unreachable"),
            Purpose::Accept => {}
        }
    }

    pub fn on_tls_ready(&mut self, direction: Direction, purpose: Purpose, stream: SessionStream, info: PeerInfo, dialed: Option<String>) {
        self.dirty = true;
        match direction {
            Direction::Inbound => self.pending_handshakes = self.pending_handshakes.saturating_sub(1),
            Direction::Outbound => {
                if purpose == Purpose::Session {
                    self.dial_in_flight = false;
                }
            }
        }
        let id = self.next_conn_id;
        self.next_conn_id += 1;
        let (read_half, write_half) = tokio::io::split(stream);
        let (outbound, writer) = spawn_writer(write_half);
        let events = self.events_tx.clone();
        let reader = tokio::spawn(async move {
            let mut read_half = read_half;
            loop {
                match read_message(&mut read_half).await {
                    Ok(message) => {
                        if events.send(Event::ConnMessage { conn: id, message }).is_err() {
                            return;
                        }
                    }
                    Err(err) => {
                        let reason = match err {
                            ReadError::Closed => String::from("closed"),
                            ReadError::Io(err) => format!("io: {err}"),
                            ReadError::Protocol(err) => format!("protocol: {err}"),
                        };
                        let _ = events.send(Event::ConnClosed { conn: id, reason });
                        return;
                    }
                }
            }
        });
        let now = Instant::now();
        let mut conn = Conn { direction, info, outbound, reader, writer, state: ConnState::AwaitHello, opened_at: now, last_rx: now, dialed };
        match purpose {
            Purpose::Session => {
                let hello = self.own_hello();
                let _ = conn.outbound.send_high(Message::Hello(hello));
            }
            Purpose::Pairing { code } => {
                let (exchange, spake_message) = PairingExchange::start(&code);
                let _ = conn.outbound.send_high(Message::PairStart { identity: self.own_pair_identity(), spake_message });
                conn.state = ConnState::PairInitiatorAwaitReply { exchange: Some(exchange) };
                self.pairing_attach_conn(id);
            }
            Purpose::Accept => {}
        }
        self.conns.insert(id, conn);
    }

    pub fn own_hello(&self) -> Hello {
        Hello {
            protocol_version: PROTOCOL_VERSION,
            device_id: self.identity.device_id().to_owned(),
            device_name: self.config.settings.device_name.clone(),
            os: self.os,
            app_version: APP_VERSION.to_owned(),
            listen_port: self.listen_port,
        }
    }

    pub fn own_pair_identity(&self) -> unify_proto::PairIdentity {
        unify_proto::PairIdentity {
            protocol_version: PROTOCOL_VERSION,
            device_id: self.identity.device_id().to_owned(),
            device_name: self.config.settings.device_name.clone(),
            os: self.os,
        }
    }

    pub fn send_active(&mut self, message: Message) -> bool {
        let Some(id) = self.active else {
            return false;
        };
        let result = match self.conns.get(&id) {
            Some(conn) => conn.outbound.send_high(message),
            None => return false,
        };
        match result {
            Ok(()) => true,
            Err(SendError::Overloaded) => {
                tracing::warn!("peer is not draining its connection; closing session");
                self.close_conn(id, "overloaded", None);
                false
            }
            Err(SendError::Closed) => false,
        }
    }

    pub fn close_conn(&mut self, id: ConnId, reason: &str, goodbye: Option<GoodbyeReason>) {
        let Some(conn) = self.conns.remove(&id) else {
            return;
        };
        tracing::info!(conn = id, reason, "closing connection");
        if let Some(goodbye) = goodbye {
            let _ = conn.outbound.send_high(Message::Goodbye { reason: goodbye });
        }
        conn.reader.abort();
        drop(conn.outbound);
        let writer = conn.writer;
        tokio::spawn(async move {
            let abort = writer.abort_handle();
            if tokio::time::timeout(WRITER_DRAIN_TIMEOUT, writer).await.is_err() {
                abort.abort();
            }
        });
        self.pairing_conn_closed(id);
        if self.active == Some(id) {
            self.active = None;
            self.rtt_ms = None;
            self.connection_since_ms = now_ms();
            self.on_session_lost();
            self.assembler.reset();
            self.clipboard_activity = crate::status::ClipboardActivity::Idle;
            if self.config.peer.is_some() {
                self.next_dial_at = Some(Instant::now() + Duration::from_millis(300));
            }
        }
        self.dirty = true;
    }

    pub fn on_conn_closed(&mut self, id: ConnId, reason: &str) {
        if self.conns.contains_key(&id) {
            if self.active == Some(id) {
                self.set_error("connection_lost");
            }
            self.close_conn(id, reason, None);
        }
    }

    pub fn on_conn_message(&mut self, id: ConnId, message: Message) {
        let Some(conn) = self.conns.get_mut(&id) else {
            return;
        };
        conn.last_rx = Instant::now();
        if let Message::Goodbye { reason } = message {
            self.on_goodbye(id, reason);
            return;
        }
        if conn.is_established() {
            self.on_session_message(id, message);
            return;
        }
        if conn.is_pairing() || matches!(message, Message::PairStart { .. }) {
            self.on_pairing_message(id, message);
            return;
        }
        match message {
            Message::Hello(hello) => self.on_hello(id, hello),
            other => {
                tracing::warn!(conn = id, kind = other.kind(), "unexpected message before authentication");
                self.stats.rejected_connections += 1;
                self.close_conn(id, "unauthenticated_message", Some(GoodbyeReason::ProtocolViolation));
            }
        }
    }

    fn on_goodbye(&mut self, id: ConnId, reason: GoodbyeReason) {
        let is_pairing = self.conns.get(&id).is_some_and(Conn::is_pairing);
        match reason {
            GoodbyeReason::Unpaired if !is_pairing => self.set_error("peer_rejected"),
            GoodbyeReason::VersionMismatch => self.set_error("version_mismatch"),
            GoodbyeReason::PairingRejected => self.pairing_failed("pairing_code_mismatch"),
            _ => {}
        }
        self.close_conn(id, "goodbye", None);
    }

    fn on_hello(&mut self, id: ConnId, hello: Hello) {
        let Some(peer) = self.config.peer.clone() else {
            self.stats.rejected_connections += 1;
            self.close_conn(id, "not_paired", Some(GoodbyeReason::Unpaired));
            return;
        };
        let Some(conn) = self.conns.get(&id) else {
            return;
        };
        if hello.protocol_version != PROTOCOL_VERSION {
            self.set_error("version_mismatch");
            self.close_conn(id, "version_mismatch", Some(GoodbyeReason::VersionMismatch));
            return;
        }
        let pinned = Fingerprint::from_hex(&peer.fingerprint);
        if pinned != Some(conn.info.fingerprint) || hello.device_id != peer.device_id {
            tracing::warn!(
                remote = %conn.info.remote_addr.ip(),
                presented = %conn.info.fingerprint.short(),
                "rejected connection from unpaired device"
            );
            self.stats.rejected_connections += 1;
            self.close_conn(id, "unpaired_device", Some(GoodbyeReason::Unpaired));
            return;
        }
        let direction = conn.direction;
        let remote_ip = conn.info.remote_addr.ip();
        let dialed = conn.dialed.clone();
        if direction == Direction::Inbound {
            let reply = self.own_hello();
            if let Some(conn) = self.conns.get(&id) {
                let _ = conn.outbound.send_high(Message::Hello(reply));
            }
        }
        let peer_port = hello.listen_port;
        if let Some(conn) = self.conns.get_mut(&id) {
            conn.state = ConnState::Established { peer_hello: hello.clone() };
        }
        if let Some(existing) = self.active.filter(|existing| *existing != id) {
            let own_id = self.identity.device_id().to_owned();
            let existing_client = self.conns.get(&existing).map(|c| c.client_id(&own_id).to_owned()).unwrap_or_default();
            let new_client = self.conns.get(&id).map(|c| c.client_id(&own_id).to_owned()).unwrap_or_default();
            let keep_new = new_client <= existing_client;
            if keep_new {
                self.close_conn(existing, "replaced", Some(GoodbyeReason::Replaced));
            } else {
                self.close_conn(id, "duplicate", Some(GoodbyeReason::Replaced));
                return;
            }
        }
        self.active = Some(id);
        self.ever_connected = true;
        self.connection_since_ms = now_ms();
        self.backoff.reset();
        self.next_dial_at = None;
        self.stats.sessions_established += 1;
        if matches!(self.last_error.as_ref().map(|e| e.code), Some("peer_unreachable" | "connection_lost" | "peer_timeout")) {
            self.last_error = None;
        }
        let address = dialed.or_else(|| (peer_port != 0).then(|| SocketAddr::new(remote_ip, peer_port).to_string()));
        if let Some(stored) = self.config.peer.as_mut() {
            let mut changed = false;
            if stored.device_name != hello.device_name {
                stored.device_name = hello.device_name.clone();
                changed = true;
            }
            if stored.os != hello.os {
                stored.os = hello.os;
                changed = true;
            }
            if address.is_some() && stored.last_address != address {
                stored.last_address = address;
                changed = true;
            }
            if changed {
                self.save_config();
            }
        }
        tracing::info!(conn = id, "session established");
        self.send_layout_claim();
        self.on_session_ready();
        self.dirty = true;
    }

    pub fn send_layout_claim(&mut self) {
        let claim = LayoutClaim { receiver_side: self.config.settings.peer_side, revision: self.config.settings.layout_revision };
        self.send_active(Message::Layout(claim));
    }

    pub fn on_layout_claim(&mut self, claim: LayoutClaim) {
        let implied = claim.receiver_side.opposite();
        let settings = &self.config.settings;
        if implied == settings.peer_side {
            return;
        }
        let peer_id = self.config.peer.as_ref().map(|p| p.device_id.clone()).unwrap_or_default();
        let theirs = (claim.revision, peer_id.as_str());
        let ours = (settings.layout_revision, self.identity.device_id());
        if theirs > ours {
            self.config.settings.peer_side = implied;
            self.config.settings.layout_revision = claim.revision;
            self.switcher.reconfigure(crate::actor::switch_config(&self.config.settings));
            self.save_config();
            self.dirty = true;
        } else {
            self.send_layout_claim();
        }
    }

    fn on_session_message(&mut self, id: ConnId, message: Message) {
        if self.active != Some(id) {
            return;
        }
        match message {
            Message::Ping { nonce } => {
                self.send_active(Message::Pong { nonce });
            }
            Message::Pong { nonce } => {
                let now = self.started.elapsed().as_millis() as u64;
                if nonce <= now {
                    self.rtt_ms = Some((now - nonce).min(u64::from(u32::MAX)) as u32);
                }
            }
            Message::Layout(claim) => self.on_layout_claim(claim),
            Message::Enter(entry) => self.on_remote_enter(entry),
            Message::Leave { side, ratio } => self.on_remote_leave(side, ratio),
            Message::Release => self.on_remote_release(),
            Message::Refuse { reason } => self.on_remote_refuse(reason),
            Message::Input(event) => self.on_remote_input(event),
            Message::ClipboardBegin { transfer_id, format: _, total_len, sha256 } => self.on_clipboard_begin(transfer_id, total_len, sha256),
            Message::ClipboardChunk { transfer_id, offset, data } => self.on_clipboard_chunk(transfer_id, offset, &data),
            other => {
                tracing::warn!(kind = other.kind(), "unexpected message in session");
                self.close_conn(id, "protocol_violation", Some(GoodbyeReason::ProtocolViolation));
            }
        }
    }

    pub fn heartbeat(&mut self, now: Instant) {
        let mut expired = Vec::new();
        for (id, conn) in &self.conns {
            let idle = now.duration_since(conn.last_rx).as_millis() as u64;
            let age = now.duration_since(conn.opened_at).as_millis() as u64;
            let limit = match conn.state {
                ConnState::Established { .. } => PEER_TIMEOUT_MS,
                ConnState::AwaitHello => HELLO_TIMEOUT_MS,
                _ => PAIRING_STEP_TIMEOUT_MS,
            };
            let reference = if conn.is_established() { idle } else { age };
            if reference > limit {
                expired.push(*id);
            }
        }
        for id in expired {
            if self.active == Some(id) {
                self.set_error("peer_timeout");
            }
            self.close_conn(id, "timeout", None);
        }
        if self.active.is_some() {
            let nonce = self.started.elapsed().as_millis() as u64;
            self.send_active(Message::Ping { nonce });
        }
    }

    pub fn set_manual_address(&mut self, address: Option<String>) -> Result<(), crate::RuntimeError> {
        let normalized = match address.as_deref().map(str::trim).filter(|value| !value.is_empty()) {
            Some(value) => Some(normalize_address(value, DEFAULT_PORT).map_err(|_| crate::RuntimeError::new("address_invalid"))?),
            None => None,
        };
        let Some(peer) = self.config.peer.as_mut() else {
            return Err(crate::RuntimeError::new("not_paired"));
        };
        peer.manual_address = normalized;
        if !self.save_config() {
            return Err(crate::RuntimeError::new("config_save_failed"));
        }
        self.backoff.reset();
        if self.active.is_none() {
            self.next_dial_at = Some(Instant::now());
        }
        Ok(())
    }

    pub fn remove_peer(&mut self) -> Result<(), crate::RuntimeError> {
        self.release_control_everywhere();
        if let Some(active) = self.active {
            self.close_conn(active, "unpaired", Some(GoodbyeReason::Unpaired));
        }
        self.config.peer = None;
        self.ever_connected = false;
        self.next_dial_at = None;
        self.backoff.reset();
        if !self.save_config() {
            return Err(crate::RuntimeError::new("config_save_failed"));
        }
        Ok(())
    }
}
