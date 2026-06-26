import {
  expect,
  test,
  type Page,
} from "@playwright/test";
import {
  spawn,
  type ChildProcessWithoutNullStreams,
} from "node:child_process";
import { createServer } from "node:net";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const thisFilePath = fileURLToPath(import.meta.url);
const frontendRoot = resolve(thisFilePath, "..", "..");
const mockStorageKey = "tron.rag.mock.corpora";

let frontendProcess: ChildProcessWithoutNullStreams | undefined;
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

async function openCreatePage(page: Page) {
  await page.goto(`${frontendBaseUrl}/rag`);
  await expect(page.getByRole("heading", { name: "RAG Management" })).toBeVisible();
  await page.getByRole("button", { name: "New Corpus" }).click();
  await expect(page).toHaveURL(/\/rag\/corpora\/new$/);
  await expect(page.getByRole("heading", { name: "New corpus" })).toBeVisible();
}

async function openAdvancedStorage(page: Page) {
  if (await page.getByLabel("Storage target 1 table").count() === 0) {
    await page.getByRole("button", { name: "Advanced storage settings" }).click();
  }
  await expect(page.getByLabel("Storage target 1 table")).toBeVisible();
}

async function enableSummaryTree(page: Page) {
  const summaryTreeToggle = page.getByRole("button", { name: "Enable summary tree" });
  if ((await summaryTreeToggle.getAttribute("aria-pressed")) !== "true") {
    await summaryTreeToggle.click();
  }
  await expect(summaryTreeToggle).toHaveAttribute("aria-pressed", "true");
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
  frontendProcess = spawn("npm", ["run", "dev", "--", "--host", "127.0.0.1", "--port", String(port)], {
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
  if (!frontendProcess || frontendProcess.killed) return;
  frontendProcess.kill("SIGTERM");
  await new Promise((resolveProcess) => frontendProcess?.once("exit", resolveProcess));
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
  await expect(page.getByLabel("Storage target 1 id")).toHaveCount(0);
  await expect(page.getByLabel("Filter field 1 name")).toHaveCount(0);
  const sectionHeadings = await page.locator("h2").allTextContents();
  expect(sectionHeadings.indexOf("Storage binding")).toBeLessThan(sectionHeadings.indexOf("Field configuration"));

  await page.getByLabel("Corpus code").fill("support-notes");
  await page.getByLabel("Corpus name").fill("Support Notes");
  await page.getByLabel("Description").fill("Support playbooks and incident review notes.");
  await expect(page.getByText("3 targets")).toBeVisible();
  await openAdvancedStorage(page);
  await expect(page.getByLabel("Storage target 1 table")).toHaveValue("rag_support_notes_sources");
  await expect(page.getByLabel("Storage target 2 table")).toHaveValue("rag_support_notes_text_units");
  await expect(page.getByLabel("Storage target 3 table")).toHaveValue("rag_support_notes_vectors");
  await expect(page.getByLabel("Storage target 4 table")).toHaveCount(0);
  await expect(page.getByLabel("Source target")).toHaveValue("source-store");
  await expect(page.getByLabel("Text unit target")).toHaveValue("text-unit-store");
  await expect(page.getByLabel("Fulltext index target")).toHaveValue("text-unit-store");
  await expect(page.getByLabel("Vector target")).toHaveValue("vector-store");
  await expect(page.getByLabel("Summary tree target")).toHaveCount(0);
  await enableSummaryTree(page);
  await expect(page.getByLabel("Storage target 4 table")).toHaveValue("rag_support_notes_tree_nodes");
  await expect(page.getByLabel("Summary tree target")).toHaveValue("tree-store");
  await page.getByLabel("Fulltext index target").selectOption("vector-store");
  await page.getByLabel("Storage target 3 backend").selectOption("GRAPH");
  await page.getByLabel("Storage target 3 graph namespace").fill("rag_doc");
  await page.getByLabel("Storage target 3 node label").fill("RagDocUnit");
  await expect(page.getByLabel("Enable graph capability")).toHaveCount(0);

  await openFieldConfig(page, "Source fields");
  await page.getByRole("button", { name: "Add source field" }).click();
  await page.getByLabel("Source mapping 1 logical name").fill("documentCode");
  await page.getByLabel("Source mapping 1 column").fill("document_code");
  await openFieldConfig(page, "Filter fields");
  await page.getByLabel("Filter field 1 name").selectOption("documentCode");
  await page.getByLabel("Filter field 1 type").selectOption("string");
  await page.getByLabel("Filter field 1 operators").fill("eq, in");

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
  await page.getByLabel("Storage target 1 table").fill("bad-table");
  await page.getByLabel("Storage target 2 table").fill("");
  await page.getByLabel("Vector target").selectOption("");
  await openFieldConfig(page, "Source fields");
  await page.getByRole("button", { name: "Add source field" }).click();
  await page.getByLabel("Source mapping 1 logical name").fill("missingField");
  await page.getByLabel("Source mapping 1 column").fill("missing_column");
  await openFieldConfig(page, "Text unit fields");
  await page.getByRole("button", { name: "Add text unit field" }).click();
  await page.getByLabel("Text unit mapping 1 logical name").fill("missingText");
  await page.getByLabel("Text unit mapping 1 column").fill("missing_column");
  await openFieldConfig(page, "Filter fields");
  await page.getByLabel("Filter field 1 name").selectOption("missingField");
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
  await page.getByLabel("Storage target 4 backend").selectOption("GRAPH");
  await page.getByLabel("Storage target 4 graph namespace").fill("rag_doc");
  await page.getByLabel("Storage target 4 node label").fill("RagDocUnit");
  await page.getByLabel("Vector target").selectOption("target-4");
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
  await expect(page.getByLabel("Storage target 1 table")).toBeVisible();
  await expect(page.getByLabel("Storage target 2 table")).toBeVisible();
  await expect(page.getByLabel("Storage target 3 table")).toBeVisible();
  await expect(page.getByLabel("Storage target 4 table")).toHaveCount(0);
  await expect(page.getByLabel("Storage target 1 id")).toHaveCount(0);
  await enableSummaryTree(page);
  await expect(page.getByLabel("Storage target 4 table")).toBeVisible();
  await page.getByRole("button", { name: "Add custom storage" }).click();

  await expect(page.getByLabel("Storage target 5 table")).toBeVisible();
  await expect(addTargetButton).toBeEnabled();
  await expect(page.getByLabel("Storage target 5 id")).toHaveCount(0);
});

test("keeps at least one storage target and shows physical target names in segment bindings", async ({ page }) => {
  await openCreatePage(page);

  await page.getByLabel("Corpus code").fill("test");
  await openAdvancedStorage(page);
  await expect(page.locator('select[aria-label="Source target"] option:checked')).toHaveText("rag_test_sources");
  await expect(page.locator('select[aria-label="Text unit target"] option:checked')).toHaveText("rag_test_text_units");
  await expect(page.locator('select[aria-label="Vector target"] option:checked')).toHaveText("rag_test_vectors");
  await expect(page.locator('select[aria-label="Summary tree target"]')).toHaveCount(0);
  const sourceTargetOptionTexts = await page.locator('select[aria-label="Source target"] option').allTextContents();
  expect(sourceTargetOptionTexts.join(" ")).not.toContain("Storage Target");

  await page.getByRole("button", { name: "Remove storage target 3" }).click();
  await page.getByRole("button", { name: "Remove storage target 2" }).click();

  await expect(page.getByLabel("Storage target 1 table")).toBeVisible();
  await expect(page.getByRole("button", { name: /Remove storage target/ })).toHaveCount(0);
});

test("hides contract mappings by default while keeping custom rows removable", async ({ page }) => {
  await openCreatePage(page);

  await expect(page.getByLabel("Source mapping 1 logical name")).toHaveCount(0);
  await expect(page.getByLabel("Text unit mapping 1 logical name")).toHaveCount(0);
  await openFieldConfig(page, "Source fields");
  await page.getByRole("button", { name: "Show source contract fields" }).click();
  await expect(page.getByLabel("Source contract field 1 logical name")).toBeDisabled();
  await expect(page.locator('input[aria-label$="logical name"][value="sourceType"]')).toBeVisible();
  await expect(page.locator('input[aria-label$="logical name"][value="status"]')).toBeVisible();
  await expect(page.locator('input[aria-label$="logical name"][value="title"]')).toBeVisible();
  await expect(page.locator('input[aria-label$="logical name"][value="metadata"]')).toBeVisible();
  await page.getByRole("button", { name: "Hide source contract fields" }).click();

  await page.getByRole("button", { name: "Add source field" }).click();
  await page.getByLabel("Source mapping 1 logical name").fill("documentCode");
  await page.getByLabel("Source mapping 1 column").fill("document_code");
  await page.getByRole("button", { name: "Remove Source mapping 1" }).click();

  await openFieldConfig(page, "Text unit fields");
  await page.getByRole("button", { name: "Show text unit contract fields" }).click();
  await expect(page.getByLabel("Text unit contract field 1 logical name")).toBeDisabled();
  await expect(page.locator('input[aria-label$="logical name"][value="textUnitStatus"]')).toBeVisible();
  await page.getByRole("button", { name: "Hide text unit contract fields" }).click();
  await page.getByRole("button", { name: "Add text unit field" }).click();
  await page.getByLabel("Text unit mapping 1 logical name").fill("documentCode");
  await page.getByLabel("Text unit mapping 1 column").fill("document_code");
  await page.getByRole("button", { name: "Remove Text unit mapping 1" }).click();

  await openFieldConfig(page, "Filter fields");
  await page.getByRole("button", { name: "Remove filter field 1" }).click();

  await expect(page.getByLabel("Source mapping 1 logical name")).toHaveCount(0);
  await expect(page.getByLabel("Text unit mapping 1 logical name")).toHaveCount(0);
  await expect(page.getByLabel("Filter field 1 name")).toHaveCount(0);
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
  await page.goto(`${frontendBaseUrl}/rag`);
  await page.getByRole("cell", { name: "Project Document project-doc" }).click();
  await expect(page).toHaveURL(/\/rag\/corpora\/project-doc\/edit$/);

  await expect(page.getByLabel("Corpus code")).toHaveValue("project-doc");
  await expect(page.getByLabel("Storage target 1 table")).toHaveCount(0);
  await openAdvancedStorage(page);
  await expect(page.getByLabel("Storage target 1 id")).toHaveCount(0);
  await expect(page.getByLabel("Storage target 1 table")).toHaveValue("rag_project_doc_sources");
  await expect(page.getByLabel("Storage target 2 table")).toHaveValue("rag_project_doc_text_units");
  await expect(page.getByLabel("Source target")).toHaveValue("source-store");
  await expect(page.getByLabel("Text unit target")).toHaveValue("content-store");
  await expect(page.getByLabel("Fulltext index target")).toHaveValue("content-store");
  await expect(page.getByLabel("Vector target")).toHaveValue("content-store");

  await page.getByLabel("Storage target 2 backend").selectOption("GRAPH");
  await page.getByLabel("Storage target 2 graph namespace").fill("rag_doc");
  await page.getByLabel("Storage target 2 node label").fill("RagDocUnit");
  await expect(page.getByLabel("Text unit target")).toHaveValue("content-store");
  await expect(page.getByLabel("Vector target")).toHaveValue("content-store");
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
  await page.goto(`${frontendBaseUrl}/rag`);
  await page.getByRole("cell", { name: "Memory Card memory-card" }).click();
  await expect(page).toHaveURL(/\/rag\/corpora\/memory-card\/edit$/);

  await expect(page.getByLabel("Corpus code")).toHaveValue("memory-card");
  await expect(page.getByLabel("Storage target 1 table")).toHaveCount(0);
  await openAdvancedStorage(page);
  await expect(page.getByLabel("Storage target 1 id")).toHaveCount(0);
  await expect(page.getByLabel("Storage target 1 table")).toHaveValue("rag_memory_text_units");
  await expect(page.getByLabel("Source target")).toHaveValue("content-store");
  await expect(page.getByLabel("Text unit target")).toHaveValue("content-store");
  await expect(page.getByLabel("Fulltext index target")).toHaveValue("content-store");
  await expect(page.getByLabel("Vector target")).toHaveValue("content-store");
  await expect(page.getByRole("heading", { name: "Linked Profiles" })).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "Filters And Route Summary" })).toHaveCount(0);
  await openFieldConfig(page, "Filter fields");
  await expect(page.getByLabel("Filter field 1 name")).toHaveValue("scopeType");
});
