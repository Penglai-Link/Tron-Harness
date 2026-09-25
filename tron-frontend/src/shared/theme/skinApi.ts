import { buildApiUrl, requestApi, requestApiBlob } from "@/shared/api/httpClient";
import type { Skin } from "./Skin";

export interface MarketSkin {
  id: string;
  name: string;
  author: string | null;
  previewUrl: string | null;
  downloadCount: number | null;
}
export interface MarketSkinPage { current: number; size: number; total: number; records: MarketSkin[] }
const base = "/api/v1/system/skins";
export const skinApi = {
  market: (current: number, signal?: AbortSignal) => requestApi<MarketSkinPage>(`${base}/market`, { queryParams: { current, size: 20 }, signal }),
  installMarket: (id: string) => requestApi<{ skin: Skin }>(`POST ${base}/market/${encodeURIComponent(id)}/install`, { retryOnUnauthorized: false }),
  list: () => requestApi<Skin[]>(base),
  active: () => requestApi<{ skin: Skin }>(`${base}/active`),
  activate: (id: string) => requestApi<{ skin: Skin }>(`PUT ${base}/active`, { body: { id } }),
  install: (file: File) => requestApi<{ skin: Skin }>(`POST ${base}`, { body: file }),
  uninstall: (id: string) => requestApi<void>(`DELETE ${base}/${encodeURIComponent(id)}`),
  resource: async (id: string, path: string) => {
    const result = await requestApiBlob(`${base}/${encodeURIComponent(id)}/resources/${path.split("/").map(encodeURIComponent).join("/")}`, { signal: AbortSignal.timeout(10000) });
    return result.blob;
  },
  watch: (onChange: () => void) => {
    const source = new EventSource(buildApiUrl(`${base}/events`), { withCredentials: true });
    source.addEventListener("skin-changed", onChange);
    return () => source.close();
  },
};
