import { expect, test, type Page } from "@playwright/test";

import {
  routeResourceUiShellApi,
  seedResourceUiShell,
  startResourceUiFrontend,
  stopResourceUiFrontend,
  type ResourceUiFrontend,
} from "./support/resource-ui-harness";

let frontend: ResourceUiFrontend | undefined;

test.describe.configure({ timeout: 60_000 });

async function openResources(page: Page, locale: "en-US" | "zh-CN" = "en-US") {
  await routeResourceUiShellApi(page);
  await seedResourceUiShell(page, locale);
  await page.goto(`${frontend!.baseUrl}/resources`);
  await page.waitForLoadState("networkidle");
  await expect(page.getByRole("heading", {
    exact: true,
    name: locale === "zh-CN" ? "资源" : "Resources",
  })).toBeVisible({ timeout: 15_000 });
}

async function chooseFilter(page: Page, label: string, option: string) {
  await page.getByLabel(label).click();
  await page.getByRole("option", { exact: true, name: option }).click();
}

test.beforeAll(async () => {
  frontend = await startResourceUiFrontend("/resources");
});

test.afterAll(async () => {
  await stopResourceUiFrontend(frontend);
  frontend = undefined;
});

test("workspace resources default to effective ACTIVE versions with an actions column", async ({ page }) => {
  await openResources(page);

  await expect(page.getByRole("heading", { name: "Resources", exact: true })).toBeVisible();
  await expect(page.getByRole("link", { name: "Resources", exact: true })).toHaveAttribute("aria-current", "page");
  await expect(page.getByRole("link", { name: "Resource Definitions", exact: true })).not.toHaveAttribute("aria-current", "page");
  await expect(page.getByRole("navigation", { name: "Resource management" })).toHaveCount(0);
  await expect(page.getByLabel("Filter resources by version status")).toContainText("Active");
  await expect(page.getByLabel("Filter resources by validation status")).toHaveCount(0);
  await expect(page.locator("tbody tr")).toHaveCount(7);
  await expect(page.getByText(/versions visible/)).toHaveCount(0);
  await expect(page.getByLabel("Rows per page")).toBeVisible();
  await expect(page.getByLabel("Go to page")).toHaveValue("1");
  await expect(page.locator("thead th").last()).toHaveText("Actions");

  await expect(page.locator("tbody tr").filter({ hasText: "v4-M7q" })).toContainText("LOCAL");
  await expect(page.locator("tbody tr").filter({ hasText: "v2-P9x" })).toContainText("INHERITED");
  await expect(page.locator("tbody tr").filter({ hasText: "v5-R2c" })).toContainText("OVERRIDE");
  await expect(page.locator("tbody tr").filter({ hasText: "v3-A8n" })).toContainText("PLATFORM");

  await page.getByLabel("Open actions for Storefront Product Gallery v4-M7q").click();
  await expect(page.getByRole("menuitem", { name: "View details" })).toBeVisible();
  await expect(page.getByRole("menuitem", { name: "Create draft" })).toHaveCount(0);
});

test("resource filters follow the lifecycle-specific source, scope, and validation rules", async ({ page }) => {
  await openResources(page);

  await page.getByLabel("Search", { exact: true }).fill("v4-M7q");
  await expect(page.getByText("No resource versions match the current filters.")).toBeVisible();
  await page.getByLabel("Search", { exact: true }).fill("");

  await chooseFilter(page, "Filter resources by resource type", "PDF Document");
  await expect(page.locator("tbody tr")).toHaveCount(1);
  await expect(page.locator("tbody tr").first()).toContainText("Knowledge Policy PDFs");
  await chooseFilter(page, "Filter resources by resource type", "All resource types");

  await chooseFilter(page, "Filter resources by version status", "Draft");
  await expect(page.getByLabel("Filter resources by source")).toHaveCount(0);
  await expect(page.getByLabel("Filter resources by owning scope")).toBeVisible();
  await expect(page.getByLabel("Filter resources by validation status")).toBeVisible();
  await expect(page.locator("thead th").filter({ hasText: "Owning Scope" })).toBeVisible();

  await chooseFilter(page, "Filter resources by validation status", "INVALID");
  await expect(page.locator("tbody tr")).toHaveCount(2);
  await expect(page.locator("tbody tr").filter({ hasText: "v1-Q2m" })).toContainText("WORKSPACE");
  await expect(page.locator("tbody tr").filter({ hasText: "v6-C4t" })).toContainText("WORKSPACE");

  await chooseFilter(page, "Filter resources by version status", "Historical");
  await expect(page.getByLabel("Filter resources by validation status")).toHaveCount(0);
  await expect(page.locator("tbody tr")).toHaveCount(4);
});

test("the Chinese resource list aligns with the shared filter and table layout", async ({ page }) => {
  await page.setViewportSize({ height: 900, width: 1280 });
  await openResources(page, "zh-CN");

  await expect(page.getByRole("heading", { name: "资源", exact: true })).toBeVisible();
  await expect(page.getByLabel("按版本状态筛选资源")).toContainText("生效版本");
  await expect(page.getByRole("button", { name: "新建资源", exact: true })).toBeVisible();
  const tableScrollerWidths = await page.locator("table").evaluate((table) => ({
    clientWidth: table.parentElement?.clientWidth ?? 0,
    scrollWidth: table.parentElement?.scrollWidth ?? 0,
  }));
  expect(tableScrollerWidths.scrollWidth).toBeLessThanOrEqual(tableScrollerWidths.clientWidth);
  await expect.poll(async () => page.evaluate(() => (
    document.documentElement.scrollWidth <= document.documentElement.clientWidth
  ))).toBe(true);
});

test("the compact desktop list moves Resource Type into the primary cell without horizontal scrolling", async ({ page }) => {
  await page.setViewportSize({ height: 900, width: 768 });
  await openResources(page);

  const firstRow = page.locator("tbody tr").filter({ hasText: "v4-M7q" });
  await expect(firstRow).toContainText("Product Images");
  const tableScrollerWidths = await page.locator("table").evaluate((table) => ({
    clientWidth: table.parentElement?.clientWidth ?? 0,
    scrollWidth: table.parentElement?.scrollWidth ?? 0,
  }));
  expect(tableScrollerWidths.scrollWidth).toBeLessThanOrEqual(tableScrollerWidths.clientWidth);
});

test("the phone resource list keeps source context and row actions without horizontal scrolling", async ({ page }) => {
  await page.setViewportSize({ height: 844, width: 390 });
  await openResources(page);

  await expect(page.locator("thead th").last()).toHaveText("Actions");
  await expect(page.getByLabel("Open actions for Storefront Product Gallery v4-M7q")).toBeVisible();
  await expect(page.locator("tbody tr").filter({ hasText: "v4-M7q" })).toContainText("LOCAL");
  const tableScrollerWidths = await page.locator("table").evaluate((table) => ({
    clientWidth: table.parentElement?.clientWidth ?? 0,
    scrollWidth: table.parentElement?.scrollWidth ?? 0,
  }));
  expect(tableScrollerWidths.scrollWidth).toBeLessThanOrEqual(tableScrollerWidths.clientWidth);
  await expect.poll(async () => page.evaluate(() => (
    document.documentElement.scrollWidth <= document.documentElement.clientWidth
  ))).toBe(true);
});
