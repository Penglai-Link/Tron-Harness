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
let frontendLogs = "";
let frontendBaseUrl = "";

test.describe.configure({ timeout: 45_000 });

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

  while (Date.now() < deadline) {
    if (frontendProcess?.exitCode !== null) {
      throw new Error(`Frontend dev server exited early.\n${frontendLogs}`);
    }

    try {
      const response = await fetch(url);
      if (response.ok) return;
    } catch {
      // The development server is still starting.
    }

    await new Promise((resolveDelay) => setTimeout(resolveDelay, 250));
  }

  throw new Error(`Frontend dev server did not become ready.\n${frontendLogs}`);
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
        VITE_TRON_AUTH_DATA_SOURCE: "mock",
        VITE_TRON_DATA_SOURCE: "mock",
        VITE_TRON_MEMORY_DATA_SOURCE: "mock",
      },
      stdio: "pipe",
    },
  );
  frontendProcess.stdout.on("data", appendFrontendLog);
  frontendProcess.stderr.on("data", appendFrontendLog);

  await waitForFrontend(`${frontendBaseUrl}/memory`);
});

test.afterAll(async () => {
  try {
    await stopManagedProcess(frontendProcess);
  } finally {
    frontendProcess = undefined;
  }
});

test("memory rows preserve line breaks in a bounded plain-text preview", async ({ page }) => {
  const title = "长标题用于验证记忆列表不会被超长内容撑开并且始终保持单行展示";
  const text = [
    "第一行：**Markdown 标记保持原样**。",
    "第二行：<strong>HTML 标记也保持原样</strong>。",
    "第三行：该行仍在完整数据中，但不会继续撑高列表。",
  ].join("\n");

  await page.setViewportSize({ width: 1440, height: 900 });
  await routeActiveProfileApi(page);
  await page.goto(`${frontendBaseUrl}/memory/new`);
  await page.getByLabel("Title").fill(title);
  await page.locator('[data-slot="field"]', { hasText: "Memory text" }).locator("textarea").fill(text);
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expect(page).toHaveURL(/\/memory\/details\?memoryId=/);

  await page
    .getByRole("navigation", { name: "Breadcrumb" })
    .getByRole("link", { name: "Memory", exact: true })
    .click();
  await expect(page).toHaveURL(/\/memory$/);
  const titleButton = page.getByRole("button", { name: title, exact: true });
  const row = page.getByRole("row").filter({ has: titleButton });
  const preview = row.getByTestId("memory-record-text");

  await expect(row).toBeVisible();
  expect(await preview.evaluate((element) => element.textContent)).toBe(text);
  await expect(row.locator("strong")).toHaveCount(0);

  const previewLayout = await preview.evaluate((element) => {
    const style = getComputedStyle(element);
    return {
      height: element.getBoundingClientRect().height,
      lineHeight: Number.parseFloat(style.lineHeight),
      lineClamp: style.webkitLineClamp,
      overflow: style.overflow,
      whiteSpace: style.whiteSpace,
    };
  });

  expect(previewLayout.whiteSpace).toBe("pre-wrap");
  expect(previewLayout.lineClamp).toBe("2");
  expect(previewLayout.overflow).toBe("hidden");
  expect(previewLayout.height).toBeLessThanOrEqual(previewLayout.lineHeight * 2 + 1);

  const titleLayout = await titleButton.evaluate((element) => {
    const style = getComputedStyle(element);
    return {
      overflow: style.overflow,
      textOverflow: style.textOverflow,
      whiteSpace: style.whiteSpace,
    };
  });
  expect(titleLayout).toEqual({
    overflow: "hidden",
    textOverflow: "ellipsis",
    whiteSpace: "nowrap",
  });

  await titleButton.click();
  await expect(page).toHaveURL(/\/memory\/details\?memoryId=/);
  await expect(page.getByText(text, { exact: true })).toBeVisible();
});
