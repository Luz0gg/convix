// Minimal i18n: one JSON per language, `{param}` interpolation, French by default.
// Adding a language = adding a JSON file here. No library needed at this size.
import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from "react";
import fr from "./fr.json";
import en from "./en.json";

const dictionaries = { fr, en } as const;
export type Lang = keyof typeof dictionaries;
export type TranslationKey = keyof typeof fr;

type Params = Record<string, string | number>;
type T = (key: string, params?: Params) => string;

const I18nContext = createContext<{ lang: Lang; setLang: (l: Lang) => void; t: T } | null>(null);

const STORAGE_KEY = "convix.lang";

function initialLang(): Lang {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved && saved in dictionaries) return saved as Lang;
  } catch {
    /* storage unavailable */
  }
  return "fr";
}

export function I18nProvider({ children }: { children: ReactNode }) {
  const [lang, setLangState] = useState<Lang>(initialLang);

  const setLang = useCallback((l: Lang) => {
    setLangState(l);
    document.documentElement.lang = l;
    try {
      localStorage.setItem(STORAGE_KEY, l);
    } catch {
      /* ignore */
    }
  }, []);

  const t = useCallback<T>(
    (key, params) => {
      const dict = dictionaries[lang] as Record<string, string>;
      const raw = dict[key] ?? (fr as Record<string, string>)[key] ?? key;
      return params ? raw.replace(/\{(\w+)\}/g, (_, p) => String(params[p] ?? `{${p}}`)) : raw;
    },
    [lang],
  );

  const value = useMemo(() => ({ lang, setLang, t }), [lang, setLang, t]);
  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n() {
  const ctx = useContext(I18nContext);
  if (!ctx) throw new Error("useI18n must be used inside <I18nProvider>");
  return ctx;
}
