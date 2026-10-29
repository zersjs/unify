export interface PointerSample {
  readonly t: number;
  readonly u: number;
  readonly v: number;
}

export interface SpringState {
  readonly displacement: number;
  readonly velocity: number;
}

export interface Vec2 {
  readonly u: number;
  readonly v: number;
}

export const STANDARD_DECELERATION = 0.998;
export const SNAPPY_DECELERATION = 0.99;
export const RUBBER_BAND_CONSTANT = 0.55;
export const VELOCITY_WINDOW_MS = 100;
export const MAX_NORMALIZED_SPEED = 4.5;
export const HYSTERESIS_PX = 10;

export function rubberBand(overshoot: number, dimension: number, constant = RUBBER_BAND_CONSTANT): number {
  if (dimension <= 0 || overshoot === 0) {
    return 0;
  }
  const absOvershoot = Math.abs(overshoot);
  const displaced = (absOvershoot * dimension * constant) / (dimension + constant * absOvershoot);
  return Math.sign(overshoot) * displaced;
}

export function constrainWithRubberBand(value: number, min: number, max: number, dimension: number): number {
  if (value < min) {
    return min + rubberBand(value - min, dimension);
  }
  if (value > max) {
    return max + rubberBand(value - max, dimension);
  }
  return value;
}

export function clamp(value: number, min: number, max: number): number {
  if (!Number.isFinite(value)) {
    return min;
  }
  return Math.min(max, Math.max(min, value));
}

export function projectRest(position: number, velocityPerSecond: number, deceleration = STANDARD_DECELERATION): number {
  return position + ((velocityPerSecond / 1000) * deceleration) / (1 - deceleration);
}

export function releaseVelocity(samples: readonly PointerSample[], windowMs = VELOCITY_WINDOW_MS): Vec2 {
  const last = samples[samples.length - 1];
  if (!last) {
    return { u: 0, v: 0 };
  }
  const first = samples.find((item) => last.t - item.t <= windowMs) ?? last;
  const dtSeconds = (last.t - first.t) / 1000;
  if (dtSeconds <= 0) {
    return { u: 0, v: 0 };
  }
  const rawU = (last.u - first.u) / dtSeconds;
  const rawV = (last.v - first.v) / dtSeconds;
  const speed = Math.hypot(rawU, rawV);
  if (speed <= MAX_NORMALIZED_SPEED) {
    return { u: rawU, v: rawV };
  }
  const scale = MAX_NORMALIZED_SPEED / speed;
  return { u: rawU * scale, v: rawV * scale };
}

export function springStep(
  displacement: number,
  velocity: number,
  responseSeconds: number,
  dampingRatio: number,
  elapsedSeconds: number,
): SpringState {
  const safeResponse = Math.max(0.05, responseSeconds);
  const omega = (2 * Math.PI) / safeResponse;
  if (dampingRatio >= 1) {
    const decay = Math.exp(-omega * elapsedSeconds);
    const c2 = velocity + omega * displacement;
    return {
      displacement: (displacement + c2 * elapsedSeconds) * decay,
      velocity: (velocity - omega * c2 * elapsedSeconds) * decay,
    };
  }
  const clampedRatio = Math.max(0.05, dampingRatio);
  const dampedOmega = omega * Math.sqrt(1 - clampedRatio * clampedRatio);
  const decay = Math.exp(-clampedRatio * omega * elapsedSeconds);
  const cosTerm = Math.cos(dampedOmega * elapsedSeconds);
  const sinTerm = Math.sin(dampedOmega * elapsedSeconds);
  const bTerm = (velocity + clampedRatio * omega * displacement) / dampedOmega;
  return {
    displacement: decay * (displacement * cosTerm + bTerm * sinTerm),
    velocity:
      decay *
      (velocity * cosTerm - ((clampedRatio * omega * velocity + omega * omega * displacement) / dampedOmega) * sinTerm),
  };
}

export function isSettled(state: SpringState, epsilonDistance = 0.0008, epsilonVelocity = 0.008): boolean {
  return Math.abs(state.displacement) < epsilonDistance && Math.abs(state.velocity) < epsilonVelocity;
}

export function boundaryIntensity(u: number, seamCenter = 0.5, sigma = 0.065): number {
  const delta = (u - seamCenter) / sigma;
  return Math.exp(-0.5 * delta * delta);
}
