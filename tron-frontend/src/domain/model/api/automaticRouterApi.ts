import { requestApi } from "@/shared/api/httpClient";
export type AutomaticRouterKind = "Chat" | "TextGeneration" | "Embedding" | "Rerank";
export type RouteSourceMode = "AllProviders" | "SelectedProviders" | "SelectedModels";
export interface RouteSources { mode: RouteSourceMode; providerIds: string[]; modelCodes: string[] }
export interface RouteCandidate { providerId: string; providerName: string; modelCode: string; modelName: string; bindingId: string; inputModalities: ("Text" | "Image" | "Audio" | "Video")[]; dimension: number | null }
export interface AutomaticRouter { kind: AutomaticRouterKind; sources: RouteSources; candidates: RouteCandidate[] }
export function fetchAutomaticRouters(signal?: AbortSignal) {
  return requestApi<AutomaticRouter[]>("GET /api/v1/automatic-routers", { signal });
}
export function updateAutomaticRouter(kind: AutomaticRouterKind, sources: RouteSources) {
  return requestApi<RouteSources & { kind: AutomaticRouterKind }>("PUT /api/v1/automatic-routers/{kind}", { pathParams: { kind }, body: sources });
}
