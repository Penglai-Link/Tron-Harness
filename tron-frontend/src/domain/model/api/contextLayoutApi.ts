import { requestApi } from "@/shared/api/httpClient";

export interface ContextLayoutBlock {
  id: string;
  displayName: string;
  description: string;
  plugin: boolean;
  unavailable: boolean;
  providerId: string | null;
}

export interface ContextLayoutState {
  revision: string;
  order: string[];
  /** Basis points; the complete distribution must total exactly 10,000. */
  shares: Record<string, number>;
  blocks: ContextLayoutBlock[];
}

export function fetchContextLayout(signal?: AbortSignal) {
  return requestApi<ContextLayoutState>("GET /api/v1/context-layout", { signal });
}

export function updateContextLayout(layout: Pick<ContextLayoutState, "revision" | "order" | "shares">) {
  return requestApi<ContextLayoutState>("PUT /api/v1/context-layout", { body: layout });
}
