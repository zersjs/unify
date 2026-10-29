import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
  type PointerEvent as ReactPointerEvent,
  type RefObject,
} from "react";
import { prefersReducedMotion, SpringAnimator } from "./motion";
import {
  boundaryIntensity,
  clamp,
  constrainWithRubberBand,
  HYSTERESIS_PX,
  projectRest,
  releaseVelocity,
  type PointerSample,
} from "./physics";

export type ActiveMachine = "macos" | "windows";
export type StageMode = "auto" | "manual";

export interface WorkspacePose {
  readonly u: number;
  readonly v: number;
}

export interface WorkspaceStageViewModel {
  readonly stageRef: RefObject<HTMLDivElement | null>;
  readonly pose: WorkspacePose;
  readonly activeMachine: ActiveMachine;
  readonly seamGlow: number;
  readonly ratioDisplay: string;
  readonly mode: StageMode;
  readonly dragging: boolean;
  readonly focusMachine: (machine: ActiveMachine) => void;
  readonly resumeAutoCycle: () => void;
  readonly onStagePointerDown: (event: ReactPointerEvent<HTMLDivElement>) => void;
  readonly onStagePointerMove: (event: ReactPointerEvent<HTMLDivElement>) => void;
  readonly onStagePointerUp: (event: ReactPointerEvent<HTMLDivElement>) => void;
  readonly onStagePointerCancel: (event: ReactPointerEvent<HTMLDivElement>) => void;
  readonly onStageKeyDown: (event: ReactKeyboardEvent<HTMLDivElement>) => void;
}

const WAYPOINTS: readonly WorkspacePose[] = [
  { u: 0.28, v: 0.46 },
  { u: 0.46, v: 0.52 },
  { u: 0.72, v: 0.52 },
  { u: 0.79, v: 0.62 },
  { u: 0.54, v: 0.49 },
  { u: 0.25, v: 0.54 },
];

const MIN_U = 0.08;
const MAX_U = 0.92;
const MIN_V = 0.22;
const MAX_V = 0.8;
const STEP_INTERVAL_MS = 2350;

interface DragSession {
  pointerId: number;
  startClientX: number;
  startClientY: number;
  grabOffsetU: number;
  grabOffsetV: number;
  movedBeyondThreshold: boolean;
  samples: PointerSample[];
}

export function machineForU(u: number): ActiveMachine {
  return u < 0.5 ? "macos" : "windows";
}

export function useWorkspaceStageViewModel(): WorkspaceStageViewModel {
  const stageRef = useRef<HTMLDivElement | null>(null);
  const [pose, setPose] = useState<WorkspacePose>({ u: 0.28, v: 0.46 });
  const [mode, setMode] = useState<StageMode>("auto");
  const [dragging, setDragging] = useState(false);
  const [inViewport, setInViewport] = useState(true);

  const animatorRef = useRef<SpringAnimator<"u" | "v"> | null>(null);
  const waypointIndexRef = useRef(0);
  const dragSessionRef = useRef<DragSession | null>(null);

  useEffect(() => {
    const animator = new SpringAnimator<"u" | "v">(
      { u: 0.28, v: 0.46 },
      (values) => {
        setPose({ u: values.u, v: values.v });
      },
      (active) => {
        if (stageRef.current) {
          stageRef.current.dataset.animating = active ? "true" : "false";
        }
      },
    );
    animatorRef.current = animator;
    return () => {
      animator.stop();
      animatorRef.current = null;
    };
  }, []);

  useEffect(() => {
    const element = stageRef.current;
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
      { threshold: 0.15 },
    );
    observer.observe(element);
    return () => {
      observer.disconnect();
    };
  }, []);

  useEffect(() => {
    if (mode !== "auto" || !inViewport || prefersReducedMotion()) {
      return;
    }
    const timer = window.setInterval(() => {
      if (document.hidden) {
        return;
      }
      waypointIndexRef.current = (waypointIndexRef.current + 1) % WAYPOINTS.length;
      const next = WAYPOINTS[waypointIndexRef.current];
      if (next) {
        animatorRef.current?.to(
          { u: next.u, v: next.v },
          { response: 0.72, damping: 1.0, epsilonDistance: 0.0006, epsilonVelocity: 0.005 },
        );
      }
    }, STEP_INTERVAL_MS);
    return () => {
      window.clearInterval(timer);
    };
  }, [inViewport, mode]);

  const normalizedFromPointer = useCallback((clientX: number, clientY: number): WorkspacePose | null => {
    const element = stageRef.current;
    if (!element) {
      return null;
    }
    const rect = element.getBoundingClientRect();
    if (rect.width <= 0 || rect.height <= 0) {
      return null;
    }
    const u = (clientX - rect.left) / rect.width;
    const v = (clientY - rect.top) / rect.height;
    return { u, v };
  }, []);

  const onStagePointerDown = useCallback(
    (event: ReactPointerEvent<HTMLDivElement>) => {
      if (event.button !== 0) {
        return;
      }
      const norm = normalizedFromPointer(event.clientX, event.clientY);
      if (!norm) {
        return;
      }
      setMode("manual");
      const live = animatorRef.current?.current() ?? pose;
      animatorRef.current?.stop();
      const now = performance.now();
      dragSessionRef.current = {
        pointerId: event.pointerId,
        startClientX: event.clientX,
        startClientY: event.clientY,
        grabOffsetU: live.u - norm.u,
        grabOffsetV: live.v - norm.v,
        movedBeyondThreshold: false,
        samples: [{ t: now, u: live.u, v: live.v }],
      };
      setDragging(true);
      event.currentTarget.setPointerCapture?.(event.pointerId);
    },
    [normalizedFromPointer, pose],
  );

  const onStagePointerMove = useCallback(
    (event: ReactPointerEvent<HTMLDivElement>) => {
      const session = dragSessionRef.current;
      if (!session || session.pointerId !== event.pointerId) {
        return;
      }
      const norm = normalizedFromPointer(event.clientX, event.clientY);
      if (!norm) {
        return;
      }
      const movedPx = Math.hypot(event.clientX - session.startClientX, event.clientY - session.startClientY);
      if (movedPx >= HYSTERESIS_PX) {
        session.movedBeyondThreshold = true;
      }
      const rawU = norm.u + session.grabOffsetU;
      const rawV = norm.v + session.grabOffsetV;
      const constrainedU = constrainWithRubberBand(rawU, MIN_U, MAX_U, 0.32);
      const constrainedV = constrainWithRubberBand(rawV, MIN_V, MAX_V, 0.28);
      const now = performance.now();
      session.samples.push({ t: now, u: constrainedU, v: constrainedV });
      if (session.samples.length > 16) {
        session.samples.shift();
      }
      animatorRef.current?.set({ u: constrainedU, v: constrainedV });
    },
    [normalizedFromPointer],
  );

  const finishPointerSession = useCallback(
    (event: ReactPointerEvent<HTMLDivElement>, cancelled: boolean) => {
      const session = dragSessionRef.current;
      if (!session || session.pointerId !== event.pointerId) {
        return;
      }
      dragSessionRef.current = null;
      setDragging(false);
      if (event.currentTarget.hasPointerCapture?.(event.pointerId)) {
        event.currentTarget.releasePointerCapture(event.pointerId);
      }

      const live = animatorRef.current?.current() ?? pose;

      if (cancelled) {
        animatorRef.current?.to(
          { u: clamp(live.u, MIN_U, MAX_U), v: clamp(live.v, MIN_V, MAX_V) },
          { response: 0.35, damping: 1.0 },
        );
        return;
      }

      if (!session.movedBeyondThreshold) {
        const tapNorm = normalizedFromPointer(event.clientX, event.clientY);
        if (tapNorm) {
          animatorRef.current?.to(
            { u: clamp(tapNorm.u, MIN_U, MAX_U), v: clamp(tapNorm.v, MIN_V, MAX_V) },
            { response: 0.4, damping: 1.0 },
          );
        }
        return;
      }

      const velocity = releaseVelocity(session.samples);
      const speed = Math.hypot(velocity.u, velocity.v);
      const projectedU = clamp(projectRest(live.u, velocity.u * 90), MIN_U, MAX_U);
      const projectedV = clamp(projectRest(live.v, velocity.v * 90), MIN_V, MAX_V);
      animatorRef.current?.to(
        { u: projectedU, v: projectedV },
        {
          response: 0.38,
          damping: speed > 0.85 ? 0.8 : 1.0,
          initialVelocity: { u: velocity.u, v: velocity.v },
        },
      );
    },
    [normalizedFromPointer, pose],
  );

  const onStagePointerUp = useCallback(
    (event: ReactPointerEvent<HTMLDivElement>) => {
      finishPointerSession(event, false);
    },
    [finishPointerSession],
  );

  const onStagePointerCancel = useCallback(
    (event: ReactPointerEvent<HTMLDivElement>) => {
      finishPointerSession(event, true);
    },
    [finishPointerSession],
  );

  const focusMachine = useCallback((machine: ActiveMachine) => {
    setMode("manual");
    const target: WorkspacePose = machine === "macos" ? { u: 0.27, v: 0.48 } : { u: 0.73, v: 0.52 };
    animatorRef.current?.to({ u: target.u, v: target.v }, { response: 0.4, damping: 1.0 });
  }, []);

  const resumeAutoCycle = useCallback(() => {
    setMode("auto");
    const next = WAYPOINTS[waypointIndexRef.current];
    if (next) {
      animatorRef.current?.to({ u: next.u, v: next.v }, { response: 0.55, damping: 1.0 });
    }
  }, []);

  const onStageKeyDown = useCallback(
    (event: ReactKeyboardEvent<HTMLDivElement>) => {
      const live = animatorRef.current?.current() ?? pose;
      let deltaU = 0;
      let deltaV = 0;
      switch (event.key) {
        case "ArrowLeft":
          deltaU = -0.09;
          break;
        case "ArrowRight":
          deltaU = 0.09;
          break;
        case "ArrowUp":
          deltaV = -0.07;
          break;
        case "ArrowDown":
          deltaV = 0.07;
          break;
        default:
          return;
      }
      event.preventDefault();
      setMode("manual");
      animatorRef.current?.to(
        { u: clamp(live.u + deltaU, MIN_U, MAX_U), v: clamp(live.v + deltaV, MIN_V, MAX_V) },
        { response: 0.35, damping: 1.0 },
      );
    },
    [pose],
  );

  const activeMachine = machineForU(pose.u);
  const seamGlow = boundaryIntensity(pose.u, 0.5, 0.075);
  const normalizedRatio = clamp((pose.v - MIN_V) / (MAX_V - MIN_V), 0, 1);
  const ratioDisplay = normalizedRatio.toFixed(2);

  return {
    stageRef,
    pose,
    activeMachine,
    seamGlow,
    ratioDisplay,
    mode,
    dragging,
    focusMachine,
    resumeAutoCycle,
    onStagePointerDown,
    onStagePointerMove,
    onStagePointerUp,
    onStagePointerCancel,
    onStageKeyDown,
  };
}
