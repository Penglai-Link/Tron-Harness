import {
  expect,
  test,
  type Page,
} from "@playwright/test";
import {
  spawn,
  type ChildProcessWithoutNullStreams,
} from "node:child_process";
import { readFileSync } from "node:fs";
import { createServer } from "node:net";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const thisFilePath = fileURLToPath(import.meta.url);
const frontendRoot = resolve(thisFilePath, "..", "..");
let frontendProcess: ChildProcessWithoutNullStreams | undefined;
let frontendLogs = "";
let frontendBaseUrl = "";

const alexUserRecord = {
  id: "9007199254740993",
  userCode: "USER_ALEX_JOHNSON",
  username: "alex.johnson",
  firstName: "Alex",
  lastName: "Johnson",
  displayName: "Alex Johnson",
  email: "alex.johnson@example.com",
  phone: null,
  avatarUrl: null,
  locale: "en-US",
  timezone: "UTC",
  roleCodes: ["ADMIN", "DEVELOPER"],
  spaceCodes: ["PLATFORM", "DATA"],
  status: "Active",
  passwordResetRequired: false,
  lastActiveAt: null,
  createdAt: "2026-05-13T00:00:00.000Z",
  updatedAt: "2026-05-13T00:00:00.000Z",
  archivedAt: null,
};

function source(pathFromFrontendRoot: string): string {
  return readFileSync(
    resolve(frontendRoot, pathFromFrontendRoot),
    "utf8",
  );
}

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

async function routeAdminUsers(page: Page, records: unknown[]) {
  await page.route("**/api/v1/admin/users**", async (route) => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        code: "SUCCESS",
        message: "ok",
        data: {
          current: 1,
          size: 20,
          total: records.length,
          records,
        },
      }),
    });
  });
}

test.beforeAll(async () => {
  const port = await reservePort();
  frontendBaseUrl = `http://127.0.0.1:${port}`;
  frontendProcess = spawn(
    "npm",
    ["run", "dev", "--", "--host", "127.0.0.1", "--port", String(port), "--strictPort"],
    {
      cwd: frontendRoot,
      env: { ...process.env, BROWSER: "none" },
      stdio: ["ignore", "pipe", "pipe"],
    },
  );
  frontendProcess.stdout.on("data", appendFrontendLog);
  frontendProcess.stderr.on("data", appendFrontendLog);

  await waitForFrontend(frontendBaseUrl);
});

test.afterAll(async () => {
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
});

test("TDD-UM-08: admin user model uses stable user-management boundary fields", async () => {
  const modelSource = source("src/domain/admin/model/AdminModel.ts");

  expect(modelSource).toContain("userCode");
  expect(modelSource).toContain("username");
  expect(modelSource).toContain("roleCodes");
  expect(modelSource).toContain("spaceCodes");

  expect(modelSource).not.toContain("workspaces:");
  expect(modelSource).not.toContain("roles:");
});

test("TDD-UM-08: admin user statuses are narrowed to Active or Archived only", async () => {
  const mockDataSource = source("src/domain/admin/mock/data.ts");
  const usersArrayBlock = mockDataSource.match(
    /const users:\s*UserRecord\[\]\s*=\s*\[(?<users>[\s\S]*?)\];/,
  )?.groups?.users;
  expect(usersArrayBlock).toBeDefined();

  const statusLiterals = Array.from(
    usersArrayBlock!.matchAll(/status:\s*"([^"]+)"/g),
    (match) => match[1],
  );

  expect(statusLiterals.length).toBeGreaterThan(0);
  for (const status of statusLiterals) {
    expect(["Active", "Archived"]).toContain(status);
  }
});

test("TDD-UM-08: user detail navigation is keyed by userCode query boundary", async () => {
  const usersPageSource = source("src/domain/admin/pages/UsersPage.tsx");
  const detailsPageSource = source("src/domain/admin/pages/UserDetailsPage.tsx");

  expect(usersPageSource).toContain("/admin/users/detail?userCode=");
  expect(detailsPageSource).toContain("get(\"userCode\")");
});

test("TDD-UM-08: admin api data source calls backend list endpoint", async ({ page }) => {
  let usersEndpointCalled = false;

  await page.route("**/api/v1/admin/users**", async (route) => {
    usersEndpointCalled = true;
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        code: "SUCCESS",
        message: "ok",
        data: {
          current: 1,
          size: 20,
          total: 1,
          records: [
            {
              id: "9007199254740993",
              userCode: "USER_API_SOURCE",
              username: "api.source",
              firstName: "Api",
              lastName: "Source",
              displayName: "Api Source",
              email: "api.source@example.com",
              phone: null,
              avatarUrl: null,
              locale: "en-US",
              timezone: "UTC",
              roleCodes: ["ADMIN"],
              spaceCodes: ["PLATFORM"],
              status: "Active",
              passwordResetRequired: false,
              lastActiveAt: null,
              createdAt: "2026-05-13T00:00:00.000Z",
              updatedAt: "2026-05-13T00:00:00.000Z",
              archivedAt: null,
            },
          ],
        },
      }),
    });
  });

  await page.goto(`${frontendBaseUrl}/admin/users`);

  await expect(page.getByText("Api Source")).toBeVisible();
  expect(usersEndpointCalled).toBe(true);
});

test("TDD-UM-08: missing or unknown userCode detail route exposes only not-found recovery", async ({ page }) => {
  await page.goto(`${frontendBaseUrl}/admin/users/detail?userCode=UNKNOWN`);

  await expect(page.getByRole("heading", { name: "User Detail" })).toBeVisible();
  await expect(page.getByRole("link", { name: "Back to Users" })).toBeVisible();
  await expect(page.getByText("No user record matches UNKNOWN.")).toBeVisible();
  await expect(page.getByText("Alex Johnson")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Archive User" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Restore User" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Reset Password" })).toHaveCount(0);
  await expect(page.getByText("Role Codes")).toHaveCount(0);
  await expect(page.getByText("Space Codes")).toHaveCount(0);

  await page.goto(`${frontendBaseUrl}/admin/users/detail`);

  await expect(page.getByRole("heading", { name: "User Detail" })).toBeVisible();
  await expect(page.getByRole("link", { name: "Back to Users" })).toBeVisible();
  await expect(page.getByText("No user record was requested.")).toBeVisible();
  await expect(page.getByText("Alex Johnson")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Archive User" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Restore User" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Reset Password" })).toHaveCount(0);
  await expect(page.getByText("Role Codes")).toHaveCount(0);
  await expect(page.getByText("Space Codes")).toHaveCount(0);
});

test("TDD-UM-08: valid userCode detail route keeps the user detail journey", async ({ page }) => {
  await routeAdminUsers(page, [alexUserRecord]);

  await page.goto(`${frontendBaseUrl}/admin/users/detail?userCode=USER_ALEX_JOHNSON`);

  await expect(page.getByRole("heading", { name: "Alex Johnson" })).toBeVisible();
  await expect(page.getByText("USER_ALEX_JOHNSON").first()).toBeVisible();
  await expect(page.getByRole("button", { name: "Archive User" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Reset Password" })).toBeVisible();
  await expect(page.getByText("Role Codes")).toBeVisible();
  await expect(page.getByText("Space Codes")).toBeVisible();
});
