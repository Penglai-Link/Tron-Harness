import { useCallback } from "react";
import { useTranslation } from "react-i18next";

export type NavigationAdminLocale = "zh-CN" | "en-US";

export function useNavigationAdminLocale(): NavigationAdminLocale {
  const { i18n } = useTranslation("pages");
  return i18n.resolvedLanguage?.toLowerCase().startsWith("en") ? "en-US" : "zh-CN";
}

export function useNavigationAdminText() {
  const { i18n, t } = useTranslation("pages");
  const english = i18n.resolvedLanguage?.toLowerCase().startsWith("en") ?? false;

  return useCallback((key: string, chinese: string, englishText: string, options?: Record<string, unknown>): string => String(
    t(`navigationAdmin.${key}`, {
      ...options,
      defaultValue: english ? englishText : chinese,
    } as never),
  ), [english, t]);
}
