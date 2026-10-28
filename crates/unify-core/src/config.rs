use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use unify_proto::{OsKind, Side, MAX_DEVICE_NAME_CHARS};

pub const CONFIG_SCHEMA_VERSION: u32 = 1;
pub const DEFAULT_PORT: u16 = 47_823;
pub const MAX_DWELL_MS: u32 = 1_000;
pub const MAX_CORNER_GUARD_PX: u32 = 200;
const MAX_ADDRESS_LEN: usize = 260;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MacKeyboardLayout {
    Auto,
    Iso,
    Ansi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UiLanguage {
    En,
    Tr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
}

impl LogLevel {
    pub fn as_filter(self) -> &'static str {
        match self {
            LogLevel::Error => "error",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub device_name: String,
    pub listen_port: u16,
    pub peer_side: Side,
    pub layout_revision: u64,
    pub input_sharing: bool,
    pub clipboard_sync: bool,
    pub edge_switch: bool,
    pub edge_dwell_ms: u32,
    pub corner_guard_px: u32,
    pub switch_hotkey: bool,
    pub swap_command_control: bool,
    pub invert_scroll: bool,
    pub mac_keyboard_layout: MacKeyboardLayout,
    pub discovery: bool,
    pub autostart: bool,
    pub log_level: LogLevel,
    pub language: UiLanguage,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            device_name: String::from("Unify"),
            listen_port: DEFAULT_PORT,
            peer_side: Side::Right,
            layout_revision: 0,
            input_sharing: true,
            clipboard_sync: true,
            edge_switch: true,
            edge_dwell_ms: 0,
            corner_guard_px: 0,
            switch_hotkey: true,
            swap_command_control: true,
            invert_scroll: false,
            mac_keyboard_layout: MacKeyboardLayout::Auto,
            discovery: true,
            autostart: false,
            log_level: LogLevel::Info,
            language: UiLanguage::En,
        }
    }
}

pub fn sanitize_device_name(name: &str) -> String {
    let cleaned: String = name.chars().filter(|c| !c.is_control()).collect::<String>().trim().chars().take(MAX_DEVICE_NAME_CHARS).collect();
    let trimmed = cleaned.trim().to_owned();
    if trimmed.is_empty() {
        String::from("Unify")
    } else {
        trimmed
    }
}

impl Settings {
    pub fn normalized(mut self) -> Self {
        self.device_name = sanitize_device_name(&self.device_name);
        if self.listen_port < 1024 {
            self.listen_port = DEFAULT_PORT;
        }
        self.edge_dwell_ms = self.edge_dwell_ms.min(MAX_DWELL_MS);
        self.corner_guard_px = self.corner_guard_px.min(MAX_CORNER_GUARD_PX);
        self
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SettingsPatch {
    pub device_name: Option<String>,
    pub listen_port: Option<u16>,
    pub peer_side: Option<Side>,
    pub input_sharing: Option<bool>,
    pub clipboard_sync: Option<bool>,
    pub edge_switch: Option<bool>,
    pub edge_dwell_ms: Option<u32>,
    pub corner_guard_px: Option<u32>,
    pub switch_hotkey: Option<bool>,
    pub swap_command_control: Option<bool>,
    pub invert_scroll: Option<bool>,
    pub mac_keyboard_layout: Option<MacKeyboardLayout>,
    pub discovery: Option<bool>,
    pub autostart: Option<bool>,
    pub log_level: Option<LogLevel>,
    pub language: Option<UiLanguage>,
}

impl SettingsPatch {
    pub fn apply(&self, settings: &Settings, now_ms: u64) -> Settings {
        let mut next = settings.clone();
        if let Some(value) = &self.device_name {
            next.device_name = value.clone();
        }
        if let Some(value) = self.listen_port {
            next.listen_port = value;
        }
        if let Some(value) = self.peer_side {
            if value != next.peer_side {
                next.peer_side = value;
                next.layout_revision = now_ms.max(next.layout_revision + 1);
            }
        }
        macro_rules! copy {
            ($($field:ident),*) => {
                $(if let Some(value) = self.$field { next.$field = value; })*
            };
        }
        copy!(
            input_sharing,
            clipboard_sync,
            edge_switch,
            edge_dwell_ms,
            corner_guard_px,
            switch_hotkey,
            swap_command_control,
            invert_scroll,
            mac_keyboard_layout,
            discovery,
            autostart,
            log_level,
            language
        );
        next.normalized()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairedPeer {
    pub device_id: String,
    pub device_name: String,
    pub os: OsKind,
    pub certificate: String,
    pub fingerprint: String,
    pub paired_at: u64,
    #[serde(default)]
    pub manual_address: Option<String>,
    #[serde(default)]
    pub last_address: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigFile {
    pub schema_version: u32,
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub peer: Option<PairedPeer>,
}

impl ConfigFile {
    pub fn with_device_name(name: &str) -> Self {
        Self {
            schema_version: CONFIG_SCHEMA_VERSION,
            settings: Settings { device_name: sanitize_device_name(name), ..Settings::default() },
            peer: None,
        }
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("config io: {0}")]
    Io(#[from] std::io::Error),
    #[error("config encode: {0}")]
    Encode(#[from] serde_json::Error),
}

#[derive(Debug, PartialEq, Eq)]
pub enum LoadOutcome {
    Loaded,
    Created,
    RecoveredFromCorruption { backup: PathBuf },
}

#[derive(Debug, Clone)]
pub struct ConfigStore {
    path: PathBuf,
}

impl ConfigStore {
    pub fn new(dir: &Path) -> Self {
        Self { path: dir.join("config.json") }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load_or_create(&self, default_name: &str) -> Result<(ConfigFile, LoadOutcome), ConfigError> {
        let raw = match fs::read(&self.path) {
            Ok(raw) => raw,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                let created = ConfigFile::with_device_name(default_name);
                self.save(&created)?;
                return Ok((created, LoadOutcome::Created));
            }
            Err(err) => return Err(err.into()),
        };
        match serde_json::from_slice::<ConfigFile>(&raw) {
            Ok(mut file) => {
                file.settings = file.settings.normalized();
                file.schema_version = CONFIG_SCHEMA_VERSION;
                if file.peer.as_ref().is_some_and(|peer| unify_proto::validate_device_id(&peer.device_id).is_err()) {
                    file.peer = None;
                }
                Ok((file, LoadOutcome::Loaded))
            }
            Err(_) => {
                let backup = self.path.with_extension("json.corrupt");
                fs::rename(&self.path, &backup)?;
                let created = ConfigFile::with_device_name(default_name);
                self.save(&created)?;
                Ok((created, LoadOutcome::RecoveredFromCorruption { backup }))
            }
        }
    }

    pub fn save(&self, file: &ConfigFile) -> Result<(), ConfigError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let encoded = serde_json::to_vec_pretty(file)?;
        let temp = self.path.with_extension("json.tmp");
        {
            let mut handle = fs::File::create(&temp)?;
            handle.write_all(&encoded)?;
            handle.sync_all()?;
        }
        fs::rename(&temp, &self.path)?;
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AddressError {
    #[error("address is empty or too long")]
    Length,
    #[error("address must be host:port")]
    MissingPort,
    #[error("invalid port")]
    Port,
    #[error("invalid host")]
    Host,
}

pub fn normalize_address(input: &str, default_port: u16) -> Result<String, AddressError> {
    let trimmed = input.trim();
    if trimmed.is_empty() || trimmed.len() > MAX_ADDRESS_LEN {
        return Err(AddressError::Length);
    }
    if let Ok(addr) = trimmed.parse::<std::net::SocketAddr>() {
        if addr.port() == 0 {
            return Err(AddressError::Port);
        }
        return Ok(addr.to_string());
    }
    if let Ok(ip) = trimmed.parse::<std::net::IpAddr>() {
        return Ok(std::net::SocketAddr::new(ip, default_port).to_string());
    }
    let (host, port) = match trimmed.rsplit_once(':') {
        Some((host, port)) => (host, port.parse::<u16>().map_err(|_| AddressError::Port)?),
        None => (trimmed, default_port),
    };
    if port == 0 {
        return Err(AddressError::Port);
    }
    let valid_host = !host.is_empty()
        && host.len() <= 253
        && host.split('.').all(|label| {
            !label.is_empty() && label.len() <= 63 && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        });
    if !valid_host {
        return Err(AddressError::Host);
    }
    Ok(format!("{host}:{port}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("unify-config-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn creates_default_config_when_missing_and_reloads_it() {
        let dir = temp_dir("create");
        let store = ConfigStore::new(&dir);
        let (created, outcome) = store.load_or_create("Ofis PC").unwrap();
        assert_eq!(outcome, LoadOutcome::Created);
        assert_eq!(created.settings.device_name, "Ofis PC");
        assert!(!created.settings.autostart);
        let (loaded, outcome) = store.load_or_create("ignored").unwrap();
        assert_eq!(outcome, LoadOutcome::Loaded);
        assert_eq!(loaded, created);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn recovers_from_corrupt_file_with_backup() {
        let dir = temp_dir("corrupt");
        let store = ConfigStore::new(&dir);
        fs::write(store.path(), b"{not json").unwrap();
        let (_, outcome) = store.load_or_create("X").unwrap();
        assert!(matches!(outcome, LoadOutcome::RecoveredFromCorruption { .. }));
        assert!(dir.join("config.json.corrupt").exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn fills_missing_fields_with_defaults_and_clamps_values() {
        let dir = temp_dir("partial");
        let store = ConfigStore::new(&dir);
        fs::write(store.path(), br#"{"schema_version":1,"settings":{"edge_dwell_ms":99999,"listen_port":80}}"#).unwrap();
        let (loaded, _) = store.load_or_create("X").unwrap();
        assert_eq!(loaded.settings.edge_dwell_ms, MAX_DWELL_MS);
        assert_eq!(loaded.settings.listen_port, DEFAULT_PORT);
        assert!(loaded.settings.clipboard_sync);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn patch_updates_fields_and_bumps_layout_revision() {
        let base = Settings::default();
        let patch = SettingsPatch { peer_side: Some(Side::Left), clipboard_sync: Some(false), ..Default::default() };
        let next = patch.apply(&base, 1_000);
        assert_eq!(next.peer_side, Side::Left);
        assert!(!next.clipboard_sync);
        assert_eq!(next.layout_revision, 1_000);
        let same = SettingsPatch { peer_side: Some(Side::Left), ..Default::default() }.apply(&next, 2_000);
        assert_eq!(same.layout_revision, 1_000);
    }

    #[test]
    fn language_defaults_to_english_and_accepts_turkish() {
        assert_eq!(Settings::default().language, UiLanguage::En);
        let patch: SettingsPatch = serde_json::from_str(r#"{"language":"tr"}"#).unwrap();
        assert_eq!(patch.apply(&Settings::default(), 0).language, UiLanguage::Tr);
        let legacy: Settings = serde_json::from_str(r#"{"device_name":"X"}"#).unwrap();
        assert_eq!(legacy.language, UiLanguage::En);
    }

    #[test]
    fn patch_rejects_unknown_fields() {
        assert!(serde_json::from_str::<SettingsPatch>(r#"{"bogus":true}"#).is_err());
    }

    #[test]
    fn sanitizes_device_names() {
        assert_eq!(sanitize_device_name("  Ofis\u{0000}PC  "), "OfisPC");
        assert_eq!(sanitize_device_name("   "), "Unify");
        assert_eq!(sanitize_device_name(&"ğ".repeat(100)).chars().count(), MAX_DEVICE_NAME_CHARS);
    }

    #[test]
    fn normalizes_addresses() {
        assert_eq!(normalize_address("192.168.1.20", 47823).unwrap(), "192.168.1.20:47823");
        assert_eq!(normalize_address(" 192.168.1.20:5000 ", 1).unwrap(), "192.168.1.20:5000");
        assert_eq!(normalize_address("fe80::1", 47823).unwrap(), "[fe80::1]:47823");
        assert_eq!(normalize_address("macbook.local", 47823).unwrap(), "macbook.local:47823");
        assert_eq!(normalize_address("macbook.local:0", 1), Err(AddressError::Port));
        assert_eq!(normalize_address("bad host:1", 1), Err(AddressError::Host));
        assert_eq!(normalize_address("", 1), Err(AddressError::Length));
        assert_eq!(normalize_address("a:b", 1), Err(AddressError::Port));
    }
}
