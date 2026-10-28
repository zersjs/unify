import { describe, expect, it } from "vitest";
import {
  constrain,
  isSettled,
  nearestSide,
  projectRest,
  releaseVelocity,
  rubberBand,
  sideForArrowKey,
  slotPositions,
  springStep,
} from "./layoutPhysics";

const slots = slotPositions({ width: 148, height: 92 }, { width: 148, height: 92 }, 12);

describe("layout physics", () => {
  it("places slots symmetrically around the local screen", () => {
    expect(slots.Right).toEqual({ x: 160, y: 0 });
    expect(slots.Left).toEqual({ x: -160, y: 0 });
    expect(slots.Top).toEqual({ x: 0, y: -104 });
    expect(slots.Bottom).toEqual({ x: 0, y: 104 });
  });

  it("snaps to the slot nearest the given point", () => {
    expect(nearestSide({ x: 120, y: 10 }, slots)).toBe("Right");
    expect(nearestSide({ x: -10, y: -90 }, slots)).toBe("Top");
    expect(nearestSide({ x: -200, y: 40 }, slots)).toBe("Left");
  });

  it("projects a flick towards a different slot than the release point", () => {
    const releasedAt = { x: 150, y: 0 };
    expect(nearestSide(releasedAt, slots)).toBe("Right");
    const projected = { x: projectRest(releasedAt.x, -100), y: projectRest(releasedAt.y, 900) };
    expect(nearestSide(projected, slots)).toBe("Bottom");
  });

  it("applies progressive rubber-band resistance", () => {
    expect(rubberBand(0, 300)).toBe(0);
    const small = rubberBand(20, 300);
    const large = rubberBand(400, 300);
    expect(small).toBeLessThan(20);
    expect(large).toBeLessThan(300 / 0.55);
    expect(rubberBand(-20, 300)).toBeCloseTo(-small);
    expect(constrain(50, -100, 100, 300)).toBe(50);
    expect(constrain(140, -100, 100, 300)).toBeGreaterThan(100);
    expect(constrain(140, -100, 100, 300)).toBeLessThan(140);
  });

  it("derives release velocity from recent samples only", () => {
    const samples = [
      { t: 0, x: 0, y: 0 },
      { t: 500, x: 0, y: 0 },
      { t: 550, x: 50, y: -25 },
      { t: 600, x: 100, y: -50 },
    ];
    const velocity = releaseVelocity(samples);
    expect(velocity.x).toBeCloseTo(1000);
    expect(velocity.y).toBeCloseTo(-500);
    expect(releaseVelocity([])).toEqual({ x: 0, y: 0 });
  });

  it("reports zero velocity when the pointer rested before release", () => {
    const samples = [
      { t: 0, x: 0, y: 0 },
      { t: 16, x: 40, y: 0 },
      { t: 600, x: 40, y: 0 },
    ];
    expect(releaseVelocity(samples)).toEqual({ x: 0, y: 0 });
  });

  it("caps release speed", () => {
    const velocity = releaseVelocity([
      { t: 0, x: 0, y: 0 },
      { t: 1, x: 300, y: 400 },
    ]);
    expect(Math.hypot(velocity.x, velocity.y)).toBeCloseTo(5000);
    expect(velocity.x / velocity.y).toBeCloseTo(0.75);
  });

  it("critically damped spring settles without overshoot and keeps handed-off velocity", () => {
    let state = { displacement: 100, velocity: 0 };
    let crossed = false;
    for (let t = 0; t < 2; t += 1 / 120) {
      const next = springStep(100, 0, 0.4, 1, t);
      if (next.displacement < -0.01) crossed = true;
      state = next;
    }
    expect(crossed).toBe(false);
    expect(isSettled(state)).toBe(true);
    const start = springStep(0, 800, 0.4, 1, 0);
    expect(start.velocity).toBe(800);
  });

  it("under-damped spring overshoots and still settles", () => {
    let minimum = Number.POSITIVE_INFINITY;
    for (let t = 0; t < 1; t += 1 / 120) {
      minimum = Math.min(minimum, springStep(100, 0, 0.3, 0.8, t).displacement);
    }
    expect(minimum).toBeLessThan(0);
    expect(isSettled(springStep(100, 0, 0.3, 0.8, 3))).toBe(true);
  });

  it("maps arrow keys to sides", () => {
    expect(sideForArrowKey("ArrowLeft")).toBe("Left");
    expect(sideForArrowKey("ArrowDown")).toBe("Bottom");
    expect(sideForArrowKey("Enter")).toBeNull();
  });
});
