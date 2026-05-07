import i18n from "i18next";
import {
  initReactI18next,
  useTranslation,
} from "react-i18next";

import { enUS } from "./locales/en-US";

export const fallbackLocale = "en-US";
export const supportedLocales = [
  {
    code: "en-US",
    labelKey: "common:locale.english",
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

export function getInitialLocale(): SupportedLocale {
  return getStoredLocale() ?? getBrowserLocale() ?? fallbackLocale;
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
  ns: ["common", "shell", "auth", "home", "console"],
  resources: {
    "en-US": enUS,
  },
  returnEmptyString: false,
  saveMissing: import.meta.env.DEV,
  supportedLngs: [...supportedLocaleCodes],
});

export function persistLocale(locale: SupportedLocale) {
  if (typeof window !== "undefined") {
    window.localStorage.setItem(localeStorageKey, locale);
  }
}

export function useLocale() {
  const { i18n: i18next, t } = useTranslation("common");
  const resolvedLanguage = i18next.resolvedLanguage ?? i18next.language ?? fallbackLocale;
  const currentLocale = isSupportedLocale(resolvedLanguage) ? resolvedLanguage : fallbackLocale;

  async function changeLocale(locale: SupportedLocale) {
    await i18next.changeLanguage(locale);
    persistLocale(locale);
  }

  return {
    changeLocale,
    currentLocale,
    supportedLocales,
    t,
  };
}

export { i18n };
