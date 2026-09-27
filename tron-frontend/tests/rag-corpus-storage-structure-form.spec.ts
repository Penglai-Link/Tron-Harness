import { routeActiveProfileApi } from "./support/resource-ui-harness";
import {
  expect,
  test,
  type Locator,
  type Page,
} from "@playwright/test";
import {
  spawnManagedProcess,
  stopManagedProcess,
  type ManagedProcess,
} from "./support/managed-process";
import { createServer } from "node:net";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const thisFilePath = fileURLToPath(import.meta.url);
const frontendRoot = resolve(thisFilePath, "..", "..");
const mockStorageKey = "tron.rag.mock.corpora";

let frontendProcess: ManagedProcess | undefined;
let frontendLogs = "";
let frontendBaseUrl = "";

type CapturedRequest = { method: string; pathname: string };

test.describe.configure({ timeout: 60_000 });

function appendFrontendLog(chunk: Buffer) {
  frontendLogs = `${frontendLogs}${chunk.toString()}`.slice(-5000);
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
      // Vite is still warming up.
    }

    await new Promise((resolveDelay) => setTimeout(resolveDelay, 250));
  }

  throw new Error(`Frontend dev server did not become ready.\n${frontendLogs}`);
}

async function captureApiRequests(page: Page, requests: CapturedRequest[]) {
  await page.route("**/api/v1/**", async (route) => {
    const request = route.request();
    const url = new URL(request.url());
    requests.push({ method: request.method(), pathname: url.pathname });
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ code: "SUCCESS", message: "ok", data: {} }),
    });
  });
}

function controlByAriaLabel(page: Page, label: string) {
  return page.locator(`[aria-label="${label}"]`);
}

async function chooseSelectOption(page: Page, control: Locator, optionName: string) {
  await control.click();
  await expect(control).toHaveAttribute("aria-expanded", "true");
  await page.getByRole("option", { name: optionName, exact: true }).click();
  await expect(control).toHaveAttribute("aria-expanded", "false");
}

async function selectOptionTexts(page: Page, control: Locator) {
  await control.click();
  await expect(control).toHaveAttribute("aria-expanded", "true");
  const texts = await page.getByRole("option").allTextContents();
  await page.keyboard.press("Escape");
  await expect(control).toHaveAttribute("aria-expanded", "false");
  return texts;
}

async function openCreatePage(page: Page) {
  await routeActiveProfileApi(page);
  await page.goto(`${frontendBaseUrl}/rag`);
  await expect(page.getByRole("tab", { name: "RAG", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "New Corpus" }).click();
  await expect(page).toHaveURL(/\/rag\/corpora\/new$/);
  await expect(page.getByRole("navigation", { name: "Breadcrumb" }).filter({ hasText: "New corpus" })).toBeVisible();
}

async function openAdvancedStorage(page: Page) {
  if (await controlByAriaLabel(page, "Storage target 1 table").count() === 0) {
    await page.getByRole("button", { name: "Advanced storage settings" }).click();
  }
  await expect(controlByAriaLabel(page, "Storage target 1 table")).toBeVisible();
}

async function enableSummaryTree(page: Page) {
  const summaryTreeToggle = page.getByRole("switch", { name: "Enable summary tree" });
  if ((await summaryTreeToggle.getAttribute("aria-checked")) !== "true") {
    await summaryTreeToggle.click();
  }
  await expect(summaryTreeToggle).toHaveAttribute("aria-checked", "true");
}

async function openFieldConfig(page: Page, tabName: "Source fields" | "Text unit fields" | "Filter fields") {
  if (await page.getByRole("tab", { name: "Source fields" }).count() === 0) {
    await page.getByRole("button", { name: "Configure fields" }).click();
  }
  await page.getByRole("tab", { name: tabName }).click();
}

test.beforeAll(async () => {
  const port = await reservePort();
  frontendBaseUrl = `http://127.0.0.1:${port}`;
  frontendProcess = spawnManagedProcess("npm", ["run", "dev", "--", "--host", "127.0.0.1", "--port", String(port)], {
    cwd: frontendRoot,
    env: {
      ...process.env,
      CI: "1",
      VITE_TRON_API_BASE_URL: "",
      VITE_TRON_ADMIN_DATA_SOURCE: "mock",
      VITE_TRON_AUTH_DATA_SOURCE: "mock",
      VITE_TRON_DATA_SOURCE: "mock",
      VITE_TRON_RAG_DATA_SOURCE: "mock",
    },
    stdio: "pipe",
  });
  frontendProcess.stdout.on("data", appendFrontendLog);
  frontendProcess.stderr.on("data", appendFrontendLog);
  await waitForFrontend(`${frontendBaseUrl}/rag`);
});

test.afterAll(async () => {
  try {
    await stopManagedProcess(frontendProcess);
  } finally {
    frontendProcess = undefined;
  }
});

test.beforeEach(async ({ page }) => {
  await page.addInitScript((key) => {
    window.localStorage.setItem("tron.locale", "en-US");
    window.localStorage.removeItem(key);
  }, mockStorageKey);
});

test("creates target-first mixed-media corpus config and derives the registry summary", async ({ page }) => {
  const requests: CapturedRequest[] = [];
  await captureApiRequests(page, requests);

  await openCreatePage(page);

  await expect(page.getByRole("heading", { name: "Storage binding" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Field configuration" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Capabilities" })).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "Filter Schema" })).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "Field Mapping" })).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "Backend Payload Preview" })).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "Storage targets" })).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "Segment bindings" })).toHaveCount(0);
  await expect(controlByAriaLabel(page, "Storage target 1 id")).toHaveCount(0);
  await expect(controlByAriaLabel(page, "Filter field 1 name")).toHaveCount(0);
  const sectionHeadings = await page.locator("h2").allTextContents();
  expect(sectionHeadings.indexOf("Storage binding")).toBeLessThan(sectionHeadings.indexOf("Field configuration"));

  await page.getByLabel("Corpus code").fill("support-notes");
  await page.getByLabel("Corpus name").fill("Support Notes");
  await page.getByLabel("Description").fill("Support playbooks and incident review notes.");
  await expect(page.getByText("3 targets")).toBeVisible();
  await openAdvancedStorage(page);
  await expect(controlByAriaLabel(page, "Storage target 1 table")).toHaveValue("rag_support_notes_sources");
  await expect(controlByAriaLabel(page, "Storage target 2 table")).toHaveValue("rag_support_notes_text_units");
  await expect(controlByAriaLabel(page, "Storage target 3 table")).toHaveValue("rag_support_notes_vectors");
  await expect(controlByAriaLabel(page, "Storage target 4 table")).toHaveCount(0);
  await expect(controlByAriaLabel(page, "Source target")).toContainText("rag_support_notes_sources");
  await expect(controlByAriaLabel(page, "Text unit target")).toContainText("rag_support_notes_text_units");
  await expect(controlByAriaLabel(page, "Fulltext index target")).toContainText("rag_support_notes_text_units");
  await expect(controlByAriaLabel(page, "Vector target")).toContainText("rag_support_notes_vectors");
  await expect(controlByAriaLabel(page, "Summary tree target")).toHaveCount(0);
  await enableSummaryTree(page);
  await expect(controlByAriaLabel(page, "Storage target 4 table")).toHaveValue("rag_support_notes_tree_nodes");
  await expect(controlByAriaLabel(page, "Summary tree target")).toContainText("rag_support_notes_tree_nodes");
  await chooseSelectOption(page, controlByAriaLabel(page, "Fulltext index target"), "rag_support_notes_vectors");
  await chooseSelectOption(page, controlByAriaLabel(page, "Storage target 3 backend"), "Graph storage");
  await controlByAriaLabel(page, "Storage target 3 graph namespace").fill("rag_doc");
  await controlByAriaLabel(page, "Storage target 3 node label").fill("RagDocUnit");
  await expect(page.getByLabel("Enable graph capability")).toHaveCount(0);

  await openFieldConfig(page, "Source fields");
  await page.getByRole("button", { name: "Add source field" }).click();
  await controlByAriaLabel(page, "Source mapping 1 logical name").fill("documentCode");
  await controlByAriaLabel(page, "Source mapping 1 column").fill("document_code");
  await openFieldConfig(page, "Filter fields");
  await chooseSelectOption(page, controlByAriaLabel(page, "Filter field 1 name"), "documentCode");
  await chooseSelectOption(page, controlByAriaLabel(page, "Filter field 1 type"), "String");
  await controlByAriaLabel(page, "Filter field 1 operators").fill("eq, in");

  await page.getByRole("button", { name: "Save" }).click();

  await expect(page).toHaveURL(/\/rag$/);
  await expect(page.getByRole("cell", { name: "Support Notes support-notes" })).toBeVisible();

  const storedRecords = await page.evaluate((key) => JSON.parse(window.localStorage.getItem(key) ?? "[]"), mockStorageKey);
  expect(storedRecords).toEqual(expect.arrayContaining([
    expect.objectContaining({
      code: "support-notes",
      config: expect.objectContaining({
        storageBinding: expect.objectContaining({
          mode: "TARGETED",
          bindings: expect.objectContaining({
            diagnostics: "support-notes-diagnostics",
            fulltext: "vector-store",
            source: "source-store",
            textUnit: "text-unit-store",
            tree: "tree-store",
            vector: "vector-store",
          }),
          targets: expect.arrayContaining([
            expect.objectContaining({
              backend: "SQLITE",
              id: "source-store",
              layout: "TABLE",
              table: "rag_support_notes_sources",
            }),
            expect.objectContaining({
              backend: "SQLITE",
              id: "text-unit-store",
              layout: "TABLE",
              table: "rag_support_notes_text_units",
            }),
            expect.objectContaining({
              backend: "GRAPH",
              id: "vector-store",
              layout: "GRAPH_SINGLE_NODE",
              namespace: "rag_doc",
              nodeLabel: "RagDocUnit",
            }),
            expect.objectContaining({
              backend: "SQLITE",
              id: "tree-store",
              layout: "TABLE",
              table: "rag_support_notes_tree_nodes",
            }),
            expect.objectContaining({
              backend: "SQLITE",
              id: "support-notes-diagnostics",
              layout: "TABLE",
              table: "rag_support_notes_diagnostics",
            }),
          ]),
        }),
      }),
    }),
  ]));
  expect(requests.filter((request) => request.pathname.includes("/api/v1/rag/corpora"))).toEqual([]);
});

test("validates target definitions and missing bindings", async ({ page }) => {
  await openCreatePage(page);

  await page.getByLabel("Corpus code").fill("broken-notes");
  await page.getByLabel("Corpus name").fill("Broken Notes");
  await openAdvancedStorage(page);
  await controlByAriaLabel(page, "Storage target 1 table").fill("bad-table");
  await controlByAriaLabel(page, "Storage target 2 table").fill("");
  await page.getByRole("button", { name: "Remove storage target 3" }).click();
  await openFieldConfig(page, "Source fields");
  await page.getByRole("button", { name: "Add source field" }).click();
  await controlByAriaLabel(page, "Source mapping 1 logical name").fill("missingField");
  await controlByAriaLabel(page, "Source mapping 1 column").fill("missing_column");
  await openFieldConfig(page, "Text unit fields");
  await page.getByRole("button", { name: "Add text unit field" }).click();
  await controlByAriaLabel(page, "Text unit mapping 1 logical name").fill("missingText");
  await controlByAriaLabel(page, "Text unit mapping 1 column").fill("missing_column");
  await openFieldConfig(page, "Filter fields");
  await chooseSelectOption(page, controlByAriaLabel(page, "Filter field 1 name"), "missingField");
  await page.getByRole("button", { name: "Save" }).click();

  await expect(page.getByText("Storage target 1 table name must start with rag_ and use only lowercase letters, numbers, or underscores.")).toBeVisible();
  await expect(page.getByText("Storage target 2 SQLite table name is required.")).toBeVisible();
  await expect(page.getByText("Vector storage target is required.")).toBeVisible();
  await openFieldConfig(page, "Filter fields");
  await expect(page.getByText("Filter fields must resolve to columns in a bound storage target.")).toBeVisible();
  await openFieldConfig(page, "Source fields");
  await expect(page.getByText("Source mapping columns must exist in the bound source storage target.")).toBeVisible();
  await openFieldConfig(page, "Text unit fields");
  await expect(page.getByText("Text unit mapping columns must exist in the bound text unit storage target.")).toBeVisible();
  await expect(page).toHaveURL(/\/rag\/corpora\/new$/);
});

test("infers graph capability from a graph-backed business segment binding", async ({ page }) => {
  await openCreatePage(page);

  await page.getByLabel("Corpus code").fill("graph-notes");
  await page.getByLabel("Corpus name").fill("Graph Notes");
  await expect(page.getByLabel("Enable graph capability")).toHaveCount(0);
  await openAdvancedStorage(page);
  await page.getByRole("button", { name: "Add custom storage" }).click();
  await chooseSelectOption(page, controlByAriaLabel(page, "Storage target 4 backend"), "Graph storage");
  await controlByAriaLabel(page, "Storage target 4 graph namespace").fill("rag_doc");
  await controlByAriaLabel(page, "Storage target 4 node label").fill("RagDocUnit");
  await chooseSelectOption(page, controlByAriaLabel(page, "Vector target"), "rag_doc/RagDocUnit");
  await page.getByRole("button", { name: "Save" }).click();

  await expect(page).toHaveURL(/\/rag$/);
  const storedRecords = await page.evaluate((key) => JSON.parse(window.localStorage.getItem(key) ?? "[]"), mockStorageKey);
  expect(storedRecords).toEqual(expect.arrayContaining([
    expect.objectContaining({
      code: "graph-notes",
      config: expect.objectContaining({
        capabilities: expect.objectContaining({ graph: true }),
        storageBinding: expect.objectContaining({
          bindings: expect.objectContaining({ vector: "target-4" }),
        }),
      }),
    }),
  ]));
});

test("starts with three default storage targets and can add more", async ({ page }) => {
  await openCreatePage(page);

  await openAdvancedStorage(page);
  const addTargetButton = page.getByRole("button", { name: "Add custom storage" });
  await expect(addTargetButton).toBeEnabled();
  await expect(controlByAriaLabel(page, "Storage target 1 table")).toBeVisible();
  await expect(controlByAriaLabel(page, "Storage target 2 table")).toBeVisible();
  await expect(controlByAriaLabel(page, "Storage target 3 table")).toBeVisible();
  await expect(controlByAriaLabel(page, "Storage target 4 table")).toHaveCount(0);
  await expect(controlByAriaLabel(page, "Storage target 1 id")).toHaveCount(0);
  await enableSummaryTree(page);
  await expect(controlByAriaLabel(page, "Storage target 4 table")).toBeVisible();
  await page.getByRole("button", { name: "Add custom storage" }).click();

  await expect(controlByAriaLabel(page, "Storage target 5 table")).toBeVisible();
  await expect(addTargetButton).toBeEnabled();
  await expect(controlByAriaLabel(page, "Storage target 5 id")).toHaveCount(0);
});

test("keeps at least one storage target and shows physical target names in segment bindings", async ({ page }) => {
  await openCreatePage(page);

  await page.getByLabel("Corpus code").fill("test");
  await openAdvancedStorage(page);
  await expect(controlByAriaLabel(page, "Source target")).toContainText("rag_test_sources");
  await expect(controlByAriaLabel(page, "Text unit target")).toContainText("rag_test_text_units");
  await expect(controlByAriaLabel(page, "Vector target")).toContainText("rag_test_vectors");
  await expect(controlByAriaLabel(page, "Summary tree target")).toHaveCount(0);
  const sourceTargetOptionTexts = await selectOptionTexts(page, controlByAriaLabel(page, "Source target"));
  expect(sourceTargetOptionTexts.join(" ")).not.toContain("Storage Target");

  await page.getByRole("button", { name: "Remove storage target 3" }).click();
  await expect(page.getByRole("button", { name: /Remove storage target/ })).toHaveCount(2);
  await page.getByRole("button", { name: "Remove storage target 2" }).click();

  await expect(controlByAriaLabel(page, "Storage target 1 table")).toHaveValue("rag_test_sources");
  await expect(controlByAriaLabel(page, "Source target")).toContainText("rag_test_sources");
  await expect(page.getByRole("button", { name: /Remove storage target/ })).toHaveCount(0);
});

test("hides contract mappings by default while keeping custom rows removable", async ({ page }) => {
  await openCreatePage(page);

  await expect(controlByAriaLabel(page, "Source mapping 1 logical name")).toHaveCount(0);
  await expect(controlByAriaLabel(page, "Text unit mapping 1 logical name")).toHaveCount(0);
  await openFieldConfig(page, "Source fields");
  await page.getByRole("button", { name: "Show source contract fields" }).click();
  await expect(controlByAriaLabel(page, "Source contract field 1 logical name")).toBeDisabled();
  await expect(page.locator('input[aria-label$="logical name"][value="sourceType"]')).toHaveCount(0);
  await expect(page.locator('input[aria-label$="logical name"][value="status"]')).toBeVisible();
  await expect(page.locator('input[aria-label$="logical name"][value="title"]')).toBeVisible();
  await expect(page.locator('input[aria-label$="logical name"][value="metadata"]')).toBeVisible();
  await page.getByRole("button", { name: "Hide source contract fields" }).click();

  await page.getByRole("button", { name: "Add source field" }).click();
  await controlByAriaLabel(page, "Source mapping 1 logical name").fill("documentCode");
  await controlByAriaLabel(page, "Source mapping 1 column").fill("document_code");
  await page.getByRole("button", { name: "Remove Source mapping 1" }).click();

  await openFieldConfig(page, "Text unit fields");
  await page.getByRole("button", { name: "Show text unit contract fields" }).click();
  await expect(controlByAriaLabel(page, "Text unit contract field 1 logical name")).toBeDisabled();
  await expect(page.locator('input[aria-label$="logical name"][value="textUnitStatus"]')).toBeVisible();
  await page.getByRole("button", { name: "Hide text unit contract fields" }).click();
  await page.getByRole("button", { name: "Add text unit field" }).click();
  await controlByAriaLabel(page, "Text unit mapping 1 logical name").fill("documentCode");
  await controlByAriaLabel(page, "Text unit mapping 1 column").fill("document_code");
  await page.getByRole("button", { name: "Remove Text unit mapping 1" }).click();

  await openFieldConfig(page, "Filter fields");
  await page.getByRole("button", { name: "Remove filter field 1" }).click();

  await expect(controlByAriaLabel(page, "Source mapping 1 logical name")).toHaveCount(0);
  await expect(controlByAriaLabel(page, "Text unit mapping 1 logical name")).toHaveCount(0);
  await expect(controlByAriaLabel(page, "Filter field 1 name")).toHaveCount(0);
  await openFieldConfig(page, "Source fields");
  await expect(page.getByRole("button", { name: "Add source field" })).toBeVisible();
  await openFieldConfig(page, "Text unit fields");
  await expect(page.getByRole("button", { name: "Add text unit field" })).toBeVisible();
  await openFieldConfig(page, "Filter fields");
  await expect(page.getByRole("button", { name: "Add filter field" })).toBeVisible();

  await page.getByRole("button", { name: "Save" }).click();

  await expect(page.getByText("At least one filter schema field is required.")).toBeVisible();
  await expect(page.getByText("At least one source field mapping is required.")).toHaveCount(0);
  await expect(page.getByText("At least one text-unit field mapping is required.")).toHaveCount(0);
});

test("edits seeded project document storage structure with target reuse in preview", async ({ page }) => {
  await routeActiveProfileApi(page);
  await page.goto(`${frontendBaseUrl}/rag`);
  await page.getByRole("cell", { name: "Project Document project-doc" }).click();
  await expect(page).toHaveURL(/\/rag\/corpora\/project-doc\/edit$/);

  await expect(page.getByLabel("Corpus code")).toHaveValue("project-doc");
  await expect(controlByAriaLabel(page, "Storage target 1 table")).toHaveCount(0);
  await openAdvancedStorage(page);
  await expect(controlByAriaLabel(page, "Storage target 1 id")).toHaveCount(0);
  await expect(controlByAriaLabel(page, "Storage target 1 table")).toHaveValue("rag_project_doc_sources");
  await expect(controlByAriaLabel(page, "Storage target 2 table")).toHaveValue("rag_project_doc_text_units");
  await expect(controlByAriaLabel(page, "Source target")).toContainText("rag_project_doc_sources");
  await expect(controlByAriaLabel(page, "Text unit target")).toContainText("rag_project_doc_text_units");
  await expect(controlByAriaLabel(page, "Fulltext index target")).toContainText("rag_project_doc_text_units");
  await expect(controlByAriaLabel(page, "Vector target")).toContainText("rag_project_doc_text_units");

  await chooseSelectOption(page, controlByAriaLabel(page, "Storage target 2 backend"), "Graph storage");
  await controlByAriaLabel(page, "Storage target 2 graph namespace").fill("rag_doc");
  await controlByAriaLabel(page, "Storage target 2 node label").fill("RagDocUnit");
  await expect(controlByAriaLabel(page, "Text unit target")).toContainText("rag_doc/RagDocUnit");
  await expect(controlByAriaLabel(page, "Vector target")).toContainText("rag_doc/RagDocUnit");
  await page.getByRole("button", { name: "Save" }).click();

  await expect(page).toHaveURL(/\/rag$/);
  const storedRecords = await page.evaluate((key) => JSON.parse(window.localStorage.getItem(key) ?? "[]"), mockStorageKey);
  expect(storedRecords).toEqual(expect.arrayContaining([
    expect.objectContaining({
      code: "project-doc",
      config: expect.objectContaining({
        storageBinding: expect.objectContaining({
          bindings: expect.objectContaining({
            diagnostics: "project-doc-diagnostics",
            fulltext: "content-store",
            source: "source-store",
            textUnit: "content-store",
            vector: "content-store",
          }),
          targets: expect.arrayContaining([
            expect.objectContaining({
              id: "source-store",
              table: "rag_project_doc_sources",
            }),
            expect.objectContaining({
              backend: "GRAPH",
              id: "content-store",
              namespace: "rag_doc",
              nodeLabel: "RagDocUnit",
            }),
            expect.objectContaining({
              backend: "SQLITE",
              id: "project-doc-diagnostics",
              table: "rag_project_doc_diagnostics",
            }),
          ]),
        }),
      }),
    }),
  ]));
});

test("seeded memory-card edit preserves target-first bindings", async ({ page }) => {
  await routeActiveProfileApi(page);
  await page.goto(`${frontendBaseUrl}/rag`);
  await page.getByRole("cell", { name: "Memory Card memory-card" }).click();
  await expect(page).toHaveURL(/\/rag\/corpora\/memory-card\/edit$/);

  await expect(page.getByLabel("Corpus code")).toHaveValue("memory-card");
  await expect(controlByAriaLabel(page, "Storage target 1 table")).toHaveCount(0);
  await openAdvancedStorage(page);
  await expect(controlByAriaLabel(page, "Storage target 1 id")).toHaveCount(0);
  await expect(controlByAriaLabel(page, "Storage target 1 table")).toHaveValue("rag_memory_text_units");
  await expect(controlByAriaLabel(page, "Source target")).toContainText("rag_memory_text_units");
  await expect(controlByAriaLabel(page, "Text unit target")).toContainText("rag_memory_text_units");
  await expect(controlByAriaLabel(page, "Fulltext index target")).toContainText("rag_memory_text_units");
  await expect(controlByAriaLabel(page, "Vector target")).toContainText("rag_memory_text_units");
  await expect(page.getByRole("heading", { name: "Linked Profiles" })).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "Filters And Route Summary" })).toHaveCount(0);
  await openFieldConfig(page, "Filter fields");
  await expect(controlByAriaLabel(page, "Filter field 1 name")).toContainText("scopeType");
});
