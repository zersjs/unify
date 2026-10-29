import { describe, expect, it } from "vitest";
import {
  boundaryIntensity,
  constrainWithRubberBand,
  isSettled,
  MAX_NORMALIZED_SPEED,
  projectRest,
  releaseVelocity,
  rubberBand,
  springStep,
} from "./physics";

describe("physical motion primitives", () => {
  it("converges monotonically without overshoot when critically damped", () => {
    const early = springStep(1, 0, 0.4, 1.0, 0.1);
    const mid = springStep(1, 0, 0.4, 1.0, 0.25);
    const late = springStep(1, 0, 0.4, 1.0, 0.9);
    expect(early.displacement).toBeGreaterThan(mid.displacement);
    expect(mid.displacement).toBeGreaterThan(0);
    expect(isSettled(late)).toBe(true);
  });

  it("Crosses zero once when under-damped with momentum", () => {
    const step = springStep(1, -8, 0.35, 0.8, 0.18);
    expect(step.displacement).toBeLessThan(0.25);
  });

  it("applies progressive 0.55 rubber-band resistance bounded by the dimension", () => {
    const small = rubberBand(0.2, 1.0);
    const large = rubberBand(10.0, 1.0);
    expect(small).toBeCloseTo((0.2 * 1.0 * 0.55) / (1.0 + 0.55 * 0.2), 6);
    expect(large).toBeLessThan(1.0);
    expect(constrainWithRubberBand(-0.1, 0.08, 0.92, 0.32)).toBeLessThan(0.08);
    expect(constrainWithRubberBand(-0.1, 0.08, 0.92, 0.32)).toBeGreaterThan(-0.1);
  });

  it("projects resting position along velocity vector", () => {
    expect(projectRest(0.5, 2.0)).toBeGreaterThan(0.5);
    expect(projectRest(0.5, -2.0)).toBeLessThan(0.5);
  });

  it("caps release velocity to MAX_NORMALIZED_SPEED and handles sparse samples", () => {
    expect(releaseVelocity([])).toEqual({ u: 0, v: 0 });
    expect(releaseVelocity([{ t: 10, u: 0.4, v: 0.5 }])).toEqual({ u: 0, v: 0 });
    const fast = releaseVelocity([
      { t: 0, u: 0, v: 0 },
      { t: 10, u: 0.9, v: 0.9 },
    ]);
    expect(Math.hypot(fast.u, fast.v)).toBeCloseTo(MAX_NORMALIZED_SPEED, 5);
  });

  it("peaks boundary intensity at the shared screen edge", () => {
    expect(boundaryIntensity(0.5)).toBeCloseTo(1, 6);
    expect(boundaryIntensity(0.2)).toBeLessThan(boundaryIntensity(0.46));
  });
});
