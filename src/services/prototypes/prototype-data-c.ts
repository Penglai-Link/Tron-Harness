export type StatusTone = "neutral" | "accent" | "success" | "warning" | "error";

export interface MetricDatum {
  label: string;
  value: string;
  trend: string;
  tone?: StatusTone;
}

export interface TraceDatum {
  id: string;
  agent: string;
  span: string;
  latency: string;
  status: string;
  time: string;
}

export interface ObservabilityData {
  metrics: MetricDatum[];
  throughput: { label: string; value: number }[];
  errors: { label: string; value: number }[];
  heatmap: number[];
  traces: TraceDatum[];
}

export interface PromptRecord {
  id: string;
  name: string;
  category: string;
  inheritance: string;
  updated: string;
  owner: string;
  body: string;
  variables: Record<string, string>;
}

export interface ResourceRecord {
  id: string;
  name: string;
  type: string;
  owner: string;
  scope: string;
  size: string;
  status: string;
  location: string;
  lastAccessed: string;
  updated: string;
}

export interface RoleRecord {
  id: string;
  name: string;
  description: string;
  permissions: Record<string, string[]>;
}

export interface CapabilityRecord {
  id: string;
  mode: "skills" | "mcp";
  name: string;
  description: string;
  provider: string;
  status: string;
  enabled: boolean;
  latency: string;
  schema: string;
  activity: string[];
}

export interface TaskDetailData {
  id: string;
  title: string;
  status: string;
  priority: string;
  progress: number;
  agent: string;
  description: string;
  requirements: string[];
  resources: string[];
  timeline: { time: string; message: string; tone?: StatusTone }[];
  metadata: Record<string, string>;
  subtasks: { id: string; title: string; status: string; progress: number; done: boolean }[];
}

export interface KanbanTask {
  id: string;
  title: string;
  priority: "LOW" | "MED" | "HIGH";
  progress: number;
  owner: string;
  signal: string;
}

export interface KanbanColumn {
  id: string;
  title: string;
  accent: string;
  tasks: KanbanTask[];
}

export interface UserRecord {
  id: string;
  name: string;
  email: string;
  role: string;
  workspace: string;
  status: string;
  lastActive: string;
}

export interface VariableRecord {
  id: string;
  key: string;
  scope: string;
  type: string;
  valuePreview: string;
  status: string;
  updated: string;
  usage: string[];
}

const observabilityData: ObservabilityData = {
  metrics: [
    { label: "Requests", value: "14.2M", trend: "+8.4% from last week", tone: "success" },
    { label: "P95 latency", value: "1.24s", trend: "Stable under failover threshold", tone: "accent" },
    { label: "Error rate", value: "0.08%", trend: "0.02% below budget", tone: "success" },
    { label: "Active traces", value: "342", trend: "42 with open spans", tone: "warning" },
  ],
  throughput: [
    { label: "Mon", value: 48 },
    { label: "Tue", value: 66 },
    { label: "Wed", value: 54 },
    { label: "Thu", value: 78 },
    { label: "Fri", value: 70 },
    { label: "Sat", value: 38 },
    { label: "Sun", value: 44 },
  ],
  errors: [
    { label: "Tool timeout", value: 16 },
    { label: "MCP auth", value: 8 },
    { label: "Prompt guard", value: 4 },
    { label: "Queue retry", value: 12 },
  ],
  heatmap: [22, 42, 68, 35, 80, 58, 48, 76, 31, 64, 54, 88, 39, 71, 46, 62, 28, 57, 74, 44, 69, 51, 33, 60],
  traces: [
    { id: "trc_91A", agent: "Meta-Agent", span: "route.llm.failover", latency: "412ms", status: "Review", time: "2m ago" },
    { id: "trc_88F", agent: "Code Agent", span: "tool.python.execute", latency: "1.8s", status: "Healthy", time: "8m ago" },
    { id: "trc_83D", agent: "Ops Agent", span: "mcp.postgres.query", latency: "242ms", status: "Healthy", time: "16m ago" },
  ],
};

const prompts: PromptRecord[] = [
  {
    id: "prompt-router",
    name: "LLM Router System",
    category: "System",
    inheritance: "Workspace override",
    updated: "Today 10:42",
    owner: "Platform",
    body: "You are routing a user request across approved workspace models. Prefer low-latency healthy providers and explain fallback decisions.",
    variables: { workspace: "enterprise-prod", primary_lang: "TypeScript", user_query: "Explain the hydration error in App.tsx" },
  },
  {
    id: "prompt-review",
    name: "Code Review Guidelines",
    category: "Tools",
    inheritance: "Inherited",
    updated: "Yesterday",
    owner: "Engineering",
    body: "Review changes for correctness, regressions, and missing tests. Lead with findings and cite file lines.",
    variables: { repo_name: "frontend-core", primary_lang: "TypeScript" },
  },
  {
    id: "prompt-persona",
    name: "Support Triage Persona",
    category: "Persona",
    inheritance: "Local",
    updated: "Apr 29",
    owner: "Operations",
    body: "Classify incidents, map owner teams, and propose the least disruptive remediation path.",
    variables: { severity: "P2", workspace: "enterprise-prod" },
  },
];

const resources: ResourceRecord[] = [
  { id: "res-prod-db", name: "Production DB (Primary)", type: "PostgreSQL 14", owner: "Data Platform", scope: "Workspace", size: "3.8 TB", status: "Active", location: "us-east-1a", lastAccessed: "Just now", updated: "Just now" },
  { id: "res-user-uploads", name: "User Uploads Archive", type: "AWS S3 Bucket", owner: "Platform Storage", scope: "Workspace", size: "2.4 TB", status: "Active", location: "eu-west-2", lastAccessed: "2 mins ago", updated: "2 mins ago" },
  { id: "res-auth-configs", name: "Global Auth Configs", type: "JSON Config File", owner: "Identity", scope: "Inherited", size: "18 MB", status: "Inherited", location: "Global Edge", lastAccessed: "1 hr ago", updated: "1 hr ago" },
  { id: "res-hero-assets", name: "Marketing Hero Assets", type: "Media Library", owner: "Brand", scope: "App", size: "392 GB", status: "Active", location: "CDN Core", lastAccessed: "5 hrs ago", updated: "5 hrs ago" },
  { id: "res-legacy-redis", name: "Legacy Redis Cache", type: "Redis 5.0", owner: "Infrastructure", scope: "Workspace", size: "92 GB", status: "Deprecated", location: "us-west-2b", lastAccessed: "3 days ago", updated: "3 days ago" },
];

const permissionDomains = ["Workspace", "Models", "Tasks", "Memory", "Skills", "MCP"];
const roles: RoleRecord[] = [
  {
    id: "admin",
    name: "Admin",
    description: "Full System Access",
    permissions: Object.fromEntries(permissionDomains.map((domain) => [domain, ["R", "W", "D", "A"]])),
  },
  {
    id: "editor",
    name: "Editor",
    description: "Content Management",
    permissions: {
      Workspace: ["R", "W"],
      Models: ["R", "W"],
      Tasks: ["R", "W"],
      Memory: ["R"],
      Skills: ["R", "W"],
      MCP: ["R"],
    },
  },
  {
    id: "viewer",
    name: "Viewer",
    description: "Read-only Access",
    permissions: Object.fromEntries(permissionDomains.map((domain) => [domain, ["R"]])),
  },
];

const capabilities: CapabilityRecord[] = [
  {
    id: "python",
    mode: "skills",
    name: "Python Code Interpreter",
    description: "Executes Python code in an isolated sandbox environment.",
    provider: "Local Runtime",
    status: "Connected & Healthy",
    enabled: true,
    latency: "42ms",
    schema: '{\n  "name": "execute_python",\n  "parameters": { "code": "string", "timeout": 30 }\n}',
    activity: ["Script execution completed", "Sandbox image refreshed"],
  },
  {
    id: "postgres",
    mode: "mcp",
    name: "Postgres Connector",
    description: "Read-only connection to the primary analytics database.",
    provider: "MCP Server",
    status: "Connected",
    enabled: true,
    latency: "68ms",
    schema: '{\n  "name": "query_postgres",\n  "parameters": { "sql": "string" }\n}',
    activity: ["Schema introspection completed", "Read policy verified"],
  },
  {
    id: "sendgrid",
    mode: "skills",
    name: "SendGrid Mailer",
    description: "Dispatches transactional emails to users via template.",
    provider: "SendGrid",
    status: "Disabled",
    enabled: false,
    latency: "n/a",
    schema: '{\n  "name": "send_email",\n  "parameters": { "template": "string", "to": "string" }\n}',
    activity: ["Disabled by workspace policy"],
  },
  {
    id: "stripe",
    mode: "mcp",
    name: "Stripe Billing API",
    description: "Manage customer subscriptions and invoice retrieval.",
    provider: "MCP Server",
    status: "Connected",
    enabled: true,
    latency: "91ms",
    schema: '{\n  "name": "lookup_invoice",\n  "parameters": { "customer_id": "string" }\n}',
    activity: ["Invoice retrieval completed", "OAuth token rotated"],
  },
];

const taskDetail: TaskDetailData = {
  id: "AGT-101",
  title: "Implement new LLM routing logic for failover",
  status: "In Progress",
  priority: "High",
  progress: 45,
  agent: "Meta-Agent",
  description:
    "The current LLM routing mechanism is susceptible to latency spikes when the primary node experiences high load. Implement failover to secondary regional nodes if the primary does not respond within 400ms.",
  requirements: [
    "Implement health check polling for all registered LLM endpoints.",
    "Develop the fallback routing controller in core/router.py.",
    "Ensure failover events are logged structurally for audit trails.",
    "Maintain exact state consistency during the handoff process.",
  ],
  resources: ["Model Config", "Customer Portal", "Main Console"],
  timeline: [
    { time: "10:45:02", message: "Initializing MCP connector...", tone: "accent" },
    { time: "10:45:05", message: "Validating workspace credentials [OK]", tone: "success" },
    { time: "10:45:15", message: "Fetching failover nodes metadata..." },
    { time: "10:46:01", message: "Awaiting sub-task completion...", tone: "warning" },
  ],
  metadata: {
    "Parent Task": "AGT-090",
    Owner: "Alex J.",
    "Created Date": "Oct 24, 2023",
    "Estimated Tokens": "~4,500",
    Branch: "feature/llm-failover",
  },
  subtasks: [
    { id: "st-1", title: "Draft architectural diagram for routing", status: "Completed", progress: 100, done: true },
    { id: "st-2", title: "Implement Ping/Health endpoint in Python", status: "In Progress", progress: 45, done: false },
    { id: "st-3", title: "Write integration tests for timeout scenarios", status: "Pending", progress: 0, done: false },
    { id: "st-4", title: "Deploy to staging and run shadow traffic", status: "Pending", progress: 0, done: false },
  ],
};

const kanban: KanbanColumn[] = [
  {
    id: "backlog",
    title: "Backlog",
    accent: "var(--color-outline-variant)",
    tasks: [
      { id: "AGT-105", title: "Refactor MCP connector for stability", priority: "LOW", progress: 35, owner: "MS", signal: "chat" },
      { id: "AGT-102", title: "Update memory vector store schema", priority: "MED", progress: 15, owner: "UA", signal: "attachment" },
    ],
  },
  { id: "todo", title: "To Do", accent: "var(--color-secondary)", tasks: [{ id: "AGT-101", title: "Implement new LLM routing logic for failover", priority: "HIGH", progress: 60, owner: "AJ", signal: "checklist" }] },
  {
    id: "progress",
    title: "In Progress",
    accent: "var(--color-primary)",
    tasks: [
      { id: "AGT-098", title: "Debug variable interpolation in core system prompts", priority: "HIGH", progress: 85, owner: "EV", signal: "2h" },
      { id: "AGT-096", title: "UI: Agent feed typography alignment", priority: "MED", progress: 45, owner: "JB", signal: "bug" },
    ],
  },
  { id: "review", title: "Review", accent: "var(--color-accent)", tasks: [{ id: "AGT-095", title: "Optimize skill execution latency", priority: "MED", progress: 95, owner: "RK", signal: "code" }] },
  { id: "done", title: "Done", accent: "var(--color-success)", tasks: [{ id: "AGT-090", title: "Add observability tracing to agent execution path", priority: "LOW", progress: 100, owner: "OK", signal: "done" }] },
];

const users: UserRecord[] = [
  { id: "usr-alex", name: "Alex Johnson", email: "alex@tron.ai", role: "Admin", workspace: "Enterprise", status: "Active", lastActive: "2m ago" },
  { id: "usr-maya", name: "Maya Singh", email: "maya@tron.ai", role: "Editor", workspace: "Platform", status: "Active", lastActive: "14m ago" },
  { id: "usr-chen", name: "Chen Wei", email: "chen@tron.ai", role: "Viewer", workspace: "Data", status: "Invited", lastActive: "Pending" },
  { id: "usr-rina", name: "Rina Patel", email: "rina@tron.ai", role: "Editor", workspace: "Support", status: "Suspended", lastActive: "Apr 28" },
];

const variables: VariableRecord[] = [
  { id: "openai", key: "OPENAI_API_KEY", scope: "Workspace", type: "Secret", valuePreview: "sk-...a92f", status: "Active", updated: "Today", usage: ["LLM Router System", "Code Review Guidelines"] },
  { id: "region", key: "DEFAULT_REGION", scope: "Inherited", type: "String", valuePreview: "us-east-1", status: "Active", updated: "Yesterday", usage: ["Failover Controller", "Trace Exporter"] },
  { id: "timeout", key: "ROUTER_TIMEOUT_MS", scope: "Local", type: "Number", valuePreview: "400", status: "Override", updated: "Apr 30", usage: ["LLM Router System"] },
];

export function listObservabilityData() {
  return observabilityData;
}

export function listPrompts() {
  return prompts;
}

export function listResources() {
  return resources;
}

export function listPermissionDomains() {
  return permissionDomains;
}

export function listRoles() {
  return roles;
}

export function listCapabilities() {
  return capabilities;
}

export function getTaskDetail() {
  return taskDetail;
}

export function listKanbanColumns() {
  return kanban;
}

export function listUsers() {
  return users;
}

export function listVariables() {
  return variables;
}
