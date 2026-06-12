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
  await expect(page.getByRole("heading", { name: "Create Corpus" })).toBeVisible();
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
  await page.addInitScript((key) => window.localStorage.removeItem(key), mockStorageKey);
});

test("creates target-first mixed-media corpus config and derives the registry summary", async ({ page }) => {
  const requests: CapturedRequest[] = [];
  await captureApiRequests(page, requests);

  await openCreatePage(page);

  await expect(page.getByRole("heading", { name: "Storage Binding" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Capabilities" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Filter Schema" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Field Mapping" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Backend Payload Preview" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Storage Targets" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Segment Bindings" })).toBeVisible();
  await expect(page.getByLabel("Storage target 1 id")).toHaveCount(0);
  await expect(page.getByLabel("Filter field 1 column")).toHaveCount(0);
  const sectionHeadings = await page.locator("h2").allTextContents();
  expect(sectionHeadings.indexOf("Storage Binding")).toBeLessThan(sectionHeadings.indexOf("Capabilities"));
  expect(sectionHeadings.indexOf("Field Mapping")).toBeLessThan(sectionHeadings.indexOf("Filter Schema"));

  await page.getByLabel("Corpus code").fill("support-notes");
  await page.getByLabel("Corpus name").fill("Support Notes");
  await page.getByLabel("Corpus description").fill("Support playbooks and incident review notes.");
  await expect(page.getByLabel("Storage target 1 table")).toHaveValue("rag_support_notes_sources");
  await expect(page.getByLabel("Storage target 2 table")).toHaveValue("rag_support_notes_text_units");
  await expect(page.getByLabel("Storage target 3 table")).toHaveValue("rag_support_notes_vectors");
  await expect(page.getByLabel("Source target")).toHaveValue("source-store");
  await expect(page.getByLabel("Text Unit target")).toHaveValue("text-unit-store");
  await expect(page.getByLabel("Fulltext target")).toHaveValue("text-unit-store");
  await expect(page.getByLabel("Vector target")).toHaveValue("vector-store");
  await page.getByLabel("Fulltext target").selectOption("vector-store");
  await expect(page.getByLabel("Graph enabled")).toBeDisabled();
  await page.getByLabel("Storage target 3 backend").selectOption("GRAPH");
  await page.getByLabel("Storage target 3 graph namespace").fill("rag_doc");
  await page.getByLabel("Storage target 3 node label").fill("RagDocUnit");
  await expect(page.getByLabel("Graph enabled")).toBeEnabled();
  await page.getByLabel("Fulltext enabled").check();
  await page.getByLabel("Vector enabled").check();
  await page.getByLabel("Graph enabled").check();
  await page.getByRole("button", { name: "Add Source Mapping" }).click();
  await page.getByLabel("Source mapping 1 logical").fill("documentCode");
  await page.getByLabel("Source mapping 1 column").fill("document_code");
  await page.getByLabel("Filter field 1 name").selectOption("documentCode");
  await page.getByLabel("Filter field 1 type").selectOption("string");
  await page.getByLabel("Filter field 1 operators").fill("eq, in");

  await expect(page.getByText("storageBindingJson")).toBeVisible();
  await expect(page.getByText('"mode": "TARGETED"')).toBeVisible();
  await expect(page.getByText('"fulltext": "vector-store"')).toBeVisible();
  await expect(page.getByText('"id": "vector-store"')).toBeVisible();
  await expect(page.getByText('"documentCode"')).toBeVisible();
  await expect(page.getByText('"column": "document_code"')).toBeVisible();
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
  await page.getByLabel("Storage target 1 table").fill("bad-table");
  await page.getByLabel("Storage target 2 table").fill("");
  await page.getByLabel("Vector target").selectOption("");
  await page.getByRole("button", { name: "Add Source Mapping" }).click();
  await page.getByLabel("Source mapping 1 logical").fill("missingField");
  await page.getByLabel("Source mapping 1 column").fill("missing_column");
  await page.getByRole("button", { name: "Add Text Unit Mapping" }).click();
  await page.getByLabel("Text unit mapping 1 logical").fill("missingText");
  await page.getByLabel("Text unit mapping 1 column").fill("missing_column");
  await page.getByLabel("Filter field 1 name").selectOption("missingField");
  await page.getByRole("button", { name: "Save" }).click();

  await expect(page.getByText("Storage target 1 table must start with rag_ and use lowercase letters, digits, or underscores.")).toBeVisible();
  await expect(page.getByText("Storage target 2 table is required for SQLite storage.")).toBeVisible();
  await expect(page.getByText("Vector target is required.")).toBeVisible();
  await expect(page.getByText("Filter field must resolve to a column in one of the bound storage targets.")).toBeVisible();
  await expect(page.getByText("Source mapping column must exist in the bound source target.")).toBeVisible();
  await expect(page.getByText("Text-unit mapping column must exist in the bound text-unit target.")).toBeVisible();
  await expect(page).toHaveURL(/\/rag\/corpora\/new$/);
});

test("requires a non-diagnostics graph binding when graph enabled is checked", async ({ page }) => {
  const graphClosureError = "Graph enabled requires at least one non-diagnostics segment to bind to a GRAPH target.";

  await openCreatePage(page);

  await expect(page.getByLabel("Graph enabled")).toBeDisabled();

  await page.getByRole("button", { name: "Add Storage Target" }).click();
  await page.getByLabel("Storage target 4 backend").selectOption("GRAPH");
  await page.getByLabel("Storage target 4 graph namespace").fill("rag_doc");
  await page.getByLabel("Storage target 4 node label").fill("RagDocUnit");
  await expect(page.getByLabel("Graph enabled")).toBeEnabled();

  await page.getByLabel("Graph enabled").check();
  await expect(page.getByText(graphClosureError)).toBeVisible();

  await page.getByRole("button", { name: "Save" }).click();
  await expect(page.getByText(graphClosureError)).toBeVisible();
  await expect(page).toHaveURL(/\/rag\/corpora\/new$/);

  await page.getByLabel("Vector target").selectOption("target-4");
  await expect(page.getByText(graphClosureError)).toHaveCount(0);

  await page.getByLabel("Storage target 4 backend").selectOption("SQLITE");
  await expect(page.getByLabel("Graph enabled")).toBeDisabled();
  await expect(page.getByLabel("Graph enabled")).not.toBeChecked();
});

test("starts with three default storage targets and can add more", async ({ page }) => {
  await openCreatePage(page);

  const addTargetButton = page.getByRole("button", { name: "Add Storage Target" });
  await expect(addTargetButton).toBeEnabled();
  await expect(page.getByLabel("Storage target 1 table")).toBeVisible();
  await expect(page.getByLabel("Storage target 2 table")).toBeVisible();
  await expect(page.getByLabel("Storage target 3 table")).toBeVisible();
  await expect(page.getByLabel("Storage target 1 id")).toHaveCount(0);
  await page.getByRole("button", { name: "Add Storage Target" }).click();

  await expect(page.getByLabel("Storage target 4 table")).toBeVisible();
  await expect(addTargetButton).toBeEnabled();
  await expect(page.getByLabel("Storage target 4 id")).toHaveCount(0);
});

test("keeps at least one storage target and shows physical target names in segment bindings", async ({ page }) => {
  await openCreatePage(page);

  await page.getByLabel("Corpus code").fill("test");
  await expect(page.locator('select[aria-label="Source target"] option:checked')).toHaveText("rag_test_sources");
  await expect(page.locator('select[aria-label="Text Unit target"] option:checked')).toHaveText("rag_test_text_units");
  await expect(page.locator('select[aria-label="Vector target"] option:checked')).toHaveText("rag_test_vectors");
  const sourceTargetOptionTexts = await page.locator('select[aria-label="Source target"] option').allTextContents();
  expect(sourceTargetOptionTexts.join(" ")).not.toContain("Storage Target");

  await page.getByRole("button", { name: "Remove storage target 3" }).click();
  await page.getByRole("button", { name: "Remove storage target 2" }).click();

  await expect(page.getByLabel("Storage target 1 table")).toBeVisible();
  await expect(page.getByRole("button", { name: /Remove storage target/ })).toHaveCount(0);
});

test("hides contract mappings by default while keeping custom rows removable", async ({ page }) => {
  await openCreatePage(page);

  await expect(page.getByLabel("Source mapping 1 logical")).toHaveCount(0);
  await expect(page.getByLabel("Text unit mapping 1 logical")).toHaveCount(0);
  await page.getByRole("button", { name: "Show Source Contract Fields" }).click();
  await page.getByRole("button", { name: "Show Text Unit Contract Fields" }).click();
  await expect(page.getByLabel("Source contract field 1 logical")).toBeDisabled();
  await expect(page.getByLabel("Text unit contract field 1 logical")).toBeDisabled();
  await expect(page.locator('input[aria-label$="logical"][value="sourceType"]')).toBeVisible();
  await expect(page.locator('input[aria-label$="logical"][value="status"]')).toBeVisible();
  await expect(page.locator('input[aria-label$="logical"][value="title"]')).toBeVisible();
  await expect(page.locator('input[aria-label$="logical"][value="metadata"]')).toBeVisible();
  await expect(page.locator('input[aria-label$="logical"][value="textUnitStatus"]')).toBeVisible();
  await page.getByRole("button", { name: "Hide Source Contract Fields" }).click();
  await page.getByRole("button", { name: "Hide Text Unit Contract Fields" }).click();

  await page.getByRole("button", { name: "Add Source Mapping" }).click();
  await page.getByLabel("Source mapping 1 logical").fill("documentCode");
  await page.getByLabel("Source mapping 1 column").fill("document_code");
  await page.getByRole("button", { name: "Remove source mapping 1" }).click();

  await page.getByRole("button", { name: "Add Text Unit Mapping" }).click();
  await page.getByLabel("Text unit mapping 1 logical").fill("documentCode");
  await page.getByLabel("Text unit mapping 1 column").fill("document_code");
  await page.getByRole("button", { name: "Remove text unit mapping 1" }).click();

  await page.getByRole("button", { name: "Remove filter field 1" }).click();

  await expect(page.getByLabel("Source mapping 1 logical")).toHaveCount(0);
  await expect(page.getByLabel("Text unit mapping 1 logical")).toHaveCount(0);
  await expect(page.getByLabel("Filter field 1 name")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Add Source Mapping" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Add Text Unit Mapping" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Add Filter Field" })).toBeVisible();

  await page.getByRole("button", { name: "Save" }).click();

  await expect(page.getByText("At least one filter schema field is required.")).toBeVisible();
  await expect(page.getByText("At least one source field mapping is required.")).toHaveCount(0);
  await expect(page.getByText("At least one text-unit field mapping is required.")).toHaveCount(0);
});

test("edits seeded project document storage structure with target reuse in preview", async ({ page }) => {
  await page.goto(`${frontendBaseUrl}/rag`);
  await page.getByRole("button", { name: "Edit Project Document" }).click();
  await expect(page).toHaveURL(/\/rag\/corpora\/project-doc\/edit$/);

  await expect(page.getByLabel("Corpus code")).toHaveValue("project-doc");
  await expect(page.getByLabel("Storage target 1 id")).toHaveCount(0);
  await expect(page.getByLabel("Storage target 1 table")).toHaveValue("rag_project_doc_sources");
  await expect(page.getByLabel("Storage target 2 table")).toHaveValue("rag_project_doc_text_units");
  await expect(page.getByLabel("Source target")).toHaveValue("source-store");
  await expect(page.getByLabel("Text Unit target")).toHaveValue("content-store");
  await expect(page.getByLabel("Fulltext target")).toHaveValue("content-store");
  await expect(page.getByLabel("Vector target")).toHaveValue("content-store");
  await expect(page.getByText('"mode": "TARGETED"')).toBeVisible();

  await page.getByLabel("Storage target 2 backend").selectOption("GRAPH");
  await page.getByLabel("Storage target 2 graph namespace").fill("rag_doc");
  await page.getByLabel("Storage target 2 node label").fill("RagDocUnit");
  await expect(page.getByLabel("Text Unit target")).toHaveValue("content-store");
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
  await page.getByRole("button", { name: "Edit Memory Card" }).click();
  await expect(page).toHaveURL(/\/rag\/corpora\/memory-card\/edit$/);

  await expect(page.getByLabel("Corpus code")).toHaveValue("memory-card");
  await expect(page.getByLabel("Storage target 1 id")).toHaveCount(0);
  await expect(page.getByLabel("Storage target 1 table")).toHaveValue("rag_memory_text_units");
  await expect(page.getByLabel("Source target")).toHaveValue("content-store");
  await expect(page.getByLabel("Text Unit target")).toHaveValue("content-store");
  await expect(page.getByLabel("Fulltext target")).toHaveValue("content-store");
  await expect(page.getByLabel("Vector target")).toHaveValue("content-store");
  await expect(page.getByRole("heading", { name: "Linked Profiles" })).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "Filters And Route Summary" })).toHaveCount(0);
  await expect(page.getByText('"mode": "TARGETED"')).toBeVisible();
  await expect(page.getByText('"memory-card-diagnostics"')).toBeVisible();
  await expect(page.getByText('"source_status"')).toBeVisible();
});
