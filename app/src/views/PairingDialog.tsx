import { useId } from "react";
import { bridge, type StatusSnapshot } from "@/bridge";
import { Button, InlineError, Modal, Segmented } from "@/components/controls";
import { useI18n } from "@/i18n";
import { useAction } from "@/viewmodels/useAction";
import { usePairingViewModel, type PairingTab } from "@/viewmodels/usePairingViewModel";

export function PairingDialog({ status, onClose }: { status: StatusSnapshot; onClose: () => void }) {
  const { t, errorMessage } = useI18n();
  const vm = usePairingViewModel(status);
  const addressId = useId();
  const codeId = useId();
  const pairing = vm.pairing;
  const close = () => {
    if (pairing && (pairing.kind === "hosting" || pairing.kind === "joining" || pairing.kind === "awaiting_peer_approval")) {
      void vm.cancel.run();
    }
    onClose();
  };

  const footer = <Button onClick={close}>{pairing?.kind === "succeeded" ? t.pairing.close : t.pairing.cancel}</Button>;

  let body;
  if (pairing?.kind === "succeeded") {
    body = <p className="lead">{t.pairing.succeeded(pairing.peer_name)}</p>;
  } else if (pairing?.kind === "joining") {
    body = <p className="lead" role="status">{t.pairing.joining(pairing.address)}</p>;
  } else if (pairing?.kind === "awaiting_peer_approval") {
    body = (
      <>
        <p className="lead" role="status">{t.pairing.awaitingPeer(pairing.peer_name)}</p>
        <p className="muted">{t.pairing.ownFingerprint}</p>
        <p className="mono fingerprint">{status.device.fingerprint}</p>
      </>
    );
  } else if (pairing?.kind === "hosting") {
    body = (
      <div className="pairing-code-block">
        <p>{t.pairing.hostIntro}</p>
        <p className="pairing-code" aria-label={`${t.pairing.codeLabel}: ${pairing.code.split("").join(" ")}`}>
          {pairing.code.slice(0, 3)}&#8201;{pairing.code.slice(3)}
        </p>
        <p className="muted" aria-live="polite">{t.pairing.expiresIn(vm.remainingSeconds)}</p>
        <p className="muted">
          {t.pairing.ownFingerprint}: <span className="mono">{status.device.fingerprint}</span>
        </p>
      </div>
    );
  } else {
    body = (
      <>
        {pairing?.kind === "failed" ? <p className="inline-error" role="alert">{errorMessage(pairing.reason)}</p> : null}
        {status.peer ? <p className="muted">{t.pairing.replaceWarning}</p> : null}
        <Segmented<PairingTab>
          label={t.pairing.title}
          value={vm.tab}
          onChange={vm.setTab}
          options={[
            { value: "host", label: t.pairing.hostTab },
            { value: "join", label: t.pairing.joinTab },
          ]}
        />
        {vm.tab === "host" ? (
          <div className="stack">
            <p>{t.pairing.hostIntro}</p>
            <div>
              <Button variant="primary" onClick={() => void vm.start.run()} disabled={vm.start.pending}>
                {t.pairing.createCode}
              </Button>
            </div>
          </div>
        ) : (
          <form
            className="stack"
            onSubmit={(event) => {
              event.preventDefault();
              if (vm.canJoin) {
                void vm.submitJoin();
              }
            }}
          >
            <p>{t.pairing.joinIntro}</p>
            <fieldset className="discovered">
              <legend>{t.pairing.discovered}</legend>
              {vm.candidates.length === 0 ? <p className="muted">{t.pairing.noneDiscovered}</p> : null}
              {vm.candidates.map((device) => (
                <label key={device.id} className="discovered-item">
                  <input type="radio" name="discovered" checked={vm.address === device.address} onChange={() => vm.setAddress(device.address)} />
                  <span>
                    {device.name} <span className="muted">· {t.os[device.os]} · {device.address}</span>
                  </span>
                </label>
              ))}
            </fieldset>
            <label className="field" htmlFor={addressId}>
              <span>{t.pairing.addressLabel}</span>
              <input
                id={addressId}
                className="text-input"
                value={vm.address}
                placeholder={t.pairing.addressPlaceholder}
                autoComplete="off"
                spellCheck={false}
                maxLength={260}
                onChange={(event) => vm.setAddress(event.target.value)}
              />
            </label>
            <label className="field" htmlFor={codeId}>
              <span>{t.pairing.codeInputLabel}</span>
              <input
                id={codeId}
                className="text-input code-input"
                value={vm.code}
                inputMode="numeric"
                autoComplete="one-time-code"
                maxLength={6}
                onChange={(event) => vm.setCode(event.target.value)}
              />
            </label>
            <div>
              <Button type="submit" variant="primary" disabled={!vm.canJoin}>
                {t.pairing.join}
              </Button>
            </div>
          </form>
        )}
      </>
    );
  }

  return (
    <Modal title={t.pairing.title} onClose={close} footer={footer}>
      {body}
      <InlineError message={vm.error ? errorMessage(vm.error) : null} />
    </Modal>
  );
}

export function ApprovalDialog({ status }: { status: StatusSnapshot }) {
  const { t, errorMessage } = useI18n();
  const decide = useAction((accept: boolean) => bridge.approvePairing(accept));
  const pairing = status.pairing;
  if (pairing?.kind !== "awaiting_approval") {
    return null;
  }
  return (
    <Modal
      title={t.pairing.approvalTitle}
      onClose={() => void decide.run(false)}
      footer={
        <>
          <Button onClick={() => void decide.run(false)} disabled={decide.pending}>
            {t.pairing.reject}
          </Button>
          <Button variant="primary" onClick={() => void decide.run(true)} disabled={decide.pending}>
            {t.pairing.approve}
          </Button>
        </>
      }
    >
      <p className="lead">{t.pairing.approvalBody(pairing.peer_name, t.os[pairing.peer_os])}</p>
      <p className="muted">{t.pairing.approvalFingerprint}</p>
      <p className="mono fingerprint">{pairing.fingerprint}</p>
      {status.peer ? <p className="muted">{t.pairing.replaceWarning}</p> : null}
      <InlineError message={decide.error ? errorMessage(decide.error) : null} />
    </Modal>
  );
}
