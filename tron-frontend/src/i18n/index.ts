import i18n from "i18next";
import {
  initReactI18next,
  useTranslation,
} from "react-i18next";

import { enUS } from "./locales/en-US";
import { zhCN } from "./locales/zh-CN";

export const fallbackLocale = "en-US";
export const supportedLocales = [
  {
    code: "en-US",
    labelKey: "common:locale.english",
  },
  {
    code: "zh-CN",
    labelKey: "common:locale.simplifiedChinese",
  },
] as const;

export type SupportedLocale = (typeof supportedLocales)[number]["code"];

const localeStorageKey = "tron.locale";
const supportedLocaleCodes = supportedLocales.map((locale) => locale.code);

function isSupportedLocale(value: string): value is SupportedLocale {
  return supportedLocaleCodes.includes(value as SupportedLocale);
}

function getStoredLocale() {
  if (typeof window === "undefined") return undefined;
  const storedLocale = window.localStorage.getItem(localeStorageKey);
  return storedLocale && isSupportedLocale(storedLocale) ? storedLocale : undefined;
}

function getBrowserLocale() {
  if (typeof navigator === "undefined") return undefined;
  return navigator.languages.find(isSupportedLocale) ?? (isSupportedLocale(navigator.language) ? navigator.language : undefined);
}

function getDocumentLocale() {
  if (typeof document === "undefined") return undefined;
  const locale = document.documentElement.lang;
  return isSupportedLocale(locale) ? locale : undefined;
}

export function getInitialLocale(): SupportedLocale {
  return getStoredLocale() ?? getBrowserLocale() ?? fallbackLocale;
}

/**
 * Resolves the application locale that is emitted through the standard
 * Accept-Language request header. The document language is synchronized from
 * the user's selection and remains the visible source of truth across HMR.
 */
export function getCurrentLocale(): SupportedLocale {
  const documentLocale = getDocumentLocale();
  if (documentLocale) return documentLocale;
  if (isSupportedLocale(i18n.language)) return i18n.language;
  const storedLocale = getStoredLocale();
  if (storedLocale) return storedLocale;
  return getBrowserLocale() ?? fallbackLocale;
}

export function applyCurrentLocaleHeader(headers: Headers) {
  headers.set("Accept-Language", getCurrentLocale());
  return headers;
}

function syncDocumentLocale(locale: SupportedLocale) {
  if (typeof document !== "undefined") {
    document.documentElement.lang = locale;
  }
}

void i18n.use(initReactI18next).init({
  defaultNS: "common",
  fallbackLng: fallbackLocale,
  interpolation: {
    escapeValue: false,
  },
  lng: getInitialLocale(),
  missingKeyHandler: (_lng, ns, key) => {
    if (import.meta.env.DEV) {
      console.warn(`[i18n] Missing translation key: ${ns}:${key}`);
    }
  },
  ns: ["common", "shell", "console", "pages"],
  resources: {
    "en-US": enUS,
    "zh-CN": zhCN,
  },
  returnEmptyString: false,
  saveMissing: import.meta.env.DEV,
  supportedLngs: [...supportedLocaleCodes],
}).then(() => syncDocumentLocale(getCurrentLocale()));

export function persistLocale(locale: SupportedLocale) {
  if (typeof window !== "undefined") {
    window.localStorage.setItem(localeStorageKey, locale);
  }
}

i18n.on("languageChanged", (locale) => {
  if (!isSupportedLocale(locale)) return;
  persistLocale(locale);
  syncDocumentLocale(locale);
});

if (typeof window !== "undefined") {
  window.addEventListener("storage", (event) => {
    if (
      event.key !== localeStorageKey
      || !event.newValue
      || !isSupportedLocale(event.newValue)
      || event.newValue === i18n.language
    ) {
      return;
    }
    void i18n.changeLanguage(event.newValue);
  });
}

export function useLocale() {
  const { i18n: i18next, t } = useTranslation("common");
  const currentLocale = getCurrentLocale();

  async function changeLocale(locale: SupportedLocale) {
    await i18next.changeLanguage(locale);
  }

  return {
    changeLocale,
    currentLocale,
    supportedLocales,
    t,
  };
}

export { i18n };
