use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::hmac;
use thiserror::Error;

use crate::packet::{Header, PacketError, HEADER_LEN};

const KEY_LABEL: &[u8] = b"unify-stream-v1";
const REPLAY_WINDOW_BITS: u64 = 1024;
const REPLAY_WORDS: usize = (REPLAY_WINDOW_BITS / 64) as usize;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CryptoError {
    #[error("packet: {0}")]
    Packet(#[from] PacketError),
    #[error("authentication failed")]
    Authentication,
    #[error("replayed or too old")]
    Replay,
    #[error("sequence exhausted")]
    Exhausted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Video,
    Feedback,
}

impl Direction {
    fn tag(self) -> u32 {
        match self {
            Direction::Video => 1,
            Direction::Feedback => 2,
        }
    }
}

pub fn derive_stream_key(channel_binding: &[u8; 32], session_id: u64) -> [u8; 32] {
    let key = hmac::Key::new(hmac::HMAC_SHA256, channel_binding);
    let mut context = hmac::Context::with_key(&key);
    context.update(KEY_LABEL);
    context.update(&session_id.to_be_bytes());
    let mut out = [0u8; 32];
    out.copy_from_slice(context.sign().as_ref());
    out
}

fn cipher(key: &[u8; 32]) -> LessSafeKey {
    match UnboundKey::new(&AES_256_GCM, key) {
        Ok(unbound) => LessSafeKey::new(unbound),
        Err(_) => unreachable!("AES-256 key length is fixed"),
    }
}

fn nonce(direction: Direction, sequence: u64) -> Nonce {
    let mut bytes = [0u8; 12];
    bytes[..4].copy_from_slice(&direction.tag().to_be_bytes());
    bytes[4..].copy_from_slice(&sequence.to_be_bytes());
    Nonce::assume_unique_for_key(bytes)
}

pub struct Sealer {
    key: LessSafeKey,
    direction: Direction,
    next_sequence: u64,
}

impl Sealer {
    pub fn new(key: &[u8; 32], direction: Direction) -> Self {
        Self { key: cipher(key), direction, next_sequence: 0 }
    }

    pub fn seal(&mut self, mut header: Header, payload: &[u8], out: &mut Vec<u8>) -> Result<(), CryptoError> {
        if self.next_sequence == u64::MAX {
            return Err(CryptoError::Exhausted);
        }
        header.sequence = self.next_sequence;
        self.next_sequence += 1;
        let encoded = header.encode();
        out.clear();
        out.extend_from_slice(&encoded);
        out.extend_from_slice(payload);
        let tag = self
            .key
            .seal_in_place_separate_tag(nonce(self.direction, header.sequence), Aad::from(encoded), &mut out[HEADER_LEN..])
            .map_err(|_| CryptoError::Authentication)?;
        out.extend_from_slice(tag.as_ref());
        Ok(())
    }
}

pub struct Opener {
    key: LessSafeKey,
    direction: Direction,
    highest: Option<u64>,
    window: [u64; REPLAY_WORDS],
}

impl Opener {
    pub fn new(key: &[u8; 32], direction: Direction) -> Self {
        Self { key: cipher(key), direction, highest: None, window: [0; REPLAY_WORDS] }
    }

    fn is_fresh(&self, sequence: u64) -> bool {
        match self.highest {
            None => true,
            Some(highest) if sequence > highest => true,
            Some(highest) if highest - sequence >= REPLAY_WINDOW_BITS => false,
            Some(_) => {
                let bit = sequence % REPLAY_WINDOW_BITS;
                self.window[(bit / 64) as usize] & (1 << (bit % 64)) == 0
            }
        }
    }

    fn mark(&mut self, sequence: u64) {
        let highest = self.highest.unwrap_or(sequence);
        if sequence > highest {
            let gap = (sequence - highest).min(REPLAY_WINDOW_BITS);
            for step in 1..=gap {
                let bit = (highest + step) % REPLAY_WINDOW_BITS;
                self.window[(bit / 64) as usize] &= !(1 << (bit % 64));
            }
        }
        self.highest = Some(highest.max(sequence));
        let bit = sequence % REPLAY_WINDOW_BITS;
        self.window[(bit / 64) as usize] |= 1 << (bit % 64);
    }

    pub fn open<'a>(&mut self, packet: &'a mut [u8]) -> Result<(Header, &'a [u8]), CryptoError> {
        let header = Header::decode(packet)?;
        if !self.is_fresh(header.sequence) {
            return Err(CryptoError::Replay);
        }
        let (aad, body) = packet.split_at_mut(HEADER_LEN);
        let plain = self
            .key
            .open_in_place(nonce(self.direction, header.sequence), Aad::from(&*aad), body)
            .map_err(|_| CryptoError::Authentication)?;
        self.mark(header.sequence);
        Ok((header, plain))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header() -> Header {
        Header { keyframe: false, sequence: 0, frame_id: 1, chunk_index: 0, chunk_count: 1, capture_us: 5 }
    }

    fn pair() -> (Sealer, Opener) {
        let key = derive_stream_key(&[7; 32], 42);
        (Sealer::new(&key, Direction::Video), Opener::new(&key, Direction::Video))
    }

    #[test]
    fn seals_and_opens_payload() {
        let (mut sealer, mut opener) = pair();
        let mut packet = Vec::new();
        sealer.seal(header(), b"frame", &mut packet).unwrap();
        assert!(!packet.windows(5).any(|w| w == b"frame"));
        let (decoded, plain) = opener.open(&mut packet).unwrap();
        assert_eq!(plain, b"frame");
        assert_eq!(decoded.frame_id, 1);
    }

    #[test]
    fn rejects_tampered_payload_and_header() {
        let (mut sealer, mut opener) = pair();
        let mut packet = Vec::new();
        sealer.seal(header(), b"frame", &mut packet).unwrap();
        let mut body = packet.clone();
        body[HEADER_LEN] ^= 1;
        assert_eq!(opener.open(&mut body).unwrap_err(), CryptoError::Authentication);
        let mut head = packet.clone();
        head[15] ^= 1;
        assert_eq!(opener.open(&mut head).unwrap_err(), CryptoError::Authentication);
        assert!(opener.open(&mut packet).is_ok());
    }

    #[test]
    fn rejects_replay_but_accepts_reordering() {
        let (mut sealer, mut opener) = pair();
        let mut first = Vec::new();
        let mut second = Vec::new();
        sealer.seal(header(), b"a", &mut first).unwrap();
        sealer.seal(header(), b"b", &mut second).unwrap();
        let mut replay = second.clone();
        assert!(opener.open(&mut second).is_ok());
        assert!(opener.open(&mut first).is_ok());
        assert_eq!(opener.open(&mut replay).unwrap_err(), CryptoError::Replay);
    }

    #[test]
    fn rejects_packets_older_than_the_window() {
        let (mut sealer, mut opener) = pair();
        let mut old = Vec::new();
        sealer.seal(header(), b"old", &mut old).unwrap();
        let mut packet = Vec::new();
        for _ in 0..=REPLAY_WINDOW_BITS {
            sealer.seal(header(), b"x", &mut packet).unwrap();
        }
        assert!(opener.open(&mut packet).is_ok());
        assert_eq!(opener.open(&mut old).unwrap_err(), CryptoError::Replay);
    }

    #[test]
    fn wrong_key_or_direction_fails() {
        let key = derive_stream_key(&[7; 32], 42);
        let mut packet = Vec::new();
        Sealer::new(&key, Direction::Video).seal(header(), b"x", &mut packet).unwrap();
        let mut copy = packet.clone();
        assert_eq!(Opener::new(&key, Direction::Feedback).open(&mut copy).unwrap_err(), CryptoError::Authentication);
        let other = derive_stream_key(&[7; 32], 43);
        assert_eq!(Opener::new(&other, Direction::Video).open(&mut packet).unwrap_err(), CryptoError::Authentication);
    }
}
