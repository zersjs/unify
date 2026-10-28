import { useEffect, useState } from "react";
import { bridge, failureCode, type StatusSnapshot } from "@/bridge";

export interface StatusViewModel {
  status: StatusSnapshot | null;
  loadError: string | null;
}

export function useStatusViewModel(): StatusViewModel {
  const [status, setStatus] = useState<StatusSnapshot | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | null = null;
    bridge
      .onStatus((next) => {
        if (!disposed) {
          setStatus(next);
        }
      })
      .then((stop) => {
        if (disposed) {
          stop();
        } else {
          unlisten = stop;
        }
      })
      .catch((error: unknown) => setLoadError(failureCode(error)));
    bridge
      .status()
      .then((initial) => {
        if (!disposed) {
          setStatus((current) => current ?? initial);
        }
      })
      .catch((error: unknown) => setLoadError(failureCode(error)));
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  return { status, loadError };
}
