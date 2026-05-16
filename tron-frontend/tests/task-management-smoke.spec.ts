import { expect, test, type Page, type Route } from "@playwright/test";
import { spawn, type ChildProcessWithoutNullStreams } from "node:child_process";
import { createServer } from "node:net";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const thisFilePath = fileURLToPath(import.meta.url);
const frontendRoot = resolve(thisFilePath, "..", "..");
let frontendProcess: ChildProcessWithoutNullStreams | undefined;
let frontendLogs = "";
let frontendBaseUrl = "";

type TaskStatus = "Backlog" | "Todo" | "InProgress" | "Testing" | "Review" | "Reopen" | "Completed" | "Archived";
type TaskPriority = "Low" | "Medium" | "High";
type TaskType = "Feature" | "BugFix" | "Refactor";
type TaskSourceType = "Internal" | "GitHub" | "GitLab";
type CapturedRequest = { body: unknown; method: string; pathname: string; searchParams: Record<string, string> };
type TaskSummary = { priority: TaskPriority; progress: number; status: TaskStatus; taskCode: string; taskType: TaskType; title: string };
type Task = {
  agentCodes: string[];
  appCodes: string[];
  archivedAt: string | null;
  assigneeUserCodes: string[];
  createdAt: string;
  description: string | null;
  id: string;
  parent: TaskSummary | null;
  parentTaskCode: string | null;
  previousStatus: TaskStatus | null;
  priority: TaskPriority;
  progress: number;
  sourceType: TaskSourceType;
  sourceUrl: string | null;
  status: TaskStatus;
  subtasks: TaskSummary[];
  taskCode: string;
  taskType: TaskType;
  title: string;
  updatedAt: string;
  workspaceCode: string;
};

const statuses: TaskStatus[] = ["Backlog", "Todo", "InProgress", "Testing", "Review", "Reopen", "Completed", "Archived"];
const now = "2026-05-16T12:00:00Z";

const authenticatedSession = {
  authMethod: "internal",
  authenticated: true,
  provider: null,
  user: {
    displayName: "Auth Operator",
    email: "auth.operator@tron.ai",
    roleCodes: ["ADMIN", "WORKSPACE_ADMIN"],
    spaceCodes: ["TRON_CORE", "PLATFORM"],
    status: "Active",
    userCode: "USR-AUTH-0001",
    username: "auth.operator",
  },
};

const workspaceTree = [{
  id: "734400000000000001",
  workspaceCode: "TRON_CORE",
  name: "Tron Core",
  description: "Main Tron console workspace",
  parentWorkspaceCode: null,
  depth: 1,
  status: "Active",
  directUserCount: 1,
  roleGrantCount: 0,
  effectiveUserCount: 1,
  createdAt: now,
  updatedAt: now,
  archivedAt: null,
  children: [],
}];

const effectiveUsers = [{
  userCode: "USR-AUTH-0001",
  displayName: "Auth Operator",
  email: "auth.operator@tron.ai",
  source: "Direct",
  matchedRoleCodes: [],
  isOwner: true,
}];

function task(values: Partial<Task> & Pick<Task, "taskCode" | "title">): Task {
  return {
    agentCodes: [],
    appCodes: [],
    archivedAt: null,
    assigneeUserCodes: ["USR-AUTH-0001"],
    createdAt: now,
    description: null,
    id: values.taskCode.replace("TSK-", "7345"),
    parent: null,
    parentTaskCode: null,
    previousStatus: null,
    priority: "Medium",
    progress: 0,
    sourceType: "Internal",
    sourceUrl: "",
    status: "Backlog",
    subtasks: [],
    taskCode: values.taskCode,
    taskType: "Feature",
    title: values.title,
    updatedAt: now,
    workspaceCode: "TRON_CORE",
    ...values,
  };
}

function successEnvelope(data: unknown) {
  return { code: "SUCCESS", message: "ok", data };
}

function pageEnvelope(records: unknown[], current = 1, size = 100) {
  return successEnvelope({ current, size, total: records.length, records });
}

async function fulfillJson(route: Route, data: unknown, status = 200) {
  await route.fulfill({ status, contentType: "application/json", body: JSON.stringify(data) });
}

function captureRequest(route: Route): CapturedRequest {
  const request = route.request();
  const url = new URL(request.url());
  return { body: request.postDataJSON() ?? null, method: request.method(), pathname: url.pathname, searchParams: Object.fromEntries(url.searchParams.entries()) };
}

function summary(item: Task): TaskSummary {
  return {
    priority: item.priority,
    progress: item.progress,
    status: item.status,
    taskCode: item.taskCode,
    taskType: item.taskType,
    title: item.title,
  };
}

function detail(tasks: Task[], taskCode: string): Task {
  const selected = tasks.find((item) => item.taskCode === taskCode);
  if (!selected) throw new Error(`Task ${taskCode} not found`);
  const parent = selected.parentTaskCode ? tasks.find((item) => item.taskCode === selected.parentTaskCode) : undefined;
  return {
    ...selected,
    parent: parent ? summary(parent) : null,
    subtasks: tasks.filter((item) => item.parentTaskCode === selected.taskCode).map(summary),
  };
}

function listItem(tasks: Task[], item: Task) {
  return {
    agentCodes: item.agentCodes,
    appCodes: item.appCodes,
    archivedAt: item.archivedAt,
    assigneeUserCodes: item.assigneeUserCodes,
    parentTaskCode: item.parentTaskCode,
    priority: item.priority,
    progress: item.progress,
    sourceType: item.sourceType,
    sourceUrl: item.sourceUrl,
    status: item.status,
    subtaskCount: tasks.filter((candidate) => candidate.parentTaskCode === item.taskCode && candidate.status !== "Archived").length,
    taskCode: item.taskCode,
    taskType: item.taskType,
    title: item.title,
    updatedAt: item.updatedAt,
    workspaceCode: item.workspaceCode,
  };
}

function filtered(tasks: Task[], query: Record<string, string>) {
  const includeArchived = query.includeArchived === "true";
  const search = query.search?.trim().toLowerCase();
  return tasks.filter((item) => {
    if (!includeArchived && item.status === "Archived") return false;
    if (query.workspaceCode && item.workspaceCode !== query.workspaceCode) return false;
    if (query.status && item.status !== query.status) return false;
    if (query.priority && item.priority !== query.priority) return false;
    if (!search) return true;
    return `${item.taskCode} ${item.title} ${item.description ?? ""} ${item.sourceUrl ?? ""}`.toLowerCase().includes(search);
  });
}

function toast(page: Page, message: string | RegExp) {
  return page.locator('[aria-label="Notifications"]').getByRole("status").filter({ hasText: message }).first();
}

async function routeApi(page: Page, requests: CapturedRequest[], tasks: Task[]) {
  let next = 1;
  await page.route("**/api/v1/**", async (route) => {
    const captured = captureRequest(route);
    requests.push(captured);

    if (captured.pathname === "/api/v1/auth/providers") return fulfillJson(route, successEnvelope({ providers: [{ code: "internal", name: "Internal Account", type: "internal" }] }));
    if (captured.pathname === "/api/v1/auth/refresh") return fulfillJson(route, successEnvelope({ accessToken: "task-token", tokenType: "Bearer", expiresIn: 900, session: authenticatedSession }));
    if (captured.pathname === "/api/v1/auth/session") return fulfillJson(route, successEnvelope(authenticatedSession));
    if (captured.pathname === "/api/v1/auth/logout") return fulfillJson(route, successEnvelope({ loggedOut: true }));
    if (captured.pathname === "/api/v1/admin/workspaces") return fulfillJson(route, successEnvelope(workspaceTree));
    if (/^\/api\/v1\/admin\/workspaces\/[^/]+\/access\/effective-users$/.test(captured.pathname)) return fulfillJson(route, pageEnvelope(effectiveUsers));
    if (captured.pathname === "/api/v1/tasks/options") return fulfillJson(route, successEnvelope({ priorities: ["Low", "Medium", "High"], sourceTypes: ["Internal", "GitHub", "GitLab"], statuses, taskTypes: ["Feature", "BugFix", "Refactor"] }));
    if (captured.pathname === "/api/v1/tasks/quick-reference") return fulfillJson(route, successEnvelope({ tips: ["Task must belong to a workspace."], variables: ["workspaceCode", "taskType", "sourceType", "sourceUrl", "assigneeUserCodes"] }));
    if (captured.pathname === "/api/v1/tasks/kanban") {
      const records = filtered(tasks, captured.searchParams);
      return fulfillJson(route, successEnvelope(statuses.filter((status) => captured.searchParams.includeArchived === "true" || status !== "Archived").map((status) => ({ status, title: status, tasks: records.filter((item) => item.status === status).map((item) => listItem(tasks, item)) }))));
    }
    if (captured.pathname === "/api/v1/tasks" && captured.method === "GET") return fulfillJson(route, pageEnvelope(filtered(tasks, captured.searchParams).map((item) => listItem(tasks, item))));
    if (captured.pathname === "/api/v1/tasks" && captured.method === "POST") {
      const body = captured.body as Partial<Task>;
      const taskCode = `TSK-PW-${String(next).padStart(4, "0")}`;
      next += 1;
      tasks.push(task({
        agentCodes: body.agentCodes ?? [],
        appCodes: body.appCodes ?? [],
        assigneeUserCodes: body.assigneeUserCodes ?? [],
        description: body.description ?? null,
        parentTaskCode: body.parentTaskCode ?? null,
        priority: body.priority ?? "Medium",
        sourceType: body.sourceType ?? "Internal",
        sourceUrl: body.sourceUrl ?? "",
        taskCode,
        taskType: body.taskType ?? "Feature",
        title: body.title ?? "Untitled task",
        workspaceCode: body.workspaceCode ?? "TRON_CORE",
      }));
      return fulfillJson(route, successEnvelope(detail(tasks, taskCode)), 201);
    }
    const archiveMatch = captured.pathname.match(/^\/api\/v1\/tasks\/([^/]+)\/(archive|restore)$/);
    if (archiveMatch) {
      const selected = tasks.find((item) => item.taskCode === decodeURIComponent(archiveMatch[1]));
      if (selected && archiveMatch[2] === "archive" && selected.status !== "Archived") {
        selected.previousStatus = selected.status;
        selected.status = "Archived";
        selected.archivedAt = now;
      }
      if (selected && archiveMatch[2] === "restore" && selected.status === "Archived") {
        selected.status = selected.previousStatus ?? "Backlog";
        selected.previousStatus = null;
        selected.archivedAt = null;
      }
      return fulfillJson(route, successEnvelope({ taskCode: selected?.taskCode, status: selected?.status, previousStatus: selected?.previousStatus, archivedAt: selected?.archivedAt }));
    }
    const statusMatch = captured.pathname.match(/^\/api\/v1\/tasks\/([^/]+)\/status$/);
    if (statusMatch) {
      const selected = tasks.find((item) => item.taskCode === decodeURIComponent(statusMatch[1]));
      if (!selected) return fulfillJson(route, { code: "TASK_NOT_FOUND", message: "task was not found", data: {} }, 404);
      const body = captured.body as { progress?: number; status: TaskStatus };
      selected.status = body.status;
      selected.progress = body.status === "Completed" ? 100 : body.progress ?? selected.progress;
      selected.updatedAt = now;
      return fulfillJson(route, successEnvelope(detail(tasks, selected.taskCode)));
    }
    const taskMatch = captured.pathname.match(/^\/api\/v1\/tasks\/([^/]+)$/);
    if (taskMatch) {
      const selected = tasks.find((item) => item.taskCode === decodeURIComponent(taskMatch[1]));
      if (!selected) return fulfillJson(route, { code: "TASK_NOT_FOUND", message: "task was not found", data: {} }, 404);
      if (captured.method === "GET") return fulfillJson(route, successEnvelope(detail(tasks, selected.taskCode)));
      const body = captured.body as Partial<Task>;
      if (body.title !== undefined) selected.title = body.title;
      if (body.description !== undefined) selected.description = body.description;
      if (body.priority !== undefined) selected.priority = body.priority;
      if (body.taskType !== undefined) selected.taskType = body.taskType;
      if (body.sourceType !== undefined) selected.sourceType = body.sourceType;
      if (body.sourceUrl !== undefined) selected.sourceUrl = body.sourceUrl;
      if (body.agentCodes !== undefined) selected.agentCodes = body.agentCodes;
      if (body.appCodes !== undefined) selected.appCodes = body.appCodes;
      if (body.assigneeUserCodes !== undefined) selected.assigneeUserCodes = body.assigneeUserCodes;
      selected.updatedAt = now;
      return fulfillJson(route, successEnvelope(detail(tasks, selected.taskCode)));
    }
    return fulfillJson(route, { code: "NOT_FOUND", message: `Unhandled ${captured.method} ${captured.pathname}`, data: {} }, 404);
  });
}

async function reservePort() {
  return new Promise<number>((resolvePort, rejectPort) => {
    const server = createServer();
    server.on("error", rejectPort);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      if (!address || typeof address === "string") return rejectPort(new Error("Failed to reserve a frontend test port."));
      server.close(() => resolvePort(address.port));
    });
  });
}

async function waitForFrontend(url: string) {
  const deadline = Date.now() + 30_000;
  while (Date.now() < deadline) {
    if (frontendProcess?.exitCode !== null) throw new Error(`Frontend dev server exited early.\n${frontendLogs}`);
    try {
      if ((await fetch(url)).ok) return;
    } catch {
      // dev server is still starting
    }
    await new Promise((resolveDelay) => setTimeout(resolveDelay, 250));
  }
  throw new Error(`Frontend dev server did not become ready.\n${frontendLogs}`);
}

test.beforeAll(async () => {
  const port = await reservePort();
  frontendBaseUrl = `http://127.0.0.1:${port}`;
  frontendProcess = spawn("npm", ["run", "dev", "--", "--host", "127.0.0.1", "--port", String(port)], {
    cwd: frontendRoot,
    env: { ...process.env, CI: "1", VITE_TRON_ADMIN_DATA_SOURCE: "api", VITE_TRON_DATA_SOURCE: "api", VITE_TRON_TASK_DATA_SOURCE: "api" },
    stdio: "pipe",
  });
  frontendProcess.stdout.on("data", (chunk) => { frontendLogs = `${frontendLogs}${chunk.toString()}`.slice(-5000); });
  frontendProcess.stderr.on("data", (chunk) => { frontendLogs = `${frontendLogs}${chunk.toString()}`.slice(-5000); });
  await waitForFrontend(`${frontendBaseUrl}/tasks`);
});

test.afterAll(async () => {
  if (!frontendProcess || frontendProcess.killed) return;
  frontendProcess.kill("SIGTERM");
  await new Promise((resolveProcess) => frontendProcess?.once("exit", resolveProcess));
});

test("task management supports kanban search create detail updates subtasks archive restore execution toast and no import export actions", async ({ page }) => {
  const requests: CapturedRequest[] = [];
  const tasks: Task[] = [
    task({ description: "Seeded task for board search", priority: "Medium", taskCode: "TSK-SEED-BACKLOG", title: "Design task management v1" }),
    task({ description: "Active implementation fixture", priority: "High", progress: 65, status: "Testing", taskCode: "TSK-SEED-TESTING", taskType: "BugFix", title: "Verify task smoke flow" }),
    task({ archivedAt: now, description: "Archived board fixture", previousStatus: "Todo", status: "Archived", taskCode: "TSK-SEED-ARCHIVED", taskType: "Refactor", title: "Archived task example" }),
  ];
  await routeApi(page, requests, tasks);

  await page.goto(`${frontendBaseUrl}/tasks`);
  const taskHeader = page.locator("header").filter({ has: page.getByRole("heading", { name: "Task Management" }) });
  await expect(taskHeader.getByText("Kanban", { exact: true })).toHaveCount(0);
  await expect(page.getByRole("button", { name: /^Refresh$/ })).toBeVisible();
  await expect(page.getByRole("button", { name: /^Sync$/ })).toHaveCount(0);
  await page.getByRole("button", { name: /^Refresh$/ }).click();
  await expect(toast(page, "Task board refreshed")).toBeVisible();
  await expect(page.getByLabel("Task workspace")).toBeVisible();
  await expect(page.getByLabel("Task priority filter")).toBeVisible();
  await expect(page.getByLabel("Task status filter")).toHaveCount(0);
  await page.getByLabel("Task workspace").selectOption("TRON_CORE");
  await expect(page.getByRole("button", { name: /^Import$/ })).toHaveCount(0);
  await expect(page.getByRole("button", { name: /^Export$/ })).toHaveCount(0);
  await page.getByPlaceholder("Filter task code, title, or description...").fill("Design");
  await expect(page.getByText("Design task management v1")).toBeVisible();
  await expect.poll(() => requests.some((request) => request.pathname === "/api/v1/tasks/kanban" && request.searchParams.search === "Design")).toBe(true);

  await page.getByPlaceholder("Filter task code, title, or description...").fill("");
  await page.getByLabel("Task priority filter").selectOption("High");
  await expect(page.getByText("Verify task smoke flow")).toBeVisible();
  const highPriorityTaskCard = page.getByRole("button", { name: /Verify task smoke flow/ });
  await expect.poll(async () => {
    const taskTypeClass = await highPriorityTaskCard.getByText("BugFix", { exact: true }).getAttribute("class");
    const sourceTypeClass = await highPriorityTaskCard.getByText("Internal", { exact: true }).getAttribute("class");
    return taskTypeClass !== null && sourceTypeClass !== null && taskTypeClass !== sourceTypeClass;
  }).toBe(true);
  await expect.poll(() => requests.some((request) => request.pathname === "/api/v1/tasks/kanban" && request.searchParams.priority === "High")).toBe(true);

  await page.getByRole("button", { name: /Create Task/ }).click();
  await expect(page.getByRole("heading", { name: "Create Task" })).toBeVisible();
  await expect(page.getByLabel("Task Type").locator("option", { hasText: "Etc" })).toHaveCount(0);
  await page.getByLabel("Task title").fill("Playwright created task");
  await page.getByLabel("Priority").selectOption("High");
  await page.getByLabel("Task Type").selectOption("BugFix");
  await page.getByLabel("Source Type").selectOption("GitHub");
  await page.getByLabel("Source URL").fill("https://github.com/clouds3n/tron/issues/100");
  await page.getByLabel("Agent Codes").fill("agent-reviewer, agent-builder");
  await page.getByLabel("App Codes").fill("github, notion");
  await page.getByLabel("Assign Auth Operator").check();
  await expect(page.getByLabel("Task description")).toHaveClass(/w-full/);
  await page.getByLabel("Task description").fill("Created through the task-management smoke.");
  await page.getByRole("button", { name: /Create Task/ }).click();
  await expect(page).toHaveURL(/\/tasks\/detail\?taskCode=TSK-PW-0001/);
  await expect(page.getByRole("heading", { name: "Task Details" })).toBeVisible();
  await expect(page.getByLabel("Task detail title")).toHaveValue("Playwright created task");
  await expect.poll(() => {
    const body = requests.find((request) => request.method === "POST" && request.pathname === "/api/v1/tasks")?.body as Partial<Task> | undefined;
    return body?.taskType === "BugFix" && body.sourceType === "GitHub" && body.agentCodes?.includes("agent-reviewer") === true && body.appCodes?.includes("notion") === true;
  }).toBe(true);

  await page.getByLabel("Task detail title").fill("Playwright created task updated");
  await page.getByLabel("Task detail type").selectOption("Refactor");
  await page.getByLabel("Task detail source type").selectOption("GitLab");
  await page.getByLabel("Task detail source URL").fill("https://gitlab.com/clouds3n/tron/-/issues/101");
  await page.getByRole("button", { name: /Save Details/ }).click();
  await expect(toast(page, /Task TSK-PW-0001 updated/)).toBeVisible();

  await page.getByLabel("Task detail status").selectOption("Testing");
  await page.getByLabel("Task detail progress").fill("55");
  await page.getByRole("button", { name: "Update Status" }).click();
  await expect(toast(page, /Task TSK-PW-0001 status updated/)).toBeVisible();
  await expect.poll(() => requests.some((request) => request.pathname === "/api/v1/tasks/TSK-PW-0001/status" && (request.body as { progress?: number; status?: TaskStatus } | null)?.progress === 55 && (request.body as { status?: TaskStatus } | null)?.status === "Testing")).toBe(true);

  await page.getByLabel("New subtask title").fill("Smoke child task");
  await page.getByRole("button", { name: /Add Subtask/ }).click();
  await expect(toast(page, /Subtask TSK-PW-0002 created/)).toBeVisible();
  await expect(page.getByText("Smoke child task")).toBeVisible();
  await expect.poll(() => requests.some((request) => request.method === "POST" && request.pathname === "/api/v1/tasks" && (request.body as { parentTaskCode?: string } | null)?.parentTaskCode === "TSK-PW-0001")).toBe(true);

  await page.getByRole("button", { name: "Archive" }).click();
  await expect(toast(page, "Task TSK-PW-0001 archived")).toBeVisible();
  await expect(page.getByRole("button", { name: "Restore" })).toBeVisible();
  await page.getByRole("button", { name: "Restore" }).click();
  await expect(toast(page, "Task TSK-PW-0001 restored")).toBeVisible();
  await expect(page.getByRole("button", { name: "Archive" })).toBeVisible();

  await page.getByRole("button", { name: "Execute Task" }).click();
  await expect(toast(page, "Task execution is not available in v1")).toBeVisible();
});
