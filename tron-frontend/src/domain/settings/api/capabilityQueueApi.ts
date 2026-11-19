import { requestApi } from "@/shared/api/httpClient";

export type CapabilityQueueOperation = "enable" | "disable" | "uninstall";

export interface CapabilityQueueItem {
  id: string;
  packId: string;
  packName: string;
  operation: CapabilityQueueOperation;
  force: boolean;
  /** `running` is a transient in-memory state on the backend. */
  status: "queued" | "running";
  createdAt: number;
}

export interface CapabilityOperationLog {
  id: string;
  packId: string;
  packName: string;
  operation: CapabilityQueueOperation;
  force: boolean;
  succeeded: boolean;
  error: string | null;
  errorCode: string | null;
  finishedAt: number;
}

export interface CapabilityQueueSnapshot {
  agentBusy: boolean;
  items: CapabilityQueueItem[];
  log: CapabilityOperationLog[];
}

export interface CapabilityEnqueueResult {
  itemId: string;
  status: "pending" | "succeeded" | "failed";
  error: string | null;
  code: string | null;
}

export function getCapabilityQueue(signal?: AbortSignal) {
  return requestApi<CapabilityQueueSnapshot>("GET /api/v1/capabilities/queue", {
    signal,
  });
}

export function enqueueCapabilityOperation(input: {
  packId: string;
  packName: string;
  operation: CapabilityQueueOperation;
  force?: boolean;
}) {
  return requestApi<CapabilityEnqueueResult>("POST /api/v1/capabilities/queue", {
    body: input,
  });
}
