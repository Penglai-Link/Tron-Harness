export type BadgeStatus = "Healthy" | "Running" | "Success" | "Warning" | "Failed" | "Disabled" | "Draft";

export interface AppRecord {
  id: string;
  name: string;
  source: string;
  path: string;
  worktree: string;
  tags: string[];
  updated: string;
}

export interface AuditLogRecord {
  id: string;
  timestamp: string;
  actor: string;
  action: string;
  resourceId: string;
  status: "Success" | "Warning" | "Failed";
  ipAddress: string;
}

export interface AutomationRecord {
  id: string;
  name: string;
  cadence: string;
  cron: string;
  lastRun: string;
  status: "Running" | "Success" | "Failed";
  owner: string;
  dependencies: { label: string; status: "ok" | "blocked" | "failed" }[];
  logs: string[];
  traces: string[];
}

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

export interface ModelCostLine {
  label: string;
  value: string;
}

export interface PromptVariable {
  name: string;
  type: string;
  sample: string;
}

export interface SkillSchemaPreview {
  name: string;
  fields: { name: string; type: string; required: boolean }[];
}

export interface QuickReference {
  tips: string[];
  variables: string[];
}

const apps: AppRecord[] = [
  { id: "app-customer", name: "Customer Portal", source: "Frontend", path: "/apps/customer-portal", worktree: "main", tags: ["React", "Public"], updated: "4m ago" },
  { id: "app-auth", name: "Auth Service", source: "Backend", path: "/services/auth", worktree: "release/2.8", tags: ["Go", "Critical"], updated: "18m ago" },
  { id: "app-data", name: "Data Pipeline", source: "Worker", path: "/jobs/data-pipeline", worktree: "feature/sync-v3", tags: ["Python", "ETL"], updated: "42m ago" },
  { id: "app-billing", name: "Billing Console", source: "Frontend", path: "/apps/billing", worktree: "main", tags: ["React", "Finance"], updated: "1h ago" },
];

const auditLogs: AuditLogRecord[] = [
  { id: "log-9131", timestamp: "2026-05-01 14:42:18", actor: "jdoe@tron.com", action: "Role updated", resourceId: "role.workspace.admin", status: "Success", ipAddress: "10.24.8.12" },
  { id: "log-9130", timestamp: "2026-05-01 14:39:02", actor: "asmith@tron.com", action: "Inheritance override", resourceId: "model.tron-opus-v2", status: "Warning", ipAddress: "10.24.9.77" },
  { id: "log-9129", timestamp: "2026-05-01 14:34:55", actor: "system_agent", action: "Secret rotated", resourceId: "vault.openai.primary", status: "Success", ipAddress: "127.0.0.1" },
  { id: "log-9128", timestamp: "2026-05-01 14:31:21", actor: "jdoe@tron.com", action: "Export requested", resourceId: "audit.batch.2451", status: "Success", ipAddress: "10.24.8.12" },
];

const automations: AutomationRecord[] = [
  {
    id: "auto-daily-sync",
    name: "Daily Data Sync",
    cadence: "Every day at midnight",
    cron: "0 0 * * *",
    lastRun: "2h ago",
    status: "Success",
    owner: "Agent Alpha",
    dependencies: [
      { label: "Task B (Prereq)", status: "ok" },
      { label: "Daily Data Sync", status: "ok" },
      { label: "Model Eval Matrix", status: "failed" },
    ],
    logs: ["00:00 trigger accepted", "00:02 Salesforce extraction complete", "00:07 warehouse load complete"],
    traces: ["root.span sync.daily 432ms", "tool.salesforce.fetch 2.2s", "warehouse.merge 4.8s"],
  },
  {
    id: "auto-refresh",
    name: "Data Refresh Monitor",
    cadence: "Every 12 hours",
    cron: "0 */12 * * *",
    lastRun: "Running",
    status: "Running",
    owner: "Global Pool",
    dependencies: [
      { label: "Metric Snapshot", status: "ok" },
      { label: "Refresh Monitor", status: "ok" },
    ],
    logs: ["12:00 trigger accepted", "12:01 checking stale partitions"],
    traces: ["root.span refresh.monitor active", "tool.snowflake.inspect 1.1s"],
  },
  {
    id: "auto-hourly",
    name: "Hourly Guardrail Audit",
    cadence: "Every hour",
    cron: "0 * * * *",
    lastRun: "18m ago",
    status: "Failed",
    owner: "Agent Beta",
    dependencies: [
      { label: "Prompt Snapshot", status: "ok" },
      { label: "Guardrail Audit", status: "failed" },
    ],
    logs: ["14:00 trigger accepted", "14:02 prompt source timed out", "14:03 retry exhausted"],
    traces: ["root.span audit.hourly failed", "prompt.registry.fetch timeout"],
  },
];

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

export function getApps() {
  return apps;
}

export function getAuditLogs() {
  return auditLogs;
}

export function getAutomations() {
  return automations;
}

export function getChatSessionGroups() {
  return chatGroups;
}

export function getChatMessages(variant: "reasoning" | "execution") {
  return variant === "reasoning" ? reasoningMessages : executionMessages;
}

export function getModelCostPreview(): ModelCostLine[] {
  return [
    { label: "Input (1M Tokens)", value: "$10.00" },
    { label: "Output (1M Tokens)", value: "$30.00" },
    { label: "Context Window", value: "128,000" },
  ];
}

export function getPromptVariables(): PromptVariable[] {
  return [
    { name: "{{customer_tier}}", type: "string", sample: "Enterprise" },
    { name: "{{incident_summary}}", type: "markdown", sample: "OAuth callback failures after deploy" },
  ];
}

export function getSkillSchemaPreview(): SkillSchemaPreview {
  return {
    name: "get_weather",
    fields: [
      { name: "location", type: "string", required: true },
      { name: "unit", type: "enum", required: false },
    ],
  };
}

export function getTaskQuickReference(): QuickReference {
  return {
    tips: ["State the expected output explicitly.", "Attach app and agent context before execution.", "Use variables for credentials and runtime IDs."],
    variables: ["@user_id", "@auth_token", "@api_endpoint"],
  };
}
