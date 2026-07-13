import {
  expect,
  test,
} from "@playwright/test";
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
let frontendBaseUrl = "";
let frontendLogs = "";

function reservePort() {
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
  while (Date.now() < deadline) {
    if (frontendProcess?.exitCode !== null) throw new Error(frontendLogs);
    try {
      const response = await fetch(url);
      if (response.ok) return;
    } catch {
      // The server can refuse connections briefly while Vite initializes.
    }
    await new Promise((resolveDelay) => setTimeout(resolveDelay, 200));
  }
  throw new Error(`Frontend did not become ready.\n${frontendLogs}`);
}

test.describe.configure({ timeout: 45_000 });

test.beforeAll(async () => {
  const port = await reservePort();
  frontendBaseUrl = `http://127.0.0.1:${port}`;
  frontendProcess = spawnManagedProcess(
    "npm",
    ["run", "dev", "--", "--host", "127.0.0.1", "--port", String(port)],
    {
      cwd: frontendRoot,
      env: {
        ...process.env,
        VITE_TRON_AUTH_DATA_SOURCE: "mock",
        VITE_TRON_DATA_SOURCE: "mock",
        VITE_TRON_MODEL_DATA_SOURCE: "mock",
        VITE_TRON_PROMPT_DATA_SOURCE: "mock",
      },
      stdio: "pipe",
    },
  );
  frontendProcess.stdout.on("data", (chunk: Buffer) => {
    frontendLogs = `${frontendLogs}${chunk.toString()}`.slice(-5000);
  });
  frontendProcess.stderr.on("data", (chunk: Buffer) => {
    frontendLogs = `${frontendLogs}${chunk.toString()}`.slice(-5000);
  });
  await waitForFrontend(`${frontendBaseUrl}/prompts`);
});

test.afterAll(async () => {
  await stopManagedProcess(frontendProcess);
  frontendProcess = undefined;
});

test("manages workspace prompts and exposes platform prompt routes", async ({ page }) => {
  await page.goto(`${frontendBaseUrl}/prompts`);
  await expect(page.getByRole("heading", { name: "Prompt Management" })).toBeVisible();
  await expect(page.getByRole("button", { name: "All Prompts" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Filter", exact: true })).toBeVisible();
  await expect(page.getByRole("columnheader", { name: "Inheritance" })).toBeVisible();
  await expect(page.getByRole("columnheader", { name: "Last Updated" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Workspace Router", exact: true })).toBeVisible();

  await page.getByRole("button", { name: "Workspace Router", exact: true }).click();
  await expect(page.getByText("Prompt Details", { exact: true })).toBeVisible();
  await expect(page.getByText("Test Variables", { exact: true })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Render Preview", exact: true })).toHaveCount(0);

  await page.getByRole("button", { name: "Actions for Workspace Router" }).click();
  await expect(page.getByText("Create override", { exact: true })).toBeVisible();
  await page.keyboard.press("Escape");

  await page.getByRole("button", { name: "New", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Create Prompt" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Prompt Settings" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Model Parameters" })).toHaveCount(0);
  await expect(page.getByText("Temperature", { exact: false })).toHaveCount(0);
  await expect(page.getByText("Max output tokens", { exact: true })).toHaveCount(0);
  await page.getByLabel("Name").fill("Greeting Template");
  await page.getByLabel("Prompt body").fill("Hello {{name}} from {{workspace_name}}.");
  await expect(page.getByText("STRING").first()).toBeVisible();
  await expect(page.getByText("{{name}}", { exact: true })).toBeVisible();
  await expect(page.getByText("{{workspace_name}}", { exact: true })).toBeVisible();
  await expect(page.getByLabel("{{name}}")).toHaveCount(0);
  await expect(page.getByLabel("{{workspace_name}}")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Preview", exact: true })).toHaveCount(0);
  await expect(page.getByText("Rendered prompt", { exact: true })).toHaveCount(0);

  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expect(page).toHaveURL(/\/prompts\/detail\?promptCode=/);
  await expect(page.getByRole("heading", { name: "Prompt Details" })).toBeVisible();
  await expect(page.getByRole("textbox", { name: "Prompt code", exact: true })).toHaveAttribute("readonly", "");

  await page.goto(`${frontendBaseUrl}/settings/prompts`);
  await expect(page.getByRole("heading", { name: "Platform Prompts" })).toBeVisible();
  await expect(page.getByRole("button", { name: "New", exact: true })).toBeVisible();
});
