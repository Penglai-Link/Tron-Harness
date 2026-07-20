import { create } from "zustand";

import type { ResourceDefinitionVersionDetail } from "./ResourceDefinitionModel";
import type { ResourceScopeKind } from "./ResourceModel";

export interface QuickResourceUploadHandoffPayload {
  allowInheritance: boolean;
  attributes: Record<string, unknown>;
  binaryName: string;
  definition: ResourceDefinitionVersionDetail;
  description: string;
  file: File;
  name: string;
  resourceCode: string;
  scopeCode: string;
  scopeKind: ResourceScopeKind;
}

export interface QuickResourceUploadHandoff extends QuickResourceUploadHandoffPayload {
  id: string;
}

interface QuickResourceUploadHandoffState {
  clear: (id: string) => void;
  peek: (id: string) => QuickResourceUploadHandoff | null;
  put: (payload: QuickResourceUploadHandoffPayload) => string;
  records: Record<string, QuickResourceUploadHandoff>;
}

function createHandoffId() {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") return crypto.randomUUID();
  return `quick-resource-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`;
}

/**
 * File handles are intentionally memory-only. A refreshed editor can keep the
 * resource fields but must ask the user to choose the file again.
 */
export const useQuickResourceUploadHandoffStore = create<QuickResourceUploadHandoffState>()((set, get) => ({
  clear: (id) => set((state) => {
    if (!(id in state.records)) return state;
    const { [id]: _, ...records } = state.records;
    return { records };
  }),
  peek: (id) => get().records[id] ?? null,
  put: (payload) => {
    const id = createHandoffId();
    set((state) => ({ records: { ...state.records, [id]: { ...payload, id } } }));
    return id;
  },
  records: {},
}));
