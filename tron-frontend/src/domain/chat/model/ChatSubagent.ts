import type { ChatAguiStreamEvent, HistoryStatus, SessionHistory } from "@eliteforge/agent-ui-chat-core";

export interface ChatSubagent {
  runId: string;
  sessionId: string;
  parentRunId: string;
  agentCode: string;
  agentName: string;
  avatarId?: string | null;
  task: string;
  status: HistoryStatus;
  startedAtMs: number;
  finishedAtMs: number | null;
}
export interface ChatSubagentDetail {
  runId: string;
  sessionId: string;
  status: HistoryStatus;
  records: SessionHistory["records"];
  events: ChatAguiStreamEvent[];
}
