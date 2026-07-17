import { expect, test, type Page } from "@playwright/test";

import {
  routeResourceUiShellApi,
  seedResourceUiShell,
  startResourceUiFrontend,
  stopResourceUiFrontend,
  type ResourceUiFrontend,
} from "./support/resource-ui-harness";

let frontend: ResourceUiFrontend | undefined;

async function openPlatformDefinitions(page: Page, locale: "en-US" | "zh-CN" = "en-US") {
  await routeResourceUiShellApi(page);
  await seedResourceUiShell(page, locale);
  await page.goto(`${frontend!.baseUrl}/settings/resource-definitions`);
  await expect(page.getByRole("heading", { exact: true, name: locale === "zh-CN" ? "平台资源定义" : "Platform Resource Definitions" })).toBeVisible();
}

async function chooseFilter(page: Page, label: string, option: string) {
  await page.getByLabel(label).click();
  await page.getByRole("option", { exact: true, name: option }).click();
}

test.beforeAll(async () => {
  frontend = await startResourceUiFrontend("/settings/resource-definitions");
});

test.afterAll(async () => {
  await stopResourceUiFrontend(frontend);
  frontend = undefined;
});

test("platform definitions use their own version data and shared table controls", async ({ page }) => {
  await openPlatformDefinitions(page);

  await expect(page.getByRole("link", { exact: true, name: "Resource Definitions" })).toHaveAttribute("aria-current", "page");
  await expect(page.getByRole("link", { exact: true, name: "Resources" })).not.toHaveAttribute("aria-current", "page");
  await expect(page.locator("tbody tr")).toHaveCount(4);
  await expect(page.locator("tbody tr").filter({ hasText: "POSTGRESQL_CONNECTION" })).toContainText("PLATFORM");
  await expect(page.getByText("INHERITED", { exact: true })).toHaveCount(0);
  await expect(page.locator("thead th").last()).toHaveText("Actions");
  await expect(page.getByLabel("Rows per page")).toBeVisible();
  await expect(page.getByLabel("Go to page")).toHaveValue("1");
});

test("platform filters expose draft validation without Workspace projections", async ({ page }) => {
  await openPlatformDefinitions(page);

  await chooseFilter(page, "Filter resource definitions by lifecycle", "Draft");
  await expect(page.locator("tbody tr")).toHaveCount(1);
  await expect(page.locator("tbody tr").first()).toContainText("KAFKA_CONNECTION");
  await expect(page.locator("tbody tr").first()).toContainText("INVALID");
  await page.getByLabel("Search").fill("PostgreSQL");
  await expect(page.getByText("No resource definition versions match the current filters.")).toBeVisible();
});

test("the platform detail reuses the JSONJoy-based Draft 7 editor", async ({ page }) => {
  await routeResourceUiShellApi(page);
  await seedResourceUiShell(page);
  await page.goto(`${frontend!.baseUrl}/settings/resource-definitions/detail?definitionCode=MYSQL_CONNECTION&definitionVersion=v4-a7K`);

  await expect(page.getByRole("heading", { exact: true, name: "MySQL Connection" })).toBeVisible();
  await expect(page.getByText("Platform resource definitions", { exact: true })).toBeVisible();
  await expect(page.getByText("PLATFORM", { exact: true }).first()).toBeVisible();
  await expect(page.getByLabel("Attributes Schema JSON")).toHaveValue(/"credentials"/);
  await expect(page.getByRole("tab", { name: "Structured attributes 6" })).toBeVisible();

  await page.getByRole("button", { exact: true, name: "Edit" }).click();
  await expect(page.getByText("DRAFT", { exact: true }).first()).toBeVisible();
  await expect(page.getByText("PLATFORM", { exact: true }).first()).toBeVisible();
  await expect(page.getByRole("button", { exact: true, name: "Save draft" })).toBeVisible();
});

test("a new platform definition starts as an unpublishable platform draft", async ({ page }) => {
  await routeResourceUiShellApi(page);
  await seedResourceUiShell(page);
  await page.goto(`${frontend!.baseUrl}/settings/resource-definitions/new`);

  await expect(page.getByLabel("Definition name")).toHaveValue("");
  await expect(page.getByLabel("Definition code")).toHaveValue("");
  await expect(page.getByText("PLATFORM", { exact: true }).first()).toBeVisible();
  await expect(page.getByRole("button", { exact: true, name: "Publish" })).toBeDisabled();
});

test("the Chinese phone list keeps settings navigation and the actions column without overflow", async ({ page }) => {
  await page.setViewportSize({ height: 844, width: 390 });
  await openPlatformDefinitions(page, "zh-CN");

  await expect(page.getByRole("button", { exact: true, name: "新建定义" })).toBeVisible();
  await expect(page.locator("thead th").last()).toHaveText("操作");
  await expect(page.getByLabel("打开 MySQL Connection v4-a7K 的操作菜单")).toBeVisible();
  await expect.poll(async () => page.evaluate(() => (
    document.documentElement.scrollWidth <= document.documentElement.clientWidth
  ))).toBe(true);
});
