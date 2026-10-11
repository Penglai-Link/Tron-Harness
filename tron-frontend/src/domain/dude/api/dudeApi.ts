import { normalizeWorkspaceInternalAgentRecord, type WorkspaceInternalAgentRecordApi } from "@/domain/agent/api/agentManagementApi";
import { requestApi } from "@/shared/api/httpClient";
import { normalizeChatSession, type ChatSession } from "@tron/chat-core";

export interface Dude {
  dudeCode: string; name: string; rolePrompt: string; loopCode: string | null;
  avatarId: string; candidateModelCodes: string[];
  enabled: boolean; heartbeatMinutes: number; memoryAutoApprove: boolean;
  kind: "Main" | "Memory" | "Custom"; isMain: boolean;
  createdAt: number; updatedAt: number;
}
export type DudeInput = Pick<Dude, "dudeCode" | "name" | "rolePrompt" | "loopCode" | "avatarId" | "candidateModelCodes" | "enabled" | "heartbeatMinutes" | "memoryAutoApprove"> & { kind?: "Custom" | "Memory" };
export interface DudeSession extends ChatSession { purpose: string; ownerDudeCode: string; }
interface DudeSessionResponse extends Omit<DudeSession, "createdAt" | "updatedAt"> { createdAt: number; updatedAt: number; }
function normalizeDudeSession(value: DudeSessionResponse): DudeSession {
  return normalizeChatSession({ ...value, createdAt: new Date(value.createdAt).toISOString(), updatedAt: new Date(value.updatedAt).toISOString() }) as DudeSession;
}
export interface DudeTask {
  taskCode: string; title: string; description: string | null; status: string;
  ownerDudeCode: string; dueAt: number | null; nextFollowupAt: number | null;
  plannedStartAt: number | null; workSessionCode: string | null; updatedAt: string;
}
export interface DudeEvent { eventId: string; source: string; status: string; sessionCode: string | null; createdAt: number; result: string | null; }
export const dudeQueryKeys = { all: ["dudes"] as const, sessions: (code: string) => ["dude-sessions", code] as const, tasks: (code: string) => ["dude-tasks", code] as const };
export const listDudes = (signal?: AbortSignal) => requestApi<Dude[]>("GET /api/v1/dudes", { signal });
export const createDude = (body: DudeInput) => requestApi<Dude>("POST /api/v1/dudes", { body });
export const updateDude = (dudeCode: string, body: Partial<Omit<DudeInput, "dudeCode">>) => requestApi<Dude>("PATCH /api/v1/dudes/{dudeCode}", { pathParams: { dudeCode }, body });
export const deleteDude = (dudeCode: string) => requestApi<void>("DELETE /api/v1/dudes/{dudeCode}", { pathParams: { dudeCode } });
export const resolveMainDudeSession = () => requestApi<DudeSessionResponse>("POST /api/v1/dudes/main/session").then(normalizeDudeSession);
export const resolveDudeSession = (dudeCode: string) => requestApi<DudeSessionResponse>("POST /api/v1/dudes/{dudeCode}/sessions", { pathParams: { dudeCode } }).then(normalizeDudeSession);
export const listDudeSessions = (dudeCode: string, signal?: AbortSignal) => requestApi<DudeSessionResponse[]>("GET /api/v1/dudes/{dudeCode}/sessions", { pathParams: { dudeCode }, signal }).then(items => items.map(normalizeDudeSession));
export const listDudeTasks = (dudeCode: string, signal?: AbortSignal) => requestApi<{ records: DudeTask[] }>("GET /api/v1/tasks", { queryParams: { ownerDudeCode: dudeCode, size: 200 }, signal }).then(page => page.records);
export const createDudeTask = (dudeCode: string, body: { description: string; dueAt: number | null }) => requestApi<DudeTask>("POST /api/v1/tasks", { body: { workspaceCode: null, ownerDudeCode: dudeCode, title: body.description.split("\n")[0].slice(0, 100), description: body.description, dueAt: body.dueAt, priority: "Medium", taskType: "General", sourceType: "Internal", assigneeUserCodes: [], agentCodes: [], appCodes: [] } });
export const updateDudeTask = (_dudeCode: string, taskCode: string, body: { description?: string; dueAt?: number | null }) => requestApi<DudeTask>("PATCH /api/v1/tasks/{taskCode}", { pathParams: { taskCode }, body: { ...body, ...(body.description ? { title: body.description.split("\n")[0].slice(0, 100) } : {}) } });
export const archiveDudeTask = (taskCode: string) => requestApi<unknown>("POST /api/v1/tasks/{taskCode}/archive", { pathParams: { taskCode } });
export const listDudeEvents = (dudeCode: string, signal?: AbortSignal) => requestApi<DudeEvent[]>("GET /api/v1/dudes/{dudeCode}/events", { pathParams: { dudeCode }, signal });

export interface DudeThread { threadCode: string; title: string | null; status: string; latestRunCode: string | null; }
export const listDudeThreads = (dudeCode: string, signal?: AbortSignal) => requestApi<DudeThread[]>("GET /api/v1/dudes/{dudeCode}/threads", { pathParams: { dudeCode }, signal });
export const getDudeThreadHistory = (dudeCode: string, threadCode: string, query: import("@eliteforge/agent-ui-chat-core").HistoryQuery, signal?: AbortSignal) => requestApi<import("@eliteforge/agent-ui-chat-core").SessionHistory>("POST /api/v1/dudes/{dudeCode}/threads/{threadCode}/history/query", { pathParams: { dudeCode, threadCode }, body: { size: query.limit, cursor: query.cursor }, signal });

/** Dude definitions are independent and intentionally absent from ordinary Agent lists. */
export const getDudeResources = (dudeCode: string, signal?: AbortSignal) => requestApi<WorkspaceInternalAgentRecordApi>("GET /api/v1/dudes/{dudeCode}", { pathParams: { dudeCode }, signal }).then(record => normalizeWorkspaceInternalAgentRecord({ ...record, description: record.description ?? "", platformDefined: true }));
export const sendDudeThreadMessage = (dudeCode: string, threadCode: string, content: string) => requestApi<DudeThread>("POST /api/v1/dudes/{dudeCode}/threads/{threadCode}/messages", { pathParams: { dudeCode, threadCode }, body: { content } });
export const answerDudeThreadInteraction = (dudeCode: string, threadCode: string, interactionId: string, value: unknown) => requestApi<DudeThread>("POST /api/v1/dudes/{dudeCode}/threads/{threadCode}/answer", { pathParams: { dudeCode, threadCode }, body: { interactionId, value } });
