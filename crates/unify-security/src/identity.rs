use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use rand::rngs::OsRng;
use rand::RngCore;
use rcgen::{CertificateParams, DnType, KeyPair, PKCS_ECDSA_P256_SHA256};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::sign::CertifiedKey;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::keystore::SecretStore;
use crate::tls::crypto_provider;
use crate::SecurityError;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Fingerprint(pub [u8; 32]);

impl Fingerprint {
    pub fn of(certificate_der: &[u8]) -> Self {
        Self(Sha256::digest(certificate_der).into())
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn from_hex(value: &str) -> Option<Self> {
        let bytes = hex::decode(value).ok()?;
        Some(Self(bytes.try_into().ok()?))
    }

    pub fn short(&self) -> String {
        let hex = hex::encode_upper(&self.0[..8]);
        hex.as_bytes().chunks(4).map(|chunk| String::from_utf8_lossy(chunk).into_owned()).collect::<Vec<_>>().join(" ")
    }
}

impl fmt::Debug for Fingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Fingerprint({})", self.short())
    }
}

pub struct DeviceIdentity {
    device_id: String,
    certificate: CertificateDer<'static>,
    private_key: Zeroizing<Vec<u8>>,
}

impl fmt::Debug for DeviceIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DeviceIdentity").field("device_id", &self.device_id).field("fingerprint", &self.fingerprint()).finish()
    }
}

impl DeviceIdentity {
    pub fn generate() -> Result<Self, SecurityError> {
        let mut id_bytes = [0u8; 16];
        OsRng.fill_bytes(&mut id_bytes);
        let device_id = hex::encode(id_bytes);
        let key_pair = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256).map_err(|_| SecurityError::CertificateGeneration)?;
        let mut params =
            CertificateParams::new(vec![format!("unify-{device_id}.local")]).map_err(|_| SecurityError::CertificateGeneration)?;
        params.distinguished_name.push(DnType::CommonName, format!("Unify {device_id}"));
        let certificate = params.self_signed(&key_pair).map_err(|_| SecurityError::CertificateGeneration)?;
        let identity = Self {
            device_id,
            certificate: certificate.der().clone(),
            private_key: Zeroizing::new(key_pair.serialize_der()),
        };
        identity.certified_key()?;
        Ok(identity)
    }

    pub fn from_parts(device_id: String, certificate: Vec<u8>, private_key: Zeroizing<Vec<u8>>) -> Result<Self, SecurityError> {
        unify_proto::validate_device_id(&device_id).map_err(|_| SecurityError::InvalidIdentity)?;
        let identity = Self { device_id, certificate: CertificateDer::from(certificate), private_key };
        identity.certified_key().map_err(|_| SecurityError::InvalidIdentity)?;
        Ok(identity)
    }

    pub fn device_id(&self) -> &str {
        &self.device_id
    }

    pub fn certificate(&self) -> &CertificateDer<'static> {
        &self.certificate
    }

    pub fn fingerprint(&self) -> Fingerprint {
        Fingerprint::of(&self.certificate)
    }

    pub fn private_key(&self) -> PrivateKeyDer<'static> {
        PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(self.private_key.to_vec()))
    }

    pub fn certified_key(&self) -> Result<Arc<CertifiedKey>, SecurityError> {
        let key = CertifiedKey::from_der(vec![self.certificate.clone()], self.private_key(), &crypto_provider())?;
        Ok(Arc::new(key))
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct IdentityFile {
    device_id: String,
    certificate: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityOutcome {
    Loaded,
    Created,
    Regenerated,
}

pub struct IdentityStore {
    path: PathBuf,
    secrets: Arc<dyn SecretStore>,
    account_prefix: String,
}

impl IdentityStore {
    pub fn new(config_dir: &Path, secrets: Arc<dyn SecretStore>, profile: &str) -> Self {
        Self { path: config_dir.join("identity.json"), secrets, account_prefix: format!("device-key-{profile}") }
    }

    pub fn secret_backend(&self) -> &'static str {
        self.secrets.backend_name()
    }

    fn account(&self, device_id: &str) -> String {
        format!("{}-{device_id}", self.account_prefix)
    }

    pub fn load_or_create(&self) -> Result<(DeviceIdentity, IdentityOutcome), SecurityError> {
        let file = match fs::read(&self.path) {
            Ok(raw) => serde_json::from_slice::<IdentityFile>(&raw).ok(),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
            Err(err) => return Err(err.into()),
        };
        let Some(file) = file else {
            let identity = self.create()?;
            return Ok((identity, IdentityOutcome::Created));
        };
        let certificate = BASE64.decode(file.certificate.as_bytes()).map_err(|_| SecurityError::Encoding);
        let key = self.secrets.load(&self.account(&file.device_id))?;
        match (certificate, key) {
            (Ok(certificate), Some(key)) => match DeviceIdentity::from_parts(file.device_id.clone(), certificate, key) {
                Ok(identity) => Ok((identity, IdentityOutcome::Loaded)),
                Err(_) => {
                    self.secrets.delete(&self.account(&file.device_id))?;
                    Ok((self.create()?, IdentityOutcome::Regenerated))
                }
            },
            _ => Ok((self.create()?, IdentityOutcome::Regenerated)),
        }
    }

    pub fn reset(&self, current: &DeviceIdentity) -> Result<DeviceIdentity, SecurityError> {
        self.secrets.delete(&self.account(current.device_id()))?;
        self.create()
    }

    pub fn erase(&self, current: &DeviceIdentity) -> Result<(), SecurityError> {
        self.secrets.delete(&self.account(current.device_id()))?;
        match fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err.into()),
        }
    }

    fn create(&self) -> Result<DeviceIdentity, SecurityError> {
        let identity = DeviceIdentity::generate()?;
        self.secrets.store(&self.account(identity.device_id()), &identity.private_key)?;
        let file = IdentityFile { device_id: identity.device_id.clone(), certificate: BASE64.encode(identity.certificate.as_ref()) };
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let encoded = serde_json::to_vec_pretty(&file).map_err(|_| SecurityError::Encoding)?;
        let temp = self.path.with_extension("json.tmp");
        {
            let mut handle = fs::File::create(&temp)?;
            handle.write_all(&encoded)?;
            handle.sync_all()?;
        }
        fs::rename(&temp, &self.path)?;
        Ok(identity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keystore::MemorySecretStore;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("unify-identity-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn generated_identity_has_matching_key_and_hex_id() {
        let identity = DeviceIdentity::generate().unwrap();
        assert_eq!(identity.device_id().len(), 32);
        assert!(identity.certified_key().is_ok());
        assert_eq!(identity.fingerprint().short().len(), 19);
    }

    #[test]
    fn identity_persists_across_loads_with_key_in_secret_store() {
        let dir = temp_dir("persist");
        let secrets: Arc<dyn SecretStore> = Arc::new(MemorySecretStore::default());
        let store = IdentityStore::new(&dir, secrets.clone(), "test");
        let (first, outcome) = store.load_or_create().unwrap();
        assert_eq!(outcome, IdentityOutcome::Created);
        let raw = fs::read_to_string(dir.join("identity.json")).unwrap();
        assert!(!raw.contains("PRIVATE"));
        let (second, outcome) = store.load_or_create().unwrap();
        assert_eq!(outcome, IdentityOutcome::Loaded);
        assert_eq!(first.fingerprint(), second.fingerprint());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn missing_secret_regenerates_identity() {
        let dir = temp_dir("regen");
        let secrets: Arc<dyn SecretStore> = Arc::new(MemorySecretStore::default());
        let store = IdentityStore::new(&dir, secrets.clone(), "test");
        let (first, _) = store.load_or_create().unwrap();
        secrets.delete(&store.account(first.device_id())).unwrap();
        let (second, outcome) = store.load_or_create().unwrap();
        assert_eq!(outcome, IdentityOutcome::Regenerated);
        assert_ne!(first.fingerprint(), second.fingerprint());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn mismatched_key_is_rejected() {
        let a = DeviceIdentity::generate().unwrap();
        let b = DeviceIdentity::generate().unwrap();
        let result = DeviceIdentity::from_parts(a.device_id().to_owned(), a.certificate().to_vec(), b.private_key.clone());
        assert!(matches!(result, Err(SecurityError::InvalidIdentity)));
    }

    #[test]
    fn reset_produces_new_identity_and_deletes_old_key() {
        let dir = temp_dir("reset");
        let secrets: Arc<dyn SecretStore> = Arc::new(MemorySecretStore::default());
        let store = IdentityStore::new(&dir, secrets.clone(), "test");
        let (first, _) = store.load_or_create().unwrap();
        let second = store.reset(&first).unwrap();
        assert_ne!(first.device_id(), second.device_id());
        assert!(secrets.load(&store.account(first.device_id())).unwrap().is_none());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn fingerprint_hex_roundtrip() {
        let fp = Fingerprint::of(b"abc");
        assert_eq!(Fingerprint::from_hex(&fp.to_hex()), Some(fp));
        assert_eq!(Fingerprint::from_hex("zz"), None);
    }
}
