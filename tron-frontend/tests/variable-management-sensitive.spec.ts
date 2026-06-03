import {
  expect,
  test,
  type Page,
  type Route,
  type TestInfo,
} from "@playwright/test";
import { readFileSync } from "node:fs";
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

const mask = "********";
const now = "2026-06-02T09:00:00Z";
const stepUpPassword = "StepUpPass-For-Test";

type CapturedRequest = {
  body: unknown;
  method: string;
  pathname: string;
  searchParams: Record<string, string>;
};
type VariableSourceKind = "Inherited" | "Local" | "Override";
type ValueMode = "PRESERVE" | "REPLACE";
type StepUpTarget =
  | {
    scope: "platform";
    variableKey: string;
  }
  | {
    scope: "workspace";
    variableKey: string;
    workspaceCode: string;
  };

interface SensitiveFields {
  hasValue: boolean;
  isSensitive: boolean;
  valueMasked: boolean;
}

interface WorkspaceVariableRecord extends SensitiveFields {
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
}

interface PlatformVariableRecord extends SensitiveFields {
  createdAt: string;
  description: string | null;
  id: string;
  key: string;
  updatedAt: string;
  value: string;
}

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

function revealedPlaceholder(scope: string, key: string) {
  return ["revealed", scope.toLowerCase(), key.toLowerCase(), "value"].join("-");
}

function hiddenSearchFragment(scope: string, key: string) {
  return ["revealed", scope.toLowerCase(), key.toLowerCase()].join("-");
}

function stepUpTargetsMatch(left: StepUpTarget | undefined, right: StepUpTarget | undefined) {
  if (!left || !right || left.scope !== right.scope || left.variableKey !== right.variableKey) return false;
  if (left.scope === "workspace" || right.scope === "workspace") {
    return left.scope === "workspace" && right.scope === "workspace" && left.workspaceCode === right.workspaceCode;
  }
  return true;
}

function sensitiveWorkspaceRecord(key = "WORKSPACE_API_CREDENTIAL"): WorkspaceVariableRecord {
  return {
    key,
    value: mask,
    description: "Workspace credential used by the runtime resolver.",
    sourceKind: "Local",
    sourceWorkspaceCode: "TRON_CORE",
    sourceWorkspaceName: "Tron Core",
    sourceLabel: "Enterprise / Platform / Tron Core",
    shadowedWorkspaceCode: null,
    shadowedWorkspaceName: null,
    shadowedLabel: null,
    updatedAt: now,
    isSensitive: true,
    valueMasked: true,
    hasValue: true,
  };
}

function workspaceRows(): WorkspaceVariableRecord[] {
  return [
    {
      key: "ROUTER_TIMEOUT_MS",
      value: "400",
      description: "Tron Core override for agent console flows.",
      sourceKind: "Local",
      sourceWorkspaceCode: "TRON_CORE",
      sourceWorkspaceName: "Tron Core",
      sourceLabel: "Enterprise / Platform / Tron Core",
      shadowedWorkspaceCode: null,
      shadowedWorkspaceName: null,
      shadowedLabel: null,
      updatedAt: now,
      isSensitive: false,
      valueMasked: false,
      hasValue: true,
    },
    sensitiveWorkspaceRecord(),
  ];
}

function platformRows(): PlatformVariableRecord[] {
  return [
    {
      id: "734500000000100001",
      key: "GLOBAL_LLM_TIMEOUT_MS",
      value: "1200",
      description: "Platform-wide default timeout for LLM requests.",
      createdAt: now,
      updatedAt: now,
      isSensitive: false,
      valueMasked: false,
      hasValue: true,
    },
    {
      id: "734500000000100002",
      key: "PLATFORM_PROVIDER_CREDENTIAL",
      value: mask,
      description: "Platform provider credential.",
      createdAt: now,
      updatedAt: now,
      isSensitive: true,
      valueMasked: true,
      hasValue: true,
    },
  ];
}

async function routeSensitiveApi(
  page: Page,
  requests: CapturedRequest[],
  options: {
    platformVariables?: PlatformVariableRecord[];
    workspaceVariables?: WorkspaceVariableRecord[];
  } = {},
) {
  const workspaceVariables = options.workspaceVariables ?? workspaceRows();
  const platformVariables = options.platformVariables ?? platformRows();
  const validStepUpTokens = new Map<string, StepUpTarget>();

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
    if (captured.pathname === "/api/v1/auth/verify-step-up-password" && captured.method === "POST") {
      const body = captured.body as { password?: string; target?: StepUpTarget } | null;
      if (body?.password !== stepUpPassword) {
        return fulfillJson(route, { code: "VARIABLE_STEP_UP_FAILED", message: "Step-up verification failed", data: {} }, 401);
      }
      if (!body.target?.variableKey || (body.target.scope === "workspace" && !body.target.workspaceCode)) {
        return fulfillJson(route, { code: "VARIABLE_STEP_UP_TARGET_INVALID", message: "Step-up target is required", data: {} }, 400);
      }
      const stepUpToken = `step-up-${validStepUpTokens.size + 1}`;
      validStepUpTokens.set(stepUpToken, body.target);
      return fulfillJson(route, successEnvelope({ stepUpToken, expiresAt: "2026-06-02T09:15:00Z" }));
    }

    const workspaceListMatch = captured.pathname.match(/^\/api\/v1\/variables\/workspaces\/([^/]+)$/);
    if (workspaceListMatch && captured.method === "GET") {
      return fulfillJson(route, successEnvelope(workspaceVariables));
    }
    const workspaceItemMatch = captured.pathname.match(/^\/api\/v1\/variables\/workspaces\/([^/]+)\/([^/]+)$/);
    if (workspaceItemMatch && captured.method === "PATCH") {
      const workspaceCode = decodeURIComponent(workspaceItemMatch[1]);
      const variableKey = decodeURIComponent(workspaceItemMatch[2]);
      const index = workspaceVariables.findIndex((record) => record.key === variableKey);
      if (index < 0) {
        return fulfillJson(route, { code: "VARIABLE_NOT_FOUND", message: "Variable was not found", data: {} }, 404);
      }
      const body = captured.body as Partial<WorkspaceVariableRecord> & { valueMode?: ValueMode } | null;
      const previous = workspaceVariables[index];
      workspaceVariables[index] = {
        ...previous,
        value: body?.isSensitive ? mask : body?.value ?? previous.value,
        description: typeof body?.description === "string" ? body.description : null,
        sourceWorkspaceCode: workspaceCode,
        updatedAt: now,
        isSensitive: Boolean(body?.isSensitive),
        valueMasked: Boolean(body?.isSensitive),
        hasValue: body?.valueMode === "PRESERVE" ? previous.hasValue : Boolean(body?.value ?? previous.hasValue),
      };
      return fulfillJson(route, successEnvelope(workspaceVariables[index]));
    }
    if (captured.pathname === "/api/v1/variables/reveal-workspace-variable" && captured.method === "POST") {
      const body = captured.body as { stepUpToken?: string; variableKey?: string; workspaceCode?: string } | null;
      const expectedTarget = body?.stepUpToken ? validStepUpTokens.get(body.stepUpToken) : undefined;
      if (!body?.stepUpToken || !stepUpTargetsMatch(expectedTarget, {
        scope: "workspace",
        variableKey: body.variableKey ?? "",
        workspaceCode: body.workspaceCode ?? "",
      })) {
        return fulfillJson(route, { code: "VARIABLE_STEP_UP_TOKEN_MISMATCH", message: "Step-up token mismatch", data: {} }, 403);
      }
      return fulfillJson(route, successEnvelope({ value: revealedPlaceholder(body.workspaceCode ?? "workspace", body.variableKey ?? "variable") }));
    }

    if (captured.pathname === "/api/v1/admin/platform/variables") {
      if (captured.method === "GET") {
        return fulfillJson(route, successEnvelope(platformVariables.sort((left, right) => left.key.localeCompare(right.key))));
      }
      if (captured.method === "POST") {
        const body = captured.body as Partial<PlatformVariableRecord> & { valueMode?: ValueMode } | null;
        const key = body?.key?.trim();
        if (!key) {
          return fulfillJson(route, { code: "VARIABLE_KEY_INVALID", message: "Variable key is required", data: {} }, 400);
        }
        const nextRecord: PlatformVariableRecord = {
          id: String(734500000000100000n + BigInt(platformVariables.length + 1)),
          key,
          value: body.isSensitive ? mask : body.value ?? "",
          description: typeof body.description === "string" ? body.description : null,
          createdAt: now,
          updatedAt: now,
          isSensitive: Boolean(body.isSensitive),
          valueMasked: Boolean(body.isSensitive),
          hasValue: Boolean(body.value),
        };
        platformVariables.push(nextRecord);
        return fulfillJson(route, successEnvelope(nextRecord), 201);
      }
    }

    const platformItemMatch = captured.pathname.match(/^\/api\/v1\/admin\/platform\/variables\/([^/]+)$/);
    if (platformItemMatch && captured.method === "PATCH") {
      const variableKey = decodeURIComponent(platformItemMatch[1]);
      const index = platformVariables.findIndex((record) => record.key === variableKey);
      if (index < 0) {
        return fulfillJson(route, { code: "VARIABLE_NOT_FOUND", message: "Variable was not found", data: {} }, 404);
      }
      const body = captured.body as Partial<PlatformVariableRecord> & { valueMode?: ValueMode } | null;
      const previous = platformVariables[index];
      platformVariables[index] = {
        ...previous,
        value: body?.isSensitive ? mask : body?.value ?? previous.value,
        description: typeof body?.description === "string" ? body.description : null,
        updatedAt: now,
        isSensitive: Boolean(body?.isSensitive),
        valueMasked: Boolean(body?.isSensitive),
        hasValue: body?.valueMode === "PRESERVE" ? previous.hasValue : Boolean(body?.value ?? previous.hasValue),
      };
      return fulfillJson(route, successEnvelope(platformVariables[index]));
    }
    if (platformItemMatch && captured.method === "DELETE") {
      const variableKey = decodeURIComponent(platformItemMatch[1]);
      const index = platformVariables.findIndex((record) => record.key === variableKey);
      if (index >= 0) platformVariables.splice(index, 1);
      return fulfillJson(route, successEnvelope({ key: variableKey, workspaceCode: "PLATFORM" }));
    }
    if (captured.pathname === "/api/v1/admin/platform/variables/reveal-platform-variable" && captured.method === "POST") {
      const body = captured.body as { stepUpToken?: string; variableKey?: string } | null;
      const expectedTarget = body?.stepUpToken ? validStepUpTokens.get(body.stepUpToken) : undefined;
      if (!body?.stepUpToken || !stepUpTargetsMatch(expectedTarget, {
        scope: "platform",
        variableKey: body.variableKey ?? "",
      })) {
        return fulfillJson(route, { code: "VARIABLE_STEP_UP_TOKEN_MISMATCH", message: "Step-up token mismatch", data: {} }, 403);
      }
      return fulfillJson(route, successEnvelope({ value: revealedPlaceholder("platform", body.variableKey ?? "variable") }));
    }

    return fulfillJson(route, { code: "NOT_FOUND", message: `Unhandled ${captured.method} ${captured.pathname}`, data: {} }, 404);
  });
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

function requestBodyFor<TBody>(requests: CapturedRequest[], predicate: (request: CapturedRequest) => boolean) {
  return requests.find(predicate)?.body as TBody | undefined;
}

function platformVariablesCreatedAsSensitive(requests: CapturedRequest[], variableKey: string) {
  const body = requestBodyFor<{ isSensitive?: boolean; key?: string; valueMode?: string }>(
    requests,
    (request) => request.method === "POST"
      && request.pathname === "/api/v1/admin/platform/variables"
      && (request.body as { key?: string } | null)?.key === variableKey,
  );
  return Boolean(body?.isSensitive && body.valueMode === "REPLACE");
}

async function expectVisibleValueMatches(locator: ReturnType<Page["getByTestId"]>, expectedValue: string) {
  const matches = await locator.evaluate((node, expected) => node.textContent?.includes(expected) ?? false, expectedValue);
  expect(matches, "revealed value should be present in the visible sensitive value surface").toBe(true);
}

async function captureSafeScreenshot(page: Page, testInfo: TestInfo, name: string) {
  await page.screenshot({ path: testInfo.outputPath(name), fullPage: true });
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
      VITE_TRON_AUTH_DATA_SOURCE: "api",
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

test("AC-VSD-07 workspace variables mask reveal hide search and remask", async ({ page }, testInfo) => {
  const requests: CapturedRequest[] = [];
  await routeSensitiveApi(page, requests);

  await page.goto(`${frontendBaseUrl}/variables`);
  await expect(page.getByRole("heading", { name: "Variable Management" })).toBeVisible();
  await expect(page.getByText(mask).first()).toBeVisible();
  await expect(page.getByText(revealedPlaceholder("TRON_CORE", "WORKSPACE_API_CREDENTIAL"))).toHaveCount(0);

  await page.getByPlaceholder("Filter effective variables...").fill(hiddenSearchFragment("TRON_CORE", "WORKSPACE_API_CREDENTIAL"));
  await expect(page.getByText("No variables match the current filters.")).toBeVisible();
  await page.getByPlaceholder("Filter effective variables...").fill("");

  await page.getByRole("button", { name: "WORKSPACE_API_CREDENTIAL", exact: true }).click();
  await page.getByRole("button", { name: "Reveal WORKSPACE_API_CREDENTIAL" }).click();
  await page.getByLabel("Step-up password").fill(stepUpPassword);
  await page.getByRole("button", { name: "Reveal Value" }).click();

  await expect(page.getByTestId("variable-sensitive-value-WORKSPACE_API_CREDENTIAL")).toHaveAttribute("data-sensitive-state", "revealed");
  expect(requestBodyFor<{ password?: string; target?: StepUpTarget }>(
    requests,
    (request) => request.pathname === "/api/v1/auth/verify-step-up-password",
  )).toMatchObject({
    password: stepUpPassword,
    target: {
      scope: "workspace",
      workspaceCode: "ENTERPRISE",
      variableKey: "WORKSPACE_API_CREDENTIAL",
    },
  });
  const workspaceRevealBody = requestBodyFor<{ stepUpToken?: string; variableKey?: string; workspaceCode?: string }>(
    requests,
    (request) => request.pathname === "/api/v1/variables/reveal-workspace-variable",
  );
  expect(workspaceRevealBody).toMatchObject({
    variableKey: "WORKSPACE_API_CREDENTIAL",
    workspaceCode: "ENTERPRISE",
  });
  await expectVisibleValueMatches(
    page.getByTestId("variable-sensitive-value-WORKSPACE_API_CREDENTIAL"),
    revealedPlaceholder(workspaceRevealBody?.workspaceCode ?? "workspace", "WORKSPACE_API_CREDENTIAL"),
  );

  await page.getByRole("button", { name: "Hide WORKSPACE_API_CREDENTIAL" }).click();
  await expect(page.getByTestId("variable-sensitive-value-WORKSPACE_API_CREDENTIAL")).toHaveAttribute("data-sensitive-state", "masked");
  await expect(page.getByText(revealedPlaceholder(workspaceRevealBody?.workspaceCode ?? "workspace", "WORKSPACE_API_CREDENTIAL"))).toHaveCount(0);
  await captureSafeScreenshot(page, testInfo, "workspace-hidden.png");

  await page.getByRole("button", { name: "ROUTER_TIMEOUT_MS", exact: true }).click();
  const workspacePanel = page.getByTestId("tron-right-sidebar");
  await expect(workspacePanel.getByRole("heading", { name: "ROUTER_TIMEOUT_MS" })).toBeVisible();
  await expect(workspacePanel.getByLabel("Variable key")).toHaveValue("ROUTER_TIMEOUT_MS");
  await workspacePanel.getByText("Sensitive value", { exact: true }).click();
  await expect(workspacePanel.getByRole("button", { name: "Replace sensitive value" })).toHaveAttribute("aria-pressed", "true");
  await workspacePanel.getByRole("button", { name: "Save" }).click();
  await expect(page.getByText("Sensitive value is required before turning on sensitivity")).toBeVisible();
  expect(requests.some((request) => request.method === "PATCH" && request.pathname === "/api/v1/variables/workspaces/ENTERPRISE/ROUTER_TIMEOUT_MS")).toBe(false);
  await workspacePanel.getByLabel("Variable value").fill("sensitive-router-timeout");
  await workspacePanel.getByRole("button", { name: "Save" }).click();
  const workspaceSensitivePatch = requestBodyFor<{ isSensitive?: boolean; value?: string; valueMode?: string }>(
    requests,
    (request) => request.method === "PATCH" && request.pathname === "/api/v1/variables/workspaces/ENTERPRISE/ROUTER_TIMEOUT_MS",
  );
  expect(workspaceSensitivePatch).toMatchObject({
    isSensitive: true,
    value: "sensitive-router-timeout",
    valueMode: "REPLACE",
  });
  await page.getByRole("button", { name: "WORKSPACE_API_CREDENTIAL", exact: true }).click();

  await page.getByRole("button", { name: "Reveal WORKSPACE_API_CREDENTIAL" }).click();
  await page.getByLabel("Step-up password").fill(stepUpPassword);
  await page.getByRole("button", { name: "Reveal Value" }).click();
  await expect(page.getByTestId("variable-sensitive-value-WORKSPACE_API_CREDENTIAL")).toHaveAttribute("data-sensitive-state", "revealed");
  await page.getByRole("button", { name: "Refresh" }).click();
  await expect(page.getByTestId("variable-sensitive-value-WORKSPACE_API_CREDENTIAL")).toHaveAttribute("data-sensitive-state", "masked");
  await page.reload();
  await expect(page.getByRole("button", { name: "WORKSPACE_API_CREDENTIAL", exact: true })).toBeVisible();
  await expect(page.getByText(revealedPlaceholder(workspaceRevealBody?.workspaceCode ?? "workspace", "WORKSPACE_API_CREDENTIAL"))).toHaveCount(0);
  await page.getByRole("button", { name: "WORKSPACE_API_CREDENTIAL", exact: true }).click();
  await expect(page.getByTestId("variable-sensitive-value-WORKSPACE_API_CREDENTIAL")).toHaveAttribute("data-sensitive-state", "masked");
  await captureSafeScreenshot(page, testInfo, "workspace-reload-remasked.png");
});

test("AC-VSD-08 platform variables mask reveal preserve and replace sensitive values", async ({ page }, testInfo) => {
  const requests: CapturedRequest[] = [];
  await routeSensitiveApi(page, requests);

  await page.goto(`${frontendBaseUrl}/settings/variables`);
  await expect(page.getByRole("heading", { name: "Platform Variables" })).toBeVisible();
  await expect(page.getByText(mask).first()).toBeVisible();
  await page.getByLabel("Search platform variables").fill(hiddenSearchFragment("platform", "PLATFORM_PROVIDER_CREDENTIAL"));
  await expect(page.getByText("No platform variables match the current filters.")).toBeVisible();
  await page.getByLabel("Search platform variables").fill("");

  await page.getByRole("button", { name: "New" }).click();
  await page.getByLabel("Platform variable key").fill("PLAYWRIGHT_PLATFORM_SECRET");
  await page.getByRole("button", { name: "Sensitive value" }).click();
  await page.getByLabel("Platform variable value").fill(revealedPlaceholder("platform", "PLAYWRIGHT_PLATFORM_SECRET"));
  await page.getByLabel("Platform variable description").fill("Sensitive platform value created by the browser flow.");
  await page.getByRole("button", { name: "Save" }).click();
  await expect(page.getByRole("heading", { name: "PLAYWRIGHT_PLATFORM_SECRET" })).toBeVisible();
  await expect(page.getByTestId("platform-sensitive-value-PLAYWRIGHT_PLATFORM_SECRET")).toHaveAttribute("data-sensitive-state", "masked");
  expect(requestBodyFor<{ isSensitive?: boolean; valueMode?: string }>(
    requests,
    (request) => request.method === "POST" && request.pathname === "/api/v1/admin/platform/variables",
  )).toMatchObject({
    isSensitive: true,
    valueMode: "REPLACE",
  });
  expect(platformVariablesCreatedAsSensitive(requests, "PLAYWRIGHT_PLATFORM_SECRET")).toBe(true);

  await page.getByRole("button", { name: "Edit" }).click();
  await expect(page.getByRole("heading", { name: "Edit PLAYWRIGHT_PLATFORM_SECRET" })).toBeVisible();
  await expect(page.getByLabel("Platform variable value")).toHaveValue("");
  await expect(page.getByRole("button", { name: "Keep existing sensitive value" })).toHaveAttribute("aria-pressed", "true");
  await page.getByLabel("Platform variable description").fill("Description-only update preserves the encrypted value.");
  await page.getByRole("button", { name: "Save" }).click();
  await expect(page.getByRole("heading", { name: "PLAYWRIGHT_PLATFORM_SECRET" })).toBeVisible();
  const preserveBody = requestBodyFor<{ value?: string; valueMode?: string }>(
    requests,
    (request) => request.method === "PATCH"
      && request.pathname === "/api/v1/admin/platform/variables/PLAYWRIGHT_PLATFORM_SECRET"
      && (request.body as { description?: string } | null)?.description?.includes("preserves"),
  );
  expect(preserveBody?.valueMode).toBe("PRESERVE");
  expect("value" in (preserveBody ?? {}), "preserve edit must not submit a value field").toBe(false);

  await page.getByRole("button", { name: "Reveal PLAYWRIGHT_PLATFORM_SECRET" }).click();
  await page.getByLabel("Step-up password").fill(stepUpPassword);
  await page.getByRole("button", { name: "Reveal Value" }).click();
  await expect(page.getByTestId("platform-sensitive-value-PLAYWRIGHT_PLATFORM_SECRET")).toHaveAttribute("data-sensitive-state", "revealed");
  expect(requestBodyFor<{ password?: string; target?: StepUpTarget }>(
    requests,
    (request) => request.pathname === "/api/v1/auth/verify-step-up-password"
      && (request.body as { target?: { variableKey?: string } } | null)?.target?.variableKey === "PLAYWRIGHT_PLATFORM_SECRET",
  )).toMatchObject({
    password: stepUpPassword,
    target: {
      scope: "platform",
      variableKey: "PLAYWRIGHT_PLATFORM_SECRET",
    },
  });
  await expectVisibleValueMatches(
    page.getByTestId("platform-sensitive-value-PLAYWRIGHT_PLATFORM_SECRET"),
    revealedPlaceholder("platform", "PLAYWRIGHT_PLATFORM_SECRET"),
  );
  expect(requestBodyFor<{ stepUpToken?: string; variableKey?: string }>(
    requests,
    (request) => request.pathname === "/api/v1/admin/platform/variables/reveal-platform-variable",
  )).toMatchObject({
    variableKey: "PLAYWRIGHT_PLATFORM_SECRET",
  });
  await page.getByRole("button", { name: "Hide PLAYWRIGHT_PLATFORM_SECRET" }).click();
  await expect(page.getByTestId("platform-sensitive-value-PLAYWRIGHT_PLATFORM_SECRET")).toHaveAttribute("data-sensitive-state", "masked");
  await captureSafeScreenshot(page, testInfo, "platform-hidden.png");

  await page.getByRole("button", { name: "Edit" }).click();
  await page.getByRole("button", { name: "Replace sensitive value" }).click();
  await page.getByLabel("Platform variable value").fill(revealedPlaceholder("platform", "PLAYWRIGHT_PLATFORM_SECRET_REPLACEMENT"));
  await page.getByRole("button", { name: "Save" }).click();
  await expect(page.getByRole("heading", { name: "PLAYWRIGHT_PLATFORM_SECRET" })).toBeVisible();
  const replaceBody = requestBodyFor<{ isSensitive?: boolean; value?: string; valueMode?: string }>(
    requests,
    (request) => request.method === "PATCH"
      && request.pathname === "/api/v1/admin/platform/variables/PLAYWRIGHT_PLATFORM_SECRET"
      && (request.body as { valueMode?: string } | null)?.valueMode === "REPLACE",
  );
  expect(replaceBody?.isSensitive).toBe(true);
  expect(replaceBody?.valueMode).toBe("REPLACE");
  expect(typeof replaceBody?.value).toBe("string");
  await expect(page.getByTestId("platform-sensitive-value-PLAYWRIGHT_PLATFORM_SECRET")).toHaveAttribute("data-sensitive-state", "masked");
  await captureSafeScreenshot(page, testInfo, "platform-replace-remasked.png");

  await page.getByRole("button", { name: "Edit" }).click();
  await page.getByRole("button", { name: "Sensitive value", exact: true }).click();
  await page.getByRole("button", { name: "Save" }).click();
  await expect(page.getByText("New value is required before turning off sensitivity")).toBeVisible();
  await page.getByLabel("Platform variable value").fill("platform-visible-replacement");
  await page.getByRole("button", { name: "Save" }).click();
  await expect(page.getByRole("heading", { name: "PLAYWRIGHT_PLATFORM_SECRET" })).toBeVisible();
  const downgradeBody = requestBodyFor<{ isSensitive?: boolean; value?: string; valueMode?: string }>(
    requests,
    (request) => request.method === "PATCH"
      && request.pathname === "/api/v1/admin/platform/variables/PLAYWRIGHT_PLATFORM_SECRET"
      && (request.body as { isSensitive?: boolean; value?: string } | null)?.isSensitive === false
      && (request.body as { value?: string } | null)?.value === "platform-visible-replacement",
  );
  expect(downgradeBody).toMatchObject({
    isSensitive: false,
    value: "platform-visible-replacement",
    valueMode: "REPLACE",
  });
});

test("AC-VSD-09 variable mock fixtures do not contain plaintext credential markers", async () => {
  const mockSource = readFileSync(resolve(frontendRoot, "src/domain/variable/mock/data.ts"), "utf8");
  const legacyCredentialMarker = ["sk", "tron", "core", "demo"].join("-");
  expect(mockSource.includes(legacyCredentialMarker), "mock fixtures must not include legacy plaintext credential markers").toBe(false);
});
