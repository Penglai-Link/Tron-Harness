import { expect, test, type Page, type Route } from "@playwright/test";
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

type CapturedRequest = { body: unknown; method: string; pathname: string; searchParams: Record<string, string> };
type VariableSourceKind = "Inherited" | "Local" | "Override";
type LocalVariable = {
  description: string | null;
  key: string;
  updatedAt: string;
  value: string;
  workspaceCode: string;
};

const now = "2026-05-18T12:00:00Z";

const authenticatedSession = {
  authMethod: "internal",
  authenticated: true,
  provider: null,
  user: {
    displayName: "Auth Operator",
    email: "auth.operator@tron.ai",
    roleCodes: ["ADMIN", "WORKSPACE_ADMIN"],
    spaceCodes: ["TRON_CORE", "PLATFORM", "COMPLIANCE"],
    status: "Active",
    userCode: "USR-AUTH-0001",
    username: "auth.operator",
  },
};

const workspaceSwitcher = {
  primarySpaces: [
    {
      workspaceCode: "ENTERPRISE",
      name: "Enterprise",
      description: "Global governance workspace",
      childCount: 1,
      status: "Active",
    },
    {
      workspaceCode: "COMPLIANCE",
      name: "Compliance",
      description: "Compliance evidence workspace",
      childCount: 0,
      status: "Active",
    },
  ],
  secondarySpaces: [
    {
      workspaceCode: "ENTERPRISE",
      primaryWorkspaceCode: "ENTERPRISE",
      label: "Enterprise",
      name: "Enterprise",
      description: "Global governance workspace",
      depth: 1,
      childCount: 1,
      effectiveUserCount: 1,
      status: "Active",
    },
    {
      workspaceCode: "PLATFORM",
      primaryWorkspaceCode: "ENTERPRISE",
      label: "Enterprise / Platform",
      name: "Platform",
      description: "Platform runtime workspace",
      depth: 2,
      childCount: 1,
      effectiveUserCount: 1,
      status: "Active",
    },
    {
      workspaceCode: "TRON_CORE",
      primaryWorkspaceCode: "ENTERPRISE",
      label: "Enterprise / Platform / Tron Core",
      name: "Tron Core",
      description: "Main Tron console workspace",
      depth: 3,
      childCount: 0,
      effectiveUserCount: 1,
      status: "Active",
    },
    {
      workspaceCode: "COMPLIANCE",
      primaryWorkspaceCode: "COMPLIANCE",
      label: "Compliance",
      name: "Compliance",
      description: "Compliance evidence workspace",
      depth: 1,
      childCount: 0,
      effectiveUserCount: 1,
      status: "Active",
    },
  ],
} as const;

const workspaceLineages = {
  ENTERPRISE: [
    { workspaceCode: "ENTERPRISE", name: "Enterprise", label: "Enterprise" },
  ],
  PLATFORM: [
    { workspaceCode: "ENTERPRISE", name: "Enterprise", label: "Enterprise" },
    { workspaceCode: "PLATFORM", name: "Platform", label: "Enterprise / Platform" },
  ],
  TRON_CORE: [
    { workspaceCode: "ENTERPRISE", name: "Enterprise", label: "Enterprise" },
    { workspaceCode: "PLATFORM", name: "Platform", label: "Enterprise / Platform" },
    { workspaceCode: "TRON_CORE", name: "Tron Core", label: "Enterprise / Platform / Tron Core" },
  ],
  COMPLIANCE: [
    { workspaceCode: "COMPLIANCE", name: "Compliance", label: "Compliance" },
  ],
} as const;

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
      version: 4,
    }));
  }, workspaceSwitcher);
}

function successEnvelope(data: unknown) {
  return { code: "SUCCESS", message: "ok", data };
}

async function fulfillJson(route: Route, data: unknown, status = 200) {
  await route.fulfill({
    status,
    contentType: "application/json",
    body: JSON.stringify(data),
  });
}

function captureRequest(route: Route): CapturedRequest {
  const request = route.request();
  const url = new URL(request.url());
  return {
    body: request.postDataJSON() ?? null,
    method: request.method(),
    pathname: url.pathname,
    searchParams: Object.fromEntries(url.searchParams.entries()),
  };
}

function effectiveVariables(variables: LocalVariable[], workspaceCode: keyof typeof workspaceLineages) {
  const lineage = workspaceLineages[workspaceCode];
  const currentWorkspace = lineage.at(-1);
  if (!currentWorkspace) return [];

  const resolved = new Map<string, {
    description: string | null;
    key: string;
    shadowedLabel: string | null;
    shadowedWorkspaceCode: string | null;
    shadowedWorkspaceName: string | null;
    sourceKind: VariableSourceKind;
    sourceLabel: string;
    sourceWorkspaceCode: string;
    sourceWorkspaceName: string;
    updatedAt: string;
    value: string;
  }>();

  for (const workspace of lineage) {
    const workspaceVariables = variables
      .filter((variable) => variable.workspaceCode === workspace.workspaceCode)
      .sort((left, right) => left.key.localeCompare(right.key));
    for (const variable of workspaceVariables) {
      const previous = resolved.get(variable.key);
      const sourceKind: VariableSourceKind = previous
        ? (workspace.workspaceCode === currentWorkspace.workspaceCode ? "Override" : "Inherited")
        : (workspace.workspaceCode === currentWorkspace.workspaceCode ? "Local" : "Inherited");
      resolved.set(variable.key, {
        key: variable.key,
        value: variable.value,
        description: variable.description,
        sourceKind,
        sourceWorkspaceCode: workspace.workspaceCode,
        sourceWorkspaceName: workspace.name,
        sourceLabel: workspace.label,
        shadowedWorkspaceCode: sourceKind === "Override" ? previous?.sourceWorkspaceCode ?? null : null,
        shadowedWorkspaceName: sourceKind === "Override" ? previous?.sourceWorkspaceName ?? null : null,
        shadowedLabel: sourceKind === "Override" ? previous?.sourceLabel ?? null : null,
        updatedAt: variable.updatedAt,
      });
    }
  }

  return [...resolved.values()].sort((left, right) => left.key.localeCompare(right.key));
}

async function routeApi(page: Page, requests: CapturedRequest[], variables: LocalVariable[]) {
  await page.route("**/api/v1/**", async (route) => {
    const captured = captureRequest(route);
    requests.push(captured);

    if (captured.pathname === "/api/v1/auth/providers") {
      return fulfillJson(route, successEnvelope({ providers: [{ code: "internal", name: "Internal Account", type: "internal" }] }));
    }
    if (captured.pathname === "/api/v1/auth/refresh") {
      return fulfillJson(route, successEnvelope({ accessToken: "variable-token", tokenType: "Bearer", expiresIn: 900, session: authenticatedSession }));
    }
    if (captured.pathname === "/api/v1/auth/session") {
      return fulfillJson(route, successEnvelope(authenticatedSession));
    }
    if (captured.pathname === "/api/v1/auth/logout") {
      return fulfillJson(route, successEnvelope({ loggedOut: true }));
    }
    if (captured.pathname === "/api/v1/admin/workspaces/switcher") {
      return fulfillJson(route, successEnvelope(workspaceSwitcher));
    }

    const variableWorkspaceMatch = captured.pathname.match(/^\/api\/v1\/variables\/workspaces\/([^/]+)$/);
    if (variableWorkspaceMatch) {
      const workspaceCode = decodeURIComponent(variableWorkspaceMatch[1]) as keyof typeof workspaceLineages;
      if (captured.method === "GET") {
        return fulfillJson(route, successEnvelope(effectiveVariables(variables, workspaceCode)));
      }

      const body = captured.body as Partial<LocalVariable>;
      const key = body.key?.trim();
      if (!key) {
        return fulfillJson(route, { code: "VARIABLE_KEY_INVALID", message: "variable key must use letters, digits, underscore, hyphen, or dot", data: {} }, 400);
      }
      if (variables.some((variable) => variable.workspaceCode === workspaceCode && variable.key === key)) {
        return fulfillJson(route, { code: "VARIABLE_KEY_DUPLICATE", message: "variable key already exists in the current workspace", data: {} }, 409);
      }

      variables.push({
        workspaceCode,
        key,
        value: body.value ?? "",
        description: typeof body.description === "string" ? body.description : null,
        updatedAt: now,
      });
      const record = effectiveVariables(variables, workspaceCode).find((variable) => variable.key === key);
      return fulfillJson(route, successEnvelope(record), 201);
    }

    const variableItemMatch = captured.pathname.match(/^\/api\/v1\/variables\/workspaces\/([^/]+)\/([^/]+)$/);
    if (variableItemMatch) {
      const workspaceCode = decodeURIComponent(variableItemMatch[1]) as keyof typeof workspaceLineages;
      const variableKey = decodeURIComponent(variableItemMatch[2]);
      const localIndex = variables.findIndex((variable) => variable.workspaceCode === workspaceCode && variable.key === variableKey);
      const effective = effectiveVariables(variables, workspaceCode).find((variable) => variable.key === variableKey);

      if (captured.method === "PATCH") {
        if (localIndex < 0) {
          if (effective) {
            return fulfillJson(route, { code: "VARIABLE_INHERITED_MUTATION_INVALID", message: "override the inherited variable locally before modifying it", data: {} }, 400);
          }
          return fulfillJson(route, { code: "VARIABLE_NOT_FOUND", message: "variable was not found", data: {} }, 404);
        }

        const body = captured.body as Partial<LocalVariable>;
        variables[localIndex] = {
          ...variables[localIndex],
          value: body.value ?? variables[localIndex].value,
          description: typeof body.description === "string" ? body.description : null,
          updatedAt: now,
        };
        const record = effectiveVariables(variables, workspaceCode).find((variable) => variable.key === variableKey);
        return fulfillJson(route, successEnvelope(record));
      }

      if (captured.method === "DELETE") {
        if (localIndex < 0) {
          if (effective) {
            return fulfillJson(route, { code: "VARIABLE_INHERITED_MUTATION_INVALID", message: "override the inherited variable locally before modifying it", data: {} }, 400);
          }
          return fulfillJson(route, { code: "VARIABLE_NOT_FOUND", message: "variable was not found", data: {} }, 404);
        }

        variables.splice(localIndex, 1);
        return fulfillJson(route, successEnvelope({ key: variableKey, workspaceCode }));
      }
    }

    return fulfillJson(route, { code: "NOT_FOUND", message: `Unhandled ${captured.method} ${captured.pathname}`, data: {} }, 404);
  });
}

function toast(page: Page, message: string | RegExp) {
  return page.locator('[aria-label="Notifications"]').getByRole("status").filter({ hasText: message }).first();
}

async function openDesktopRightSidebar(page: Page) {
  const reopen = page.getByTestId("tron-right-sidebar-reopen");
  if (await reopen.count()) {
    await reopen.click();
  }
  await expect(page.getByTestId("tron-right-sidebar")).toBeVisible();
}

function appendFrontendLog(chunk: Buffer) {
  frontendLogs = `${frontendLogs}${chunk.toString()}`.slice(-5000);
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
  frontendProcess = spawn("npm", ["run", "dev", "--", "--host", "127.0.0.1", "--port", String(port)], {
    cwd: frontendRoot,
    env: {
      ...process.env,
      CI: "1",
      VITE_TRON_ADMIN_DATA_SOURCE: "api",
      VITE_TRON_DATA_SOURCE: "api",
      VITE_TRON_VARIABLE_DATA_SOURCE: "api",
    },
    stdio: "pipe",
  });
  frontendProcess.stdout.on("data", appendFrontendLog);
  frontendProcess.stderr.on("data", appendFrontendLog);
  await waitForFrontend(`${frontendBaseUrl}/variables`);
});

test.afterAll(async () => {
  if (!frontendProcess || frontendProcess.killed) return;
  frontendProcess.kill("SIGTERM");
  await new Promise((resolveProcess) => frontendProcess?.once("exit", resolveProcess));
});

test("variable management supports inheritance override revert local CRUD and workspace isolation", async ({ page }) => {
  const requests: CapturedRequest[] = [];
  const variables: LocalVariable[] = [
    {
      workspaceCode: "ENTERPRISE",
      key: "DEFAULT_REGION",
      value: "us-east-1",
      description: "Shared default deployment region.",
      updatedAt: now,
    },
    {
      workspaceCode: "ENTERPRISE",
      key: "ROUTER_TIMEOUT_MS",
      value: "800",
      description: "Root timeout for interactive routing.",
      updatedAt: now,
    },
    {
      workspaceCode: "PLATFORM",
      key: "OPENAI_API_BASE_URL",
      value: "https://api.openai.com/v1",
      description: "Shared OpenAI gateway base URL.",
      updatedAt: now,
    },
    {
      workspaceCode: "PLATFORM",
      key: "ROUTER_TIMEOUT_MS",
      value: "600",
      description: "Platform override for interactive latency.",
      updatedAt: now,
    },
    {
      workspaceCode: "TRON_CORE",
      key: "OPENAI_API_KEY",
      value: "sk-tron-core-demo",
      description: "Workspace secret used by the demo workspace.",
      updatedAt: now,
    },
    {
      workspaceCode: "TRON_CORE",
      key: "ROUTER_TIMEOUT_MS",
      value: "400",
      description: "Tron Core override for agent console flows.",
      updatedAt: now,
    },
    {
      workspaceCode: "COMPLIANCE",
      key: "AUDIT_RETENTION_DAYS",
      value: "365",
      description: "Retention period for compliance exports.",
      updatedAt: now,
    },
  ];
  await routeApi(page, requests, variables);
  await seedTronCoreShellSelection(page);

  await page.goto(`${frontendBaseUrl}/variables`);

  await expect(page.getByRole("heading", { name: "Variable Management" })).toBeVisible();
  await expect.poll(() => requests.some((request) => request.pathname === "/api/v1/variables/workspaces/TRON_CORE" && request.method === "GET")).toBe(true);
  await expect(page.getByRole("button", { name: "DEFAULT_REGION", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "OPENAI_API_KEY", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "ROUTER_TIMEOUT_MS", exact: true })).toBeVisible();

  await page.getByRole("button", { name: "DEFAULT_REGION", exact: true }).click();
  await openDesktopRightSidebar(page);
  const variableInspector = page.getByTestId("tron-right-sidebar");
  await expect(variableInspector.getByText("Inherited From", { exact: true })).toBeVisible();
  await expect(variableInspector.getByRole("button", { name: "Override", exact: true })).toBeVisible();

  await variableInspector.getByRole("button", { name: "Override", exact: true }).click();
  await page.getByLabel("Variable value").fill("ap-southeast-1");
  await page.getByLabel("Variable description").fill("Regional override for APAC");
  await variableInspector.getByRole("button", { name: "Save", exact: true }).click();
  await expect(toast(page, "DEFAULT_REGION overridden in TRON_CORE")).toBeVisible();
  await expect.poll(() => requests.some((request) => (
    request.method === "POST"
    && request.pathname === "/api/v1/variables/workspaces/TRON_CORE"
    && (request.body as { key?: string; value?: string } | null)?.key === "DEFAULT_REGION"
    && (request.body as { value?: string } | null)?.value === "ap-southeast-1"
  ))).toBe(true);
  await expect(page.getByText("Shadowed Ancestor", { exact: true })).toBeVisible();

  await page.getByRole("button", { name: "Revert", exact: true }).click();
  await expect(toast(page, "DEFAULT_REGION reverted to inherited value")).toBeVisible();
  await expect.poll(() => requests.some((request) => (
    request.method === "DELETE"
    && request.pathname === "/api/v1/variables/workspaces/TRON_CORE/DEFAULT_REGION"
  ))).toBe(true);
  await expect(variableInspector.getByText("Inherited From", { exact: true })).toBeVisible();

  await page.getByRole("button", { name: "New", exact: true }).first().click();
  await page.getByLabel("Variable key").fill("MAX_CONTEXT_WINDOW");
  await page.getByLabel("Variable value").fill("128000");
  await page.getByLabel("Variable description").fill("Local window size");
  await variableInspector.getByRole("button", { name: "Save", exact: true }).click();
  await expect(toast(page, "MAX_CONTEXT_WINDOW created")).toBeVisible();
  await expect.poll(() => requests.some((request) => (
    request.method === "POST"
    && request.pathname === "/api/v1/variables/workspaces/TRON_CORE"
    && (request.body as { key?: string } | null)?.key === "MAX_CONTEXT_WINDOW"
  ))).toBe(true);

  await page.getByRole("button", { name: "MAX_CONTEXT_WINDOW", exact: true }).click();
  await page.getByLabel("Variable value").fill("256000");
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expect(toast(page, "MAX_CONTEXT_WINDOW saved")).toBeVisible();
  await expect.poll(() => requests.some((request) => (
    request.method === "PATCH"
    && request.pathname === "/api/v1/variables/workspaces/TRON_CORE/MAX_CONTEXT_WINDOW"
    && (request.body as { value?: string } | null)?.value === "256000"
  ))).toBe(true);

  await page.getByRole("button", { name: "Delete", exact: true }).click();
  await expect(toast(page, "MAX_CONTEXT_WINDOW deleted")).toBeVisible();
  await expect.poll(() => requests.some((request) => (
    request.method === "DELETE"
    && request.pathname === "/api/v1/variables/workspaces/TRON_CORE/MAX_CONTEXT_WINDOW"
  ))).toBe(true);
});
