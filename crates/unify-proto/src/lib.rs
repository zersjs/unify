pub mod frame;
pub mod keymap;
pub mod message;

pub use frame::{decode_payload, encode_frame, FrameDecoder, ProtoError, FRAME_HEADER_LEN, MAX_FRAME_LEN};
pub use message::*;

pub const PROTOCOL_VERSION: u16 = 1;
pub const CLIPBOARD_CHUNK_LEN: usize = 16 * 1024;
pub const MAX_CLIPBOARD_TEXT_LEN: usize = 16 * 1024 * 1024;
pub const MAX_DEVICE_NAME_CHARS: usize = 64;
pub const MAX_DEVICE_ID_LEN: usize = 64;
pub const RATIO_SCALE: u16 = 10_000;
pub const SPAKE_MESSAGE_LEN: usize = 33;
