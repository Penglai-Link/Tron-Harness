import { expect, test, type Page, type Route } from "@playwright/test";
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

let frontendBaseUrl = "";
let frontendLogs = "";
let frontendProcess: ManagedProcess | undefined;

const authenticatedSession = {
  authMethod: "internal",
  authenticated: true,
  provider: null,
  user: {
    displayName: "Resource Operator",
    email: "resource.operator@tron.ai",
    roleCodes: ["ADMIN", "WORKSPACE_ADMIN"],
    spaceCodes: ["TRON_CORE"],
    status: "Active",
    userCode: "USR-RESOURCE-0001",
    username: "resource.operator",
  },
};

const workspaceSwitcher = {
  primarySpaces: [
    {
      childCount: 1,
      description: "Global governance workspace",
      name: "Enterprise",
      status: "Active",
      workspaceCode: "ENTERPRISE",
    },
  ],
  secondarySpaces: [
    {
      childCount: 1,
      depth: 1,
      description: "Global governance workspace",
      effectiveUserCount: 1,
      label: "Enterprise",
      name: "Enterprise",
      primaryWorkspaceCode: "ENTERPRISE",
      status: "Active",
      workspaceCode: "ENTERPRISE",
    },
    {
      childCount: 1,
      depth: 2,
      description: "Platform workspace",
      effectiveUserCount: 1,
      label: "Enterprise / Platform",
      name: "Platform",
      primaryWorkspaceCode: "ENTERPRISE",
      status: "Active",
      workspaceCode: "PLATFORM",
    },
    {
      childCount: 0,
      depth: 3,
      description: "Tron product workspace",
      effectiveUserCount: 1,
      label: "Enterprise / Platform / Tron Core",
      name: "Tron Core",
      primaryWorkspaceCode: "ENTERPRISE",
      status: "Active",
      workspaceCode: "TRON_CORE",
    },
  ],
};

function successEnvelope(data: unknown) {
  return { code: "SUCCESS", data, message: "ok" };
}

async function fulfillJson(route: Route, data: unknown, status = 200) {
  await route.fulfill({
    body: JSON.stringify(data),
    contentType: "application/json",
    status,
  });
}

async function routeShellApi(page: Page) {
  await page.route("**/api/v1/**", async (route) => {
    const request = route.request();
    const pathname = new URL(request.url()).pathname;

    if (pathname === "/api/v1/auth/session" && request.method() === "GET") {
      return fulfillJson(route, successEnvelope(authenticatedSession));
    }
    if (pathname === "/api/v1/auth/refresh" && request.method() === "POST") {
      return fulfillJson(route, successEnvelope({
        accessToken: "resource-ui-token",
        expiresIn: 3600,
        session: authenticatedSession,
        tokenType: "Bearer",
      }));
    }
    if (pathname === "/api/v1/admin/workspaces/switcher" && request.method() === "GET") {
      return fulfillJson(route, successEnvelope(workspaceSwitcher));
    }

    return fulfillJson(route, {
      code: "NOT_FOUND",
      data: {},
      message: `${request.method()} ${pathname} is not mocked`,
    }, 404);
  });
}

async function seedShell(page: Page, locale: "en-US" | "zh-CN") {
  await page.addInitScript(({ selectedLocale, switcher }) => {
    window.localStorage.setItem("tron.locale", selectedLocale);
    window.localStorage.setItem("tron-app-shell", JSON.stringify({
      state: {
        openSections: { Data: true },
        selectedPrimarySpaceId: "ENTERPRISE",
        selectedSecondarySpaceId: "TRON_CORE",
        sidebarCollapsed: false,
        workspaceSwitcherSnapshot: switcher,
      },
      version: 4,
    }));
  }, { selectedLocale: locale, switcher: workspaceSwitcher });
}

async function openResourceDefinitions(page: Page, locale: "en-US" | "zh-CN" = "en-US") {
  await routeShellApi(page);
  await seedShell(page, locale);
  await page.goto(`${frontendBaseUrl}/resource-definitions`);
}

async function chooseFilter(page: Page, label: string, option: string) {
  await page.getByLabel(label).click();
  await page.getByRole("option", { exact: true, name: option }).click();
}

async function reservePort() {
  return new Promise<number>((resolvePort, rejectPort) => {
    const server = createServer();
    server.on("error", rejectPort);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      if (!address || typeof address === "string") {
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
      if ((await fetch(url)).ok) return;
    } catch {
      // The Vite server is still starting.
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
      env: { ...process.env, CI: "1" },
      stdio: "pipe",
    },
  );
  frontendProcess.stdout.on("data", (chunk: Buffer) => {
    frontendLogs = `${frontendLogs}${chunk.toString()}`.slice(-5_000);
  });
  frontendProcess.stderr.on("data", (chunk: Buffer) => {
    frontendLogs = `${frontendLogs}${chunk.toString()}`.slice(-5_000);
  });
  await waitForFrontend(`${frontendBaseUrl}/resource-definitions`);
});

test.afterAll(async () => {
  try {
    await stopManagedProcess(frontendProcess);
  } finally {
    frontendProcess = undefined;
  }
});

test("workspace resource definitions default to active versions and preserve source semantics", async ({ page }) => {
  await openResourceDefinitions(page);

  await expect(page.getByRole("heading", { name: "Resource Definitions" })).toBeVisible();
  await expect(page.getByRole("link", { name: "Resource Definitions", exact: true })).toHaveAttribute("aria-current", "page");
  await expect(page.getByRole("navigation", { name: "Resource management" })).toHaveCount(0);
  await expect(page.getByLabel("Filter resource definitions by lifecycle")).toContainText("Active");
  await expect(page.getByLabel("Filter resource definitions by validation result")).toContainText("All");
  await expect(page.locator("tbody tr")).toHaveCount(7);
  await expect(page.getByText(/versions visible/)).toHaveCount(0);
  await expect(page.getByLabel("Rows per page")).toBeVisible();
  await expect(page.getByLabel("Go to page")).toHaveValue("1");
  await expect(page.locator("thead th").last()).toHaveText("Actions");

  await page.getByLabel("Open actions for Product Images v2-P4d").click();
  await expect(page.getByRole("menuitem", { name: "View details" })).toBeVisible();
  await expect(page.getByRole("menuitem", { name: "Create draft" })).toBeVisible();
  await page.keyboard.press("Escape");

  await expect(page.locator("tbody tr").filter({ hasText: "v2-P4d" })).toContainText("LOCAL");
  await expect(page.locator("tbody tr").filter({ hasText: "v3-N8x" })).toContainText("INHERITED");
  await expect(page.locator("tbody tr").filter({ hasText: "v2-L6q" })).toContainText("OVERRIDE");
  await expect(page.getByText("v3-K7m", { exact: true })).toHaveCount(0);
});

test("search and filters operate on the approved version dimensions", async ({ page }) => {
  await openResourceDefinitions(page);

  await page.getByLabel("Search", { exact: true }).fill("v4-a7K");
  await expect(page.getByText("No resource definition versions match the current filters.")).toBeVisible();

  await page.getByLabel("Search", { exact: true }).fill("");
  await chooseFilter(page, "Filter resource definitions by lifecycle", "Draft");
  await chooseFilter(page, "Filter resource definitions by validation result", "Invalid");
  await expect(page.getByLabel("Filter resource definitions by validation result")).toContainText("Invalid");
  await expect(page.locator("tbody tr")).toHaveCount(2);
  await expect(page.locator("tbody tr").filter({ hasText: "v3-R2c" })).toContainText("WORKSPACE");
  await expect(page.locator("tbody tr").filter({ hasText: "v1-X5b" })).toContainText("WORKSPACE");

  await chooseFilter(page, "Filter resource definitions by lifecycle", "Active");
  await chooseFilter(page, "Filter resource definitions by validation result", "All");
  await chooseFilter(page, "Filter resource definitions by enabled state", "Disabled");
  await expect(page.locator("tbody tr")).toHaveCount(1);
  await expect(page.locator("tbody tr").first()).toContainText("REDIS_CONNECTION");
});

test("the Chinese layout remains readable without page-level horizontal overflow", async ({ page }) => {
  await page.setViewportSize({ height: 900, width: 1280 });
  await openResourceDefinitions(page, "zh-CN");

  await expect(page.getByRole("heading", { name: "资源定义" })).toBeVisible();
  await expect(page.getByLabel("按生命周期筛选资源定义")).toContainText("生效版本");
  await expect(page.getByRole("button", { name: "新建定义" })).toBeVisible();
  const tableScrollerWidths = await page.locator("table").evaluate((table) => ({
    clientWidth: table.parentElement?.clientWidth ?? 0,
    scrollWidth: table.parentElement?.scrollWidth ?? 0,
  }));
  expect(tableScrollerWidths.scrollWidth).toBeLessThanOrEqual(tableScrollerWidths.clientWidth);
  await expect.poll(async () => page.evaluate(() => (
    document.documentElement.scrollWidth <= document.documentElement.clientWidth
  ))).toBe(true);
});

test("the phone layout preserves row actions without horizontal table scrolling", async ({ page }) => {
  await page.setViewportSize({ height: 844, width: 390 });
  await openResourceDefinitions(page);

  await expect(page.locator("thead th").last()).toHaveText("Actions");
  await expect(page.getByLabel("Open actions for Product Images v2-P4d")).toBeVisible();
  const tableScrollerWidths = await page.locator("table").evaluate((table) => ({
    clientWidth: table.parentElement?.clientWidth ?? 0,
    scrollWidth: table.parentElement?.scrollWidth ?? 0,
  }));
  expect(tableScrollerWidths.scrollWidth).toBeLessThanOrEqual(tableScrollerWidths.clientWidth);
});
