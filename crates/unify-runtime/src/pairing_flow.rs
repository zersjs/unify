use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use tokio::time::Instant;
use unify_core::config::{normalize_address, PairedPeer, DEFAULT_PORT};
use unify_proto::{GoodbyeReason, Message, PairIdentity, PROTOCOL_VERSION};
use unify_security::pairing::{normalize_code, PairingExchange, PairingWindow, Role, Transcript, WindowStatus, PAIRING_CONFIRM_TIMEOUT_MS};
use unify_security::tls::{client_config, ServerExpectation};
use unify_security::Fingerprint;

use crate::actor::{now_ms, Actor};
use crate::session::{ConnId, ConnState, Purpose};
use crate::status::PairingStatus;
use crate::RuntimeError;

pub(crate) enum PairingActivity {
    Hosting { window: PairingWindow, wall_expiry_ms: u64, responder: Option<ConnId> },
    Joining { address: String, conn: Option<ConnId>, awaiting_peer_approval: Option<String> },
    AwaitingApproval { conn: ConnId, identity: PairIdentity, certificate: Vec<u8>, deadline: Instant },
    Finished(PairingStatus),
}

impl PairingActivity {
    pub fn status(&self) -> PairingStatus {
        match self {
            PairingActivity::Hosting { window, wall_expiry_ms, .. } => {
                PairingStatus::Hosting { code: window.code().to_owned(), expires_at_ms: *wall_expiry_ms }
            }
            PairingActivity::Joining { awaiting_peer_approval: Some(name), .. } => {
                PairingStatus::AwaitingPeerApproval { peer_name: name.clone() }
            }
            PairingActivity::Joining { address, .. } => PairingStatus::Joining { address: address.clone() },
            PairingActivity::AwaitingApproval { identity, certificate, .. } => PairingStatus::AwaitingApproval {
                peer_name: identity.device_name.clone(),
                peer_os: identity.os.as_str(),
                fingerprint: Fingerprint::of(certificate).short(),
            },
            PairingActivity::Finished(status) => status.clone(),
        }
    }
}

impl Actor {
    fn pairing_clock_ms(&self) -> u64 {
        self.started.elapsed().as_millis() as u64
    }

    pub fn start_pairing_host(&mut self) -> Result<(), RuntimeError> {
        if self.listen_port == 0 {
            return Err(RuntimeError::new("listen_failed"));
        }
        self.cancel_pairing();
        let window = PairingWindow::open(self.pairing_clock_ms());
        let wall_expiry_ms = now_ms() + unify_security::pairing::PAIRING_CODE_TTL_MS;
        self.pairing = Some(PairingActivity::Hosting { window, wall_expiry_ms, responder: None });
        self.dirty = true;
        Ok(())
    }

    pub fn cancel_pairing(&mut self) {
        let conn = match self.pairing.take() {
            Some(PairingActivity::Hosting { responder, .. }) => responder,
            Some(PairingActivity::Joining { conn, .. }) => conn,
            Some(PairingActivity::AwaitingApproval { conn, .. }) => {
                if let Some(c) = self.conns.get(&conn) {
                    let _ = c.outbound.send_high(Message::PairResult { accepted: false });
                }
                Some(conn)
            }
            Some(PairingActivity::Finished(_)) | None => None,
        };
        if let Some(conn) = conn {
            self.close_conn(conn, "pairing_cancelled", None);
        }
        self.pairing = None;
        self.dirty = true;
    }

    pub fn join_pairing(&mut self, address: &str, code: &str) -> Result<(), RuntimeError> {
        let code = normalize_code(code).ok_or(RuntimeError::new("pairing_code_invalid"))?;
        let address = normalize_address(address, DEFAULT_PORT).map_err(|_| RuntimeError::new("address_invalid"))?;
        self.cancel_pairing();
        let config = client_config(&self.identity, ServerExpectation::AnyForPairing).map_err(|_| RuntimeError::new("tls_config_failed"))?;
        self.pairing = Some(PairingActivity::Joining { address: address.clone(), conn: None, awaiting_peer_approval: None });
        self.spawn_dial(vec![address], config, Purpose::Pairing { code });
        self.dirty = true;
        Ok(())
    }

    pub fn pairing_attach_conn(&mut self, id: ConnId) {
        if let Some(PairingActivity::Joining { conn, .. }) = self.pairing.as_mut() {
            *conn = Some(id);
        }
    }

    pub fn pairing_failed(&mut self, reason: &'static str) {
        tracing::info!(reason, "pairing failed");
        let conn = match self.pairing.take() {
            Some(PairingActivity::Joining { conn, .. }) => conn,
            Some(PairingActivity::AwaitingApproval { conn, .. }) => Some(conn),
            Some(PairingActivity::Hosting { responder, .. }) => responder,
            _ => None,
        };
        self.pairing = Some(PairingActivity::Finished(PairingStatus::Failed { reason }));
        if let Some(conn) = conn {
            self.close_conn(conn, reason, None);
        }
        self.dirty = true;
    }

    pub fn pairing_conn_closed(&mut self, id: ConnId) {
        let interrupted = match self.pairing.as_mut() {
            Some(PairingActivity::Joining { conn, .. }) => *conn == Some(id),
            Some(PairingActivity::AwaitingApproval { conn, .. }) => *conn == id,
            Some(PairingActivity::Hosting { responder, .. }) => {
                if *responder == Some(id) {
                    *responder = None;
                }
                false
            }
            _ => false,
        };
        if interrupted {
            self.pairing = Some(PairingActivity::Finished(PairingStatus::Failed { reason: "pairing_interrupted" }));
            self.dirty = true;
        }
    }

    pub fn expire_pairing(&mut self, now: Instant) {
        let clock = self.pairing_clock_ms();
        match self.pairing.as_ref() {
            Some(PairingActivity::Hosting { window, .. }) => match window.status(clock) {
                WindowStatus::Open => {}
                WindowStatus::Expired => self.pairing_failed("pairing_expired"),
                WindowStatus::Exhausted => self.pairing_failed("pairing_exhausted"),
            },
            Some(PairingActivity::AwaitingApproval { deadline, conn, .. }) if now >= *deadline => {
                let conn = *conn;
                if let Some(c) = self.conns.get(&conn) {
                    let _ = c.outbound.send_high(Message::PairResult { accepted: false });
                }
                self.pairing_failed("pairing_timeout");
            }
            _ => {}
        }
    }

    fn persist_peer(&mut self, identity: &PairIdentity, certificate: &[u8], last_address: Option<String>) {
        let fingerprint = Fingerprint::of(certificate);
        if let Some(active) = self.active {
            self.close_conn(active, "repaired", Some(GoodbyeReason::Unpaired));
        }
        self.config.peer = Some(PairedPeer {
            device_id: identity.device_id.clone(),
            device_name: identity.device_name.clone(),
            os: identity.os,
            certificate: BASE64.encode(certificate),
            fingerprint: fingerprint.to_hex(),
            paired_at: now_ms(),
            manual_address: None,
            last_address,
        });
        self.ever_connected = false;
        self.save_config();
        self.backoff.reset();
        self.next_dial_at = Some(Instant::now() + std::time::Duration::from_millis(200));
        tracing::info!(peer = %fingerprint.short(), "device paired");
    }

    pub fn approve_pairing(&mut self, accept: bool) {
        let Some(PairingActivity::AwaitingApproval { conn, identity, certificate, .. }) = self.pairing.take() else {
            return;
        };
        if let Some(c) = self.conns.get(&conn) {
            let _ = c.outbound.send_high(Message::PairResult { accepted: accept });
        }
        if accept {
            self.persist_peer(&identity, &certificate, None);
            self.pairing = Some(PairingActivity::Finished(PairingStatus::Succeeded { peer_name: identity.device_name.clone() }));
        } else {
            self.pairing = Some(PairingActivity::Finished(PairingStatus::Failed { reason: "pairing_rejected_local" }));
        }
        self.close_conn(conn, "pairing_complete", None);
        self.dirty = true;
    }

    fn reject_pairing_conn(&mut self, id: ConnId) {
        self.stats.rejected_connections += 1;
        self.close_conn(id, "pairing_not_allowed", Some(GoodbyeReason::ProtocolViolation));
    }

    pub fn on_pairing_message(&mut self, id: ConnId, message: Message) {
        let Some(conn) = self.conns.get_mut(&id) else {
            return;
        };
        let state = std::mem::replace(&mut conn.state, ConnState::PairResponderAwaitApproval);
        let binding = conn.info.channel_binding;
        let peer_fingerprint = conn.info.fingerprint;
        let peer_certificate = conn.info.certificate.clone();
        let dialed = conn.dialed.clone();
        let own_fingerprint = self.identity.fingerprint();
        let own_id = self.identity.device_id().to_owned();
        match (state, message) {
            (ConnState::AwaitHello, Message::PairStart { identity, spake_message }) => {
                let clock = self.pairing_clock_ms();
                let code = match self.pairing.as_mut() {
                    Some(PairingActivity::Hosting { window, responder, .. }) if window.status(clock) == WindowStatus::Open && responder.is_none() => {
                        *responder = Some(id);
                        window.code().to_owned()
                    }
                    _ => {
                        self.reject_pairing_conn(id);
                        return;
                    }
                };
                if identity.protocol_version != PROTOCOL_VERSION {
                    self.close_conn(id, "version_mismatch", Some(GoodbyeReason::VersionMismatch));
                    return;
                }
                let (exchange, reply) = PairingExchange::start(&code);
                let Ok(key) = exchange.finish(&spake_message) else {
                    self.close_conn(id, "spake_failed", Some(GoodbyeReason::PairingRejected));
                    return;
                };
                let transcript = Transcript {
                    channel_binding: binding,
                    initiator_fingerprint: peer_fingerprint,
                    responder_fingerprint: own_fingerprint,
                    initiator_device_id: identity.device_id.clone(),
                    responder_device_id: own_id,
                };
                let own_identity = self.own_pair_identity();
                if let Some(conn) = self.conns.get_mut(&id) {
                    let _ = conn.outbound.send_high(Message::PairReply { identity: own_identity, spake_message: reply });
                    conn.state = ConnState::PairResponderAwaitConfirm { key, transcript, identity };
                }
            }
            (ConnState::PairResponderAwaitConfirm { key, transcript, identity }, Message::PairConfirm { mac }) => {
                if !key.verify(Role::Initiator, &transcript, &mac) {
                    let exhausted = match self.pairing.as_mut() {
                        Some(PairingActivity::Hosting { window, responder, .. }) => {
                            *responder = None;
                            window.record_failure() == WindowStatus::Exhausted
                        }
                        _ => false,
                    };
                    tracing::warn!("pairing confirmation failed (wrong code or interception)");
                    self.close_conn(id, "pairing_confirm_failed", Some(GoodbyeReason::PairingRejected));
                    if exhausted {
                        self.pairing_failed("pairing_exhausted");
                    }
                    return;
                }
                let confirmation = key.confirmation(Role::Responder, &transcript);
                if let Some(conn) = self.conns.get_mut(&id) {
                    let _ = conn.outbound.send_high(Message::PairConfirm { mac: confirmation });
                    conn.state = ConnState::PairResponderAwaitApproval;
                }
                self.pairing = Some(PairingActivity::AwaitingApproval {
                    conn: id,
                    identity,
                    certificate: peer_certificate,
                    deadline: Instant::now() + std::time::Duration::from_millis(PAIRING_CONFIRM_TIMEOUT_MS),
                });
                self.dirty = true;
            }
            (ConnState::PairInitiatorAwaitReply { exchange: Some(exchange) }, Message::PairReply { identity, spake_message }) => {
                let Ok(key) = exchange.finish(&spake_message) else {
                    self.pairing_failed("pairing_code_mismatch");
                    return;
                };
                let transcript = Transcript {
                    channel_binding: binding,
                    initiator_fingerprint: own_fingerprint,
                    responder_fingerprint: peer_fingerprint,
                    initiator_device_id: own_id,
                    responder_device_id: identity.device_id.clone(),
                };
                let confirmation = key.confirmation(Role::Initiator, &transcript);
                if let Some(conn) = self.conns.get_mut(&id) {
                    let _ = conn.outbound.send_high(Message::PairConfirm { mac: confirmation });
                    conn.state = ConnState::PairInitiatorAwaitConfirm { key, transcript, identity };
                }
            }
            (ConnState::PairInitiatorAwaitConfirm { key, transcript, identity }, Message::PairConfirm { mac }) => {
                if !key.verify(Role::Responder, &transcript, &mac) {
                    self.pairing_failed("pairing_code_mismatch");
                    return;
                }
                if let Some(PairingActivity::Joining { awaiting_peer_approval, .. }) = self.pairing.as_mut() {
                    *awaiting_peer_approval = Some(identity.device_name.clone());
                }
                if let Some(conn) = self.conns.get_mut(&id) {
                    conn.state = ConnState::PairInitiatorAwaitResult { identity };
                }
                self.dirty = true;
            }
            (ConnState::PairInitiatorAwaitResult { identity }, Message::PairResult { accepted }) => {
                if accepted {
                    self.persist_peer(&identity, &peer_certificate, dialed);
                    self.pairing = Some(PairingActivity::Finished(PairingStatus::Succeeded { peer_name: identity.device_name.clone() }));
                    self.close_conn(id, "pairing_complete", None);
                } else {
                    self.pairing_failed("pairing_rejected_remote");
                }
                self.dirty = true;
            }
            (_, other) => {
                tracing::warn!(kind = other.kind(), "unexpected pairing message");
                if matches!(self.pairing, Some(PairingActivity::Joining { conn: Some(c), .. }) if c == id) {
                    self.pairing_failed("pairing_protocol_error");
                } else {
                    self.close_conn(id, "pairing_protocol_error", Some(GoodbyeReason::ProtocolViolation));
                }
            }
        }
    }
}
