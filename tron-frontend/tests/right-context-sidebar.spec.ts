import {
  expect,
  test,
  type Page,
  type Route,
} from "@playwright/test";
import {
  spawn,
  type ChildProcessWithoutNullStreams,
} from "node:child_process";
import { createServer } from "node:net";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { getWorkspaceSwitcherMock } from "../src/shared/constants/workspaceSpaces";

const thisFilePath = fileURLToPath(import.meta.url);
const frontendRoot = resolve(thisFilePath, "..", "..");

let frontendProcess: ChildProcessWithoutNullStreams | undefined;
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

async function stopFrontend() {
  const processToStop = frontendProcess;
  if (!processToStop || processToStop.exitCode !== null) return;

  const exited = new Promise((resolveExit) => {
    processToStop.once("exit", resolveExit);
  });
  processToStop.kill("SIGTERM");
  await Promise.race([exited, delay(2000)]);
  if (processToStop.exitCode === null) {
    processToStop.kill("SIGKILL");
  }
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

function rightReopenControl(page: Page) {
  return page.getByTestId("tron-right-sidebar-reopen");
}

async function openRightSidebar(page: Page) {
  if (await rightSidebar(page).count()) {
    await expect(rightSidebar(page)).toBeVisible();
    return;
  }
  await expect(rightReopenControl(page)).toBeVisible();
  await rightReopenControl(page).click();
  await expect(rightSidebar(page)).toBeVisible();
}

async function expectNoRightSidebarControls(page: Page) {
  await expect(rightSidebar(page)).toHaveCount(0);
  await expect(rightReopenControl(page)).toHaveCount(0);
  await expect(page.getByTestId("tron-mobile-right-sidebar-toggle")).toHaveCount(0);
}

test.beforeAll(async () => {
  const port = await reservePort();
  frontendBaseUrl = `http://127.0.0.1:${port}`;

  frontendProcess = spawn(
    "npm",
    ["run", "dev", "--", "--host", "127.0.0.1", "--port", String(port), "--strictPort"],
    {
      cwd: frontendRoot,
      env: {
        ...process.env,
        BROWSER: "none",
        VITE_TRON_API_BASE_URL: "",
        VITE_TRON_DATA_SOURCE: "mock",
        VITE_TRON_ADMIN_DATA_SOURCE: "mock",
        VITE_TRON_AGENT_DATA_SOURCE: "mock",
        VITE_TRON_APP_DATA_SOURCE: "mock",
        VITE_TRON_AUTOMATION_DATA_SOURCE: "mock",
        VITE_TRON_CHAT_DATA_SOURCE: "mock",
        VITE_TRON_MODEL_DATA_SOURCE: "mock",
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
  await stopFrontend();
});

test("AC-RCS-01: desktop right sidebar collapses out of layout, restores width, and reopens", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await routeAuthApi(page);
  await signIn(page, "/shared-shell-preview");

  await expect(page.getByRole("heading", { name: "Workspace console foundation" })).toBeVisible();
  await expect(rightSidebar(page)).toHaveCount(0);
  await expect(rightReopenControl(page)).toBeVisible();
  const mainBefore = await page.locator("main").first().boundingBox();
  expect(mainBefore, "The shell main region should be rendered before opening the default-collapsed sidebar.").not.toBeNull();

  await rightReopenControl(page).click();

  await expect(rightSidebar(page)).toBeVisible();
  await expect(rightSidebar(page).getByText("Resource Pattern")).toBeVisible();
  await expect(rightReopenControl(page)).toHaveCount(0);

  const mainAfterOpen = await page.locator("main").first().boundingBox();
  expect(mainAfterOpen, "The shell main region should remain rendered after opening the sidebar.").not.toBeNull();
  expect(mainBefore!.width).toBeGreaterThan(mainAfterOpen!.width + 250);

  await page.getByTestId("tron-right-sidebar-collapse").click();

  await expect(rightSidebar(page)).toHaveCount(0);
  await expect(rightReopenControl(page)).toBeVisible();
  await expect(page.getByRole("heading", { name: "Resource Pattern" })).toHaveCount(0);

  const mainAfterCollapse = await page.locator("main").first().boundingBox();
  expect(mainAfterCollapse, "The shell main region should remain rendered after collapse.").not.toBeNull();
  expect(mainAfterCollapse!.width).toBeGreaterThan(mainAfterOpen!.width + 250);
  await expect(rightReopenControl(page)).toBeVisible();

  await rightReopenControl(page).click();

  await expect(rightSidebar(page)).toBeVisible();
  await expect(rightSidebar(page).getByText("Resource Pattern")).toBeVisible();
  await expect(rightReopenControl(page)).toHaveCount(0);
});

test("AC-RCS-02: mobile right sidebar opens as a drawer and closes by all required paths", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await routeAuthApi(page);
  await signIn(page, "/shared-shell-preview");

  const drawerToggle = page.getByTestId("tron-mobile-right-sidebar-toggle");
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
  await page.goto(`${frontendBaseUrl}/dashboard`);
  await expect(page.getByRole("heading", { name: "Workspace Overview" })).toBeVisible();
  await expect(page.getByTestId("tron-mobile-right-sidebar")).toHaveCount(0);
});

test("AC-RCS-03: shell right-panel pages retain content and no-panel pages show no stale controls", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await seedWorkspaceSelection(page);
  await routeAuthApi(page);
  await signIn(page, "/chats/8842-alpha");
  await expect(page).toHaveURL(/\/chats\/8842-alpha$/);
  await page.getByText("Refactor discussion", { exact: true }).first().click();
  await openRightSidebar(page);
  await expect(rightSidebar(page).getByText("Run Context")).toBeVisible();

  await page.goto(`${frontendBaseUrl}/observability`);
  await expect(page.getByRole("heading", { name: "Observability Dashboard" })).toBeVisible();
  await openRightSidebar(page);
  await expect(rightSidebar(page).getByRole("heading", { name: "Inspector" })).toBeVisible();
  await rightSidebar(page).getByRole("button", { name: "Close inspector" }).click();
  await expectNoRightSidebarControls(page);

  await page.goto(`${frontendBaseUrl}/shared-shell-preview`);
  await openRightSidebar(page);
  await expect(rightSidebar(page).getByText("Resource Pattern")).toBeVisible();

  await page.goto(`${frontendBaseUrl}/dashboard`);
  await expect(page.getByRole("heading", { name: "Workspace Overview" })).toBeVisible();
  await expectNoRightSidebarControls(page);

  await page.goto(`${frontendBaseUrl}/settings/general`);
  await expect(page.getByRole("heading", { name: "General" })).toBeVisible();
  await expect(page.locator("#tron-workspace-trigger")).toHaveCount(0);
  await expectNoRightSidebarControls(page);
});

test("AC-RCS-04: graph and memory inspectors render in shell right sidebar and preserve selected context", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await routeAuthApi(page);
  await signIn(page, "/knowledge-graph");

  await expect(page.getByRole("heading", { name: "Entities" })).toBeVisible();
  await openRightSidebar(page);
  await expect(rightSidebar(page).getByText("Node Inspector")).toBeVisible();
  await expect(rightSidebar(page).getByText("Customer_0192")).toBeVisible();

  await page.getByTestId("tron-right-sidebar-collapse").click();
  await expect(rightSidebar(page)).toHaveCount(0);
  await expect(rightReopenControl(page)).toBeVisible();
  await page.locator("aside").filter({ has: page.getByRole("heading", { name: "Entities" }) }).getByRole("button", { name: /Order_TXN_55/ }).click();
  await expect(rightSidebar(page)).toBeVisible();
  await expect(rightSidebar(page).getByText("Order_TXN_55")).toBeVisible();
  await expect(rightSidebar(page).getByText("Transaction")).toBeVisible();

  await page.getByTestId("tron-right-sidebar-collapse").click();
  await expect(rightSidebar(page)).toHaveCount(0);
  await expect(rightReopenControl(page)).toBeVisible();
  await rightReopenControl(page).click();
  await expect(rightSidebar(page)).toBeVisible();
  await expect(rightSidebar(page).getByText("Order_TXN_55")).toBeVisible();

  await page.getByTestId("tron-right-sidebar-collapse").click();
  await expect(rightReopenControl(page)).toBeVisible();
  await rightReopenControl(page).click();
  await expect(rightSidebar(page)).toBeVisible();

  await page.goto(`${frontendBaseUrl}/knowledge-graph/domain-filtering`);
  await openRightSidebar(page);
  const domainGraphLayout = page.locator("aside", { has: page.getByRole("heading", { name: "Entities" }) }).locator("xpath=..");
  await expect(domainGraphLayout.locator("> aside")).toHaveCount(1);
  await expect(domainGraphLayout.locator("aside").filter({ hasText: "Node Inspector" })).toHaveCount(0);
  await expect(rightSidebar(page)).toBeVisible();
  await expect(rightSidebar(page).getByText("Node Inspector")).toBeVisible();
  await expect(rightSidebar(page).getByText("Customer_0192")).toBeVisible();
  await expect(page.locator("aside").filter({ hasText: "Domain" }).locator("select")).toHaveCount(1);
  await expect(page.locator("aside").filter({ has: page.getByRole("heading", { name: "Entities" }) }).getByRole("button", { name: /Customer_0192/ })).toBeVisible();

  await page.locator("aside").filter({ has: page.getByRole("heading", { name: "Entities" }) }).getByRole("button", { name: /Order_TXN_56/ }).click();
  await expect(rightSidebar(page).getByText("Order_TXN_56")).toBeVisible();
  await expect(rightSidebar(page).getByText("Node Inspector")).toBeVisible();

  await page.goto(`${frontendBaseUrl}/memory`);
  await openRightSidebar(page);
  await expect(rightSidebar(page).getByRole("button", { name: "Edit Fragment" })).toBeVisible();
  await expect(rightSidebar(page).getByText("User prefers terse, direct responses without conversational filler.")).toBeVisible();
  await page.getByTestId("tron-right-sidebar-collapse").click();
  await expect(rightSidebar(page)).toHaveCount(0);
  await expect(rightReopenControl(page)).toBeVisible();
  await page.getByRole("button", { name: /System default tone set to professional and helpful./ }).click();
  await expect(rightSidebar(page)).toBeVisible();
  await expect(rightSidebar(page).getByText("System default tone set to professional and helpful.")).toBeVisible();
  await page.getByTestId("tron-right-sidebar-collapse").click();
  await expect(rightReopenControl(page)).toBeVisible();
  await expect(rightSidebar(page)).toHaveCount(0);
  await rightReopenControl(page).click();
  await expect(rightSidebar(page).getByText("System default tone set to professional and helpful.")).toBeVisible();

  await page.goto(`${frontendBaseUrl}/memory/details`);
  await openRightSidebar(page);
  await expect(rightSidebar(page).getByRole("button", { name: "Edit Fragment" })).toBeVisible();
  await expect(rightSidebar(page).getByText("System default tone set to professional and helpful.", { exact: false })).toHaveCount(0);
  await page.getByRole("button", { name: /Project Alpha requires strict adherence to security protocols./ }).click();
  await expect(rightSidebar(page).getByText("Project Alpha requires strict adherence to security protocols outlined in doc-4A.")).toBeVisible();
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
  await page.getByRole("button", { name: "Edit Details" }).click();
  await expect(page.getByRole("textbox", { name: "Task detail title" })).toHaveValue("Fix task board workflow regression");
  await page.getByRole("button", { name: "Cancel" }).click();
  await expect(rightSidebar(page).getByRole("heading", { name: "Task Metadata" }).first()).toBeVisible();
  await page.getByTestId("tron-right-sidebar-collapse").click();
  await expect(rightSidebar(page)).toHaveCount(0);
  await expect(rightReopenControl(page)).toBeVisible();
  await rightReopenControl(page).click();
  await expect(rightSidebar(page).getByRole("heading", { name: "Task Metadata" }).first()).toBeVisible();
  await expect(page.locator("p.font-mono", { hasText: "TSK-SEED-BUG" }).first()).toBeVisible();

  await page.goto(`${frontendBaseUrl}/automations/detail?automationCode=AUT-734600000000000001`);
  await openRightSidebar(page);
  await expect(rightSidebar(page).getByText("Automation Details")).toBeVisible();
  await expect(rightSidebar(page).locator("p", { hasText: "Automation code" })).toBeVisible();
  await page.getByRole("heading", { name: "Execution instances" }).scrollIntoViewIfNeeded();
  await expect(page.getByRole("columnheader", { name: "Execution" })).toBeVisible();

  await page.goto(`${frontendBaseUrl}/apps/detail?appCode=APP-TRON-CONSOLE`);
  await openRightSidebar(page);
  await expect(rightSidebar(page).getByText("App Metadata")).toBeVisible();
  await expect(page.getByRole("heading", { name: "Tron Console" })).toBeVisible();
  await page.getByTestId("tron-right-sidebar-collapse").click();
  await rightReopenControl(page).click();
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

  await page.getByTestId("tron-right-sidebar-collapse").click();
  await rightReopenControl(page).click();
  await expect(rightSidebar(page).getByText("LLM Router System")).toBeVisible();

  await page.goto(`${frontendBaseUrl}/variables/details`);
  await openRightSidebar(page);
  await expect(rightSidebar(page).getByText("Variable Studio")).toBeVisible();
  await page.locator("tbody tr").first().click();
  await expect(rightSidebar(page).getByText("Variable Studio")).toBeVisible();

  await page.goto(`${frontendBaseUrl}/variables`);
  await expect(rightReopenControl(page)).toBeVisible();
  const firstVariableRow = page.locator("tbody tr").first();
  const variableKey = (await firstVariableRow.locator("td").first().textContent())?.trim();
  await firstVariableRow.click();
  await expect(rightSidebar(page).getByText("Variable Studio")).toBeVisible();
  if (variableKey) {
    await expect(rightSidebar(page).getByText(variableKey)).toBeVisible();
  }

  await page.goto(`${frontendBaseUrl}/knowledge-base`);
  await expect(rightReopenControl(page)).toBeVisible();
  await page.locator("tbody tr").filter({ hasText: "Operations SOP Library" }).click();
  await expect(rightSidebar(page).getByText("Selected KnowledgeBase")).toBeVisible();
  await expect(rightSidebar(page).getByText("Operations SOP Library")).toBeVisible();

  await page.goto(`${frontendBaseUrl}/skills`);
  await expect(rightReopenControl(page)).toBeVisible();
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
