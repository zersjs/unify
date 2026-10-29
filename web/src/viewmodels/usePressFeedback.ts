import { useCallback, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
import { HYSTERESIS_PX } from "./physics";

export interface PressFeedbackHandlers {
  readonly pressed: boolean;
  readonly onPointerDown: (event: ReactPointerEvent<HTMLElement>) => void;
  readonly onPointerMove: (event: ReactPointerEvent<HTMLElement>) => void;
  readonly onPointerUp: () => void;
  readonly onPointerCancel: () => void;
  readonly onPointerLeave: () => void;
  readonly onBlur: () => void;
}

export function usePressFeedback(): PressFeedbackHandlers {
  const [pressed, setPressed] = useState(false);
  const originRef = useRef<{ x: number; y: number } | null>(null);

  const onPointerDown = useCallback((event: ReactPointerEvent<HTMLElement>) => {
    if (event.button !== 0) {
      return;
    }
    originRef.current = { x: event.clientX, y: event.clientY };
    setPressed(true);
  }, []);

  const onPointerMove = useCallback((event: ReactPointerEvent<HTMLElement>) => {
    const origin = originRef.current;
    if (!origin) {
      return;
    }
    const moved = Math.hypot(event.clientX - origin.x, event.clientY - origin.y);
    if (moved > HYSTERESIS_PX) {
      originRef.current = null;
      setPressed(false);
    }
  }, []);

  const clear = useCallback(() => {
    originRef.current = null;
    setPressed(false);
  }, []);

  return {
    pressed,
    onPointerDown,
    onPointerMove,
    onPointerUp: clear,
    onPointerCancel: clear,
    onPointerLeave: clear,
    onBlur: clear,
  };
}
