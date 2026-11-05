export type ColorMode = "light" | "dark" | "system";
export type ResolvedColorMode = Exclude<ColorMode, "system">;

export interface Skin {
  marketId?: string | null;
  id: string;
  name: string;
  version: number;
  variants: Record<ResolvedColorMode, Record<string, string>>;
  layout: Record<string, string>;
  resources: { background: string | null; font: string | null; css?: string | null };
}

export interface ResolvedSkin extends Omit<Skin, "variants"> {
  mode: "Light" | "Dark";
  colors: Record<string, string>;
}

export function resolveSkin(skin: Skin, mode: ResolvedColorMode): ResolvedSkin {
  const { variants, ...shared } = skin;
  return { ...shared, mode: mode === "dark" ? "Dark" : "Light", colors: variants[mode] };
}
