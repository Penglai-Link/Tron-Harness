import { useTranslation } from "react-i18next";

import {
  Field,
  PageHeader,
  SelectInput,
  SurfaceCard,
  useTheme,
} from "@/shared";
import { useLocale } from "@/i18n";

export function SettingsGeneralPage() {
  const { t } = useTranslation(["common", "shell"]);
  const { changeLocale, currentLocale, supportedLocales } = useLocale();
  const { setTheme, theme, themes } = useTheme();

  return (
    <>
      <PageHeader title={t("shell:settings.items.general")} />

      <div className="grid gap-4 xl:grid-cols-2">
        <SurfaceCard className="space-y-3">
          <Field label={t("shell:theme.label")}>
            <SelectInput
              value={theme}
              onChange={(event) => setTheme(event.target.value === "matrix-dark" ? "matrix-dark" : "light")}
            >
              {themes.map((themeOption) => (
                <option key={themeOption.value} value={themeOption.value}>
                  {t(themeOption.labelKey)}
                </option>
              ))}
            </SelectInput>
          </Field>
          <p className="text-sm text-[var(--color-on-surface-variant)]">{t("shell:theme.hint")}</p>
        </SurfaceCard>

        <SurfaceCard className="space-y-3">
          <Field label={t("common:locale.label")}>
            <SelectInput
              value={currentLocale}
              onChange={(event) => void changeLocale(event.target.value as typeof currentLocale)}
            >
              {supportedLocales.map((locale) => (
                <option key={locale.code} value={locale.code}>
                  {t(locale.labelKey)}
                </option>
              ))}
            </SelectInput>
          </Field>
          <p className="text-sm text-[var(--color-on-surface-variant)]">{t("common:locale.unavailable")}</p>
        </SurfaceCard>
      </div>
    </>
  );
}
