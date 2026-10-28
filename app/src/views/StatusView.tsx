import { bridge, type StatusSnapshot } from "@/bridge";
import { Banner, Button, Group, InlineError, Row, StatusDot, Toggle, type DotTone } from "@/components/controls";
import { LayoutDiagram } from "@/components/LayoutDiagram";
import { useI18n, type I18n } from "@/i18n";
import { useAction } from "@/viewmodels/useAction";

const tones: Record<StatusSnapshot["connection"]["state"], DotTone> = {
  connected: "positive",
  connecting: "progress",
  reconnecting: "progress",
  waiting: "neutral",
  not_paired: "warning",
};

function lastEvent(status: StatusSnapshot, { t, errorMessage, formatTime }: I18n): string {
  if (status.clipboard.activity === "sending") return t.status.clipboardSending;
  if (status.clipboard.activity === "receiving") return t.status.clipboardReceiving;
  if (status.lastError) return `${errorMessage(status.lastError.code)} (${formatTime(status.lastError.atMs)})`;
  if (status.clipboard.lastTransferMs !== null) {
    const time = formatTime(status.clipboard.lastTransferMs);
    return status.clipboard.lastDirection === "incoming" ? t.status.lastClipboardIn(time) : t.status.lastClipboardOut(time);
  }
  return t.status.nothingYet;
}

export function StatusView({ status, onPair, onOpenSecurity }: { status: StatusSnapshot; onPair: () => void; onOpenSecurity: () => void }) {
  const i18n = useI18n();
  const { t, errorMessage } = i18n;
  const update = useAction((patch: Parameters<typeof bridge.updateSettings>[0]) => bridge.updateSettings(patch));
  const release = useAction(() => bridge.releaseControl());
  const resync = useAction(() => bridge.resyncClipboard());
  const dismiss = useAction(() => bridge.clearError());
  const { connection, peer, settings } = status;
  const isMac = status.device.os === "macos";
  const connectionDetail =
    connection.state === "connected" && connection.rttMs !== null
      ? t.status.measuredLatency(connection.rttMs)
      : connection.nextRetryMs !== null && connection.state !== "connected"
        ? t.status.retryAt(Math.max(0, Math.ceil((connection.nextRetryMs - Date.now()) / 1000)))
        : undefined;
  const controlText = status.control === "controlling" && peer ? `${peer.name}` : status.control === "controlled" ? t.control.controlled : t.control.local;
  const diagramDescription = peer ? t.status.diagram(peer.name, t.sideOf[settings.peer_side], controlText) : t.status.noPeer;
  const actionError = update.error ?? release.error ?? resync.error ?? dismiss.error;

  return (
    <div className="page">
      <header className="page-header">
        <div>
          <h1 className="page-title">{status.device.name}</h1>
          <p className="page-subtitle">
            {t.os[status.device.os]} · {t.status.thisComputer}
          </p>
        </div>
        <p className="connection-pill" role="status" aria-live="polite">
          <StatusDot tone={tones[connection.state]} />
          {t.connection[connection.state]}
        </p>
      </header>

      {status.capture === "permission_required" ? (
        <Banner tone="warning" action={<Button onClick={onOpenSecurity}>{t.status.openPermissions}</Button>}>
          {t.status.permissionBanner}
        </Banner>
      ) : null}
      {status.capture === "failed" ? <Banner tone="warning">{t.status.captureFailed}</Banner> : null}

      <LayoutDiagram
        side={settings.peer_side}
        localName={status.device.name}
        peerName={peer?.name ?? null}
        control={status.control}
        description={diagramDescription}
      />

      <Group>
        <Row label={t.status.pairedComputer} detail={peer ? t.os[peer.os] : t.status.noPeer}>
          {peer ? <span className="value">{peer.name}</span> : <Button variant="primary" onClick={onPair}>{t.status.pairButton}</Button>}
        </Row>
        <Row label={t.status.connection} detail={connectionDetail}>
          <span className="value">{t.connection[connection.state]}</span>
        </Row>
        <Row label={t.status.activeControl} detail={t.status.hotkeyHint(isMac)}>
          {status.control === "local" ? (
            <span className="value">{controlText}</span>
          ) : (
            <Button onClick={() => void release.run()}>{t.status.releaseControl}</Button>
          )}
        </Row>
      </Group>

      <Group>
        <Row label={t.status.inputSharing}>
          <Toggle checked={settings.input_sharing} label={t.status.inputSharing} onChange={(value) => void update.run({ input_sharing: value })} />
        </Row>
        <Row label={t.status.clipboardSync}>
          <div className="inline-controls">
            <Button onClick={() => void resync.run()} disabled={!settings.clipboard_sync || connection.state !== "connected"}>
              {t.status.syncNow}
            </Button>
            <Toggle checked={settings.clipboard_sync} label={t.status.clipboardSync} onChange={(value) => void update.run({ clipboard_sync: value })} />
          </div>
        </Row>
        <Row label={t.status.lastEvent} detail={lastEvent(status, i18n)}>
          {status.lastError ? <Button onClick={() => void dismiss.run()}>{t.status.dismiss}</Button> : null}
        </Row>
      </Group>
      <InlineError message={actionError ? errorMessage(actionError) : null} />
    </div>
  );
}
