export interface Skin {
  marketId?: string | null;
  id: string;
  name: string;
  version: number;
  mode: "Light" | "Dark";
  colors: Record<string, string>;
  layout: Record<string, string>;
  resources: { background: string | null; font: string | null; css?: string | null };
}
