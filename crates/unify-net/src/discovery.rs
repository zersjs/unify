use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use tokio::sync::mpsc;
use unify_proto::{validate_device_id, validate_device_name, OsKind};

use crate::NetError;

pub const SERVICE_TYPE: &str = "_unify._tcp.local.";
const TXT_NAME_MAX_BYTES: usize = 200;

#[derive(Debug, Clone)]
pub struct Advertisement {
    pub device_id: String,
    pub device_name: String,
    pub os: OsKind,
    pub port: u16,
    pub fingerprint_prefix: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredDevice {
    pub device_id: String,
    pub device_name: String,
    pub os: OsKind,
    pub addresses: Vec<SocketAddr>,
    pub fingerprint_prefix: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryEvent {
    Found(DiscoveredDevice),
    Lost { device_id: String },
}

fn truncate_utf8(value: &str, max_bytes: usize) -> &str {
    if value.len() <= max_bytes {
        return value;
    }
    let mut end = max_bytes;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    &value[..end]
}

fn os_from_txt(value: Option<&str>) -> OsKind {
    match value {
        Some("macos") => OsKind::MacOs,
        Some("windows") => OsKind::Windows,
        _ => OsKind::Other,
    }
}

fn parse_resolved(info: &ServiceInfo, own_id: &str) -> Option<DiscoveredDevice> {
    let device_id = info.get_property_val_str("id")?.to_owned();
    if device_id == own_id || validate_device_id(&device_id).is_err() {
        return None;
    }
    let device_name = info.get_property_val_str("name").unwrap_or("Unify").to_owned();
    if validate_device_name(&device_name).is_err() {
        return None;
    }
    let fingerprint_prefix = info.get_property_val_str("fp").unwrap_or_default().to_owned();
    if fingerprint_prefix.len() > 64 || !fingerprint_prefix.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let port = info.get_port();
    if port == 0 {
        return None;
    }
    let mut addresses: Vec<SocketAddr> = info
        .get_addresses()
        .iter()
        .filter(|ip| !ip.is_loopback() && !ip.is_multicast() && !ip.is_unspecified())
        .map(|ip| SocketAddr::new(*ip, port))
        .collect();
    addresses.sort_by_key(|addr| (addr.is_ipv6(), *addr));
    if addresses.is_empty() {
        return None;
    }
    Some(DiscoveredDevice { device_id, device_name, os: os_from_txt(info.get_property_val_str("os")), addresses, fingerprint_prefix })
}

fn instance_from_fullname(fullname: &str) -> Option<String> {
    let instance = fullname.strip_suffix(SERVICE_TYPE)?.strip_suffix('.')?;
    Some(instance.to_owned())
}

pub struct Discovery {
    daemon: ServiceDaemon,
    stopping: Arc<AtomicBool>,
}

impl Discovery {
    pub fn start(advertisement: Advertisement, events: mpsc::UnboundedSender<DiscoveryEvent>) -> Result<Self, NetError> {
        let daemon = ServiceDaemon::new().map_err(|err| NetError::Discovery(err.to_string()))?;
        let short_id: String = advertisement.device_id.chars().take(12).collect();
        let host_name = format!("unify-{short_id}.local.");
        let mut properties = HashMap::new();
        properties.insert("id".to_owned(), advertisement.device_id.clone());
        properties.insert("name".to_owned(), truncate_utf8(&advertisement.device_name, TXT_NAME_MAX_BYTES).to_owned());
        properties.insert("os".to_owned(), advertisement.os.as_str().to_owned());
        properties.insert("fp".to_owned(), advertisement.fingerprint_prefix.clone());
        properties.insert("v".to_owned(), unify_proto::PROTOCOL_VERSION.to_string());
        let info = ServiceInfo::new(SERVICE_TYPE, &advertisement.device_id, &host_name, (), advertisement.port, properties)
            .map_err(|err| NetError::Discovery(err.to_string()))?
            .enable_addr_auto();
        daemon.register(info).map_err(|err| NetError::Discovery(err.to_string()))?;
        let receiver = daemon.browse(SERVICE_TYPE).map_err(|err| NetError::Discovery(err.to_string()))?;
        let stopping = Arc::new(AtomicBool::new(false));
        let thread_stopping = stopping.clone();
        let own_id = advertisement.device_id;
        std::thread::Builder::new()
            .name("unify-discovery".into())
            .spawn(move || {
                let mut known: HashMap<String, String> = HashMap::new();
                while !thread_stopping.load(Ordering::Relaxed) {
                    let event = match receiver.recv_timeout(Duration::from_millis(500)) {
                        Ok(event) => event,
                        Err(flume::RecvTimeoutError::Timeout) => continue,
                        Err(flume::RecvTimeoutError::Disconnected) => break,
                    };
                    let outgoing = match event {
                        ServiceEvent::ServiceResolved(info) => parse_resolved(&info, &own_id).map(|device| {
                            known.insert(info.get_fullname().to_owned(), device.device_id.clone());
                            DiscoveryEvent::Found(device)
                        }),
                        ServiceEvent::ServiceRemoved(_, fullname) => known
                            .remove(&fullname)
                            .or_else(|| instance_from_fullname(&fullname))
                            .filter(|id| *id != own_id)
                            .map(|device_id| DiscoveryEvent::Lost { device_id }),
                        _ => None,
                    };
                    if let Some(outgoing) = outgoing {
                        if events.send(outgoing).is_err() {
                            break;
                        }
                    }
                }
            })
            .map_err(NetError::Io)?;
        Ok(Self { daemon, stopping })
    }

    pub fn stop(self) {
        self.stopping.store(true, Ordering::Relaxed);
        if let Err(err) = self.daemon.shutdown() {
            tracing::debug!(error = %err, "mdns daemon shutdown reported an error");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncates_on_char_boundary() {
        let name = "ğ".repeat(150);
        let truncated = truncate_utf8(&name, TXT_NAME_MAX_BYTES);
        assert!(truncated.len() <= TXT_NAME_MAX_BYTES);
        assert!(name.starts_with(truncated));
    }

    #[test]
    fn extracts_instance_from_fullname() {
        assert_eq!(instance_from_fullname("abcd._unify._tcp.local."), Some("abcd".into()));
        assert_eq!(instance_from_fullname("abcd._other._tcp.local."), None);
    }

    #[test]
    fn resolves_peer_and_ignores_self() {
        let mut props = HashMap::new();
        props.insert("id".to_owned(), "beef".to_owned());
        props.insert("name".to_owned(), "Resul MacBook".to_owned());
        props.insert("os".to_owned(), "macos".to_owned());
        props.insert("fp".to_owned(), "a1b2".to_owned());
        let info = ServiceInfo::new(SERVICE_TYPE, "beef", "unify-beef.local.", "192.168.1.30", 47823, props).unwrap();
        let device = parse_resolved(&info, "cafe").unwrap();
        assert_eq!(device.os, OsKind::MacOs);
        assert_eq!(device.addresses, vec!["192.168.1.30:47823".parse().unwrap()]);
        assert!(parse_resolved(&info, "beef").is_none());
    }

    #[test]
    fn rejects_malformed_txt_records() {
        let mut props = HashMap::new();
        props.insert("id".to_owned(), "not-hex!".to_owned());
        let info = ServiceInfo::new(SERVICE_TYPE, "x", "x.local.", "192.168.1.30", 1, props).unwrap();
        assert!(parse_resolved(&info, "cafe").is_none());
    }
}
