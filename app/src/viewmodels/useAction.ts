import { useCallback, useRef, useState } from "react";
import { failureCode } from "@/bridge";

export interface ActionState<A extends unknown[]> {
  run: (...args: A) => Promise<boolean>;
  pending: boolean;
  error: string | null;
  clearError: () => void;
}

export function useAction<A extends unknown[]>(operation: (...args: A) => Promise<unknown>): ActionState<A> {
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const inFlight = useRef(false);
  const operationRef = useRef(operation);
  operationRef.current = operation;

  const run = useCallback(async (...args: A) => {
    if (inFlight.current) {
      return false;
    }
    inFlight.current = true;
    setPending(true);
    setError(null);
    try {
      await operationRef.current(...args);
      return true;
    } catch (caught) {
      setError(failureCode(caught));
      return false;
    } finally {
      inFlight.current = false;
      setPending(false);
    }
  }, []);

  const clearError = useCallback(() => setError(null), []);
  return { run, pending, error, clearError };
}
