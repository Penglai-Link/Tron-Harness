import { routeActiveProfileApi } from "./support/resource-ui-harness";
import { expect, test, type Page, type Route } from "@playwright/test";
import { createServer } from "node:net";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

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

type AppRuntimeRole = "Frontend" | "Backend";
type AppStatus = "Active" | "Archived";
type CapturedRequest = { body: unknown; method: string; pathname: string; searchParams: Record<string, string> };
type AppRecord = {
  appCode: string;
  archivedAt: string | null;
  createdAt: string;
  defaultBranch: string | null;
  description: string | null;
  id: string;
  localDirectory: string;
  name: string;
  repositoryUrl: string | null;
  runtimeRole: AppRuntimeRole;
  status: AppStatus;
  tags: string[];
  updatedAt: string;
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

async function seedTronCoreShellSelection(page: Page) {
  await page.addInitScript((snapshot) => {
    window.localStorage.setItem("tron-app-shell", JSON.stringify({
      state: {
        openSections: { Core: true },
        selectedPrimarySpaceId: "ENTERPRISE",
        selectedSecondarySpaceId: "TRON_CORE",
        sidebarCollapsed: false,
        workspaceSwitcherSnapshot: snapshot,
      },
      version: 9,
    }));
  }, workspaceSwitcher);
}

function appRecord(values: Partial<AppRecord> & Pick<AppRecord, "appCode" | "name" | "workspaceCode">): AppRecord {
  return {
    appCode: values.appCode,
    archivedAt: null,
    createdAt: now,
    defaultBranch: "master",
    description: null,
    id: values.id ?? values.appCode.replace("APP-", "7349"),
    localDirectory: "/workspace/tron/app",
    name: values.name,
    repositoryUrl: null,
    runtimeRole: "Frontend",
    status: "Active",
    tags: [],
    updatedAt: now,
    workspaceCode: values.workspaceCode,
    ...values,
  };
}

function successEnvelope(data: unknown) {
  return { code: "SUCCESS", message: "ok", data };
}

function pageEnvelope(records: unknown[], current = 1, size = 15) {
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

function appMatchesSearch(app: AppRecord, search?: string) {
  if (!search?.trim()) return true;
  return [
    app.appCode,
    app.name,
    app.description ?? "",
    app.localDirectory,
    app.repositoryUrl ?? "",
    app.defaultBranch ?? "",
    app.tags.join(" "),
  ].join(" ").toLowerCase().includes(search.trim().toLowerCase());
}

function filterApps(apps: AppRecord[], workspaceCode: string, query: Record<string, string>) {
  return apps
    .filter((app) => app.workspaceCode === workspaceCode)
    .filter((app) => query.runtimeRole ? app.runtimeRole === query.runtimeRole : true)
    .filter((app) => {
      if (query.status) return app.status === query.status;
      if (query.includeArchived === "true") return true;
      return app.status === "Active";
    })
    .filter((app) => appMatchesSearch(app, query.search))
    .sort((left, right) => left.name.localeCompare(right.name) || left.appCode.localeCompare(right.appCode));
}

async function routeApi(
  page: Page,
  requests: CapturedRequest[],
  apps: AppRecord[],
  options: {
    detailFailureStatusByAppCode?: Record<string, number>;
  } = {},
) {
  let next = 1;

  await page.route("**/api/v1/**", async (route) => {
    const captured = captureRequest(route);
    requests.push(captured);

    if (captured.pathname === "/api/v1/auth/providers") return fulfillJson(route, successEnvelope({ providers: [{ code: "internal", name: "Internal Account", type: "internal" }] }));
    if (captured.pathname === "/api/v1/auth/refresh") return fulfillJson(route, successEnvelope({ accessToken: "app-token", tokenType: "Bearer", expiresIn: 900, session: authenticatedSession }));
    if (captured.pathname === "/api/v1/auth/session") return fulfillJson(route, successEnvelope(authenticatedSession));
    if (captured.pathname === "/api/v1/auth/logout") return fulfillJson(route, successEnvelope({ loggedOut: true }));
    if (captured.pathname === "/api/v1/admin/workspaces") return fulfillJson(route, successEnvelope(workspaceTree));
    if (captured.pathname === "/api/v1/admin/workspaces/options") return fulfillJson(route, successEnvelope(workspaceOptions));
    if (captured.pathname === "/api/v1/admin/workspaces/switcher") return fulfillJson(route, successEnvelope(workspaceSwitcher));

    const appOptionsMatch = captured.pathname.match(/^\/api\/v1\/apps\/workspaces\/([^/]+)\/options$/);
    if (appOptionsMatch) {
      const workspaceCode = decodeURIComponent(appOptionsMatch[1]);
      const records = apps
        .filter((app) => app.workspaceCode === workspaceCode && app.status === "Active")
        .map((app) => ({
          appCode: app.appCode,
          name: app.name,
          description: app.description,
          runtimeRole: app.runtimeRole,
        }));
      return fulfillJson(route, successEnvelope(records));
    }

    const appListMatch = captured.pathname.match(/^\/api\/v1\/apps\/workspaces\/([^/]+)$/);
    if (appListMatch && captured.method === "GET") {
      const workspaceCode = decodeURIComponent(appListMatch[1]);
      return fulfillJson(route, pageEnvelope(filterApps(apps, workspaceCode, captured.searchParams)));
    }
    if (appListMatch && captured.method === "POST") {
      const workspaceCode = decodeURIComponent(appListMatch[1]);
      const body = captured.body as Partial<AppRecord>;
      if (apps.some((app) => app.workspaceCode === workspaceCode && app.name.toLowerCase() === (body.name ?? "").toLowerCase())) {
        return fulfillJson(route, { code: "APP_NAME_DUPLICATE", message: "app name already exists in the current workspace", data: {} }, 409);
      }
      const appCode = `APP-PW-${String(next).padStart(4, "0")}`;
      next += 1;
      const created = appRecord({
        appCode,
        name: body.name ?? "Untitled app",
        workspaceCode,
        defaultBranch: body.defaultBranch ?? null,
        description: body.description ?? null,
        localDirectory: body.localDirectory ?? "/workspace/tron/app",
        repositoryUrl: body.repositoryUrl ?? null,
        runtimeRole: (body.runtimeRole as AppRuntimeRole | undefined) ?? "Frontend",
        tags: Array.isArray(body.tags) ? body.tags : [],
      });
      apps.push(created);
      return fulfillJson(route, successEnvelope(created), 201);
    }

    const lifecycleMatch = captured.pathname.match(/^\/api\/v1\/apps\/([^/]+)\/(archive|restore)$/);
    if (lifecycleMatch) {
      const selected = apps.find((app) => app.appCode === decodeURIComponent(lifecycleMatch[1]));
      if (!selected) return fulfillJson(route, { code: "APP_NOT_FOUND", message: "app was not found", data: {} }, 404);
      if (lifecycleMatch[2] === "archive") {
        selected.status = "Archived";
        selected.archivedAt = now;
      } else {
        selected.status = "Active";
        selected.archivedAt = null;
      }
      selected.updatedAt = now;
      return fulfillJson(route, successEnvelope({ appCode: selected.appCode, status: selected.status, archivedAt: selected.archivedAt }));
    }

    const appDetailMatch = captured.pathname.match(/^\/api\/v1\/apps\/([^/]+)$/);
    if (appDetailMatch) {
      const appCode = decodeURIComponent(appDetailMatch[1]);
      const selected = apps.find((app) => app.appCode === appCode);
      if (!selected) return fulfillJson(route, { code: "APP_NOT_FOUND", message: "app was not found", data: {} }, 404);
      if (captured.method === "GET") {
        const failureStatus = options.detailFailureStatusByAppCode?.[appCode];
        if (failureStatus) {
          return fulfillJson(route, { code: "APP_DETAIL_FAILED", message: "app detail request failed", data: {} }, failureStatus);
        }
        return fulfillJson(route, successEnvelope(selected));
      }
      if (captured.method === "PATCH") {
        const body = captured.body as Partial<AppRecord>;
        selected.name = body.name ?? selected.name;
        selected.description = body.description ?? selected.description;
        selected.localDirectory = body.localDirectory ?? selected.localDirectory;
        selected.runtimeRole = (body.runtimeRole as AppRuntimeRole | undefined) ?? selected.runtimeRole;
        selected.repositoryUrl = body.repositoryUrl ?? selected.repositoryUrl;
        selected.defaultBranch = body.defaultBranch ?? selected.defaultBranch;
        selected.tags = Array.isArray(body.tags) ? body.tags : selected.tags;
        selected.updatedAt = now;
        return fulfillJson(route, successEnvelope(selected));
      }
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
  frontendProcess = spawnManagedProcess("npm", ["run", "dev", "--", "--host", "127.0.0.1", "--port", String(port)], {
    cwd: frontendRoot,
    env: {
      ...process.env,
      CI: "1",
      VITE_TRON_ADMIN_DATA_SOURCE: "api",
      VITE_TRON_APP_DATA_SOURCE: "api",
      VITE_TRON_DATA_SOURCE: "api",
    },
    stdio: "pipe",
  });
  frontendProcess.stdout.on("data", (chunk) => { frontendLogs = `${frontendLogs}${chunk.toString()}`.slice(-5000); });
  frontendProcess.stderr.on("data", (chunk) => { frontendLogs = `${frontendLogs}${chunk.toString()}`.slice(-5000); });
  await waitForFrontend(`${frontendBaseUrl}/apps`);
});

test.afterAll(async () => {
  try {
    await stopManagedProcess(frontendProcess);
  } finally {
    frontendProcess = undefined;
  }
});

test("app management supports workspace list create detail edit archive restore and removes import actions", async ({ page }) => {
  const requests: CapturedRequest[] = [];
  const apps: AppRecord[] = [
    appRecord({
      appCode: "APP-TRON-CONSOLE",
      name: "Tron Console",
      workspaceCode: "TRON_CORE",
      localDirectory: "/workspace/tron/tron-frontend",
      description: "Primary operator-facing frontend.",
      repositoryUrl: "https://github.com/clouds3n/tron-console",
      runtimeRole: "Frontend",
      tags: ["Operator", "React"],
    }),
    appRecord({
      appCode: "APP-TRON-API",
      name: "Tron API",
      workspaceCode: "TRON_CORE",
      localDirectory: "/workspace/tron/tron-backend",
      description: "Core backend serving task workflows.",
      repositoryUrl: "https://github.com/clouds3n/tron-backend",
      runtimeRole: "Backend",
      tags: ["Core", "Rust"],
    }),
    appRecord({
      appCode: "APP-TRON-LEGACY",
      name: "Legacy Console",
      workspaceCode: "TRON_CORE",
      archivedAt: now,
      localDirectory: "/workspace/tron/legacy-console",
      description: "Archived frontend kept for historical reference.",
      runtimeRole: "Frontend",
      status: "Archived",
      tags: ["Legacy"],
    }),
    appRecord({
      appCode: "APP-COMPLIANCE-API",
      name: "Compliance API",
      workspaceCode: "COMPLIANCE",
      localDirectory: "/workspace/apps/compliance-api",
      description: "Backend for regulated workflows.",
      runtimeRole: "Backend",
      tags: ["Audit"],
    }),
  ];
  await routeApi(page, requests, apps);
  await seedTronCoreShellSelection(page);

  await routeActiveProfileApi(page);
  await page.goto(`${frontendBaseUrl}/apps`);

  await expect(page.getByRole("tab", { name: "Apps", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: /^Import$/ })).toHaveCount(0);
  await expect(page.getByText("Tron Console")).toBeVisible();
  await expect(page.getByText("Compliance API")).toHaveCount(0);
  await expect.poll(() => requests.some((request) => request.pathname === "/api/v1/apps/workspaces/TRON_CORE")).toBe(true);

  await page.getByLabel("App status").click(); await page.getByRole("option", { name: "Archived", exact: true }).click();
  await expect(page.getByText("Legacy Console")).toBeVisible();
  await expect(page.getByText("Tron API")).toHaveCount(0);

  await page.getByLabel("App status").click(); await page.getByRole("option", { name: "Active", exact: true }).click();
  await page.getByLabel("Runtime role").click(); await page.getByRole("option", { name: "Backend", exact: true }).click();
  await expect(page.getByText("Tron API")).toBeVisible();
  await expect(page.getByText("Tron Console")).toHaveCount(0);

  await page.getByLabel("Runtime role").click(); await page.getByRole("option", { name: "All roles", exact: true }).click();
  await page.getByLabel("Search apps").fill("Console");
  await expect(page.getByText("Tron Console")).toBeVisible();
  await expect(page.getByText("Tron API")).toHaveCount(0);

  await page.getByLabel("Search apps").fill("");
  await page.getByRole("button", { name: "New", exact: true }).click();
  await expect(page.getByLabel("App name")).toBeVisible();
  await page.getByLabel("App name").fill("Playwright Runtime");
  await page.getByLabel("Local directory").fill("/workspace/apps/playwright-runtime");
  await page.getByLabel("Runtime role").click(); await page.getByRole("option", { name: "Backend", exact: true }).click();
  await page.getByLabel("Repository URL").fill("https://github.com/clouds3n/playwright-runtime");
  await page.getByLabel("Default branch").fill("main");
  await page.getByLabel("App tags").fill("Automation, Playwright");
  await page.getByLabel("App description").fill("Backend app created from the smoke flow.");
  await page.locator("form").evaluate((form) => (form as HTMLFormElement).requestSubmit());

  await expect(page).toHaveURL(/\/apps\/detail\?appCode=APP-PW-0001/);
  await expect(page.getByRole("navigation", { name: "Breadcrumb" }).filter({ hasText: "APP-PW-0001" })).toBeVisible();
  await expect(page.getByText("Backend", { exact: true })).toBeVisible();
  await expect(page.locator("main").getByText("/workspace/apps/playwright-runtime")).toBeVisible();
  await expect(page.getByRole("main").getByText("https://github.com/clouds3n/playwright-runtime")).toBeVisible();
  await expect.poll(() => requests.some((request) => request.method === "POST" && request.pathname === "/api/v1/apps/workspaces/TRON_CORE" && (request.body as Partial<AppRecord> | null)?.runtimeRole === "Backend" && (request.body as Partial<AppRecord> | null)?.localDirectory === "/workspace/apps/playwright-runtime")).toBe(true);

  await page.getByRole("button", { name: "Edit" }).click();
  await expect(page.getByLabel("App name")).toBeVisible();
  await page.getByLabel("App name").fill("Playwright Runtime Updated");
  await page.getByLabel("Local directory").fill("/workspace/tron/playwright-runtime");
  await page.getByLabel("Runtime role").click(); await page.getByRole("option", { name: "Frontend", exact: true }).click();
  await page.getByLabel("Default branch").fill("develop");
  await page.getByLabel("App tags").fill("Automation, UI");
  await page.getByLabel("App description").fill("Frontend app updated from the smoke flow.");
  await page.getByRole("button", { name: "Save" }).click();

  await expect(page).toHaveURL(/\/apps\/detail\?appCode=APP-PW-0001/);
  await expect(page.getByRole("navigation", { name: "Breadcrumb" }).filter({ hasText: "APP-PW-0001" })).toBeVisible();
  await expect(page.locator("span.inline-flex").filter({ hasText: "Frontend" }).first()).toBeVisible();
  await expect(page.getByRole("main").getByText("/workspace/tron/playwright-runtime")).toBeVisible();
  await expect(page.getByRole("main").getByText("develop")).toBeVisible();
  await expect.poll(() => requests.some((request) => request.method === "PATCH" && request.pathname === "/api/v1/apps/APP-PW-0001" && (request.body as Partial<AppRecord> | null)?.runtimeRole === "Frontend" && (request.body as Partial<AppRecord> | null)?.localDirectory === "/workspace/tron/playwright-runtime")).toBe(true);

  await page.getByRole("button", { name: "Archive" }).click();
  await expect(page.getByText("Archived", { exact: true })).toBeVisible();
  await expect.poll(() => requests.some((request) => request.method === "POST" && request.pathname === "/api/v1/apps/APP-PW-0001/archive")).toBe(true);

  await page.getByLabel("Breadcrumb").getByRole("link", { name: "Apps" }).click();
  await expect(page).toHaveURL(/\/apps$/);
  await page.getByLabel("App status").click(); await page.getByRole("option", { name: "Archived", exact: true }).click();
  await expect(page.getByText("Playwright Runtime Updated", { exact: true })).toBeVisible();
  await page.getByText("Playwright Runtime Updated", { exact: true }).click();
  await expect(page).toHaveURL(/\/apps\/detail\?appCode=APP-PW-0001/);
  await page.getByRole("button", { name: "Restore" }).first().click();
  await expect.poll(() => requests.some((request) => request.method === "POST" && request.pathname === "/api/v1/apps/APP-PW-0001/restore")).toBe(true);

  await page.getByLabel("Breadcrumb").getByRole("link", { name: "Apps" }).click();
  await expect(page).toHaveURL(/\/apps$/);
  await expect(page.getByText("Playwright Runtime Updated", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Open actions for Playwright Runtime Updated" }).click();
  await expect(page.getByRole("menuitem", { name: "Edit" })).toBeVisible();
});

test("app detail does not fall back to mock data when the API request fails", async ({ page }) => {
  const requests: CapturedRequest[] = [];
  const apps: AppRecord[] = [
    appRecord({
      appCode: "APP-TRON-API",
      name: "Tron API",
      workspaceCode: "TRON_CORE",
      localDirectory: "/workspace/tron/tron-backend",
      description: "Core backend serving task workflows.",
      repositoryUrl: "https://github.com/clouds3n/tron-backend",
      runtimeRole: "Backend",
      tags: ["Core", "Rust"],
    }),
  ];
  await routeApi(page, requests, apps, {
    detailFailureStatusByAppCode: {
      "APP-TRON-API": 500,
    },
  });
  await seedTronCoreShellSelection(page);

  await routeActiveProfileApi(page);
  await page.goto(`${frontendBaseUrl}/apps/detail?appCode=APP-TRON-API`);

  await expect(page.getByRole("navigation", { name: "Breadcrumb" })).toBeVisible();
  await expect(page.getByText("App not found", { exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Tron API" })).toHaveCount(0);
  await expect.poll(() => requests.some((request) => request.method === "GET" && request.pathname === "/api/v1/apps/APP-TRON-API")).toBe(true);
});
