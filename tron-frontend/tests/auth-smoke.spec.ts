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
import { readFileSync } from "node:fs";
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
    spaceCodes: ["PLATFORM", "DATA"],
    status: "Active",
    userCode: "USR-AUTH-0001",
    username: "auth.operator",
  },
};

const oauthUser = {
  ...authUser,
  authMethod: "oauth2",
  provider: {
    code: "dev",
    name: "Development OAuth2",
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

async function fulfillJson(route: Route, data: unknown, status = 200) {
  await route.fulfill({
    status,
    contentType: "application/json",
    body: JSON.stringify(data),
  });
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

async function routeAuthApi(page: Page) {
  let currentSession: typeof authUser | typeof oauthUser | null = null;

  await page.route("**/api/v1/auth/providers", async (route) => {
    await fulfillJson(route, successEnvelope({
      providers: [
        { code: "internal", name: "Internal Account", type: "internal" },
        { code: "dev", name: "Development OAuth2", type: "oauth2" },
      ],
    }));
  });

  await page.route("**/api/v1/auth/session", async (route) => {
    if (!currentSession) {
      await fulfillJson(route, unauthorizedEnvelope(), 401);
      return;
    }

    await fulfillJson(route, successEnvelope(currentSession));
  });

  await page.route("**/api/v1/auth/login", async (route) => {
    const body = route.request().postDataJSON() as { login?: string; password?: string };
    if (body.login === "auth.operator" && body.password === "TronAuthPass123!") {
      currentSession = authUser;
      await fulfillJson(route, successEnvelope(authUser));
      return;
    }

    await fulfillJson(route, unauthorizedEnvelope("AUTH_INVALID_CREDENTIALS", "invalid credentials"), 401);
  });

  await page.route("**/api/v1/auth/logout", async (route) => {
    currentSession = null;
    await fulfillJson(route, successEnvelope({ loggedOut: true }));
  });

  await page.route("**/api/v1/auth/refresh", async (route) => {
    if (!currentSession) {
      await fulfillJson(route, unauthorizedEnvelope("AUTH_REFRESH_INVALID", "invalid refresh session"), 401);
      return;
    }

    await fulfillJson(route, successEnvelope(currentSession));
  });

  await page.route("**/api/v1/auth/oauth2/authorize/dev**", async (route) => {
    currentSession = oauthUser;
    const url = new URL(route.request().url());
    const redirect = url.searchParams.get("redirect") ?? "/dashboard";
    await route.fulfill({
      status: 302,
      headers: {
        location: `${frontendBaseUrl}/auth/callback?redirect=${encodeURIComponent(redirect)}`,
      },
      body: "",
    });
  });

  await page.route("**/api/v1/admin/users**", async (route) => {
    if (!currentSession) {
      await fulfillJson(route, unauthorizedEnvelope(), 401);
      return;
    }

    await fulfillJson(route, successEnvelope({
      current: 1,
      size: 20,
      total: 1,
      records: [
        {
          id: "9007199254740993",
          userCode: "USR-AUTH-0001",
          username: "auth.operator",
          firstName: "Auth",
          lastName: "Operator",
          displayName: "Auth Operator",
          email: "auth.operator@tron.ai",
          phone: null,
          avatarUrl: null,
          locale: "en-US",
          timezone: "UTC",
          roleCodes: ["ADMIN", "DEVELOPER"],
          spaceCodes: ["PLATFORM", "DATA"],
          status: "Active",
          passwordResetRequired: false,
          lastActiveAt: "2026-05-15T10:00:00.000Z",
          createdAt: "2026-05-15T09:00:00.000Z",
          updatedAt: "2026-05-15T10:00:00.000Z",
        },
      ],
    }));
  });

  await page.route("**/api/v1/admin/users/statistics/**", async (route) => {
    if (!currentSession) {
      await fulfillJson(route, unauthorizedEnvelope(), 401);
      return;
    }

    await fulfillJson(route, successEnvelope({ count: 1 }));
  });
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

test("redirects anonymous users to login, signs in, reaches user management, and returns to login after logout", async ({ page }) => {
  await routeAuthApi(page);

  await page.goto(`${frontendBaseUrl}/dashboard`);
  await expect(page).toHaveURL(/\/login\?redirect=%2Fdashboard$/);

  await page.getByLabel(/operator id|username|email/i).fill("auth.operator");
  await page.getByLabel(/passkey|password/i).fill("TronAuthPass123!");
  await page.getByRole("button", { name: /sign in/i }).click();

  await expect(page).toHaveURL(/\/dashboard$/);
  await page.goto(`${frontendBaseUrl}/admin/users`);
  await expect(page).toHaveURL(/\/admin\/users$/);

  await expect(page.getByRole("heading", { name: /users/i }).first()).toBeVisible();
  await expect(page.getByRole("table")).toContainText("Auth Operator");

  await page.getByRole("button", { name: /log out/i }).click();
  await expect(page).toHaveURL(/\/login/);

  await page.goto(`${frontendBaseUrl}/admin/users`);
  await expect(page).toHaveURL(/\/login\?redirect=%2Fadmin%2Fusers$/);
});

test("supports oauth2 entry and lands on the intended protected route", async ({ page }) => {
  await routeAuthApi(page);

  await page.goto(`${frontendBaseUrl}/login?redirect=%2Fadmin%2Fusers`);
  await page.getByRole("button", { name: /development oauth2/i }).click();

  await expect(page).toHaveURL(/\/admin\/users$/);
  await expect(page.getByRole("table")).toContainText("Auth Operator");
});
