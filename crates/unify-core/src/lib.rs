pub mod backoff;
pub mod capture;
pub mod clipboard;
pub mod config;
pub mod geometry;
pub mod keys;
pub mod switcher;

pub const PING_INTERVAL_MS: u64 = 1_000;
pub const PEER_TIMEOUT_MS: u64 = 5_000;
pub const SLEEP_GAP_MS: u64 = 6_000;
pub const CONNECT_TIMEOUT_MS: u64 = 3_000;
pub const HANDSHAKE_TIMEOUT_MS: u64 = 5_000;
