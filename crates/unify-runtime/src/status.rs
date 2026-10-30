use serde::Serialize;
use unify_core::config::Settings;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionState {
    NotPaired,
    Waiting,
    Connecting,
    Reconnecting,
    Connected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlState {
    Local,
    Controlling,
    Controlled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureState {
    Disabled,
    Running,
    PermissionRequired,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ClipboardActivity {
    Idle,
    Sending,
    Receiving,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferDirection {
    Outgoing,
    Incoming,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceStatus {
    pub id: String,
    pub name: String,
    pub os: &'static str,
    pub fingerprint: String,
    pub port: u16,
    pub secret_backend: &'static str,
    pub config_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerStatus {
    pub id: String,
    pub name: String,
    pub os: &'static str,
    pub fingerprint: String,
    pub paired_at: u64,
    pub manual_address: Option<String>,
    pub last_address: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionStatus {
    pub state: ConnectionState,
    pub attempt: u32,
    pub rtt_ms: Option<u32>,
    pub since_ms: u64,
    pub remote_address: Option<String>,
    pub next_retry_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardStatus {
    pub activity: ClipboardActivity,
    pub last_transfer_ms: Option<u64>,
    pub last_direction: Option<TransferDirection>,
    pub last_skip: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionsStatus {
    pub accessibility: &'static str,
    pub input_monitoring: &'static str,
    pub local_network: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum PairingStatus {
    Hosting { code: String, expires_at_ms: u64 },
    Joining { address: String },
    AwaitingApproval { peer_name: String, peer_os: &'static str, fingerprint: String },
    AwaitingPeerApproval { peer_name: String },
    Succeeded { peer_name: String },
    Failed { reason: &'static str },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredStatus {
    pub id: String,
    pub name: String,
    pub os: &'static str,
    pub address: String,
    pub is_paired_peer: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorStatus {
    pub code: &'static str,
    pub at_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub sessions_established: u64,
    pub input_events_sent: u64,
    pub input_events_received: u64,
    pub clipboard_sent: u64,
    pub clipboard_received: u64,
    pub rejected_connections: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamState {
    Idle,
    Starting,
    Viewing,
    Sharing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamStatus {
    pub state: StreamState,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub latency_p50_us: Option<u32>,
    pub latency_p99_us: Option<u32>,
    pub dropped_frames: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusSnapshot {
    pub device: DeviceStatus,
    pub peer: Option<PeerStatus>,
    pub connection: ConnectionStatus,
    pub control: ControlState,
    pub capture: CaptureState,
    pub stream: StreamStatus,
    pub clipboard: ClipboardStatus,
    pub permissions: PermissionsStatus,
    pub pairing: Option<PairingStatus>,
    pub discovered: Vec<DiscoveredStatus>,
    pub last_error: Option<ErrorStatus>,
    pub stats: Stats,
    pub settings: Settings,
    pub iso_keyboard: bool,
    pub displays: Vec<unify_core::geometry::Rect>,
    pub app_version: &'static str,
}
