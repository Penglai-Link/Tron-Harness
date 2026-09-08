import {
  expect,
  test,
  type Page,
  type Route,
} from "@playwright/test";
import {
  spawnManagedProcess,
  stopManagedProcess,
  type ManagedProcess,
} from "./support/managed-process";
import { createServer } from "node:net";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { getWorkspaceSwitcherMock } from "../src/shared/constants/workspaceSpaces";

const thisFilePath = fileURLToPath(import.meta.url);
const frontendRoot = resolve(thisFilePath, "..", "..");

let frontendProcess: ManagedProcess | undefined;
let frontendLogs = "";
let frontendBaseUrl = "";

const authUser = {
  authMethod: "internal",
  authenticated: true,
  provider: null,
  user: {
    displayName: "Auth Operator",
    email: "auth.operator@tron.ai",
    roleCodes: ["ADMIN", "DEVELOPER"],
    spaceCodes: ["PLATFORM", "TRON_CORE"],
    status: "Active",
    userCode: "USR-AUTH-0001",
    username: "auth.operator",
  },
};

function delay(ms: number) {
  return new Promise((resolveDelay) => {
    setTimeout(resolveDelay, ms);
  });
}

function appendFrontendLog(chunk: Buffer) {
  frontendLogs = `${frontendLogs}${chunk.toString()}`.slice(-4000);
}

async function reservePort() {
  return new Promise<number>((resolvePort, rejectPort) => {
    const server = createServer();

    server.on("error", rejectPort);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      if (!address || typeof address === "string") {
        server.close();
        rejectPort(new Error("Failed to reserve a frontend test port."));
        return;
      }

      const { port } = address;
      server.close(() => resolvePort(port));
    });
  });
}

async function waitForFrontend(url: string) {
  const deadline = Date.now() + 30_000;
  let lastError: unknown;

  while (Date.now() < deadline) {
    if (frontendProcess?.exitCode !== null) {
      throw new Error(`Frontend dev server exited early.\n${frontendLogs}`);
    }

    try {
      const response = await fetch(url);
      if (response.ok) return;
    } catch (error) {
      lastError = error;
    }

    await delay(250);
  }

  throw new Error(`Frontend dev server did not become ready: ${String(lastError)}\n${frontendLogs}`);
}

function successEnvelope(data: unknown) {
  return {
    code: "SUCCESS",
    message: "ok",
    data,
  };
}

function unauthorizedEnvelope(code = "AUTH_UNAUTHORIZED", message = "authentication required") {
  return {
    code,
    message,
    data: {},
  };
}

async function fulfillJson(route: Route, data: unknown, status = 200) {
  await route.fulfill({
    status,
    contentType: "application/json",
    body: JSON.stringify(data),
  });
}

async function routeAuthApi(page: Page) {
  let currentSession: typeof authUser | null = null;
  let hasRefreshSession = false;
  let currentAccessToken = "";
  let nextAccessTokenId = 1;

  function issueGrant(session: typeof authUser) {
    currentSession = session;
    hasRefreshSession = true;
    currentAccessToken = `test-access-token-${nextAccessTokenId++}`;

    return successEnvelope({
      accessToken: currentAccessToken,
      tokenType: "Bearer",
      expiresIn: 900,
      session,
    });
  }

  await page.route("**/api/v1/auth/providers", async (route) => {
    await fulfillJson(route, successEnvelope({
      providers: [
        { code: "internal", name: "Internal Account", type: "internal" },
      ],
    }));
  });

  await page.route("**/api/v1/auth/session", async (route) => {
    if (
      !currentSession
      || route.request().headers().authorization !== `Bearer ${currentAccessToken}`
    ) {
      await fulfillJson(route, unauthorizedEnvelope(), 401);
      return;
    }

    await fulfillJson(route, successEnvelope(currentSession));
  });

  await page.route("**/api/v1/auth/login", async (route) => {
    const body = route.request().postDataJSON() as { login?: string; password?: string };
    if (body.login === "auth.operator" && body.password === "TronAuthPass123!") {
      await fulfillJson(route, issueGrant(authUser));
      return;
    }

    await fulfillJson(route, unauthorizedEnvelope("AUTH_INVALID_CREDENTIALS", "invalid credentials"), 401);
  });

  await page.route("**/api/v1/auth/refresh", async (route) => {
    if (!currentSession || !hasRefreshSession) {
      await fulfillJson(route, unauthorizedEnvelope("AUTH_REFRESH_INVALID", "invalid refresh session"), 401);
      return;
    }

    await fulfillJson(route, issueGrant(currentSession));
  });

  await page.route("**/api/v1/auth/logout", async (route) => {
    currentAccessToken = "";
    currentSession = null;
    hasRefreshSession = false;
    await fulfillJson(route, successEnvelope({ loggedOut: true }));
  });
}

function escapedPathPattern(path: string) {
  return new RegExp(`${path.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}$`);
}

async function signIn(page: Page, targetPath: string, expectedPath = targetPath) {
  const expectedUrlPattern = escapedPathPattern(expectedPath);
  const loginUrlPattern = new RegExp(`/login\\?redirect=${encodeURIComponent(targetPath).replace(/\//g, "%2F")}$`);

  await page.goto(`${frontendBaseUrl}${targetPath}`);

  if (!expectedUrlPattern.test(page.url()) && !loginUrlPattern.test(page.url())) {
    await page.waitForURL((url) => expectedUrlPattern.test(url.href) || loginUrlPattern.test(url.href));
  }

  if (expectedUrlPattern.test(page.url())) {
    return;
  }

  await expect(page).toHaveURL(loginUrlPattern);

  await page.getByLabel(/operator id|username|email/i).fill("auth.operator");
  await page.getByLabel(/passkey|password/i).fill("TronAuthPass123!");
  await page.getByRole("button", { name: /sign in/i }).click();
  await expect(page).toHaveURL(expectedUrlPattern);
}

async function seedWorkspaceSelection(page: Page) {
  await page.addInitScript((snapshot) => {
    window.localStorage.setItem("tron-app-shell", JSON.stringify({
      state: {
        openSections: { Core: true },
        selectedPrimarySpaceId: "ENTERPRISE",
        selectedSecondarySpaceId: "TRON_CORE",
        rightSidebarCollapsed: false,
        sidebarCollapsed: false,
        workspaceSwitcherSnapshot: snapshot,
      },
      version: 4,
    }));
  }, getWorkspaceSwitcherMock());
}

function rightSidebar(page: Page) {
  return page.getByTestId("tron-right-sidebar");
}

function rightSidebarOpenControl(page: Page) {
  return page.getByRole("button", { name: "Open right sidebar" });
}

function rightSidebarCloseControl(page: Page) {
  return page.getByRole("button", { name: "Close right sidebar" });
}

async function expectRightSidebarClosed(page: Page) {
  await expect(rightSidebar(page)).toBeHidden();
  await expect(rightSidebarOpenControl(page)).toBeVisible();
}

async function openRightSidebar(page: Page) {
  await expect.poll(async () => (
    await rightSidebarOpenControl(page).isVisible().catch(() => false)
    || await rightSidebarCloseControl(page).isVisible().catch(() => false)
  )).toBe(true);
  if (await rightSidebarOpenControl(page).isVisible().catch(() => false)) {
    await rightSidebarOpenControl(page).click();
  }
  await expect(rightSidebarCloseControl(page)).toBeVisible();
  await expect(rightSidebar(page)).toBeVisible();
}

async function closeRightSidebar(page: Page) {
  await expect(rightSidebar(page)).toBeVisible();
  await expect(rightSidebarCloseControl(page)).toBeVisible();
  await rightSidebarCloseControl(page).click();
  await expectRightSidebarClosed(page);
}

async function selectTaskSessions(page: Page) {
  await page.getByRole("tab", { name: "Task sessions" }).click();
  await expect(page.getByRole("button", { name: "Auth latency rollback" })).toBeVisible();
}

async function expectNoRightSidebarControls(page: Page) {
  await expect(rightSidebar(page)).toHaveCount(0);
  await expect(rightSidebarOpenControl(page)).toHaveCount(0);
  await expect(rightSidebarCloseControl(page)).toHaveCount(0);
  await expect(page.getByTestId("tron-mobile-right-sidebar-toggle")).toHaveCount(0);
}

test.beforeAll(async () => {
  const port = await reservePort();
  frontendBaseUrl = `http://127.0.0.1:${port}`;

  frontendProcess = spawnManagedProcess(
    "npm",
    ["run", "dev", "--", "--host", "127.0.0.1", "--port", String(port), "--strictPort"],
    {
      cwd: frontendRoot,
      env: {
        ...process.env,
        BROWSER: "none",
        VITE_TRON_AUTH_DATA_SOURCE: "mock",
        VITE_TRON_API_BASE_URL: "",
        VITE_TRON_DATA_SOURCE: "mock",
        VITE_TRON_ADMIN_DATA_SOURCE: "mock",
        VITE_TRON_AGENT_DATA_SOURCE: "mock",
        VITE_TRON_APP_DATA_SOURCE: "mock",
        VITE_TRON_AUTOMATION_DATA_SOURCE: "mock",
        VITE_TRON_CHAT_DATA_SOURCE: "mock",
        VITE_TRON_KNOWLEDGE_GRAPH_DATA_SOURCE: "mock",
        VITE_TRON_MCP_DATA_SOURCE: "mock",
        VITE_TRON_MEMORY_DATA_SOURCE: "mock",
        VITE_TRON_MODEL_DATA_SOURCE: "mock",
        VITE_TRON_OBSERVABILITY_DATA_SOURCE: "mock",
        VITE_TRON_PROMPT_DATA_SOURCE: "mock",
        VITE_TRON_RAG_DATA_SOURCE: "mock",
        VITE_TRON_RESOURCE_DATA_SOURCE: "mock",
        VITE_TRON_SKILL_DATA_SOURCE: "mock",
        VITE_TRON_TASK_DATA_SOURCE: "mock",
        VITE_TRON_VARIABLE_DATA_SOURCE: "mock",
      },
      stdio: ["ignore", "pipe", "pipe"],
    },
  );
  frontendProcess.stdout.on("data", appendFrontendLog);
  frontendProcess.stderr.on("data", appendFrontendLog);

  await waitForFrontend(frontendBaseUrl);
});

test.afterAll(async () => {
  try {
    await stopManagedProcess(frontendProcess);
  } finally {
    frontendProcess = undefined;
  }
});

test("AC-RCS-01: desktop right sidebar collapses out of layout, restores width, and reopens", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await routeAuthApi(page);
  await signIn(page, "/shared-shell-preview");

  await expect(page.getByRole("heading", { name: "Workspace console foundation" })).toBeVisible();
  await expectRightSidebarClosed(page);
  const mainBefore = await page.locator("main").first().boundingBox();
  expect(mainBefore, "The shell main region should be rendered before opening the default-collapsed sidebar.").not.toBeNull();

  await rightSidebarOpenControl(page).click();

  await expect(rightSidebar(page)).toBeVisible();
  await expect(rightSidebar(page).getByText("Resource Pattern")).toBeVisible();
  await expect(rightSidebarOpenControl(page)).toHaveCount(0);

  const mainAfterOpen = await page.locator("main").first().boundingBox();
  expect(mainAfterOpen, "The shell main region should remain rendered after opening the sidebar.").not.toBeNull();
  expect(mainBefore!.width).toBeGreaterThan(mainAfterOpen!.width + 250);

  await closeRightSidebar(page);
  await expect(rightSidebar(page).getByText("Resource Pattern")).toBeHidden();

  const mainAfterCollapse = await page.locator("main").first().boundingBox();
  expect(mainAfterCollapse, "The shell main region should remain rendered after collapse.").not.toBeNull();
  expect(mainAfterCollapse!.width).toBeGreaterThan(mainAfterOpen!.width + 250);

  await rightSidebarOpenControl(page).click();

  await expect(rightSidebar(page)).toBeVisible();
  await expect(rightSidebar(page).getByText("Resource Pattern")).toBeVisible();
  await expect(rightSidebarOpenControl(page)).toHaveCount(0);
});

test("AC-RCS-02: mobile right sidebar opens as a drawer and closes by all required paths", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await routeAuthApi(page);
  await signIn(page, "/shared-shell-preview");

  const drawerToggle = page.getByRole("button", { name: "Open right sidebar" });
  await expect(drawerToggle).toBeVisible();
  await expect(page.getByTestId("tron-mobile-right-sidebar")).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "Resource Pattern" })).not.toBeVisible();

  await drawerToggle.click();
  await expect(page.getByTestId("tron-mobile-right-sidebar")).toBeVisible();
  await expect(page.getByTestId("tron-mobile-right-sidebar").getByText("Resource Pattern")).toBeVisible();
  await expect(page.getByTestId("tron-mobile-right-sidebar-backdrop")).toBeVisible();

  await page.getByTestId("tron-mobile-right-sidebar").getByRole("button", { name: "Close right sidebar" }).click();
  await expect(page.getByTestId("tron-mobile-right-sidebar")).toHaveCount(0);

  await drawerToggle.click();
  await expect(page.getByTestId("tron-mobile-right-sidebar")).toBeVisible();
  await page.mouse.click(12, 100);
  await expect(page.getByTestId("tron-mobile-right-sidebar")).toHaveCount(0);

  await drawerToggle.click();
  await expect(page.getByTestId("tron-mobile-right-sidebar")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("tron-mobile-right-sidebar")).toHaveCount(0);

  await drawerToggle.click();
  await expect(page.getByTestId("tron-mobile-right-sidebar")).toBeVisible();
  await page.goto(`${frontendBaseUrl}/apps`);
  await expect(page.getByRole("heading", { name: "App Management" })).toBeVisible();
  await expect(page.getByTestId("tron-mobile-right-sidebar")).toHaveCount(0);
});

test("AC-RCS-03: shell right-panel pages retain content and no-panel pages show no stale controls", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await seedWorkspaceSelection(page);
  await routeAuthApi(page);
  await signIn(page, "/chats/8842-alpha");
  await expect(page).toHaveURL(/\/chats\/8842-alpha$/);
  await selectTaskSessions(page);
  await page.getByText("Refactor discussion", { exact: true }).first().click();
  await openRightSidebar(page);
  await rightSidebar(page).getByRole("tab", { name: "Context" }).click();
  await expect(rightSidebar(page).getByText("Short-Term Memory Overview")).toBeVisible();

  await page.goto(`${frontendBaseUrl}/observability`);
  await expect(page.getByRole("heading", { name: "Observability Dashboard" })).toBeVisible();
  await openRightSidebar(page);
  await expect(rightSidebar(page).getByRole("heading", { name: "Inspector" })).toBeVisible();
  await rightSidebar(page).getByRole("button", { name: "Close inspector" }).click();
  await expectNoRightSidebarControls(page);

  await page.goto(`${frontendBaseUrl}/shared-shell-preview`);
  await openRightSidebar(page);
  await expect(rightSidebar(page).getByText("Resource Pattern")).toBeVisible();

  await page.goto(`${frontendBaseUrl}/apps`);
  await expect(page.getByRole("heading", { name: "App Management" })).toBeVisible();
  await expectNoRightSidebarControls(page);

  await page.goto(`${frontendBaseUrl}/settings/general`);
  await expect(page.getByRole("heading", { name: "General" })).toBeVisible();
  await expect(page.locator("#tron-workspace-trigger")).toHaveCount(0);
  await expectNoRightSidebarControls(page);
});

test("AC-STM-08: chat right sidebar renders short-term memory state and compaction action", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await seedWorkspaceSelection(page);
  await routeAuthApi(page);
  await signIn(page, "/chats/8842-alpha");
  await expect(page).toHaveURL(/\/chats\/8842-alpha$/);
  await selectTaskSessions(page);
  await page.getByText("Primary investigation", { exact: true }).first().click();
  await openRightSidebar(page);

  const sidebar = rightSidebar(page);
  await expect(sidebar.getByRole("heading", { name: "Goal" })).toBeVisible();
  await expect(sidebar.getByRole("heading", { name: "Todo" })).toBeVisible();
  await expect(sidebar.getByText("Validate transcript storage after the next chat turn")).toBeVisible();
  await expect(sidebar.getByText("Review compaction checkpoint output")).toBeVisible();
  await expect(sidebar.getByText("In progress")).toBeVisible();
  await expect(sidebar.getByText("Pending", { exact: true })).toBeVisible();

  await sidebar.getByRole("tab", { name: "Context" }).click();
  await expect(sidebar.getByText("Context Structure")).toBeVisible();
  await expect(sidebar.getByTestId("context-runtime-notice")).toContainText("Context compacted");
  await expect(sidebar.getByText("Tool Schema", { exact: true }).locator("..")).toContainText("1.8k / 6k");
  await expect(sidebar.getByText("Current Session Checkpoint")).toBeVisible();
  await expect(sidebar.getByText("seq 1-8", { exact: true })).toBeVisible();

  await sidebar.getByRole("button", { name: "Create checkpoint" }).click();

  await expect(sidebar.getByText("seq 1-2", { exact: true })).toBeVisible();
  await expect(sidebar.getByText("seq 1-8", { exact: true })).toHaveCount(0);

  await page.getByRole("button", { name: "Open user menu" }).click();
  await page.locator("#tron-locale-select").click();
  await page.getByRole("option", { name: "Simplified Chinese" }).click();
  await page.keyboard.press("Escape");
  await expect(sidebar.getByText("上下文结构")).toBeVisible();
  await expect(sidebar.getByTestId("context-runtime-notice")).toContainText("上下文已压缩");
  await expect(sidebar.getByText("Tool Schema", { exact: true }).locator("..")).toContainText("1.8k / 6k");
});

test("AC-CHAT-SESSION-SIDEBAR-01: manual history collapse survives opening the right sidebar", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await seedWorkspaceSelection(page);
  await routeAuthApi(page);
  await signIn(page, "/chats/8842-alpha");
  await expect(page).toHaveURL(/\/chats\/8842-alpha$/);
  await selectTaskSessions(page);
  await page.getByText("Primary investigation", { exact: true }).first().click();

  const sessionSidebar = page.getByTestId("chat-session-sidebar");
  await expect(sessionSidebar.getByRole("heading", { name: "Sessions" })).toBeVisible();
  await sessionSidebar.getByRole("button", { name: "Collapse session history" }).click();
  await expect(sessionSidebar.getByRole("heading", { name: "Sessions" })).toHaveCount(0);

  await openRightSidebar(page);

  await expect(sessionSidebar.getByRole("heading", { name: "Sessions" })).toHaveCount(0);
  await expect(sessionSidebar.getByRole("button", { name: "Expand session history" })).toBeVisible();
});

test("AC-RCS-04: graph inspector stays inline and memory inspectors preserve selected context", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await seedWorkspaceSelection(page);
  await routeAuthApi(page);
  await signIn(page, "/knowledge-graph");

  await expect(page.getByRole("heading", { name: "Knowledge Graph" })).toBeVisible();
  await expectNoRightSidebarControls(page);
  await page.getByRole("searchbox", { name: "Locate graph node" }).fill("RAG_REVIEWER");
  await page.getByRole("option", { name: /RAG_REVIEWER/ }).click();
  await expect(page.getByText("Node Inspector")).toBeVisible();
  await expect(page.getByRole("heading", { name: "RAG_REVIEWER" })).toBeVisible();
  await expectNoRightSidebarControls(page);
  await page.getByRole("button", { name: "Close node inspector" }).click();
  await expect(page.getByText("Node Inspector")).toHaveCount(0);

  await page.goto(`${frontendBaseUrl}/knowledge-graph/domain-filtering`);
  await expect(page.getByRole("heading", { name: "Knowledge Graph" })).toBeVisible();
  await expectNoRightSidebarControls(page);
  await page.getByRole("searchbox", { name: "Locate graph node" }).fill("SOURCE_CONTROL");
  await page.getByRole("option", { name: /SOURCE_CONTROL/ }).click();
  await expect(page.getByText("Node Inspector")).toBeVisible();
  await expect(page.getByRole("heading", { name: "SOURCE_CONTROL" })).toBeVisible();

  await page.goto(`${frontendBaseUrl}/memory`);
  await expect(page.getByRole("heading", { name: "Memory" }).first()).toBeVisible();
  await expectNoRightSidebarControls(page);
  await expect(page.getByText("manual lookup")).toBeVisible();

  await page.goto(`${frontendBaseUrl}/memory/details`);
  await expect(page.getByRole("heading", { name: "Memory" }).first()).toBeVisible();
  await expectNoRightSidebarControls(page);
});

test("memory empty state uses neutral copy", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await seedWorkspaceSelection(page);
  await routeAuthApi(page);
  await signIn(page, "/memory");

  await page.getByRole("searchbox", { name: "Search" }).fill("zz-empty-memory-copy");

  await expect(page.getByText("No memories found")).toBeVisible();
  await expect(page.getByText('No memories in Tron Core match "zz-empty-memory-copy".', { exact: true })).toBeVisible();
  await expect(page.getByText(/mock memories/i)).toHaveCount(0);
});

test("memory domain preview omits instructional helper copy", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await seedWorkspaceSelection(page);
  await routeAuthApi(page);
  await signIn(page, "/settings/memory-domains");

  await page.getByText("Backend Engineering", { exact: true }).click();

  const dialog = page.getByRole("dialog", { name: "Memory Domain" });
  await expect(dialog).toBeVisible();
  await expect(dialog.getByText("Review the selected domain before switching to edit mode.")).toHaveCount(0);
  await expect(dialog.getByText("Domain-scoped memory targets use this catalog.")).toHaveCount(0);
  await expect(dialog.getByText("Backend Engineering", { exact: true })).toBeVisible();
});

test("AC-RCS-05: management inspectors render in shell right sidebar and preserve page actions", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await seedWorkspaceSelection(page);
  await routeAuthApi(page);

  await signIn(page, "/tasks/detail?taskCode=TSK-SEED-FEATURE");
  await openRightSidebar(page);
  await expect(rightSidebar(page).getByRole("heading", { name: "Task Metadata" }).first()).toBeVisible();
  await expect(page.locator("p.font-mono", { hasText: "TSK-SEED-FEATURE" }).first()).toBeVisible();
  await page.goto(`${frontendBaseUrl}/tasks/detail?taskCode=TSK-SEED-BUG`);
  await expect(page).toHaveURL(/taskCode=TSK-SEED-BUG$/);
  await openRightSidebar(page);
  await expect(rightSidebar(page).getByRole("heading", { name: "Task Metadata" }).first()).toBeVisible();
  await expect(page.locator("p.font-mono", { hasText: "TSK-SEED-BUG" }).first()).toBeVisible();
  await page.getByRole("button", { name: "Edit" }).click();
  await expect(page.locator('input[aria-label="Task detail title"]')).toHaveValue("Fix task board workflow regression");
  await page.getByRole("button", { name: "Cancel" }).click();
  await expect(rightSidebar(page).getByRole("heading", { name: "Task Metadata" }).first()).toBeVisible();
  await closeRightSidebar(page);
  await rightSidebarOpenControl(page).click();
  await expect(rightSidebar(page).getByRole("heading", { name: "Task Metadata" }).first()).toBeVisible();
  await expect(page.locator("p.font-mono", { hasText: "TSK-SEED-BUG" }).first()).toBeVisible();

  await page.goto(`${frontendBaseUrl}/automations/executions?automationCode=AUT-734600000000000001`);
  await expect(page.getByRole("heading", { name: "Execution instances" })).toBeVisible();
  await page.getByRole("row", { name: /AEX-734600000000000001/ }).click();
  await openRightSidebar(page);
  await expect(rightSidebar(page).getByRole("heading", { name: "Session detail" })).toBeVisible();
  await expect(rightSidebar(page).getByText("AEX-734600000000000001", { exact: true })).toBeVisible();
  await page.getByRole("heading", { name: "Execution instances" }).scrollIntoViewIfNeeded();
  await expect(page.getByRole("columnheader", { name: "Execution" })).toBeVisible();

  await page.goto(`${frontendBaseUrl}/apps/detail?appCode=APP-TRON-CONSOLE`);
  await openRightSidebar(page);
  await expect(rightSidebar(page).getByText("App Metadata")).toBeVisible();
  await expect(page.getByRole("heading", { name: "Tron Console" })).toBeVisible();
  await closeRightSidebar(page);
  await rightSidebarOpenControl(page).click();
  await expect(rightSidebar(page).getByText("Identifier")).toBeVisible();

  await page.goto(`${frontendBaseUrl}/prompts`);
  await openRightSidebar(page);
  await expect(rightSidebar(page).getByText("Prompt Details")).toBeVisible();
  await expect(rightSidebar(page).getByRole("button", { name: "Run Test Preview" })).toBeVisible();
  await page.getByRole("row", { hasText: "Code Review Guidelines" })
    .getByRole("button", { name: "Code Review Guidelines", exact: true })
    .click();
  await expect(rightSidebar(page).getByText("Code Review Guidelines")).toBeVisible();
  await page.getByRole("button", { name: "Run Test Preview" }).click();
  await expect(page.getByText("Preview ran for Code Review Guidelines")).toBeVisible();
  await page.getByRole("button", { name: /Close prompt details/ }).click();
  await expect(rightSidebar(page).getByText("Prompt Details")).toHaveCount(0);
  await page.getByRole("row", { hasText: "LLM Router System" })
    .getByRole("button", { name: "LLM Router System", exact: true })
    .click();
  await expect(rightSidebar(page).getByText("LLM Router System")).toBeVisible();

  await closeRightSidebar(page);
  await rightSidebarOpenControl(page).click();
  await expect(rightSidebar(page).getByText("LLM Router System")).toBeVisible();

  await page.goto(`${frontendBaseUrl}/variables`);
  await expectNoRightSidebarControls(page);
  const firstVariableRow = page.locator("tbody tr").first();
  const variableKey = (await firstVariableRow.locator("td").first().locator("p").first().textContent())?.trim();
  await firstVariableRow.click();
  await expectNoRightSidebarControls(page);
  if (variableKey) {
    await expect(page.getByRole("heading", { name: variableKey })).toBeVisible();
  }

  await page.goto(`${frontendBaseUrl}/knowledge-base`);
  await expect(rightSidebarOpenControl(page)).toBeVisible();
  await page.locator("tbody tr").filter({ hasText: "Operations SOP Library" }).click();
  await expect(rightSidebar(page).getByText("Selected KnowledgeBase")).toBeVisible();
  await expect(rightSidebar(page).getByText("Operations SOP Library")).toBeVisible();

  await page.goto(`${frontendBaseUrl}/skills`);
  await expect(rightSidebarOpenControl(page)).toBeVisible();
  const skillRows = page.locator("tbody tr");
  const skillRowsCount = await skillRows.count();
  if (skillRowsCount > 0) {
    const firstCellText = (await skillRows.first().locator("td").first().textContent({ timeout: 1000 }).catch(() => ""))?.trim();
    const skillName = firstCellText?.split(/\s+/)[0];
    if (skillName && !firstCellText?.includes("No workspace skills")) {
      await skillRows.first().click();
      await expect(rightSidebar(page)).toBeVisible();
      await expect(rightSidebar(page).getByText(skillName)).toBeVisible();
    }
  }

  await page.goto(`${frontendBaseUrl}/settings/skills`);
  await expectNoRightSidebarControls(page);
});
