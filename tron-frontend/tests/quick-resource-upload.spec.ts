import { expect, test, type Page, type Route } from "@playwright/test";

import {
  routeResourceUiShellApi,
  seedResourceUiShell,
  startResourceUiFrontend,
  stopResourceUiFrontend,
  type ResourceUiFrontend,
} from "./support/resource-ui-harness";

let frontend: ResourceUiFrontend | undefined;

test.describe.configure({ timeout: 60_000 });

type JsonRecord = Record<string, unknown>;

interface CapturedMultipartRequest {
  command: JsonRecord;
  path: string;
  rawBody: string;
}

interface QuickUploadApiOptions {
  createFailure?: { code: string; message: string };
}

function success(data: unknown) {
  return { code: "0", data, message: "ok" };
}

async function fulfillJson(route: Route, data: unknown) {
  await route.fulfill({ body: JSON.stringify(data), contentType: "application/json", status: 200 });
}

function multipartCommand(rawBody: string): JsonRecord {
  const serialized = rawBody.match(/name="command"\r?\n\r?\n([\s\S]*?)\r?\n--/)?.[1];
  if (!serialized) return {};
  const value: unknown = JSON.parse(serialized);
  return value && typeof value === "object" ? value as JsonRecord : {};
}

function multipartNames(rawBody: string) {
  return Array.from(rawBody.matchAll(/name="binary"; filename="([^"]+)"/g), (match) => match[1]);
}

function onePagePdf() {
  const objects = [
    "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
    "2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n",
    "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << >> /Contents 4 0 R >>\nendobj\n",
    "4 0 obj\n<< /Length 4 >>\nstream\nq\nQ\nendstream\nendobj\n",
  ];
  let document = "%PDF-1.4\n";
  const offsets = objects.map((object) => {
    const offset = Buffer.byteLength(document);
    document += object;
    return offset;
  });
  const startXref = Buffer.byteLength(document);
  document += `xref\n0 ${objects.length + 1}\n0000000000 65535 f \n`;
  document += offsets.map((offset) => `${String(offset).padStart(10, "0")} 00000 n \n`).join("");
  document += `trailer\n<< /Size ${objects.length + 1} /Root 1 0 R >>\nstartxref\n${startXref}\n%%EOF\n`;
  return Buffer.from(document, "ascii");
}

async function routeQuickUploadApi(
  page: Page,
  multipart: CapturedMultipartRequest[],
  options: QuickUploadApiOptions = {},
) {
  const resources: JsonRecord[] = [];
  await routeResourceUiShellApi(page);

  await page.route("**/api/v1/resources/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path.endsWith("/create")) {
      const rawBody = route.request().postDataBuffer().toString("utf8");
      const command = multipartCommand(rawBody);
      multipart.push({ command, path, rawBody });
      if (options.createFailure) {
        await fulfillJson(route, { code: options.createFailure.code, data: {}, message: options.createFailure.message });
        return;
      }
      const kind = String(command.kind ?? "FILE");
      const resourceCode = String(command.resourceCode ?? "");
      const scopeCode = String(command.workspaceCode ?? "TRON_CORE");
      const fileName = multipartNames(rawBody)[0] ?? "";
      const created = {
        allowInheritance: command.allowInheritance === true,
        attributes: null,
        createdAt: 1_768_000_000_000,
        description: command.description ?? "",
        file: { byteSize: 7, fileExtension: "png", fileName },
        kind,
        name: command.name,
        resourceCode,
        scopeCode,
        workspaceCode: scopeCode,
        sourceKind: "Local",
        updatedAt: 1_768_000_000_000,
      };
      resources.push(created);
      await fulfillJson(route, success({ kind, resourceCode }));
      return;
    }

    const body = route.request().postDataJSON() as JsonRecord | null;
    if (path.endsWith("/page")) {
      const kindFilter = body && Array.isArray(body.kinds) && body.kinds.length === 1
        ? String(body.kinds[0])
        : null;
      const search = body && typeof body.search === "string" ? body.search.toLowerCase() : "";
      const matching = resources.filter((item) => (
        (!kindFilter || item.kind === kindFilter)
        && (!search || [item.name, item.resourceCode].some((value) => String(value).toLowerCase().includes(search)))
      ));
      await fulfillJson(route, success({
        current: Number(body?.current ?? 1),
        records: matching,
        size: Number(body?.size ?? 15),
        total: matching.length,
      }));
      return;
    }
    if (path.endsWith("/get")) {
      const resource = resources.find((item) => item.resourceCode === body?.resourceCode);
      await fulfillJson(route, success(resource ?? {}));
      return;
    }
    await fulfillJson(route, { code: "-1", data: {}, message: "Unexpected resource request" });
  });
}

test.beforeAll(async () => {
  frontend = await startResourceUiFrontend("/resources", { VITE_TRON_RESOURCE_DATA_SOURCE: "api" });
});

test.afterAll(async () => {
  await stopResourceUiFrontend(frontend);
  frontend = undefined;
});

test("quick upload creates an IMAGE with one multipart file", async ({ page }) => {
  const multipart: CapturedMultipartRequest[] = [];
  await routeQuickUploadApi(page, multipart);
  await seedResourceUiShell(page);

  await page.goto(`${frontend!.baseUrl}/resources`);
  await expect(page.getByRole("tab", { name: "Resources", exact: true })).toBeVisible({ timeout: 15_000 });
  await page.getByRole("button", { exact: true, name: "Quick Upload" }).click();
  const dialog = page.getByRole("dialog", { name: "Quick upload resource" });
  await dialog.getByLabel("Choose one file for quick upload").setInputFiles({
    buffer: Buffer.from("png-bytes"),
    mimeType: "image/png",
    name: "Brand Logo.PNG",
  });
  await expect(dialog.getByLabel("Name")).toHaveValue("Brand Logo");
  await dialog.getByRole("button", { exact: true, name: "Upload" }).click();

  await expect.poll(() => multipart).toHaveLength(1);
  expect(multipart[0].path).toMatch(/\/api\/v1\/resources\/create$/);
  expect(multipartNames(multipart[0].rawBody)).toEqual(["Brand Logo.PNG"]);
  expect(multipart[0].command).toMatchObject({
    allowInheritance: false,
    description: "",
    kind: "Image",
    name: "Brand Logo",
    resourceCode: "brand-logo",
    workspaceCode: "TRON_CORE",
  });
  await expect(page).toHaveURL(/\/resources\/detail\?resourceCode=brand-logo$/);
});

test("quick upload waits for a readable PDF page count before creating", async ({ page }) => {
  const multipart: CapturedMultipartRequest[] = [];
  await routeQuickUploadApi(page, multipart);
  await seedResourceUiShell(page);

  await page.goto(`${frontend!.baseUrl}/resources`);
  await expect(page.getByRole("tab", { name: "Resources", exact: true })).toBeVisible({ timeout: 15_000 });
  await page.getByRole("button", { exact: true, name: "Quick Upload" }).click();
  const dialog = page.getByRole("dialog", { name: "Quick upload resource" });
  await dialog.getByLabel("Choose one file for quick upload").setInputFiles({
    buffer: onePagePdf(),
    mimeType: "application/pdf",
    name: "policy.pdf",
  });
  await expect(dialog.getByText("1 pages detected", { exact: true })).toBeVisible({ timeout: 20_000 });
  await dialog.getByRole("button", { exact: true, name: "Upload" }).click();

  await expect.poll(() => multipart).toHaveLength(1);
  expect(multipart[0].command).toMatchObject({ kind: "Pdf", resourceCode: "policy" });
});

test("a resource-code conflict keeps the selected file and expands its editable code", async ({ page }) => {
  const multipart: CapturedMultipartRequest[] = [];
  await routeQuickUploadApi(page, multipart, {
    createFailure: { code: "RESOURCE_EXISTS", message: "Resource code already exists" },
  });
  await seedResourceUiShell(page);

  await page.goto(`${frontend!.baseUrl}/resources`);
  await expect(page.getByRole("tab", { name: "Resources", exact: true })).toBeVisible({ timeout: 15_000 });
  await page.getByRole("button", { exact: true, name: "Quick Upload" }).click();
  const dialog = page.getByRole("dialog", { name: "Quick upload resource" });
  await dialog.getByLabel("Choose one file for quick upload").setInputFiles({
    buffer: Buffer.from("png-bytes"),
    mimeType: "image/png",
    name: "Brand Logo.PNG",
  });
  await dialog.getByRole("button", { exact: true, name: "Upload" }).click();

  await expect(dialog.getByText("Resource code already exists", { exact: true })).toBeVisible();
  await expect(dialog.getByLabel("Resource code")).toBeVisible();
  await expect(dialog.getByLabel("Resource code")).toHaveValue("brand-logo");
  await expect(dialog.getByTestId("quick-resource-upload-dropzone").getByText("Brand Logo.PNG", { exact: true })).toBeVisible();
  expect(multipart).toHaveLength(1);
  await expect(page).toHaveURL(/\/resources$/);
});

test("platform resources have no source filter and only expose quick upload creation", async ({ page }) => {
  const multipart: CapturedMultipartRequest[] = [];
  await routeQuickUploadApi(page, multipart);
  await seedResourceUiShell(page);

  await page.goto(`${frontend!.baseUrl}/settings/resources`);
  await expect(page.getByRole("columnheader", { name: "Resource", exact: true })).toBeVisible({ timeout: 15_000 });
  await expect(page.getByRole("button", { exact: true, name: "Quick Upload" })).toBeVisible();
  await expect(page.getByRole("button", { exact: true, name: "New Resource" })).toHaveCount(0);
  await expect(page.getByLabel("Filter resources by source")).toHaveCount(0);
});

test("quick upload rejects a multiple-file drop and accepts an unreadable PDF", async ({ page }) => {
  const multipart: CapturedMultipartRequest[] = [];
  await routeQuickUploadApi(page, multipart);
  await seedResourceUiShell(page);

  await page.goto(`${frontend!.baseUrl}/resources`);
  await page.getByRole("button", { exact: true, name: "Quick Upload" }).click();
  await page.getByTestId("quick-resource-upload-dropzone").evaluate((element) => {
    const transfer = new DataTransfer();
    transfer.items.add(new File(["one"], "one.txt"));
    transfer.items.add(new File(["two"], "two.txt"));
    element.dispatchEvent(new DragEvent("drop", { bubbles: true, dataTransfer: transfer }));
  });
  await expect(page.getByText("Quick upload accepts one file at a time. Split multiple files into separate uploads.", { exact: true })).toBeVisible();

  await page.getByLabel("Choose one file for quick upload").setInputFiles({
    buffer: Buffer.from("not a valid PDF"),
    mimeType: "application/pdf",
    name: "broken.pdf",
  });
  await expect(page.getByText("Page count could not be read. The PDF can still be uploaded.", { exact: true })).toBeVisible({ timeout: 20_000 });
  await page.getByRole("button", { exact: true, name: "Upload" }).click();
  await expect.poll(() => multipart).toHaveLength(1);
  expect(multipart[0].command).toMatchObject({ kind: "Pdf", resourceCode: "broken" });
  await expect(page).toHaveURL(/\/resources\/detail\?resourceCode=broken$/);
});
