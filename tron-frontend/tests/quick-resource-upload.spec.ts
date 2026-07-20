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

const fileSchema = {
  $schema: "http://json-schema.org/draft-07/schema#",
  additionalProperties: false,
  properties: {
    fileExtension: { type: "string" },
    fileName: { minLength: 1, type: "string" },
    sizeBytes: { minimum: 0, type: "integer" },
  },
  required: ["fileName", "fileExtension", "sizeBytes"],
  type: "object",
};

const imageSchema = {
  $schema: "http://json-schema.org/draft-07/schema#",
  additionalProperties: false,
  properties: {
    images: {
      items: {
        additionalProperties: false,
        properties: {
          binaryName: { type: "string" },
          fileExtension: { type: "string" },
          fileName: { type: "string" },
          imageFormat: { enum: ["PNG"], type: "string" },
          sizeBytes: { minimum: 0, type: "integer" },
        },
        required: ["binaryName", "fileName", "imageFormat", "sizeBytes"],
        type: "object",
      },
      minItems: 1,
      type: "array",
    },
  },
  required: ["images"],
  type: "object",
};

const pdfSchema = {
  $schema: "http://json-schema.org/draft-07/schema#",
  additionalProperties: false,
  properties: {
    fileName: { minLength: 1, type: "string" },
    pageCount: { minimum: 1, type: "integer" },
    sizeBytes: { minimum: 0, type: "integer" },
  },
  required: ["fileName", "sizeBytes", "pageCount"],
  type: "object",
};

function success(data: unknown) {
  return { code: "0", data, message: "ok" };
}

async function fulfillJson(route: Route, data: unknown) {
  await route.fulfill({ body: JSON.stringify(data), contentType: "application/json", status: 200 });
}

function jsonBody(route: Route): JsonRecord {
  const body = route.request().postDataJSON();
  return body && typeof body === "object" ? body as JsonRecord : {};
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

function definition(
  definitionCode: "FILE" | "IMAGE" | "PDF",
  attributesSchema: JsonRecord,
  overrides: JsonRecord = {},
): JsonRecord {
  return {
    allowInheritance: true,
    attributesSchema,
    binarySchema: {
      maxCount: 1,
      maxNameLength: 255,
      maxSizeBytes: 1_048_576,
      minCount: 1,
      namePattern: "^[^/]+$",
    },
    binaryStorageProfile: "@default",
    builtIn: true,
    createdAt: 1_768_000_000_000,
    definitionCode,
    definitionStatus: "ENABLED",
    definitionVersion: "v1-A1b",
    description: "",
    draftRevision: null,
    lifecycleStatus: "ACTIVE",
    name: definitionCode === "IMAGE" ? "Image" : definitionCode === "PDF" ? "PDF" : "File",
    publishedAt: 1_768_000_000_000,
    scopeCode: "TRON_CORE",
    scopeKind: "WORKSPACE",
    sourceKind: "LOCAL",
    updatedAt: 1_768_000_000_000,
    validationStatus: "VALID",
    ...overrides,
  };
}

async function routeQuickUploadApi(
  page: Page,
  definitions: JsonRecord[],
  multipart: CapturedMultipartRequest[],
  options: QuickUploadApiOptions = {},
) {
  const resources: JsonRecord[] = [];
  await routeResourceUiShellApi(page);

  await page.route("**/api/v1/resource-definitions/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    const body = jsonBody(route);
    if (path.endsWith("/page")) {
      await fulfillJson(route, success({
        current: Number(body.current ?? 1),
        records: definitions,
        size: Number(body.size ?? 50),
        total: definitions.length,
      }));
      return;
    }
    if (path.endsWith("/get")) {
      const record = definitions.find((item) => (
        item.definitionCode === body.definitionCode && item.definitionVersion === body.definitionVersion
      ));
      await fulfillJson(route, success(record ?? {}));
      return;
    }
    await fulfillJson(route, { code: "-1", data: {}, message: "Unexpected definition request" });
  });

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
      const definitionCode = String(command.definitionCode ?? "");
      const definitionRecord = definitions.find((item) => item.definitionCode === definitionCode);
      const resourceCode = String(command.resourceCode ?? "");
      const scopeKind = String(command.scopeKind ?? "WORKSPACE");
      const scopeCode = scopeKind === "PLATFORM" ? "PLATFORM" : String(command.workspaceCode ?? "TRON_CORE");
      const created = {
        allowInheritance: command.allowInheritance === true,
        attributes: command.attributes,
        attributesSchema: definitionRecord?.attributesSchema ?? {},
        binaries: multipartNames(rawBody).map((binaryName) => ({ binaryName, byteSize: 7 })),
        binarySchema: definitionRecord?.binarySchema ?? {},
        binaryStorageProfile: "@default",
        createdAt: 1_768_000_000_000,
        definitionCode,
        definitionName: definitionRecord?.name ?? definitionCode,
        definitionScopeCode: definitionRecord?.scopeCode ?? scopeCode,
        definitionScopeKind: definitionRecord?.scopeKind ?? scopeKind,
        definitionStatus: "ENABLED",
        definitionVersion: definitionRecord?.definitionVersion ?? "v1-A1b",
        description: command.description,
        draftRevision: null,
        lifecycleStatus: "ACTIVE",
        name: command.name,
        publishedAt: 1_768_000_000_000,
        resourceCode,
        resourceVersion: "v1-Q1a",
        scopeCode,
        scopeKind,
        sourceKind: scopeKind === "PLATFORM" ? "PLATFORM" : "LOCAL",
        updatedAt: 1_768_000_000_000,
        validationStatus: "VALID",
      };
      resources.push(created);
      await fulfillJson(route, success({
        activationResult: "COMMITTED",
        definitionCode,
        definitionScopeCode: created.definitionScopeCode,
        definitionScopeKind: created.definitionScopeKind,
        definitionVersion: created.definitionVersion,
        lifecycleStatus: "ACTIVE",
        previousActiveResourceVersion: null,
        resourceCode,
        resourceVersion: "v1-Q1a",
      }));
      return;
    }

    const body = jsonBody(route);
    if (path.endsWith("/page")) {
      const matching = resources.filter((item) => !body.resourceCode || item.resourceCode === body.resourceCode);
      await fulfillJson(route, success({
        current: Number(body.current ?? 1),
        records: matching,
        size: Number(body.size ?? 15),
        total: matching.length,
      }));
      return;
    }
    if (path.endsWith("/get")) {
      const resource = resources.find((item) => (
        item.resourceCode === body.resourceCode && item.resourceVersion === body.resourceVersion
      ));
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

test("quick upload creates a platform IMAGE with one multipart file", async ({ page }) => {
  const multipart: CapturedMultipartRequest[] = [];
  await routeQuickUploadApi(page, [definition("IMAGE", imageSchema, {
    // Matches the built-in IMAGE definition: the persisted rule only defines
    // cardinality, while the service resolves its global size/name defaults.
    binarySchema: { maxCount: 10, minCount: 1 },
    effectiveBinaryPolicy: {
      maxCount: 10,
      maxNameLength: 255,
      maxSizeBytes: 1_073_741_824,
      minCount: 1,
      namePattern: null,
    },
    scopeCode: "PLATFORM",
    scopeKind: "PLATFORM",
    sourceKind: "PLATFORM",
  })], multipart);
  await seedResourceUiShell(page);

  await page.goto(`${frontend!.baseUrl}/settings/resources`);
  await expect(page.getByRole("heading", { name: "Platform Resources" })).toBeVisible({ timeout: 15_000 });
  await page.getByRole("button", { exact: true, name: "Quick Upload" }).click();
  const dialog = page.getByRole("dialog", { name: "Quick upload resource" });
  await dialog.getByLabel("Choose one file for quick upload").setInputFiles({
    buffer: Buffer.from("png-bytes"),
    mimeType: "image/png",
    name: "Brand Logo.PNG",
  });
  await expect(dialog.getByLabel("Name")).toHaveValue("Brand Logo");
  await expect(dialog.getByRole("button", { exact: true, name: "Image" })).toHaveAttribute("aria-pressed", "true");
  await dialog.getByRole("button", { exact: true, name: "Upload" }).click();

  await expect.poll(() => multipart).toHaveLength(1);
  expect(multipart[0].path).toMatch(/\/api\/v1\/resources\/create$/);
  expect(multipartNames(multipart[0].rawBody)).toEqual(["Brand Logo.PNG"]);
  expect(multipart[0].command).toMatchObject({
    allowInheritance: true,
    attributes: {
      images: [{
        binaryName: "Brand Logo.PNG",
        fileExtension: "png",
        fileName: "Brand Logo.PNG",
        imageFormat: "PNG",
        sizeBytes: 9,
      }],
    },
    definitionCode: "IMAGE",
    expectedActiveResourceVersion: null,
    resourceCode: "brand-logo",
    scopeKind: "PLATFORM",
    workspaceCode: null,
  });
  await expect(page).toHaveURL(/\/settings\/resources\/detail\?resourceCode=brand-logo&resourceVersion=v1-Q1a/);
});

test("quick upload waits for a readable PDF page count before creating", async ({ page }) => {
  const multipart: CapturedMultipartRequest[] = [];
  await routeQuickUploadApi(page, [definition("PDF", pdfSchema)], multipart);
  await seedResourceUiShell(page);

  await page.goto(`${frontend!.baseUrl}/resources`);
  await expect(page.getByRole("heading", { name: "Resources" })).toBeVisible({ timeout: 15_000 });
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
  expect(multipart[0].command).toMatchObject({
    attributes: { fileName: "policy.pdf", pageCount: 1, sizeBytes: onePagePdf().byteLength },
    definitionCode: "PDF",
  });
});

test("a resource-code conflict keeps the selected file and expands its editable code", async ({ page }) => {
  const multipart: CapturedMultipartRequest[] = [];
  await routeQuickUploadApi(page, [definition("IMAGE", imageSchema, {
    scopeCode: "PLATFORM",
    scopeKind: "PLATFORM",
    sourceKind: "PLATFORM",
  })], multipart, {
    createFailure: { code: "RESOURCE_EXISTS", message: "Resource code already exists" },
  });
  await seedResourceUiShell(page);

  await page.goto(`${frontend!.baseUrl}/settings/resources`);
  await expect(page.getByRole("heading", { name: "Platform Resources" })).toBeVisible({ timeout: 15_000 });
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
  await expect(page).toHaveURL(/\/settings\/resources$/);
});

test("a Workspace override with extra required fields carries the file into the full editor", async ({ page }) => {
  const multipart: CapturedMultipartRequest[] = [];
  const requiredTitleSchema = {
    ...fileSchema,
    properties: { ...fileSchema.properties, title: { minLength: 1, type: "string" } },
    required: [...fileSchema.required, "title"],
  };
  await routeQuickUploadApi(page, [definition("FILE", requiredTitleSchema, {
    scopeCode: "TRON_CORE",
    scopeKind: "WORKSPACE",
    sourceKind: "OVERRIDE",
  })], multipart);
  await seedResourceUiShell(page);

  await page.goto(`${frontend!.baseUrl}/resources`);
  await expect(page.getByRole("heading", { name: "Resources" })).toBeVisible({ timeout: 15_000 });
  await page.getByRole("button", { exact: true, name: "Quick Upload" }).click();
  const dialog = page.getByRole("dialog", { name: "Quick upload resource" });
  await dialog.getByLabel("Choose one file for quick upload").setInputFiles({
    buffer: Buffer.from("text"),
    name: "workspace-guide.txt",
  });
  await expect(page).toHaveURL(/\/resources\/new\?quickUpload=/);
  await expect(page.getByLabel("Resource name")).toHaveValue("workspace-guide");
  await expect(page.getByLabel("Resource code")).toHaveValue("workspace-guide");
  await expect(page.getByLabel("fileName")).toHaveValue("workspace-guide.txt");
  await expect(page.getByText("workspace-guide.txt", { exact: true })).toBeVisible();
  expect(multipart).toHaveLength(0);

  await page.reload();
  await expect(page.getByText("The selected quick-upload file is no longer available. Choose it again in Files to continue.", { exact: true })).toBeVisible();
});

test("quick upload rejects a multiple-file drop and sends an unreadable PDF to the editor with pageCount 0", async ({ page }) => {
  const multipart: CapturedMultipartRequest[] = [];
  await routeQuickUploadApi(page, [definition("PDF", pdfSchema)], multipart);
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
  await expect(page).toHaveURL(/\/resources\/new\?quickUpload=/, { timeout: 20_000 });
  await expect(page.getByLabel("pageCount")).toHaveValue("0");
  expect(multipart).toHaveLength(0);
});
