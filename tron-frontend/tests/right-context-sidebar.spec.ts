import { routeActiveProfileApi } from "./support/resource-ui-harness";
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

async function routeAuthApi(page: Page) { await routeActiveProfileApi(page); }
async function signIn(page: Page, targetPath: string) {
  await routeActiveProfileApi(page);
  await page.goto(`${frontendBaseUrl}${targetPath}`);
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
      version: 9,
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

async function expectNoStaleInspector(page: Page) {
  if (new URL(page.url()).pathname.startsWith("/settings/")) {
    await expect(rightSidebar(page)).toHaveCount(0);
    await expect(rightSidebarOpenControl(page)).toHaveCount(0);
    return;
  }
  await expect(rightSidebar(page).getByRole("tab", { name: "Short-term memory", exact: true })).toHaveCount(0);
  await expect(rightSidebar(page).getByText("Task Metadata", { exact: true })).toHaveCount(0);
  await expect.poll(async () => await rightSidebarOpenControl(page).isVisible() || await rightSidebarCloseControl(page).isVisible()).toBe(true);
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
  await signIn(page, "/apps");
  await expect(page.getByRole("tab", { name: "Apps", exact: true })).toBeVisible();
  await expectRightSidebarClosed(page);
  const mainBefore = await page.locator("main").first().boundingBox();
  expect(mainBefore, "The shell main region should be rendered before opening the default-collapsed sidebar.").not.toBeNull();

  await rightSidebarOpenControl(page).click();

  await expect(rightSidebar(page)).toBeVisible();
  await expect(rightSidebar(page).getByRole("button", { name: "New Tab", exact: true })).toBeVisible();
  await expect(rightSidebarOpenControl(page)).toHaveCount(0);

  const mainAfterOpen = await page.locator("main").first().boundingBox();
  expect(mainAfterOpen, "The shell main region should remain rendered after opening the sidebar.").not.toBeNull();
  expect(mainBefore!.width).toBeGreaterThan(mainAfterOpen!.width + 250);

  await closeRightSidebar(page);
  await expect(rightSidebar(page).getByRole("button", { name: "New Tab", exact: true })).toBeHidden();

  const mainAfterCollapse = await page.locator("main").first().boundingBox();
  expect(mainAfterCollapse, "The shell main region should remain rendered after collapse.").not.toBeNull();
  expect(mainAfterCollapse!.width).toBeGreaterThan(mainAfterOpen!.width + 250);

  await rightSidebarOpenControl(page).click();

  await expect(rightSidebar(page)).toBeVisible();
  await expect(rightSidebar(page).getByRole("button", { name: "New Tab", exact: true })).toBeVisible();
  await expect(rightSidebarOpenControl(page)).toHaveCount(0);
});

test("AC-RCS-02: mobile right sidebar opens as a drawer and closes by all required paths", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await routeAuthApi(page);
  await signIn(page, "/apps");

  const drawerToggle = page.getByRole("button", { name: "Open right sidebar" });
  await expect(drawerToggle).toBeVisible();
  await expect(page.getByTestId("tron-mobile-right-sidebar")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "New Tab", exact: true })).not.toBeVisible();

  await drawerToggle.click();
  await expect(page.getByTestId("tron-mobile-right-sidebar")).toBeVisible();
  await expect(page.getByTestId("tron-mobile-right-sidebar").getByRole("button", { name: "New Tab", exact: true })).toBeVisible();
  await expect(page.getByTestId("tron-mobile-right-sidebar-backdrop")).toBeVisible();

  await page.getByRole("banner").getByRole("button", { name: "Close right sidebar" }).click();
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
  await routeActiveProfileApi(page);
  await page.goto(`${frontendBaseUrl}/apps`);
  await expect(page.getByRole("tab", { name: "Apps", exact: true })).toBeVisible();
  await expect(page.getByTestId("tron-mobile-right-sidebar")).toHaveCount(0);
});

test("AC-RCS-03: shell right-panel pages retain content and no-panel pages show no stale controls", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await seedWorkspaceSelection(page);
  await routeAuthApi(page);
  await signIn(page, "/chats?sessionCode=SESSION-8842");
  await expect(page).toHaveURL(/sessionCode=SESSION-8842$/);
  await page.goto(`${frontendBaseUrl}/chats?sessionCode=SESSION-REFACTOR`);
  await openRightSidebar(page);
  await rightSidebar(page).getByRole("tab", { name: "Short-term memory" }).click();
  await expect(rightSidebar(page).getByRole("heading", { name: "Short-Term Memory", exact: true })).toBeVisible();

  await routeActiveProfileApi(page);
  await page.goto(`${frontendBaseUrl}`);
  await openRightSidebar(page);
  await expect(rightSidebar(page).getByRole("button", { name: "New Tab", exact: true })).toBeVisible();

  await routeActiveProfileApi(page);
  await page.goto(`${frontendBaseUrl}/apps`);
  await expect(page.getByRole("tab", { name: "Apps", exact: true })).toBeVisible();
  await expectNoStaleInspector(page);

  await routeActiveProfileApi(page);
  await page.goto(`${frontendBaseUrl}/settings/general`);
  await expect(page.getByRole("spinbutton", { name: "AG-UI recent work turn limit" })).toBeVisible();
  await expect(page.locator("#tron-workspace-trigger")).toHaveCount(0);
  await expectNoStaleInspector(page);
});

test("AC-STM-08: chat right sidebar renders short-term memory state and compaction action", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await seedWorkspaceSelection(page);
  await routeAuthApi(page);
  await signIn(page, "/chats?sessionCode=SESSION-8842");
  await expect(page).toHaveURL(/sessionCode=SESSION-8842$/);
  await openRightSidebar(page);

  const sidebar = rightSidebar(page);
  await expect(sidebar.getByRole("heading", { name: "Goal" })).toBeVisible();
  await expect(sidebar.getByRole("heading", { name: "Todo" })).toBeVisible();
  await expect(sidebar.getByText("Validate transcript storage after the next chat turn")).toBeVisible();
  await expect(sidebar.getByText("Review compaction checkpoint output")).toBeVisible();
  await expect(sidebar.getByText("In progress")).toBeVisible();
  await expect(sidebar.getByText("Pending", { exact: true })).toBeVisible();

  await sidebar.getByRole("tab", { name: "Short-term memory" }).click();
  await expect(sidebar.getByTestId("context-short-term-memory-card")).toBeVisible();
  await expect(sidebar.getByText("Short-Term Memory", { exact: true })).toBeVisible();
  await expect(sidebar.getByTestId("context-runtime-notice")).toContainText("Context compacted");
  await expect(sidebar.getByRole("button", { name: /^Tool Schema/ })).toContainText("1.8k / 6k");
  await expect(sidebar.getByText("Current Session Checkpoint")).toBeVisible();
  await expect(sidebar.getByText("· 1 checkpoint", { exact: true })).toBeVisible();

  await sidebar.getByRole("button", { name: "Create checkpoint" }).click();

  await expect(page.getByText("Session checkpoint created", { exact: true })).toBeVisible();
  await expect(sidebar.getByRole("button", { name: "Create checkpoint" })).toBeEnabled();

  await page.getByRole("button", { name: "Open user menu" }).click();
  await page.locator("#tron-locale-select").click();
  await page.getByRole("menuitemradio", { name: "Simplified Chinese" }).click();
  await page.keyboard.press("Escape");
  await expect(sidebar.getByRole("heading", { name: "短期记忆", exact: true })).toBeVisible();
  await expect(sidebar.getByTestId("context-runtime-notice")).toContainText("上下文已压缩");
  await expect(sidebar.getByRole("button", { name: /^工具定义/ })).toContainText("1.8k / 6k");
});

test("AC-CHAT-SESSION-SIDEBAR-01: manual history collapse survives opening the right sidebar", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await seedWorkspaceSelection(page);
  await routeAuthApi(page);
  await signIn(page, "/chats?sessionCode=SESSION-8842");
  await expect(page).toHaveURL(/sessionCode=SESSION-8842$/);

  await page.getByRole("button", { name: "Close navigation menu", exact: true }).click();
  await expect(page.getByRole("button", { name: "Open navigation menu", exact: true })).toBeVisible();
  await openRightSidebar(page);
  await expect(page.getByRole("button", { name: "Open navigation menu", exact: true })).toBeVisible();
  await expect(rightSidebar(page).getByRole("tab", { name: "Short-term memory", exact: true })).toBeVisible();

});

test("AC-RCS-04: graph inspector stays inline and memory inspectors preserve selected context", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await seedWorkspaceSelection(page);
  await routeAuthApi(page);
  await signIn(page, "/knowledge-graph");

  await expect(page.getByRole("tab", { name: "KnowledgeGraph", exact: true })).toBeVisible();
  await expectNoStaleInspector(page);
  await page.getByRole("searchbox", { name: "Locate graph node" }).fill("RAG_REVIEWER");
  await page.getByRole("option", { name: /RAG_REVIEWER/ }).click();
  await expect(page.getByText("Node Inspector")).toBeVisible();
  await expect(page.getByRole("heading", { name: "RAG_REVIEWER" })).toBeVisible();
  await expectNoStaleInspector(page);
  await page.getByRole("button", { name: "Close node inspector" }).click();
  await expect(page.getByText("Node Inspector")).toHaveCount(0);

  await routeActiveProfileApi(page);
  await page.goto(`${frontendBaseUrl}/knowledge-graph`);
  await expect(page.getByRole("tab", { name: "KnowledgeGraph", exact: true })).toBeVisible();
  await expectNoStaleInspector(page);
  await page.getByRole("searchbox", { name: "Locate graph node" }).fill("RAG_REVIEWER");
  await page.getByRole("option", { name: /RAG_REVIEWER/ }).click();
  await expect(page.getByText("Node Inspector")).toBeVisible();
  await expect(page.getByRole("heading", { name: "RAG_REVIEWER" })).toBeVisible();

  await routeActiveProfileApi(page);
  await page.goto(`${frontendBaseUrl}/memory`);
  await expect(page.getByRole("heading", { name: "Memory" }).first()).toBeVisible();
  await expectNoStaleInspector(page);
  await expect(page.getByText("manual lookup")).toBeVisible();

  await routeActiveProfileApi(page);
  await page.goto(`${frontendBaseUrl}/memory/details`);
  await expect(page.getByRole("heading", { name: "Memory" }).first()).toBeVisible();
  await expectNoStaleInspector(page);
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
  await expect(dialog.getByRole("textbox", { name: "Name", exact: true })).toHaveValue("Backend Engineering");
});

test("AC-RCS-05: management inspectors render in shell right sidebar and preserve page actions", async ({ page }) => {
  // This acceptance flow visits nine management pages and exercises their inspectors.
  test.setTimeout(60_000);
  await page.setViewportSize({ width: 1440, height: 900 });
  await seedWorkspaceSelection(page);
  await routeAuthApi(page);

  await signIn(page, "/tasks/detail?taskCode=TSK-SEED-FEATURE");
  await openRightSidebar(page);
  await expect(rightSidebar(page).getByRole("heading", { name: "Task Metadata" }).first()).toBeVisible();
  await expect(page.locator("p.font-mono", { hasText: "TSK-SEED-FEATURE" }).first()).toBeVisible();
  await routeActiveProfileApi(page);
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

  await routeActiveProfileApi(page);
  await page.goto(`${frontendBaseUrl}/automations/executions?automationCode=AUT-734600000000000001`);
  await expect(page.getByRole("columnheader", { name: "Execution", exact: true })).toBeVisible();
  await page.getByRole("row", { name: /AEX-734600000000000001/ }).click();
  await openRightSidebar(page);
  await expect(rightSidebar(page).getByRole("heading", { name: "Session detail" })).toBeVisible();
  await expect(rightSidebar(page).getByText("AEX-734600000000000001", { exact: true })).toBeVisible();
  await page.getByRole("columnheader", { name: "Execution", exact: true }).scrollIntoViewIfNeeded();
  await expect(page.getByRole("columnheader", { name: "Execution" })).toBeVisible();

  await routeActiveProfileApi(page);
  await page.goto(`${frontendBaseUrl}/apps/detail?appCode=APP-TRON-CONSOLE`);
  await openRightSidebar(page);
  await expect(rightSidebar(page).getByText("App Metadata").first()).toBeVisible();
  await expect(page.getByRole("navigation", { name: "Breadcrumb" }).filter({ hasText: "APP-TRON-CONSOLE" })).toBeVisible();
  await closeRightSidebar(page);
  await rightSidebarOpenControl(page).click();
  await expect(rightSidebar(page).getByText("Identifier")).toBeVisible();

  await routeActiveProfileApi(page);
  await page.goto(`${frontendBaseUrl}/prompts`);
  await openRightSidebar(page);
  await expectNoStaleInspector(page);
  await page.getByRole("row").filter({ hasText: "Tron Incident Summary" }).click();
  await expect(page).toHaveURL(/\/prompts\/detail\?promptCode=/);
  await expect(page.getByRole("textbox", { name: "Prompt code", exact: true })).toHaveAttribute("readonly", "");
  await expectNoStaleInspector(page);

  await routeActiveProfileApi(page);
  await page.goto(`${frontendBaseUrl}/variables`);
  await expectNoStaleInspector(page);
  const firstVariableRow = page.locator("tbody tr").first();
  const variableKey = (await firstVariableRow.locator("td").first().locator("p").first().textContent())?.trim();
  await firstVariableRow.click();
  await expectNoStaleInspector(page);
  if (variableKey) {
    await expect(page.getByRole("navigation", { name: "Breadcrumb" }).filter({ hasText: variableKey })).toBeVisible();
  }

  await routeActiveProfileApi(page);
  await page.goto(`${frontendBaseUrl}/knowledge-base`);
  await expect(rightSidebarOpenControl(page)).toBeVisible();
  await page.locator("tbody tr").filter({ hasText: "Operations SOP Library" }).click();
  await expect(rightSidebar(page).getByText("Selected KnowledgeBase")).toBeVisible();
  await expect(rightSidebar(page).getByText("Operations SOP Library")).toBeVisible();

  await routeActiveProfileApi(page);
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

  await routeActiveProfileApi(page);
  await page.goto(`${frontendBaseUrl}/settings/skills`);
  await expectNoStaleInspector(page);
});
