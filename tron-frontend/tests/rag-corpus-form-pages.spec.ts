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

import {
  installRagManagementApiMock,
  type CapturedRequest,
} from "./rag-management-api-mock";

const thisFilePath = fileURLToPath(import.meta.url);
const frontendRoot = resolve(thisFilePath, "..", "..");

let frontendProcess: ChildProcessWithoutNullStreams | undefined;
let frontendLogs = "";
let frontendBaseUrl = "";

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

function notifications(page: Page, message: string | RegExp) {
  return page.locator('[aria-label="Notifications"]').getByRole("status").filter({ hasText: message }).first();
}

function requestBodies(requests: CapturedRequest[], pathname: string, method: string) {
  return requests
    .filter((request) => request.pathname === pathname && request.method === method)
    .map((request) => request.body as Record<string, unknown>);
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
      VITE_TRON_AUTH_DATA_SOURCE: "mock",
      VITE_TRON_DATA_SOURCE: "mock",
      VITE_TRON_RAG_DATA_SOURCE: "api",
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

test("loads corpora from management APIs and persists create/edit through verify plus save requests", async ({ page }) => {
  const requests: CapturedRequest[] = [];
  await installRagManagementApiMock(page, requests);

  await page.goto(`${frontendBaseUrl}/rag`);
  await expect(page.getByRole("heading", { name: "RAG Management" })).toBeVisible();
  await expect(page.getByRole("cell", { name: "Project Document project-doc" })).toBeVisible();

  await page.getByRole("button", { name: "New Corpus" }).click();
  await expect(page).toHaveURL(/\/rag\/corpora\/new$/);
  await expect(page.getByRole("heading", { name: "Create Corpus" })).toBeVisible();

  await page.getByLabel("Corpus code").fill("support-notes");
  await page.getByLabel("Corpus name").fill("Support Notes");
  await page.getByLabel("Corpus description").fill("Support playbooks and incident review notes.");
  await page.getByRole("button", { name: "Save" }).click();

  await expect(page).toHaveURL(/\/rag$/);
  await expect(notifications(page, "Support Notes created")).toBeVisible();
  await expect(page.getByRole("cell", { name: "Support Notes support-notes" })).toBeVisible();

  await page.getByRole("button", { name: "Edit Project Document" }).click();
  await expect(page).toHaveURL(/\/rag\/corpora\/project-doc\/edit$/);
  await expect(page.getByRole("heading", { name: "Edit Corpus" })).toBeVisible();
  await expect(page.getByLabel("Corpus code")).toHaveValue("project-doc");
  await expect(page.getByLabel("Corpus code")).toBeDisabled();
  await expect(page.getByLabel("Text Unit target")).toHaveValue("content-store");
  await expect(page.getByLabel("Fulltext target")).toHaveValue("content-store");

  await page.getByLabel("Corpus name").fill("Project Documents QA");
  await page.getByLabel("Storage target 2 table").fill("rag_project_doc_text_units_qa");
  await page.getByRole("button", { name: "Save" }).click();

  await expect(page).toHaveURL(/\/rag$/);
  await expect(notifications(page, "Project Documents QA saved")).toBeVisible();
  await expect(page.getByRole("cell", { name: "Project Documents QA project-doc" })).toBeVisible();

  expect(requests.some((request) => request.pathname === "/api/v1/rag/corpora" && request.method === "GET")).toBeTruthy();
  expect(requests.some((request) => request.pathname === "/api/v1/rag/corpora/catalog" && request.method === "GET")).toBeTruthy();
  expect(requests.some((request) => request.pathname === "/api/v1/rag/corpora/project-doc" && request.method === "GET")).toBeTruthy();

  const verifyBodies = requestBodies(requests, "/api/v1/rag/corpora/verify", "POST");
  expect(verifyBodies).toEqual(expect.arrayContaining([
    expect.objectContaining({
      corpusId: "support-notes",
      displayName: "Support Notes",
      metadataGroups: [],
      storageBinding: expect.objectContaining({ mode: "TARGETED" }),
    }),
    expect.objectContaining({
      corpusId: "project-doc",
      displayName: "Project Documents QA",
      metadataGroups: [{ groupId: "provenance", labels: ["scopeType", "scopeCode"] }],
      storageBinding: expect.objectContaining({
        bindings: expect.objectContaining({
          fulltext: "content-store",
          textUnit: "content-store",
        }),
      }),
    }),
  ]));

  const createBodies = requestBodies(requests, "/api/v1/rag/corpora", "POST");
  expect(createBodies).toEqual(expect.arrayContaining([
    expect.objectContaining({
      corpusId: "support-notes",
      displayName: "Support Notes",
      metadataGroups: [],
    }),
  ]));

  const updateBodies = requestBodies(requests, "/api/v1/rag/corpora/project-doc", "PUT");
  expect(updateBodies).toEqual(expect.arrayContaining([
    expect.objectContaining({
      corpusId: "project-doc",
      displayName: "Project Documents QA",
      metadataGroups: [{ groupId: "provenance", labels: ["scopeType", "scopeCode"] }],
      storageBinding: expect.objectContaining({
        bindings: expect.objectContaining({
          fulltext: "content-store",
          textUnit: "content-store",
        }),
      }),
    }),
  ]));
});

test("allows mixed graph and sqlite segment bindings", async ({ page }) => {
  const requests: CapturedRequest[] = [];
  await installRagManagementApiMock(page, requests);

  await page.goto(`${frontendBaseUrl}/rag/corpora/new`);
  await expect(page.getByRole("heading", { name: "Create Corpus" })).toBeVisible();

  await page.getByLabel("Corpus code").fill("support-mixed");
  await page.getByLabel("Corpus name").fill("Support Mixed");
  await page.getByRole("button", { name: "Add Storage Target" }).click();
  await page.getByLabel("Storage target 4 backend").selectOption("GRAPH");
  await page.getByLabel("Storage target 4 graph namespace").fill("rag_doc");
  await page.getByLabel("Storage target 4 node label").fill("RagDocUnit");
  await page.getByLabel("Graph enabled").check();
  await page.getByLabel("Vector target").selectOption("target-4");
  await page.getByRole("button", { name: "Save" }).click();

  await expect(page).toHaveURL(/\/rag$/);
  await expect(notifications(page, "Support Mixed created")).toBeVisible();

  const verifyBodies = requestBodies(requests, "/api/v1/rag/corpora/verify", "POST")
    .filter((body) => body.corpusId === "support-mixed");
  expect(verifyBodies).toHaveLength(1);

  const createBodies = requestBodies(requests, "/api/v1/rag/corpora", "POST")
    .filter((body) => body.corpusId === "support-mixed");
  expect(createBodies).toEqual(expect.arrayContaining([
    expect.objectContaining({
      capabilities: expect.objectContaining({ graph: true }),
      corpusId: "support-mixed",
      storageBinding: expect.objectContaining({
        bindings: expect.objectContaining({
          source: "source-store",
          textUnit: "text-unit-store",
          vector: "target-4",
        }),
      }),
    }),
  ]));
});
