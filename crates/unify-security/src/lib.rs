pub mod identity;
pub mod keystore;
pub mod pairing;
pub mod tls;

pub use identity::{DeviceIdentity, Fingerprint, IdentityOutcome, IdentityStore};
pub use keystore::{MemorySecretStore, OsSecretStore, SecretStore};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SecurityError {
    #[error("certificate generation failed")]
    CertificateGeneration,
    #[error("stored identity is invalid")]
    InvalidIdentity,
    #[error("secure storage unavailable: {0}")]
    SecretStore(String),
    #[error("identity file io: {0}")]
    Io(#[from] std::io::Error),
    #[error("identity file encoding")]
    Encoding,
    #[error("tls configuration: {0}")]
    Tls(#[from] rustls::Error),
    #[error("pairing key exchange failed")]
    KeyExchange,
}
