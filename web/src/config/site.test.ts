import { describe, expect, it } from "vitest";
import {
  isExternalHttpUrl,
  localizedDocsUrl,
  resolveSiteConfig,
  sanitizeDocsUrl,
  sanitizeHttpUrl,
  sanitizeOptionalCanonicalUrl,
} from "./site";

describe("site configuration validation", () => {
  it("falls back to the verified GitHub repository, integrated /docs/ path, and null canonicalUrl when env is empty", () => {
    const config = resolveSiteConfig({});
    expect(config.repositoryUrl).toBe("https://github.com/zersjs/unify");
    expect(config.documentationUrl).toBe("/docs/");
    expect(config.licenseUrl).toBe("https://github.com/zersjs/unify/blob/main/LICENSE");
    expect(config.canonicalUrl).toBeNull();
  });

  it("derives subpath documentation route automatically from BASE_URL on GitHub Pages", () => {
    const config = resolveSiteConfig({ BASE_URL: "/unify/" });
    expect(config.documentationUrl).toBe("/unify/docs/");
    expect(localizedDocsUrl(config.documentationUrl, "tr")).toBe("/unify/docs/tr/");
  });

  it("localizes integrated /docs/ routes for Turkish while leaving external URLs untouched", () => {
    expect(localizedDocsUrl("/docs/", "en")).toBe("/docs/");
    expect(localizedDocsUrl("/docs/", "tr")).toBe("/docs/tr/");
    expect(localizedDocsUrl("/unify/docs/", "tr")).toBe("/unify/docs/tr/");
    expect(localizedDocsUrl("https://docs.example.org", "tr")).toBe("https://docs.example.org");
    expect(isExternalHttpUrl("/docs/")).toBe(false);
    expect(isExternalHttpUrl("https://docs.example.org")).toBe(true);
  });

  it("derives license and issues paths when a custom repository URL is supplied", () => {
    const config = resolveSiteConfig({
      VITE_REPO_URL: "https://example.org/org/unify/",
      VITE_DOCS_URL: "/unify/docs",
    });
    expect(config.repositoryUrl).toBe("https://example.org/org/unify");
    expect(config.documentationUrl).toBe("/unify/docs/");
    expect(config.licenseUrl).toBe("https://example.org/org/unify/blob/main/LICENSE");
  });

  it("rejects non-http protocols, protocol-relative open redirects, and malformed URLs", () => {
    expect(sanitizeHttpUrl("javascript:alert(1)", "https://github.com/zersjs/unify")).toBe(
      "https://github.com/zersjs/unify",
    );
    expect(sanitizeHttpUrl("not a url", "https://github.com/zersjs/unify")).toBe("https://github.com/zersjs/unify");
    expect(sanitizeDocsUrl("javascript:alert(1)")).toBe("/docs/");
    expect(sanitizeDocsUrl("//evil.example.org/docs")).toBe("/docs/");
    expect(sanitizeDocsUrl("/\\evil.example.org")).toBe("/docs/");
    expect(sanitizeOptionalCanonicalUrl("javascript:void(0)")).toBeNull();
    expect(sanitizeOptionalCanonicalUrl("   ")).toBeNull();
  });

  it("normalizes a valid canonical URL when explicitly configured", () => {
    const config = resolveSiteConfig({ VITE_SITE_URL: "https://unify.example.org/" });
    expect(config.canonicalUrl).toBe("https://unify.example.org");
  });
});
