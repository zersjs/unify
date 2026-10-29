import { describe, expect, it } from "vitest";
import { en } from "./en";
import { tr } from "./tr";

function collectLeafEntries(node: unknown, prefix = ""): Array<{ path: string; value: string }> {
  if (typeof node === "string") {
    return [{ path: prefix, value: node }];
  }
  if (Array.isArray(node)) {
    return node.flatMap((item, index) => collectLeafEntries(item, `${prefix}[${index}]`));
  }
  if (node && typeof node === "object") {
    return Object.entries(node).flatMap(([key, val]) =>
      collectLeafEntries(val, prefix ? `${prefix}.${key}` : key),
    );
  }
  return [];
}

const BANNED_EN_TOKENS = [
  "delve",
  "tapestry",
  "supercharge",
  "unleash",
  "game-changer",
  "revolutionary",
  "cutting-edge",
  "state-of-the-art",
  "synergy",
  "best-in-class",
];

const BANNED_TR_PHRASES = [
  "günümüz dünyasında",
  "dijital çağda",
  "giderek önem kazanan",
  "önemli bir rol oynamaktadır",
  "dikkat çekici bir şekilde",
  "şüphesiz",
  "sunduğu benzersiz deneyim",
  "geleceğe ışık tutmaktadır",
];

describe("localization tables and anti-slop copy invariants", () => {
  it("defines identical leaf paths in English and Turkish with non-empty values", () => {
    const enLeaves = collectLeafEntries(en);
    const trLeaves = collectLeafEntries(tr);
    expect(trLeaves.map((item) => item.path).sort()).toEqual(enLeaves.map((item) => item.path).sort());
    for (const item of [...enLeaves, ...trLeaves]) {
      expect(item.value.trim().length).toBeGreaterThan(0);
    }
  });

  it("formats all eyebrows in natural sentence case rather than all-caps", () => {
    const eyebrows = [
      en.hero.eyebrow,
      en.seam.eyebrow,
      en.clipboard.eyebrow,
      en.security.eyebrow,
      en.openSource.eyebrow,
      en.platforms.eyebrow,
      en.earlyAccess.eyebrow,
      tr.hero.eyebrow,
      tr.seam.eyebrow,
      tr.clipboard.eyebrow,
      tr.security.eyebrow,
      tr.openSource.eyebrow,
      tr.platforms.eyebrow,
      tr.earlyAccess.eyebrow,
    ];
    for (const eyebrow of eyebrows) {
      expect(eyebrow).not.toBe(eyebrow.toUpperCase());
    }
  });

  it("contains zero banned promotional clichés in English and Turkish copy", () => {
    const allEn = collectLeafEntries(en)
      .map((item) => item.value.toLowerCase())
      .join(" ");
    const allTr = collectLeafEntries(tr)
      .map((item) => item.value.toLowerCase())
      .join(" ");

    for (const token of BANNED_EN_TOKENS) {
      expect(allEn).not.toContain(token);
    }
    for (const phrase of BANNED_TR_PHRASES) {
      expect(allTr).not.toContain(phrase);
    }
  });
});
