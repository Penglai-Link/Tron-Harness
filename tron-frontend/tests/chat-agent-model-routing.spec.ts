import { routeActiveProfileApi } from "./support/resource-ui-harness";
import { expect, test, type Page, type Route } from "@playwright/test";
import { createServer } from "node:net";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { getWorkspaceSwitcherMock } from "../src/shared/constants/workspaceSpaces";
import {
  spawnManagedProcess,
  stopManagedProcess,
  type ManagedProcess,
} from "./support/managed-process";

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

async function expectMainComposerFocused(page: Page) {
  await expect.poll(() => page.evaluate(() => {
    const active = document.activeElement;
    return active instanceof HTMLTextAreaElement
      && active.placeholder === "Enter to send, Shift+Enter for newline"
      && active.closest("main") !== null;
  })).toBe(true);
}

async function selectTaskSessions(page: Page) {
  await page.goto(`${frontendBaseUrl}/chats?sessionCode=SESSION-8842`);
  await expect(page.getByText("Primary investigation", { exact: true }).first()).toBeVisible();
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
    await fulfillJson(route, successEnvelope({ providers: [{ code: "internal", name: "Internal Account", type: "internal" }] }));
  });

  await page.route("**/api/v1/auth/session", async (route) => {
    if (!currentSession || route.request().headers().authorization !== `Bearer ${currentAccessToken}`) {
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
}

function escapedPathPattern(path: string) {
  return new RegExp(`${path.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}$`);
}

async function signIn(page: Page, targetPath: string, expectedPath = targetPath) {
  const expectedUrlPattern = escapedPathPattern(expectedPath);
  const loginUrlPattern = new RegExp(`/login\\?redirect=${encodeURIComponent(targetPath).replace(/\//g, "%2F")}$`);
  const loginField = page.locator("input[autocomplete='username']").first();
  const passwordField = page.locator("input[autocomplete='current-password']").first();
  const signInButton = page.getByRole("button", { name: /sign in/i });

  await page.goto(`${frontendBaseUrl}${targetPath}`);
  if (!expectedUrlPattern.test(page.url()) && !loginUrlPattern.test(page.url())) {
    await page.waitForURL((url) => expectedUrlPattern.test(url.href) || loginUrlPattern.test(url.href));
  }

  if (expectedUrlPattern.test(page.url())) {
    await expect(page).toHaveURL(expectedUrlPattern);
    return;
  }

  await expect(page).toHaveURL(loginUrlPattern);
  await loginField.waitFor({ state: "visible", timeout: 10_000 });
  await passwordField.waitFor({ state: "visible", timeout: 10_000 });
  await loginField.fill("auth.operator");
  await passwordField.fill("TronAuthPass123!");
  await signInButton.click();
  await expect(page).toHaveURL(expectedUrlPattern);
}

test.beforeEach(async ({ page }) => {
  await routeActiveProfileApi(page);
  await page.addInitScript(() => window.localStorage.setItem("tron.locale", "en-US"));
});

test.afterEach(async ({ page }, info) => {
  if (info.status !== info.expectedStatus) {
    await page.screenshot({ path: info.outputPath("failure.png"), fullPage: true });
    console.log("UI SNAPSHOT", await page.locator("body").innerText());
  }
});

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
        VITE_TRON_API_BASE_URL: "",
        VITE_TRON_DATA_SOURCE: "mock",
        VITE_TRON_AUTH_DATA_SOURCE: "mock",
        VITE_TRON_ADMIN_DATA_SOURCE: "mock",
        VITE_TRON_CHAT_DATA_SOURCE: "mock",
        VITE_TRON_MODEL_DATA_SOURCE: "mock",
        VITE_TRON_AGENT_DATA_SOURCE: "mock",
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

test("chat agent and model selectors are linked and agent locks after first turn", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.addInitScript((snapshot) => {
    window.localStorage.setItem("tron.auth.mock.session", JSON.stringify({
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
    }));
    window.localStorage.removeItem("tron.auth.mock.loggedOut");
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

  await routeActiveProfileApi(page);
  await page.goto(`${frontendBaseUrl}/chats?sessionCode=SESSION-REFACTOR`);
  await expect(page).toHaveURL(/sessionCode=SESSION-REFACTOR$/);

  await expect(page.locator('a[href="/chats?sessionCode=SESSION-UNASSIGNED"]')).toHaveText("Historical trace review");

  await page.getByRole("button", { name: "New chat", exact: true }).click();

  const agentSelector = page.getByRole("button", { name: "Select chat agent", exact: true });
  const modelSelector = page.getByRole("button", { name: "Select chat model", exact: true });
  await expect(agentSelector).toBeEnabled();
  await expect(modelSelector).toBeEnabled();

  const agentBox = await agentSelector.boundingBox();
  const modelBox = await modelSelector.boundingBox();
  expect(agentBox).not.toBeNull();
  expect(modelBox).not.toBeNull();
  expect((agentBox?.x ?? 0)).toBeLessThan(modelBox?.x ?? 0);

  await modelSelector.click();
  const mainModelList = page.getByRole("menu");
  await expect(mainModelList.getByText("GPT-4o Main")).toBeVisible();
  const modelSearch = page.getByLabel("Search chat models");
  await modelSearch.type("GPT-4o Main");
  await expect(modelSearch).toHaveValue("GPT-4o Main");
  await page.getByRole("menuitemradio", { name: "GPT-4o Main", exact: true }).click();
  await expect(modelSelector).toContainText("GPT-4o Main");

  await agentSelector.click();
  const agentList = page.getByRole("menu");
  await agentList.getByRole("menuitem", { name: /Codex/i }).click();

  await modelSelector.click();
  const systemModelList = page.getByRole("menu");
  await expect(systemModelList.getByRole("menuitemradio", { name: "GPT-5 Codex", exact: true })).toBeVisible();
  await expect(systemModelList.getByText("GPT-4o Main")).toHaveCount(0);
  await systemModelList.getByRole("menuitemradio", { name: /GPT-5 Codex/ }).first().click();

  const composer = page.getByRole("textbox", { name: "Enter to send, Shift+Enter for newline", exact: true });
  await composer.fill("lock agent after first turn");
  await composer.press("Enter");

  await expect(page.getByText("lock agent after first turn").first()).toBeVisible();
  await expect(page.getByText("Recorded in")).toBeVisible();

  await expect(agentSelector).toBeVisible();
  await expect(agentSelector).toBeDisabled();
  await expect(agentSelector).toContainText("Codex");
  await expect(modelSelector).toBeEnabled();

  await modelSelector.click();
  await expect(
    page.getByRole("menu")
      .getByRole("menuitemradio", { name: "GPT-5 Codex", exact: true }),
  ).toBeVisible();
});

test("new sessions always start on main agent after browsing a system agent session", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.addInitScript((snapshot) => {
    window.localStorage.setItem("tron.auth.mock.session", JSON.stringify({
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
    }));
    window.localStorage.removeItem("tron.auth.mock.loggedOut");
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

  await routeActiveProfileApi(page);
  await page.goto(`${frontendBaseUrl}/chats?sessionCode=SESSION-REFACTOR`);
  await expect(page.getByRole("button", { name: "Select chat model", exact: true })).toContainText("GPT-5 Codex");

  await page.getByRole("button", { name: "New chat", exact: true }).click();

  await expect(page.getByRole("button", { name: "Select chat agent", exact: true })).toContainText("Main");
});

test("composer autofocuses when opening or creating a session on desktop", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.addInitScript((snapshot) => {
    window.localStorage.setItem("tron.auth.mock.session", JSON.stringify({
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
    }));
    window.localStorage.removeItem("tron.auth.mock.loggedOut");
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

  await page.goto(`${frontendBaseUrl}/chats`);
  await expect(page).toHaveURL(/\/chats$/);
  await selectTaskSessions(page);

  await expectMainComposerFocused(page);

  await page.goto(`${frontendBaseUrl}/chats?sessionCode=SESSION-REFACTOR`);
  await expectMainComposerFocused(page);

  await page.getByRole("button", { name: "New chat", exact: true }).click();
  await expectMainComposerFocused(page);
});

test("system agent sessions show history snapshot in context sidebar", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.addInitScript((snapshot) => {
    window.localStorage.setItem("tron.auth.mock.session", JSON.stringify({
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
    }));
    window.localStorage.removeItem("tron.auth.mock.loggedOut");
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

  await routeActiveProfileApi(page);
  await page.goto(`${frontendBaseUrl}/chats?sessionCode=SESSION-REFACTOR`);
  const openRightSidebarButton = page.getByRole("button", { name: "Open right sidebar" });
  await expect(page.getByRole("button", { name: "Select chat model", exact: true })).toContainText("GPT-5 Codex");
  await openRightSidebarButton.click();
  const rightSidebar = page.getByTestId("tron-right-sidebar");
  await rightSidebar.getByRole("tab", { name: "Short-term memory", exact: true }).click();
  await expect(rightSidebar.getByText("Session Total", { exact: true })).toBeVisible();
  await expect(rightSidebar.getByText("Current Session History")).toBeVisible();
});
