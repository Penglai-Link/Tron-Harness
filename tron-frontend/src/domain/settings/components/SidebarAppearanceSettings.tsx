import { useTranslation } from "react-i18next";
import { ItemDescription, ItemTitle } from "@/components/ui/item";
import { AppearanceSection } from "./AppearanceLayout";
import { useTheme } from "@/shared/theme/ThemeProvider";

export function SidebarAppearanceSettings() {
  const { t } = useTranslation("pages");
  const { sidebarAppearanceSettings, setSidebarAppearanceSettings } = useTheme();

  return <AppearanceSection aria-label={t("settingsAppearance.sidebar")}>
    <div className="space-y-1">
      <ItemTitle>{t("settingsAppearance.sidebar")}</ItemTitle>
      <ItemDescription className="line-clamp-none text-xs">{t("settingsAppearance.sidebarDescription")}</ItemDescription>
    </div>
    <div className="grid gap-x-8 gap-y-4 @xl/appearance:grid-cols-2">
      <label className="grid gap-1.5 text-sm">
        <span className="flex items-center justify-between gap-2">
          <span>{t("settingsAppearance.sidebarOpacity")}</span>
          <span className="tabular-nums text-[var(--color-on-surface-variant)]">{sidebarAppearanceSettings.opacity}%</span>
        </span>
        <input
          aria-label={t("settingsAppearance.sidebarOpacity")}
          className="h-5 w-full cursor-pointer accent-[var(--color-primary)]"
          max={100}
          min={0}
          step={5}
          type="range"
          value={sidebarAppearanceSettings.opacity}
          onChange={(event) => setSidebarAppearanceSettings({ ...sidebarAppearanceSettings, opacity: Number(event.target.value) })}
        />
      </label>
      <label className="grid gap-1.5 text-sm">
        <span className="flex items-center justify-between gap-2">
          <span>{t("settingsAppearance.sidebarBlur")}</span>
          <span className="tabular-nums text-[var(--color-on-surface-variant)]">{sidebarAppearanceSettings.blur} px</span>
        </span>
        <input
          aria-label={t("settingsAppearance.sidebarBlur")}
          className="h-5 w-full cursor-pointer accent-[var(--color-primary)]"
          max={24}
          min={0}
          step={1}
          type="range"
          value={sidebarAppearanceSettings.blur}
          onChange={(event) => setSidebarAppearanceSettings({ ...sidebarAppearanceSettings, blur: Number(event.target.value) })}
        />
      </label>
    </div>
  </AppearanceSection>;
}
