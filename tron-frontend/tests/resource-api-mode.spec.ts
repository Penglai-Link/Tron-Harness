import { expect, test, type Route } from "@playwright/test";

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

interface CapturedJsonRequest {
  body: JsonRecord;
  path: string;
}

interface CapturedMultipartRequest {
  contentType: string | undefined;
  path: string;
  rawBody: string;
}

const draft7TitleSchema = {
  $schema: "http://json-schema.org/draft-07/schema#",
  additionalProperties: false,
  properties: {
    title: {
      title: "Title",
      type: "string",
    },
  },
  required: ["title"],
  type: "object",
};

const permissiveBinarySchema = {
  maxCount: 2,
  maxNameLength: 255,
  maxSizeBytes: 1_048_576,
  minCount: 0,
  namePattern: "",
};

function successEnvelope(data: unknown) {
  return { code: "0", data, message: "ok" };
}

async function fulfillJson(route: Route, data: unknown) {
  await route.fulfill({
    body: JSON.stringify(data),
    contentType: "application/json",
    status: 200,
  });
}

function definitionRecord(overrides: Record<string, unknown> = {}) {
  return {
    allowInheritance: true,
    attributesSchema: null,
    binarySchema: null,
    binaryStorageProfile: null,
    builtIn: true,
    createdAt: 1_767_225_600_000,
    definitionCode: "FILE",
    definitionStatus: "ENABLED",
    definitionVersion: "v1-000",
    description: "Managed file resource",
    draftRevision: null,
    lifecycleStatus: "ACTIVE",
    name: "File",
    publishedAt: 1_767_225_600_000,
    scopeCode: "PLATFORM",
    scopeKind: "PLATFORM",
    sourceKind: "PLATFORM",
    updatedAt: 1_767_225_600_000,
    validationStatus: "VALID",
    ...overrides,
  };
}

function resourceRecord(overrides: Record<string, unknown> = {}) {
  return {
    allowInheritance: true,
    attributes: null,
    attributesSchema: null,
    binaries: null,
    binarySchema: null,
    binaryStorageProfile: null,
    createdAt: 1_767_225_600_000,
    definitionCode: "FILE",
    definitionName: "File",
    definitionScopeCode: "PLATFORM",
    definitionScopeKind: "PLATFORM",
    definitionStatus: "ENABLED",
    definitionVersion: "v1-000",
    description: "Inherited release archive",
    draftRevision: null,
    lifecycleStatus: "ACTIVE",
    name: "Release archive",
    publishedAt: 1_767_225_600_000,
    resourceCode: "release-archive",
    resourceVersion: "v2-A1b",
    scopeCode: "PLATFORM",
    scopeKind: "PLATFORM",
    sourceKind: "INHERITED",
    updatedAt: 1_767_225_600_000,
    validationStatus: "VALID",
    ...overrides,
  };
}

async function captureJsonPageRequest(route: Route, requests: unknown[]) {
  requests.push(route.request().postDataJSON());
}

function jsonRequest(route: Route): JsonRecord {
  const body = route.request().postDataJSON();
  return body && typeof body === "object" ? body as JsonRecord : {};
}

function multipartCommand(rawBody: string): JsonRecord {
  const serialized = rawBody.match(/name="command"\r?\n\r?\n([\s\S]*?)\r?\n--/)?.[1];
  if (!serialized) return {};
  try {
    const value: unknown = JSON.parse(serialized);
    return value && typeof value === "object" ? value as JsonRecord : {};
  } catch {
    return {};
  }
}

function multipartBinaryNames(rawBody: string) {
  return Array.from(
    rawBody.matchAll(/name="binary"; filename="([^"]+)"/g),
    (match) => match[1],
  );
}

function commandString(command: JsonRecord, key: string) {
  return typeof command[key] === "string" ? command[key] : "";
}

function commandStringArray(command: JsonRecord, key: string) {
  return Array.isArray(command[key])
    ? command[key].filter((value): value is string => typeof value === "string")
    : [];
}

function commandAttributes(command: JsonRecord) {
  const value = command.attributes;
  return value && typeof value === "object" && !Array.isArray(value)
    ? value as JsonRecord
    : {};
}

function cloneRecord<T extends JsonRecord>(record: T): T {
  return JSON.parse(JSON.stringify(record)) as T;
}

function nextRevision(record: JsonRecord) {
  return String(Number(record.draftRevision ?? "0") + 1);
}

function definitionDraftAction(record: JsonRecord, discarded: boolean | null = null) {
  return {
    definitionCode: record.definitionCode,
    definitionVersion: record.definitionVersion,
    discarded,
    draftRevision: record.draftRevision,
    lifecycleStatus: record.lifecycleStatus,
    scopeCode: record.scopeCode,
    scopeKind: record.scopeKind,
    validationErrors: [],
    validationStatus: record.validationStatus,
  };
}

function definitionActivation(record: JsonRecord, result: "DRAFT_PUBLISHED" | "HISTORICAL_REACTIVATED") {
  return {
    activationResult: result,
    definitionCode: record.definitionCode,
    definitionVersion: record.definitionVersion,
    lifecycleStatus: record.lifecycleStatus,
    previousActiveDefinitionVersion: null,
  };
}

function resourceDraftAction(record: JsonRecord, discarded: boolean | null = null) {
  return {
    definitionCode: record.definitionCode,
    definitionScopeCode: record.definitionScopeCode,
    definitionScopeKind: record.definitionScopeKind,
    definitionVersion: record.definitionVersion,
    discarded,
    draftResult: "DRAFT_SAVED",
    draftRevision: record.draftRevision,
    resourceCode: record.resourceCode,
    resourceRemoved: false,
    resourceVersion: record.resourceVersion,
    validationErrors: [],
    validationStatus: record.validationStatus,
  };
}

function resourceActivation(
  record: JsonRecord,
  result: "COMMITTED" | "DRAFT_PUBLISHED" | "HISTORICAL_REACTIVATED",
  previousActiveResourceVersion: string | null = null,
) {
  return {
    activationResult: result,
    definitionCode: record.definitionCode,
    definitionScopeCode: record.definitionScopeCode,
    definitionScopeKind: record.definitionScopeKind,
    definitionVersion: record.definitionVersion,
    lifecycleStatus: record.lifecycleStatus,
    previousActiveResourceVersion,
    resourceCode: record.resourceCode,
    resourceVersion: record.resourceVersion,
  };
}

async function fulfillNotFound(route: Route) {
  await fulfillJson(route, {
    code: "-1",
    data: {},
    message: "Resource test route is not implemented",
  });
}

async function routeDefinitionEditorApi(
  page: Parameters<typeof routeResourceUiShellApi>[0],
  records: JsonRecord[],
  requests: CapturedJsonRequest[],
) {
  let nextDraftNumber = 2;
  await page.route("**/api/v1/resource-definitions/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    const body = jsonRequest(route);
    requests.push({ body, path });

    if (path.endsWith("/page")) {
      await fulfillJson(route, successEnvelope({
        current: Number(body.current ?? 1),
        records,
        size: Number(body.size ?? 50),
        total: records.length,
      }));
      return;
    }

    if (path.endsWith("/get")) {
      const record = records.find((item) => (
        item.definitionCode === body.definitionCode
          && item.definitionVersion === body.definitionVersion
      ));
      if (!record) return fulfillNotFound(route);
      await fulfillJson(route, successEnvelope(record));
      return;
    }

    if (path.endsWith("/create-draft")) {
      const source = records.find((item) => item.definitionVersion === body.sourceDefinitionVersion);
      if (!source) return fulfillNotFound(route);
      const draft = {
        ...cloneRecord(source),
        createdAt: 1_767_312_000_000,
        definitionVersion: `v${nextDraftNumber}-D${nextDraftNumber}a`,
        draftRevision: "1",
        lifecycleStatus: "DRAFT",
        publishedAt: null,
        updatedAt: 1_767_312_000_000,
      };
      nextDraftNumber += 1;
      records.push(draft);
      await fulfillJson(route, successEnvelope(definitionDraftAction(draft)));
      return;
    }

    if (path.endsWith("/update-draft")) {
      const draft = records.find((item) => item.definitionVersion === body.definitionVersion);
      if (!draft) return fulfillNotFound(route);
      Object.assign(draft, {
        allowInheritance: body.allowInheritance,
        attributesSchema: body.attributesSchema,
        binarySchema: body.binarySchema,
        binaryStorageProfile: body.binaryStorageProfile,
        definitionStatus: body.definitionStatus,
        description: body.description,
        draftRevision: nextRevision(draft),
        name: body.name,
        updatedAt: 1_767_398_400_000,
      });
      await fulfillJson(route, successEnvelope(definitionDraftAction(draft)));
      return;
    }

    if (path.endsWith("/publish-draft")) {
      const draft = records.find((item) => item.definitionVersion === body.definitionVersion);
      if (!draft) return fulfillNotFound(route);
      records.forEach((item) => {
        if (item.lifecycleStatus === "ACTIVE") item.lifecycleStatus = "HISTORICAL";
      });
      Object.assign(draft, {
        draftRevision: null,
        lifecycleStatus: "ACTIVE",
        publishedAt: 1_767_484_800_000,
        updatedAt: 1_767_484_800_000,
      });
      await fulfillJson(route, successEnvelope(definitionActivation(draft, "DRAFT_PUBLISHED")));
      return;
    }

    if (path.endsWith("/activate-version")) {
      const historical = records.find((item) => item.definitionVersion === body.definitionVersion);
      if (!historical) return fulfillNotFound(route);
      records.forEach((item) => {
        if (item.lifecycleStatus === "ACTIVE") item.lifecycleStatus = "HISTORICAL";
      });
      Object.assign(historical, {
        lifecycleStatus: "ACTIVE",
        publishedAt: 1_767_571_200_000,
        updatedAt: 1_767_571_200_000,
      });
      await fulfillJson(route, successEnvelope(definitionActivation(historical, "HISTORICAL_REACTIVATED")));
      return;
    }

    if (path.endsWith("/discard-draft")) {
      const index = records.findIndex((item) => item.definitionVersion === body.definitionVersion);
      if (index < 0) return fulfillNotFound(route);
      const [draft] = records.splice(index, 1);
      await fulfillJson(route, successEnvelope(definitionDraftAction(draft, true)));
      return;
    }

    await fulfillNotFound(route);
  });
}

async function routeResourceEditorApi(
  page: Parameters<typeof routeResourceUiShellApi>[0],
  resources: JsonRecord[],
  definitions: JsonRecord[],
  resourceRequests: CapturedJsonRequest[],
  multipartRequests: CapturedMultipartRequest[],
) {
  let nextDraftNumber = 3;
  let nextCommitNumber = 3;
  let rejectNextBinaryRemoval = true;

  await page.route("**/api/v1/resource-definitions/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    const body = jsonRequest(route);
    if (path.endsWith("/page")) {
      await fulfillJson(route, successEnvelope({
        current: Number(body.current ?? 1),
        records: definitions,
        size: Number(body.size ?? 50),
        total: definitions.length,
      }));
      return;
    }
    if (path.endsWith("/get")) {
      const record = definitions.find((item) => (
        item.definitionCode === body.definitionCode
          && item.definitionVersion === body.definitionVersion
      ));
      if (!record) return fulfillNotFound(route);
      await fulfillJson(route, successEnvelope(record));
      return;
    }
    await fulfillNotFound(route);
  });

  await page.route("**/api/v1/resources/**", async (route) => {
    const path = new URL(route.request().url()).pathname;

    if (path.endsWith("/create") || path.endsWith("/update")) {
      const rawBody = route.request().postDataBuffer().toString("utf8");
      const command = multipartCommand(rawBody);
      multipartRequests.push({
        contentType: route.request().headers()["content-type"],
        path,
        rawBody,
      });

      const resourceCode = commandString(command, "resourceCode");
      const expectedActiveVersion = commandString(command, "expectedActiveResourceVersion");
      const source = path.endsWith("/update")
        ? resources.find((item) => (
          item.resourceCode === resourceCode
            && item.resourceVersion === expectedActiveVersion
            && item.lifecycleStatus === "ACTIVE"
        ))
        : undefined;
      if (path.endsWith("/update") && !source) return fulfillNotFound(route);

      const definitionCode = commandString(command, "definitionCode") || String(source?.definitionCode ?? "");
      const definition = definitions.find((item) => (
        item.definitionCode === definitionCode && item.lifecycleStatus === "ACTIVE"
      ));
      if (!definition) return fulfillNotFound(route);

      const binaries = new Map<string, JsonRecord>();
      const sourceBinaries = Array.isArray(source?.binaries) ? source.binaries as JsonRecord[] : [];
      sourceBinaries.forEach((binary) => binaries.set(String(binary.binaryName), cloneRecord(binary)));
      commandStringArray(command, "removedBinaryNames").forEach((binaryName) => binaries.delete(binaryName));
      multipartBinaryNames(rawBody).forEach((binaryName) => {
        binaries.set(binaryName, { binaryName, byteSize: 17 });
      });

      const scopeKind = commandString(command, "scopeKind") || "WORKSPACE";
      const scopeCode = scopeKind === "PLATFORM"
        ? "PLATFORM"
        : commandString(command, "workspaceCode");
      const versionNumber = nextCommitNumber;
      nextCommitNumber += 1;
      const committed = {
        ...cloneRecord(source ?? resourceRecord()),
        allowInheritance: command.allowInheritance === true,
        attributes: commandAttributes(command),
        attributesSchema: definition.attributesSchema,
        binaries: [...binaries.values()],
        binarySchema: definition.binarySchema,
        binaryStorageProfile: definition.binaryStorageProfile,
        createdAt: 1_767_571_200_000,
        definitionCode,
        definitionName: definition.name,
        definitionScopeCode: definition.scopeCode,
        definitionScopeKind: definition.scopeKind,
        definitionStatus: definition.definitionStatus,
        definitionVersion: definition.definitionVersion,
        description: commandString(command, "description"),
        draftRevision: null,
        lifecycleStatus: "ACTIVE",
        name: commandString(command, "name"),
        publishedAt: 1_767_571_200_000,
        resourceCode,
        resourceVersion: `v${versionNumber}-C${versionNumber}a`,
        scopeCode,
        scopeKind,
        sourceKind: scopeKind === "PLATFORM"
          ? "PLATFORM"
          : source?.scopeKind === "PLATFORM" ? "OVERRIDE" : "LOCAL",
        updatedAt: 1_767_571_200_000,
        validationStatus: "VALID",
      };
      if (source) source.lifecycleStatus = "HISTORICAL";
      resources.push(committed);
      await fulfillJson(route, successEnvelope(resourceActivation(committed, "COMMITTED", source?.resourceVersion ?? null)));
      return;
    }

    if (path.endsWith("/upload-binary")) {
      const rawBody = route.request().postDataBuffer().toString("utf8");
      multipartRequests.push({
        contentType: route.request().headers()["content-type"],
        path,
        rawBody,
      });
      const binaryName = rawBody.match(/name="binaryName"\r?\n\r?\n([^\r\n]+)/)?.[1];
      const draft = resources.find((item) => item.lifecycleStatus === "DRAFT");
      if (!draft || !binaryName) return fulfillNotFound(route);
      const binaries = Array.isArray(draft.binaries) ? draft.binaries as JsonRecord[] : [];
      const existing = binaries.find((item) => item.binaryName === binaryName);
      if (existing) {
        existing.byteSize = 15;
      } else {
        binaries.push({ binaryName, byteSize: 11 });
      }
      Object.assign(draft, {
        binaries,
        draftRevision: nextRevision(draft),
        updatedAt: 1_767_398_400_000,
      });
      await fulfillJson(route, successEnvelope({
        action: "UPLOADED",
        binaryName,
        byteSize: existing ? 15 : 11,
        draftRevision: draft.draftRevision,
        resourceCode: draft.resourceCode,
        resourceVersion: draft.resourceVersion,
        validationErrors: [],
        validationStatus: "VALID",
      }));
      return;
    }

    if (path.endsWith("/download-binary")) {
      const body = jsonRequest(route);
      resourceRequests.push({ body, path });
      await route.fulfill({
        body: Buffer.from("opaque-resource-binary"),
        contentType: "application/octet-stream",
        headers: {
          "Content-Disposition": `attachment; filename*=UTF-8''${encodeURIComponent(String(body.binaryName))}`,
        },
        status: 200,
      });
      return;
    }

    const body = jsonRequest(route);
    resourceRequests.push({ body, path });

    if (path.endsWith("/page")) {
      await fulfillJson(route, successEnvelope({
        current: Number(body.current ?? 1),
        records: resources,
        size: Number(body.size ?? 50),
        total: resources.length,
      }));
      return;
    }

    if (path.endsWith("/get")) {
      const record = resources.find((item) => (
        item.resourceCode === body.resourceCode
          && item.resourceVersion === body.resourceVersion
      ));
      if (!record) return fulfillNotFound(route);
      await fulfillJson(route, successEnvelope(record));
      return;
    }

    if (path.endsWith("/create-draft")) {
      const source = resources.find((item) => item.resourceVersion === body.sourceResourceVersion);
      if (!source) return fulfillNotFound(route);
      const draft = {
        ...cloneRecord(source),
        createdAt: 1_767_312_000_000,
        draftRevision: "1",
        lifecycleStatus: "DRAFT",
        publishedAt: null,
        resourceVersion: `v${nextDraftNumber}-D${nextDraftNumber}a`,
        sourceKind: "LOCAL",
        updatedAt: 1_767_312_000_000,
      };
      nextDraftNumber += 1;
      resources.push(draft);
      await fulfillJson(route, successEnvelope(resourceDraftAction(draft)));
      return;
    }

    if (path.endsWith("/update-draft")) {
      const draft = resources.find((item) => item.resourceVersion === body.resourceVersion);
      if (!draft) return fulfillNotFound(route);
      Object.assign(draft, {
        allowInheritance: body.allowInheritance,
        attributes: body.attributes,
        description: body.description,
        draftRevision: nextRevision(draft),
        name: body.name,
        updatedAt: 1_767_398_400_000,
      });
      await fulfillJson(route, successEnvelope(resourceDraftAction(draft)));
      return;
    }

    if (path.endsWith("/remove-binary")) {
      if (rejectNextBinaryRemoval) {
        rejectNextBinaryRemoval = false;
        await fulfillJson(route, {
          code: "-1",
          data: {},
          message: "Binary revision conflict",
        }, 409);
        return;
      }
      const draft = resources.find((item) => item.resourceVersion === body.resourceVersion);
      if (!draft) return fulfillNotFound(route);
      const binaries = Array.isArray(draft.binaries) ? draft.binaries as JsonRecord[] : [];
      draft.binaries = binaries.filter((item) => item.binaryName !== body.binaryName);
      Object.assign(draft, {
        draftRevision: nextRevision(draft),
        updatedAt: 1_767_484_800_000,
      });
      await fulfillJson(route, successEnvelope({
        action: "REMOVED",
        binaryName: body.binaryName,
        byteSize: null,
        draftRevision: draft.draftRevision,
        resourceCode: draft.resourceCode,
        resourceVersion: draft.resourceVersion,
        validationErrors: [],
        validationStatus: "VALID",
      }));
      return;
    }

    if (path.endsWith("/publish-draft")) {
      const draft = resources.find((item) => item.resourceVersion === body.resourceVersion);
      if (!draft) return fulfillNotFound(route);
      resources.forEach((item) => {
        if (item.lifecycleStatus === "ACTIVE") item.lifecycleStatus = "HISTORICAL";
      });
      Object.assign(draft, {
        draftRevision: null,
        lifecycleStatus: "ACTIVE",
        publishedAt: 1_767_571_200_000,
        updatedAt: 1_767_571_200_000,
      });
      await fulfillJson(route, successEnvelope(resourceActivation(draft, "DRAFT_PUBLISHED")));
      return;
    }

    if (path.endsWith("/activate-version")) {
      const historical = resources.find((item) => item.resourceVersion === body.resourceVersion);
      if (!historical) return fulfillNotFound(route);
      resources.forEach((item) => {
        if (item.lifecycleStatus === "ACTIVE") item.lifecycleStatus = "HISTORICAL";
      });
      Object.assign(historical, {
        lifecycleStatus: "ACTIVE",
        publishedAt: 1_767_657_600_000,
        updatedAt: 1_767_657_600_000,
      });
      await fulfillJson(route, successEnvelope(resourceActivation(historical, "HISTORICAL_REACTIVATED")));
      return;
    }

    if (path.endsWith("/discard-draft")) {
      const index = resources.findIndex((item) => item.resourceVersion === body.resourceVersion);
      if (index < 0) return fulfillNotFound(route);
      const [draft] = resources.splice(index, 1);
      await fulfillJson(route, successEnvelope(resourceDraftAction(draft, true)));
      return;
    }

    await fulfillNotFound(route);
  });
}

test.beforeAll(async () => {
  frontend = await startResourceUiFrontend("/settings/resource-definitions", {
    VITE_TRON_RESOURCE_DATA_SOURCE: "api",
  });
});

test.afterAll(async () => {
  await stopResourceUiFrontend(frontend);
  frontend = undefined;
});

test("platform definition list renders through the Resource API page contract", async ({ page }) => {
  const requests: unknown[] = [];
  await routeResourceUiShellApi(page);
  await page.route("**/api/v1/resource-definitions/page", async (route) => {
    await captureJsonPageRequest(route, requests);
    await fulfillJson(route, successEnvelope({
      current: 1,
      records: [definitionRecord()],
      size: 15,
      total: 1,
    }));
  });
  await seedResourceUiShell(page);

  await page.goto(`${frontend!.baseUrl}/settings/resource-definitions`);
  await expect(page.getByRole("heading", { name: "Platform Resource Definitions" })).toBeVisible({ timeout: 15_000 });
  await expect(page.getByText("File", { exact: true })).toBeVisible();
  await expect.poll(() => requests.length).toBeGreaterThan(0);
  expect(requests).toContainEqual(expect.objectContaining({
    current: 1,
    lifecycleStatus: "ACTIVE",
    scopeKind: "PLATFORM",
    workspaceCode: null,
  }));
});

test("workspace resource list resolves its selected workspace through the Resource API", async ({ page }) => {
  const resourceRequests: unknown[] = [];
  const definitionRequests: unknown[] = [];
  await routeResourceUiShellApi(page);
  await page.route("**/api/v1/resources/page", async (route) => {
    await captureJsonPageRequest(route, resourceRequests);
    await fulfillJson(route, successEnvelope({
      current: 1,
      records: [resourceRecord()],
      size: 15,
      total: 1,
    }));
  });
  await page.route("**/api/v1/resource-definitions/page", async (route) => {
    await captureJsonPageRequest(route, definitionRequests);
    await fulfillJson(route, successEnvelope({
      current: 1,
      records: [definitionRecord({ sourceKind: "INHERITED" })],
      size: 50,
      total: 1,
    }));
  });
  await seedResourceUiShell(page);

  await page.goto(`${frontend!.baseUrl}/resources`);
  await expect(page.getByRole("heading", { name: "Resources" })).toBeVisible({ timeout: 15_000 });
  await expect(page.getByText("Release archive", { exact: true })).toBeVisible();
  await expect(page.locator("tbody tr").filter({ hasText: "release-archive" })).toContainText("INHERITED");
  await expect.poll(() => resourceRequests.length).toBeGreaterThan(0);
  await expect.poll(() => definitionRequests.length).toBeGreaterThan(0);
  expect(resourceRequests).toContainEqual(expect.objectContaining({
    lifecycleStatus: "ACTIVE",
    scopeKind: "WORKSPACE",
    workspaceCode: "TRON_CORE",
  }));
  expect(definitionRequests).toContainEqual(expect.objectContaining({
    lifecycleStatus: "ACTIVE",
    scopeKind: "WORKSPACE",
    workspaceCode: "TRON_CORE",
  }));
});

test("workspace definition list activates history against its local active override", async ({ page }) => {
  const requests: CapturedJsonRequest[] = [];
  const active = definitionRecord({
    definitionCode: "WORKSPACE_FILE",
    definitionVersion: "v3-A1a",
    name: "Workspace file",
    scopeCode: "TRON_CORE",
    scopeKind: "WORKSPACE",
    sourceKind: "OVERRIDE",
  });
  const historical = definitionRecord({
    definitionCode: "WORKSPACE_FILE",
    definitionVersion: "v2-H1a",
    lifecycleStatus: "HISTORICAL",
    name: "Workspace file",
    scopeCode: "TRON_CORE",
    scopeKind: "WORKSPACE",
    sourceKind: "LOCAL",
  });

  await routeResourceUiShellApi(page);
  await page.route("**/api/v1/resource-definitions/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    const body = jsonRequest(route);
    requests.push({ body, path });

    if (path.endsWith("/page")) {
      const records = body.lifecycleStatus === "HISTORICAL" ? [historical] : [active];
      await fulfillJson(route, successEnvelope({
        current: Number(body.current ?? 1),
        records,
        size: Number(body.size ?? 15),
        total: records.length,
      }));
      return;
    }

    if (path.endsWith("/activate-version")) {
      await fulfillJson(route, successEnvelope(definitionActivation(historical, "HISTORICAL_REACTIVATED")));
      return;
    }

    await fulfillNotFound(route);
  });
  await seedResourceUiShell(page);

  await page.goto(`${frontend!.baseUrl}/resource-definitions`);
  await expect(page.getByText("Workspace file", { exact: true })).toBeVisible({ timeout: 15_000 });
  await page.getByLabel("Filter resource definitions by lifecycle").click();
  await page.getByRole("option", { exact: true, name: "Historical" }).click();
  await expect(page.locator("tbody tr").filter({ hasText: "v2-H1a" })).toBeVisible();

  await page.getByLabel("Open actions for Workspace file v2-H1a").click();
  await page.getByRole("menuitem", { exact: true, name: "Activate version" }).click();

  await expect.poll(() => requests.filter((request) => request.path.endsWith("/activate-version")).length).toBe(1);
  const activation = requests.find((request) => request.path.endsWith("/activate-version"));
  expect(activation?.body).toMatchObject({
    definitionCode: "WORKSPACE_FILE",
    definitionVersion: "v2-H1a",
    expectedActiveDefinitionVersion: "v3-A1a",
    scopeKind: "WORKSPACE",
    workspaceCode: "TRON_CORE",
  });
  expect(requests).toContainEqual(expect.objectContaining({
    body: expect.objectContaining({
      definitionCode: "WORKSPACE_FILE",
      lifecycleStatus: "ACTIVE",
      scopeKind: "WORKSPACE",
      sourceKind: null,
      workspaceCode: "TRON_CORE",
    }),
    path: expect.stringMatching(/\/page$/),
  }));
});

test("workspace resource list activates history against its local active override", async ({ page }) => {
  const resourceRequests: CapturedJsonRequest[] = [];
  const active = resourceRecord({
    name: "Workspace archive",
    resourceCode: "workspace-archive",
    resourceVersion: "v3-A1a",
    scopeCode: "TRON_CORE",
    scopeKind: "WORKSPACE",
    sourceKind: "OVERRIDE",
  });
  const historical = resourceRecord({
    lifecycleStatus: "HISTORICAL",
    name: "Workspace archive",
    resourceCode: "workspace-archive",
    resourceVersion: "v2-H1a",
    scopeCode: "TRON_CORE",
    scopeKind: "WORKSPACE",
    sourceKind: "LOCAL",
  });

  await routeResourceUiShellApi(page);
  await page.route("**/api/v1/resource-definitions/page", async (route) => {
    const body = jsonRequest(route);
    await fulfillJson(route, successEnvelope({
      current: Number(body.current ?? 1),
      records: [definitionRecord({
        definitionCode: "FILE",
        scopeCode: "TRON_CORE",
        scopeKind: "WORKSPACE",
        sourceKind: "LOCAL",
      })],
      size: Number(body.size ?? 50),
      total: 1,
    }));
  });
  await page.route("**/api/v1/resources/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    const body = jsonRequest(route);
    resourceRequests.push({ body, path });

    if (path.endsWith("/page")) {
      const records = body.lifecycleStatus === "HISTORICAL" ? [historical] : [active];
      await fulfillJson(route, successEnvelope({
        current: Number(body.current ?? 1),
        records,
        size: Number(body.size ?? 15),
        total: records.length,
      }));
      return;
    }

    if (path.endsWith("/activate-version")) {
      await fulfillJson(route, successEnvelope(resourceActivation(historical, "HISTORICAL_REACTIVATED")));
      return;
    }

    await fulfillNotFound(route);
  });
  await seedResourceUiShell(page);

  await page.goto(`${frontend!.baseUrl}/resources`);
  await expect(page.getByText("Workspace archive", { exact: true })).toBeVisible({ timeout: 15_000 });
  await page.getByLabel("Filter resources by version status").click();
  await page.getByRole("option", { exact: true, name: "Historical" }).click();
  await expect(page.locator("tbody tr").filter({ hasText: "v2-H1a" })).toBeVisible();

  await page.getByLabel("Open actions for Workspace archive v2-H1a").click();
  await page.getByRole("menuitem", { exact: true, name: "Activate version" }).click();

  await expect.poll(() => resourceRequests.filter((request) => request.path.endsWith("/activate-version")).length).toBe(1);
  const activation = resourceRequests.find((request) => request.path.endsWith("/activate-version"));
  expect(activation?.body).toMatchObject({
    expectedActiveResourceVersion: "v3-A1a",
    resourceCode: "workspace-archive",
    resourceVersion: "v2-H1a",
    scopeKind: "WORKSPACE",
    workspaceCode: "TRON_CORE",
  });
  expect(resourceRequests).toContainEqual(expect.objectContaining({
    body: expect.objectContaining({
      lifecycleStatus: "ACTIVE",
      resourceCode: "workspace-archive",
      scopeKind: "WORKSPACE",
      sourceKind: null,
      workspaceCode: "TRON_CORE",
    }),
    path: expect.stringMatching(/\/page$/),
  }));
});

test("discarding the only last-page definition draft returns the list to the remaining page", async ({ page }) => {
  const requests: CapturedJsonRequest[] = [];
  const firstPageRecords = Array.from({ length: 15 }, (_, index) => definitionRecord({
    definitionCode: `DRAFT_${index + 1}`,
    definitionVersion: `v1-D${String(index + 1).padStart(2, "0")}`,
    draftRevision: "1",
    lifecycleStatus: "DRAFT",
    name: `Draft ${index + 1}`,
    scopeCode: "TRON_CORE",
    scopeKind: "WORKSPACE",
    sourceKind: "LOCAL",
  }));
  const lastPageDraft = definitionRecord({
    definitionCode: "LAST_PAGE_DRAFT",
    definitionVersion: "v2-D2a",
    draftRevision: "3",
    lifecycleStatus: "DRAFT",
    name: "Last page draft",
    scopeCode: "TRON_CORE",
    scopeKind: "WORKSPACE",
    sourceKind: "LOCAL",
  });
  let discarded = false;

  await routeResourceUiShellApi(page);
  await page.route("**/api/v1/resource-definitions/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    const body = jsonRequest(route);
    requests.push({ body, path });

    if (path.endsWith("/page")) {
      const isDraftPage = body.lifecycleStatus === "DRAFT";
      const current = Number(body.current ?? 1);
      const records = !isDraftPage
        ? []
        : current === 2 && !discarded
          ? [lastPageDraft]
          : firstPageRecords;
      await fulfillJson(route, successEnvelope({
        current,
        records,
        size: Number(body.size ?? 15),
        total: isDraftPage ? (discarded ? 15 : 16) : 0,
      }));
      return;
    }

    if (path.endsWith("/discard-draft")) {
      discarded = true;
      await fulfillJson(route, successEnvelope(definitionDraftAction(lastPageDraft, true)));
      return;
    }

    await fulfillNotFound(route);
  });
  await seedResourceUiShell(page);

  await page.goto(`${frontend!.baseUrl}/resource-definitions`);
  await page.getByLabel("Filter resource definitions by lifecycle").click();
  await page.getByRole("option", { exact: true, name: "Draft" }).click();
  await expect(page.getByText("Draft 1", { exact: true })).toBeVisible({ timeout: 15_000 });

  await page.getByLabel("Next page").click();
  await expect(page.getByText("Last page draft", { exact: true })).toBeVisible();
  await page.getByLabel("Open actions for Last page draft v2-D2a").click();
  await page.getByRole("menuitem", { exact: true, name: "Discard draft" }).click();

  await expect.poll(() => requests.filter((request) => request.path.endsWith("/discard-draft")).length).toBe(1);
  expect(requests.find((request) => request.path.endsWith("/discard-draft"))?.body).toMatchObject({
    definitionCode: "LAST_PAGE_DRAFT",
    definitionVersion: "v2-D2a",
    expectedDraftRevision: "3",
  });
  await expect(page.getByLabel("Go to page")).toHaveValue("1");
  await expect(page.getByText("Draft 1", { exact: true })).toBeVisible();
  await expect(page.getByText("Last page draft", { exact: true })).toHaveCount(0);
});

test("platform definition editor drives draft lifecycle actions through Resource API requests", async ({ page }) => {
  const requests: CapturedJsonRequest[] = [];
  const records: JsonRecord[] = [definitionRecord({
    attributesSchema: cloneRecord(draft7TitleSchema),
    binarySchema: cloneRecord(permissiveBinarySchema),
    binaryStorageProfile: "@default",
    definitionCode: "FILE",
    definitionVersion: "v1-A1a",
    description: "Platform file resource",
    name: "File",
  })];
  await routeResourceUiShellApi(page);
  await routeDefinitionEditorApi(page, records, requests);
  await seedResourceUiShell(page);

  await page.goto(`${frontend!.baseUrl}/settings/resource-definitions/detail?definitionCode=FILE&definitionVersion=v1-A1a`);
  await expect(page.getByRole("heading", { exact: true, name: "File" })).toBeVisible({ timeout: 15_000 });
  await expect(page.getByText("ACTIVE", { exact: true }).first()).toBeVisible();

  await page.getByRole("button", { exact: true, name: "Edit" }).click();
  await expect(page.getByText("DRAFT", { exact: true }).first()).toBeVisible();
  await expect(page.getByText("v2-D2a", { exact: true }).first()).toBeVisible();
  await expect.poll(() => requests.filter((request) => request.path.endsWith("/create-draft")).length).toBe(1);
  expect(requests.find((request) => request.path.endsWith("/create-draft"))?.body).toMatchObject({
    definitionCode: "FILE",
    scopeKind: "PLATFORM",
    sourceDefinitionVersion: "v1-A1a",
    workspaceCode: null,
  });

  await page.getByLabel("Edit description").click();
  await page.getByRole("textbox", { exact: true, name: "Description" }).fill("Updated platform file resource");
  await page.getByRole("button", { exact: true, name: "Save draft" }).click();
  await expect.poll(() => requests.filter((request) => request.path.endsWith("/update-draft")).length).toBe(1);
  expect(requests.find((request) => request.path.endsWith("/update-draft"))?.body).toMatchObject({
    definitionCode: "FILE",
    definitionVersion: "v2-D2a",
    description: "Updated platform file resource",
    expectedDraftRevision: "1",
    scopeKind: "PLATFORM",
    workspaceCode: null,
  });

  await page.getByRole("button", { exact: true, name: "Publish" }).click();
  await expect.poll(() => requests.filter((request) => request.path.endsWith("/publish-draft")).length).toBe(1);
  const definitionUpdates = requests.filter((request) => request.path.endsWith("/update-draft"));
  expect(definitionUpdates).toHaveLength(2);
  expect(definitionUpdates[1].body).toMatchObject({ expectedDraftRevision: "2" });
  expect(requests.find((request) => request.path.endsWith("/publish-draft"))?.body).toMatchObject({
    definitionCode: "FILE",
    definitionVersion: "v2-D2a",
    expectedDraftRevision: "3",
  });
  await expect(page.getByText("ACTIVE", { exact: true }).first()).toBeVisible();

  await page.getByRole("tab", { name: /Version history/ }).click();
  await page.locator("tbody tr").filter({ hasText: "v1-A1a" }).click();
  await page.getByRole("button", { exact: true, name: "Activate" }).click();
  await expect.poll(() => requests.filter((request) => request.path.endsWith("/activate-version")).length).toBe(1);
  expect(requests.find((request) => request.path.endsWith("/activate-version"))?.body).toMatchObject({
    definitionCode: "FILE",
    definitionVersion: "v1-A1a",
    expectedActiveDefinitionVersion: "v2-D2a",
  });

  await page.getByRole("button", { exact: true, name: "Edit" }).click();
  await expect(page.getByText("v3-D3a", { exact: true }).first()).toBeVisible();
  await page.getByLabel("More definition actions").click();
  await page.getByRole("menuitem", { exact: true, name: "Discard draft" }).click();
  await expect.poll(() => requests.filter((request) => request.path.endsWith("/discard-draft")).length).toBe(1);
  expect(requests.find((request) => request.path.endsWith("/discard-draft"))?.body).toMatchObject({
    definitionCode: "FILE",
    definitionVersion: "v3-D3a",
    expectedDraftRevision: "1",
  });
});

test("workspace resource editor commits staged binary changes as one ACTIVE version and retains explicit drafts", async ({ page }) => {
  const resourceRequests: CapturedJsonRequest[] = [];
  const multipartRequests: CapturedMultipartRequest[] = [];
  const definitions: JsonRecord[] = [definitionRecord({
    attributesSchema: cloneRecord(draft7TitleSchema),
    binarySchema: { ...permissiveBinarySchema, maxCount: 3 },
    binaryStorageProfile: "@default",
    definitionCode: "FILE",
    definitionVersion: "v1-A1a",
    scopeCode: "TRON_CORE",
    scopeKind: "WORKSPACE",
    sourceKind: "LOCAL",
  })];
  const resources: JsonRecord[] = [resourceRecord({
    attributes: { title: "Initial release" },
    attributesSchema: cloneRecord(draft7TitleSchema),
    binaries: [
      { binaryName: "existing.bin", byteSize: 5 },
      { binaryName: "remove.bin", byteSize: 7 },
    ],
    binarySchema: { ...permissiveBinarySchema, maxCount: 3 },
    binaryStorageProfile: "@default",
    definitionScopeCode: "TRON_CORE",
    definitionScopeKind: "WORKSPACE",
    lifecycleStatus: "ACTIVE",
    resourceVersion: "v2-A1b",
    scopeCode: "TRON_CORE",
    scopeKind: "WORKSPACE",
    sourceKind: "LOCAL",
  })];
  await routeResourceUiShellApi(page);
  await routeResourceEditorApi(page, resources, definitions, resourceRequests, multipartRequests);
  await seedResourceUiShell(page);

  await page.goto(`${frontend!.baseUrl}/resources/detail?resourceCode=release-archive&resourceVersion=v2-A1b`);
  await expect(page.getByRole("heading", { exact: true, name: "Release archive" })).toBeVisible({ timeout: 15_000 });
  await expect(page.getByRole("heading", { name: "Binary data" })).toBeVisible();

  await page.getByRole("button", { exact: true, name: "Edit" }).click();
  await expect(page.getByText("ACTIVE", { exact: true }).first()).toBeVisible();
  await expect(page.getByRole("button", { exact: true, name: "Save" })).toBeVisible();
  await expect(page.getByRole("button", { exact: true, name: "Cancel" })).toBeVisible();
  expect(resourceRequests.filter((request) => request.path.endsWith("/create-draft"))).toHaveLength(0);

  await page.getByLabel("Edit description").click();
  await page.getByRole("textbox", { exact: true, name: "Description" }).fill("Updated release archive");

  await page.locator('input[type="file"]').first().setInputFiles({
    buffer: Buffer.from("new opaque bytes"),
    mimeType: "application/octet-stream",
    name: "new.bin",
  });
  await expect(page.getByLabel("Binary name")).toHaveValue("new.bin");
  await page.getByRole("button", { exact: true, name: "Add" }).click();
  await expect(page.getByText("new.bin", { exact: true })).toBeVisible();
  expect(multipartRequests).toHaveLength(0);

  await page.locator('input[type="file"][id*="replace"]').first().setInputFiles({
    buffer: Buffer.from("replacement opaque bytes"),
    mimeType: "application/octet-stream",
    name: "replacement.bin",
  });
  expect(multipartRequests).toHaveLength(0);
  await page.getByLabel("Remove remove.bin").click();
  await expect(page.getByText("remove.bin", { exact: true })).toHaveCount(0);
  expect(resourceRequests.filter((request) => request.path.endsWith("/remove-binary"))).toHaveLength(0);

  await page.getByRole("button", { exact: true, name: "Save" }).click();
  await expect.poll(() => multipartRequests.length).toBe(1);
  const commit = multipartRequests[0];
  expect(commit.path).toMatch(/\/api\/v1\/resources\/update$/);
  expect(commit.contentType).toMatch(/^multipart\/form-data; boundary=/);
  expect(multipartCommand(commit.rawBody)).toMatchObject({
    allowInheritance: true,
    attributes: { title: "Initial release" },
    definitionCode: null,
    description: "Updated release archive",
    expectedActiveResourceVersion: "v2-A1b",
    removedBinaryNames: ["remove.bin"],
    resourceCode: "release-archive",
    scopeKind: "WORKSPACE",
    workspaceCode: "TRON_CORE",
  });
  expect(multipartBinaryNames(commit.rawBody).sort()).toEqual(["existing.bin", "new.bin"]);
  expect(commit.rawBody).toContain("new opaque bytes");
  expect(commit.rawBody).toContain("replacement opaque bytes");
  expect(resourceRequests.filter((request) => (
    request.path.endsWith("/create-draft")
      || request.path.endsWith("/update-draft")
      || request.path.endsWith("/upload-binary")
      || request.path.endsWith("/remove-binary")
      || request.path.endsWith("/publish-draft")
  ))).toHaveLength(0);
  await expect(page.getByText("ACTIVE", { exact: true }).first()).toBeVisible();
  await expect(page.getByText("v3-C3a", { exact: true }).first()).toBeVisible();

  await page.getByLabel("More resource actions").click();
  await page.getByRole("menuitem", { exact: true, name: "Save unfinished content" }).click();
  await expect(page.getByText("DRAFT", { exact: true }).first()).toBeVisible();
  await expect.poll(() => resourceRequests.filter((request) => request.path.endsWith("/create-draft")).length).toBe(1);
  expect(resourceRequests.find((request) => request.path.endsWith("/create-draft"))?.body).toMatchObject({
    resourceCode: "release-archive",
    sourceResourceVersion: "v3-C3a",
  });
  await page.getByLabel("More resource actions").click();
  await page.getByRole("menuitem", { exact: true, name: "Discard draft" }).click();
  await expect.poll(() => resourceRequests.filter((request) => request.path.endsWith("/discard-draft")).length).toBe(1);
  await expect(page).toHaveURL(/resourceVersion=v3-C3a/);
  await expect(page.getByText("DRAFT", { exact: true })).toHaveCount(0);
  await expect(page.getByRole("button", { exact: true, name: "Edit" })).toBeVisible();

  await page.getByRole("tab", { name: /Version history/ }).click();
  await page.locator("tbody tr").filter({ hasText: "v2-A1b" }).click();
  await page.getByRole("button", { exact: true, name: "Activate" }).click();
  await expect.poll(() => resourceRequests.filter((request) => request.path.endsWith("/activate-version")).length).toBe(1);
  expect(resourceRequests.find((request) => request.path.endsWith("/activate-version"))?.body).toMatchObject({
    expectedActiveResourceVersion: "v3-C3a",
    resourceCode: "release-archive",
    resourceVersion: "v2-A1b",
  });
});

test("a new workspace resource creates its first ACTIVE version with staged binaries", async ({ page }) => {
  const resourceRequests: CapturedJsonRequest[] = [];
  const multipartRequests: CapturedMultipartRequest[] = [];
  const definitions: JsonRecord[] = [definitionRecord({
    attributesSchema: cloneRecord(draft7TitleSchema),
    binarySchema: cloneRecord(permissiveBinarySchema),
    binaryStorageProfile: "@default",
    definitionCode: "FILE",
    definitionVersion: "v1-A1a",
    scopeCode: "TRON_CORE",
    scopeKind: "WORKSPACE",
    sourceKind: "LOCAL",
  })];
  const resources: JsonRecord[] = [];
  await routeResourceUiShellApi(page);
  await routeResourceEditorApi(page, resources, definitions, resourceRequests, multipartRequests);
  await seedResourceUiShell(page);

  await page.goto(`${frontend!.baseUrl}/resources/new`);
  await page.getByLabel("Resource name").fill("Release bundle");
  await page.getByLabel("Resource code").fill("release-bundle");
  await page.getByLabel("Resource type").click();
  await page.getByRole("option", { exact: true, name: "File · v1-A1a" }).click();
  await page.getByLabel("Title").fill("Release bundle");
  await page.locator('input[type="file"]').first().setInputFiles({
    buffer: Buffer.from("new release bytes"),
    mimeType: "application/octet-stream",
    name: "release.bin",
  });
  await page.getByRole("button", { exact: true, name: "Add" }).click();
  await expect(page.getByText("release.bin", { exact: true })).toBeVisible();
  expect(multipartRequests).toHaveLength(0);

  await page.getByRole("button", { exact: true, name: "Create" }).click();
  await expect.poll(() => multipartRequests.length).toBe(1);
  const commit = multipartRequests[0];
  expect(commit.path).toMatch(/\/api\/v1\/resources\/create$/);
  expect(multipartCommand(commit.rawBody)).toMatchObject({
    attributes: { title: "Release bundle" },
    definitionCode: "FILE",
    expectedActiveResourceVersion: null,
    name: "Release bundle",
    resourceCode: "release-bundle",
    scopeKind: "WORKSPACE",
    workspaceCode: "TRON_CORE",
  });
  expect(multipartBinaryNames(commit.rawBody)).toEqual(["release.bin"]);
  expect(resourceRequests.filter((request) => request.path.endsWith("/create-draft"))).toHaveLength(0);
  await expect(page.getByText("ACTIVE", { exact: true }).first()).toBeVisible();
  await expect(page.getByText("v3-C3a", { exact: true }).first()).toBeVisible();
});
