import { isSettled, springStep } from "./physics";

export interface SpringOptions<K extends string> {
  readonly response?: number;
  readonly damping?: number;
  readonly epsilonDistance?: number;
  readonly epsilonVelocity?: number;
  readonly initialVelocity?: Partial<Record<K, number>>;
}

export function prefersReducedMotion(): boolean {
  return (
    typeof window !== "undefined" &&
    typeof window.matchMedia === "function" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches
  );
}

export class SpringAnimator<K extends string> {
  private values: Record<K, number>;
  private velocities: Record<K, number>;
  private frameId: number | null = null;

  constructor(
    initial: Record<K, number>,
    private readonly apply: (values: Record<K, number>) => void,
    private readonly onActiveChange?: (active: boolean) => void,
  ) {
    this.values = { ...initial };
    this.velocities = Object.fromEntries(Object.keys(initial).map((key) => [key, 0])) as Record<K, number>;
    this.apply(this.values);
  }

  current(): Record<K, number> {
    return { ...this.values };
  }

  velocity(): Record<K, number> {
    return { ...this.velocities };
  }

  set(nextValues: Record<K, number>): void {
    this.stop();
    this.values = { ...nextValues };
    for (const key of Object.keys(this.velocities) as K[]) {
      this.velocities[key] = 0;
    }
    this.apply(this.values);
  }

  to(target: Record<K, number>, options: SpringOptions<K> = {}): void {
    const {
      response = 0.4,
      damping = 1.0,
      epsilonDistance = 0.0008,
      epsilonVelocity = 0.008,
      initialVelocity,
    } = options;

    if (prefersReducedMotion() || typeof requestAnimationFrame !== "function") {
      this.set(target);
      return;
    }

    this.stop();
    const keys = Object.keys(target) as K[];
    const startValues = { ...this.values };
    const startVelocities = { ...this.velocities };
    if (initialVelocity) {
      for (const key of keys) {
        const handedOff = initialVelocity[key];
        if (typeof handedOff === "number" && Number.isFinite(handedOff)) {
          startVelocities[key] = handedOff;
        }
      }
    }

    const startedAt = performance.now();
    this.onActiveChange?.(true);

    const stepFrame = (now: number) => {
      const elapsedSeconds = Math.max(0, (now - startedAt) / 1000);
      let allSettled = true;
      for (const key of keys) {
        const currentStart = startValues[key];
        const currentTarget = target[key];
        const state = springStep(
          currentStart - currentTarget,
          startVelocities[key],
          response,
          damping,
          elapsedSeconds,
        );
        this.values[key] = currentTarget + state.displacement;
        this.velocities[key] = state.velocity;
        allSettled = allSettled && isSettled(state, epsilonDistance, epsilonVelocity);
      }
      if (allSettled) {
        this.values = { ...this.values, ...target };
        for (const key of keys) {
          this.velocities[key] = 0;
        }
        this.apply(this.values);
        this.frameId = null;
        this.onActiveChange?.(false);
        return;
      }
      this.apply(this.values);
      this.frameId = requestAnimationFrame(stepFrame);
    };

    this.frameId = requestAnimationFrame(stepFrame);
  }

  stop(): void {
    if (this.frameId !== null) {
      cancelAnimationFrame(this.frameId);
      this.frameId = null;
      this.onActiveChange?.(false);
    }
  }
}
