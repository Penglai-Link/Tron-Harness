import { useTranslation } from "react-i18next";
import { Item, ItemContent, ItemDescription, ItemTitle } from "@/components/ui/item";
import { SurfaceCard } from "@/shared/layout/SurfaceCard";
import { useTheme } from "@/shared/theme/ThemeProvider";

export function SidebarAppearanceSettings() {
  const { t } = useTranslation("pages");
  const { sidebarAppearanceSettings, setSidebarAppearanceSettings } = useTheme();

  return <SurfaceCard className="p-0">
    <div className="border-b border-[var(--color-outline-variant)]/50 px-3 py-3">
      <Item size="xs">
        <ItemContent>
          <ItemTitle>{t("settingsAppearance.sidebar")}</ItemTitle>
          <ItemDescription>{t("settingsAppearance.sidebarDescription")}</ItemDescription>
        </ItemContent>
      </Item>
    </div>
    <div className="grid gap-4 px-3 py-3 sm:grid-cols-2">
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
  </SurfaceCard>;
}
