import { routeActiveProfileApi } from "./support/resource-ui-harness";
import {
  expect,
  test,
  type Page,
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
let frontendProcess: ManagedProcess | undefined;
let frontendLogs = "";
let frontendBaseUrl = "";

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

      server.close(() => resolvePort(address.port));
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
      if (response.ok) {
        return;
      }
    } catch (error) {
      lastError = error;
    }

    await delay(250);
  }

  throw new Error(`Frontend dev server did not become ready: ${String(lastError)}\n${frontendLogs}`);
}

async function blockApiRequests(page: Page) {
  const requests: string[] = [];

  await page.route("**/api/v1/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path.startsWith("/api/v1/system/skins")) {
      await route.fulfill({ contentType: "application/json", body: JSON.stringify({ code: "SUCCESS", data: path.endsWith("/active") ? null : [] }) });
      return;
    }
    requests.push(route.request().url());
    await route.abort();
  });

  return requests;
}

async function seedWorkspaceSelection(page: Page) {
  await page.addInitScript(() => {
    localStorage.setItem("tron-app-shell", JSON.stringify({
      state: {
        openSections: { Core: true },
        selectedPrimarySpaceId: "ENTERPRISE",
        selectedSecondarySpaceId: "TRON_CORE",
        sidebarCollapsed: false,
        workspaceSwitcherSnapshot: {
          primarySpaces: [
            {
              workspaceCode: "ENTERPRISE",
              name: "Enterprise",
              description: "Global governance and shared AI resource policy.",
              childCount: 1,
              status: "Active",
            },
          ],
          secondarySpaces: [
            {
              workspaceCode: "ENTERPRISE",
              primaryWorkspaceCode: "ENTERPRISE",
              parentWorkspaceCode: null,
              label: "Enterprise",
              name: "Enterprise",
              description: "Global governance and shared AI resource policy.",
              status: "Active",
              depth: 1,
              childCount: 1,
              effectiveUserCount: 1,
            },
            {
              workspaceCode: "PLATFORM",
              primaryWorkspaceCode: "ENTERPRISE",
              parentWorkspaceCode: "ENTERPRISE",
              label: "Enterprise / Platform",
              name: "Platform",
              description: "Agent runtime governance and admin operations.",
              status: "Active",
              depth: 2,
              childCount: 1,
              effectiveUserCount: 3,
            },
            {
              workspaceCode: "TRON_CORE",
              primaryWorkspaceCode: "ENTERPRISE",
              parentWorkspaceCode: "PLATFORM",
              label: "Enterprise / Platform / Tron Core",
              name: "Tron Core",
              description: "Main Tron console workspace.",
              status: "Active",
              depth: 3,
              childCount: 0,
              effectiveUserCount: 2,
            },
          ],
        },
      },
      version: 9,
    }));
  });
}

function rowByName(page: Page, name: string) {
  return page.locator("tbody tr").filter({
    has: page.getByText(name, { exact: true }),
  }).first();
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
        VITE_TRON_ADMIN_DATA_SOURCE: "mock",
        VITE_TRON_AGENT_DATA_SOURCE: "mock",
        VITE_TRON_AUTH_DATA_SOURCE: "mock",
        VITE_TRON_DATA_SOURCE: "mock",
        VITE_TRON_MCP_DATA_SOURCE: "mock",
      },
      stdio: "pipe",
    },
  );

  frontendProcess.stdout.on("data", appendFrontendLog);
  frontendProcess.stderr.on("data", appendFrontendLog);

  await waitForFrontend(`${frontendBaseUrl}/settings/mcp-servers`);
});

test.afterAll(async () => {
  try {
    await stopManagedProcess(frontendProcess);
  } finally {
    frontendProcess = undefined;
  }
});

test("mcp settings, workspace policy, and agent assignment work in mock mode", async ({ page }) => {
  const blockedRequests = await blockApiRequests(page);
  await seedWorkspaceSelection(page);

  await routeActiveProfileApi(page);
  await page.goto(`${frontendBaseUrl}/settings/mcp-servers`);
  await expect(page.getByRole("heading", { name: "Platform registry", exact: true })).toBeVisible();
  const internalModulesTitle = page.getByRole("heading", { name: "Internal modules" });
  await expect(internalModulesTitle).toBeVisible();
  await expect(internalModulesTitle).toHaveAttribute("title", "Manage platform-internal MCP modules exposed through the gateway.");
  await expect(page.getByRole("button", { name: "Gateway" })).toHaveCount(0);
  await expect(page.getByRole("switch", { name: "Variables endpoint" })).toBeVisible();
  await page.getByRole("switch", { name: "Variables endpoint" }).click();
  await expect(page.getByRole("dialog").filter({ hasText: "Module settings saved" })).toBeVisible();

  await page.getByRole("button", { name: "New" }).click();
  await expect(page).toHaveURL(/\/settings\/mcp-servers\/new/);
  await expect(page.getByLabel("Platform MCP server code")).toBeVisible();

  await page.getByLabel("Platform MCP server code").fill("release_tools");
  await page.getByLabel("Platform MCP server name").fill("Release Tools");
  await page.getByLabel("Platform MCP server URL").fill("https://mcp.example.com/release");
  await page.getByLabel("Platform MCP server description").fill("Release orchestration MCP server.");
  await page.getByRole("button", { name: "Verify" }).click();
  await expect(page.getByRole("dialog").filter({ hasText: "Release Tools verified" })).toBeVisible();
  await page.getByRole("button", { name: "Save" }).click();
  const releaseToolsCreatedToast = page.getByRole("dialog").filter({ hasText: "Release Tools created" });
  await expect(releaseToolsCreatedToast).toBeVisible();
  await expect(page).toHaveURL(/\/settings\/mcp-servers\/detail\?serverCode=release_tools/);
  await expect(page.getByRole("navigation", { name: "Breadcrumb" }).filter({ hasText: "Release Tools" })).toBeVisible();
  await expect(releaseToolsCreatedToast).toBeHidden({ timeout: 10_000 });
  await page.getByRole("button", { name: "Edit" }).click();
  await expect(page).toHaveURL(/\/settings\/mcp-servers\/edit\?serverCode=release_tools/);
  await expect(page.getByRole("textbox", { name: "Platform MCP server code" })).toBeVisible();
  await page.getByRole("button", { name: "Verify" }).click();
  await expect(page.getByRole("dialog").filter({ hasText: "Release Tools verified" })).toBeVisible();
  await page.getByRole("button", { name: "Cancel" }).click();
  await expect(rowByName(page, "Release Tools")).toBeVisible();

  const knowledgeRow = rowByName(page, "Knowledge Tools");
  await knowledgeRow.getByRole("button", { name: /Open actions/i }).click();
  await page.getByRole("button", { name: "Verify" }).click();
  await expect(page.getByRole("dialog").filter({ hasText: "Knowledge Tools verified" })).toBeVisible();

  await routeActiveProfileApi(page);
  await page.goto(`${frontendBaseUrl}/mcp-servers`);
  await expect(page.getByRole("tab", { name: "MCP Servers", exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Effective MCP servers", exact: true })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Open workspace selector" })).toBeVisible();
  const workspaceKnowledgeRow = rowByName(page, "Knowledge Tools");
  await workspaceKnowledgeRow.getByRole("switch").click();
  await expect(page.getByRole("dialog").filter({ hasText: "Knowledge Tools disabled in" })).toBeVisible();
  await expect(workspaceKnowledgeRow).toContainText("Local disable");
  await workspaceKnowledgeRow.getByRole("switch").click();
  await expect(page.getByRole("dialog").filter({ hasText: "Knowledge Tools now follows platform policy" })).toBeVisible();

  await routeActiveProfileApi(page);
  await page.goto(`${frontendBaseUrl}/agents`);
  await expect(page.getByRole("tab", { name: "Internal Agents", exact: true })).toBeVisible();
  const mainAgentSection = page.locator("section").filter({
    has: page.getByRole("heading", { name: "Main Agent", exact: true }),
  }).first();
  const mcpAccessSelect = page.getByRole("combobox", { name: /^mcp access mode/i });
  await mcpAccessSelect.click(); await page.getByRole("option", { name: "Selected servers", exact: true }).click();
  await expect(page.getByText("Selected MCP servers")).toBeVisible();

  const knowledgeServerCard = page.locator("label").filter({ has: page.getByText("Knowledge Tools", { exact: true }) }).first();
  await knowledgeServerCard.locator('input[type="checkbox"]').check();
  await page.waitForTimeout(1200);
  await expect(mcpAccessSelect).toContainText("Selected servers");
  await expect(knowledgeServerCard.locator('input[type="checkbox"]')).toBeChecked();

  await routeActiveProfileApi(page);
  await page.goto(`${frontendBaseUrl}/settings/mcp-servers/new`);
  await page.getByRole("tab", { name: "STDIO" }).click();
  await expect(page.getByLabel("Platform MCP server code")).toBeVisible();
  await page.getByLabel("Platform MCP server code").fill("ops_tools_new");
  await page.getByLabel("Platform MCP server name").fill("Ops Tools New");
  await page.getByLabel("Platform MCP stdio command").fill("openai-dev-mcp");
  await page.getByRole("textbox", { name: "Arguments item 1" }).fill("serve-ops");
  await page.getByRole("textbox", { name: "Environment variables key 1" }).fill("MCP_TOKEN");
  await page.getByRole("textbox", { name: "Environment variables value 1" }).fill("MCP_OPS_TOKEN");
  await page.getByRole("textbox", { name: "Environment variable passthrough item 1" }).fill("PATH");
  await page.getByLabel("Platform MCP working directory").fill("~/code");
  await page.getByRole("button", { name: "Verify" }).click();
  await expect(page.getByRole("dialog").filter({ hasText: "Ops Tools New verified" })).toBeVisible();
  await page.getByRole("button", { name: "Save" }).click();
  await expect(page.getByRole("dialog").filter({ hasText: "Ops Tools New created" })).toBeVisible();
  await expect(page).toHaveURL(/\/settings\/mcp-servers\/detail\?serverCode=ops_tools_new/);
  await expect(page.getByRole("navigation", { name: "Breadcrumb" }).filter({ hasText: "Ops Tools New" })).toBeVisible();
  await page.getByRole("button", { name: "Edit" }).click();
  await expect(page).toHaveURL(/\/settings\/mcp-servers\/edit\?serverCode=ops_tools_new/);
  await page.getByRole("button", { name: "Cancel" }).click();
  await expect(rowByName(page, "Ops Tools New")).toBeVisible();

  expect(blockedRequests).toEqual([]);
});
