import type { Side } from "@/bridge";

export interface Vec {
  x: number;
  y: number;
}

export interface Size {
  width: number;
  height: number;
}

export interface Sample {
  t: number;
  x: number;
  y: number;
}

export const SIDES: Side[] = ["Left", "Right", "Top", "Bottom"];
export const STANDARD_DECELERATION = 0.998;
export const RUBBER_BAND_CONSTANT = 0.55;
export const VELOCITY_WINDOW_MS = 100;
export const MAX_RELEASE_SPEED_PX_S = 5000;
export const SETTLE_DISTANCE_PX = 0.5;
export const SETTLE_VELOCITY_PX_S = 5;

export function slotPositions(local: Size, peer: Size, gap: number): Record<Side, Vec> {
  const horizontal = local.width / 2 + gap + peer.width / 2;
  const vertical = local.height / 2 + gap + peer.height / 2;
  return {
    Left: { x: -horizontal, y: 0 },
    Right: { x: horizontal, y: 0 },
    Top: { x: 0, y: -vertical },
    Bottom: { x: 0, y: vertical },
  };
}

export function nearestSide(point: Vec, slots: Record<Side, Vec>): Side {
  let best: Side = "Right";
  let bestDistance = Number.POSITIVE_INFINITY;
  for (const side of SIDES) {
    const slot = slots[side];
    const distance = (point.x - slot.x) ** 2 + (point.y - slot.y) ** 2;
    if (distance < bestDistance) {
      bestDistance = distance;
      best = side;
    }
  }
  return best;
}

export function projectRest(position: number, velocityPxPerSecond: number, deceleration = STANDARD_DECELERATION): number {
  return position + ((velocityPxPerSecond / 1000) * deceleration) / (1 - deceleration);
}

export function rubberBand(overshoot: number, dimension: number, constant = RUBBER_BAND_CONSTANT): number {
  if (dimension <= 0) {
    return 0;
  }
  const magnitude = (Math.abs(overshoot) * dimension * constant) / (dimension + constant * Math.abs(overshoot));
  return Math.sign(overshoot) * magnitude;
}

export function constrain(value: number, min: number, max: number, dimension: number): number {
  if (value < min) {
    return min + rubberBand(value - min, dimension);
  }
  if (value > max) {
    return max + rubberBand(value - max, dimension);
  }
  return value;
}

export function releaseVelocity(samples: Sample[], windowMs = VELOCITY_WINDOW_MS): Vec {
  const last = samples[samples.length - 1];
  if (!last) {
    return { x: 0, y: 0 };
  }
  const first = samples.find((sample) => last.t - sample.t <= windowMs) ?? last;
  const elapsed = (last.t - first.t) / 1000;
  if (elapsed <= 0) {
    return { x: 0, y: 0 };
  }
  const raw = { x: (last.x - first.x) / elapsed, y: (last.y - first.y) / elapsed };
  const speed = Math.hypot(raw.x, raw.y);
  if (speed <= MAX_RELEASE_SPEED_PX_S) {
    return raw;
  }
  const scale = MAX_RELEASE_SPEED_PX_S / speed;
  return { x: raw.x * scale, y: raw.y * scale };
}

export interface SpringState {
  displacement: number;
  velocity: number;
}

export function springStep(displacement: number, velocity: number, responseSeconds: number, dampingRatio: number, t: number): SpringState {
  const omega = (2 * Math.PI) / responseSeconds;
  if (dampingRatio >= 1) {
    const decay = Math.exp(-omega * t);
    const coefficient = velocity + omega * displacement;
    return {
      displacement: (displacement + coefficient * t) * decay,
      velocity: (velocity - omega * coefficient * t) * decay,
    };
  }
  const damped = omega * Math.sqrt(1 - dampingRatio * dampingRatio);
  const decay = Math.exp(-dampingRatio * omega * t);
  const cos = Math.cos(damped * t);
  const sin = Math.sin(damped * t);
  return {
    displacement: decay * (displacement * cos + ((velocity + dampingRatio * omega * displacement) / damped) * sin),
    velocity: decay * (velocity * cos - ((dampingRatio * omega * velocity + omega * omega * displacement) / damped) * sin),
  };
}

export function isSettled(state: SpringState): boolean {
  return Math.abs(state.displacement) < SETTLE_DISTANCE_PX && Math.abs(state.velocity) < SETTLE_VELOCITY_PX_S;
}

export function sideForArrowKey(key: string): Side | null {
  switch (key) {
    case "ArrowLeft":
      return "Left";
    case "ArrowRight":
      return "Right";
    case "ArrowUp":
      return "Top";
    case "ArrowDown":
      return "Bottom";
    default:
      return null;
  }
}
