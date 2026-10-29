import { useCallback, useEffect, useRef, useState, type RefObject } from "react";
import type { SeamPhase } from "@/i18n/en";
import { prefersReducedMotion, SpringAnimator } from "./motion";

export interface SeamVisualValues {
  readonly cursorU: number;
  readonly cursorV: number;
  readonly dwellFill: number;
}

export interface SeamInspectorViewModel {
  readonly containerRef: RefObject<HTMLDivElement | null>;
  readonly phases: readonly SeamPhase[];
  readonly activePhase: SeamPhase;
  readonly visual: SeamVisualValues;
  readonly selectPhase: (phase: SeamPhase) => void;
}

const PHASE_ORDER: readonly SeamPhase[] = ["armed", "dwell", "crossed", "cooldown"];

const PHASE_TARGETS: Record<SeamPhase, SeamVisualValues> = {
  armed: { cursorU: 0.28, cursorV: 0.58, dwellFill: 0 },
  dwell: { cursorU: 0.495, cursorV: 0.62, dwellFill: 1 },
  crossed: { cursorU: 0.72, cursorV: 0.62, dwellFill: 0 },
  cooldown: { cursorU: 0.41, cursorV: 0.62, dwellFill: 0.35 },
};

export function useSeamInspectorViewModel(): SeamInspectorViewModel {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [activePhase, setActivePhase] = useState<SeamPhase>("armed");
  const [manualOverride, setManualOverride] = useState(false);
  const [inViewport, setInViewport] = useState(false);
  const [visual, setVisual] = useState<SeamVisualValues>(PHASE_TARGETS.armed);
  const animatorRef = useRef<SpringAnimator<"cursorU" | "cursorV" | "dwellFill"> | null>(null);

  useEffect(() => {
    const animator = new SpringAnimator<"cursorU" | "cursorV" | "dwellFill">(
      PHASE_TARGETS.armed,
      (values) => {
        setVisual({
          cursorU: values.cursorU,
          cursorV: values.cursorV,
          dwellFill: values.dwellFill,
        });
      },
    );
    animatorRef.current = animator;
    return () => {
      animator.stop();
      animatorRef.current = null;
    };
  }, []);

  useEffect(() => {
    const element = containerRef.current;
    if (!element || typeof IntersectionObserver !== "function") {
      return;
    }
    const observer = new IntersectionObserver(
      (entries) => {
        const entry = entries[0];
        if (entry) {
          setInViewport(entry.isIntersecting);
        }
      },
      { threshold: 0.2 },
    );
    observer.observe(element);
    return () => {
      observer.disconnect();
    };
  }, []);

  useEffect(() => {
    animatorRef.current?.to(PHASE_TARGETS[activePhase], {
      response: 0.4,
      damping: 1.0,
      epsilonDistance: 0.001,
      epsilonVelocity: 0.008,
    });
  }, [activePhase]);

  useEffect(() => {
    if (manualOverride || !inViewport || prefersReducedMotion()) {
      return;
    }
    const timer = window.setInterval(() => {
      if (document.hidden) {
        return;
      }
      setActivePhase((current) => {
        const index = PHASE_ORDER.indexOf(current);
        const next = PHASE_ORDER[(index + 1) % PHASE_ORDER.length];
        return next ?? "armed";
      });
    }, 2600);
    return () => {
      window.clearInterval(timer);
    };
  }, [inViewport, manualOverride]);

  const selectPhase = useCallback((phase: SeamPhase) => {
    setManualOverride(true);
    setActivePhase(phase);
  }, []);

  return {
    containerRef,
    phases: PHASE_ORDER,
    activePhase,
    visual,
    selectPhase,
  };
}
