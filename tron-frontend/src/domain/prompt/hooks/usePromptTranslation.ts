import { useCallback } from "react";
import { useTranslation } from "react-i18next";

export function usePromptTranslation() {
  const { t } = useTranslation(["pages"]);

  return useCallback((key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`pages:promptManagement.${key}`, { defaultValue, ...options })
  ), [t]);
}
