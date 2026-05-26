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
const themeStorageKey = "tron.theme";

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

async function routeThemeApi(
  page: Page,
  options: {
    workspaceSwitcherData?: ReturnType<typeof getWorkspaceSwitcherMock>;
  } = {},
) {
  let currentSession: typeof authUser | null = null;
  let hasRefreshSession = false;
  let currentAccessToken = "";
  let nextAccessTokenId = 1;
  const workspaceSwitcherData = options.workspaceSwitcherData ?? getWorkspaceSwitcherMock();

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

  await page.route("**/api/v1/admin/workspaces/switcher", async (route) => {
    if (
      !currentSession
      || route.request().headers().authorization !== `Bearer ${currentAccessToken}`
    ) {
      await fulfillJson(route, unauthorizedEnvelope(), 401);
      return;
    }

    await fulfillJson(route, successEnvelope(workspaceSwitcherData));
  });

  await page.route("**/api/v1/dashboard/overview", async (route) => {
    if (
      !currentSession
      || route.request().headers().authorization !== `Bearer ${currentAccessToken}`
    ) {
      await fulfillJson(route, unauthorizedEnvelope(), 401);
      return;
    }

    await fulfillJson(route, successEnvelope({
      activities: [],
      metrics: [],
    }));
  });

  await page.route("**/api/v1/admin/platform/models", async (route) => {
    if (
      !currentSession
      || route.request().headers().authorization !== `Bearer ${currentAccessToken}`
    ) {
      await fulfillJson(route, unauthorizedEnvelope(), 401);
      return;
    }

    await fulfillJson(route, successEnvelope([]));
  });
}

async function signIn(page: Page) {
  await page.goto(`${frontendBaseUrl}/dashboard`);
  await expect(page).toHaveURL(/\/login\?redirect=%2Fdashboard$/);

  await page.getByLabel(/operator id|username|email/i).fill("auth.operator");
  await page.getByLabel(/passkey|password/i).fill("TronAuthPass123!");
  await page.getByRole("button", { name: /sign in/i }).click();
  await expect(page).toHaveURL(/\/dashboard$/);
}

test.beforeAll(async () => {
  const port = await reservePort();
  frontendBaseUrl = `http://127.0.0.1:${port}`;

  frontendProcess = spawn(
    "npm",
    ["run", "dev", "--", "--host", "127.0.0.1", "--port", String(port)],
    {
      cwd: frontendRoot,
      env: {
        ...process.env,
        VITE_TRON_API_BASE_URL: "",
        VITE_TRON_DATA_SOURCE: "api",
      },
      stdio: "pipe",
    },
  );

  frontendProcess.stdout.on("data", appendFrontendLog);
  frontendProcess.stderr.on("data", appendFrontendLog);

  await waitForFrontend(`${frontendBaseUrl}/login`);
});

test.afterAll(async () => {
  if (!frontendProcess) return;
  frontendProcess.kill("SIGTERM");
  await new Promise<void>((resolveProcess) => {
    frontendProcess?.once("exit", () => resolveProcess());
    setTimeout(() => resolveProcess(), 5_000);
  });
});

test("opens the full-screen settings page, switches settings sections, and persists theme changes", async ({ page }) => {
  await routeThemeApi(page);
  await signIn(page);

  const header = page.getByRole("banner");
  await expect(header.getByRole("button", { name: /open filters/i })).toHaveCount(0);
  await expect(header.getByRole("button", { name: /open system settings/i })).toBeVisible();

  await header.getByRole("button", { name: /open system settings/i }).click();
  await expect(page).toHaveURL(/\/settings\/general$/);
  await expect(page.locator("#tron-workspace-trigger")).toHaveCount(0);
  await expect(page.getByRole("link", { name: "Models" })).toBeVisible();

  const themeSelect = page.locator('label:has-text("Theme") select');
  await expect(themeSelect).toHaveValue("light");
  await expect.poll(() => page.evaluate(() => document.documentElement.dataset.theme)).toBe("light");
  await expect.poll(() => page.evaluate(() => getComputedStyle(document.body).backgroundColor)).toBe("rgb(253, 248, 248)");

  await themeSelect.selectOption("matrix-dark");

  await expect.poll(() => page.evaluate(() => document.documentElement.dataset.theme)).toBe("matrix-dark");
  await expect.poll(() => page.evaluate((storageKey) => window.localStorage.getItem(storageKey), themeStorageKey)).toBe("matrix-dark");
  await expect.poll(() => page.evaluate(() => getComputedStyle(document.body).backgroundColor)).toBe("rgb(1, 6, 3)");
  await expect.poll(() => header.evaluate((node) => getComputedStyle(node).backgroundColor)).toBe("rgba(3, 12, 8, 0.92)");

  await page.getByRole("link", { name: "Models" }).click();
  await expect(page).toHaveURL(/\/settings\/models$/);
  await expect(page.getByRole("heading", { name: "Platform Models" })).toBeVisible();
  await page.getByRole("link", { name: "General" }).click();
  await expect(page).toHaveURL(/\/settings\/general$/);

  await page.reload();
  await expect(page).toHaveURL(/\/settings\/general$/);
  await expect(page.locator('label:has-text("Theme") select')).toHaveValue("matrix-dark");

  await page.locator('label:has-text("Theme") select').selectOption("light");
  await expect.poll(() => page.evaluate(() => document.documentElement.dataset.theme)).toBe("light");
  await expect.poll(() => page.evaluate((storageKey) => window.localStorage.getItem(storageKey), themeStorageKey)).toBe("light");
  await expect.poll(() => page.evaluate(() => getComputedStyle(document.body).backgroundColor)).toBe("rgb(253, 248, 248)");
});

test("loads workspace selector data from api after sign in", async ({ page }) => {
  const workspaceSwitcherData = {
    primarySpaces: [
      {
        workspaceCode: "INDUSTRIAL",
        name: "工业互联网",
        description: "工业互联网总部空间",
        status: "Active" as const,
        childCount: 1,
      },
    ],
    secondarySpaces: [
      {
        workspaceCode: "INDUSTRIAL",
        primaryWorkspaceCode: "INDUSTRIAL",
        parentWorkspaceCode: null,
        label: "工业互联网",
        name: "工业互联网",
        description: "工业互联网总部空间",
        status: "Active" as const,
        depth: 1,
        childCount: 1,
        effectiveUserCount: 0,
      },
      {
        workspaceCode: "ELITEFORGE",
        primaryWorkspaceCode: "INDUSTRIAL",
        parentWorkspaceCode: "INDUSTRIAL",
        label: "工业互联网 / 璀璨工坊",
        name: "璀璨工坊",
        description: "私有云环境",
        status: "Active" as const,
        depth: 2,
        childCount: 1,
        effectiveUserCount: 0,
      },
    ],
  };

  await routeThemeApi(page, { workspaceSwitcherData });
  await page.addInitScript((snapshot) => {
    window.localStorage.setItem("tron-app-shell", JSON.stringify({
      state: {
        openSections: { Core: true },
        selectedPrimarySpaceId: "ENTERPRISE",
        selectedSecondarySpaceId: "TRON_CORE",
        sidebarCollapsed: false,
        workspaceSwitcherSnapshot: snapshot,
      },
      version: 0,
    }));
  }, getWorkspaceSwitcherMock());
  await signIn(page);

  const workspaceTrigger = page.locator("#tron-workspace-trigger");
  await expect(workspaceTrigger).toContainText("工业互联网/璀璨工坊");
  await expect(workspaceTrigger).not.toContainText("Enterprise/Platform/Tron Core");
});

test("dashboard inheritance map keeps descendant workspaces nested under their direct parent", async ({ page }) => {
  const workspaceSwitcherData = {
    primarySpaces: [
      {
        workspaceCode: "INDUSTRIAL",
        name: "工业互联网",
        description: "工业互联网总部空间",
        status: "Active" as const,
        childCount: 1,
      },
    ],
    secondarySpaces: [
      {
        workspaceCode: "ELITEFORGE",
        primaryWorkspaceCode: "INDUSTRIAL",
        parentWorkspaceCode: "INDUSTRIAL",
        label: "工业互联网 / 璀璨工坊",
        name: "璀璨工坊",
        description: "私有云环境",
        status: "Active" as const,
        depth: 2,
        childCount: 1,
        effectiveUserCount: 0,
      },
      {
        workspaceCode: "ONLINE_DOCS",
        primaryWorkspaceCode: "INDUSTRIAL",
        parentWorkspaceCode: "ELITEFORGE",
        label: "工业互联网 / 璀璨工坊 / 在线文档",
        name: "在线文档",
        description: "知识协同工作区",
        status: "Active" as const,
        depth: 3,
        childCount: 0,
        effectiveUserCount: 0,
      },
    ],
  };

  await routeThemeApi(page, { workspaceSwitcherData });
  await page.addInitScript((snapshot) => {
    window.localStorage.setItem("tron-app-shell", JSON.stringify({
      state: {
        openSections: { Core: true },
        selectedPrimarySpaceId: "INDUSTRIAL",
        selectedSecondarySpaceId: "ONLINE_DOCS",
        sidebarCollapsed: false,
        workspaceSwitcherSnapshot: snapshot,
      },
      version: 0,
    }));
  }, workspaceSwitcherData);
  await signIn(page);

  const eliteforgeNode = page.locator('[data-workspace-code="ELITEFORGE"]');
  const onlineDocsNode = page.locator('[data-workspace-code="ONLINE_DOCS"]');

  await expect(eliteforgeNode).toContainText("璀璨工坊");
  await expect(onlineDocsNode).toContainText("在线文档");
  await expect(eliteforgeNode.locator('[data-workspace-code="ONLINE_DOCS"]')).toHaveCount(1);

  const nearestWorkspaceAncestor = await onlineDocsNode.evaluate((node) => {
    let current = node.parentElement;
    while (current) {
      if (current.hasAttribute("data-workspace-code")) {
        return current.getAttribute("data-workspace-code");
      }
      current = current.parentElement;
    }

    return null;
  });
  expect(nearestWorkspaceAncestor).toBe("ELITEFORGE");
});

test("sidebar groups knowledge and resource navigation under data", async ({ page }) => {
  await routeThemeApi(page);
  await signIn(page);

  const navigation = page.getByRole("navigation", { name: "Tron primary navigation" });
  const dataToggle = navigation.getByRole("button", { name: "Data" });
  await dataToggle.click();
  const dataSection = dataToggle.locator("xpath=ancestor::section[1]");
  const agentOperationsSection = navigation.getByRole("button", { name: "Agent Operations" }).locator("xpath=ancestor::section[1]");

  await expect(dataSection.getByRole("link", { name: "KnowledgeGraph" })).toBeVisible();
  await expect(dataSection.getByRole("link", { name: "KnowledgeBase" })).toBeVisible();
  await expect(dataSection.getByRole("link", { name: "Resources" })).toBeVisible();
  await expect(agentOperationsSection.getByRole("link", { name: "Resources" })).toHaveCount(0);

  await dataSection.getByRole("link", { name: "KnowledgeBase" }).click();
  await expect(page).toHaveURL(/\/knowledge-base$/);
  await expect(page.getByRole("heading", { name: "Knowledge Base" })).toBeVisible();
});

test("sidebar navigation still works after entering chats", async ({ page }) => {
  await routeThemeApi(page);
  await signIn(page);

  await page.goto(`${frontendBaseUrl}/chats`);
  await expect(page).toHaveURL(/\/chats\/8842-alpha$/);
  await expect(page.getByRole("heading", { name: "Primary Investigation" })).toBeVisible();

  await page.getByRole("link", { name: "Dashboard" }).click();
  await expect(page).toHaveURL(/\/dashboard$/);
  await expect(page.getByRole("heading", { name: "Workspace Overview" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Primary Investigation" })).toHaveCount(0);

  const dataToggle = page.getByRole("button", { name: "Data" });
  const resourcesLink = page.getByRole("link", { name: "Resources" });
  if (!(await resourcesLink.isVisible())) {
    await dataToggle.click();
  }
  await resourcesLink.click();
  await expect(page).toHaveURL(/\/resources$/);
  await expect(page.getByRole("heading", { name: "Data & Assets" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Primary Investigation" })).toHaveCount(0);
});
