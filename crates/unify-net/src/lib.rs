pub mod discovery;
pub mod transport;

pub use discovery::{Advertisement, DiscoveredDevice, Discovery, DiscoveryEvent, SERVICE_TYPE};
pub use transport::{
    accept_tls, connect_tls, read_message, spawn_writer, Outbound, PeerInfo, ReadError, SendError, SessionStream,
    HIGH_PRIORITY_CAPACITY, LOW_PRIORITY_CAPACITY,
};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum NetError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("timed out")]
    Timeout,
    #[error("address could not be resolved")]
    Resolve,
    #[error("tls: {0}")]
    Tls(String),
    #[error("peer presented no usable certificate")]
    NoPeerCertificate,
    #[error("discovery: {0}")]
    Discovery(String),
}
