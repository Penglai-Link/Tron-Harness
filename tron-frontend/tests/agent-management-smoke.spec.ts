import {
  expect,
  test,
  type Page,
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

  frontendProcess = spawn(
    "npm",
    ["run", "dev", "--", "--host", "127.0.0.1", "--port", String(port), "--strictPort"],
    {
      cwd: frontendRoot,
      env: {
        ...process.env,
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
  if (!frontendProcess) {
    return;
  }

  frontendProcess.kill("SIGTERM");
  await new Promise<void>((resolveProcess) => {
    frontendProcess?.once("exit", () => resolveProcess());
    setTimeout(() => resolveProcess(), 5_000);
  });
});

test("agent management works fully in mock mode without backend requests", async ({ page }) => {
  const blockedRequests = await blockApiRequests(page);

  await page.goto(`${frontendBaseUrl}/agents`);
  await expect(page).toHaveURL(/\/agents$/);
  await expect(page.getByRole("heading", { name: "Agents", exact: true })).toBeVisible();
  await expect(page.getByText("Current Workspace")).toBeVisible();

  const workspaceTable = page.locator("table").first();
  await expect(workspaceTable).not.toContainText("Main Agent");

  const openCodeWorkspaceRow = rowByName(page, "OpenCode");
  await expect(openCodeWorkspaceRow.locator("td").nth(4)).toContainText("Disabled");
  await expect(openCodeWorkspaceRow.locator("td").nth(5)).toContainText("Platform disabled");
  await expect(openCodeWorkspaceRow.locator("td").nth(6)).toContainText("Read only");
  await expect(openCodeWorkspaceRow.getByRole("button")).toHaveCount(0);

  const codexWorkspaceRow = rowByName(page, "Codex");
  await expect(rowActionButton(codexWorkspaceRow)).toHaveText("Disable");
  await rowActionButton(codexWorkspaceRow).click();
  await expect(page.getByRole("status").filter({ hasText: "Codex disabled in" })).toBeVisible();
  await expect(codexWorkspaceRow.locator("td").nth(4)).toContainText("Disabled");
  await expect(codexWorkspaceRow.locator("td").nth(5)).toContainText("Local disable");

  await expect(rowActionButton(codexWorkspaceRow)).toHaveText("Enable");
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
  await expect(page.getByRole("status").filter({ hasText: "Scanned 2 system agents; 1 unavailable" })).toBeVisible();
  await expect(codexSettingsRow.locator("td").nth(3)).not.toHaveText("");

  await expect(rowActionButton(codexSettingsRow)).toHaveText("Disable");
  await rowActionButton(codexSettingsRow).click();
  await expect(page.getByRole("status").filter({ hasText: "Codex disabled" })).toBeVisible();
  await expect(codexSettingsRow.locator("td").nth(2)).toContainText("Disabled");

  await expect(rowActionButton(codexSettingsRow)).toHaveText("Enable");
  await rowActionButton(codexSettingsRow).click();
  await expect(page.getByRole("status").filter({ hasText: "Codex enabled" })).toBeVisible();
  await expect(codexSettingsRow.locator("td").nth(2)).toContainText("Enabled");

  const openCodeSettingsRow = rowByName(page, "OpenCode");
  await expect(openCodeSettingsRow.locator("td").nth(2)).toContainText("Unavailable");
  await expect(openCodeSettingsRow.locator("td").nth(4)).toContainText("Read only");
  await expect(openCodeSettingsRow.getByRole("button")).toHaveCount(0);

  expect(blockedRequests).toEqual([]);
});
