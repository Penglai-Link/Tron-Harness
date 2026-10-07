import { requestApi } from "@/shared/api/httpClient";
import type { ChatMessage, ChatSession } from "@/domain/chat/model";

export interface ArchivedChatSessionPage {
  records: ChatSession[];
  total: number;
  current: number;
  size: number;
}

export interface ArchivedChatSessionDeleteItem {
  sessionCode: string;
  success: boolean;
  errorCode?: string | null;
  message?: string | null;
}

export interface ArchivedChatSessionBatchDeleteResult {
  succeeded: number;
  failed: number;
  items: ArchivedChatSessionDeleteItem[];
}

const archivedChatEndpoints = {
  list: "GET /api/v1/chats/archived-sessions",
  restore: "POST /api/v1/chats/sessions/{sessionCode}/restore",
  delete: "DELETE /api/v1/chats/sessions/{sessionCode}",
  deleteBatch: "POST /api/v1/chats/archived-sessions/batch-delete",
  messages: "GET /api/v1/chats/sessions/{sessionCode}/messages",
} as const;

export function fetchArchivedChatSessions(query: {
  workspaceCode?: string;
  current: number;
  size: number;
}) {
  return requestApi<ArchivedChatSessionPage>(archivedChatEndpoints.list, {
    queryParams: query,
  });
}

export function fetchArchivedChatMessages(sessionCode: string) {
  return requestApi<ChatMessage[]>(archivedChatEndpoints.messages, {
    pathParams: { sessionCode },
  });
}

export function restoreArchivedChatSession(sessionCode: string) {
  return requestApi<ChatSession>(archivedChatEndpoints.restore, {
    pathParams: { sessionCode },
  });
}

export function deleteArchivedChatSession(sessionCode: string) {
  return requestApi<ChatSession>(archivedChatEndpoints.delete, {
    pathParams: { sessionCode },
  });
}

export function deleteArchivedChatSessions(sessionCodes: string[]) {
  return requestApi<ArchivedChatSessionBatchDeleteResult>(archivedChatEndpoints.deleteBatch, {
    body: { sessionCodes },
  });
}
