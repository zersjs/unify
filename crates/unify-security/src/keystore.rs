use std::collections::HashMap;
use std::sync::Mutex;

use zeroize::Zeroizing;

use crate::SecurityError;

pub const KEYCHAIN_SERVICE: &str = "dev.unify.desktop";

pub trait SecretStore: Send + Sync {
    fn load(&self, account: &str) -> Result<Option<Zeroizing<Vec<u8>>>, SecurityError>;
    fn store(&self, account: &str, secret: &[u8]) -> Result<(), SecurityError>;
    fn delete(&self, account: &str) -> Result<(), SecurityError>;
    fn backend_name(&self) -> &'static str;
}

#[derive(Debug, Default)]
pub struct OsSecretStore;

impl OsSecretStore {
    fn entry(account: &str) -> Result<keyring::Entry, SecurityError> {
        keyring::Entry::new(KEYCHAIN_SERVICE, account).map_err(|err| SecurityError::SecretStore(err.to_string()))
    }
}

impl SecretStore for OsSecretStore {
    fn load(&self, account: &str) -> Result<Option<Zeroizing<Vec<u8>>>, SecurityError> {
        match Self::entry(account)?.get_secret() {
            Ok(secret) => Ok(Some(Zeroizing::new(secret))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(err) => Err(SecurityError::SecretStore(err.to_string())),
        }
    }

    fn store(&self, account: &str, secret: &[u8]) -> Result<(), SecurityError> {
        Self::entry(account)?.set_secret(secret).map_err(|err| SecurityError::SecretStore(err.to_string()))
    }

    fn delete(&self, account: &str) -> Result<(), SecurityError> {
        match Self::entry(account)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(err) => Err(SecurityError::SecretStore(err.to_string())),
        }
    }

    fn backend_name(&self) -> &'static str {
        if cfg!(target_os = "macos") {
            "macos-keychain"
        } else if cfg!(target_os = "windows") {
            "windows-credential-manager"
        } else {
            "platform-keyring"
        }
    }
}

#[derive(Debug, Default)]
pub struct MemorySecretStore {
    secrets: Mutex<HashMap<String, Zeroizing<Vec<u8>>>>,
}

impl SecretStore for MemorySecretStore {
    fn load(&self, account: &str) -> Result<Option<Zeroizing<Vec<u8>>>, SecurityError> {
        let guard = self.secrets.lock().map_err(|_| SecurityError::SecretStore("poisoned".into()))?;
        Ok(guard.get(account).cloned())
    }

    fn store(&self, account: &str, secret: &[u8]) -> Result<(), SecurityError> {
        let mut guard = self.secrets.lock().map_err(|_| SecurityError::SecretStore("poisoned".into()))?;
        guard.insert(account.to_owned(), Zeroizing::new(secret.to_vec()));
        Ok(())
    }

    fn delete(&self, account: &str) -> Result<(), SecurityError> {
        let mut guard = self.secrets.lock().map_err(|_| SecurityError::SecretStore("poisoned".into()))?;
        guard.remove(account);
        Ok(())
    }

    fn backend_name(&self) -> &'static str {
        "memory"
    }
}
