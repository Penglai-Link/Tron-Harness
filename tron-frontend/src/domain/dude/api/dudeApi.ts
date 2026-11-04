import { normalizeWorkspaceInternalAgentRecord, type WorkspaceInternalAgentRecordApi } from "@/domain/agent/api/agentManagementApi";
import { requestApi } from "@/shared/api/httpClient";
import { normalizeChatSession, type ChatSession } from "@tron/chat-core";

export interface Dude {
  description?: string;
  dudeCode: string; name: string; rolePrompt: string;
  avatarId: string; candidateModelCodes: string[];
  enabled: boolean; heartbeatMinutes: number; memoryAutoApprove: boolean;
  kind: "Main" | "Memory" | "Custom"; isMain: boolean;
  createdAt: number; updatedAt: number;
}
export type DudeInput = Pick<Dude, "dudeCode" | "name" | "description" | "rolePrompt" | "avatarId" | "candidateModelCodes" | "enabled" | "heartbeatMinutes" | "memoryAutoApprove"> & { kind?: "Custom" | "Memory" };
export interface DudeSession extends ChatSession { purpose: string; ownerDudeCode: string; }
interface DudeSessionResponse extends Omit<DudeSession, "createdAt" | "updatedAt"> { createdAt: number; updatedAt: number; }
function normalizeDudeSession(value: DudeSessionResponse): DudeSession {
  return normalizeChatSession({ ...value, createdAt: new Date(value.createdAt).toISOString(), updatedAt: new Date(value.updatedAt).toISOString() }) as DudeSession;
}
export interface DudeTask {
  taskCode: string; title: string; status: string;
  ownerDudeCode: string; dueAt: number | null; nextFollowupAt: number | null;
  plannedStartAt: number | null; workSessionCode: string | null; updatedAt: string;
}
export interface DudeTaskDetail extends DudeTask {
  description: string | null; executionResult: string | null; progress: number;
  createdAt: string;
}
export interface DudeEvent { eventId: string; source: string; status: string; sessionCode: string | null; createdAt: number; result: string | null; }
export const dudeQueryKeys = { all: ["dudes"] as const, sessions: (code: string) => ["dude-sessions", code] as const, tasks: (code: string) => ["dude-tasks", code] as const, task: (code: string) => ["dude-task", code] as const };
export const listDudes = (signal?: AbortSignal) => requestApi<Dude[]>("GET /api/v1/dudes", { signal });
export const createDude = (body: DudeInput) => requestApi<Dude>("POST /api/v1/dudes", { body });
export const updateDude = (dudeCode: string, body: Partial<Omit<DudeInput, "dudeCode" | "name">>) => requestApi<Dude>("PATCH /api/v1/dudes/{dudeCode}", { pathParams: { dudeCode }, body });
export const deleteDude = (dudeCode: string) => requestApi<void>("DELETE /api/v1/dudes/{dudeCode}", { pathParams: { dudeCode } });
export const resolveMainDudeSession = () => requestApi<DudeSessionResponse>("POST /api/v1/dudes/main/session").then(normalizeDudeSession);
export const resolveDudeSession = (dudeCode: string) => requestApi<DudeSessionResponse>("POST /api/v1/dudes/{dudeCode}/sessions", { pathParams: { dudeCode } }).then(normalizeDudeSession);
export const listDudeSessions = (dudeCode: string, signal?: AbortSignal) => requestApi<DudeSessionResponse[]>("GET /api/v1/dudes/{dudeCode}/sessions", { pathParams: { dudeCode }, signal }).then(items => items.map(normalizeDudeSession));
export const listDudeTasks = (dudeCode: string, signal?: AbortSignal) => requestApi<{ records: DudeTask[] }>("GET /api/v1/tasks", { queryParams: { ownerDudeCode: dudeCode, size: 200 }, signal }).then(page => page.records);
export const getDudeTask = (taskCode: string, signal?: AbortSignal) => requestApi<DudeTaskDetail>("GET /api/v1/tasks/{taskCode}", { pathParams: { taskCode }, signal });
export const createDudeTask = (dudeCode: string, body: { description: string; dueAt: number | null }) => requestApi<DudeTask>("POST /api/v1/tasks", { body: { workspaceCode: null, ownerDudeCode: dudeCode, title: body.description.split("\n")[0].slice(0, 100), description: body.description, dueAt: body.dueAt, priority: "Medium", taskType: "General", sourceType: "Internal", assigneeUserCodes: [], agentCodes: [], appCodes: [] } });
export const updateDudeTask = (_dudeCode: string, taskCode: string, body: { description?: string; dueAt?: number | null }) => requestApi<DudeTask>("PATCH /api/v1/tasks/{taskCode}", { pathParams: { taskCode }, body: { ...body, ...(body.description ? { title: body.description.split("\n")[0].slice(0, 100) } : {}) } });
export const cancelDudeTask = (dudeCode: string, taskCode: string) => requestApi<void>("POST /api/v1/dudes/{dudeCode}/tasks/{taskCode}/cancel", { pathParams: { dudeCode, taskCode } });
export const listDudeEvents = (dudeCode: string, signal?: AbortSignal) => requestApi<DudeEvent[]>("GET /api/v1/dudes/{dudeCode}/events", { pathParams: { dudeCode }, signal });

export interface DudeThread { sessionCode: string; title: string | null; status: string; latestRunCode: string | null; }
export const listDudeThreads = (dudeCode: string, signal?: AbortSignal) => requestApi<DudeThread[]>("GET /api/v1/dudes/{dudeCode}/threads", { pathParams: { dudeCode }, signal });
export const getDudeThreadHistory = (dudeCode: string, sessionCode: string, query: import("@eliteforge/agent-ui-chat-core").HistoryQuery, signal?: AbortSignal) => requestApi<import("@eliteforge/agent-ui-chat-core").SessionHistory>("POST /api/v1/dudes/{dudeCode}/threads/{sessionCode}/history/query", { pathParams: { dudeCode, sessionCode }, body: { size: query.limit, cursor: query.cursor }, signal });

/** Dude definitions are independent and intentionally absent from ordinary Agent lists. */
export const getDudeResources = (dudeCode: string, signal?: AbortSignal) => requestApi<WorkspaceInternalAgentRecordApi>("GET /api/v1/dudes/{dudeCode}", { pathParams: { dudeCode }, signal }).then(record => normalizeWorkspaceInternalAgentRecord({ ...record, description: record.description ?? "", platformDefined: true }));
export const sendDudeThreadMessage = (dudeCode: string, sessionCode: string, content: string) => requestApi<DudeThread>("POST /api/v1/dudes/{dudeCode}/threads/{sessionCode}/messages", { pathParams: { dudeCode, sessionCode }, body: { content } });
export const answerDudeThreadInteraction = (dudeCode: string, sessionCode: string, answers: { interactionId: string; value: unknown }[]) => requestApi<DudeThread>("POST /api/v1/dudes/{dudeCode}/threads/{sessionCode}/answer", { pathParams: { dudeCode, sessionCode }, body: { answers } });

export type SoulState = "Uninitialized" | "AwaitingUser" | "Ready";
export interface DudeSoul { dudeCode: string; name: string; markdown: string; nameConfirmed: boolean; state: SoulState }
export interface AwakeningStatus { dudeCode: string; name: string; state: SoulState; started: boolean; deferred: boolean; sessionCode: string | null; canResume: boolean }
export const getDudeSoul = (dudeCode: string, signal?: AbortSignal) => requestApi<DudeSoul>("GET /api/v1/dudes/{dudeCode}/soul", { pathParams: { dudeCode }, signal });
export const saveDudeSoul = (dudeCode: string, body: { name: string; markdown: string }) => requestApi<DudeSoul>("PUT /api/v1/dudes/{dudeCode}/soul", { pathParams: { dudeCode }, body });
export const getAwakening = (signal?: AbortSignal) => requestApi<AwakeningStatus>("GET /api/v1/dudes/main/awakening", { signal });
export const beginAwakening = () => requestApi<AwakeningStatus & { playEntrance: boolean }>("POST /api/v1/dudes/main/awakening", { body: {
  a2ui: { version: "v0.9.1", catalogIds: ["https://eliteforge.dev/a2ui/v0.9.1/agent", "https://eliteforge.dev/a2ui/v0.9.1/minimal"] },
} });
export const deferAwakening = () => requestApi<AwakeningStatus>("POST /api/v1/dudes/main/awakening/defer");
