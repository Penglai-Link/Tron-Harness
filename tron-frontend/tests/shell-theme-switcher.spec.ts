import lightSkin from "../src/shared/theme/builtin/light.json" with { type: "json" };
import darkSkin from "../src/shared/theme/builtin/matrix-dark.json" with { type: "json" };
import { routeActiveProfileApi } from "./support/resource-ui-harness";
import {
  expect,
  test,
  type Page,
  type Route,
} from "@playwright/test";
import {
  spawnManagedProcess,
  stopManagedProcess,
  type ManagedProcess,
} from "./support/managed-process";
import { createServer } from "node:net";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { getWorkspaceSwitcherMock } from "../src/shared/constants/workspaceSpaces";

test.describe.configure({ timeout: 45_000 });

const thisFilePath = fileURLToPath(import.meta.url);
const frontendRoot = resolve(thisFilePath, "..", "..");
const themeStorageKey = "tron.theme";

let frontendProcess: ManagedProcess | undefined;
let frontendLogs = "";
let frontendBaseUrl = "";

const authUser = {
  authMethod: "internal",
  authenticated: true,
  provider: null,
  user: {
    displayName: "Auth Operator",
    email: "auth.operator@tron.ai",
    roleCodes: ["ADMIN", "DEVELOPER"],
    spaceCodes: ["PLATFORM", "TRON_CORE"],
    status: "Active",
    userCode: "USR-AUTH-0001",
    username: "auth.operator",
  },
};

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

      const { port } = address;
      server.close(() => resolvePort(port));
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
      if (response.ok) return;
    } catch (error) {
      lastError = error;
    }

    await delay(250);
  }

  throw new Error(`Frontend dev server did not become ready: ${String(lastError)}\n${frontendLogs}`);
}

function successEnvelope(data: unknown) {
  return {
    code: "SUCCESS",
    message: "ok",
    data,
  };
}

function unauthorizedEnvelope(code = "AUTH_UNAUTHORIZED", message = "authentication required") {
  return {
    code,
    message,
    data: {},
  };
}

async function fulfillJson(route: Route, data: unknown, status = 200) {
  await route.fulfill({
    status,
    contentType: "application/json",
    body: JSON.stringify(data),
  });
}

async function routeThemeApi(
  page: Page,
  options: {
    workspaceSwitcherData?: ReturnType<typeof getWorkspaceSwitcherMock>;
  } = {},
) {
  let activeSkin = lightSkin;
  await page.route("**/api/v1/system/skins", route => fulfillJson(route, successEnvelope([lightSkin, darkSkin])));
  await page.route("**/api/v1/system/skins/active", route => {
    if (route.request().method() === "PUT") activeSkin = route.request().postDataJSON().id === darkSkin.id ? darkSkin : lightSkin;
    return fulfillJson(route, successEnvelope({ skin: activeSkin }));
  });
  await page.route("**/api/v1/chats/session-groups?*", route => fulfillJson(route, successEnvelope([])));
  const data = options.workspaceSwitcherData ?? getWorkspaceSwitcherMock();
  await page.route("**/api/v1/admin/workspaces/switcher", route => fulfillJson(route, successEnvelope(data)));
  await page.route("**/api/v1/admin/platform/models", route => fulfillJson(route, successEnvelope([])));
}

async function signIn(page: Page) {
  await routeActiveProfileApi(page);
  await page.goto(`${frontendBaseUrl}/apps`);
  await expect(page).toHaveURL(/\/apps$/);
}

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
        VITE_TRON_API_BASE_URL: "",
        VITE_TRON_DATA_SOURCE: "api",
      },
      stdio: "pipe",
    },
  );

  frontendProcess.stdout.on("data", appendFrontendLog);
  frontendProcess.stderr.on("data", appendFrontendLog);

  await waitForFrontend(`${frontendBaseUrl}/login`);
});

test.afterAll(async () => {
  try {
    await stopManagedProcess(frontendProcess);
  } finally {
    frontendProcess = undefined;
  }
});

test("opens the full-screen settings page, switches settings sections, and persists theme changes", async ({ page }) => {
  await routeThemeApi(page);
  await signIn(page);

  const header = page.getByRole("banner");
  await expect(header.getByRole("button", { name: /open filters/i })).toHaveCount(0);
  await page.getByRole("button", { name: "Open user menu" }).click();
  await page.getByRole("button", { name: "Open system settings", exact: true }).click();
  await expect(page).toHaveURL(/\/settings\/general$/);
  await expect(page.locator("#tron-workspace-trigger")).toHaveCount(0);
  await expect(page.getByRole("link", { name: "Models" })).toBeVisible();

  await page.getByRole("link", { name: "Appearance", exact: true }).click();
  await expect(page).toHaveURL(/\/settings\/appearance$/);
  await page.getByRole("button", { name: "Matrix Dark", exact: true }).click();
  await expect.poll(() => page.evaluate(() => document.documentElement.dataset.theme)).toBe("matrix-dark");
  await expect.poll(() => page.evaluate(key => localStorage.getItem(key), themeStorageKey)).toBe("matrix-dark");
  const darkBackground = await page.evaluate(() => getComputedStyle(document.body).backgroundColor);
  await page.getByRole("link", { name: "Models", exact: true }).click();
  await expect(page).toHaveURL(/\/settings\/models$/);
  await page.getByRole("link", { name: "Appearance", exact: true }).click();
  await page.reload();
  await expect(page.getByRole("button", { name: "Matrix Dark", exact: true })).toHaveAttribute("aria-pressed", "true");
  await page.getByRole("button", { name: "Light", exact: true }).click();
  await expect.poll(() => page.evaluate(() => document.documentElement.dataset.theme)).toBe("light");
  await expect.poll(() => page.evaluate(key => localStorage.getItem(key), themeStorageKey)).toBe("light");
  await expect.poll(() => page.evaluate(() => getComputedStyle(document.body).backgroundColor)).not.toBe(darkBackground);

});

test("loads workspace selector data from api after sign in", async ({ page }) => {
  const workspaceSwitcherData = {
    primarySpaces: [
      {
        workspaceCode: "INDUSTRIAL",
        name: "工业互联网",
        description: "工业互联网总部空间",
        status: "Active" as const,
        childCount: 1,
      },
    ],
    secondarySpaces: [
      {
        workspaceCode: "INDUSTRIAL",
        primaryWorkspaceCode: "INDUSTRIAL",
        parentWorkspaceCode: null,
        label: "工业互联网",
        name: "工业互联网",
        description: "工业互联网总部空间",
        status: "Active" as const,
        depth: 1,
        childCount: 1,
        effectiveUserCount: 0,
      },
      {
        workspaceCode: "ELITEFORGE",
        primaryWorkspaceCode: "INDUSTRIAL",
        parentWorkspaceCode: "INDUSTRIAL",
        label: "工业互联网 / 璀璨工坊",
        name: "璀璨工坊",
        description: "私有云环境",
        status: "Active" as const,
        depth: 2,
        childCount: 1,
        effectiveUserCount: 0,
      },
    ],
  };

  await routeThemeApi(page, { workspaceSwitcherData });
  await page.addInitScript((snapshot) => {
    window.localStorage.setItem("tron-app-shell", JSON.stringify({
      state: {
        openSections: { Core: true },
        selectedPrimarySpaceId: "ENTERPRISE",
        selectedSecondarySpaceId: "TRON_CORE",
        sidebarCollapsed: false,
        workspaceSwitcherSnapshot: snapshot,
      },
      version: 0,
    }));
  }, getWorkspaceSwitcherMock());
  await signIn(page);

  const workspaceTrigger = page.locator("#tron-workspace-trigger");
  await expect(workspaceTrigger).toContainText("工业互联网");
  await expect(workspaceTrigger).not.toContainText("Enterprise/Platform/Tron Core");
  await workspaceTrigger.click();
  const workspaceSelector = page.locator("#tron-workspace-selector-panel");
  await expect(workspaceSelector).toBeVisible();
  await workspaceSelector.getByRole("button", { name: "工业互联网/璀璨工坊", exact: true }).click();
  await expect(workspaceTrigger).toContainText("工业互联网/璀璨工坊");
});

test("workspace inheritance map keeps descendant workspaces nested under their direct parent", async ({ page }) => {
  const workspaceSwitcherData = {
    primarySpaces: [
      {
        workspaceCode: "INDUSTRIAL",
        name: "工业互联网",
        description: "工业互联网总部空间",
        status: "Active" as const,
        childCount: 1,
      },
    ],
    secondarySpaces: [
      {
        workspaceCode: "ELITEFORGE",
        primaryWorkspaceCode: "INDUSTRIAL",
        parentWorkspaceCode: "INDUSTRIAL",
        label: "工业互联网 / 璀璨工坊",
        name: "璀璨工坊",
        description: "私有云环境",
        status: "Active" as const,
        depth: 2,
        childCount: 1,
        effectiveUserCount: 0,
      },
      {
        workspaceCode: "ONLINE_DOCS",
        primaryWorkspaceCode: "INDUSTRIAL",
        parentWorkspaceCode: "ELITEFORGE",
        label: "工业互联网 / 璀璨工坊 / 在线文档",
        name: "在线文档",
        description: "知识协同工作区",
        status: "Active" as const,
        depth: 3,
        childCount: 0,
        effectiveUserCount: 0,
      },
    ],
  };

  await routeThemeApi(page, { workspaceSwitcherData });
  await page.addInitScript((snapshot) => {
    window.localStorage.setItem("tron-app-shell", JSON.stringify({
      state: {
        openSections: { Core: true },
        selectedPrimarySpaceId: "INDUSTRIAL",
        selectedSecondarySpaceId: "ONLINE_DOCS",
        sidebarCollapsed: false,
        workspaceSwitcherSnapshot: snapshot,
      },
      version: 0,
    }));
  }, workspaceSwitcherData);
  await signIn(page);

  await page.locator("#tron-workspace-trigger").click();
  const selector = page.locator("#tron-workspace-selector-panel");
  await selector.getByRole("button", { name: "Expand 工业互联网/璀璨工坊", exact: true }).click();
  const descendant = selector.getByRole("button", { name: "工业互联网/璀璨工坊/在线文档", exact: true });
  await expect(descendant).toBeVisible();
  await expect(selector.getByRole("button", { name: "工业互联网/璀璨工坊", exact: true })).toBeVisible();
  await descendant.click();
  await expect(page.locator("#tron-workspace-trigger")).toContainText("工业互联网/璀璨工坊/在线文档");

});

test("sidebar groups knowledge and resource navigation under data", async ({ page }) => {
  await routeThemeApi(page);
  await signIn(page);

  const navigation = page.getByRole("navigation", { name: "Tron primary navigation" });
  const dataToggle = navigation.getByRole("button", { name: "Data" });
  await dataToggle.click();
  const dataSection = dataToggle.locator("xpath=ancestor::*[@data-slot='sidebar-group'][1]");
  const agentOperationsSection = navigation.getByRole("button", { name: "Agent Operations" }).locator("xpath=ancestor::*[@data-slot='sidebar-group'][1]");

  await expect(dataSection.getByRole("link", { name: "KnowledgeGraph" })).toBeVisible();
  await expect(dataSection.getByRole("link", { name: "KnowledgeBase" })).toBeVisible();
  await dataSection.getByRole("button", { name: "Expand Resource Management submenu" }).click();
  await expect(dataSection.getByRole("link", { name: "File Management", exact: true })).toBeVisible();
  await expect(agentOperationsSection.getByRole("link", { name: "File Management" })).toHaveCount(0);

  await dataSection.getByRole("link", { name: "KnowledgeBase" }).click();
  await expect(page).toHaveURL(/\/knowledge-base$/);
  await expect(page.getByRole("tab", { name: "KnowledgeBase", exact: true })).toBeVisible();
});

test("sidebar navigation still works after entering chats", async ({ page }) => {
  await routeThemeApi(page);
  await signIn(page);

  await routeActiveProfileApi(page);
  await page.goto(`${frontendBaseUrl}/chats`, { waitUntil: "domcontentloaded" });
  await expect(page).toHaveURL(/\/(?:chats|default)(?:\?.*)?$/);
  await expect(page.getByRole("button", { name: "New chat", exact: true }).first()).toBeVisible();

  await page.getByRole("link", { name: "Apps" }).click();
  await expect(page).toHaveURL(/\/apps$/);
  await expect(page.getByRole("tab", { name: "Apps", exact: true })).toBeVisible();

  const dataToggle = page.getByRole("button", { name: "Data" });
  const resourcesLink = page.getByRole("link", { name: "File Management" });
  if (!(await resourcesLink.isVisible())) {
    if (await dataToggle.getAttribute("aria-expanded") !== "true") await dataToggle.click();
    await page.getByRole("button", { name: "Expand Resource Management submenu" }).click();
  }
  await resourcesLink.click();
  await expect(page).toHaveURL(/\/resource-management\/files$/);
  await expect(page.getByRole("tab", { name: "File Management", exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Primary Investigation" })).toHaveCount(0);
});
