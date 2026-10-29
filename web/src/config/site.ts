export interface SiteConfig {
  readonly repositoryUrl: string;
  readonly documentationUrl: string;
  readonly licenseUrl: string;
  readonly issuesUrl: string;
  readonly canonicalUrl: string | null;
}

export interface RawSiteEnv {
  readonly BASE_URL?: string;
  readonly VITE_REPO_URL?: string;
  readonly VITE_DOCS_URL?: string;
  readonly VITE_LICENSE_URL?: string;
  readonly VITE_ISSUES_URL?: string;
  readonly VITE_SITE_URL?: string;
}

const DEFAULT_REPO_URL = "https://github.com/zersjs/unify";
const DEFAULT_DOCS_URL = "/docs/";

function hasControlOrWhitespace(value: string): boolean {
  for (let i = 0; i < value.length; i += 1) {
    const code = value.charCodeAt(i);
    if (code <= 32 || code === 127) {
      return true;
    }
  }
  return false;
}

function defaultDocsPathFromBase(baseUrl: string | undefined): string {
  const trimmed = baseUrl?.trim();
  if (!trimmed || trimmed === "/") {
    return DEFAULT_DOCS_URL;
  }
  if (!trimmed.startsWith("/") || trimmed.startsWith("//") || hasControlOrWhitespace(trimmed)) {
    return DEFAULT_DOCS_URL;
  }
  const withSlash = trimmed.endsWith("/") ? trimmed : `${trimmed}/`;
  return `${withSlash}docs/`;
}

export function sanitizeHttpUrl(candidate: string | undefined, fallback: string): string {
  const trimmed = candidate?.trim();
  if (!trimmed) {
    return fallback;
  }
  try {
    const parsed = new URL(trimmed);
    if (parsed.protocol === "https:" || parsed.protocol === "http:") {
      return parsed.toString().replace(/\/$/, "");
    }
    return fallback;
  } catch {
    return fallback;
  }
}

export function sanitizeDocsUrl(candidate: string | undefined, fallback: string = DEFAULT_DOCS_URL): string {
  const trimmed = candidate?.trim();
  if (!trimmed) {
    return fallback;
  }
  if (trimmed.startsWith("/") && !trimmed.startsWith("//") && !trimmed.startsWith("/\\") && !hasControlOrWhitespace(trimmed)) {
    if (trimmed.endsWith("/") || trimmed.endsWith(".html")) {
      return trimmed;
    }
    return `${trimmed}/`;
  }
  try {
    const parsed = new URL(trimmed);
    if (parsed.protocol === "https:" || parsed.protocol === "http:") {
      return parsed.toString().replace(/\/$/, "");
    }
    return fallback;
  } catch {
    return fallback;
  }
}

export function isExternalHttpUrl(url: string): boolean {
  return url.startsWith("https://") || url.startsWith("http://");
}

export function localizedDocsUrl(documentationUrl: string, locale: "en" | "tr"): string {
  if (locale === "tr" && !isExternalHttpUrl(documentationUrl) && documentationUrl.endsWith("/docs/")) {
    return `${documentationUrl}tr/`;
  }
  return documentationUrl;
}

export function sanitizeOptionalCanonicalUrl(candidate: string | undefined): string | null {
  const trimmed = candidate?.trim();
  if (!trimmed) {
    return null;
  }
  try {
    const parsed = new URL(trimmed);
    if (parsed.protocol === "https:" || parsed.protocol === "http:") {
      return parsed.toString().replace(/\/$/, "");
    }
    return null;
  } catch {
    return null;
  }
}

export function resolveSiteConfig(env: RawSiteEnv): SiteConfig {
  const repositoryUrl = sanitizeHttpUrl(env.VITE_REPO_URL, DEFAULT_REPO_URL);
  const fallbackDocsUrl = defaultDocsPathFromBase(env.BASE_URL);
  const documentationUrl = sanitizeDocsUrl(env.VITE_DOCS_URL, fallbackDocsUrl);
  const licenseUrl = sanitizeHttpUrl(env.VITE_LICENSE_URL, `${repositoryUrl}/blob/main/LICENSE`);
  const issuesUrl = sanitizeHttpUrl(env.VITE_ISSUES_URL, `${repositoryUrl}/issues`);
  const canonicalUrl = sanitizeOptionalCanonicalUrl(env.VITE_SITE_URL);
  return {
    repositoryUrl,
    documentationUrl,
    licenseUrl,
    issuesUrl,
    canonicalUrl,
  };
}

export const siteConfig: SiteConfig = resolveSiteConfig({
  BASE_URL: import.meta.env.BASE_URL,
  VITE_REPO_URL: import.meta.env.VITE_REPO_URL as string | undefined,
  VITE_DOCS_URL: import.meta.env.VITE_DOCS_URL as string | undefined,
  VITE_LICENSE_URL: import.meta.env.VITE_LICENSE_URL as string | undefined,
  VITE_ISSUES_URL: import.meta.env.VITE_ISSUES_URL as string | undefined,
  VITE_SITE_URL: import.meta.env.VITE_SITE_URL as string | undefined,
});
