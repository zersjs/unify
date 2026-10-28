import { afterEach, describe, expect, it, vi } from "vitest";
import { SpringAnimator } from "./motion";

describe("SpringAnimator", () => {
  afterEach(() => vi.unstubAllGlobals());

  it("applies initial and directly set values", () => {
    const applied: Array<Record<"x", number>> = [];
    const animator = new SpringAnimator({ x: 3 }, (values) => applied.push(values));
    animator.set({ x: 10 });
    expect(applied.map((v) => v.x)).toEqual([3, 10]);
    expect(animator.current()).toEqual({ x: 10 });
  });

  it("animates towards the target and settles exactly on it", () => {
    let now = 0;
    const queue: FrameRequestCallback[] = [];
    vi.stubGlobal("requestAnimationFrame", (cb: FrameRequestCallback) => queue.push(cb));
    vi.stubGlobal("cancelAnimationFrame", () => undefined);
    vi.spyOn(performance, "now").mockImplementation(() => now);
    const active: boolean[] = [];
    const animator = new SpringAnimator({ x: 0 }, () => undefined, (value) => active.push(value));
    animator.to({ x: 100 }, { response: 0.3 });
    let previous = 0;
    let monotonic = true;
    for (let frame = 0; frame < 240 && queue.length > 0; frame += 1) {
      now += 1000 / 120;
      const next = queue.shift();
      next?.(now);
      const x = animator.current().x;
      if (x < previous - 1e-6) monotonic = false;
      previous = x;
    }
    expect(animator.current().x).toBe(100);
    expect(monotonic).toBe(true);
    expect(active).toEqual([true, false]);
  });

  it("jumps to the target when reduced motion is preferred", () => {
    vi.stubGlobal("matchMedia", () => ({ matches: true }));
    const animator = new SpringAnimator({ x: 0 }, () => undefined);
    animator.to({ x: 42 });
    expect(animator.current().x).toBe(42);
  });
});
