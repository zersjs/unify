import { useCallback, useState } from "react";
import type { CrateId } from "@/i18n/en";

export const CRATE_IDS: readonly CrateId[] = [
  "unify-proto",
  "unify-core",
  "unify-security",
  "unify-net",
  "unify-platform",
  "unify-runtime",
  "unify-cli",
];

const CRATE_EXCERPTS: Record<CrateId, string> = {
  "unify-proto": `#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Message {
    Hello(Hello),
    Layout(LayoutClaim),
    Ping { nonce: u64 },
    Pong { nonce: u64 },
    Enter(EntryPoint),
    Leave { side: Side, ratio: u16 },
    Release,
    Input(InputEvent),
    ClipboardBegin {
        transfer_id: u64,
        format: ClipboardFormat,
        total_len: u32,
        sha256: [u8; 32],
    },
    ClipboardChunk { transfer_id: u64, offset: u32, data: Vec<u8> },
    Goodbye { reason: GoodbyeReason },
}`,
  "unify-core": `pub fn on_motion(
    &mut self,
    desktop: &Desktop,
    x: i32,
    y: i32,
    now_ms: u64,
    can_switch: bool,
) -> EdgeDecision {
    if !self.config.enabled {
        return EdgeDecision::Stay;
    }
    let side = self.config.side;
    let at_edge = desktop.at_edge(x, y, side)
        && !desktop.in_corner(x, y, side, self.config.corner_guard_px);
    if !at_edge {
        self.armed = true;
        self.edge_since = None;
        return EdgeDecision::Stay;
    }
    if !self.armed || now_ms < self.cooldown_until || !can_switch {
        self.edge_since = None;
        return EdgeDecision::Stay;
    }
    let since = *self.edge_since.get_or_insert(now_ms);
    if now_ms.saturating_sub(since) < self.config.dwell_ms {
        return EdgeDecision::Stay;
    }
    self.edge_since = None;
    self.armed = false;
    EdgeDecision::Switch {
        ratio: ratio_from_f64(desktop.ratio_along(x, y, side)),
    }
}`,
  "unify-security": `impl PairingExchange {
    pub fn start(code: &str) -> (Self, Vec<u8>) {
        let (state, message) = Spake2::<Ed25519Group>::start_symmetric(
            &Password::new(code.as_bytes()),
            &Identity::new(SPAKE_IDENTITY),
        );
        (Self { state }, message)
    }

    pub fn finish(self, peer_message: &[u8]) -> Result<PairingKey, SecurityError> {
        let key = self.state
            .finish(peer_message)
            .map_err(|_| SecurityError::KeyExchange)?;
        Ok(PairingKey { key: Zeroizing::new(key) })
    }
}`,
  "unify-net": `pub const HIGH_PRIORITY_CAPACITY: usize = 8_192;
pub const LOW_PRIORITY_CAPACITY: usize = 64;

#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub fingerprint: Fingerprint,
    pub certificate: Vec<u8>,
    pub remote_addr: SocketAddr,
    pub channel_binding: [u8; 32],
}

fn peer_info(
    state: &rustls::CommonState,
    exporter: [u8; 32],
    remote_addr: SocketAddr,
) -> Result<PeerInfo, NetError> {
    let (fingerprint, certificate) =
        peer_fingerprint(state).ok_or(NetError::NoPeerCertificate)?;
    Ok(PeerInfo { fingerprint, certificate, remote_addr, channel_binding: exporter })
}`,
  "unify-platform": `#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionKind {
    Accessibility,
    InputMonitoring,
    LocalNetwork,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PermissionReport {
    pub accessibility: PermissionState,
    pub input_monitoring: PermissionState,
    pub local_network: PermissionState,
}

impl PermissionReport {
    pub fn all_granted(&self) -> bool {
        [self.accessibility, self.input_monitoring]
            .iter()
            .all(|s| matches!(s, PermissionState::Granted | PermissionState::NotRequired))
    }
}`,
  "unify-runtime": `impl Actor {
    fn can_control(&self) -> bool {
        self.config.settings.input_sharing
            && self.active.is_some()
            && self.control == Control::Local
    }

    pub fn sync_capture(&mut self) {
        if !self.config.settings.input_sharing {
            if let Some(capture) = self.capture.take() {
                capture.stop();
            }
            self.capture_shared.leave_remote();
            self.capture_state = CaptureState::Disabled;
            self.dirty = true;
        }
    }
}`,
  "unify-cli": `#[derive(Parser, Debug)]
#[command(
    name = "unify-cli",
    version,
    about = "Unify technical prototype: secure keyboard, mouse and clipboard sharing"
)]
struct Args {
    #[arg(long, default_value = "default")]
    profile: String,
    #[arg(long)]
    config_dir: Option<PathBuf>,
    #[arg(long)]
    port: Option<u16>,
    #[arg(long)]
    no_discovery: bool,
    #[arg(long, default_value = "info")]
    log: String,
}`,
};

export interface CrateExplorerViewModel {
  readonly crates: readonly CrateId[];
  readonly selectedCrate: CrateId;
  readonly excerpt: string;
  readonly selectCrate: (crateId: CrateId) => void;
}

export function useCrateExplorerViewModel(): CrateExplorerViewModel {
  const [selectedCrate, setSelectedCrate] = useState<CrateId>("unify-core");

  const selectCrate = useCallback((crateId: CrateId) => {
    setSelectedCrate(crateId);
  }, []);

  return {
    crates: CRATE_IDS,
    selectedCrate,
    excerpt: CRATE_EXCERPTS[selectedCrate],
    selectCrate,
  };
}
