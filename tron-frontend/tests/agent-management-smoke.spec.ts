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
    requests.push(route.request().url());
    await route.abort();
  });

  return requests;
}

function rowByName(page: Page, name: string) {
  return page.locator("tbody tr").filter({
    has: page.getByText(name, { exact: true }),
  }).first();
}

function rowActionButton(row: ReturnType<typeof rowByName>) {
  return row.locator("button").first();
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
        VITE_TRON_API_BASE_URL: "",
        VITE_TRON_AUTH_DATA_SOURCE: "mock",
        VITE_TRON_DATA_SOURCE: "mock",
      },
      stdio: "pipe",
    },
  );

  frontendProcess.stdout.on("data", appendFrontendLog);
  frontendProcess.stderr.on("data", appendFrontendLog);

  await waitForFrontend(`${frontendBaseUrl}/settings/agents`);
});

test.afterAll(async () => {
  try {
    await stopManagedProcess(frontendProcess);
  } finally {
    frontendProcess = undefined;
  }
});

test("agent management works fully in mock mode without backend requests", async ({ page }) => {
  const blockedRequests = await blockApiRequests(page);
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
            {
              workspaceCode: "COMPLIANCE",
              name: "Compliance",
              description: "Audit, retention, permission, and regulated resource controls.",
              childCount: 0,
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
      version: 4,
    }));
  });

  await page.goto(`${frontendBaseUrl}/agents`);
  await expect(page).toHaveURL(/\/agents\/internal$/);
  await expect(page.getByRole("button", { name: "Collapse Agents submenu" })).toBeVisible();
  await expect(page.getByRole("link", { name: "Internal Agents", exact: true })).toBeVisible();
  await expect(page.getByRole("link", { name: "System Agents", exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Internal Agents", exact: true })).toBeVisible();
  await expect(page.getByText("Manage internal agent policy and tool assignment for the selected workspace.")).toHaveCount(0);
  await expect(page.getByText("Current Workspace")).toHaveCount(0);
  await expect(page.getByText("All platform tools are available for this agent.")).toHaveCount(0);

  const mainAgentHeader = page.getByRole("heading", { name: "Main Agent", exact: true }).first();
  const mcpAccessSelect = page.getByRole("combobox", { name: /^mcp access mode/i });
  const toolAssignmentSelect = page.getByRole("combobox", { name: /^tool assignment mode/i });
  const toolExposureSelect = page.getByRole("combobox", { name: /^tool exposure mode/i });
  const expandedPersistenceSelect = page.getByRole("combobox", { name: /^expanded domain persistence mode/i });
  const toolExposureHelpButton = page.getByRole("button", { name: "Explain tool exposure mode" });
  await expect(mainAgentHeader).toBeVisible();
  await expect(mcpAccessSelect).toBeVisible();
  await expect(toolAssignmentSelect).toBeVisible();
  await expect(toolExposureSelect).toBeVisible();
  await expect(expandedPersistenceSelect).toBeVisible();
  await expect(mcpAccessSelect).toHaveValue("all_enabled_servers");
  await expect(page.getByRole("button", { name: "Explain tool assignment mode" })).toBeVisible();
  await expect(toolExposureHelpButton).toBeVisible();
  await expect(page.getByRole("button", { name: "Explain expanded domain persistence mode" })).toBeVisible();
  await toolExposureHelpButton.hover();
  const toolExposureTooltip = page.getByRole("tooltip").filter({
    hasText: "Direct list: send the concrete tool schemas",
  });
  await expect(toolExposureTooltip).toBeVisible();
  const tooltipBox = await toolExposureTooltip.boundingBox();
  const mainBox = await page.locator("main").boundingBox();
  const viewport = page.viewportSize();
  if (!tooltipBox || !mainBox || !viewport) {
    throw new Error("Failed to measure the tool exposure tooltip bounds.");
  }
  expect(tooltipBox.x).toBeGreaterThanOrEqual(mainBox.x + 15);
  expect(tooltipBox.x + tooltipBox.width).toBeLessThanOrEqual(viewport.width - 15);
  await expect(page.getByRole("button", { name: /^save$/i })).toHaveCount(0);
  await expect(page.getByRole("button", { name: /^reset$/i })).toHaveCount(0);
  await expect(toolAssignmentSelect).toHaveValue("all_platform_tools");
  await expect(toolExposureSelect).toHaveValue("direct_list");
  await expect(expandedPersistenceSelect).toHaveValue("sticky_expanded");

  await page.goto(`${frontendBaseUrl}/agents/system`);
  await expect(page).toHaveURL(/\/agents\/system$/);
  await expect(page.getByRole("heading", { name: "Inherited System Agents", exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Main Agent", exact: true })).toHaveCount(0);

  const workspaceTable = page.locator("table").first();
  await expect(workspaceTable).not.toContainText("Main Agent");

  const openCodeWorkspaceRow = rowByName(page, "OpenCode");
  await expect(openCodeWorkspaceRow.locator("td").nth(4)).toContainText("Disabled");
  await expect(openCodeWorkspaceRow.locator("td").nth(5)).toContainText("Platform disabled");
  await expect(openCodeWorkspaceRow.getByRole("button")).toHaveCount(0);

  const codexWorkspaceRow = rowByName(page, "Codex");
  await expect(rowActionButton(codexWorkspaceRow)).toHaveAttribute("aria-label", /Disable Codex/i);
  await rowActionButton(codexWorkspaceRow).click();
  await expect(page.getByRole("status").filter({ hasText: "Codex disabled in" })).toBeVisible();
  await expect(codexWorkspaceRow.locator("td").nth(4)).toContainText("Disabled");
  await expect(codexWorkspaceRow.locator("td").nth(5)).toContainText("Local disable");

  await expect(rowActionButton(codexWorkspaceRow)).toHaveAttribute("aria-label", /Enable Codex/i);
  await rowActionButton(codexWorkspaceRow).click();
  await expect(page.getByRole("status").filter({ hasText: "Codex now follows platform policy" })).toBeVisible();
  await expect(codexWorkspaceRow.locator("td").nth(4)).toContainText("Enabled");
  await expect(codexWorkspaceRow.locator("td").nth(5)).toContainText("Platform baseline");

  await page.goto(`${frontendBaseUrl}/settings/agents`);
  await expect(page).toHaveURL(/\/settings\/agents$/);
  await expect(page.getByRole("heading", { name: "Agent Management" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Main Agent" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "System Agents" })).toBeVisible();

  const personalityInput = page.getByLabel("Personality");
  const instructionsInput = page.getByLabel("Instructions");
  const originalPersonality = await personalityInput.inputValue();
  const originalInstructions = await instructionsInput.inputValue();
  const updatedPersonality = `${originalPersonality} Validate workspace inheritance before closing tasks.`;
  const updatedInstructions = `${originalInstructions}\nAlways keep platform and workspace agent policy aligned.`;

  await personalityInput.fill(updatedPersonality);
  await instructionsInput.fill(updatedInstructions);
  await page.getByRole("button", { name: /^save$/i }).click();
  await expect(page.getByRole("status").filter({ hasText: "Main Agent saved" })).toBeVisible();
  await expect(personalityInput).toHaveValue(updatedPersonality);
  await expect(instructionsInput).toHaveValue(updatedInstructions);
  await expect(page.getByRole("button", { name: /^save$/i })).toBeDisabled();

  const codexSettingsRow = rowByName(page, "Codex");
  await page.getByRole("button", { name: /^scan$/i }).click();
  await expect(page.getByRole("status").filter({ hasText: "Scan complete. Found 1 available system agents" })).toBeVisible();
  await expect(codexSettingsRow.locator("td").nth(3)).not.toHaveText("");

  await expect(rowActionButton(codexSettingsRow)).toHaveAttribute("aria-label", /Disable Codex/i);
  await rowActionButton(codexSettingsRow).click();
  await expect(page.getByRole("status").filter({ hasText: "Codex disabled" })).toBeVisible();
  await expect(codexSettingsRow.locator("td").nth(2)).toContainText("Disabled");

  await expect(rowActionButton(codexSettingsRow)).toHaveAttribute("aria-label", /Enable Codex/i);
  await rowActionButton(codexSettingsRow).click();
  await expect(page.getByRole("status").filter({ hasText: "Codex enabled" })).toBeVisible();
  await expect(codexSettingsRow.locator("td").nth(2)).toContainText("Enabled");

  await expect(page.locator("table").filter({ hasText: "OpenCode" })).toHaveCount(0);

  await page.goto(`${frontendBaseUrl}/settings/tools`);
  await expect(page).toHaveURL(/\/settings\/tools$/);
  await expect(page.getByRole("heading", { name: "Platform Tools" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Tool Domains" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Tools", exact: true })).toBeVisible();
  await expect(page.getByText("workspace.page", { exact: true })).toBeVisible();
  const toolRowsPerPage = page.getByLabel("Rows per page");
  await expect(toolRowsPerPage).toHaveCount(2);
  await expect(toolRowsPerPage.nth(0)).toHaveValue("15");
  await expect(toolRowsPerPage.nth(1)).toHaveValue("15");

  expect(blockedRequests).toEqual([]);
});
