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

function expectCapabilityPayloadWithoutReserved(body: Record<string, unknown>) {
  expect(body.capabilities).toEqual(expect.any(Object));
  expect(body.capabilities as Record<string, unknown>).not.toHaveProperty("reserved");
}

function storageBindings(body: Record<string, unknown>) {
  const storageBinding = body.storageBinding as { bindings?: Record<string, unknown> } | undefined;
  return storageBinding?.bindings ?? {};
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

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => window.localStorage.setItem("tron.locale", "en-US"));
});

test("summary tree corpus form submits tree segment and graph tree payload", async ({ page }) => {
  const requests: CapturedRequest[] = [];
  await installRagManagementApiMock(page, requests);

  await page.goto(`${frontendBaseUrl}/rag/corpora/new`);
  await expect(page.getByRole("heading", { name: /New corpus|Create Corpus|新建语料库/ })).toBeVisible();

  await page.getByLabel(/Corpus code|语料库编码/).fill("support-tree");
  await page.getByLabel(/Corpus name|语料库名称/).fill("Support Tree");
  await expect(page.getByText(/Default storage is ready|默认存储已就绪/)).toBeVisible();
  const summaryTreeToggle = page.getByRole("button", { name: /Enable summary tree|启用摘要树/ });
  await expect(summaryTreeToggle).toHaveAttribute("aria-pressed", "false");
  await expect(page.getByLabel(/Storage target 4 table|存储目标 4 表名/)).toBeHidden();

  await page.getByRole("button", { name: /Advanced storage settings|高级存储设置/ }).click();
  await expect(page.getByLabel(/Summary tree target|Tree target|摘要树目标/)).toHaveCount(0);
  await summaryTreeToggle.click();
  await expect(summaryTreeToggle).toHaveAttribute("aria-pressed", "true");
  await expect(page.getByLabel(/Storage target 4 table|存储目标 4 表名/)).toHaveValue("rag_support_tree_tree_nodes");
  await expect(page.getByLabel(/Summary tree target|Tree target|摘要树目标/)).toHaveValue("tree-store");

  await page.getByRole("button", { name: /Add custom storage|添加自定义存储/ }).click();
  await page.getByLabel(/Storage target 5 backend|存储目标 5 后端/).selectOption("GRAPH");
  await page.getByLabel(/Storage target 5 graph namespace|存储目标 5 图命名空间/).fill("rag_support_tree_tree");
  await page.getByLabel(/Storage target 5 node label|存储目标 5 节点标签/).fill("RagTreeNode");
  await expect(page.getByLabel(/Enable graph capability|Graph enabled|启用图能力/)).toBeHidden();
  await page.getByLabel(/Summary tree target|Tree target|摘要树目标/).selectOption("target-5");
  await page.getByRole("button", { name: /^Save$|^保存$/ }).click();

  await expect(page).toHaveURL(/\/rag$/);

  const createBodies = requestBodies(requests, "/api/v1/rag/corpora", "POST")
    .filter((body) => body.corpusId === "support-tree");
  createBodies.forEach(expectCapabilityPayloadWithoutReserved);
  expect(createBodies).toEqual(expect.arrayContaining([
    expect.objectContaining({
      capabilities: expect.objectContaining({ graph: true, tree: true }),
      corpusId: "support-tree",
      storageBinding: expect.objectContaining({
        bindings: expect.objectContaining({
          source: "source-store",
          tree: "target-5",
          textUnit: "text-unit-store",
          vector: "vector-store",
        }),
        targets: expect.arrayContaining([
          expect.objectContaining({
            backend: "GRAPH",
            id: "target-5",
            layout: "GRAPH_TREE",
            namespace: "rag_support_tree_tree",
            nodeLabel: "RagTreeNode",
          }),
        ]),
      }),
    }),
  ]));
});

test("loads corpora from management APIs and persists create/edit through verify plus save requests", async ({ page }) => {
  const requests: CapturedRequest[] = [];
  await installRagManagementApiMock(page, requests);

  await page.goto(`${frontendBaseUrl}/rag`);
  await expect(page.getByRole("heading", { name: /RAG Management|RAG 管理|知识检索管理/ })).toBeVisible();
  await expect(page.getByRole("cell", { name: "Project Document project-doc" })).toBeVisible();

  await page.getByRole("button", { name: /New corpus|新建语料库/ }).click();
  await expect(page).toHaveURL(/\/rag\/corpora\/new$/);
  await expect(page.getByRole("heading", { name: /New corpus|Create Corpus|新建语料库/ })).toBeVisible();

  await page.getByLabel(/Corpus code|语料库编码/).fill("support-notes");
  await page.getByLabel(/Corpus name|语料库名称/).fill("Support Notes");
  await page.getByRole("textbox", { name: /Description|说明/ }).fill("Support playbooks and incident review notes.");
  await expect(page.getByLabel(/Source target|来源目标/)).toBeHidden();
  await expect(page.getByLabel(/Enable fulltext index|启用全文索引/)).toBeHidden();
  await expect(page.getByLabel(/Enable vector index|启用向量索引/)).toBeHidden();
  await expect(page.getByRole("button", { name: /Enable summary tree|启用摘要树/ })).toHaveAttribute("aria-pressed", "false");
  await expect(page.getByLabel(/Enable graph capability|Graph enabled|启用图能力/)).toBeHidden();
  await expect(page.getByLabel(/Reserved corpus|保留语料库/)).toHaveCount(0);
  await expect(page.getByRole("heading", { name: /Field configuration|字段配置/ })).toBeVisible();
  await expect(page.getByRole("button", { name: /Configure fields|配置字段/ })).toBeVisible();
  await expect(page.getByRole("tab", { name: /Source fields|来源字段/ })).toBeHidden();
  await page.getByRole("button", { name: /Configure fields|配置字段/ }).click();
  await expect(page.getByRole("tab", { name: /Source fields|来源字段/ })).toHaveAttribute("aria-selected", "true");
  await expect(page.getByRole("tab", { name: /Text unit fields|文本单元字段/ })).toBeVisible();
  await page.getByRole("tab", { name: /Filter fields|筛选字段/ }).click();
  await expect(page.getByLabel(/Filter field 1 name|筛选字段 1 名称/)).toBeVisible();
  await expect(page.getByRole("heading", { name: /Backend payload preview|后端 payload 预览/ })).toHaveCount(0);
  await page.getByRole("button", { name: /^Save$|^保存$/ }).click();

  await expect(page).toHaveURL(/\/rag$/);
  await expect(notifications(page, /Support Notes created|Support Notes 已创建/)).toBeVisible();
  await expect(page.getByRole("cell", { name: "Support Notes support-notes" })).toBeVisible();

  await page.getByRole("cell", { name: "Project Document project-doc" }).click();
  await expect(page).toHaveURL(/\/rag\/corpora\/project-doc\/edit$/);
  await expect(page.getByRole("heading", { name: /Edit corpus|Edit Corpus|编辑语料库/ })).toBeVisible();
  await expect(page.getByLabel(/Corpus code|语料库编码/)).toHaveValue("project-doc");
  await expect(page.getByLabel(/Corpus code|语料库编码/)).toBeDisabled();
  await expect(page.getByLabel(/Text unit target|Text Unit target|文本单元目标/)).toHaveCount(0);
  await page.getByRole("button", { name: /Advanced storage settings|高级存储设置/ }).click();
  await expect(page.getByLabel(/Text unit target|Text Unit target|文本单元目标/)).toHaveValue("content-store");
  await expect(page.getByLabel(/Fulltext index target|Fulltext target|全文索引目标/)).toHaveValue("content-store");

  await page.getByLabel(/Corpus name|语料库名称/).fill("Project Documents QA");
  await page.getByLabel(/Storage target 2 table|存储目标 2 表名/).fill("rag_project_doc_text_units_qa");
  await page.getByRole("button", { name: /^Save$|^保存$/ }).click();

  await expect(page).toHaveURL(/\/rag$/);
  await expect(notifications(page, /Project Documents QA saved|Project Documents QA 已保存/)).toBeVisible();
  await expect(page.getByRole("cell", { name: "Project Documents QA project-doc" })).toBeVisible();

  expect(requests.some((request) => request.pathname === "/api/v1/rag/corpora" && request.method === "GET")).toBeTruthy();
  expect(requests.some((request) => request.pathname === "/api/v1/rag/corpora/catalog" && request.method === "GET")).toBeTruthy();
  expect(requests.some((request) => request.pathname === "/api/v1/rag/corpora/project-doc" && request.method === "GET")).toBeTruthy();

  const verifyBodies = requestBodies(requests, "/api/v1/rag/corpora/verify", "POST");
  verifyBodies.forEach(expectCapabilityPayloadWithoutReserved);
  expect(verifyBodies).toEqual(expect.arrayContaining([
    expect.objectContaining({
      corpusId: "support-notes",
      displayName: "Support Notes",
      metadataGroups: [],
      capabilities: expect.objectContaining({
        fulltext: true,
        graph: false,
        tree: false,
        vector: true,
      }),
      storageBinding: expect.objectContaining({
        bindings: expect.objectContaining({
          fulltext: "text-unit-store",
          source: "source-store",
          textUnit: "text-unit-store",
          vector: "vector-store",
        }),
        mode: "TARGETED",
        targets: expect.arrayContaining([
          expect.objectContaining({ backend: "SQLITE", id: "source-store", table: "rag_support_notes_sources" }),
          expect.objectContaining({ backend: "SQLITE", id: "text-unit-store", table: "rag_support_notes_text_units" }),
          expect.objectContaining({ backend: "SQLITE", id: "vector-store", table: "rag_support_notes_vectors" }),
        ]),
      }),
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
  const supportVerifyBody = verifyBodies.find((body) => body.corpusId === "support-notes");
  expect(storageBindings(supportVerifyBody ?? {})).not.toHaveProperty("tree");

  const createBodies = requestBodies(requests, "/api/v1/rag/corpora", "POST");
  createBodies.forEach(expectCapabilityPayloadWithoutReserved);
  expect(createBodies).toEqual(expect.arrayContaining([
    expect.objectContaining({
      corpusId: "support-notes",
      displayName: "Support Notes",
      metadataGroups: [],
    }),
  ]));

  const updateBodies = requestBodies(requests, "/api/v1/rag/corpora/project-doc", "PUT");
  updateBodies.forEach(expectCapabilityPayloadWithoutReserved);
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
  await expect(page.getByRole("heading", { name: /New corpus|Create Corpus|新建语料库/ })).toBeVisible();

  await page.getByLabel(/Corpus code|语料库编码/).fill("support-mixed");
  await page.getByLabel(/Corpus name|语料库名称/).fill("Support Mixed");
  await page.getByRole("button", { name: /Enable summary tree|启用摘要树/ }).click();
  await page.getByRole("button", { name: /Advanced storage settings|高级存储设置/ }).click();
  await page.getByRole("button", { name: /Add custom storage|添加自定义存储/ }).click();
  await page.getByLabel(/Storage target 5 backend|存储目标 5 后端/).selectOption("GRAPH");
  await page.getByLabel(/Storage target 5 graph namespace|存储目标 5 图命名空间/).fill("rag_support_mixed_tree");
  await page.getByLabel(/Storage target 5 node label|存储目标 5 节点标签/).fill("RagTreeNode");
  await expect(page.getByLabel(/Enable graph capability|Graph enabled|启用图能力/)).toBeHidden();
  await page.getByLabel(/Summary tree target|Tree target|摘要树目标/).selectOption("target-5");
  await page.getByRole("button", { name: /^Save$|^保存$/ }).click();

  await expect(page).toHaveURL(/\/rag$/);
  await expect(notifications(page, /Support Mixed (created|已创建)/)).toBeVisible();

  const verifyBodies = requestBodies(requests, "/api/v1/rag/corpora/verify", "POST")
    .filter((body) => body.corpusId === "support-mixed");
  expect(verifyBodies).toHaveLength(1);

  const createBodies = requestBodies(requests, "/api/v1/rag/corpora", "POST")
    .filter((body) => body.corpusId === "support-mixed");
  createBodies.forEach(expectCapabilityPayloadWithoutReserved);
  expect(createBodies).toEqual(expect.arrayContaining([
    expect.objectContaining({
      capabilities: expect.objectContaining({ graph: true }),
      corpusId: "support-mixed",
      storageBinding: expect.objectContaining({
        bindings: expect.objectContaining({
          source: "source-store",
          tree: "target-5",
          textUnit: "text-unit-store",
          vector: "vector-store",
        }),
        targets: expect.arrayContaining([
          expect.objectContaining({
            backend: "GRAPH",
            id: "target-5",
            layout: "GRAPH_TREE",
            namespace: "rag_support_mixed_tree",
            nodeLabel: "RagTreeNode",
          }),
        ]),
      }),
    }),
  ]));
});
