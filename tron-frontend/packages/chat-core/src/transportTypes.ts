/** Host transport data. Presentation libraries map these records at their boundary. */
export interface ChatAnnotation { id: string; messageId: string; selectedText: string; comment: string }
export interface ToolActionSummary { commands: number; editedFiles: number; editOperations: number; reads: number; searches: number; otherActions: number }
export interface MessageProvenance {
  sender: { kind: string; id?: string; displayName?: string; avatarUrl?: string };
  source?: string;
  correlation?: unknown;
}
export interface A2uiActionMetadata { a2uiClientDataModel?: { surfaces: Record<string, unknown> } }
export interface A2uiActionInput {
  actionId: string; surfaceInstanceId: string; expectedRevision: number;
  message: { version: "v0.9.1"; action: { name: string; surfaceId: string; sourceComponentId: string; timestamp: string; context: Record<string, unknown> } };
  metadata: A2uiActionMetadata;
}
export interface A2uiState {
  sessionId: string; revision: number;
  surfaces: Array<{
    surfaceId: string; surfaceInstanceId: string; revision: number; mode: "Display" | "SubmitOnce" | "Reusable";
    ownerAgentId: string; runId: string; messageId: string; createdAtMs: number;
    snapshot: {
      surfaceId: string; catalogId: string; sendDataModel: boolean;
      components: Record<string, { id: string; component: string; [key: string]: unknown }>;
      dataModel: unknown;
      viewState?: Record<string, { tab?: number; open?: boolean; query?: string; month?: string; size?: number; sort?: string; desc?: boolean; expanded?: string[] }>;
    };
  }>;
}
export type HistoryStatus = "Running" | "Waiting" | "Completed" | "Interrupted" | "Failed" | "Unknown" | "Planned" | "Approved";
export type HistoryContentBlock =
  | { type: "Text" | "Reasoning"; text: string }
  | { type: "Json"; value: unknown }
  | { type: "Media"; mimeType: string; source: { type: "url" | "base64"; value: string }; name: string | null }
  | { type: "Resource"; uri: string; mimeType: string | null; name: string | null; sizeBytes: number | null };
export interface HistoryQuery { limit: number; cursor: { sessionId: string; version: string; before: number } | null }
export interface HistoryRun { id: string; sessionId: string; parentRunId: string | null; rootRunId: string; status: HistoryStatus; startedAtMs: number; finishedAtMs: number | null; error: string | null }
export interface HistoryInteraction {
  responseProvenance?: MessageProvenance | null;
  id: string; runId: string; invocationId: string | null; prompt: HistoryContentBlock[];
  responseSchema: Record<string, unknown>; presentation: Record<string, unknown> | null; response: unknown; expiresAtMs: number | null;
}
export interface SessionHistory {
  canDeleteTurns?: boolean;
  sessionId: string; version: string; nextCursor: { sessionId: string; version: string; before: number } | null;
  runs: HistoryRun[];
  records: {
    steps: Array<{ id: string; runId: string; nodeInvocationId: string | null; sequence: number; attempt: number; name: string; model: { connection: string; model: string } | null; status: HistoryStatus; usage: { inputTokens: number; outputTokens: number } | null; startedAtMs: number; finishedAtMs: number | null }>;
    messages: Array<{ annotations?: ChatAnnotation[]; requestedSkills?: string[]; provenance?: MessageProvenance; id: string; runId: string; stepId: string | null; role: "User" | "Assistant"; phase?: string | null; outputIndex?: number | null; content: HistoryContentBlock[]; structured: unknown; status: HistoryStatus; createdAtMs: number }>;
    toolInvocations: Array<{ batchRevision?: string; batch?: { parentCallId: string; parentInvocationId: string; callId: string; invocationId: string; index: number } | null; id: string; runId: string; stepId: string; messageId: string; callId: string; toolId: string; toolName: string; arguments: unknown; status: HistoryStatus; content: HistoryContentBlock[]; structured: unknown; isError: boolean | null; resultReference: HistoryContentBlock | null; startedAtMs: number | null; finishedAtMs: number | null }>;
    interactions: HistoryInteraction[];
  };
  pendingInteractions: HistoryInteraction[]; activeRuns: HistoryRun[];
}
export interface ChatHistoryLabels { loading: string; loadOlder: string; step: string; attempt: string; statuses: Record<HistoryStatus, string> }
