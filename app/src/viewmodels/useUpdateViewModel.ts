import { useEffect, useState } from "react";
import { bridge, type UpdateStatus } from "@/bridge";
import { useAction } from "./useAction";

export function useUpdateViewModel() {
  const [status, setStatus] = useState<UpdateStatus>({ kind: "idle" });
  const check = useAction(() => bridge.checkForUpdates());
  const install = useAction(() => bridge.installUpdate());

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | null = null;
    bridge
      .onUpdate((next) => {
        if (!disposed) setStatus(next);
      })
      .then((stop) => {
        if (disposed) stop();
        else unlisten = stop;
      })
      .catch(() => undefined);
    bridge
      .updateStatus()
      .then((current) => {
        if (!disposed) setStatus(current);
      })
      .catch(() => undefined);
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  return { status, check, install };
}
