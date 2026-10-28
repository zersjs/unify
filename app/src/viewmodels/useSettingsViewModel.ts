import { useCallback, useState } from "react";
import { bridge, type PermissionKind, type SettingsPatch, type StatusSnapshot } from "@/bridge";
import { useAction } from "./useAction";

export function useSettingsViewModel(status: StatusSnapshot) {
  const update = useAction((patch: SettingsPatch) => bridge.updateSettings(patch));
  const setManualAddress = useAction((address: string | null) => bridge.setManualAddress(address));
  const removePeer = useAction(() => bridge.removePeer());
  const resetIdentity = useAction(() => bridge.resetIdentity());
  const reconnect = useAction(() => bridge.reconnectNow());
  const resync = useAction(() => bridge.resyncClipboard());
  const requestPermissions = useAction(() => bridge.requestPermissions());
  const openPermissionSettings = useAction((kind: PermissionKind) => bridge.openPermissionSettings(kind));
  const openLogs = useAction(() => bridge.openLogFolder());
  const [diagnostics, setDiagnostics] = useState<string | null>(null);
  const loadDiagnostics = useAction(async () => setDiagnostics(await bridge.diagnostics()));

  const toggle = useCallback(
    (key: keyof Pick<SettingsPatch, "input_sharing" | "clipboard_sync" | "edge_switch" | "switch_hotkey" | "swap_command_control" | "invert_scroll" | "discovery" | "autostart">) =>
      (value: boolean) => update.run({ [key]: value }),
    [update],
  );

  const firstError =
    update.error ??
    setManualAddress.error ??
    removePeer.error ??
    resetIdentity.error ??
    reconnect.error ??
    resync.error ??
    requestPermissions.error ??
    openPermissionSettings.error ??
    openLogs.error ??
    loadDiagnostics.error;

  return {
    settings: status.settings,
    update,
    toggle,
    setManualAddress,
    removePeer,
    resetIdentity,
    reconnect,
    resync,
    requestPermissions,
    openPermissionSettings,
    openLogs,
    diagnostics,
    loadDiagnostics,
    error: firstError,
  };
}

export type SettingsViewModel = ReturnType<typeof useSettingsViewModel>;
