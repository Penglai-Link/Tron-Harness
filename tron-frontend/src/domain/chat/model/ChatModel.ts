export interface ChatSessionGroup {
  task: string;
  sessions: { id: string; title: string; active?: boolean }[];
}

export interface ChatMessage {
  id: string;
  actor: string;
  time: string;
  body: string;
  kind: "user" | "agent" | "tool" | "code";
  meta?: string;
}
