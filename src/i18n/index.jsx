// src/i18n/index.jsx
import { load } from "@tauri-apps/plugin-store";
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useState,
} from "react";

import de from "./locales/de.json";
import en from "./locales/en.json";
import es from "./locales/es.json";
import fr from "./locales/fr.json";
import nl from "./locales/nl.json";
import ru from "./locales/ru.json";

const STORE_KEY = "language";

const catalogs = { en, nl, de, fr, es, ru };

export const LANGUAGES = [
  { code: "en", label: "English" },
  { code: "nl", label: "Nederlands" },
  { code: "de", label: "Deutsch" },
  { code: "fr", label: "Français" },
  { code: "es", label: "Español" },
  { code: "ru", label: "Русский" },
];

const I18nContext = createContext(null);

function detectLanguage() {
  const browserLang = navigator.language?.split("-")[0];
  if (browserLang && catalogs[browserLang]) return browserLang;
  return "en";
}

function interpolate(template, params) {
  if (!params) return template;
  return template.replace(/\{\{(\w+)\}\}/g, (_, key) =>
    params[key] !== undefined ? String(params[key]) : `{{${key}}}`,
  );
}

export function I18nProvider({ children }) {
  const [language, setLanguageState] = useState("en");
  const [ready, setReady] = useState(false);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const store = await load("settings.json", { autoSave: true });
        const persisted = await store.get(STORE_KEY);
        if (!cancelled) {
          setLanguageState(
            persisted && catalogs[persisted] ? persisted : detectLanguage(),
          );
        }
      } catch {
        if (!cancelled) setLanguageState(detectLanguage());
      } finally {
        if (!cancelled) setReady(true);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const setLanguage = useCallback(async (code) => {
    if (!catalogs[code]) return;
    setLanguageState(code);
    try {
      const store = await load("settings.json", { autoSave: true });
      await store.set(STORE_KEY, code);
    } catch {
      // persist is best-effort
    }
  }, []);

  const t = useCallback(
    (key, params) => {
      const value = catalogs[language]?.[key] ?? catalogs.en[key] ?? key;
      return interpolate(value, params);
    },
    [language],
  );

  if (!ready) return null;

  return (
    <I18nContext.Provider value={{ t, language, setLanguage }}>
      {children}
    </I18nContext.Provider>
  );
}

export function useTranslation() {
  const ctx = useContext(I18nContext);
  if (!ctx) throw new Error("useTranslation must be used within I18nProvider");
  return ctx;
}
