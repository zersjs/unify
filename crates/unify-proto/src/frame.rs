use thiserror::Error;

use crate::Message;

pub const FRAME_HEADER_LEN: usize = 4;
pub const MAX_FRAME_LEN: usize = 64 * 1024;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProtoError {
    #[error("frame length {0} exceeds limit")]
    FrameTooLarge(usize),
    #[error("empty frame")]
    EmptyFrame,
    #[error("malformed payload")]
    Malformed,
    #[error("invalid field: {0}")]
    Invalid(&'static str),
}

pub fn encode_frame(message: &Message) -> Result<Vec<u8>, ProtoError> {
    message.validate()?;
    let payload = postcard::to_stdvec(message).map_err(|_| ProtoError::Malformed)?;
    if payload.len() > MAX_FRAME_LEN {
        return Err(ProtoError::FrameTooLarge(payload.len()));
    }
    let mut frame = Vec::with_capacity(FRAME_HEADER_LEN + payload.len());
    frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    frame.extend_from_slice(&payload);
    Ok(frame)
}

pub fn parse_frame_len(header: [u8; FRAME_HEADER_LEN]) -> Result<usize, ProtoError> {
    let len = u32::from_be_bytes(header) as usize;
    if len == 0 {
        return Err(ProtoError::EmptyFrame);
    }
    if len > MAX_FRAME_LEN {
        return Err(ProtoError::FrameTooLarge(len));
    }
    Ok(len)
}

pub fn decode_payload(payload: &[u8]) -> Result<Message, ProtoError> {
    let (message, rest) = postcard::take_from_bytes::<Message>(payload).map_err(|_| ProtoError::Malformed)?;
    if !rest.is_empty() {
        return Err(ProtoError::Malformed);
    }
    message.validate()?;
    Ok(message)
}

#[derive(Default)]
pub struct FrameDecoder {
    buffer: Vec<u8>,
}

impl FrameDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);
    }

    pub fn next_message(&mut self) -> Result<Option<Message>, ProtoError> {
        if self.buffer.len() < FRAME_HEADER_LEN {
            return Ok(None);
        }
        let header: [u8; FRAME_HEADER_LEN] = self.buffer[..FRAME_HEADER_LEN].try_into().map_err(|_| ProtoError::Malformed)?;
        let len = parse_frame_len(header)?;
        if self.buffer.len() < FRAME_HEADER_LEN + len {
            return Ok(None);
        }
        let message = decode_payload(&self.buffer[FRAME_HEADER_LEN..FRAME_HEADER_LEN + len]);
        self.buffer.drain(..FRAME_HEADER_LEN + len);
        message.map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    fn roundtrip(message: Message) {
        let frame = encode_frame(&message).expect("encode");
        let mut decoder = FrameDecoder::new();
        decoder.push(&frame);
        assert_eq!(decoder.next_message().expect("decode"), Some(message));
        assert_eq!(decoder.next_message().expect("drained"), None);
    }

    #[test]
    fn roundtrips_every_message_kind() {
        let identity = PairIdentity {
            protocol_version: PROTOCOL_VERSION,
            device_id: "0123abcd".into(),
            device_name: "Resul’un MacBook’u".into(),
            os: OsKind::MacOs,
        };
        let messages = vec![
            Message::Hello(Hello {
                protocol_version: PROTOCOL_VERSION,
                device_id: "ab12".into(),
                device_name: "Çalışma Masası".into(),
                os: OsKind::Windows,
                app_version: "0.1.0".into(),
                listen_port: 47823,
            }),
            Message::Layout(LayoutClaim { receiver_side: Side::Right, revision: 42 }),
            Message::Ping { nonce: u64::MAX },
            Message::Pong { nonce: 7 },
            Message::Enter(EntryPoint::Edge { side: Side::Left, ratio: 5000 }),
            Message::Enter(EntryPoint::Center),
            Message::Leave { side: Side::Left, ratio: 0 },
            Message::Release,
            Message::Input(InputEvent::Key { usage: 0xE3, down: true }),
            Message::Input(InputEvent::MouseMove { dx: -3, dy: 12 }),
            Message::Input(InputEvent::MouseButton { button: MouseButton::Forward, down: false }),
            Message::Input(InputEvent::Wheel { dx: 0, dy: -120 }),
            Message::ClipboardBegin { transfer_id: 9, format: ClipboardFormat::PlainText, total_len: 10, sha256: [7; 32] },
            Message::ClipboardChunk { transfer_id: 9, offset: 0, data: "ğüşıöç İ".as_bytes().to_vec() },
            Message::Goodbye { reason: GoodbyeReason::Replaced },
            Message::PairStart { identity: identity.clone(), spake_message: vec![1; SPAKE_MESSAGE_LEN] },
            Message::PairReply { identity, spake_message: vec![2; SPAKE_MESSAGE_LEN] },
            Message::PairConfirm { mac: [3; 32] },
            Message::PairResult { accepted: true },
            Message::Refuse { reason: RefuseReason::PermissionMissing },
        ];
        for message in messages {
            roundtrip(message);
        }
    }

    #[test]
    fn decodes_frames_split_across_reads() {
        let first = encode_frame(&Message::Ping { nonce: 1 }).unwrap();
        let second = encode_frame(&Message::Pong { nonce: 2 }).unwrap();
        let mut stream = first.clone();
        stream.extend_from_slice(&second);
        let mut decoder = FrameDecoder::new();
        let mut decoded = Vec::new();
        for byte in stream {
            decoder.push(&[byte]);
            while let Some(message) = decoder.next_message().unwrap() {
                decoded.push(message);
            }
        }
        assert_eq!(decoded, vec![Message::Ping { nonce: 1 }, Message::Pong { nonce: 2 }]);
    }

    #[test]
    fn rejects_oversized_length_header_before_buffering_payload() {
        let mut decoder = FrameDecoder::new();
        decoder.push(&((MAX_FRAME_LEN as u32) + 1).to_be_bytes());
        assert_eq!(decoder.next_message(), Err(ProtoError::FrameTooLarge(MAX_FRAME_LEN + 1)));
    }

    #[test]
    fn rejects_zero_length_frame() {
        let mut decoder = FrameDecoder::new();
        decoder.push(&0u32.to_be_bytes());
        assert_eq!(decoder.next_message(), Err(ProtoError::EmptyFrame));
    }

    #[test]
    fn rejects_garbage_payload() {
        assert_eq!(decode_payload(&[0xFF, 0xFF, 0xFF]), Err(ProtoError::Malformed));
    }

    #[test]
    fn rejects_trailing_bytes_after_message() {
        let mut frame = encode_frame(&Message::Release).unwrap();
        frame.push(0);
        assert_eq!(decode_payload(&frame[FRAME_HEADER_LEN..]), Err(ProtoError::Malformed));
    }

    #[test]
    fn rejects_truncated_payload() {
        let frame = encode_frame(&Message::Ping { nonce: u64::MAX }).unwrap();
        assert_eq!(decode_payload(&frame[FRAME_HEADER_LEN..frame.len() - 1]), Err(ProtoError::Malformed));
    }

    #[test]
    fn rejects_out_of_range_ratio() {
        let message = Message::Leave { side: Side::Left, ratio: RATIO_SCALE + 1 };
        assert_eq!(encode_frame(&message), Err(ProtoError::Invalid("ratio")));
    }

    #[test]
    fn rejects_oversized_clipboard_chunk() {
        let message = Message::ClipboardChunk { transfer_id: 1, offset: 0, data: vec![0; CLIPBOARD_CHUNK_LEN + 1] };
        assert_eq!(encode_frame(&message), Err(ProtoError::Invalid("clipboard_chunk_len")));
    }

    #[test]
    fn rejects_non_hex_device_id_and_control_chars_in_name() {
        assert!(validate_device_id("zz").is_err());
        assert!(validate_device_id("").is_err());
        assert!(validate_device_name("a\u{0007}b").is_err());
        assert!(validate_device_name("").is_err());
        assert!(validate_device_name(&"ş".repeat(MAX_DEVICE_NAME_CHARS)).is_ok());
        assert!(validate_device_name(&"ş".repeat(MAX_DEVICE_NAME_CHARS + 1)).is_err());
    }

    #[test]
    fn rejects_wrong_spake_length() {
        let message = Message::PairConfirm { mac: [0; 32] };
        assert!(encode_frame(&message).is_ok());
        let bad = Message::PairStart {
            identity: PairIdentity { protocol_version: 1, device_id: "ab".into(), device_name: "x".into(), os: OsKind::Other },
            spake_message: vec![0; 10],
        };
        assert_eq!(encode_frame(&bad), Err(ProtoError::Invalid("spake_message")));
    }

    #[test]
    fn ratio_conversion_clamps_and_handles_nan() {
        assert_eq!(ratio_from_f64(-1.0), 0);
        assert_eq!(ratio_from_f64(2.0), RATIO_SCALE);
        assert_eq!(ratio_from_f64(f64::NAN), RATIO_SCALE / 2);
        assert!((ratio_to_f64(ratio_from_f64(0.25)) - 0.25).abs() < 1e-4);
    }
}
