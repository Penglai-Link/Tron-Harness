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

const thisFilePath = fileURLToPath(import.meta.url);
const frontendRoot = resolve(thisFilePath, "..", "..");
const STORAGE_KEY = "tron.admin.role-matrix.v1";
let frontendProcess: ManagedProcess | undefined;
let frontendLogs = "";
let frontendBaseUrl = "";
const authenticatedSession = {
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
const authenticatedGrant = {
  accessToken: "role-matrix-token",
  tokenType: "Bearer",
  expiresIn: 900,
  session: authenticatedSession,
};

const permissionDomainList = ["Workspace", "Models", "Tasks", "Memory", "Skills", "MCP"];
type RoleCatalogRecord = {
  code: string;
  description: string;
  id: string;
  kind: "System" | "Custom";
  managedBy: "Platform" | "Workspace";
  name: string;
  permissions: Record<string, string[]>;
  updatedAt: string;
};

type ReplaceRolesPayload = {
  roles: RoleCatalogRecord[];
};

function createRoleCatalog(): RoleCatalogRecord[] {
  return [
    {
      id: "734300000000000001",
      code: "ADMIN",
      name: "Admin",
      description: "Full System Access",
      kind: "System",
      managedBy: "Platform",
      permissions: Object.fromEntries(permissionDomainList.map((domain) => [domain, ["R", "W", "D", "A"]])),
      updatedAt: "2026-05-15T00:00:00Z",
    },
    {
      id: "734300000000000002",
      code: "WORKSPACE_ADMIN",
      name: "Workspace Admin",
      description: "Workspace administration",
      kind: "System",
      managedBy: "Workspace",
      permissions: {
        Workspace: ["R", "W"],
        Models: ["R", "W"],
        Tasks: ["R", "W"],
        Memory: ["R"],
        Skills: ["R", "W"],
        MCP: ["R"],
      },
      updatedAt: "2026-05-15T00:00:00Z",
    },
    {
      id: "734300000000000003",
      code: "DEVELOPER",
      name: "Developer",
      description: "Build and operate applications",
      kind: "System",
      managedBy: "Platform",
      permissions: {
        Workspace: ["R"],
        Models: ["R", "W"],
        Tasks: ["R", "W"],
        Memory: ["R"],
        Skills: ["R", "W"],
        MCP: ["R"],
      },
      updatedAt: "2026-05-15T00:00:00Z",
    },
    {
      id: "734300000000000004",
      code: "VIEWER",
      name: "Viewer",
      description: "Read-only Access",
      kind: "System",
      managedBy: "Platform",
      permissions: Object.fromEntries(permissionDomainList.map((domain) => [domain, ["R"]])),
      updatedAt: "2026-05-15T00:00:00Z",
    },
  ];
}
const users = [
  {
    id: "734102998871252992",
    userCode: "USER_ALEX_JOHNSON",
    username: "alex.johnson",
    firstName: "Alex",
    lastName: "Johnson",
    displayName: "Alex Johnson",
    email: "alex@tron.ai",
    phone: { countryCode: "+1", number: "5550100" },
    avatarUrl: null,
    locale: "en-US",
    timezone: "America/Los_Angeles",
    roleCodes: ["ADMIN", "DEVELOPER"],
    spaceCodes: ["PLATFORM", "DATA"],
    status: "Active",
    passwordResetRequired: false,
    lastActiveAt: "2026-05-11T08:30:00Z",
    createdAt: "2026-05-11T08:00:00Z",
    updatedAt: "2026-05-11T08:00:00Z",
    archivedAt: null,
  },
  {
    id: "734103002743185408",
    userCode: "USER_MAYA_SINGH",
    username: "maya.singh",
    firstName: "Maya",
    lastName: "Singh",
    displayName: "Maya Singh",
    email: "maya@tron.ai",
    phone: { countryCode: "+1", number: "5550101" },
    avatarUrl: null,
    locale: "en-US",
    timezone: "America/Los_Angeles",
    roleCodes: ["DEVELOPER"],
    spaceCodes: ["PLATFORM", "DATA"],
    status: "Active",
    passwordResetRequired: false,
    lastActiveAt: "2026-05-11T08:14:00Z",
    createdAt: "2026-05-11T08:40:00Z",
    updatedAt: "2026-05-11T08:40:00Z",
    archivedAt: null,
  },
  {
    id: "9007199254740993",
    userCode: "USER_CHEN_WEI",
    username: "chen.wei",
    firstName: "Chen",
    lastName: "Wei",
    displayName: "Chen Wei",
    email: "chen@tron.ai",
    phone: null,
    avatarUrl: null,
    locale: "en-US",
    timezone: "Asia/Shanghai",
    roleCodes: ["VIEWER"],
    spaceCodes: ["ENTERPRISE", "DATA"],
    status: "Active",
    passwordResetRequired: false,
    lastActiveAt: null,
    createdAt: "2026-05-10T03:20:00Z",
    updatedAt: "2026-05-10T03:20:00Z",
    archivedAt: null,
  },
  {
    id: "734103112812822528",
    userCode: "USER_RINA_PATEL",
    username: "rina.patel",
    firstName: "Rina",
    lastName: "Patel",
    displayName: "Rina Patel",
    email: "rina@tron.ai",
    phone: { countryCode: "+1", number: "5550102" },
    avatarUrl: null,
    locale: "en-US",
    timezone: "America/Los_Angeles",
    roleCodes: ["WORKSPACE_ADMIN", "VIEWER"],
    spaceCodes: ["SUPPORT", "PLATFORM"],
    status: "Archived",
    passwordResetRequired: false,
    lastActiveAt: "2026-04-28T17:15:00Z",
    createdAt: "2026-04-12T10:00:00Z",
    updatedAt: "2026-05-08T19:00:00Z",
    archivedAt: "2026-05-08T19:00:00Z",
  },
];

const workspaceSwitcher = {
  primarySpaces: [{
    workspaceCode: "ENTERPRISE",
    name: "Enterprise",
    description: "Global governance workspace",
    childCount: 1,
    status: "Active",
  }],
  secondarySpaces: [{
    workspaceCode: "PLATFORM",
    primaryWorkspaceCode: "ENTERPRISE",
    label: "Enterprise / Platform",
    name: "Platform",
    description: "Core platform runtime workspace",
    depth: 2,
    childCount: 0,
    effectiveUserCount: 2,
    status: "Active",
  }],
};
const auditLogs = [
  {
    id: "log-1",
    timestamp: "2026-05-14 08:30:00",
    actor: "alex@tron.ai",
    action: "Role matrix viewed",
    resourceId: "role.admin",
    status: "Success",
    ipAddress: "10.1.1.20",
  },
];

type CapturedRequest = {
  method: string;
  pathname: string;
  searchParams: Record<string, string>;
};

function delay(ms: number) {
  return new Promise((resolveDelay) => {
    setTimeout(resolveDelay, ms);
  });
}

function appendFrontendLog(chunk: Buffer) {
  frontendLogs = `${frontendLogs}${chunk.toString()}`.slice(-5000);
}

function successEnvelope(data: unknown) {
  return {
    code: "SUCCESS",
    message: "ok",
    data,
  };
}

async function routeAuthenticatedSession(page: Page) {
  await page.route("**/api/v1/auth/session", async (route) => {
    await fulfillJson(route, successEnvelope(authenticatedSession));
  });

  await page.route("**/api/v1/auth/refresh", async (route) => {
    await fulfillJson(route, successEnvelope(authenticatedGrant));
  });

  await page.route("**/api/v1/auth/logout", async (route) => {
    await fulfillJson(route, successEnvelope({ loggedOut: true }));
  });
}

function pagedEnvelope(records: unknown[], current = 1, size = records.length, total = records.length) {
  return successEnvelope({
    current,
    size,
    total,
    records,
  });
}

async function fulfillJson(route: Route, data: unknown, status = 200) {
  await route.fulfill({
    status,
    contentType: "application/json",
    body: JSON.stringify(data),
  });
}

function sharedToastMessage(page: Page, message: string | RegExp) {
  return page.locator('[aria-label="Notifications"]').getByRole("status").filter({ hasText: message }).first();
}

function captureRequest(route: Route): CapturedRequest {
  const request = route.request();
  const url = new URL(request.url());

  return {
    method: request.method(),
    pathname: url.pathname,
    searchParams: Object.fromEntries(url.searchParams.entries()),
  };
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
        CI: "1",
        VITE_TRON_ADMIN_DATA_SOURCE: "api",
        VITE_TRON_DATA_SOURCE: "api",
      },
      stdio: "pipe",
    },
  );

  frontendProcess.stdout.on("data", appendFrontendLog);
  frontendProcess.stderr.on("data", appendFrontendLog);

  await waitForFrontend(`${frontendBaseUrl}/admin/roles`);
});

test.afterAll(async () => {
  try {
    await stopManagedProcess(frontendProcess);
  } finally {
    frontendProcess = undefined;
  }
});

test("role matrix interactions stay real across the simplified matrix workspace", async ({ page }) => {
  const requests: CapturedRequest[] = [];
  const roleReplacePayloads: ReplaceRolesPayload[] = [];
  let roleCatalog = createRoleCatalog();
  let nextCustomRoleId = 734300000000000100n;

  await routeAuthenticatedSession(page);

  function persistRoleCatalog(payload: ReplaceRolesPayload) {
    roleCatalog = payload.roles.map((role) => ({
      ...role,
      id: /^\d+$/.test(role.id) ? role.id : String(nextCustomRoleId++),
      permissions: Object.fromEntries(
        permissionDomainList.map((domain) => [domain, [...(role.permissions[domain] ?? [])]]),
      ),
      updatedAt: role.updatedAt || "2026-05-15T00:00:00Z",
    }));

    return roleCatalog;
  }

  await page.route("**/api/v1/admin/**", async (route) => {
    const captured = captureRequest(route);
    requests.push(captured);

    if (captured.pathname === "/api/v1/admin/permission-domains") {
      await fulfillJson(route, successEnvelope(permissionDomainList));
      return;
    }

    if (captured.pathname === "/api/v1/admin/roles") {
      if (captured.method === "PUT") {
        const payload = route.request().postDataJSON() as ReplaceRolesPayload;
        roleReplacePayloads.push(payload);
        await fulfillJson(route, successEnvelope(persistRoleCatalog(payload)));
        return;
      }

      await fulfillJson(route, successEnvelope(roleCatalog));
      return;
    }

    if (captured.pathname === "/api/v1/admin/users") {
      await fulfillJson(route, pagedEnvelope(users, 1, 200, users.length));
      return;
    }

    if (captured.pathname === "/api/v1/admin/users/statistics/total-users") {
      await fulfillJson(route, successEnvelope({ count: users.length }));
      return;
    }

    if (captured.pathname === "/api/v1/admin/users/statistics/active-users") {
      await fulfillJson(route, successEnvelope({ count: users.filter((user) => user.status === "Active").length }));
      return;
    }

    if (captured.pathname === "/api/v1/admin/users/statistics/role-assignments") {
      await fulfillJson(route, successEnvelope({ count: users.reduce((total, user) => total + user.roleCodes.length, 0) }));
      return;
    }

    if (captured.pathname === "/api/v1/admin/users/statistics/space-assignments") {
      await fulfillJson(route, successEnvelope({ count: users.reduce((total, user) => total + user.spaceCodes.length, 0) }));
      return;
    }

    if (captured.pathname === "/api/v1/admin/audit-logs") {
      await fulfillJson(route, successEnvelope(auditLogs));
      return;
    }

    if (captured.pathname === "/api/v1/admin/workspaces/switcher") {
      await fulfillJson(route, successEnvelope(workspaceSwitcher));
      return;
    }

    await fulfillJson(route, successEnvelope({}));
  });

  await page.goto(`${frontendBaseUrl}/admin/roles`);

  const pageHeader = page.locator("main > header").first();
  await expect(page.getByRole("heading", { name: "Roles & Permissions" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Platform Permissions" })).toHaveCount(0);
  await expect(page.getByText("Search roles, focus domains, and edit permission grants directly from the matrix.", { exact: true })).toHaveCount(0);
  await expect(page.getByText("Saved drafts persist through the backend role catalog.", { exact: true })).toHaveCount(0);
  await expect(page.locator("tbody").getByText("Admin", { exact: true }).first()).toBeVisible();
  await expect.poll(() => requests.some((request) => request.pathname === "/api/v1/admin/roles")).toBe(true);
  await expect.poll(() => requests.some((request) => request.pathname === "/api/v1/admin/permission-domains")).toBe(true);
  await expect.poll(() => requests.some((request) => request.pathname === "/api/v1/admin/users")).toBe(true);
  await expect(page.getByText("Role Matrix", { exact: true })).toHaveCount(0);
  await expect(page.getByText("INSPECTOR", { exact: true })).toHaveCount(0);
  await expect(page.getByRole("tab", { name: "Users" })).toHaveCount(0);
  await expect(page.getByRole("tab", { name: "Audit Logs" })).toHaveCount(0);
  await expect(page.getByText("System Roles", { exact: true })).toHaveCount(0);
  await expect(page.getByText("Custom Roles", { exact: true })).toHaveCount(0);
  await expect(page.getByText("Changed Roles", { exact: true })).toHaveCount(0);
  await expect(page.getByText("Permission Focus", { exact: true })).toHaveCount(0);
  await expect(page.getByText("User Management Impact", { exact: true })).toHaveCount(0);
  await expect(page.getByRole("textbox", { name: "Role name" })).toHaveCount(0);
  await expect(page.getByRole("textbox", { name: "Role code" })).toHaveCount(0);
  await expect(page.getByRole("textbox", { name: "Role description" })).toHaveCount(0);
  await expect(page.getByText(/\/4 permissions$/)).toHaveCount(0);
  await expect(pageHeader.getByRole("button", { name: "Add Role", exact: true })).toBeVisible();
  await expect(pageHeader.getByRole("button", { name: "Clone", exact: true })).toBeVisible();
  await expect(pageHeader.getByRole("button", { name: "Reset", exact: true })).toBeVisible();
  await expect(pageHeader.getByRole("button", { name: "Discard", exact: true })).toBeVisible();
  await expect(pageHeader.getByRole("button", { name: "Save", exact: true })).toBeVisible();
  await expect(pageHeader.getByRole("button", { name: "Delete", exact: true })).toBeVisible();
  await expect(pageHeader.getByRole("button", { name: "Delete", exact: true })).toHaveCSS("color", "rgb(147, 0, 10)");

  const searchInput = page.getByRole("searchbox", { name: "Search roles" });
  const matrixBody = page.locator("tbody");
  await searchInput.fill("workspace");
  await expect.poll(async () => await matrixBody.getByText("Workspace Admin", { exact: true }).count()).toBe(1);
  await expect.poll(async () => await matrixBody.getByText("Viewer", { exact: true }).count()).toBe(0);
  await searchInput.fill("");
  await expect.poll(async () => await matrixBody.getByText("Viewer", { exact: true }).count() > 0).toBe(true);

  const domainFilter = page.getByRole("combobox", { name: "Filter by permission domain" });
  await domainFilter.selectOption("Skills");
  await expect(page.getByRole("columnheader", { name: "Skills" })).toBeVisible();
  await expect(page.getByRole("columnheader", { name: "Models" })).toHaveCount(0);
  await domainFilter.selectOption("All domains");

  await pageHeader.getByRole("button", { name: "Add Role", exact: true }).dblclick();
  await expect(sharedToastMessage(page, "Draft role created")).toBeVisible();
  const newRoleNameInput = page.getByRole("textbox", { name: /Edit role name for CUSTOM_ROLE_01/ });
  await expect(newRoleNameInput).toHaveValue("New Custom Role 01");
  await expect(matrixBody.getByText("New Custom Role 02", { exact: true })).toHaveCount(0);

  const newRoleSkillsApprove = page.getByRole("button", { name: "Toggle A for New Custom Role 01 on Skills" });
  await newRoleSkillsApprove.dblclick();
  await expect(newRoleSkillsApprove).toHaveAttribute("aria-pressed", "true");

  const roleTypeFilter = page.getByRole("combobox", { name: "Filter by role type" });
  await roleTypeFilter.selectOption("Custom");
  await expect(newRoleNameInput).toHaveValue("New Custom Role 01");
  await expect(matrixBody.getByText("Admin", { exact: true })).toHaveCount(0);
  await roleTypeFilter.selectOption("All");

  await pageHeader.getByRole("button", { name: "Save", exact: true }).click();
  await expect(sharedToastMessage(page, "Role matrix draft saved")).toBeVisible();
  await expect.poll(() => requests.some((request) => request.pathname === "/api/v1/admin/roles" && request.method === "PUT")).toBe(true);
  expect(roleReplacePayloads.at(-1)?.roles.some((role) => role.code === "CUSTOM_ROLE_01" && role.name === "New Custom Role 01" && role.permissions.Skills?.includes("A"))).toBe(true);
  await expect.poll(() => roleCatalog.some((role) => role.name === "New Custom Role 01")).toBe(true);
  await expect.poll(async () => page.evaluate((storageKey) => window.localStorage.getItem(storageKey), STORAGE_KEY)).toBeNull();

  await page.reload();
  await expect(page.getByRole("textbox", { name: /Edit role name for CUSTOM_ROLE_01/ })).toHaveValue("New Custom Role 01");
  const savedRoleModelsWrite = page.getByRole("button", { name: "Toggle W for New Custom Role 01 on Models" });
  await savedRoleModelsWrite.click();
  await expect(savedRoleModelsWrite).toHaveAttribute("aria-pressed", "true");
  await pageHeader.getByRole("button", { name: "Reset", exact: true }).click();
  await expect(sharedToastMessage(page, /Reset New Custom Role 01/)).toBeVisible();
  await expect(savedRoleModelsWrite).toHaveAttribute("aria-pressed", "false");

  await pageHeader.getByRole("button", { name: "Clone", exact: true }).click();
  await expect(sharedToastMessage(page, /Cloned New Custom Role 01/)).toBeVisible();
  const clonedRoleNameInput = page.getByRole("textbox", { name: /Edit role name for CUSTOM_ROLE_01_COPY/ });
  await expect(clonedRoleNameInput).toHaveValue("New Custom Role 01 Copy");
  await clonedRoleNameInput.fill("Release Manager Copy");
  await expect(clonedRoleNameInput).toHaveValue("Release Manager Copy");

  await pageHeader.getByRole("button", { name: "Delete", exact: true }).click();
  await expect(pageHeader.getByRole("button", { name: "Confirm", exact: true })).toBeVisible();
  await pageHeader.getByRole("button", { name: "Confirm", exact: true }).click();
  await expect(sharedToastMessage(page, /Release Manager Copy deleted/)).toBeVisible();
  await expect(clonedRoleNameInput).toHaveCount(0);

  await newRoleSkillsApprove.click();
  await expect(newRoleSkillsApprove).toHaveAttribute("aria-pressed", "false");
  await page.getByRole("button", { name: "Show changed only" }).click();
  await expect(page.getByRole("textbox", { name: /Edit role name for CUSTOM_ROLE_01/ })).toHaveValue("New Custom Role 01");
  await expect(matrixBody.getByText("Admin", { exact: true })).toHaveCount(0);

  await pageHeader.getByRole("button", { name: "Discard", exact: true }).click();
  await expect(sharedToastMessage(page, "Unsaved changes discarded")).toBeVisible();
  await expect(page.getByText("No roles match the current filters.")).toBeVisible();
  await page.getByRole("button", { name: "Clear Filters" }).click();
  await expect(matrixBody.getByText("Admin", { exact: true }).first()).toBeVisible();

  const savedDraft = await page.evaluate((storageKey) => window.localStorage.getItem(storageKey), STORAGE_KEY);
  expect(savedDraft).toBeNull();
  expect(roleCatalog.some((role) => role.name === "New Custom Role 01")).toBe(true);
  expect(roleCatalog.some((role) => role.name === "Release Manager Copy")).toBe(false);
});
