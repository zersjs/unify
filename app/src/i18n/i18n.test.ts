import { describe, expect, it } from "vitest";
import { createI18n } from "./index";
import { en } from "./en";
import { tr } from "./tr";

function leafPaths(value: unknown, prefix = ""): string[] {
  if (value && typeof value === "object") {
    return Object.entries(value).flatMap(([key, child]) => leafPaths(child, prefix ? `${prefix}.${key}` : key));
  }
  return [prefix];
}

describe("string tables", () => {
  it("define the same keys in English and Turkish", () => {
    expect(leafPaths(tr).sort()).toEqual(leafPaths(en).sort());
  });

  it("contain no empty strings", () => {
    for (const table of [en, tr]) {
      for (const path of leafPaths(table)) {
        const value = path.split(".").reduce<unknown>((node, key) => (node as Record<string, unknown>)[key], table);
        if (typeof value === "string") {
          expect(value.trim().length, path).toBeGreaterThan(0);
        }
      }
    }
  });

  it("falls back to the generic message for unknown error codes", () => {
    expect(createI18n("en").errorMessage("no_such_code")).toBe(en.errors.unknown);
    expect(createI18n("tr").errorMessage("peer_timeout")).toBe(tr.errors.peer_timeout);
  });

  it("formats times per locale", () => {
    const at = Date.UTC(2026, 0, 2, 3, 4, 5);
    expect(createI18n("en").formatTime(at)).toMatch(/\d{2}:\d{2}:\d{2}/);
    expect(createI18n("tr").formatTime(at)).toMatch(/\d{2}:\d{2}:\d{2}/);
  });
});
