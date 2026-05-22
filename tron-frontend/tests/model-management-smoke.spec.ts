import { expect, test, type Page, type Route } from "@playwright/test";
import { spawn, type ChildProcessWithoutNullStreams } from "node:child_process";
import { createServer } from "node:net";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const thisFilePath = fileURLToPath(import.meta.url);
const frontendRoot = resolve(thisFilePath, "..", "..");
let frontendProcess: ChildProcessWithoutNullStreams | undefined;
let frontendLogs = "";
let frontendBaseUrl = "";

type WorkspaceModelStatus = "Available" | "Removed";
type ModelVendor = "OpenAI" | "Anthropic" | "Google";
type PlatformModelRecord = {
  context: string;
  maxOutput: string;
  modelCode: string;
  name: string;
  price: string;
  purpose: string;
  speed: string;
  trainingData: string;
  vendor: ModelVendor;
};

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

const workspaceSwitcher = {
  primarySpaces: [{
    workspaceCode: "ENTERPRISE",
    name: "Enterprise",
    description: "Global governance workspace",
    childCount: 1,
    status: "Active",
  }],
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
      description: "Agent runtime governance and admin operations.",
      depth: 2,
      childCount: 1,
      effectiveUserCount: 3,
      status: "Active",
    },
    {
      workspaceCode: "TRON_CORE",
      primaryWorkspaceCode: "ENTERPRISE",
      label: "Enterprise / Platform / Tron Core",
      name: "Tron Core",
      description: "Main Tron console workspace.",
      depth: 3,
      childCount: 0,
      effectiveUserCount: 2,
      status: "Active",
    },
  ],
};

const workspaceOptions = workspaceSwitcher.secondarySpaces.map((space) => ({
  workspaceCode: space.workspaceCode,
  label: space.label,
  name: space.name,
  description: space.description,
  parentWorkspaceCode: space.depth === 1 ? null : space.depth === 2 ? "ENTERPRISE" : "PLATFORM",
  depth: space.depth,
  status: space.status,
}));

const platformModels: PlatformModelRecord[] = [
  {
    modelCode: "claude-3-opus-20240229",
    name: "claude-3-opus-20240229",
    purpose: "Complex Tasks",
    vendor: "Anthropic",
    context: "200k tokens",
    maxOutput: "4,096 tokens",
    trainingData: "Aug 2023",
    speed: "~31 tk/s",
    price: "$15.00 / 1M Input",
  },
  {
    modelCode: "gemini-1.5-pro",
    name: "gemini-1.5-pro",
    purpose: "Multimodal Review",
    vendor: "Google",
    context: "2M tokens",
    maxOutput: "8,192 tokens",
    trainingData: "Apr 2024",
    speed: "~36 tk/s",
    price: "$7.00 / 1M Input",
  },
  {
    modelCode: "gpt-4-turbo",
    name: "gpt-4-turbo",
    purpose: "Default Reasoning",
    vendor: "OpenAI",
    context: "128k tokens",
    maxOutput: "4,096 tokens",
    trainingData: "Dec 2023",
    speed: "~42 tk/s",
    price: "$10.00 / 1M Input",
  },
];

const removalState = new Map<string, Set<string>>();

function successEnvelope(data: unknown) {
  return { code: "SUCCESS", message: "ok", data };
}

async function fulfillJson(route: Route, data: unknown, status = 200) {
  await route.fulfill({ status, contentType: "application/json", body: JSON.stringify(data) });
}

function workspaceModels(workspaceCode: string) {
  const removed = workspaceCode === "PLATFORM" ? new Set<string>() : removalState.get(workspaceCode) ?? new Set<string>();
  return platformModels.map((model) => ({
    ...model,
    status: (removed.has(model.modelCode) ? "Removed" : "Available") as WorkspaceModelStatus,
    sourceWorkspaceCode: "PLATFORM",
    sourceWorkspaceName: "Platform",
    sourceLabel: "Platform",
    removedAt: removed.has(model.modelCode) ? "2026-05-22T00:00:00Z" : null,
  }));
}

async function routeApi(page: Page) {
  await page.route("**/api/v1/**", async (route) => {
    const request = route.request();
    const url = new URL(request.url());
    const pathname = url.pathname;

    if (pathname === "/api/v1/auth/providers") return fulfillJson(route, successEnvelope({ providers: [{ code: "internal", name: "Internal Account", type: "internal" }] }));
    if (pathname === "/api/v1/auth/refresh") return fulfillJson(route, successEnvelope({ accessToken: "model-token", tokenType: "Bearer", expiresIn: 900, session: authenticatedSession }));
    if (pathname === "/api/v1/auth/session") return fulfillJson(route, successEnvelope(authenticatedSession));
    if (pathname === "/api/v1/auth/logout") return fulfillJson(route, successEnvelope({ loggedOut: true }));
    if (pathname === "/api/v1/admin/workspaces/options") return fulfillJson(route, successEnvelope(workspaceOptions));
    if (pathname === "/api/v1/admin/workspaces/switcher") return fulfillJson(route, successEnvelope(workspaceSwitcher));

    const listMatch = pathname.match(/^\/api\/v1\/models\/workspaces\/([^/]+)$/);
    if (listMatch && request.method() === "GET") {
      const workspaceCode = decodeURIComponent(listMatch[1]);
      return fulfillJson(route, successEnvelope(workspaceModels(workspaceCode)));
    }

    const removeMatch = pathname.match(/^\/api\/v1\/models\/workspaces\/([^/]+)\/([^/]+)\/remove$/);
    if (removeMatch && request.method() === "POST") {
      const workspaceCode = decodeURIComponent(removeMatch[1]);
      const modelCode = decodeURIComponent(removeMatch[2]);
      const removed = removalState.get(workspaceCode) ?? new Set<string>();
      removed.add(modelCode);
      removalState.set(workspaceCode, removed);
      return fulfillJson(route, successEnvelope({
        workspaceCode,
        modelCode,
        status: "Removed",
        removedAt: "2026-05-22T00:00:00Z",
      }));
    }

    const restoreMatch = pathname.match(/^\/api\/v1\/models\/workspaces\/([^/]+)\/([^/]+)\/restore$/);
    if (restoreMatch && request.method() === "POST") {
      const workspaceCode = decodeURIComponent(restoreMatch[1]);
      const modelCode = decodeURIComponent(restoreMatch[2]);
      const removed = removalState.get(workspaceCode) ?? new Set<string>();
      removed.delete(modelCode);
      removalState.set(workspaceCode, removed);
      return fulfillJson(route, successEnvelope({
        workspaceCode,
        modelCode,
        status: "Available",
        removedAt: null,
      }));
    }

    return fulfillJson(route, { code: "NOT_FOUND", message: `Unhandled ${request.method()} ${pathname}`, data: {} }, 404);
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
  frontendProcess = spawn("npm", ["run", "dev", "--", "--host", "127.0.0.1", "--port", String(port)], {
    cwd: frontendRoot,
    env: {
      ...process.env,
      CI: "1",
      VITE_TRON_ADMIN_DATA_SOURCE: "api",
      VITE_TRON_MODEL_DATA_SOURCE: "api",
      VITE_TRON_DATA_SOURCE: "api",
    },
    stdio: "pipe",
  });
  frontendProcess.stdout.on("data", (chunk) => { frontendLogs = `${frontendLogs}${chunk.toString()}`.slice(-5000); });
  frontendProcess.stderr.on("data", (chunk) => { frontendLogs = `${frontendLogs}${chunk.toString()}`.slice(-5000); });
  await waitForFrontend(`${frontendBaseUrl}/models`);
});

test.afterAll(async () => {
  frontendProcess?.kill("SIGTERM");
});

test("workspace model removal only hides the model in the current workspace", async ({ page }) => {
  await routeApi(page);
  await page.goto(`${frontendBaseUrl}/models`);

  await expect(page.getByRole("heading", { name: "Workspace Models" })).toBeVisible();
  await expect(page.locator("p").filter({ hasText: /^Enterprise \/ Platform \/ Tron Core$/ }).first()).toBeVisible();
  await expect(page.getByRole("button", { name: /gpt-4-turbo/i })).toContainText("Available");

  await page.getByRole("button", { name: /gpt-4-turbo/i }).click();
  await page.getByRole("button", { name: /Remove From Enterprise \/ Platform \/ Tron Core/i }).click();

  await expect(page.getByRole("button", { name: /gpt-4-turbo/i })).toContainText("Removed");
  await expect(page.getByRole("button", { name: /Restore In Enterprise \/ Platform \/ Tron Core/i })).toBeVisible();

  const platformPayload = await page.evaluate(async () => {
    const response = await fetch("/api/v1/models/workspaces/PLATFORM");
    return response.json();
  });
  expect(platformPayload.data.find((record: { modelCode: string }) => record.modelCode === "gpt-4-turbo")?.status).toBe("Available");

  await page.getByRole("button", { name: /Restore In Enterprise \/ Platform \/ Tron Core/i }).click();
  await expect(page.getByRole("button", { name: /gpt-4-turbo/i })).toContainText("Available");
});
