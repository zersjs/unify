import { useCallback, useEffect, useRef, useState, type RefObject } from "react";
import type { ClipboardDirection } from "@/i18n/en";
import { prefersReducedMotion } from "./motion";

export type ClipboardStep = "idle" | "copied" | "transmitting" | "pasted";

export interface ClipboardDemoViewModel {
  readonly sectionRef: RefObject<HTMLDivElement | null>;
  readonly direction: ClipboardDirection;
  readonly step: ClipboardStep;
  readonly selectDirection: (next: ClipboardDirection) => void;
  readonly replay: () => void;
}

const SEQUENCE: readonly { readonly step: ClipboardStep; readonly delayMs: number }[] = [
  { step: "copied", delayMs: 420 },
  { step: "transmitting", delayMs: 1050 },
  { step: "pasted", delayMs: 1750 },
];

export function useClipboardDemoViewModel(): ClipboardDemoViewModel {
  const sectionRef = useRef<HTMLDivElement | null>(null);
  const [direction, setDirection] = useState<ClipboardDirection>("mac-to-win");
  const [step, setStep] = useState<ClipboardStep>(() => (prefersReducedMotion() ? "pasted" : "pasted"));
  const [runToken, setRunToken] = useState(0);
  const hasEnteredRef = useRef(false);

  useEffect(() => {
    const element = sectionRef.current;
    if (!element || typeof IntersectionObserver !== "function" || prefersReducedMotion()) {
      return;
    }
    const observer = new IntersectionObserver(
      (entries) => {
        const entry = entries[0];
        if (entry?.isIntersecting && !hasEnteredRef.current) {
          hasEnteredRef.current = true;
          setRunToken((count) => count + 1);
        }
      },
      { threshold: 0.25 },
    );
    observer.observe(element);
    return () => {
      observer.disconnect();
    };
  }, []);

  useEffect(() => {
    if (runToken === 0) {
      return;
    }
    if (prefersReducedMotion()) {
      setStep("pasted");
      return;
    }
    setStep("idle");
    const timers = SEQUENCE.map((item) =>
      window.setTimeout(() => {
        setStep(item.step);
      }, item.delayMs),
    );
    return () => {
      for (const id of timers) {
        window.clearTimeout(id);
      }
    };
  }, [runToken]);

  const selectDirection = useCallback((next: ClipboardDirection) => {
    setDirection(next);
    if (prefersReducedMotion()) {
      setStep("pasted");
      return;
    }
    setRunToken((count) => count + 1);
  }, []);

  const replay = useCallback(() => {
    if (prefersReducedMotion()) {
      setStep("pasted");
      return;
    }
    setRunToken((count) => count + 1);
  }, []);

  return {
    sectionRef,
    direction,
    step,
    selectDirection,
    replay,
  };
}
