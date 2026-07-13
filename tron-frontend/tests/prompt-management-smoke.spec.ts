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
  await expect(page.getByRole("columnheader", { name: "Status" })).toBeVisible();
  await expect(page.getByRole("columnheader", { name: "Last Updated" })).toBeVisible();
  const workspaceRouterRow = page.getByRole("row").filter({ hasText: "Workspace Router" });
  await expect(workspaceRouterRow).toBeVisible();

  await workspaceRouterRow.click();
  await expect(page).toHaveURL(/\/prompts\/detail\?promptCode=.*mode=override/);
  await expect(page.getByRole("heading", { name: "Create Prompt Override" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Render Preview", exact: true })).toHaveCount(0);

  await page.goto(`${frontendBaseUrl}/prompts`);
  await page.getByRole("button", { name: "Actions for Workspace Router" }).click();
  await expect(page.getByText("Duplicate", { exact: true })).toHaveCount(0);
  await page.keyboard.press("Escape");

  await page.getByRole("button", { name: "New", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Create Prompt" })).toBeVisible();
  await expect(page.getByRole("switch", { name: "Enabled", exact: true })).toBeVisible();
  await expect(page.getByRole("switch", { name: "Allow inheritance", exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Model Parameters" })).toHaveCount(0);
  await expect(page.getByText("Temperature", { exact: false })).toHaveCount(0);
  await expect(page.getByText("Max output tokens", { exact: true })).toHaveCount(0);
  await page.getByLabel("Name").fill("Greeting Template");
  await page.getByLabel("Prompt body").fill("Hello {{name}} with priority {{priority}}.");
  await expect(page.getByText("STRING").first()).toBeVisible();
  await expect(page.getByRole("code").filter({ hasText: /^name$/ })).toBeVisible();
  await expect(page.getByRole("code").filter({ hasText: /^priority$/ })).toBeVisible();
  await expect(page.getByRole("button", { name: "Preview", exact: true })).toHaveCount(0);
  await expect(page.getByText("Rendered prompt", { exact: true })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Save", exact: true })).toBeDisabled();
  await page.getByRole("textbox", { name: "Description name", exact: true }).fill("Customer display name");
  await page.getByRole("textbox", { name: "Description priority", exact: true }).fill("Priority number");
  await page.getByRole("combobox", { name: "Type priority", exact: true }).click();
  await page.getByRole("option", { name: "NUMBER", exact: true }).click();

  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expect(page).toHaveURL(/\/prompts\/detail\?promptCode=/);
  await expect(page.getByRole("heading", { name: "Prompt Details" })).toBeVisible();
  await expect(page.getByRole("textbox", { name: "Prompt code", exact: true })).toHaveAttribute("readonly", "");
  await expect(page.getByRole("textbox", { name: "Description name", exact: true })).toHaveValue(
    "Customer display name",
  );
  await expect(page.getByRole("textbox", { name: "Description priority", exact: true })).toHaveValue(
    "Priority number",
  );

  await page.goto(`${frontendBaseUrl}/settings/prompts`);
  await expect(page.getByRole("heading", { name: "Platform Prompts" })).toBeVisible();
  await expect(page.getByRole("button", { name: "New", exact: true })).toBeVisible();
  await page.getByRole("row").filter({ hasText: "Workspace Router" }).click();
  await expect(page).toHaveURL(/\/settings\/prompts\/detail\?promptCode=/);
});

test("reverts a workspace override without leaving the prompt list", async ({ page }) => {
  await page.goto(`${frontendBaseUrl}/prompts`);
  await page.getByRole("row").filter({ hasText: "Workspace Router" }).click();
  await expect(page.getByRole("heading", { name: "Create Prompt Override" })).toBeVisible();
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expect(page).toHaveURL(/\/prompts\/detail\?promptCode=/);

  await page.getByRole("button", { name: "Cancel", exact: true }).click();
  await expect(page).toHaveURL(`${frontendBaseUrl}/prompts`);
  await page.getByRole("button", { name: "Actions for Workspace Router" }).click();
  await page.getByRole("menuitem", { name: "Revert", exact: true }).click();
  await expect(page).toHaveURL(`${frontendBaseUrl}/prompts`);

  const confirmDialog = page.getByRole("dialog", { name: "Revert override?" });
  await expect(confirmDialog).toBeVisible();
  await confirmDialog.getByRole("button", { name: "Revert", exact: true }).click();
  await expect(confirmDialog).toHaveCount(0);
  await expect(page).toHaveURL(`${frontendBaseUrl}/prompts`);
});

test("localizes prompt UI while preserving prompt data and technical values", async ({ page }) => {
  await page.addInitScript(() => window.localStorage.setItem("tron.locale", "zh-CN"));
  await page.goto(`${frontendBaseUrl}/prompts`);

  await expect(page.getByRole("heading", { name: "提示词管理" })).toBeVisible();
  await expect(page.getByRole("button", { name: "全部提示词" })).toBeVisible();
  await expect(page.getByRole("button", { name: "筛选", exact: true })).toBeVisible();
  await expect(page.getByRole("columnheader", { name: "继承方式" })).toBeVisible();
  await expect(page.getByRole("columnheader", { name: "状态" })).toBeVisible();
  await expect(page.getByRole("columnheader", { name: "最后更新时间" })).toBeVisible();
  await expect(page.getByPlaceholder("搜索提示词...").first()).toBeVisible();

  const workspacePrompt = page.getByRole("row").filter({ hasText: "Workspace Router" });
  await expect(workspacePrompt).toBeVisible();
  await workspacePrompt.click();
  await expect(page).toHaveURL(/\/prompts\/detail\?promptCode=.*mode=override/);
  await expect(page.getByRole("heading", { name: "创建提示词覆盖" })).toBeVisible();

  await page.goto(`${frontendBaseUrl}/prompts`);
  await page.getByRole("button", { name: "Workspace Router 的操作" }).click();
  await expect(page.getByText("复制", { exact: true })).toHaveCount(0);
  await page.keyboard.press("Escape");

  await page.getByRole("button", { name: "新建", exact: true }).click();
  await expect(page.getByRole("heading", { name: "新建提示词" })).toBeVisible();
  await expect(page.getByRole("switch", { name: "已启用", exact: true })).toBeVisible();
  await expect(page.getByRole("switch", { name: "允许继承", exact: true })).toBeVisible();
  await page.getByLabel("名称").fill("Greeting Template");
  await page.getByLabel("提示词内容").fill("Hello {{name}}.");

  await expect(page.getByText("变量名", { exact: true }).first()).toBeVisible();
  await expect(page.getByRole("code").filter({ hasText: /^name$/ })).toBeVisible();
  await expect(page.getByText("STRING", { exact: true }).first()).toBeVisible();

  await page.goto(`${frontendBaseUrl}/settings/prompts`);
  await expect(page.getByRole("heading", { name: "平台提示词" })).toBeVisible();
  await expect(page.getByRole("button", { name: "新建", exact: true })).toBeVisible();
});
