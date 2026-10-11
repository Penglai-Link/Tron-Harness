import { routeActiveProfileApi } from "./support/resource-ui-harness";
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
  await waitForFrontend(`${frontendBaseUrl}/settings/prompts`);
});

test.afterAll(async () => {
  await stopManagedProcess(frontendProcess);
  frontendProcess = undefined;
});

test("manages global platform prompts and restores list state after detail", async ({ page }) => {
  await page.addInitScript(() => window.localStorage.setItem("tron.locale", "en-US"));
  await routeActiveProfileApi(page);
  await page.goto(`${frontendBaseUrl}/settings/prompts`);

  const search = page.getByRole("searchbox", { name: "Search prompts" });
  await expect(search).toBeVisible();
  await expect(page.getByRole("combobox", { name: "Category", exact: true })).toBeVisible();
  await expect(page.getByRole("combobox", { name: "Status", exact: true })).toBeVisible();
  await expect(page.getByRole("columnheader", { name: "Variables" })).toBeVisible();
  await expect(page.getByRole("columnheader", { name: "Source" })).toHaveCount(0);
  await expect(page.getByRole("columnheader", { name: "Inheritance" })).toHaveCount(0);

  await search.fill("Main Agent");
  const mainAgent = page.getByRole("row").filter({ hasText: "ROLE_MAIN_AGENT" });
  await expect(mainAgent).toBeVisible();
  await mainAgent.click();
  await expect(page).toHaveURL(/\/settings\/prompts\/detail\?promptCode=ROLE_MAIN_AGENT/);
  await page.getByRole("navigation", { name: "Breadcrumb" }).getByRole("link", { name: "Platform Prompts" }).click();
  await expect(page).toHaveURL(`${frontendBaseUrl}/settings/prompts`);
  await expect(page.getByRole("searchbox", { name: "Search prompts" })).toHaveValue("Main Agent");
  await expect(page.getByRole("row").filter({ hasText: "ROLE_MAIN_AGENT" })).toBeVisible();
});

test("localizes the platform prompt manager without workspace scope controls", async ({ page }) => {
  await page.addInitScript(() => window.localStorage.setItem("tron.locale", "zh-CN"));
  await routeActiveProfileApi(page);
  await page.goto(`${frontendBaseUrl}/settings/prompts`);

  await expect(page.getByRole("searchbox", { name: "搜索提示词" })).toBeVisible();
  await expect(page.getByRole("combobox", { name: "分类", exact: true })).toBeVisible();
  await expect(page.getByRole("combobox", { name: "状态", exact: true })).toBeVisible();
  await expect(page.getByRole("columnheader", { name: "变量" })).toBeVisible();
  await expect(page.getByRole("columnheader", { name: "来源" })).toHaveCount(0);
  await expect(page.getByRole("columnheader", { name: "继承方式" })).toHaveCount(0);
  await expect(page.getByRole("row").filter({ hasText: "ROLE_MAIN_AGENT" })).toBeVisible();
});
