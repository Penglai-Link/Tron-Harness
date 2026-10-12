export interface SidebarAppearanceSettings {
  opacity: number;
  blur: number;
}

export const defaultSidebarAppearanceSettings: SidebarAppearanceSettings = {
  opacity: 100,
  blur: 0,
};

const storageKey = "tron.sidebar-appearance";

export function normalizeSidebarAppearanceSettings(
  value: Partial<SidebarAppearanceSettings> | null | undefined,
): SidebarAppearanceSettings {
  return {
    opacity: typeof value?.opacity === "number" && Number.isFinite(value.opacity)
      ? Math.min(100, Math.max(0, value.opacity))
      : defaultSidebarAppearanceSettings.opacity,
    blur: typeof value?.blur === "number" && Number.isFinite(value.blur)
      ? Math.min(24, Math.max(0, value.blur))
      : defaultSidebarAppearanceSettings.blur,
  };
}

export function readSidebarAppearanceSettings(storage: Pick<Storage, "getItem">): SidebarAppearanceSettings {
  try {
    const value = JSON.parse(storage.getItem(storageKey) ?? "null") as Partial<SidebarAppearanceSettings> | null;
    return normalizeSidebarAppearanceSettings(value);
  } catch {
    return defaultSidebarAppearanceSettings;
  }
}

export function writeSidebarAppearanceSettings(
  storage: Pick<Storage, "setItem">,
  value: SidebarAppearanceSettings,
): SidebarAppearanceSettings {
  const settings = normalizeSidebarAppearanceSettings(value);
  storage.setItem(storageKey, JSON.stringify(settings));
  return settings;
}
