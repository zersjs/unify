import { useEffect, useId, useState, type ReactNode } from "react";
import type { LogLevel, MacKeyboardLayout, Side, StatusSnapshot, UiLanguage } from "@/bridge";
import { Button, Group, InlineError, Row, Segmented, Toggle, ToggleRow } from "@/components/controls";
import { DraggableLayout } from "@/components/DraggableLayout";
import { Logo } from "@/components/Logo";
import { languageNames, useI18n } from "@/i18n";
import type { SettingsViewModel } from "@/viewmodels/useSettingsViewModel";

interface PageProps {
  status: StatusSnapshot;
  vm: SettingsViewModel;
}

function PageFrame({ title, children, vm }: { title: string; children: ReactNode; vm: SettingsViewModel }) {
  const { errorMessage } = useI18n();
  return (
    <div className="page">
      <header className="page-header">
        <h1 className="page-title">{title}</h1>
      </header>
      {children}
      <InlineError message={vm.error ? errorMessage(vm.error) : null} />
    </div>
  );
}

function CommitField({
  label,
  detail,
  value,
  onCommit,
  inputMode,
  maxLength,
  placeholder,
}: {
  label: string;
  detail?: string;
  value: string;
  onCommit: (value: string) => void;
  inputMode?: "numeric" | "text";
  maxLength: number;
  placeholder?: string;
}) {
  const { t } = useI18n();
  const id = useId();
  const [draft, setDraft] = useState(value);
  useEffect(() => setDraft(value), [value]);
  const dirty = draft !== value;
  return (
    <Row label={label} detail={detail} htmlFor={id}>
      <form
        className="inline-controls"
        onSubmit={(event) => {
          event.preventDefault();
          if (dirty) onCommit(draft);
        }}
      >
        <input
          id={id}
          className="text-input"
          value={draft}
          inputMode={inputMode}
          maxLength={maxLength}
          placeholder={placeholder}
          spellCheck={false}
          autoComplete="off"
          onChange={(event) => setDraft(event.target.value)}
        />
        <Button type="submit" disabled={!dirty}>
          {t.settings.save}
        </Button>
      </form>
    </Row>
  );
}

export function GeneralPage({ status, vm }: PageProps) {
  const { t } = useI18n();
  return (
    <PageFrame title={t.nav.general} vm={vm}>
      <Group>
        <CommitField
          label={t.settings.deviceName}
          detail={t.settings.deviceNameHelp}
          value={vm.settings.device_name}
          maxLength={64}
          onCommit={(device_name) => void vm.update.run({ device_name })}
        />
        <ToggleRow label={t.settings.autostart} detail={t.settings.autostartHelp} checked={vm.settings.autostart} onChange={vm.toggle("autostart")} />
      </Group>
      <Group>
        <Row label={t.settings.language} detail={t.settings.languageHelp}>
          <Segmented<UiLanguage>
            label={t.settings.language}
            value={vm.settings.language}
            onChange={(language) => void vm.update.run({ language })}
            options={(["en", "tr"] as const).map((language) => ({ value: language, label: languageNames[language] }))}
          />
        </Row>
      </Group>
      <Group>
        <Row label={t.settings.configLocation} detail={<span className="mono selectable">{status.device.configPath}</span>} />
      </Group>
    </PageFrame>
  );
}

export function DevicesPage({ status, vm, onPair }: PageProps & { onPair: () => void }) {
  const { t, formatDate } = useI18n();
  const [confirming, setConfirming] = useState(false);
  const peer = status.peer;
  return (
    <PageFrame title={t.nav.devices} vm={vm}>
      <Group title={t.settings.peerSection}>
        {peer ? (
          <>
            <Row label={peer.name} detail={t.os[peer.os]} />
            <Row label={t.settings.fingerprint} detail={<span className="mono">{peer.fingerprint}</span>} />
            <Row label={t.settings.pairedAt} detail={formatDate(peer.pairedAt)} />
            <Row label={t.settings.lastAddress} detail={peer.lastAddress ?? "—"} />
            <Row label={confirming ? t.settings.removePeerConfirm : t.settings.removePeer}>
              {confirming ? (
                <div className="inline-controls">
                  <Button onClick={() => setConfirming(false)}>{t.settings.no}</Button>
                  <Button
                    variant="destructive"
                    onClick={() => {
                      setConfirming(false);
                      void vm.removePeer.run();
                    }}
                  >
                    {t.settings.removePeer}
                  </Button>
                </div>
              ) : (
                <Button variant="destructive" onClick={() => setConfirming(true)}>
                  {t.settings.removePeer}
                </Button>
              )}
            </Row>
          </>
        ) : (
          <Row label={t.status.noPeer} />
        )}
      </Group>
      <Group>
        <Row label={t.pairing.ownFingerprint} detail={<span className="mono">{status.device.fingerprint}</span>}>
          <Button variant="primary" onClick={onPair}>
            {t.settings.pairNew}
          </Button>
        </Row>
      </Group>
    </PageFrame>
  );
}

export function LayoutPage({ status, vm }: PageProps) {
  const { t } = useI18n();
  const sides: Side[] = ["Left", "Right", "Top", "Bottom"];
  const peerName = status.peer?.name ?? null;
  return (
    <PageFrame title={t.nav.layout} vm={vm}>
      <DraggableLayout
        side={vm.settings.peer_side}
        localName={status.device.name}
        peerName={peerName ?? t.status.pairedComputer}
        onChange={(peer_side) => void vm.update.run({ peer_side })}
      />
      <Group footer={t.settings.peerPositionHelp}>
        <Row label={t.settings.peerPosition}>
          <Segmented<Side>
            label={t.settings.peerPosition}
            value={vm.settings.peer_side}
            onChange={(peer_side) => void vm.update.run({ peer_side })}
            options={sides.map((side) => ({ value: side, label: t.side[side] }))}
          />
        </Row>
      </Group>
      <Group>
        <ToggleRow label={t.settings.edgeSwitch} checked={vm.settings.edge_switch} onChange={vm.toggle("edge_switch")} />
        <RangeRow
          label={t.settings.dwell}
          detail={t.settings.dwellHelp}
          value={vm.settings.edge_dwell_ms}
          min={0}
          max={1000}
          step={50}
          format={t.settings.msUnit}
          disabled={!vm.settings.edge_switch}
          onCommit={(edge_dwell_ms) => void vm.update.run({ edge_dwell_ms })}
        />
        <RangeRow
          label={t.settings.cornerGuard}
          detail={t.settings.cornerGuardHelp}
          value={vm.settings.corner_guard_px}
          min={0}
          max={200}
          step={10}
          format={t.settings.pxUnit}
          disabled={!vm.settings.edge_switch}
          onCommit={(corner_guard_px) => void vm.update.run({ corner_guard_px })}
        />
      </Group>
      <Group title={t.settings.displays}>
        {status.displays.map((display, index) => (
          <Row key={`${display.x}:${display.y}:${index}`} label={`${display.width} × ${display.height}`} detail={`(${display.x}, ${display.y})`} />
        ))}
      </Group>
    </PageFrame>
  );
}

function RangeRow({
  label,
  detail,
  value,
  min,
  max,
  step,
  format,
  disabled,
  onCommit,
}: {
  label: string;
  detail: string;
  value: number;
  min: number;
  max: number;
  step: number;
  format: (value: number) => string;
  disabled: boolean;
  onCommit: (value: number) => void;
}) {
  const id = useId();
  const [draft, setDraft] = useState(value);
  useEffect(() => setDraft(value), [value]);
  const commit = () => {
    if (draft !== value) onCommit(draft);
  };
  return (
    <Row label={label} detail={detail} htmlFor={id}>
      <div className="inline-controls">
        <input
          id={id}
          type="range"
          min={min}
          max={max}
          step={step}
          value={draft}
          disabled={disabled}
          aria-valuetext={format(draft)}
          onChange={(event) => setDraft(Number(event.target.value))}
          onPointerUp={commit}
          onKeyUp={commit}
          onBlur={commit}
        />
        <span className="value range-value">{format(draft)}</span>
      </div>
    </Row>
  );
}

export function InputPage({ status, vm }: PageProps) {
  const { t } = useI18n();
  const isMac = status.device.os === "macos";
  return (
    <PageFrame title={t.nav.input} vm={vm}>
      <Group>
        <ToggleRow label={t.status.inputSharing} checked={vm.settings.input_sharing} onChange={vm.toggle("input_sharing")} />
        <ToggleRow label={t.settings.hotkey} detail={t.status.hotkeyHint(isMac)} checked={vm.settings.switch_hotkey} onChange={vm.toggle("switch_hotkey")} />
      </Group>
      <Group>
        <ToggleRow
          label={t.settings.swapCommand}
          detail={t.settings.swapCommandHelp}
          checked={vm.settings.swap_command_control}
          onChange={vm.toggle("swap_command_control")}
        />
        <ToggleRow label={t.settings.invertScroll} detail={t.settings.invertScrollHelp} checked={vm.settings.invert_scroll} onChange={vm.toggle("invert_scroll")} />
      </Group>
      <Group footer={t.settings.layoutNote}>
        {isMac ? (
          <Row label={t.settings.macLayout}>
            <Segmented<MacKeyboardLayout>
              label={t.settings.macLayout}
              value={vm.settings.mac_keyboard_layout}
              onChange={(mac_keyboard_layout) => void vm.update.run({ mac_keyboard_layout })}
              options={[
                { value: "auto", label: t.settings.macLayoutAuto(status.isoKeyboard) },
                { value: "iso", label: t.settings.macLayoutIso },
                { value: "ansi", label: t.settings.macLayoutAnsi },
              ]}
            />
          </Row>
        ) : null}
      </Group>
    </PageFrame>
  );
}

export function ClipboardPage({ status, vm }: PageProps) {
  const { t, skipMessage, formatTime } = useI18n();
  const last = status.clipboard.lastTransferMs;
  return (
    <PageFrame title={t.nav.clipboard} vm={vm}>
      <Group footer={t.settings.clipboardInfo}>
        <Row label={t.status.clipboardSync}>
          <div className="inline-controls">
            <Button onClick={() => void vm.resync.run()} disabled={!vm.settings.clipboard_sync || status.connection.state !== "connected"}>
              {t.status.syncNow}
            </Button>
            <Toggle checked={vm.settings.clipboard_sync} label={t.status.clipboardSync} onChange={vm.toggle("clipboard_sync")} />
          </div>
        </Row>
        <Row
          label={t.settings.lastTransfer}
          detail={
            last === null
              ? t.status.nothingYet
              : status.clipboard.lastDirection === "incoming"
                ? t.status.lastClipboardIn(formatTime(last))
                : t.status.lastClipboardOut(formatTime(last))
          }
        />
        <Row label={t.settings.lastSkip} detail={status.clipboard.lastSkip ? skipMessage(status.clipboard.lastSkip) : "—"} />
      </Group>
    </PageFrame>
  );
}

export function NetworkPage({ status, vm }: PageProps) {
  const { t } = useI18n();
  const connection = status.connection;
  return (
    <PageFrame title={t.nav.network} vm={vm}>
      <Group>
        <Row label={t.status.connection} detail={connection.rttMs !== null ? t.status.measuredLatency(connection.rttMs) : undefined}>
          <span className="value">{t.connection[connection.state]}</span>
        </Row>
        <Row label={t.settings.remoteAddress} detail={connection.remoteAddress ?? "—"} />
        <Row label={t.settings.attempts} detail={String(connection.attempt)}>
          <Button onClick={() => void vm.reconnect.run()} disabled={status.peer === null}>
            {t.settings.reconnect}
          </Button>
        </Row>
      </Group>
      <Group footer={t.settings.manualAddressHelp}>
        {status.peer ? (
          <CommitField
            label={t.settings.manualAddress}
            value={status.peer.manualAddress ?? ""}
            maxLength={260}
            placeholder={t.pairing.addressPlaceholder}
            onCommit={(value) => void vm.setManualAddress.run(value.trim() === "" ? null : value)}
          />
        ) : (
          <Row label={t.settings.manualAddress} detail={t.errors.not_paired} />
        )}
      </Group>
      <Group footer={t.settings.portHelp}>
        <CommitField
          label={t.settings.port}
          detail={status.device.port ? `${status.device.port}` : undefined}
          value={String(vm.settings.listen_port)}
          inputMode="numeric"
          maxLength={5}
          onCommit={(value) => {
            const port = Number.parseInt(value, 10);
            if (Number.isInteger(port) && port >= 1024 && port <= 65535) {
              void vm.update.run({ listen_port: port });
            }
          }}
        />
        <ToggleRow label={t.settings.discovery} checked={vm.settings.discovery} onChange={vm.toggle("discovery")} />
      </Group>
      <Group title={t.pairing.discovered}>
        {status.discovered.length === 0 ? <Row label={t.pairing.noneDiscovered} /> : null}
        {status.discovered.map((device) => (
          <Row key={device.id} label={device.name} detail={`${t.os[device.os]} · ${device.address}`} />
        ))}
      </Group>
    </PageFrame>
  );
}

export function SecurityPage({ status, vm }: PageProps) {
  const { t } = useI18n();
  const [confirming, setConfirming] = useState(false);
  const isMac = status.device.os === "macos";
  return (
    <PageFrame title={t.nav.security} vm={vm}>
      {isMac ? (
        <Group footer={t.settings.macLimits}>
          <Row label={t.settings.accessibility} detail={t.permission[status.permissions.accessibility]}>
            <Button onClick={() => void vm.openPermissionSettings.run("accessibility")}>{t.settings.openSettings}</Button>
          </Row>
          <Row label={t.settings.inputMonitoring} detail={t.permission[status.permissions.inputMonitoring]}>
            <Button onClick={() => void vm.openPermissionSettings.run("input_monitoring")}>{t.settings.openSettings}</Button>
          </Row>
          <Row label={t.settings.localNetwork} detail={t.permission[status.permissions.localNetwork]}>
            <Button onClick={() => void vm.openPermissionSettings.run("local_network")}>{t.settings.openSettings}</Button>
          </Row>
          <Row label={t.settings.requestPermissions}>
            <Button variant="primary" onClick={() => void vm.requestPermissions.run()}>
              {t.settings.requestPermissions}
            </Button>
          </Row>
        </Group>
      ) : (
        <Group footer={t.settings.windowsLimits}>
          <Row label={t.settings.accessibility} detail={t.permission[status.permissions.accessibility]} />
        </Group>
      )}
      <Group footer={t.settings.resetIdentityHelp}>
        <Row label={t.settings.keyStorage} detail={status.device.secretBackend} />
        <Row label={t.pairing.ownFingerprint} detail={<span className="mono">{status.device.fingerprint}</span>} />
        <Row label={confirming ? t.settings.resetIdentityConfirm : t.settings.resetIdentity}>
          {confirming ? (
            <div className="inline-controls">
              <Button onClick={() => setConfirming(false)}>{t.settings.no}</Button>
              <Button
                variant="destructive"
                onClick={() => {
                  setConfirming(false);
                  void vm.resetIdentity.run();
                }}
              >
                {t.settings.confirm}
              </Button>
            </div>
          ) : (
            <Button variant="destructive" onClick={() => setConfirming(true)}>
              {t.settings.resetIdentity}
            </Button>
          )}
        </Row>
      </Group>
    </PageFrame>
  );
}

export function DiagnosticsPage({ status, vm }: PageProps) {
  const { t } = useI18n();
  const [copied, setCopied] = useState(false);
  const levels: LogLevel[] = ["error", "warn", "info", "debug"];
  const reportId = useId();
  return (
    <PageFrame title={t.nav.diagnostics} vm={vm}>
      <Group>
        <Row label={t.settings.logLevel}>
          <Segmented<LogLevel>
            label={t.settings.logLevel}
            value={vm.settings.log_level}
            onChange={(log_level) => void vm.update.run({ log_level })}
            options={levels.map((level) => ({ value: level, label: t.settings.logLevels[level] }))}
          />
        </Row>
        <Row label={t.settings.openLogs}>
          <Button onClick={() => void vm.openLogs.run()}>{t.settings.openLogs}</Button>
        </Row>
      </Group>
      <Group title={t.settings.stats}>
        <Row label={t.settings.statSessions} detail={String(status.stats.sessionsEstablished)} />
        <Row label={t.settings.statSent} detail={String(status.stats.inputEventsSent)} />
        <Row label={t.settings.statReceived} detail={String(status.stats.inputEventsReceived)} />
        <Row label={t.settings.statClipSent} detail={String(status.stats.clipboardSent)} />
        <Row label={t.settings.statClipReceived} detail={String(status.stats.clipboardReceived)} />
        <Row label={t.settings.statRejected} detail={String(status.stats.rejectedConnections)} />
      </Group>
      <Group title={t.settings.diagnostics} footer={t.settings.diagnosticsHelp}>
        <Row label={t.settings.diagnostics}>
          <div className="inline-controls">
            <Button onClick={() => void vm.loadDiagnostics.run()}>{t.settings.generateDiagnostics}</Button>
            <Button
              disabled={!vm.diagnostics}
              onClick={() => {
                if (vm.diagnostics) {
                  void navigator.clipboard.writeText(vm.diagnostics).then(
                    () => setCopied(true),
                    () => setCopied(false),
                  );
                }
              }}
            >
              {copied ? t.settings.copied : t.settings.copy}
            </Button>
          </div>
        </Row>
        {vm.diagnostics ? (
          <textarea id={reportId} className="report" readOnly value={vm.diagnostics} aria-label={t.settings.diagnostics} rows={10} />
        ) : null}
      </Group>
    </PageFrame>
  );
}

export function AboutPage({ status, vm }: PageProps) {
  const { t } = useI18n();
  return (
    <PageFrame title={t.nav.about} vm={vm}>
      <div className="about-hero">
        <Logo size={72} title={t.appName} />
        <div>
          <p className="about-name">{t.appName}</p>
          <p className="muted">
            {t.settings.version} {status.appVersion}
          </p>
        </div>
      </div>
      <Group footer={t.settings.aboutBody}>
        <Row label={t.appName} detail={`${t.settings.version} ${status.appVersion}`} />
        <Row label={t.status.thisComputer} detail={`${status.device.name} · ${t.os[status.device.os]}`} />
      </Group>
    </PageFrame>
  );
}
