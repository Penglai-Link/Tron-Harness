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

async function signIn(page: Page, targetPath: string, expectedPath = targetPath) {
  await page.goto(`${frontendBaseUrl}${targetPath}`);
  await expect(page).toHaveURL(new RegExp(`/login\\?redirect=${encodeURIComponent(targetPath).replace(/\//g, "%2F")}$`));

  await page.getByLabel(/operator id|username|email/i).fill("auth.operator");
  await page.getByLabel(/passkey|password/i).fill("TronAuthPass123!");
  await page.getByRole("button", { name: /sign in/i }).click();
  await expect(page).toHaveURL(new RegExp(`${expectedPath.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}$`));
}

function rightSidebar(page: Page) {
  return page.getByTestId("tron-right-sidebar");
}

function rightReopenControl(page: Page) {
  return page.getByTestId("tron-right-sidebar-reopen");
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
  await expect(rightSidebar(page)).toBeVisible();
  await expect(rightSidebar(page).getByRole("heading", { name: "Resource Pattern" })).toBeVisible();
  await expect(rightReopenControl(page)).toHaveCount(0);

  const mainBefore = await page.locator("main").first().boundingBox();
  expect(mainBefore, "The shell main region should be rendered before collapse.").not.toBeNull();

  await page.getByTestId("tron-right-sidebar-collapse").click();

  await expect(rightSidebar(page)).toHaveCount(0);
  await expect(rightReopenControl(page)).toBeVisible();
  await expect(page.getByRole("heading", { name: "Resource Pattern" })).toHaveCount(0);

  const mainAfterCollapse = await page.locator("main").first().boundingBox();
  expect(mainAfterCollapse, "The shell main region should remain rendered after collapse.").not.toBeNull();
  expect(mainAfterCollapse!.width).toBeGreaterThan(mainBefore!.width + 250);
  await expect.poll(() => page.evaluate(() => {
    const raw = window.localStorage.getItem("tron-app-shell");
    return raw ? JSON.parse(raw).state.rightSidebarCollapsed : undefined;
  })).toBe(true);

  await rightReopenControl(page).click();

  await expect(rightSidebar(page)).toBeVisible();
  await expect(rightSidebar(page).getByRole("heading", { name: "Resource Pattern" })).toBeVisible();
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
  await expect(page.getByTestId("tron-mobile-right-sidebar").getByRole("heading", { name: "Resource Pattern" })).toBeVisible();
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
  await routeAuthApi(page);
  await signIn(page, "/chats", "/chats/8842-alpha");
  await expect(page).toHaveURL(/\/chats\/8842-alpha$/);
  await expect(rightSidebar(page).getByRole("heading", { name: "Utility Hub" })).toBeVisible();

  await page.goto(`${frontendBaseUrl}/observability`);
  await expect(page.getByRole("heading", { name: "Observability Dashboard" })).toBeVisible();
  await expect(rightSidebar(page).getByRole("heading", { name: "Inspector" })).toBeVisible();
  await rightSidebar(page).getByRole("button", { name: "Close inspector" }).click();
  await expectNoRightSidebarControls(page);

  await page.goto(`${frontendBaseUrl}/shared-shell-preview`);
  await expect(rightSidebar(page).getByRole("heading", { name: "Resource Pattern" })).toBeVisible();

  await page.goto(`${frontendBaseUrl}/dashboard`);
  await expect(page.getByRole("heading", { name: "Workspace Overview" })).toBeVisible();
  await expectNoRightSidebarControls(page);

  await page.goto(`${frontendBaseUrl}/settings/general`);
  await expect(page.getByRole("heading", { name: "General" })).toBeVisible();
  await expect(page.locator("#tron-workspace-trigger")).toHaveCount(0);
  await expectNoRightSidebarControls(page);
});
