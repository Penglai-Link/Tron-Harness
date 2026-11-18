import { requestApi } from "@/shared/api/httpClient";
import { notifyChatSessionsRemoved } from "@/shared/lib/shellEvents";

export const sessionManagementApiEndpoints = {
  list: "GET /api/v1/admin/chat-sessions",
  stop: "POST /api/v1/admin/chat-sessions/stop",
  archive: "POST /api/v1/admin/chat-sessions/archive",
  remove: "POST /api/v1/admin/chat-sessions/delete",
  move: "POST /api/v1/admin/chat-sessions/move",
  restore: "POST /api/v1/admin/chat-sessions/{sessionCode}/restore",
} as const;

export interface ManagedChatSession {
  sessionCode: string;
  workspaceCode: string;
  projectCode: string | null;
  directoryCode: string | null;
  workingDirectoryCode: string | null;
  taskCode: string | null;
  sourceType: string;
  title: string;
  status: "ACTIVE" | "ARCHIVED";
  activeRunCode: string | null;
  running: boolean;
  createdAt: number;
  updatedAt: number;
}

export interface ManagedChatSessionPage {
  records: ManagedChatSession[];
  total: number;
  current: number;
  size: number;
}

export interface ManagedSessionBatchItem {
  sessionCode: string;
  success: boolean;
  errorCode: string | null;
  message: string | null;
}

export interface ManagedSessionBatchResult {
  succeeded: number;
  failed: number;
  items: ManagedSessionBatchItem[];
}

export interface ManagedSessionStopResult {
  stoppedSessionCodes: string[];
}

export interface ManagedSessionQuery {
  workspaceCode?: string;
  status?: "ACTIVE" | "ARCHIVED";
  sourceType?: "MANUAL" | "TASK" | "AUTOMATION";
  search?: string;
  current?: number;
  size?: number;
}

export interface MoveSessionsPayload {
  sessionCodes: string[];
  targetWorkspaceCode: string;
  targetProjectCode?: string | null;
}

export function fetchManagedChatSessions(query: ManagedSessionQuery, signal?: AbortSignal) {
  return requestApi<ManagedChatSessionPage>(sessionManagementApiEndpoints.list, {
    queryParams: {
      workspaceCode: query.workspaceCode ?? "",
      status: query.status ?? "",
      sourceType: query.sourceType ?? "",
      search: query.search ?? "",
      current: query.current,
      size: query.size,
    },
    signal,
  });
}

export function stopManagedChatSessions(sessionCodes: string[], signal?: AbortSignal) {
  return requestApi<ManagedSessionStopResult>(sessionManagementApiEndpoints.stop, {
    body: { sessionCodes },
    signal,
  });
}

export async function archiveManagedChatSessions(sessionCodes: string[]) {
  const result = await requestApi<ManagedSessionBatchResult>(sessionManagementApiEndpoints.archive, {
    body: { sessionCodes },
  });
  notifyChatSessionsRemoved(result.items.filter((item) => item.success).map((item) => item.sessionCode));
  return result;
}

export async function deleteManagedChatSessions(sessionCodes: string[]) {
  const result = await requestApi<ManagedSessionBatchResult>(sessionManagementApiEndpoints.remove, {
    body: { sessionCodes },
  });
  notifyChatSessionsRemoved(result.items.filter((item) => item.success).map((item) => item.sessionCode));
  return result;
}

export function moveManagedChatSessions(payload: MoveSessionsPayload) {
  return requestApi<ManagedSessionBatchResult>(sessionManagementApiEndpoints.move, {
    body: payload,
  });
}

export function restoreManagedChatSession(sessionCode: string) {
  return requestApi<ManagedChatSession>(sessionManagementApiEndpoints.restore, {
    pathParams: { sessionCode },
  });
}
