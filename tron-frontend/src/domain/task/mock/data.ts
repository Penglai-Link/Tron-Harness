import type {
  QuickReference,
  TaskDetailData,
  KanbanColumn,
} from "@/domain/task/model/TaskModel";

export function getTaskQuickReference(): QuickReference {
  return {
    tips: ["State the expected output explicitly.", "Attach app and agent context before execution.", "Use variables for credentials and runtime IDs."],
    variables: ["@user_id", "@auth_token", "@api_endpoint"],
  };
}

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

export function getTaskDetail() {
  return taskDetail;
}

export function listKanbanColumns() {
  return kanban;
}
