import { buildApiUrl, requestApi, requestApiBlob } from "@/shared/api/httpClient";
import type { Skin } from "./Skin";

const base = "/api/v1/system/skins";
export const skinApi = {
  list: () => requestApi<Skin[]>(base),
  active: () => requestApi<{ skin: Skin }>(`${base}/active`),
  activate: (id: string) => requestApi<{ skin: Skin }>(`PUT ${base}/active`, { body: { id } }),
  install: (file: File) => requestApi<{ skin: Skin }>(`POST ${base}`, { body: file }),
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
