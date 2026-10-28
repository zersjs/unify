import { useCallback, useEffect, useRef, useState, type KeyboardEvent, type PointerEvent } from "react";
import type { Side } from "@/bridge";
import {
  constrain,
  isSettled,
  nearestSide,
  projectRest,
  releaseVelocity,
  sideForArrowKey,
  slotPositions,
  springStep,
  type Sample,
  type Size,
  type Vec,
} from "./layoutPhysics";

export const SCREEN_SIZE: Size = { width: 148, height: 92 };
export const SLOT_GAP = 12;
const RESPONSE_SECONDS = 0.4;
const FLICK_SPEED_PX_S = 600;
const FLICK_DAMPING = 0.8;
const MAX_SAMPLES = 12;

function prefersReducedMotion(): boolean {
  return typeof window !== "undefined" && typeof window.matchMedia === "function" && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

export function useLayoutDragViewModel(side: Side, onChange: (side: Side) => void) {
  const slots = slotPositions(SCREEN_SIZE, SCREEN_SIZE, SLOT_GAP);
  const containerRef = useRef<HTMLDivElement>(null);
  const peerRef = useRef<HTMLDivElement>(null);
  const position = useRef<Vec>({ ...slots[side] });
  const velocity = useRef<Vec>({ x: 0, y: 0 });
  const frame = useRef<number | null>(null);
  const grab = useRef<Vec>({ x: 0, y: 0 });
  const samples = useRef<Sample[]>([]);
  const pointerId = useRef<number | null>(null);
  const committedSide = useRef<Side>(side);
  const [dragging, setDragging] = useState(false);
  const [pressed, setPressed] = useState(false);
  const [candidate, setCandidate] = useState<Side>(side);

  const render = useCallback(() => {
    const element = peerRef.current;
    if (element) {
      element.style.transform = `translate(-50%, -50%) translate(${position.current.x}px, ${position.current.y}px)`;
    }
  }, []);

  const stopAnimation = useCallback(() => {
    if (frame.current !== null) {
      cancelAnimationFrame(frame.current);
      frame.current = null;
    }
    peerRef.current?.style.removeProperty("will-change");
  }, []);

  const animateTo = useCallback(
    (target: Vec, initialVelocity: Vec, damping: number) => {
      stopAnimation();
      if (prefersReducedMotion()) {
        position.current = { ...target };
        velocity.current = { x: 0, y: 0 };
        render();
        return;
      }
      const start = { ...position.current };
      const startedAt = performance.now();
      if (peerRef.current) {
        peerRef.current.style.willChange = "transform";
      }
      const tick = (now: number) => {
        const t = (now - startedAt) / 1000;
        const x = springStep(start.x - target.x, initialVelocity.x, RESPONSE_SECONDS, damping, t);
        const y = springStep(start.y - target.y, initialVelocity.y, RESPONSE_SECONDS, damping, t);
        position.current = { x: target.x + x.displacement, y: target.y + y.displacement };
        velocity.current = { x: x.velocity, y: y.velocity };
        render();
        if (isSettled(x) && isSettled(y)) {
          position.current = { ...target };
          velocity.current = { x: 0, y: 0 };
          render();
          stopAnimation();
          return;
        }
        frame.current = requestAnimationFrame(tick);
      };
      frame.current = requestAnimationFrame(tick);
    },
    [render, stopAnimation],
  );

  useEffect(() => {
    render();
    return stopAnimation;
  }, [render, stopAnimation]);

  useEffect(() => {
    committedSide.current = side;
    setCandidate(side);
    if (pointerId.current === null) {
      animateTo(slots[side], velocity.current, 1);
    }
  }, [side]);

  const localPoint = (event: PointerEvent<HTMLDivElement>): Vec => {
    const rect = containerRef.current?.getBoundingClientRect();
    if (!rect) {
      return { x: 0, y: 0 };
    }
    return { x: event.clientX - (rect.left + rect.width / 2), y: event.clientY - (rect.top + rect.height / 2) };
  };

  const onPointerDown = (event: PointerEvent<HTMLDivElement>) => {
    if (event.button !== 0) {
      return;
    }
    stopAnimation();
    event.currentTarget.setPointerCapture?.(event.pointerId);
    pointerId.current = event.pointerId;
    const point = localPoint(event);
    grab.current = { x: point.x - position.current.x, y: point.y - position.current.y };
    samples.current = [{ t: event.timeStamp, x: position.current.x, y: position.current.y }];
    if (peerRef.current) {
      peerRef.current.style.willChange = "transform";
    }
    setPressed(true);
    setDragging(true);
  };

  const onPointerMove = (event: PointerEvent<HTMLDivElement>) => {
    if (pointerId.current !== event.pointerId) {
      return;
    }
    const rect = containerRef.current?.getBoundingClientRect();
    const halfWidth = rect ? rect.width / 2 - SCREEN_SIZE.width / 2 : Number.POSITIVE_INFINITY;
    const halfHeight = rect ? rect.height / 2 - SCREEN_SIZE.height / 2 : Number.POSITIVE_INFINITY;
    const point = localPoint(event);
    const next = {
      x: constrain(point.x - grab.current.x, -halfWidth, halfWidth, rect?.width ?? 1),
      y: constrain(point.y - grab.current.y, -halfHeight, halfHeight, rect?.height ?? 1),
    };
    position.current = next;
    samples.current = [...samples.current.slice(-(MAX_SAMPLES - 1)), { t: event.timeStamp, x: next.x, y: next.y }];
    render();
    setCandidate(nearestSide(next, slots));
  };

  const release = (event: PointerEvent<HTMLDivElement>, cancelled: boolean) => {
    if (pointerId.current !== event.pointerId) {
      return;
    }
    pointerId.current = null;
    samples.current = [...samples.current, { t: event.timeStamp, x: position.current.x, y: position.current.y }];
    setPressed(false);
    setDragging(false);
    const handoff = cancelled ? { x: 0, y: 0 } : releaseVelocity(samples.current);
    const projected = { x: projectRest(position.current.x, handoff.x), y: projectRest(position.current.y, handoff.y) };
    const target = cancelled ? committedSide.current : nearestSide(projected, slots);
    const speed = Math.hypot(handoff.x, handoff.y);
    setCandidate(target);
    animateTo(slots[target], handoff, speed > FLICK_SPEED_PX_S ? FLICK_DAMPING : 1);
    if (target !== committedSide.current) {
      committedSide.current = target;
      onChange(target);
    }
  };

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const next = sideForArrowKey(event.key);
    if (!next) {
      return;
    }
    event.preventDefault();
    if (next !== committedSide.current) {
      committedSide.current = next;
      setCandidate(next);
      animateTo(slots[next], { x: 0, y: 0 }, 1);
      onChange(next);
    }
  };

  return {
    containerRef,
    peerRef,
    slots,
    dragging,
    pressed,
    candidate,
    handlers: {
      onPointerDown,
      onPointerMove,
      onPointerUp: (event: PointerEvent<HTMLDivElement>) => release(event, false),
      onPointerCancel: (event: PointerEvent<HTMLDivElement>) => release(event, true),
      onKeyDown,
    },
  };
}
