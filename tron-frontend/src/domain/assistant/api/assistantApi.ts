import { normalizeWorkspaceInternalAgentRecord, type WorkspaceInternalAgentRecordApi } from "@/domain/agent/api/agentManagementApi";
import { requestApi } from "@/shared/api/httpClient";
import { normalizeChatSession, type ChatSession } from "@tron/chat-core";

export interface Assistant {
  description?: string;
  assistantCode: string; name: string; rolePrompt: string;
  avatarId: string; candidateModelCodes: string[];
  enabled: boolean; heartbeatMinutes: number; memoryAutoApprove: boolean;
  kind: "Main" | "Memory" | "Custom"; isMain: boolean;
  createdAt: number; updatedAt: number;
}
export type AssistantInput = Pick<Assistant, "assistantCode" | "name" | "description" | "rolePrompt" | "avatarId" | "candidateModelCodes" | "enabled" | "heartbeatMinutes" | "memoryAutoApprove"> & { kind?: "Custom" | "Memory" };
export interface AssistantSession extends ChatSession { purpose: string; ownerAssistantCode: string; }
interface AssistantSessionResponse extends Omit<AssistantSession, "createdAt" | "updatedAt"> { createdAt: number; updatedAt: number; }
function normalizeAssistantSession(value: AssistantSessionResponse): AssistantSession {
  return normalizeChatSession({ ...value, createdAt: new Date(value.createdAt).toISOString(), updatedAt: new Date(value.updatedAt).toISOString() }) as AssistantSession;
}
export interface AssistantTask {
  taskCode: string; title: string; status: string;
  ownerAssistantCode: string; dueAt: number | null; nextFollowupAt: number | null;
  plannedStartAt: number | null; workSessionCode: string | null; updatedAt: string;
}
export interface AssistantTaskDetail extends AssistantTask {
  description: string | null; executionResult: string | null; progress: number;
  createdAt: string;
}
export interface AssistantEvent { eventId: string; source: string; status: string; sessionCode: string | null; createdAt: number; result: string | null; }
export const assistantQueryKeys = { all: ["assistants"] as const, sessions: (code: string) => ["assistant-sessions", code] as const, tasks: (code: string) => ["assistant-tasks", code] as const, task: (code: string) => ["assistant-task", code] as const };
export const listAssistants = (signal?: AbortSignal) => requestApi<Assistant[]>("GET /api/v1/assistants", { signal });
export const createAssistant = (body: AssistantInput) => requestApi<Assistant>("POST /api/v1/assistants", { body });
export const updateAssistant = (assistantCode: string, body: Partial<Omit<AssistantInput, "assistantCode" | "name">>) => requestApi<Assistant>("PATCH /api/v1/assistants/{assistantCode}", { pathParams: { assistantCode }, body });
export const deleteAssistant = (assistantCode: string) => requestApi<void>("DELETE /api/v1/assistants/{assistantCode}", { pathParams: { assistantCode } });
export const resolveMainAssistantSession = () => requestApi<AssistantSessionResponse>("POST /api/v1/assistants/main/session").then(normalizeAssistantSession);
export const resolveAssistantSession = (assistantCode: string) => requestApi<AssistantSessionResponse>("POST /api/v1/assistants/{assistantCode}/sessions", { pathParams: { assistantCode } }).then(normalizeAssistantSession);
export const listAssistantSessions = (assistantCode: string, signal?: AbortSignal) => requestApi<AssistantSessionResponse[]>("GET /api/v1/assistants/{assistantCode}/sessions", { pathParams: { assistantCode }, signal }).then(items => items.map(normalizeAssistantSession));
export const listAssistantTasks = (assistantCode: string, signal?: AbortSignal) => requestApi<{ records: AssistantTask[] }>("GET /api/v1/tasks", { queryParams: { ownerAssistantCode: assistantCode, size: 200 }, signal }).then(page => page.records);
export const getAssistantTask = (taskCode: string, signal?: AbortSignal) => requestApi<AssistantTaskDetail>("GET /api/v1/tasks/{taskCode}", { pathParams: { taskCode }, signal });
export const createAssistantTask = (assistantCode: string, body: { description: string; dueAt: number | null }) => requestApi<AssistantTask>("POST /api/v1/tasks", { body: { workspaceCode: null, ownerAssistantCode: assistantCode, title: body.description.split("\n")[0].slice(0, 100), description: body.description, dueAt: body.dueAt, priority: "Medium", taskType: "General", sourceType: "Internal", assigneeUserCodes: [], agentCodes: [], appCodes: [] } });
export const updateAssistantTask = (_assistantCode: string, taskCode: string, body: { description?: string; dueAt?: number | null }) => requestApi<AssistantTask>("PATCH /api/v1/tasks/{taskCode}", { pathParams: { taskCode }, body: { ...body, ...(body.description ? { title: body.description.split("\n")[0].slice(0, 100) } : {}) } });
export const cancelAssistantTask = (assistantCode: string, taskCode: string) => requestApi<void>("POST /api/v1/assistants/{assistantCode}/tasks/{taskCode}/cancel", { pathParams: { assistantCode, taskCode } });
export const listAssistantEvents = (assistantCode: string, signal?: AbortSignal) => requestApi<AssistantEvent[]>("GET /api/v1/assistants/{assistantCode}/events", { pathParams: { assistantCode }, signal });

export interface AssistantThread { sessionCode: string; title: string | null; status: string; latestRunCode: string | null; }
export const listAssistantThreads = (assistantCode: string, signal?: AbortSignal) => requestApi<AssistantThread[]>("GET /api/v1/assistants/{assistantCode}/threads", { pathParams: { assistantCode }, signal });
export const getAssistantThreadHistory = (assistantCode: string, sessionCode: string, query: import("@personai/agent-ui-chat-core").HistoryQuery, signal?: AbortSignal) => requestApi<import("@personai/agent-ui-chat-core").SessionHistory>("POST /api/v1/assistants/{assistantCode}/threads/{sessionCode}/history/query", { pathParams: { assistantCode, sessionCode }, body: { size: query.limit, cursor: query.cursor }, signal });

/** PersonAI definitions are independent and intentionally absent from ordinary Agent lists. */
export const getAssistantResources = (assistantCode: string, signal?: AbortSignal) => requestApi<WorkspaceInternalAgentRecordApi>("GET /api/v1/assistants/{assistantCode}", { pathParams: { assistantCode }, signal }).then(record => normalizeWorkspaceInternalAgentRecord({ ...record, description: record.description ?? "", platformDefined: true }));
export const sendAssistantThreadMessage = (assistantCode: string, sessionCode: string, content: string) => requestApi<AssistantThread>("POST /api/v1/assistants/{assistantCode}/threads/{sessionCode}/messages", { pathParams: { assistantCode, sessionCode }, body: { content } });
export const answerAssistantThreadInteraction = (assistantCode: string, sessionCode: string, answers: { interactionId: string; value: unknown }[]) => requestApi<AssistantThread>("POST /api/v1/assistants/{assistantCode}/threads/{sessionCode}/answer", { pathParams: { assistantCode, sessionCode }, body: { answers } });

export type SoulState = "Uninitialized" | "AwaitingUser" | "Ready";
export interface AssistantSoul { assistantCode: string; name: string; markdown: string; nameConfirmed: boolean; state: SoulState }
export interface AwakeningStatus { assistantCode: string; name: string; state: SoulState; started: boolean; deferred: boolean; sessionCode: string | null; canResume: boolean }
export const getAssistantSoul = (assistantCode: string, signal?: AbortSignal) => requestApi<AssistantSoul>("GET /api/v1/assistants/{assistantCode}/soul", { pathParams: { assistantCode }, signal });
export const saveAssistantSoul = (assistantCode: string, body: { name: string; markdown: string }) => requestApi<AssistantSoul>("PUT /api/v1/assistants/{assistantCode}/soul", { pathParams: { assistantCode }, body });
export const getAwakening = (signal?: AbortSignal) => requestApi<AwakeningStatus>("GET /api/v1/assistants/main/awakening", { signal });
export const beginAwakening = () => requestApi<AwakeningStatus & { playEntrance: boolean }>("POST /api/v1/assistants/main/awakening", { body: {
  a2ui: { version: "v0.9.1", catalogIds: ["https://eliteforge.dev/a2ui/v0.9.1/agent", "https://eliteforge.dev/a2ui/v0.9.1/minimal"] },
} });
export const deferAwakening = () => requestApi<AwakeningStatus>("POST /api/v1/assistants/main/awakening/defer");
