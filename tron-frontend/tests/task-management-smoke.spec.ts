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
type WorkspaceSwitcherPayload = typeof workspaceSwitcher;
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

const workspaceOptions = [{
  workspaceCode: "TRON_CORE",
  label: "Enterprise / Platform / Tron Core",
  name: "Tron Core",
  description: "Main Tron console workspace",
  parentWorkspaceCode: "PLATFORM",
  depth: 3,
  status: "Active",
}];

const workspaceSwitcher = {
  primarySpaces: [{
    workspaceCode: "ENTERPRISE",
    name: "Enterprise",
    description: "Global governance workspace",
    childCount: 1,
    status: "Active",
  }],
  secondarySpaces: [{
    workspaceCode: "TRON_CORE",
    primaryWorkspaceCode: "ENTERPRISE",
    label: "Enterprise / Platform / Tron Core",
    name: "Tron Core",
    description: "Main Tron console workspace",
    depth: 3,
    childCount: 0,
    effectiveUserCount: 1,
    status: "Active",
  }],
};

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

function effectiveProgress(tasks: Task[], item: Task): number {
  const activeSubtasks = tasks.filter((candidate) => candidate.parentTaskCode === item.taskCode && candidate.status !== "Archived");
  if (!activeSubtasks.length) return item.progress;
  const total = activeSubtasks.reduce((sum, candidate) => sum + effectiveProgress(tasks, candidate), 0);
  return Math.round(total / activeSubtasks.length);
}

function summary(tasks: Task[], item: Task): TaskSummary {
  return {
    priority: item.priority,
    progress: effectiveProgress(tasks, item),
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
    progress: effectiveProgress(tasks, selected),
    parent: parent ? summary(tasks, parent) : null,
    subtasks: tasks.filter((item) => item.parentTaskCode === selected.taskCode && item.status !== "Archived").map((item) => summary(tasks, item)),
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
    progress: effectiveProgress(tasks, item),
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

async function routeApi(
  page: Page,
  requests: CapturedRequest[],
  tasks: Task[],
  options: { workspaceSwitcherResolver?: (requestCount: number) => WorkspaceSwitcherPayload } = {},
) {
  let next = 1;
  let workspaceSwitcherRequestCount = 0;
  await page.route("**/api/v1/**", async (route) => {
    const captured = captureRequest(route);
    requests.push(captured);

    if (captured.pathname === "/api/v1/auth/providers") return fulfillJson(route, successEnvelope({ providers: [{ code: "internal", name: "Internal Account", type: "internal" }] }));
    if (captured.pathname === "/api/v1/auth/refresh") return fulfillJson(route, successEnvelope({ accessToken: "task-token", tokenType: "Bearer", expiresIn: 900, session: authenticatedSession }));
    if (captured.pathname === "/api/v1/auth/session") return fulfillJson(route, successEnvelope(authenticatedSession));
    if (captured.pathname === "/api/v1/auth/logout") return fulfillJson(route, successEnvelope({ loggedOut: true }));
    if (captured.pathname === "/api/v1/admin/workspaces") return fulfillJson(route, successEnvelope(workspaceTree));
    if (captured.pathname === "/api/v1/admin/workspaces/options") return fulfillJson(route, successEnvelope(workspaceOptions));
    if (captured.pathname === "/api/v1/admin/workspaces/switcher") {
      workspaceSwitcherRequestCount += 1;
      const payload = options.workspaceSwitcherResolver?.(workspaceSwitcherRequestCount) ?? workspaceSwitcher;
      return fulfillJson(route, successEnvelope(payload));
    }
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
      if (selected && archiveMatch[2] === "archive" && tasks.some((item) => item.parentTaskCode === selected.taskCode && item.status !== "Archived")) {
        return fulfillJson(route, { code: "TASK_ACTIVE_SUBTASK_EXISTS", message: "archive active subtasks before archiving the parent task", data: {} }, 400);
      }
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
  await expect(page.getByLabel("Task workspace")).toHaveCount(0);
  await expect(page.getByLabel("Task priority filter")).toBeVisible();
  await expect(page.getByLabel("Task status filter")).toHaveCount(0);
  await expect.poll(() => requests.some((request) => request.pathname === "/api/v1/tasks/kanban" && request.searchParams.workspaceCode === "TRON_CORE")).toBe(true);
  await expect(page.getByRole("button", { name: /^Import$/ })).toHaveCount(0);
  await expect(page.getByRole("button", { name: /^Export$/ })).toHaveCount(0);
  await page.getByPlaceholder("Filter task code, title, or description...").fill("Design");
  await expect(page.getByText("Design task management v1")).toBeVisible();
  await expect.poll(() => requests.some((request) => request.pathname === "/api/v1/tasks/kanban" && request.searchParams.search === "Design" && request.searchParams.workspaceCode === "TRON_CORE")).toBe(true);

  await page.getByPlaceholder("Filter task code, title, or description...").fill("");
  await page.getByLabel("Task priority filter").selectOption("High");
  await expect(page.getByText("Verify task smoke flow")).toBeVisible();
  const highPriorityTaskCard = page.getByRole("button", { name: /Verify task smoke flow/ });
  await expect.poll(async () => {
    const taskTypeClass = await highPriorityTaskCard.getByText("BugFix", { exact: true }).getAttribute("class");
    const sourceTypeClass = await highPriorityTaskCard.getByText("Internal", { exact: true }).getAttribute("class");
    return taskTypeClass !== null && sourceTypeClass !== null && taskTypeClass !== sourceTypeClass;
  }).toBe(true);
  await expect.poll(() => requests.some((request) => request.pathname === "/api/v1/tasks/kanban" && request.searchParams.priority === "High" && request.searchParams.workspaceCode === "TRON_CORE")).toBe(true);

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
  await page.getByLabel("Task description").fill("## Playwright Notes\n- Created through the task-management smoke.");
  await page.getByRole("button", { name: /Create Task/ }).click();
  await expect(page).toHaveURL(/\/tasks\/detail\?taskCode=TSK-PW-0001/);
  await expect(page.getByRole("heading", { name: "Task Details" })).toBeVisible();
  await expect(page.getByText("Playwright created task")).toBeVisible();
  await expect(page.locator("main > div.grid > div.space-y-5 > section > div.mb-5 > div > h2")).toHaveText(["Task Attributes", "Task Definition"]);
  await expect(page.getByRole("heading", { name: "Task Definition" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Task Attributes" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Updates" })).toHaveCount(0);
  await expect(page.getByText("Operational Updates", { exact: true })).toHaveCount(0);
  await expect(page.getByText("Changes save automatically.", { exact: true })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Archive" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Restore" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Save Updates" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Reset Updates" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Edit Details" })).toBeVisible();
  await expect(page.getByLabel("Task detail title")).toHaveCount(0);
  await expect(page.getByLabel("Task detail type")).toHaveCount(0);
  await expect(page.getByLabel("Task detail source type")).toHaveCount(0);
  await expect(page.getByLabel("Task detail source URL")).toHaveCount(0);
  await expect(page.getByLabel("Task detail status")).toBeVisible();
  await expect(page.getByLabel("Task detail priority")).toBeVisible();
  await expect(page.getByLabel("Task detail progress")).toBeVisible();
  await expect(page.getByLabel("Task detail agent codes")).toBeVisible();
  await expect(page.getByLabel("Task detail app codes")).toBeVisible();
  const descriptionMarkdown = page.getByLabel("Task description markdown");
  await expect(descriptionMarkdown.getByText("Playwright Notes", { exact: true })).toBeVisible();
  await expect(descriptionMarkdown.getByRole("listitem").filter({ hasText: "Created through the task-management smoke." })).toBeVisible();
  await expect(descriptionMarkdown.getByText("## Playwright Notes", { exact: true })).toHaveCount(0);
  await page.getByRole("button", { name: "Open description fullscreen" }).click();
  const descriptionFullscreenPreview = page.getByRole("dialog", { name: "Description fullscreen preview" });
  await expect(descriptionFullscreenPreview).toBeVisible();
  await expect(descriptionFullscreenPreview.getByText("Playwright Notes", { exact: true })).toBeVisible();
  await descriptionFullscreenPreview.getByRole("button", { name: "Close description fullscreen" }).click();
  await expect(descriptionFullscreenPreview).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Update Status" })).toHaveCount(0);
  await expect(page.locator("span.inline-flex.h-6").filter({ hasText: "BugFix" })).toHaveCount(0);
  await expect.poll(() => {
    const body = requests.find((request) => request.method === "POST" && request.pathname === "/api/v1/tasks")?.body as Partial<Task> | undefined;
    return body?.taskType === "BugFix" && body.sourceType === "GitHub" && body.agentCodes?.includes("agent-reviewer") === true && body.appCodes?.includes("notion") === true;
  }).toBe(true);

  await page.getByLabel("Task detail priority").selectOption("Low");
  await expect(toast(page, /Task TSK-PW-0001 updates saved/)).toBeVisible();
  await page.getByLabel("Task detail status").selectOption("Testing");
  await expect(toast(page, /Task TSK-PW-0001 updates saved/)).toBeVisible();
  await page.getByLabel("Task detail progress").fill("55");
  await page.getByLabel("Task detail progress").press("Tab");
  await page.getByLabel("Task detail agent codes").fill("agent-reviewer, agent-builder, agent-qa");
  await page.getByLabel("Task detail agent codes").press("Tab");
  await page.getByLabel("Task detail app codes").fill("github, notion, slack");
  await page.getByLabel("Task detail app codes").press("Tab");
  await expect(toast(page, /Task TSK-PW-0001 updates saved/)).toBeVisible();
  await expect.poll(() => requests.some((request) => request.pathname === "/api/v1/tasks/TSK-PW-0001" && request.method !== "GET" && (request.body as Partial<Task> | null)?.priority === "Low")).toBe(true);
  await expect.poll(() => requests.some((request) => request.pathname === "/api/v1/tasks/TSK-PW-0001" && request.method !== "GET" && (request.body as Partial<Task> | null)?.agentCodes?.includes("agent-qa") === true)).toBe(true);
  await expect.poll(() => requests.some((request) => request.pathname === "/api/v1/tasks/TSK-PW-0001" && request.method !== "GET" && (request.body as Partial<Task> | null)?.appCodes?.includes("slack") === true)).toBe(true);
  await expect.poll(() => requests.some((request) => request.pathname === "/api/v1/tasks/TSK-PW-0001/status" && (request.body as { progress?: number; status?: TaskStatus } | null)?.progress === 55 && (request.body as { status?: TaskStatus } | null)?.status === "Testing")).toBe(true);

  await page.getByRole("button", { name: "Edit Details" }).click();
  await page.getByLabel("Task detail title").fill("Playwright created task updated");
  await page.getByLabel("Task detail type").selectOption("Refactor");
  await page.getByLabel("Task detail source type").selectOption("GitLab");
  await page.getByLabel("Task detail source URL").fill("https://gitlab.com/clouds3n/tron/-/issues/101");
  await page.getByRole("button", { name: "Open description fullscreen" }).click();
  const descriptionFullscreenEditor = page.getByRole("dialog", { name: "Description fullscreen editor" });
  await expect(descriptionFullscreenEditor).toBeVisible();
  await descriptionFullscreenEditor.getByLabel("Task detail description fullscreen").fill("## Playwright Notes Updated\n- Fullscreen edit path");
  await descriptionFullscreenEditor.getByRole("button", { name: "Close description fullscreen" }).click();
  await expect(descriptionFullscreenEditor).toHaveCount(0);
  await page.getByRole("button", { name: "Save Details" }).click();
  await expect(toast(page, /Task TSK-PW-0001 details saved/)).toBeVisible();
  await expect(page.getByLabel("Task description markdown").getByText("Playwright Notes Updated", { exact: true })).toBeVisible();
  await expect(page.getByLabel("Task description markdown").getByRole("listitem").filter({ hasText: "Fullscreen edit path" })).toBeVisible();
  await expect.poll(() => requests.some((request) => request.pathname === "/api/v1/tasks/TSK-PW-0001" && request.method !== "GET" && (request.body as Partial<Task> | null)?.taskType === "Refactor" && (request.body as Partial<Task> | null)?.sourceType === "GitLab" && (request.body as Partial<Task> | null)?.title === "Playwright created task updated")).toBe(true);

  await expect(page.getByRole("button", { name: /Add Subtask/ })).toBeVisible();
  await page.getByLabel("New subtask title").fill("Smoke child task");
  await page.getByRole("button", { name: /Add Subtask/ }).click();
  await expect(toast(page, /Subtask TSK-PW-0002 created/)).toBeVisible();
  await expect(page.getByText("Smoke child task")).toBeVisible();
  await expect.poll(() => requests.some((request) => request.method === "POST" && request.pathname === "/api/v1/tasks" && (request.body as { parentTaskCode?: string } | null)?.parentTaskCode === "TSK-PW-0001")).toBe(true);

  await page.getByLabel("Task detail status").selectOption("Archived");
  await expect(toast(page, "archive active subtasks before archiving the parent task")).toBeVisible();
  await expect(page.getByLabel("Task detail status")).toHaveValue("Testing");

  await page.getByRole("link", { name: /Smoke child task/ }).click();
  await expect(page).toHaveURL(/\/tasks\/detail\?taskCode=TSK-PW-0002/);
  const childBreadcrumb = page.getByRole("navigation", { name: "Breadcrumb" });
  await expect(childBreadcrumb.getByRole("link", { name: "Tasks" })).toBeVisible();
  await expect(childBreadcrumb.getByRole("link", { name: "TSK-PW-0001" })).toBeVisible();
  await expect(childBreadcrumb.getByText("TSK-PW-0002", { exact: true })).toBeVisible();
  await page.getByLabel("Task detail status").selectOption("Archived");
  await expect(toast(page, "Task TSK-PW-0002 archived")).toBeVisible();
  await expect(page.getByLabel("Task detail status")).toHaveValue("Archived");
  await childBreadcrumb.getByRole("link", { name: "TSK-PW-0001" }).click();
  await expect(page).toHaveURL(/\/tasks\/detail\?taskCode=TSK-PW-0001/);
  await expect(page.getByText("No active subtasks.")).toBeVisible();

  await page.getByLabel("Task detail status").selectOption("Archived");
  await expect.poll(() => requests.some((request) => request.method === "POST" && request.pathname === "/api/v1/tasks/TSK-PW-0001/archive")).toBe(true);
  await expect(page.getByLabel("Task detail status")).toHaveValue("Archived");
  await expect(page.getByText("Not archived", { exact: true })).toHaveCount(0);
  await page.getByLabel("Task detail status").selectOption("Backlog");
  await expect.poll(() => requests.some((request) => request.method === "POST" && request.pathname === "/api/v1/tasks/TSK-PW-0001/restore")).toBe(true);
  await expect(page.getByLabel("Task detail status")).toHaveValue("Backlog");

  await expect(page.getByRole("button", { name: "Execute Task" })).toHaveCount(0);
  await page.getByRole("button", { name: "Execute" }).click();
  await expect(toast(page, "Task execution is not available in v1")).toBeVisible();
});

test("parent task detail derives progress from active subtasks and keeps the progress field read-only", async ({ page }) => {
  const requests: CapturedRequest[] = [];
  const tasks: Task[] = [
    task({ priority: "High", progress: 0, status: "InProgress", taskCode: "TSK-PROGRESS-PARENT", title: "Parent progress task" }),
    task({ parentTaskCode: "TSK-PROGRESS-PARENT", priority: "High", progress: 100, status: "Completed", taskCode: "TSK-PROGRESS-CHILD-1", title: "Completed child" }),
    task({ parentTaskCode: "TSK-PROGRESS-PARENT", priority: "Low", progress: 0, status: "Backlog", taskCode: "TSK-PROGRESS-CHILD-2", title: "Backlog child" }),
  ];
  await routeApi(page, requests, tasks);

  await page.goto(`${frontendBaseUrl}/tasks/detail?taskCode=TSK-PROGRESS-PARENT`);
  await expect(page.getByRole("heading", { name: "Task Details" })).toBeVisible();
  await expect(page.getByLabel("Task detail progress")).toHaveValue("50");
  await expect(page.getByLabel("Task detail progress")).toBeDisabled();
  await expect(page.getByText("Completed child")).toBeVisible();
  await expect(page.getByText("Backlog child")).toBeVisible();
});

test("task detail description preview stays bounded inline and expands in fullscreen", async ({ page }) => {
  const longDescription = [
    "## Playwright Notes",
    "",
    "- Description preview should stay compact.",
    "",
    ...Array.from({ length: 18 }, (_, index) => `Paragraph ${index + 1}: Lorem ipsum dolor sit amet.`),
  ].join("\n");
  const tasks: Task[] = [
    task({
      description: longDescription,
      taskCode: "TSK-DESC-PREVIEW",
      title: "Description preview coverage",
    }),
  ];
  await routeApi(page, [], tasks);

  await page.goto(`${frontendBaseUrl}/tasks/detail?taskCode=TSK-DESC-PREVIEW`);
  await expect(page.getByRole("heading", { name: "Task Details" })).toBeVisible();

  const descriptionMarkdown = page.getByLabel("Task description markdown");
  await expect(descriptionMarkdown.getByText("Playwright Notes", { exact: true })).toBeVisible();
  const inlinePreview = await descriptionMarkdown.evaluate((node) => {
    const style = window.getComputedStyle(node);
    return {
      clientHeight: node.clientHeight,
      hasOverflow: node.scrollHeight > node.clientHeight,
      maxHeight: style.maxHeight,
      overflowY: style.overflowY,
    };
  });
  expect(inlinePreview.maxHeight).not.toBe("none");
  expect(inlinePreview.overflowY).toBe("auto");
  expect(inlinePreview.hasOverflow).toBe(true);
  expect(inlinePreview.clientHeight).toBeGreaterThan(0);

  await page.getByRole("button", { name: "Open description fullscreen" }).click();
  const descriptionFullscreenPreview = page.getByRole("dialog", { name: "Description fullscreen preview" });
  await expect(descriptionFullscreenPreview).toBeVisible();
  await expect(descriptionFullscreenPreview.getByText("Playwright Notes", { exact: true })).toBeVisible();
  const fullscreenMarkdown = page.getByLabel("Task description markdown fullscreen");
  const fullscreenPreview = await fullscreenMarkdown.evaluate((node) => {
    const style = window.getComputedStyle(node);
    return {
      maxHeight: style.maxHeight,
    };
  });
  expect(fullscreenPreview.maxHeight).toBe("none");
  await descriptionFullscreenPreview.getByRole("button", { name: "Close description fullscreen" }).click();
  await expect(descriptionFullscreenPreview).toHaveCount(0);
});

test("task shell exposes a navigation drawer below the desktop breakpoint", async ({ page }) => {
  const requests: CapturedRequest[] = [];
  const tasks: Task[] = [
    task({ description: "Seeded task for responsive shell coverage", taskCode: "TSK-SEED-MOBILE", title: "Responsive shell coverage" }),
  ];
  await routeApi(page, requests, tasks);

  await page.setViewportSize({ width: 393, height: 852 });
  await page.goto(`${frontendBaseUrl}/tasks`);

  const openNavigationButton = page.getByRole("button", { name: "Open navigation menu" });
  await expect(openNavigationButton).toBeVisible();
  await expect(page.locator("#tron-mobile-sidebar")).toHaveCount(0);

  await openNavigationButton.click();

  const mobileSidebar = page.locator("#tron-mobile-sidebar");
  await expect(mobileSidebar).toBeVisible();
  await expect(mobileSidebar.getByRole("link", { name: "Tasks" })).toBeVisible();
  await mobileSidebar.getByRole("button", { name: "Administration" }).click();
  await expect(mobileSidebar.getByRole("link", { name: "Workspace" })).toBeVisible();

  await page.getByRole("button", { name: "Close navigation menu" }).click();
  await expect(mobileSidebar).toHaveCount(0);
});

test("workspace switcher refreshes from the API on every header click", async ({ page }) => {
  const requests: CapturedRequest[] = [];
  const tasks: Task[] = [
    task({ description: "Seeded task for workspace switcher refresh coverage", taskCode: "TSK-SEED-SWITCHER", title: "Workspace switcher refresh" }),
  ];

  await routeApi(page, requests, tasks, {
    workspaceSwitcherResolver: (requestCount) => ({
      primarySpaces: [{
        workspaceCode: "ENTERPRISE",
        name: `Enterprise Live ${requestCount}`,
        description: `Live workspace payload ${requestCount}`,
        childCount: 1,
        status: "Active",
      }],
      secondarySpaces: [{
        workspaceCode: "TRON_CORE",
        primaryWorkspaceCode: "ENTERPRISE",
        label: `Enterprise Live ${requestCount} / Tron Core`,
        name: `Tron Core Live ${requestCount}`,
        description: `Live console payload ${requestCount}`,
        depth: 3,
        childCount: 0,
        effectiveUserCount: 1,
        status: "Active",
      }],
    }),
  });

  await page.goto(`${frontendBaseUrl}/tasks`);

  const workspaceButton = page.locator("header button[aria-busy]").first();
  const switcherRequestCount = () => requests.filter((request) => request.pathname === "/api/v1/admin/workspaces/switcher").length;
  await expect.poll(switcherRequestCount).toBeGreaterThan(0);
  const initialRequestCount = switcherRequestCount();
  await expect(workspaceButton).toContainText(`Enterprise Live ${initialRequestCount}`);

  await workspaceButton.click();
  await expect.poll(switcherRequestCount).toBe(initialRequestCount + 1);
  await expect(workspaceButton).toContainText(`Enterprise Live ${initialRequestCount + 1}`);
  await expect(page.locator("#tron-workspace-selector-panel")).toBeVisible();

  await workspaceButton.click();
  await expect.poll(switcherRequestCount).toBe(initialRequestCount + 2);
  await expect(workspaceButton).toContainText(`Enterprise Live ${initialRequestCount + 2}`);
});

test("topbar workspace selector exposes the full hierarchy and updates the selected path", async ({ page }) => {
  const requests: CapturedRequest[] = [];
  const tasks: Task[] = [
    task({ description: "Seeded task for sidebar workspace selector coverage", taskCode: "TSK-SEED-SIDEBAR-SWITCHER", title: "Sidebar workspace selector" }),
  ];

  await routeApi(page, requests, tasks, {
    workspaceSwitcherResolver: () => ({
      primarySpaces: [{
        workspaceCode: "ENTERPRISE",
        name: "Enterprise",
        description: "Global governance workspace",
        childCount: 1,
        status: "Active",
      }, {
        workspaceCode: "COMPLIANCE",
        name: "Compliance",
        description: "Regulated operations workspace",
        childCount: 1,
        status: "Active",
      }],
      secondarySpaces: [
        {
          workspaceCode: "ENTERPRISE",
          primaryWorkspaceCode: "ENTERPRISE",
          label: "Enterprise",
          name: "Enterprise",
          description: "Global governance workspace",
          depth: 1,
          childCount: 1,
          effectiveUserCount: 4,
          status: "Active",
        },
        {
          workspaceCode: "PLATFORM",
          primaryWorkspaceCode: "ENTERPRISE",
          label: "Enterprise / Platform",
          name: "Platform",
          description: "Shared agent runtime platform",
          depth: 2,
          childCount: 1,
          effectiveUserCount: 3,
          status: "Active",
        },
        {
          workspaceCode: "TRON_CORE",
          primaryWorkspaceCode: "ENTERPRISE",
          label: "Enterprise / Platform / Tron Core",
          name: "Tron Core",
          description: "Main Tron console workspace",
          depth: 3,
          childCount: 0,
          effectiveUserCount: 2,
          status: "Active",
        },
        {
          workspaceCode: "COMPLIANCE",
          primaryWorkspaceCode: "COMPLIANCE",
          label: "Compliance",
          name: "Compliance",
          description: "Regulated operations workspace",
          depth: 1,
          childCount: 1,
          effectiveUserCount: 2,
          status: "Active",
        },
        {
          workspaceCode: "RISK_OPS",
          primaryWorkspaceCode: "COMPLIANCE",
          label: "Compliance / Risk Ops",
          name: "Risk Ops",
          description: "Regulated delivery oversight",
          depth: 2,
          childCount: 0,
          effectiveUserCount: 1,
          status: "Active",
        },
      ],
    }),
  });

  await page.goto(`${frontendBaseUrl}/tasks`);

  const workspaceTrigger = page.locator("#tron-workspace-trigger");
  const switcherRequestCount = () => requests.filter((request) => request.pathname === "/api/v1/admin/workspaces/switcher").length;
  await expect.poll(switcherRequestCount).toBeGreaterThan(0);
  const initialRequestCount = switcherRequestCount();

  await expect(workspaceTrigger).toContainText("Enterprise/Platform/Tron Core");
  await workspaceTrigger.click();
  await expect.poll(switcherRequestCount).toBe(initialRequestCount + 1);

  const selector = page.locator("#tron-workspace-selector-panel");
  await expect(selector).toBeVisible();
  await expect(page.locator("aside[data-collapsed] #tron-workspace-selector-panel")).toHaveCount(0);
  await expect(selector).not.toContainText("Select a workspace directly from the hierarchy.");
  await expect(selector.getByRole("button", { name: "Enterprise", exact: true })).toBeVisible();
  await expect(selector.getByRole("button", { name: "Compliance", exact: true })).toBeVisible();
  await expect(selector.getByRole("button", { name: "Enterprise/Platform", exact: true })).toBeVisible();
  await expect(selector.getByRole("button", { name: "Enterprise/Platform/Tron Core", exact: true })).toBeVisible();
  await expect(selector.getByRole("button", { name: "Compliance/Risk Ops", exact: true })).toHaveCount(0);
  await expect(selector.getByRole("button", { name: "Expand Compliance", exact: true })).toBeVisible();
  await expect(selector).not.toContainText("effective users");
  await expect(selector).not.toContainText("child spaces");
  await expect(selector).not.toContainText("Status:");
  await expect(selector).not.toContainText("Global governance workspace");
  await expect(selector).not.toContainText("Shared agent runtime platform");

  await selector.getByRole("button", { name: "Enterprise/Platform", exact: true }).click();
  await expect(selector).toHaveCount(0);
  await expect(workspaceTrigger).toContainText("Enterprise/Platform");
  await expect(workspaceTrigger).not.toContainText("Enterprise / Platform");

  const requestCountBeforeReopen = switcherRequestCount();
  await workspaceTrigger.click();
  await expect.poll(switcherRequestCount).toBeGreaterThan(requestCountBeforeReopen);
  await selector.getByRole("button", { name: "Enterprise/Platform/Tron Core", exact: true }).click();
  await expect(workspaceTrigger).toContainText("Enterprise/Platform/Tron Core");
  await expect(workspaceTrigger).not.toContainText("Enterprise / Platform / Tron Core");

  const requestCountBeforeComplianceOpen = switcherRequestCount();
  await workspaceTrigger.click();
  await expect.poll(switcherRequestCount).toBeGreaterThan(requestCountBeforeComplianceOpen);
  await selector.getByRole("button", { name: "Compliance", exact: true }).click();
  await expect(workspaceTrigger).toContainText("Compliance");

  const requestCountBeforeRiskOpsOpen = switcherRequestCount();
  await workspaceTrigger.click();
  await expect.poll(switcherRequestCount).toBeGreaterThan(requestCountBeforeRiskOpsOpen);
  await expect(selector.getByRole("button", { name: "Compliance/Risk Ops", exact: true })).toBeVisible();
  await selector.getByRole("button", { name: "Compliance/Risk Ops", exact: true }).click();
  await expect(workspaceTrigger).toContainText("Compliance/Risk Ops");
});

test("task board uses the exact topbar workspace without inheriting parent workspace tasks", async ({ page }) => {
  const requests: CapturedRequest[] = [];
  const tasks: Task[] = [
    task({ description: "Root workspace task", taskCode: "TSK-SEED-ENTERPRISE", title: "Enterprise parent task", workspaceCode: "ENTERPRISE" }),
    task({ description: "Child workspace task", taskCode: "TSK-SEED-TRON", title: "Tron child task", workspaceCode: "TRON_CORE" }),
  ];

  await routeApi(page, requests, tasks, {
    workspaceSwitcherResolver: () => ({
      primarySpaces: [{
        workspaceCode: "ENTERPRISE",
        name: "Enterprise",
        description: "Global governance workspace",
        childCount: 1,
        status: "Active",
      }],
      secondarySpaces: [
        {
          workspaceCode: "ENTERPRISE",
          primaryWorkspaceCode: "ENTERPRISE",
          label: "Enterprise",
          name: "Enterprise",
          description: "Global governance workspace",
          depth: 1,
          childCount: 1,
          effectiveUserCount: 1,
          status: "Active",
        },
        {
          workspaceCode: "PLATFORM",
          primaryWorkspaceCode: "ENTERPRISE",
          label: "Enterprise / Platform",
          name: "Platform",
          description: "Shared agent runtime platform",
          depth: 2,
          childCount: 1,
          effectiveUserCount: 1,
          status: "Active",
        },
        {
          workspaceCode: "TRON_CORE",
          primaryWorkspaceCode: "ENTERPRISE",
          label: "Enterprise / Platform / Tron Core",
          name: "Tron Core",
          description: "Main Tron console workspace",
          depth: 3,
          childCount: 0,
          effectiveUserCount: 1,
          status: "Active",
        },
      ],
    }),
  });

  await page.goto(`${frontendBaseUrl}/tasks`);

  await expect.poll(() => requests.some((request) => request.pathname === "/api/v1/tasks/kanban" && request.searchParams.workspaceCode === "TRON_CORE")).toBe(true);
  await expect(page.getByText("Tron child task")).toBeVisible();
  await expect(page.getByText("Enterprise parent task")).toHaveCount(0);

  const workspaceTrigger = page.locator("#tron-workspace-trigger");
  await workspaceTrigger.click();
  const selector = page.locator("#tron-workspace-selector-panel");
  await expect(selector).toBeVisible();
  await selector.getByRole("button", { name: "Enterprise", exact: true }).click();

  await expect(workspaceTrigger).toContainText("Enterprise");
  await expect.poll(() => requests.some((request) => request.pathname === "/api/v1/tasks/kanban" && request.searchParams.workspaceCode === "ENTERPRISE")).toBe(true);
  await expect(page.getByText("Enterprise parent task")).toBeVisible();
  await expect(page.getByText("Tron child task")).toHaveCount(0);
});

test("workspace switcher keeps a generic label when the API returns no spaces", async ({ page }) => {
  const requests: CapturedRequest[] = [];
  const tasks: Task[] = [
    task({ description: "Seeded task for empty workspace switcher coverage", taskCode: "TSK-SEED-SWITCHER-EMPTY", title: "Workspace switcher empty state" }),
  ];

  await routeApi(page, requests, tasks, {
    workspaceSwitcherResolver: () => ({
      primarySpaces: [],
      secondarySpaces: [],
    }),
  });

  await page.goto(`${frontendBaseUrl}/tasks`);

  const workspaceButton = page.locator("header button[aria-busy]").first();
  const switcherRequestCount = () => requests.filter((request) => request.pathname === "/api/v1/admin/workspaces/switcher").length;
  await expect.poll(switcherRequestCount).toBeGreaterThan(0);
  const initialRequestCount = switcherRequestCount();
  await expect(workspaceButton).toContainText("Workspace");
  await expect(workspaceButton).not.toContainText("Enterprise Workspace");

  await workspaceButton.click();
  await expect.poll(switcherRequestCount).toBe(initialRequestCount + 1);
  await expect(page.getByText("No primary spaces available.")).toBeVisible();
  await expect(page.getByText("Enterprise Workspace")).toHaveCount(0);
});

test("topbar workspace selector keeps root-only spaces selectable without rendering a sidebar entry", async ({ page }) => {
  const requests: CapturedRequest[] = [];
  const tasks: Task[] = [
    task({ description: "Seeded task for root-only sidebar workspace selector coverage", taskCode: "TSK-SEED-SIDEBAR-ROOT", title: "Sidebar root-only selector" }),
  ];

  await routeApi(page, requests, tasks, {
    workspaceSwitcherResolver: () => ({
      primarySpaces: [{
        workspaceCode: "ENTERPRISE",
        name: "Enterprise",
        description: "Global governance workspace",
        childCount: 0,
        status: "Active",
      }],
      secondarySpaces: [{
        workspaceCode: "ENTERPRISE",
        primaryWorkspaceCode: "ENTERPRISE",
        label: "Enterprise",
        name: "Enterprise",
        description: "Global governance workspace",
        depth: 1,
        childCount: 0,
        effectiveUserCount: 1,
        status: "Active",
      }],
    }),
  });

  await page.goto(`${frontendBaseUrl}/tasks`);

  const workspaceTrigger = page.locator("#tron-workspace-trigger");
  const selector = page.locator("#tron-workspace-selector-panel");
  await expect(workspaceTrigger).toContainText("Enterprise");
  await workspaceTrigger.click();
  await expect(selector).toBeVisible();
  await expect(selector.getByRole("button", { name: "Enterprise", exact: true })).toBeVisible();
  await expect(selector.getByRole("button", { name: /Enterprise\// })).toHaveCount(0);
  await selector.getByRole("button", { name: "Enterprise", exact: true }).click();
  await expect(selector).toHaveCount(0);
  await expect(workspaceTrigger).toContainText("Enterprise");
});
