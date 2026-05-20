import { expect, test, type Page, type Route } from "@playwright/test";
import {
  spawn,
  type ChildProcessWithoutNullStreams,
} from "node:child_process";
import { createServer } from "node:net";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const thisFilePath = fileURLToPath(import.meta.url);
const frontendRoot = resolve(thisFilePath, "..", "..");
let frontendProcess: ChildProcessWithoutNullStreams | undefined;
let frontendLogs = "";
let frontendBaseUrl = "";

type CapturedRequest = {
  body: unknown;
  method: string;
  pathname: string;
  searchParams: Record<string, string>;
};
type AutomationStatus = "Active" | "Paused" | "Archived";
type AutomationExecutionStatus = "Pending" | "Running" | "Success" | "Failed" | "Canceled" | "Skipped";
type AutomationTriggerSource = "Scheduler" | "Manual";
type Automation = {
  archivedAt: string | null;
  automationCode: string;
  createdAt: string;
  cronExpression: string;
  description: string | null;
  id: string;
  lastExecutionCode: string | null;
  lastExecutionStatus: AutomationExecutionStatus | null;
  lastExecutionSummary: string | null;
  lastFinishedAt: string | null;
  lastStartedAt: string | null;
  name: string;
  previousStatus: AutomationStatus | null;
  prompt: string;
  schedulerExpression: string;
  status: AutomationStatus;
  timezone: string;
  updatedAt: string;
  workspaceCode: string;
};
type AutomationExecution = {
  automationCode: string;
  createdAt: string;
  executionCode: string;
  finishedAt: string | null;
  scheduledAt: string | null;
  startedAt: string;
  status: AutomationExecutionStatus;
  summary: string | null;
  triggerSource: AutomationTriggerSource;
  workspaceCode: string;
};

const now = "2026-05-19T12:00:00Z";
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
} as const;

function successEnvelope(data: unknown) {
  return { code: "SUCCESS", message: "ok", data };
}

function pageEnvelope(records: unknown[], current = 1, size = 15) {
  return successEnvelope({ current, size, total: records.length, records });
}

async function fulfillJson(route: Route, data: unknown, status = 200) {
  await route.fulfill({
    status,
    contentType: "application/json",
    body: JSON.stringify(data),
  });
}

function captureRequest(route: Route): CapturedRequest {
  const request = route.request();
  const url = new URL(request.url());
  return {
    body: request.postDataJSON() ?? null,
    method: request.method(),
    pathname: url.pathname,
    searchParams: Object.fromEntries(url.searchParams.entries()),
  };
}

function toast(page: Page, message: string | RegExp) {
  return page.locator('[aria-label="Notifications"]').getByRole("status").filter({ hasText: message }).first();
}

function nextRunAt(status: AutomationStatus) {
  return status === "Active" ? "2026-05-20T09:00:00Z" : null;
}

function detail(automation: Automation) {
  return {
    ...automation,
    nextRunAt: nextRunAt(automation.status),
  };
}

function listItem(automation: Automation) {
  return {
    archivedAt: automation.archivedAt,
    automationCode: automation.automationCode,
    cronExpression: automation.cronExpression,
    description: automation.description,
    lastExecutionCode: automation.lastExecutionCode,
    lastExecutionStatus: automation.lastExecutionStatus,
    lastExecutionSummary: automation.lastExecutionSummary,
    lastFinishedAt: automation.lastFinishedAt,
    lastStartedAt: automation.lastStartedAt,
    name: automation.name,
    nextRunAt: nextRunAt(automation.status),
    status: automation.status,
    timezone: automation.timezone,
    updatedAt: automation.updatedAt,
    workspaceCode: automation.workspaceCode,
  };
}

function createAutomationFixture(values: Partial<Automation> & Pick<Automation, "automationCode" | "id" | "name">): Automation {
  return {
    archivedAt: null,
    automationCode: values.automationCode,
    createdAt: now,
    cronExpression: "0 9 * * *",
    description: null,
    id: values.id,
    lastExecutionCode: null,
    lastExecutionStatus: null,
    lastExecutionSummary: null,
    lastFinishedAt: null,
    lastStartedAt: null,
    name: values.name,
    previousStatus: null,
    prompt: "Write a detailed workspace digest.",
    schedulerExpression: "0 0 9 * * *",
    status: "Active",
    timezone: "UTC",
    updatedAt: now,
    workspaceCode: "TRON_CORE",
    ...values,
  };
}

async function routeApi(
  page: Page,
  requests: CapturedRequest[],
  automations: Automation[],
  executions: AutomationExecution[],
) {
  await page.route("**/api/v1/**", async (route) => {
    const captured = captureRequest(route);
    requests.push(captured);

    if (captured.pathname === "/api/v1/auth/providers") {
      return fulfillJson(route, successEnvelope({ providers: [{ code: "internal", name: "Internal Account", type: "internal" }] }));
    }
    if (captured.pathname === "/api/v1/auth/refresh") {
      return fulfillJson(route, successEnvelope({ accessToken: "automation-token", tokenType: "Bearer", expiresIn: 900, session: authenticatedSession }));
    }
    if (captured.pathname === "/api/v1/auth/session") {
      return fulfillJson(route, successEnvelope(authenticatedSession));
    }
    if (captured.pathname === "/api/v1/auth/logout") {
      return fulfillJson(route, successEnvelope({ loggedOut: true }));
    }
    if (captured.pathname === "/api/v1/admin/workspaces/switcher") {
      return fulfillJson(route, successEnvelope(workspaceSwitcher));
    }
    if (captured.pathname === "/api/v1/automations/options") {
      return fulfillJson(route, successEnvelope({
        defaultStatus: "Paused",
        executionStatuses: ["Pending", "Running", "Success", "Failed", "Canceled", "Skipped"],
        statuses: ["Active", "Paused", "Archived"],
        timezones: ["UTC"],
      }));
    }
    if (captured.pathname === "/api/v1/automations" && captured.method === "GET") {
      const includeArchived = captured.searchParams.includeArchived === "true";
      const search = captured.searchParams.search?.trim().toLowerCase();
      const workspaceCode = captured.searchParams.workspaceCode;
      const status = captured.searchParams.status as AutomationStatus | undefined;
      const records = automations
        .filter((automation) => !workspaceCode || automation.workspaceCode === workspaceCode)
        .filter((automation) => status ? automation.status === status : includeArchived || automation.status !== "Archived")
        .filter((automation) => {
          if (!search) return true;
          return [
            automation.automationCode,
            automation.name,
            automation.description ?? "",
            automation.lastExecutionSummary ?? "",
          ].join(" ").toLowerCase().includes(search);
        })
        .map(listItem);
      return fulfillJson(route, pageEnvelope(records));
    }
    if (captured.pathname === "/api/v1/automations" && captured.method === "POST") {
      const body = captured.body as Partial<Automation>;
      const nextId = `73460000000000000${automations.length + 1}`;
      const automationCode = `AUT-PW-000${automations.length + 1}`;
      const cronExpression = body.cronExpression ?? "0 9 * * *";
      const schedulerExpression = cronExpression.split(/\s+/).filter(Boolean).length === 5 ? `0 ${cronExpression}` : cronExpression;
      const created = createAutomationFixture({
        automationCode,
        createdAt: now,
        cronExpression,
        description: typeof body.description === "string" ? body.description : null,
        id: nextId,
        name: body.name ?? "Created automation",
        prompt: body.prompt ?? "",
        schedulerExpression,
        status: (body.status as AutomationStatus | undefined) ?? "Paused",
        updatedAt: now,
        workspaceCode: body.workspaceCode ?? "TRON_CORE",
      });
      automations.unshift(created);
      return fulfillJson(route, successEnvelope(detail(created)), 201);
    }

    const detailMatch = captured.pathname.match(/^\/api\/v1\/automations\/([^/]+)$/);
    if (detailMatch) {
      const automationCode = decodeURIComponent(detailMatch[1]);
      const automation = automations.find((item) => item.automationCode === automationCode);
      if (!automation) {
        return fulfillJson(route, { code: "AUTOMATION_NOT_FOUND", message: "automation was not found", data: {} }, 404);
      }
      if (captured.method === "GET") {
        return fulfillJson(route, successEnvelope(detail(automation)));
      }
      if (captured.method === "PATCH") {
        const body = captured.body as Partial<Automation>;
        automation.name = body.name ?? automation.name;
        automation.description = typeof body.description === "string" ? body.description : automation.description;
        automation.prompt = body.prompt ?? automation.prompt;
        if (body.cronExpression) {
          automation.cronExpression = body.cronExpression;
          automation.schedulerExpression = body.cronExpression.split(/\s+/).filter(Boolean).length === 5
            ? `0 ${body.cronExpression}`
            : body.cronExpression;
        }
        automation.updatedAt = now;
        return fulfillJson(route, successEnvelope(detail(automation)));
      }
    }

    const statusMatch = captured.pathname.match(/^\/api\/v1\/automations\/([^/]+)\/status$/);
    if (statusMatch) {
      const automationCode = decodeURIComponent(statusMatch[1]);
      const automation = automations.find((item) => item.automationCode === automationCode);
      if (!automation) {
        return fulfillJson(route, { code: "AUTOMATION_NOT_FOUND", message: "automation was not found", data: {} }, 404);
      }
      const body = captured.body as { status?: AutomationStatus };
      automation.status = body.status === "Active" ? "Active" : "Paused";
      automation.updatedAt = now;
      return fulfillJson(route, successEnvelope(detail(automation)));
    }

    const archiveMatch = captured.pathname.match(/^\/api\/v1\/automations\/([^/]+)\/archive$/);
    if (archiveMatch) {
      const automationCode = decodeURIComponent(archiveMatch[1]);
      const automation = automations.find((item) => item.automationCode === automationCode);
      if (!automation) {
        return fulfillJson(route, { code: "AUTOMATION_NOT_FOUND", message: "automation was not found", data: {} }, 404);
      }
      automation.previousStatus = automation.status;
      automation.status = "Archived";
      automation.archivedAt = now;
      automation.updatedAt = now;
      return fulfillJson(route, successEnvelope({
        archivedAt: automation.archivedAt,
        automationCode: automation.automationCode,
        previousStatus: automation.previousStatus,
        status: automation.status,
      }));
    }

    const restoreMatch = captured.pathname.match(/^\/api\/v1\/automations\/([^/]+)\/restore$/);
    if (restoreMatch) {
      const automationCode = decodeURIComponent(restoreMatch[1]);
      const automation = automations.find((item) => item.automationCode === automationCode);
      if (!automation) {
        return fulfillJson(route, { code: "AUTOMATION_NOT_FOUND", message: "automation was not found", data: {} }, 404);
      }
      automation.status = automation.previousStatus ?? "Paused";
      automation.previousStatus = null;
      automation.archivedAt = null;
      automation.updatedAt = now;
      return fulfillJson(route, successEnvelope({
        archivedAt: automation.archivedAt,
        automationCode: automation.automationCode,
        previousStatus: automation.previousStatus,
        status: automation.status,
      }));
    }

    const executeMatch = captured.pathname.match(/^\/api\/v1\/automations\/([^/]+)\/execute$/);
    if (executeMatch) {
      const automationCode = decodeURIComponent(executeMatch[1]);
      const automation = automations.find((item) => item.automationCode === automationCode);
      if (!automation) {
        return fulfillJson(route, { code: "AUTOMATION_NOT_FOUND", message: "automation was not found", data: {} }, 404);
      }
      const executionCode = `AEX-PW-000${executions.length + 1}`;
      const execution: AutomationExecution = {
        automationCode,
        createdAt: now,
        executionCode,
        finishedAt: now,
        scheduledAt: null,
        startedAt: now,
        status: "Success",
        summary: "Prompt dispatch recorded for the automation.",
        triggerSource: "Manual",
        workspaceCode: automation.workspaceCode,
      };
      executions.unshift(execution);
      automation.lastExecutionCode = execution.executionCode;
      automation.lastExecutionStatus = execution.status;
      automation.lastExecutionSummary = execution.summary;
      automation.lastStartedAt = execution.startedAt;
      automation.lastFinishedAt = execution.finishedAt;
      automation.updatedAt = now;
      return fulfillJson(route, successEnvelope(detail(automation)));
    }

    const executionsMatch = captured.pathname.match(/^\/api\/v1\/automations\/([^/]+)\/executions$/);
    if (executionsMatch) {
      const automationCode = decodeURIComponent(executionsMatch[1]);
      const records = executions.filter((record) => record.automationCode === automationCode);
      return fulfillJson(route, pageEnvelope(records));
    }

    return fulfillJson(route, { code: "NOT_IMPLEMENTED", message: `Unhandled ${captured.method} ${captured.pathname}`, data: {} }, 501);
  });
}

async function reservePort() {
  return await new Promise<number>((resolvePort, reject) => {
    const server = createServer();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      if (!address || typeof address === "string") {
        server.close();
        reject(new Error("Could not determine reserved port"));
        return;
      }
      const port = address.port;
      server.close((error) => {
        if (error) reject(error);
        else resolvePort(port);
      });
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
    env: {
      ...process.env,
      CI: "1",
      VITE_TRON_ADMIN_DATA_SOURCE: "api",
      VITE_TRON_AUTOMATION_DATA_SOURCE: "api",
      VITE_TRON_DATA_SOURCE: "api",
    },
    stdio: "pipe",
  });
  frontendProcess.stdout.on("data", (chunk) => {
    frontendLogs = `${frontendLogs}${chunk.toString()}`.slice(-5000);
  });
  frontendProcess.stderr.on("data", (chunk) => {
    frontendLogs = `${frontendLogs}${chunk.toString()}`.slice(-5000);
  });
  await waitForFrontend(`${frontendBaseUrl}/automations`);
});

test.afterAll(async () => {
  if (!frontendProcess || frontendProcess.killed) return;
  frontendProcess.kill("SIGTERM");
  await new Promise((resolveProcess) => frontendProcess?.once("exit", resolveProcess));
});

test("automation management supports workspace-scoped list create detail execute lifecycle and keeps import sync actions removed", async ({ page }) => {
  const requests: CapturedRequest[] = [];
  const automations: Automation[] = [
    createAutomationFixture({
      automationCode: "AUT-PW-0001",
      description: "Seeded automation for search coverage.",
      id: "734600000000000001",
      lastExecutionCode: "AEX-PW-0001",
      lastExecutionStatus: "Success",
      lastExecutionSummary: "Prompt dispatch recorded for the automation.",
      lastFinishedAt: now,
      lastStartedAt: now,
      name: "Daily workspace release digest",
      status: "Active",
      workspaceCode: "TRON_CORE",
    }),
    createAutomationFixture({
      automationCode: "AUT-PW-ARCHIVED",
      archivedAt: now,
      description: "Archived automation fixture.",
      id: "734600000000000002",
      name: "Archived automation fixture",
      previousStatus: "Paused",
      status: "Archived",
      workspaceCode: "TRON_CORE",
    }),
  ];
  const executions: AutomationExecution[] = [{
    automationCode: "AUT-PW-0001",
    createdAt: now,
    executionCode: "AEX-PW-0001",
    finishedAt: now,
    scheduledAt: now,
    startedAt: now,
    status: "Success",
    summary: "Prompt dispatch recorded for the automation.",
    triggerSource: "Scheduler",
    workspaceCode: "TRON_CORE",
  }];
  await routeApi(page, requests, automations, executions);

  await page.goto(`${frontendBaseUrl}/automations`);
  await expect(page.getByRole("heading", { name: "Automations Management" })).toBeVisible();
  await expect(page.getByRole("button", { name: /^Import$/ })).toHaveCount(0);
  await expect(page.getByRole("button", { name: /^Sync$/ })).toHaveCount(0);
  await expect(page.getByRole("button", { name: /^Refresh$/ })).toBeVisible();
  await expect(page.getByRole("button", { name: /^New Automation$/ })).toBeVisible();
  await expect(page.getByLabel("Automation status filter")).toHaveValue("");
  await expect.poll(() => requests.some((request) => request.pathname === "/api/v1/automations" && request.searchParams.workspaceCode === "TRON_CORE")).toBe(true);
  await page.getByLabel("Search automations").fill("digest");
  await expect.poll(() => requests.some((request) => request.pathname === "/api/v1/automations" && request.searchParams.search === "digest")).toBe(true);
  await expect(page.getByText("Daily workspace release digest")).toBeVisible();

  await page.getByRole("button", { name: /^New Automation$/ }).click();
  await expect(page.getByRole("heading", { name: "Create Automation" })).toBeVisible();
  await expect(page.getByLabel("Workspace code")).toHaveValue("TRON_CORE");
  await expect(page.getByRole("combobox", { name: "Workspace" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: /^Import$/ })).toHaveCount(0);
  await expect(page.getByRole("button", { name: /^Sync$/ })).toHaveCount(0);
  await page.getByLabel("Automation name").fill("Playwright automation");
  await page.getByLabel("Automation description").fill("Created through the automation smoke.");
  await page.getByLabel("Cron expression").fill("*/15 * * * *");
  await page.getByRole("button", { name: "Fullscreen" }).click();
  const promptDialog = page.getByRole("dialog", { name: "Prompt fullscreen editor" });
  await expect(promptDialog).toBeVisible();
  await promptDialog.getByLabel("Automation prompt fullscreen").fill("## Automation Smoke\n- Create detail lifecycle proof.\n- Use workspace scoped prompt.");
  await promptDialog.getByRole("button", { name: "Close prompt fullscreen" }).click();
  await expect(promptDialog).toHaveCount(0);
  await page.getByRole("button", { name: /^Create$/ }).click();
  await expect.poll(() => requests.some((request) => request.method === "POST" && request.pathname === "/api/v1/automations" && (request.body as { workspaceCode?: string; prompt?: string } | null)?.workspaceCode === "TRON_CORE" && ((request.body as { prompt?: string } | null)?.prompt?.includes("Automation Smoke") ?? false))).toBe(true);

  await expect(page).toHaveURL(/\/automations\/detail\?automationCode=AUT-PW-0003/);
  await expect(page.getByRole("heading", { name: "Playwright automation" })).toBeVisible();
  await expect(page.getByLabel("Scheduler expression")).toBeVisible();
  await expect(page.getByRole("button", { name: "Run Now" })).toBeVisible();
  await expect(page.getByLabel("Scheduler expression")).toHaveValue("0 */15 * * * *");
  await expect(page.getByLabel("Automation status")).toHaveValue("Paused");
  await expect(page.getByLabel("Automation prompt preview").getByText("Automation Smoke", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Edit" }).click();
  await page.getByLabel("Automation name").fill("Playwright automation updated");
  await page.getByLabel("Cron expression").fill("0 18 * * *");
  await page.getByRole("button", { name: "Fullscreen" }).click();
  const editPromptDialog = page.getByRole("dialog", { name: "Prompt fullscreen editor" });
  await expect(editPromptDialog).toBeVisible();
  await editPromptDialog.getByLabel("Automation prompt fullscreen").fill("## Automation Smoke Updated\n- Detail save path.\n- Scheduler expression refresh.");
  await editPromptDialog.getByRole("button", { name: "Close prompt fullscreen" }).click();
  await page.getByRole("button", { name: /^Save$/ }).click();
  await expect(toast(page, /Automation AUT-PW-0003 saved/)).toBeVisible();
  await expect.poll(() => requests.some((request) => request.method === "PATCH" && request.pathname === "/api/v1/automations/AUT-PW-0003" && (request.body as { cronExpression?: string; name?: string } | null)?.cronExpression === "0 18 * * *" && (request.body as { name?: string } | null)?.name === "Playwright automation updated")).toBe(true);
  await expect(page.getByLabel("Scheduler expression")).toHaveValue("0 0 18 * * *");

  await page.getByRole("button", { name: "Run Now" }).click();
  await expect(toast(page, /Manual execution recorded for AUT-PW-0003/)).toBeVisible();
  await expect.poll(() => requests.some((request) => request.method === "POST" && request.pathname === "/api/v1/automations/AUT-PW-0003/execute")).toBe(true);
  await expect(page.getByText("Latest Success", { exact: true })).toBeVisible();
  await expect(page.getByRole("cell", { name: "Manual" })).toBeVisible();

  await page.getByLabel("Automation status").selectOption("Active");
  await expect(toast(page, /Automation AUT-PW-0003 activated/)).toBeVisible();
  await expect.poll(() => requests.some((request) => request.method === "PATCH" && request.pathname === "/api/v1/automations/AUT-PW-0003/status" && (request.body as { status?: AutomationStatus } | null)?.status === "Active")).toBe(true);

  await page.getByRole("button", { name: "Archive" }).click();
  await expect(toast(page, /Automation AUT-PW-0003 archived/)).toBeVisible();
  await expect(page.getByRole("button", { name: "Restore" })).toBeVisible();
  await expect.poll(() => requests.some((request) => request.method === "POST" && request.pathname === "/api/v1/automations/AUT-PW-0003/archive")).toBe(true);

  await page.getByRole("button", { name: "Restore" }).click();
  await expect(toast(page, /Automation AUT-PW-0003 restored to Active/)).toBeVisible();
  await expect.poll(() => requests.some((request) => request.method === "POST" && request.pathname === "/api/v1/automations/AUT-PW-0003/restore")).toBe(true);
});
