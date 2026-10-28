import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Side = "Left" | "Right" | "Top" | "Bottom";
export type MacKeyboardLayout = "auto" | "iso" | "ansi";
export type LogLevel = "error" | "warn" | "info" | "debug";
export type UiLanguage = "en" | "tr";
export type OsName = "macos" | "windows" | "other";
export type PermissionValue = "granted" | "denied" | "not_required" | "unknown";
export type ConnectionState = "not_paired" | "waiting" | "connecting" | "reconnecting" | "connected";
export type ControlState = "local" | "controlling" | "controlled";
export type CaptureState = "disabled" | "running" | "permission_required" | "failed";
export type ClipboardActivity = "idle" | "sending" | "receiving";
export type PermissionKind = "accessibility" | "input_monitoring" | "local_network";

export interface Settings {
  device_name: string;
  listen_port: number;
  peer_side: Side;
  layout_revision: number;
  input_sharing: boolean;
  clipboard_sync: boolean;
  edge_switch: boolean;
  edge_dwell_ms: number;
  corner_guard_px: number;
  switch_hotkey: boolean;
  swap_command_control: boolean;
  invert_scroll: boolean;
  mac_keyboard_layout: MacKeyboardLayout;
  discovery: boolean;
  autostart: boolean;
  log_level: LogLevel;
  language: UiLanguage;
}

export type SettingsPatch = Partial<Omit<Settings, "layout_revision">>;

export type PairingStatus =
  | { kind: "hosting"; code: string; expires_at_ms: number }
  | { kind: "joining"; address: string }
  | { kind: "awaiting_approval"; peer_name: string; peer_os: OsName; fingerprint: string }
  | { kind: "awaiting_peer_approval"; peer_name: string }
  | { kind: "succeeded"; peer_name: string }
  | { kind: "failed"; reason: string };

export interface DisplayRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface StatusSnapshot {
  device: {
    id: string;
    name: string;
    os: OsName;
    fingerprint: string;
    port: number;
    secretBackend: string;
    configPath: string;
  };
  peer: {
    id: string;
    name: string;
    os: OsName;
    fingerprint: string;
    pairedAt: number;
    manualAddress: string | null;
    lastAddress: string | null;
  } | null;
  connection: {
    state: ConnectionState;
    attempt: number;
    rttMs: number | null;
    sinceMs: number;
    remoteAddress: string | null;
    nextRetryMs: number | null;
  };
  control: ControlState;
  capture: CaptureState;
  clipboard: {
    activity: ClipboardActivity;
    lastTransferMs: number | null;
    lastDirection: "outgoing" | "incoming" | null;
    lastSkip: string | null;
  };
  permissions: {
    accessibility: PermissionValue;
    inputMonitoring: PermissionValue;
    localNetwork: PermissionValue;
  };
  pairing: PairingStatus | null;
  discovered: Array<{ id: string; name: string; os: OsName; address: string; isPairedPeer: boolean }>;
  lastError: { code: string; atMs: number } | null;
  stats: {
    sessionsEstablished: number;
    inputEventsSent: number;
    inputEventsReceived: number;
    clipboardSent: number;
    clipboardReceived: number;
    rejectedConnections: number;
  };
  settings: Settings;
  isoKeyboard: boolean;
  displays: DisplayRect[];
  appVersion: string;
}

export interface CommandFailure {
  code: string;
}

export function failureCode(error: unknown): string {
  if (typeof error === "string") {
    return error;
  }
  if (error && typeof error === "object" && "code" in error && typeof (error as CommandFailure).code === "string") {
    return (error as CommandFailure).code;
  }
  return "unknown";
}

export const bridge = {
  status: () => invoke<StatusSnapshot>("get_status"),
  onStatus: (handler: (status: StatusSnapshot) => void): Promise<UnlistenFn> =>
    listen<StatusSnapshot>("status", (event) => handler(event.payload)),
  updateSettings: (patch: SettingsPatch) => invoke<void>("update_settings", { patch }),
  startPairing: () => invoke<void>("start_pairing"),
  cancelPairing: () => invoke<void>("cancel_pairing"),
  joinPairing: (address: string, code: string) => invoke<void>("join_pairing", { address, code }),
  approvePairing: (accept: boolean) => invoke<void>("approve_pairing", { accept }),
  removePeer: () => invoke<void>("remove_peer"),
  setManualAddress: (address: string | null) => invoke<void>("set_manual_address", { address }),
  reconnectNow: () => invoke<void>("reconnect_now"),
  resyncClipboard: () => invoke<void>("resync_clipboard"),
  releaseControl: () => invoke<void>("release_control"),
  requestPermissions: () => invoke<void>("request_permissions"),
  openPermissionSettings: (kind: PermissionKind) => invoke<void>("open_permission_settings", { kind }),
  resetIdentity: () => invoke<void>("reset_identity"),
  clearError: () => invoke<void>("clear_error"),
  diagnostics: () => invoke<string>("diagnostics_report"),
  openLogFolder: () => invoke<void>("open_log_folder"),
};
