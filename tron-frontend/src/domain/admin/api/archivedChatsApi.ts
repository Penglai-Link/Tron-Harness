import { requestApi, requestApiBlob } from "@/shared/api/httpClient";
import type { ChatMessageAttachment } from "@/domain/chat/model";
import { notifyChatSessionsRemoved } from "@/shared/lib/shellEvents";

export interface ArchivedSession {
  sessionCode: string; workspaceCode: string; projectCode: string | null;
  title: string; status: "ACTIVE" | "ARCHIVED"; createdAt: number; updatedAt: number;
}
export interface ArchivedMessage {
  messageCode: string; runCode: string | null; role: string; actor: string;
  content: string; attachments: ChatMessageAttachment[]; createdAt: number;
}
export interface ArchivedSessionDetail {
  a2uiState?: import("@personai/agent-ui-chat-core").ChatA2uiState;
  session: ArchivedSession; messages: ArchivedMessage[];
  goals: { goalCode: string; objective: string; status: string; progress: unknown; criteria: { id: string; text: string; completed: boolean }[]; lastDecision: { verdict: string; reason: string; next_input: string | null; required_input: string | null } | null; runsUsed: number; maxRuns: number; reason: string | null; updatedAt: number }[];
  goalEvents: { eventCode: string; goalCode: string; runCode: string | null; kind: string; payload: unknown; createdAt: number }[];
  runs: { runCode: string; parentRunCode: string | null; branch: string; kind: string; status: string; startedAt: number; finishedAt: number | null; error: string | null }[];
  history: { branch: string; generation: string; messages: { messageCode: string; role: string; content: string }[] }[];
}
export interface ArchivedChatSessionPage { records: ArchivedSession[]; total: number; current: number; size: number }
export interface ArchivedChatSessionDeleteItem { sessionCode: string; success: boolean; errorCode: string | null; message: string | null }
export interface ArchivedChatSessionBatchDeleteResult { succeeded: number; failed: number; items: ArchivedChatSessionDeleteItem[] }

const root = "/api/v1/admin/archived-sessions";
export type ArchivedContainerKind = "Topic" | "Project";
export interface ArchivedContainer { code: string; workspaceCode: string; name: string; path: string | null; chatCount: number }
export function fetchArchivedContainers(query: { kind: ArchivedContainerKind; workspaceCode: string | null; current: number; size: number }, signal?: AbortSignal) {
  return requestApi<{ records: ArchivedContainer[]; total: number; current: number; size: number }>("POST /api/v1/admin/archived-containers/query", { body: query, signal });
}
export async function restoreArchivedContainer(kind: ArchivedContainerKind, code: string) {
  const result = await requestApi<{ code: string }>("POST /api/v1/admin/archived-containers/{kind}/{code}/restore", { pathParams: { kind, code } });
  window.dispatchEvent(new Event("tron:project-created"));
  window.dispatchEvent(new Event("tron:directories-changed"));
  return result;
}

export function fetchArchivedChatSessions(query: { workspaceCode?: string; current: number; size: number }, signal?: AbortSignal) {
  return requestApi<ArchivedChatSessionPage>(`GET ${root}`, { queryParams: query, signal });
}
export function fetchArchivedChatDetail(sessionCode: string, signal?: AbortSignal) {
  return requestApi<ArchivedSessionDetail>(`GET ${root}/{sessionCode}`, { pathParams: { sessionCode }, signal });
}
export function restoreArchivedChatSession(sessionCode: string) {
  return requestApi<ArchivedSession>(`POST ${root}/{sessionCode}/restore`, { pathParams: { sessionCode } });
}
export async function deleteArchivedChatSession(sessionCode: string) {
  const session = await requestApi<ArchivedSession>(`DELETE ${root}/{sessionCode}`, { pathParams: { sessionCode } });
  notifyChatSessionsRemoved([sessionCode]);
  return session;
}
export async function deleteArchivedChatSessions(sessionCodes: string[]) {
  const result = await requestApi<ArchivedChatSessionBatchDeleteResult>(`POST ${root}/batch-delete`, { body: { sessionCodes } });
  notifyChatSessionsRemoved(result.items.filter((item) => item.success).map((item) => item.sessionCode));
  return result;
}
export function downloadArchivedChatAttachment(sessionCode: string, attachmentCode: string) {
  return requestApiBlob(`GET ${root}/{sessionCode}/attachments/{attachmentCode}`, { pathParams: { sessionCode, attachmentCode } });
}
