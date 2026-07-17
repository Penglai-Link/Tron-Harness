import { expect, test, type Page } from "@playwright/test";

import {
  routeResourceUiShellApi,
  seedResourceUiShell,
  startResourceUiFrontend,
  stopResourceUiFrontend,
  type ResourceUiFrontend,
} from "./support/resource-ui-harness";

let frontend: ResourceUiFrontend | undefined;

const draftPath = "/resources/detail?resourceCode=storefront-product-gallery&resourceVersion=v5-X7b";

async function openEditor(
  page: Page,
  locale: "en-US" | "zh-CN" = "en-US",
  path = draftPath,
  heading = "Storefront Product Gallery",
) {
  await routeResourceUiShellApi(page);
  await seedResourceUiShell(page, locale);
  await page.goto(`${frontend!.baseUrl}${path}`);
  await expect(page.getByRole("heading", { exact: true, name: heading })).toBeVisible();
}

test.beforeAll(async () => {
  frontend = await startResourceUiFrontend(draftPath);
});

test.afterAll(async () => {
  await stopResourceUiFrontend(frontend);
  frontend = undefined;
});

test("a Resource draft renders structured attributes from Draft 7 and manages opaque binaries", async ({ page }) => {
  await openEditor(page);

  await expect(page.getByText("DRAFT", { exact: true }).first()).toBeVisible();
  await expect(page.getByRole("tab", { name: "Resource data" })).toHaveAttribute("aria-selected", "true");
  await expect(page.getByLabel("Product code")).toHaveValue("TRON-STOREFRONT");
  await expect(page.getByLabel("Image type")).toContainText("GALLERY");
  await page.getByLabel("Alternative text").fill("Updated from the generated form");
  await expect(page.getByLabel("Alternative text")).toHaveValue("Updated from the generated form");

  await expect(page.getByRole("heading", { name: "Binary data" })).toBeVisible();
  await expect(page.getByText("4 / 12", { exact: true })).toBeVisible();
  await expect(page.getByText(/MIME|content type/i)).toHaveCount(0);

  const uploadInput = page.locator('input[type="file"]').first();
  await uploadInput.setInputFiles({
    buffer: Buffer.from("opaque resource bytes"),
    mimeType: "application/octet-stream",
    name: "hero-zoom.png",
  });
  await expect(page.getByLabel("Binary name")).toHaveValue("hero-zoom.png");
  await page.getByRole("button", { exact: true, name: "Upload" }).click();
  await expect(page.getByText("hero-zoom.png", { exact: true })).toBeVisible();
  await page.getByLabel("Remove hero-zoom.png").click();
  await expect(page.getByText("hero-zoom.png", { exact: true })).toHaveCount(0);
});

test("the same schema renderer handles nested MySQL data, Variables, and extension properties", async ({ page }) => {
  await openEditor(
    page,
    "en-US",
    "/resources/detail?resourceCode=production-mysql&resourceVersion=v3-A8n",
    "Production MySQL",
  );

  await expect(page.getByText("ACTIVE", { exact: true }).first()).toBeVisible();
  await expect(page.getByLabel("Connection mode")).toContainText("REPLICATION");
  await expect(page.getByLabel("Host")).toHaveCount(2);
  await expect(page.getByLabel("Port")).toHaveCount(2);
  await expect(page.getByLabel("Username variable")).toContainText("DATABASE_USERNAME");
  await expect(page.getByLabel("Password variable")).toContainText("DATABASE_PASSWORD");
  await expect(page.getByText("Driver properties", { exact: true })).toBeVisible();
  await expect(page.getByText("connectTimeout", { exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Binary data" })).toHaveCount(0);
  await expect(page.getByText("Advanced JSON", { exact: true })).toHaveCount(0);
});

test("version history switches the whole snapshot and exposes lifecycle actions", async ({ page }) => {
  await openEditor(page);

  await page.getByRole("tab", { name: "Version history 3" }).click();
  await expect(page.locator("tbody tr")).toHaveCount(3);
  await expect(page.locator("thead th").last()).toHaveText("Actions");

  await page.locator("tbody tr").filter({ hasText: "v3-V5d" }).click();
  await expect(page.getByText("HISTORICAL", { exact: true }).first()).toBeVisible();
  await expect(page.getByRole("button", { exact: true, name: "Activate" })).toBeDisabled();

  await page.getByRole("tab", { name: "Version history 3" }).click();
  await page.getByLabel("Open actions for v5-X7b").click();
  await page.getByRole("menuitem", { exact: true, name: "Publish" }).click();
  await expect(page.getByText("ACTIVE", { exact: true }).first()).toBeVisible();
  await expect(page.getByRole("button", { exact: true, name: "Edit" })).toBeVisible();
});

test("a new Resource selects an active definition before the generated form appears", async ({ page }) => {
  await routeResourceUiShellApi(page);
  await seedResourceUiShell(page);
  await page.goto(`${frontend!.baseUrl}/resources/new`);

  await expect(page.getByLabel("Resource name")).toHaveValue("");
  await expect(page.getByLabel("Resource code")).toHaveValue("");
  await expect(page.getByText("Select a resource type to configure its data.", { exact: true })).toBeVisible();
  await expect(page.getByRole("button", { exact: true, name: "Publish" })).toBeDisabled();

  await page.getByLabel("Resource type").click();
  await page.getByRole("option", { name: /Product Images/ }).click();
  await expect(page.getByLabel("Product code")).toBeVisible();
  await expect(page.getByRole("heading", { name: "Binary data" })).toBeVisible();
  await expect(page.getByText("Advanced JSON", { exact: true })).toHaveCount(0);
});

test("the Chinese Resource editor uses the same generated layout without module-switching tabs", async ({ page }) => {
  await page.setViewportSize({ height: 900, width: 1280 });
  await openEditor(page, "zh-CN");

  await expect(page.getByRole("tab", { name: "资源数据" })).toBeVisible();
  await expect(page.getByRole("button", { exact: true, name: "保存草稿" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "二进制数据" })).toBeVisible();
  await expect(page.getByRole("navigation", { name: "Resource management" })).toHaveCount(0);
  await expect.poll(async () => page.evaluate(() => (
    document.documentElement.scrollWidth <= document.documentElement.clientWidth
  ))).toBe(true);
});

test("the phone Resource editor remains one column and has no horizontal page overflow", async ({ page }) => {
  await page.setViewportSize({ height: 844, width: 390 });
  await openEditor(page);

  await expect(page.getByLabel("Product code")).toBeVisible();
  await expect(page.getByRole("tab", { name: "Version history 3" })).toBeVisible();
  await expect.poll(async () => page.evaluate(() => (
    document.documentElement.scrollWidth <= document.documentElement.clientWidth
  ))).toBe(true);
});
