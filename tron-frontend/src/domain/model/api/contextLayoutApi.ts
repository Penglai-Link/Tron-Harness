import { requestApi } from "@/shared/api/httpClient";

export interface ContextLayoutBlock {
  id: string;
  displayName: string;
  description: string;
  plugin: boolean;
  unavailable: boolean;
  providerId: string | null;
  defaultMaxTokens: number | null;
  defaultMaxRatioBps: number | null;
  defaultMaxMessages: number | null;
  overrideMaxTokens: number | null;
}

export interface ContextLayoutState {
  revision: string;
  order: string[];
  pluginOverrides: Record<string, number>;
  blocks: ContextLayoutBlock[];
}

export function fetchContextLayout(signal?: AbortSignal) {
  return requestApi<ContextLayoutState>("GET /api/v1/context-layout", { signal });
}

export function updateContextLayout(
  layout: Pick<ContextLayoutState, "revision" | "order" | "pluginOverrides">,
) {
  return requestApi<ContextLayoutState>("PUT /api/v1/context-layout", { body: layout });
}
