use hmac::{Hmac, Mac};
use rand::rngs::OsRng;
use rand::Rng;
use sha2::Sha256;
use spake2::{Ed25519Group, Identity, Password, Spake2};
use zeroize::Zeroizing;

use crate::identity::Fingerprint;
use crate::SecurityError;

pub const PAIRING_CODE_DIGITS: usize = 6;
pub const PAIRING_CODE_TTL_MS: u64 = 120_000;
pub const MAX_PAIRING_FAILURES: u32 = 5;
pub const PAIRING_CONFIRM_TIMEOUT_MS: u64 = 60_000;
const SPAKE_IDENTITY: &[u8] = b"unify-pairing-v1";
const CONFIRM_LABEL: &[u8] = b"unify-pair-confirm-v1";

type HmacSha256 = Hmac<Sha256>;

pub fn generate_code() -> String {
    let value: u32 = OsRng.gen_range(0..1_000_000);
    format!("{value:06}")
}

pub fn normalize_code(input: &str) -> Option<String> {
    let digits: String = input.chars().filter(|c| !c.is_whitespace() && *c != '-').collect();
    if digits.len() == PAIRING_CODE_DIGITS && digits.chars().all(|c| c.is_ascii_digit()) {
        Some(digits)
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Initiator,
    Responder,
}

impl Role {
    fn tag(self) -> u8 {
        match self {
            Role::Initiator => 0x01,
            Role::Responder => 0x02,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Transcript {
    pub channel_binding: [u8; 32],
    pub initiator_fingerprint: Fingerprint,
    pub responder_fingerprint: Fingerprint,
    pub initiator_device_id: String,
    pub responder_device_id: String,
}

impl Transcript {
    fn feed(&self, mac: &mut HmacSha256) {
        mac.update(CONFIRM_LABEL);
        mac.update(&self.channel_binding);
        mac.update(&self.initiator_fingerprint.0);
        mac.update(&self.responder_fingerprint.0);
        for id in [&self.initiator_device_id, &self.responder_device_id] {
            mac.update(&(id.len() as u32).to_be_bytes());
            mac.update(id.as_bytes());
        }
    }
}

pub struct PairingExchange {
    state: Spake2<Ed25519Group>,
}

impl PairingExchange {
    pub fn start(code: &str) -> (Self, Vec<u8>) {
        let (state, message) = Spake2::<Ed25519Group>::start_symmetric(&Password::new(code.as_bytes()), &Identity::new(SPAKE_IDENTITY));
        (Self { state }, message)
    }

    pub fn finish(self, peer_message: &[u8]) -> Result<PairingKey, SecurityError> {
        let key = self.state.finish(peer_message).map_err(|_| SecurityError::KeyExchange)?;
        Ok(PairingKey { key: Zeroizing::new(key) })
    }
}

pub struct PairingKey {
    key: Zeroizing<Vec<u8>>,
}

impl PairingKey {
    fn mac(&self, role: Role, transcript: &Transcript) -> HmacSha256 {
        let mut mac = <HmacSha256 as Mac>::new_from_slice(&self.key).unwrap_or_else(|_| unreachable!("HMAC accepts any key length"));
        transcript.feed(&mut mac);
        mac.update(&[role.tag()]);
        mac
    }

    pub fn confirmation(&self, role: Role, transcript: &Transcript) -> [u8; 32] {
        self.mac(role, transcript).finalize().into_bytes().into()
    }

    pub fn verify(&self, role: Role, transcript: &Transcript, received: &[u8; 32]) -> bool {
        self.mac(role, transcript).verify_slice(received).is_ok()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowStatus {
    Open,
    Expired,
    Exhausted,
}

#[derive(Debug, Clone)]
pub struct PairingWindow {
    code: Zeroizing<String>,
    expires_at_ms: u64,
    failures: u32,
}

impl PairingWindow {
    pub fn open(now_ms: u64) -> Self {
        Self { code: Zeroizing::new(generate_code()), expires_at_ms: now_ms + PAIRING_CODE_TTL_MS, failures: 0 }
    }

    pub fn with_code(code: &str, now_ms: u64) -> Self {
        Self { code: Zeroizing::new(code.to_owned()), expires_at_ms: now_ms + PAIRING_CODE_TTL_MS, failures: 0 }
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn expires_at_ms(&self) -> u64 {
        self.expires_at_ms
    }

    pub fn status(&self, now_ms: u64) -> WindowStatus {
        if self.failures >= MAX_PAIRING_FAILURES {
            WindowStatus::Exhausted
        } else if now_ms >= self.expires_at_ms {
            WindowStatus::Expired
        } else {
            WindowStatus::Open
        }
    }

    pub fn record_failure(&mut self) -> WindowStatus {
        self.failures = self.failures.saturating_add(1);
        if self.failures >= MAX_PAIRING_FAILURES {
            WindowStatus::Exhausted
        } else {
            WindowStatus::Open
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn transcript(binding: u8) -> Transcript {
        Transcript {
            channel_binding: [binding; 32],
            initiator_fingerprint: Fingerprint([1; 32]),
            responder_fingerprint: Fingerprint([2; 32]),
            initiator_device_id: "aa".into(),
            responder_device_id: "bb".into(),
        }
    }

    fn exchange(code_a: &str, code_b: &str) -> (PairingKey, PairingKey) {
        let (a, msg_a) = PairingExchange::start(code_a);
        let (b, msg_b) = PairingExchange::start(code_b);
        (a.finish(&msg_b).unwrap(), b.finish(&msg_a).unwrap())
    }

    #[test]
    fn matching_codes_confirm_both_directions() {
        let (initiator, responder) = exchange("123456", "123456");
        let t = transcript(9);
        let from_initiator = initiator.confirmation(Role::Initiator, &t);
        let from_responder = responder.confirmation(Role::Responder, &t);
        assert!(responder.verify(Role::Initiator, &t, &from_initiator));
        assert!(initiator.verify(Role::Responder, &t, &from_responder));
    }

    #[test]
    fn mismatched_codes_fail_confirmation() {
        let (initiator, responder) = exchange("123456", "654321");
        let t = transcript(9);
        assert!(!responder.verify(Role::Initiator, &t, &initiator.confirmation(Role::Initiator, &t)));
    }

    #[test]
    fn different_channel_binding_fails_confirmation() {
        let (initiator, responder) = exchange("000001", "000001");
        let mac = initiator.confirmation(Role::Initiator, &transcript(1));
        assert!(!responder.verify(Role::Initiator, &transcript(2), &mac));
    }

    #[test]
    fn reflected_confirmation_is_rejected() {
        let (initiator, responder) = exchange("111111", "111111");
        let t = transcript(3);
        let mac = responder.confirmation(Role::Responder, &t);
        assert!(!initiator.verify(Role::Initiator, &t, &mac));
    }

    #[test]
    fn swapped_fingerprints_fail_confirmation() {
        let (initiator, responder) = exchange("222222", "222222");
        let t = transcript(4);
        let mut swapped = t.clone();
        std::mem::swap(&mut swapped.initiator_fingerprint, &mut swapped.responder_fingerprint);
        assert!(!responder.verify(Role::Initiator, &swapped, &initiator.confirmation(Role::Initiator, &t)));
    }

    #[test]
    fn rejects_malformed_spake_message() {
        let (a, _) = PairingExchange::start("123456");
        assert!(a.finish(&[0u8; 5]).is_err());
    }

    #[test]
    fn codes_are_six_digits_and_normalize() {
        for _ in 0..200 {
            let code = generate_code();
            assert_eq!(normalize_code(&code), Some(code));
        }
        assert_eq!(normalize_code("123 456"), Some("123456".into()));
        assert_eq!(normalize_code("123-456"), Some("123456".into()));
        assert_eq!(normalize_code("12345"), None);
        assert_eq!(normalize_code("12345a"), None);
    }

    #[test]
    fn window_expires_and_exhausts() {
        let mut window = PairingWindow::with_code("123456", 0);
        assert_eq!(window.status(PAIRING_CODE_TTL_MS - 1), WindowStatus::Open);
        assert_eq!(window.status(PAIRING_CODE_TTL_MS), WindowStatus::Expired);
        for _ in 0..MAX_PAIRING_FAILURES - 1 {
            assert_eq!(window.record_failure(), WindowStatus::Open);
        }
        assert_eq!(window.record_failure(), WindowStatus::Exhausted);
        assert_eq!(window.status(1), WindowStatus::Exhausted);
    }
}
