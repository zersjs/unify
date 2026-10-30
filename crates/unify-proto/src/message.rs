use serde::{Deserialize, Serialize};

use crate::{
    ProtoError, CLIPBOARD_CHUNK_LEN, MAX_CLIPBOARD_TEXT_LEN, MAX_DEVICE_ID_LEN, MAX_DEVICE_NAME_CHARS, RATIO_SCALE,
    SPAKE_MESSAGE_LEN,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OsKind {
    MacOs,
    Windows,
    Other,
}

impl OsKind {
    pub fn current() -> Self {
        if cfg!(target_os = "macos") {
            OsKind::MacOs
        } else if cfg!(target_os = "windows") {
            OsKind::Windows
        } else {
            OsKind::Other
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            OsKind::MacOs => "macos",
            OsKind::Windows => "windows",
            OsKind::Other => "other",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

impl Side {
    pub fn opposite(self) -> Side {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
            Side::Top => Side::Bottom,
            Side::Bottom => Side::Top,
        }
    }

    pub fn is_horizontal(self) -> bool {
        matches!(self, Side::Left | Side::Right)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputEvent {
    Key { usage: u16, down: bool },
    MouseMove { dx: i32, dy: i32 },
    MouseButton { button: MouseButton, down: bool },
    Wheel { dx: i32, dy: i32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryPoint {
    Edge { side: Side, ratio: u16 },
    Center,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClipboardFormat {
    PlainText,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GoodbyeReason {
    Shutdown,
    Replaced,
    Unpaired,
    VersionMismatch,
    ProtocolViolation,
    PairingRejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RefuseReason {
    SharingDisabled,
    PermissionMissing,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hello {
    pub protocol_version: u16,
    pub device_id: String,
    pub device_name: String,
    pub os: OsKind,
    pub app_version: String,
    pub listen_port: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayoutClaim {
    pub receiver_side: Side,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairIdentity {
    pub protocol_version: u16,
    pub device_id: String,
    pub device_name: String,
    pub os: OsKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Message {
    Hello(Hello),
    Layout(LayoutClaim),
    Ping { nonce: u64 },
    Pong { nonce: u64 },
    Enter(EntryPoint),
    Leave { side: Side, ratio: u16 },
    Release,
    Input(InputEvent),
    ClipboardBegin { transfer_id: u64, format: ClipboardFormat, total_len: u32, sha256: [u8; 32] },
    ClipboardChunk { transfer_id: u64, offset: u32, data: Vec<u8> },
    Goodbye { reason: GoodbyeReason },
    PairStart { identity: PairIdentity, spake_message: Vec<u8> },
    PairReply { identity: PairIdentity, spake_message: Vec<u8> },
    PairConfirm { mac: [u8; 32] },
    PairResult { accepted: bool },
    Refuse { reason: RefuseReason },
}

impl Message {
    pub fn kind(&self) -> &'static str {
        match self {
            Message::Hello(_) => "hello",
            Message::Layout(_) => "layout",
            Message::Ping { .. } => "ping",
            Message::Pong { .. } => "pong",
            Message::Enter(_) => "enter",
            Message::Leave { .. } => "leave",
            Message::Release => "release",
            Message::Input(_) => "input",
            Message::ClipboardBegin { .. } => "clipboard_begin",
            Message::ClipboardChunk { .. } => "clipboard_chunk",
            Message::Goodbye { .. } => "goodbye",
            Message::PairStart { .. } => "pair_start",
            Message::PairReply { .. } => "pair_reply",
            Message::PairConfirm { .. } => "pair_confirm",
            Message::PairResult { .. } => "pair_result",
            Message::Refuse { .. } => "refuse",
        }
    }

    pub fn validate(&self) -> Result<(), ProtoError> {
        match self {
            Message::Hello(hello) => {
                validate_device_id(&hello.device_id)?;
                validate_device_name(&hello.device_name)?;
                if hello.app_version.len() > 32 {
                    return Err(ProtoError::Invalid("app_version"));
                }
                Ok(())
            }
            Message::Enter(EntryPoint::Edge { ratio, .. }) | Message::Leave { ratio, .. } => {
                if *ratio > RATIO_SCALE {
                    return Err(ProtoError::Invalid("ratio"));
                }
                Ok(())
            }
            Message::Input(InputEvent::MouseMove { dx, dy }) | Message::Input(InputEvent::Wheel { dx, dy }) => {
                const LIMIT: i32 = 1 << 20;
                if dx.abs() > LIMIT || dy.abs() > LIMIT {
                    return Err(ProtoError::Invalid("delta"));
                }
                Ok(())
            }
            Message::ClipboardBegin { total_len, .. } => {
                if *total_len as usize > MAX_CLIPBOARD_TEXT_LEN {
                    return Err(ProtoError::Invalid("clipboard_total_len"));
                }
                Ok(())
            }
            Message::ClipboardChunk { data, .. } => {
                if data.is_empty() || data.len() > CLIPBOARD_CHUNK_LEN {
                    return Err(ProtoError::Invalid("clipboard_chunk_len"));
                }
                Ok(())
            }
            Message::PairStart { identity, spake_message } | Message::PairReply { identity, spake_message } => {
                validate_device_id(&identity.device_id)?;
                validate_device_name(&identity.device_name)?;
                if spake_message.len() != SPAKE_MESSAGE_LEN {
                    return Err(ProtoError::Invalid("spake_message"));
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

pub fn validate_device_id(id: &str) -> Result<(), ProtoError> {
    if id.is_empty() || id.len() > MAX_DEVICE_ID_LEN || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(ProtoError::Invalid("device_id"));
    }
    Ok(())
}

pub fn validate_device_name(name: &str) -> Result<(), ProtoError> {
    let count = name.chars().count();
    if count == 0 || count > MAX_DEVICE_NAME_CHARS || name.chars().any(char::is_control) {
        return Err(ProtoError::Invalid("device_name"));
    }
    Ok(())
}

pub fn ratio_from_f64(value: f64) -> u16 {
    let clamped = if value.is_finite() { value.clamp(0.0, 1.0) } else { 0.5 };
    (clamped * f64::from(RATIO_SCALE)).round() as u16
}

pub fn ratio_to_f64(value: u16) -> f64 {
    f64::from(value.min(RATIO_SCALE)) / f64::from(RATIO_SCALE)
}
