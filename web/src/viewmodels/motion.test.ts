import { afterEach, describe, expect, it, vi } from "vitest";
import { SpringAnimator } from "./motion";

describe("SpringAnimator", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("settles immediately without scheduling animation frames when reduced motion is preferred", () => {
    vi.stubGlobal("matchMedia", () => ({ matches: true }));
    const states: Array<{ x: number }> = [];
    const animator = new SpringAnimator({ x: 0 }, (values) => states.push({ ...values }));
    animator.to({ x: 1 });
    expect(states[states.length - 1]).toEqual({ x: 1 });
  });

  it("preserves live position and velocity across mid-flight redirection", () => {
    let clock = 0;
    let queuedFrame: FrameRequestCallback | null = null;
    vi.stubGlobal("matchMedia", () => ({ matches: false }));
    vi.stubGlobal("performance", { now: () => clock });
    vi.stubGlobal("requestAnimationFrame", (cb: FrameRequestCallback) => {
      queuedFrame = cb;
      return 1;
    });
    vi.stubGlobal("cancelAnimationFrame", () => {
      queuedFrame = null;
    });

    const activeFlags: boolean[] = [];
    const animator = new SpringAnimator(
      { x: 0 },
      () => undefined,
      (active) => activeFlags.push(active),
    );

    animator.to({ x: 1 }, { response: 0.4, damping: 1.0 });
    expect(activeFlags).toContain(true);
    clock = 100;
    const firstTick = queuedFrame as FrameRequestCallback | null;
    if (firstTick) {
      firstTick(100);
    }
    const midValue = animator.current().x;
    expect(midValue).toBeGreaterThan(0);
    expect(midValue).toBeLessThan(1);

    animator.to({ x: 0.2 }, { response: 0.4, damping: 1.0 });
    expect(animator.current().x).toBeCloseTo(midValue, 6);
    animator.stop();
  });
});
