import { useEffect, useMemo, useState } from "react";
import { bridge, type StatusSnapshot } from "@/bridge";
import { useAction } from "./useAction";

export type PairingTab = "host" | "join";

export function normalizeCodeInput(value: string): string {
  return value.replace(/\D/g, "").slice(0, 6);
}

export function secondsUntil(targetMs: number, nowMs: number): number {
  return Math.max(0, Math.ceil((targetMs - nowMs) / 1000));
}

export function usePairingViewModel(status: StatusSnapshot) {
  const [tab, setTab] = useState<PairingTab>("host");
  const [address, setAddress] = useState("");
  const [code, setCode] = useState("");
  const [now, setNow] = useState(() => Date.now());
  const start = useAction(() => bridge.startPairing());
  const join = useAction((target: string, value: string) => bridge.joinPairing(target, value));
  const cancel = useAction(() => bridge.cancelPairing());
  const pairing = status.pairing;

  useEffect(() => {
    if (pairing?.kind !== "hosting") {
      return;
    }
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, [pairing?.kind]);

  const remainingSeconds = pairing?.kind === "hosting" ? secondsUntil(pairing.expires_at_ms, now) : 0;
  const candidates = useMemo(() => status.discovered.filter((device) => !device.isPairedPeer || status.peer === null), [status]);
  const canJoin = code.length === 6 && address.trim().length > 0 && !join.pending;

  return {
    tab,
    setTab,
    address,
    setAddress,
    code,
    setCode: (value: string) => setCode(normalizeCodeInput(value)),
    pairing,
    remainingSeconds,
    candidates,
    canJoin,
    start,
    join,
    cancel,
    submitJoin: () => join.run(address.trim(), code),
    error: start.error ?? join.error ?? cancel.error,
  };
}
