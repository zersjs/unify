use thiserror::Error;

pub const MAGIC: u16 = 0x554E;
pub const WIRE_VERSION: u8 = 1;
pub const HEADER_LEN: usize = 28;
pub const MAX_CHUNK_LEN: usize = 1_200;
pub const FLAG_KEYFRAME: u8 = 1;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PacketError {
    #[error("packet too short")]
    TooShort,
    #[error("unknown packet format")]
    BadFormat,
    #[error("frame too large")]
    FrameTooLarge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub keyframe: bool,
    pub sequence: u64,
    pub frame_id: u32,
    pub chunk_index: u16,
    pub chunk_count: u16,
    pub capture_us: u64,
}

impl Header {
    pub fn encode(&self) -> [u8; HEADER_LEN] {
        let mut out = [0u8; HEADER_LEN];
        out[0..2].copy_from_slice(&MAGIC.to_be_bytes());
        out[2] = WIRE_VERSION;
        out[3] = if self.keyframe { FLAG_KEYFRAME } else { 0 };
        out[4..12].copy_from_slice(&self.sequence.to_be_bytes());
        out[12..16].copy_from_slice(&self.frame_id.to_be_bytes());
        out[16..18].copy_from_slice(&self.chunk_index.to_be_bytes());
        out[18..20].copy_from_slice(&self.chunk_count.to_be_bytes());
        out[20..28].copy_from_slice(&self.capture_us.to_be_bytes());
        out
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, PacketError> {
        if bytes.len() < HEADER_LEN {
            return Err(PacketError::TooShort);
        }
        let read_u16 = |at: usize| u16::from_be_bytes([bytes[at], bytes[at + 1]]);
        if read_u16(0) != MAGIC || bytes[2] != WIRE_VERSION {
            return Err(PacketError::BadFormat);
        }
        let mut sequence = [0u8; 8];
        sequence.copy_from_slice(&bytes[4..12]);
        let mut frame_id = [0u8; 4];
        frame_id.copy_from_slice(&bytes[12..16]);
        let mut capture_us = [0u8; 8];
        capture_us.copy_from_slice(&bytes[20..28]);
        let header = Self {
            keyframe: bytes[3] & FLAG_KEYFRAME != 0,
            sequence: u64::from_be_bytes(sequence),
            frame_id: u32::from_be_bytes(frame_id),
            chunk_index: read_u16(16),
            chunk_count: read_u16(18),
            capture_us: u64::from_be_bytes(capture_us),
        };
        if header.chunk_count == 0 || header.chunk_index >= header.chunk_count {
            return Err(PacketError::BadFormat);
        }
        Ok(header)
    }
}

pub struct Chunk<'a> {
    pub index: u16,
    pub count: u16,
    pub data: &'a [u8],
}

pub fn packetize(frame: &[u8]) -> Result<impl Iterator<Item = Chunk<'_>>, PacketError> {
    let count = frame.len().div_ceil(MAX_CHUNK_LEN).max(1);
    let count = u16::try_from(count).map_err(|_| PacketError::FrameTooLarge)?;
    Ok((0..count).map(move |index| {
        let start = usize::from(index) * MAX_CHUNK_LEN;
        let end = (start + MAX_CHUNK_LEN).min(frame.len());
        Chunk { index, count, data: &frame[start..end] }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header() -> Header {
        Header { keyframe: true, sequence: 9, frame_id: 7, chunk_index: 2, chunk_count: 3, capture_us: 123_456 }
    }

    #[test]
    fn header_roundtrips() {
        assert_eq!(Header::decode(&header().encode()), Ok(header()));
    }

    #[test]
    fn rejects_short_foreign_and_inconsistent_headers() {
        assert_eq!(Header::decode(&[0; 4]), Err(PacketError::TooShort));
        assert_eq!(Header::decode(&[0; HEADER_LEN]), Err(PacketError::BadFormat));
        let mut bad = header();
        bad.chunk_index = 3;
        assert_eq!(Header::decode(&bad.encode()), Err(PacketError::BadFormat));
        bad.chunk_count = 0;
        bad.chunk_index = 0;
        assert_eq!(Header::decode(&bad.encode()), Err(PacketError::BadFormat));
    }

    #[test]
    fn packetize_covers_frame_exactly() {
        let frame: Vec<u8> = (0..(MAX_CHUNK_LEN * 2 + 5)).map(|i| i as u8).collect();
        let chunks: Vec<_> = packetize(&frame).unwrap().collect();
        assert_eq!(chunks.len(), 3);
        let joined: Vec<u8> = chunks.iter().flat_map(|c| c.data.iter().copied()).collect();
        assert_eq!(joined, frame);
        assert!(chunks.iter().all(|c| c.count == 3));
    }

    #[test]
    fn empty_frame_is_one_empty_chunk_and_oversized_is_rejected() {
        assert_eq!(packetize(&[]).unwrap().count(), 1);
        let huge = vec![0u8; MAX_CHUNK_LEN * 70_000];
        assert!(matches!(packetize(&huge), Err(PacketError::FrameTooLarge)));
    }
}
