import type {
  ChatSessionGroup,
  ChatMessage,
} from "@/domain/chat/model/ChatModel";

const chatGroups: ChatSessionGroup[] = [
  {
    task: "Core Infrastructure Update",
    sessions: [
      { id: "primary", title: "Primary Investigation", active: true },
      { id: "refactor", title: "Refactor Discussion" },
    ],
  },
  {
    task: "Refactor MCP Connector",
    sessions: [
      { id: "history", title: "Historical Trace Review" },
      { id: "execution", title: "Execution Notes" },
    ],
  },
];

const reasoningMessages: ChatMessage[] = [
  { id: "m1", actor: "User", time: "10:42 AM", kind: "user", body: "Auth latency spiked after the token validator deployment. Find the narrowest rollback path." },
  { id: "m2", actor: "Planner Agent", time: "10:43 AM", kind: "agent", body: "I am isolating request paths, deployment windows, and cache behavior before proposing changes.", meta: "Agent Reasoning" },
  { id: "m3", actor: "QueryTelemetry", time: "10:43 AM", kind: "tool", body: "service=\"auth\" time=\"last_24h\"\nTelemetry Results: CRITICAL latency isolated to eu-west-1 and eu-central-1.", meta: "v1.2.0" },
  { id: "m4", actor: "Coder Agent", time: "10:44 AM", kind: "agent", body: "The fast-path cache check was removed in PR #1042. Reinstating it behind a feature flag is the least invasive mitigation.", meta: "Agent Reasoning" },
];

const executionMessages: ChatMessage[] = [
  { id: "e1", actor: "User", time: "10:42 AM", kind: "user", body: "Run the auth validator inspection and show the concrete code path." },
  { id: "e2", actor: "QueryTelemetry", time: "10:43 AM", kind: "tool", body: "QueryTelemetry(service=\"auth\", time=\"last_24h\")\n1.2s eu-west-1 eu-central-1", meta: "terminal" },
  { id: "e3", actor: "Coder Agent", time: "10:44 AM", kind: "code", body: "auth/token_validator.go\nfunc validateToken(token string) error {\n  // Previous fast-path cache check was removed in PR #1042\n  if err := db.Validate(token); err != nil {\n    return err\n  }\n  return nil\n}", meta: "code_blocks" },
];

export function getChatSessionGroups() {
  return chatGroups;
}

export function getChatMessages(variant: "reasoning" | "execution") {
  return variant === "reasoning" ? reasoningMessages : executionMessages;
}
