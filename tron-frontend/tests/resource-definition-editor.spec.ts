import { expect, test, type Page } from "@playwright/test";

import {
  routeResourceUiShellApi,
  seedResourceUiShell,
  startResourceUiFrontend,
  stopResourceUiFrontend,
  type ResourceUiFrontend,
} from "./support/resource-ui-harness";

let frontend: ResourceUiFrontend | undefined;

const detailPath = "/resource-definitions/detail?definitionCode=PRODUCT_IMAGES&definitionVersion=v3-K7m";

async function openEditor(
  page: Page,
  locale: "en-US" | "zh-CN" = "en-US",
  path = detailPath,
  heading = "Product Images",
) {
  await routeResourceUiShellApi(page);
  await seedResourceUiShell(page, locale);
  await page.goto(`${frontend!.baseUrl}${path}`);
  await expect(page.getByRole("heading", { exact: true, name: heading })).toBeVisible();
}

test.beforeAll(async () => {
  frontend = await startResourceUiFrontend(detailPath);
});

test.afterAll(async () => {
  await stopResourceUiFrontend(frontend);
  frontend = undefined;
});

test("the draft editor keeps visual fields and Draft 7 JSON on one source of truth", async ({ page }) => {
  await openEditor(page);

  await expect(page.getByText("DRAFT", { exact: true }).first()).toBeVisible();
  await expect(page.getByRole("tab", { name: "Structured attributes 5" })).toHaveAttribute("aria-selected", "true");
  const schemaJson = page.getByLabel("Attributes Schema JSON");
  await expect(schemaJson).toContainText('"title": "Product code"');

  await page.getByRole("button", { name: "Edit display label for productCode" }).click();
  await page.getByRole("textbox", { exact: true, name: "Display label" }).fill("Product identifier");
  await page.getByRole("textbox", { exact: true, name: "Display label" }).press("Enter");
  await expect(schemaJson).toContainText('"title": "Product identifier"');

  const originalSchema = await schemaJson.inputValue();
  await schemaJson.fill("{");
  await expect(page.getByText("INVALID JSON", { exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Publish", exact: true })).toBeDisabled();
  await schemaJson.fill(originalSchema);
  await expect(page.getByText("VALID JSON", { exact: true })).toBeVisible();

  await page.getByRole("button", { name: "Test data" }).click();
  await expect(page.getByRole("dialog", { name: "Test structured data" })).toBeVisible();
  await page.getByRole("button", { name: "Validate", exact: true }).click();
  await expect(page.getByText("Test data matches this Schema.", { exact: true })).toBeVisible();
});

test("the shared Draft 7 editor recursively edits arrays and nested objects", async ({ page }) => {
  await openEditor(
    page,
    "en-US",
    "/resource-definitions/detail?definitionCode=MYSQL_CONNECTION&definitionVersion=v4-a7K",
    "MySQL Connection",
  );

  await page.getByLabel("Expand endpoints").click();
  await expect(page.getByText("Item schema", { exact: true })).toBeVisible();
  await expect(page.getByText("Host", { exact: true })).toBeVisible();
  await expect(page.getByText("Port", { exact: true })).toBeVisible();
  await expect(page.getByText("Role", { exact: true })).toBeVisible();

  await page.getByLabel("Expand credentials").click();
  await expect(page.getByText("Username variable", { exact: true })).toBeVisible();
  await expect(page.getByText("Password variable", { exact: true })).toBeVisible();
  await page.getByLabel("Expand username").click();
  await page.getByLabel("Expand password").click();
  await expect(page.locator('[data-slot="badge"]', { hasText: "tron-variable-ref" })).toHaveCount(2);
  await expect(page.getByLabel("Attributes Schema JSON")).toHaveValue(/"credentials"/);
});

test("binary constraints remain generic and version history exposes lifecycle actions", async ({ page }) => {
  await openEditor(page);

  await page.getByRole("tab", { name: "Binary constraints 12" }).click();
  await expect(page.getByText("Storage profile", { exact: true }).first()).toBeVisible();
  await expect(page.getByText("Names are unique within one Resource version", { exact: true })).toBeVisible();
  await expect(page.getByText(/MIME/i)).toHaveCount(0);

  await page.getByRole("tab", { name: "Version history 3" }).click();
  await expect(page.locator("tbody tr")).toHaveCount(3);
  await expect(page.locator("thead th").last()).toHaveText("Actions");

  await page.locator("tbody tr").filter({ hasText: "v1-a7K" }).click();
  await expect(page.getByRole("button", { name: "Activate", exact: true })).toBeDisabled();
  await expect(page.getByRole("button", { name: "Activate", exact: true })).toHaveAttribute("title", /v3-K7m/);

  await page.getByRole("tab", { name: "Version history 3" }).click();
  await page.getByLabel("Open actions for v3-K7m").click();
  await page.getByRole("menuitem", { name: "Publish", exact: true }).click();
  await expect(page.getByText("ACTIVE", { exact: true }).first()).toBeVisible();
  await expect(page.getByRole("button", { name: "Edit", exact: true })).toBeVisible();
});

test("the Chinese editor is readable and keeps the workspace page free of module-switching tabs", async ({ page }) => {
  await page.setViewportSize({ height: 900, width: 1280 });
  await openEditor(page, "zh-CN");

  await expect(page.getByRole("tab", { name: "结构化属性 5" })).toBeVisible();
  await expect(page.getByRole("button", { name: "保存草稿", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "添加属性", exact: true })).toBeVisible();
  await expect(page.getByRole("navigation", { name: "Resource management" })).toHaveCount(0);
  await expect.poll(async () => page.evaluate(() => (
    document.documentElement.scrollWidth <= document.documentElement.clientWidth
  ))).toBe(true);
});

test("the phone editor switches between one visual and one JSON pane without page overflow", async ({ page }) => {
  await page.setViewportSize({ height: 844, width: 390 });
  await openEditor(page);

  await expect(page.getByRole("tab", { name: "Visual", exact: true })).toBeVisible();
  await expect(page.getByLabel("Attributes Schema JSON")).toBeHidden();
  await page.getByRole("tab", { name: "JSON", exact: true }).click();
  await expect(page.getByLabel("Attributes Schema JSON")).toBeVisible();
  await expect(page.getByRole("tab", { name: "Visual", exact: true })).toHaveCount(1);
  await expect.poll(async () => page.evaluate(() => (
    document.documentElement.scrollWidth <= document.documentElement.clientWidth
  ))).toBe(true);
});

test("a new definition starts as an incomplete draft and cannot publish", async ({ page }) => {
  await routeResourceUiShellApi(page);
  await seedResourceUiShell(page);
  await page.goto(`${frontend!.baseUrl}/resource-definitions/new`);

  await expect(page.getByLabel("Definition name")).toHaveValue("");
  await expect(page.getByLabel("Definition code")).toHaveValue("");
  await expect(page.getByRole("button", { name: "Save draft", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Publish", exact: true })).toBeDisabled();
});
