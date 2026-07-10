import { expect, test, type Page } from "@playwright/test";
import {
  mkdtempSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { createServer } from "node:net";
import { join, resolve } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";

import {
  spawnManagedProcess,
  stopManagedProcesses,
  type ManagedProcess,
} from "./support/managed-process";

const thisFilePath = fileURLToPath(import.meta.url);
const frontendRoot = resolve(thisFilePath, "..", "..");
const repoRoot = resolve(frontendRoot, "..");
const featureReportsDir = resolve(repoRoot, "docs/features/openapi-open-app-management/reports");
const screenshotPath = resolve(featureReportsDir, "browser-openapi-shell.png");
const networkArtifactPath = resolve(featureReportsDir, "browser-openapi-network.json");
const createName = "Playwright Open App";
const createDescription = "Browser-driven lifecycle verification.";
const userGrantCode = "USR-ADMIN";
const backendHealthPath = "/api/v1/health";

let backendProcess: ManagedProcess | undefined;
let frontendProcess: ManagedProcess | undefined;
let backendLogs = "";
let frontendLogs = "";
let backendBaseUrl = "";
let frontendBaseUrl = "";
let tempRunDir = "";

test.describe.configure({ timeout: 180_000 });
test.use({ locale: "en-US" });

function delay(ms: number) {
  return new Promise((resolveDelay) => {
    setTimeout(resolveDelay, ms);
  });
}

function appendLogs(kind: "backend" | "frontend", chunk: Buffer) {
  const text = chunk.toString();
  if (kind === "backend") {
    backendLogs = `${backendLogs}${text}`.slice(-12_000);
    return;
  }
  frontendLogs = `${frontendLogs}${text}`.slice(-12_000);
}

async function reservePort() {
  return new Promise<number>((resolvePort, rejectPort) => {
    const server = createServer();
    server.on("error", rejectPort);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      if (!address || typeof address === "string") {
        server.close();
        rejectPort(new Error("Failed to reserve a test port."));
        return;
      }
      const { port } = address;
      server.close(() => resolvePort(port));
    });
  });
}

async function waitForUrl(url: string, label: string, failureLogs: () => string) {
  const deadline = Date.now() + 120_000;
  let lastError: unknown;

  while (Date.now() < deadline) {
    try {
      const response = await fetch(url);
      if (response.ok) return;
      lastError = new Error(`${label} returned ${response.status}`);
    } catch (error) {
      lastError = error;
    }

    await delay(250);
  }

  throw new Error(`${label} did not become ready: ${String(lastError)}\n${failureLogs()}`);
}

function toast(page: Page, message: string | RegExp) {
  return page
    .locator('[aria-label="Notifications"]')
    .getByRole("status")
    .filter({ hasText: message })
    .first();
}

function ensureArtifactDirectory() {
  mkdirSync(featureReportsDir, { recursive: true });
}

function redactedNetworkSummary(entries: Array<Record<string, unknown>>) {
  return entries.map((entry) => {
    const url = new URL(String(entry.url));
    return {
      method: entry.method,
      path: url.pathname,
      status: entry.status,
    };
  });
}

test.beforeAll(async () => {
  const backendPort = await reservePort();
  const frontendPort = await reservePort();
  tempRunDir = mkdtempSync(join(tmpdir(), "tron-openapi-smoke-"));
  const databasePath = join(tempRunDir, "openapi-smoke.sqlite3");
  const graphStoragePath = join(tempRunDir, "openapi-smoke.graph.grafeo");

  backendBaseUrl = `http://127.0.0.1:${backendPort}`;
  frontendBaseUrl = `http://127.0.0.1:${frontendPort}`;

  backendProcess = spawnManagedProcess(
    "make",
    [
      "be-run",
      `PORT=${backendPort}`,
      `GRAPH_STORAGE_PATH=${graphStoragePath}`,
    ],
    {
      cwd: repoRoot,
      env: {
        ...process.env,
        TRON_BACKEND_DATABASE_URL: databasePath,
      },
      stdio: "pipe",
    },
  );
  backendProcess.stdout.on("data", (chunk) => appendLogs("backend", chunk));
  backendProcess.stderr.on("data", (chunk) => appendLogs("backend", chunk));

  await waitForUrl(
    `${backendBaseUrl}${backendHealthPath}`,
    "Backend health",
    () => backendLogs,
  );

  frontendProcess = spawnManagedProcess(
    "npm",
    ["run", "dev", "--", "--host", "127.0.0.1", "--port", String(frontendPort)],
    {
      cwd: frontendRoot,
      env: {
        ...process.env,
        TRON_FRONTEND_API_PROXY_TARGET: backendBaseUrl,
        VITE_TRON_API_BASE_URL: "",
        VITE_TRON_DATA_SOURCE: "api",
        VITE_TRON_AUTH_DATA_SOURCE: "api",
        VITE_TRON_ADMIN_DATA_SOURCE: "api",
      },
      stdio: "pipe",
    },
  );
  frontendProcess.stdout.on("data", (chunk) => appendLogs("frontend", chunk));
  frontendProcess.stderr.on("data", (chunk) => appendLogs("frontend", chunk));

  await waitForUrl(
    `${frontendBaseUrl}/login`,
    "Frontend login",
    () => `${frontendLogs}\n--- backend ---\n${backendLogs}`,
  );
});

test.afterAll(async () => {
  const processesToStop = [frontendProcess, backendProcess];
  frontendProcess = undefined;
  backendProcess = undefined;

  try {
    await stopManagedProcesses(processesToStop);
  } finally {
    if (tempRunDir) {
      rmSync(tempRunDir, { force: true, recursive: true });
      tempRunDir = "";
    }
  }
});

test("platform open-app management works end-to-end with one-time key reveal and rotate-only lifecycle", async ({ page }) => {
  const networkEntries: Array<Record<string, unknown>> = [];
  page.on("response", async (response) => {
    const url = response.url();
    if (!url.includes("/api/v1/")) return;
    networkEntries.push({
      method: response.request().method(),
      status: response.status(),
      url,
    });
  });

  await page.goto(`${frontendBaseUrl}/settings/openapi`);
  await expect(page).toHaveURL(/\/login\?redirect=/);

  await page.getByLabel(/operator id|username|email/i).fill("admin");
  await page.getByLabel(/passkey|password/i).fill("TronAdminPass123!");
  await page.getByRole("button", { name: /^sign in$/i }).click();

  await expect(page).toHaveURL(`${frontendBaseUrl}/settings/openapi`);
  await expect(page.getByRole("heading", { name: "Platform OpenAPI" })).toBeVisible();
  await expect(page.getByRole("button", { name: "New" })).toBeVisible();
  await expect(page.getByText("Current business OpenAPI families exposed for authorization.")).toBeVisible();
  await expect(page.getByText("Registered Open Apps")).toBeVisible();
  await expect(page.getByRole("columnheader", { name: "App Name" })).toBeVisible();
  await expect(page.getByRole("columnheader", { name: "App Code" })).toBeVisible();
  await expect(page.getByRole("columnheader", { name: "Fingerprint" })).toHaveCount(0);

  await page.getByRole("button", { name: "New" }).click();
  await expect(page).toHaveURL(`${frontendBaseUrl}/settings/openapi/new`);
  await page.getByLabel("Name").fill(createName);
  await page.getByLabel("Description").fill(createDescription);
  await page.getByRole("button", { name: "Create" }).click();

  await expect(toast(page, `${createName} created`)).toBeVisible();
  await expect(page).toHaveURL(/\/settings\/openapi\/detail\?openAppCode=/);
  await expect(page.getByText("One-time secret reveal")).toBeVisible();
  await expect(page.getByText("Store this app key now")).toBeVisible();
  const openAppCodeText = await page.locator("text=/OPENAPP-[A-Z0-9-]+/").first().textContent();
  expect(openAppCodeText).toBeTruthy();

  await page.getByRole("button", { name: "Edit" }).click();
  await expect(page).toHaveURL(/\/settings\/openapi\/edit\?openAppCode=/);
  const userGrantsSection = page.locator("section").filter({ has: page.getByRole("heading", { name: "User Grants" }) });
  const capabilityGrantsSection = page.locator("section").filter({ has: page.getByRole("heading", { name: "OpenAPI Grants" }) });
  const userCodesInput = userGrantsSection.getByRole("textbox");
  await expect(userGrantsSection).toBeVisible();
  await userCodesInput.fill(userGrantCode);
  await userGrantsSection.getByRole("button", { name: "Grant" }).click();
  await expect(toast(page, "User grants updated")).toBeVisible();
  await expect(userCodesInput).toHaveValue(userGrantCode);

  await expect(capabilityGrantsSection.getByText("Read Sessions")).toHaveCount(0);
  await capabilityGrantsSection.getByRole("button", { name: "Expand Agent Chat" }).click();
  const capabilityCheckbox = capabilityGrantsSection.getByRole("checkbox", { name: "Agent Chat" });
  await capabilityCheckbox.check();
  await expect(capabilityGrantsSection.getByRole("checkbox", { name: "Read Messages" })).toBeChecked();
  await capabilityGrantsSection.getByRole("checkbox", { name: "Read Messages" }).uncheck();
  await expect(capabilityCheckbox).toHaveAttribute("aria-checked", "mixed");
  await capabilityGrantsSection.getByRole("button", { name: "Expand Read Sessions" }).click();
  await expect(capabilityGrantsSection.getByText("GET /open-api/v1/chats/session-groups")).toBeVisible();
  const readSessionsLabel = capabilityGrantsSection.getByRole("checkbox", { name: "Read Sessions" });
  await capabilityGrantsSection.getByRole("button", { name: "Grant" }).click();
  await expect(toast(page, "Capability grants updated")).toBeVisible();
  await expect(readSessionsLabel).toBeChecked();

  await page.getByRole("button", { name: "Cancel" }).click();
  await expect(page).toHaveURL(/\/settings\/openapi\/detail\?openAppCode=/);
  await expect(page.getByText(userGrantCode)).toBeVisible();
  await expect(page.getByText("Read Sessions")).toBeVisible();
  await expect(page.getByText("Current Fingerprint")).toBeVisible();

  await page.getByRole("button", { name: "Rotate key" }).click();
  await expect(page.getByText("Rotate app key?")).toBeVisible();
  await page.getByRole("button", { name: /^Rotate key$/ }).last().click();
  await expect(toast(page, "App key rotated")).toBeVisible();
  await expect(page.getByText("One-time secret reveal")).toBeVisible();

  const secondRevealPanel = page.getByText("Store this app key now");
  await expect(secondRevealPanel).toBeVisible();
  const rotatedFingerprint = await page.locator("text=/Fingerprint: [a-f0-9]+/").first().textContent();
  expect(rotatedFingerprint).toBeTruthy();

  ensureArtifactDirectory();
  await page.screenshot({ path: screenshotPath, fullPage: true });
  writeFileSync(
    networkArtifactPath,
    JSON.stringify(redactedNetworkSummary(networkEntries), null, 2),
    "utf8",
  );

  const networkText = readFileSync(networkArtifactPath, "utf8");
  expect(networkText).toContain("/api/v1/auth/login");
  expect(networkText).toContain("/api/v1/admin/platform/open-apps");
  expect(networkText).toContain("/api/v1/admin/platform/open-apps/catalog");
  expect(networkText).toContain("/user-grants");
  expect(networkText).toContain("/capability-grants");
  expect(networkText).toContain("/rotate-key");
});
