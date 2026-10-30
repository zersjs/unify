mod actor;
mod clipboard_flow;
mod control;
mod pairing_flow;
mod session;
pub mod status;

use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Arc;

use thiserror::Error;
use tokio::sync::{mpsc, oneshot, watch};
use unify_core::config::SettingsPatch;
use unify_platform::{PermissionKind, Platform};
use unify_security::SecretStore;

pub use status::*;

pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct RuntimeOptions {
    pub config_dir: PathBuf,
    pub profile: String,
    pub platform: Arc<dyn Platform>,
    pub secrets: Arc<dyn SecretStore>,
    pub bind_ip: IpAddr,
    pub listen_port_override: Option<u16>,
    pub discovery_enabled: bool,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("{code}")]
pub struct RuntimeError {
    pub code: &'static str,
}

impl RuntimeError {
    pub const fn new(code: &'static str) -> Self {
        Self { code }
    }
}

pub type Reply = oneshot::Sender<Result<(), RuntimeError>>;

pub(crate) enum Command {
    UpdateSettings(SettingsPatch, Reply),
    StartPairingHost(Reply),
    CancelPairing,
    JoinPairing { address: String, code: String, reply: Reply },
    ApprovePairing { accept: bool },
    RemovePeer(Reply),
    SetManualAddress(Option<String>, Reply),
    ReconnectNow,
    ResyncClipboard,
    ReleaseControl,
    RequestPermissions,
    ResetPermissions,
    OpenPermissionSettings(PermissionKind),
    ResetIdentity(Reply),
    ClearError,
    Shutdown(oneshot::Sender<()>),
}

#[derive(Clone)]
pub struct RuntimeHandle {
    commands: mpsc::UnboundedSender<Command>,
    status: watch::Receiver<StatusSnapshot>,
}

impl RuntimeHandle {
    pub fn status(&self) -> StatusSnapshot {
        self.status.borrow().clone()
    }

    pub fn subscribe(&self) -> watch::Receiver<StatusSnapshot> {
        self.status.clone()
    }

    fn send(&self, command: Command) -> Result<(), RuntimeError> {
        self.commands.send(command).map_err(|_| RuntimeError::new("runtime_stopped"))
    }

    async fn request(&self, build: impl FnOnce(Reply) -> Command) -> Result<(), RuntimeError> {
        let (tx, rx) = oneshot::channel();
        self.send(build(tx))?;
        rx.await.map_err(|_| RuntimeError::new("runtime_stopped"))?
    }

    pub async fn update_settings(&self, patch: SettingsPatch) -> Result<(), RuntimeError> {
        self.request(|reply| Command::UpdateSettings(patch, reply)).await
    }

    pub async fn start_pairing_host(&self) -> Result<(), RuntimeError> {
        self.request(Command::StartPairingHost).await
    }

    pub fn cancel_pairing(&self) -> Result<(), RuntimeError> {
        self.send(Command::CancelPairing)
    }

    pub async fn join_pairing(&self, address: String, code: String) -> Result<(), RuntimeError> {
        self.request(|reply| Command::JoinPairing { address, code, reply }).await
    }

    pub fn approve_pairing(&self, accept: bool) -> Result<(), RuntimeError> {
        self.send(Command::ApprovePairing { accept })
    }

    pub async fn remove_peer(&self) -> Result<(), RuntimeError> {
        self.request(Command::RemovePeer).await
    }

    pub async fn set_manual_address(&self, address: Option<String>) -> Result<(), RuntimeError> {
        self.request(|reply| Command::SetManualAddress(address, reply)).await
    }

    pub fn reconnect_now(&self) -> Result<(), RuntimeError> {
        self.send(Command::ReconnectNow)
    }

    pub fn resync_clipboard(&self) -> Result<(), RuntimeError> {
        self.send(Command::ResyncClipboard)
    }

    pub fn release_control(&self) -> Result<(), RuntimeError> {
        self.send(Command::ReleaseControl)
    }

    pub fn request_permissions(&self) -> Result<(), RuntimeError> {
        self.send(Command::RequestPermissions)
    }

    pub fn reset_permissions(&self) -> Result<(), RuntimeError> {
        self.send(Command::ResetPermissions)
    }

    pub fn open_permission_settings(&self, kind: PermissionKind) -> Result<(), RuntimeError> {
        self.send(Command::OpenPermissionSettings(kind))
    }

    pub async fn reset_identity(&self) -> Result<(), RuntimeError> {
        self.request(Command::ResetIdentity).await
    }

    pub fn clear_error(&self) -> Result<(), RuntimeError> {
        self.send(Command::ClearError)
    }

    pub async fn shutdown(&self) {
        let (tx, rx) = oneshot::channel();
        if self.commands.send(Command::Shutdown(tx)).is_ok() {
            let _ = rx.await;
        }
    }
}

pub fn start(options: RuntimeOptions) -> Result<RuntimeHandle, RuntimeError> {
    actor::Actor::spawn(options)
}
