import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { en, type Messages } from "./en";
import { tr } from "./tr";

export type Locale = "en" | "tr";

interface I18nContextValue {
  readonly locale: Locale;
  readonly t: Messages;
  readonly setLocale: (next: Locale) => void;
}

const tables: Record<Locale, Messages> = { en, tr };

const I18nContext = createContext<I18nContextValue>({
  locale: "en",
  t: en,
  setLocale: () => undefined,
});

export function I18nProvider({ initialLocale = "en", children }: { initialLocale?: Locale; children: ReactNode }) {
  const [locale, setLocale] = useState<Locale>(initialLocale);
  const t = tables[locale];

  useEffect(() => {
    document.documentElement.lang = locale;
    document.documentElement.dir = "ltr";
    document.title = t.meta.title;
    const descriptionTag = document.querySelector<HTMLMetaElement>('meta[name="description"]');
    if (descriptionTag) {
      descriptionTag.content = t.meta.description;
    }
  }, [locale, t]);

  const value = useMemo<I18nContextValue>(() => ({ locale, t, setLocale }), [locale, t]);
  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n(): I18nContextValue {
  return useContext(I18nContext);
}
