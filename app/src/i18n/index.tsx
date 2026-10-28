import { createContext, useContext, useEffect, useMemo, type ReactNode } from "react";
import type { UiLanguage } from "@/bridge";
import { en, type Strings } from "./en";
import { tr } from "./tr";

const tables: Record<UiLanguage, Strings> = { en, tr };
const intlLocales: Record<UiLanguage, string> = { en: "en-GB", tr: "tr-TR" };

export const languageNames: Record<UiLanguage, string> = { en: "English", tr: "Türkçe" };

export interface I18n {
  language: UiLanguage;
  t: Strings;
  errorMessage: (code: string) => string;
  skipMessage: (code: string) => string;
  formatTime: (ms: number) => string;
  formatDate: (ms: number) => string;
}

export function createI18n(language: UiLanguage): I18n {
  const t = tables[language];
  const time = new Intl.DateTimeFormat(intlLocales[language], { hour: "2-digit", minute: "2-digit", second: "2-digit" });
  const date = new Intl.DateTimeFormat(intlLocales[language], { dateStyle: "medium", timeStyle: "short" });
  return {
    language,
    t,
    errorMessage: (code) => t.errors[code] ?? t.errors.unknown ?? code,
    skipMessage: (code) => t.skip[code] ?? code,
    formatTime: (ms) => time.format(new Date(ms)),
    formatDate: (ms) => date.format(new Date(ms)),
  };
}

const I18nContext = createContext<I18n>(createI18n("en"));

export function I18nProvider({ language, children }: { language: UiLanguage; children: ReactNode }) {
  const value = useMemo(() => createI18n(language), [language]);
  useEffect(() => {
    document.documentElement.lang = language;
  }, [language]);
  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n(): I18n {
  return useContext(I18nContext);
}
