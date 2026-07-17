import { expect, test, type Page } from "@playwright/test";

import {
  routeResourceUiShellApi,
  seedResourceUiShell,
  startResourceUiFrontend,
  stopResourceUiFrontend,
  type ResourceUiFrontend,
} from "./support/resource-ui-harness";

let frontend: ResourceUiFrontend | undefined;

async function openPlatformResources(page: Page, locale: "en-US" | "zh-CN" = "en-US") {
  await routeResourceUiShellApi(page);
  await seedResourceUiShell(page, locale);
  await page.goto(`${frontend!.baseUrl}/settings/resources`);
  await expect(page.getByRole("heading", { exact: true, name: locale === "zh-CN" ? "平台资源" : "Platform Resources" })).toBeVisible();
}

async function chooseFilter(page: Page, label: string, option: string) {
  await page.getByLabel(label).click();
  await page.getByRole("option", { exact: true, name: option }).click();
}

test.beforeAll(async () => {
  frontend = await startResourceUiFrontend("/settings/resources");
});

test.afterAll(async () => {
  await stopResourceUiFrontend(frontend);
  frontend = undefined;
});

test("platform resources use platform-owned versions and the shared table contract", async ({ page }) => {
  await openPlatformResources(page);

  await expect(page.getByRole("link", { exact: true, name: "Resources" })).toHaveAttribute("aria-current", "page");
  await expect(page.getByRole("link", { exact: true, name: "Resource Definitions" })).not.toHaveAttribute("aria-current", "page");
  await expect(page.locator("tbody tr")).toHaveCount(4);
  await expect(page.locator("tbody tr").filter({ hasText: "analytics-warehouse" })).toContainText("PLATFORM");
  await expect(page.getByText("INHERITED", { exact: true })).toHaveCount(0);
  await expect(page.locator("thead th").last()).toHaveText("Actions");
  await expect(page.getByLabel("Rows per page")).toBeVisible();
});

test("platform lifecycle filters keep only meaningful platform source options", async ({ page }) => {
  await openPlatformResources(page);

  await page.getByLabel("Filter resources by source").click();
  await expect(page.getByRole("option", { exact: true, name: "PLATFORM" })).toBeVisible();
  await expect(page.getByRole("option", { exact: true, name: "LOCAL" })).toHaveCount(0);
  await page.keyboard.press("Escape");

  await chooseFilter(page, "Filter resources by version status", "Draft");
  await expect(page.locator("tbody tr")).toHaveCount(1);
  await expect(page.locator("tbody tr").first()).toContainText("Support Bundle");
  await expect(page.locator("tbody tr").first()).toContainText("INVALID");
  await expect(page.getByLabel("Filter resources by validation status")).toBeVisible();
});

test("platform resource detail keeps Schema fields left and binary data right", async ({ page }) => {
  await routeResourceUiShellApi(page);
  await seedResourceUiShell(page);
  await page.goto(`${frontend!.baseUrl}/settings/resources/detail?resourceCode=customer-import-templates&resourceVersion=v2-P9x`);

  await expect(page.getByRole("heading", { exact: true, name: "Customer Import Templates" })).toBeVisible();
  await expect(page.getByText("Platform resources", { exact: true })).toBeVisible();
  await expect(page.getByLabel("File name")).toHaveValue("customer-import-template.xlsx");
  await expect(page.getByText("Binary data", { exact: true })).toBeVisible();
  await expect(page.getByText("customer-import-template.xlsx", { exact: true })).toBeVisible();
  await expect(page.getByText(/MIME|content type/i)).toHaveCount(0);
  await expect(page.getByRole("switch", { name: "Allow inheritance" })).toBeEnabled();

  const formBox = await page.locator('[role="tabpanel"] > div > section').boundingBox();
  const binaryBox = await page.locator('[role="tabpanel"] > div > aside').boundingBox();
  expect(formBox).not.toBeNull();
  expect(binaryBox).not.toBeNull();
  expect(binaryBox!.x).toBeGreaterThan(formBox!.x + formBox!.width);
});

test("editing a platform resource creates a persistent platform draft", async ({ page }) => {
  await routeResourceUiShellApi(page);
  await seedResourceUiShell(page);
  await page.goto(`${frontend!.baseUrl}/settings/resources/detail?resourceCode=customer-import-templates&resourceVersion=v2-P9x`);

  await page.getByRole("button", { exact: true, name: "Edit" }).click();
  await expect(page.getByText("DRAFT", { exact: true }).first()).toBeVisible();
  await expect(page.getByText("PLATFORM", { exact: true }).first()).toBeVisible();
  await expect(page.getByRole("button", { exact: true, name: "Save draft" })).toBeVisible();
  await expect(page.locator('[aria-label="Replace customer-import-template.xlsx"]')).toBeVisible();
});

test("a new platform Resource binds an active platform definition", async ({ page }) => {
  await routeResourceUiShellApi(page);
  await seedResourceUiShell(page);
  await page.goto(`${frontend!.baseUrl}/settings/resources/new`);

  await expect(page.getByLabel("Resource name")).toHaveValue("");
  await expect(page.getByText("PLATFORM", { exact: true }).first()).toBeVisible();
  await page.getByLabel("Resource type").click();
  await page.getByRole("option", { name: /File · v1-F8s/ }).click();
  await expect(page.getByLabel("File name")).toBeVisible();
  await expect(page.getByText("Binary data", { exact: true })).toBeVisible();
  await expect(page.getByRole("button", { exact: true, name: "Publish" })).toBeDisabled();
});

test("the Chinese phone pages keep the active settings entry and avoid horizontal overflow", async ({ page }) => {
  await page.setViewportSize({ height: 844, width: 390 });
  await openPlatformResources(page, "zh-CN");

  await expect(page.getByRole("link", { exact: true, name: "资源" })).toHaveAttribute("aria-current", "page");
  await expect(page.locator("thead th").last()).toHaveText("操作");
  await expect.poll(async () => page.evaluate(() => (
    document.documentElement.scrollWidth <= document.documentElement.clientWidth
  ))).toBe(true);

  await page.goto(`${frontend!.baseUrl}/settings/resources/detail?resourceCode=customer-import-templates&resourceVersion=v2-P9x`);
  await expect(page.getByRole("heading", { exact: true, name: "Customer Import Templates" })).toBeVisible();
  const formBox = await page.locator('[role="tabpanel"] > div > section').boundingBox();
  const binaryBox = await page.locator('[role="tabpanel"] > div > aside').boundingBox();
  expect(binaryBox!.y).toBeGreaterThan(formBox!.y + formBox!.height);
  await expect.poll(async () => page.evaluate(() => (
    document.documentElement.scrollWidth <= document.documentElement.clientWidth
  ))).toBe(true);
});
