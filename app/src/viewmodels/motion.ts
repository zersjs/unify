import { isSettled, springStep } from "./layoutPhysics";

export interface SpringOptions {
  response?: number;
  damping?: number;
}

export function prefersReducedMotion(): boolean {
  return typeof window !== "undefined" && typeof window.matchMedia === "function" && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

export class SpringAnimator<K extends string> {
  private values: Record<K, number>;
  private velocities: Record<K, number>;
  private frame: number | null = null;

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

  set(values: Record<K, number>): void {
    this.stop();
    this.values = { ...values };
    for (const key of Object.keys(this.velocities) as K[]) {
      this.velocities[key] = 0;
    }
    this.apply(this.values);
  }

  to(target: Record<K, number>, { response = 0.35, damping = 1 }: SpringOptions = {}): void {
    if (prefersReducedMotion() || typeof requestAnimationFrame !== "function") {
      this.set(target);
      return;
    }
    this.stop();
    const keys = Object.keys(target) as K[];
    const start = { ...this.values };
    const initialVelocity = { ...this.velocities };
    const startedAt = performance.now();
    this.onActiveChange?.(true);
    const tick = (now: number) => {
      const t = (now - startedAt) / 1000;
      let settled = true;
      for (const key of keys) {
        const state = springStep(start[key] - target[key], initialVelocity[key], response, damping, t);
        this.values[key] = target[key] + state.displacement;
        this.velocities[key] = state.velocity;
        settled = settled && isSettled(state);
      }
      if (settled) {
        this.values = { ...this.values, ...target };
        for (const key of keys) {
          this.velocities[key] = 0;
        }
        this.apply(this.values);
        this.frame = null;
        this.onActiveChange?.(false);
        return;
      }
      this.apply(this.values);
      this.frame = requestAnimationFrame(tick);
    };
    this.frame = requestAnimationFrame(tick);
  }

  stop(): void {
    if (this.frame !== null) {
      cancelAnimationFrame(this.frame);
      this.frame = null;
      this.onActiveChange?.(false);
    }
  }
}
