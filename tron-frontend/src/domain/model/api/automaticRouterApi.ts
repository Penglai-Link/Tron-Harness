import { requestApi } from "@/shared/api/httpClient";
export type AutomaticRouterKind = "Chat" | "TextGeneration" | "Embedding" | "Rerank" | "SpeechRecognition" | "Realtime" | "ImageGeneration";
export type RouteSourceMode = "AllProviders" | "SelectedProviders" | "SelectedModels";
export interface RouteSources { mode: RouteSourceMode; providerIds: string[]; modelCodes: string[] }
export interface RouteCandidate { providerId: string; providerName: string; modelCode: string; modelName: string; bindingId: string; inputModalities: ("Text" | "Image" | "Audio" | "Video")[]; dimension: number | null }
export interface AutomaticRouter { kind: AutomaticRouterKind; sources: RouteSources; candidates: RouteCandidate[] }
export function fetchAutomaticRouters(signal?: AbortSignal) {
  return requestApi<AutomaticRouter[]>("GET /api/v1/automatic-routers", { signal });
}
export async function updateAutomaticRouter(kind: AutomaticRouterKind, sources: RouteSources) {
  const result = await requestApi<RouteSources & { kind: AutomaticRouterKind }>("PUT /api/v1/automatic-routers/{kind}", { pathParams: { kind }, body: sources });
  window.dispatchEvent(new Event(AUTOMATIC_ROUTERS_CHANGED));
  return result;
}

export const AUTOMATIC_ROUTERS_CHANGED = "tron:automatic-routers-changed";
export const automaticRouterQueryKey = ["automatic-routers"] as const;
export function routeHasCandidates(route: AutomaticRouter) {
  const { sources } = route;
  return route.candidates.some(model => sources.mode === "AllProviders"
    || (sources.mode === "SelectedProviders" ? sources.providerIds.includes(model.providerId)
      : sources.modelCodes.includes(model.modelCode) || sources.modelCodes.includes(model.bindingId)));
}
