import { requestApi } from "@/shared/api/httpClient";
import type { ContextBlockUpdateMode, ContextCompressionMetadata } from "@/shared/context/contextBlockPresentation";

export interface ContextLayoutBlock {
  id: string;
  displayName: string;
  description: string;
  updateMode: ContextBlockUpdateMode;
  compression?: ContextCompressionMetadata | null;
  plugin: boolean;
  unavailable: boolean;
  providerId: string | null;
}

export interface ContextLayoutDefaults {
  order: string[];
  shares: Record<string, number>;
}

export interface ContextLayoutState {
  revision: string;
  order: string[];
  /** Basis points; the distribution must total at most 10,000. */
  shares: Record<string, number>;
  blocks: ContextLayoutBlock[];
  defaults: ContextLayoutDefaults;
}

export function fetchContextLayout(signal?: AbortSignal) {
  return requestApi<ContextLayoutState>("GET /api/v1/context-layout", { signal });
}

export function updateContextLayout(layout: Pick<ContextLayoutState, "revision" | "order" | "shares">) {
  return requestApi<ContextLayoutState>("PUT /api/v1/context-layout", { body: layout });
}
