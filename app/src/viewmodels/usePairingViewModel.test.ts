import { describe, expect, it } from "vitest";
import { normalizeCodeInput, secondsUntil } from "./usePairingViewModel";

describe("pairing input helpers", () => {
  it("keeps only six digits from pasted codes", () => {
    expect(normalizeCodeInput("123 456")).toBe("123456");
    expect(normalizeCodeInput("12-34-56-78")).toBe("123456");
    expect(normalizeCodeInput("abc")).toBe("");
  });

  it("never reports negative remaining time", () => {
    expect(secondsUntil(10_000, 0)).toBe(10);
    expect(secondsUntil(10_001, 0)).toBe(11);
    expect(secondsUntil(0, 5_000)).toBe(0);
  });
});
