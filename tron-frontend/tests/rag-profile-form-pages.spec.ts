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

function responseBodies(requests: CapturedRequest[], pathname: string, method: string) {
  return requests
    .filter((request) => request.pathname === pathname && request.method === method)
    .map((request) => request.responseData as Record<string, unknown> | undefined)
    .filter((response): response is Record<string, unknown> => Boolean(response));
}

function pipelineNode(body: Record<string, unknown> | undefined, stageType: string) {
  const pipeline = body?.pipeline as Record<string, unknown> | undefined;
  const nodes = Array.isArray(pipeline?.nodes) ? pipeline.nodes as Array<Record<string, unknown>> : [];
  return nodes.find((node) => node.stageType === stageType);
}

async function openAdvancedNodeControls(page: Page) {
  await page.locator("summary").filter({ hasText: /Advanced Node Control|高级节点控制/ }).click();
}

const label = {
  description: /Description|描述/,
  ingestProfile: /Ingest Profile|写入配置/,
  primaryCorpus: /Primary Corpus|主语料库/,
  profileId: /Profile ID|配置标识/,
  splitStrategy: /SPLIT_TEXT (strategy ID|策略标识)/,
  supportedSourceTypes: /Supported Source Types|支持的来源类型/,
  vectorStrategy: /(^|[^_])VECTOR_INDEX (strategy ID|策略标识)$/,
};

const buttonName = {
  addIngestProfile: /Add Ingest Profile|新增写入配置/,
  editProjectDocIngest: /Edit project-doc-ingest-v1|编辑 project-doc-ingest-v1/,
  editSupportNotesIngest: /Edit support-notes-ingest-v1|编辑 support-notes-ingest-v1/,
  enablePersistSource: /Enable PERSIST_SOURCE node|启用 持久化来源 节点/,
  enablePreparePayload: /Enable PREPARE_PAYLOAD node|启用 准备载荷 节点/,
  enableSplitText: /Enable SPLIT_TEXT node|启用 文本切分 节点/,
  enableExtractTree: /Enable EXTRACT_TREE node|启用 提取结构树 节点/,
  enablePersistTree: /Enable PERSIST_TREE node|启用 持久化结构树 节点/,
  enableSummarizeTree: /Enable SUMMARIZE_TREE node|启用 摘要结构树 节点/,
  enableTreeFulltextIndex: /Enable TREE_FULLTEXT_INDEX node|启用 结构树全文索引 节点/,
  enableTreeVectorIndex: /Enable TREE_VECTOR_INDEX node|启用 结构树向量索引 节点/,
  enableVectorIndex: /Enable VECTOR_INDEX node|启用 向量索引 节点/,
  saveProfile: /Save Profile|保存配置/,
};

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
  await waitForFrontend(`${frontendBaseUrl}/rag?tab=profiles`);
});

test.afterAll(async () => {
  if (!frontendProcess || frontendProcess.killed) return;
  frontendProcess.kill("SIGTERM");
  await new Promise((resolveProcess) => frontendProcess?.once("exit", resolveProcess));
});

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => window.localStorage.setItem("tron.locale", "en-US"));
});

test("creates and edits ingest profiles through management APIs", async ({ page }) => {
  const requests: CapturedRequest[] = [];
  await installRagManagementApiMock(page, requests);

  await page.goto(`${frontendBaseUrl}/rag?tab=profiles`);
  await expect(page.getByRole("heading", { name: /RAG Management|知识检索管理/ })).toBeVisible();
  await expect(page.getByRole("heading", { name: /Profiles|配置档案/ })).toBeVisible();

  await page.getByRole("button", { name: buttonName.addIngestProfile }).click();
  await expect(page).toHaveURL(/\/rag\/profiles\/ingest\/new(\?.*)?$/);
  await expect(page.getByRole("heading", { name: /Create Ingest Profile|新建写入配置/ })).toBeVisible();
  await expect(page.getByRole("heading", { name: /Pipeline Flow|流水线流程/ })).toBeVisible();
  await expect(page.getByRole("heading", { name: /Stage Config|阶段策略/ })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Stage Bindings" })).toHaveCount(0);

  await page.getByLabel(label.profileId).fill("support-notes-ingest-v1");
  await page.getByLabel(label.primaryCorpus).selectOption("project-doc");
  await page.getByLabel(label.description).fill("Ingest support notes and incident runbooks.");
  await openAdvancedNodeControls(page);
  await page.getByRole("button", { name: buttonName.enablePreparePayload }).click();
  await page.getByRole("button", { name: buttonName.enableSplitText }).click();
  await page.getByRole("button", { name: buttonName.enableExtractTree }).click();
  await page.getByRole("button", { name: buttonName.enableSummarizeTree }).click();
  await page.getByRole("button", { name: buttonName.enableTreeFulltextIndex }).click();
  await page.getByRole("button", { name: buttonName.enableTreeVectorIndex }).click();
  await page.getByRole("button", { name: buttonName.enablePersistTree }).click();
  await page.getByRole("button", { name: buttonName.enablePersistSource }).click();
  await page.getByLabel(/PREPARE_PAYLOAD (strategy ID|策略标识)/).selectOption("RAG_TEXT_CONTRACT_PAYLOAD_V1");
  const splitStrategySelect = page.getByLabel(label.splitStrategy);
  await expect(splitStrategySelect).toContainText("RAG_SPLIT_PLAIN_TEXT_V1");
  await expect(splitStrategySelect).toContainText("RAG_SPLIT_MARKDOWN_HEADING_V1");
  await expect(splitStrategySelect).toContainText("RAG_SPLIT_TEXT");
  await splitStrategySelect.selectOption("RAG_SPLIT_MARKDOWN_HEADING_V1");
  await expect(page.getByLabel(/Text split splitter type|文本切分切分器类型/)).toHaveValue("markdownHeading");
  await expect(page.getByLabel(/Text split max tokens|文本切分最大令牌数/)).toHaveValue("800");
  await expect(page.getByLabel(/Text split overlap tokens|文本切分重叠令牌数/)).toHaveValue("80");
  await expect(page.getByLabel(/Text split heading depth|文本切分标题深度/)).toHaveValue("6");
  await splitStrategySelect.selectOption("RAG_SPLIT_PLAIN_TEXT_V1");
  await expect(page.getByLabel(/Text split splitter type|文本切分切分器类型/)).toHaveValue("plainText");
  await page.getByLabel(/EXTRACT_TREE (strategy ID|策略标识)/).selectOption("RAG_EXTRACT_TREE");
  await page.getByLabel(/SUMMARIZE_TREE (strategy ID|策略标识)/).selectOption("RAG_SUMMARIZE_TREE_V1");
  await page.getByLabel(/TREE_FULLTEXT_INDEX (strategy ID|策略标识)/).selectOption("RAG_TREE_FULLTEXT_INDEX_V1");
  await page.getByLabel(/TREE_VECTOR_INDEX (strategy ID|策略标识)/).selectOption("RAG_TREE_VECTOR_INDEX_V1");
  await page.getByLabel(/PERSIST_TREE (strategy ID|策略标识)/).selectOption("RAG_PERSIST_TREE");
  await page.getByLabel(/PERSIST_SOURCE (strategy ID|策略标识)/).selectOption("RAG_PERSIST_SOURCE");
  await page.getByLabel(label.supportedSourceTypes).fill("RUNBOOK, INCIDENT_NOTE");
  await page.getByRole("button", { name: buttonName.saveProfile }).click();

  await expect(page).toHaveURL(/\/rag\?tab=profiles(&profileType=ingest)?$/);
  await expect(notifications(page, /support-notes-ingest-v1 (created|已创建)/)).toBeVisible();
  await expect(page.getByRole("button", { name: buttonName.editSupportNotesIngest })).toBeVisible();

  await page.getByRole("button", { name: buttonName.editSupportNotesIngest }).click();
  await expect(page).toHaveURL(/\/rag\/profiles\/ingest\/support-notes-ingest-v1\/edit(\?.*)?$/);
  await expect(page.getByLabel(label.profileId)).toBeDisabled();
  await expect(page.getByLabel(label.description)).toHaveValue("Ingest support notes and incident runbooks.");
  await expect(page.getByRole("heading", { name: /Pipeline Flow|流水线流程/ })).toBeVisible();

  await page.getByLabel(label.description).fill("Ingest support notes, incident runbooks, and QA handbooks.");
  await openAdvancedNodeControls(page);
  await page.getByRole("button", { name: buttonName.enableVectorIndex }).click();
  await page.getByLabel(label.splitStrategy).selectOption("RAG_SPLIT_MARKDOWN_HEADING_V1");
  await expect(page.getByLabel(/Text split splitter type|文本切分切分器类型/)).toHaveValue("markdownHeading");
  await expect(page.getByLabel(/Text split heading depth|文本切分标题深度/)).toHaveValue("6");
  await page.getByLabel(label.vectorStrategy).selectOption("RAG_VECTOR_INDEX");
  await page.getByRole("button", { name: buttonName.saveProfile }).click();

  await expect(page).toHaveURL(/\/rag\?tab=profiles(&profileType=ingest)?$/);
  await expect(notifications(page, /support-notes-ingest-v1 (saved|已保存)/)).toBeVisible();
  await page.getByRole("tab", { name: /Workbench|调试台/ }).click();
  await expect(page.getByLabel(label.ingestProfile)).toContainText("support-notes-ingest-v1");

  expect(requests.some((request) => request.pathname === "/api/v1/rag/profiles" && request.method === "GET")).toBeTruthy();
  expect(requests.some((request) => request.pathname === "/api/v1/rag/profiles/catalog" && request.method === "GET")).toBeTruthy();
  expect(requests.some((request) => request.pathname === "/api/v1/rag/profiles/support-notes-ingest-v1" && request.method === "GET")).toBeTruthy();

  const verifyBodies = requestBodies(requests, "/api/v1/rag/profiles/verify", "POST")
    .filter((body) => body.profileType === "INGEST");
  expect(verifyBodies).toEqual(expect.arrayContaining([
    expect.objectContaining({
      pipeline: expect.objectContaining({
        edges: expect.arrayContaining([
          expect.objectContaining({ from: "prepare-payload", to: "split-text" }),
          expect.objectContaining({ from: "split-text", to: "persist-source" }),
        ]),
        mode: "DAG",
        nodes: expect.arrayContaining([
          expect.objectContaining({ nodeId: "prepare-payload", stageType: "PREPARE_PAYLOAD", strategyId: "RAG_TEXT_CONTRACT_PAYLOAD_V1" }),
          expect.objectContaining({ nodeId: "split-text", stageType: "SPLIT_TEXT", strategyId: "RAG_SPLIT_PLAIN_TEXT_V1" }),
          expect.objectContaining({ nodeId: "extract-tree", stageType: "EXTRACT_TREE", strategyId: "RAG_EXTRACT_TREE" }),
          expect.objectContaining({ nodeId: "summarize-tree", stageType: "SUMMARIZE_TREE", strategyId: "RAG_SUMMARIZE_TREE_V1" }),
          expect.objectContaining({ nodeId: "tree-fulltext-index", stageType: "TREE_FULLTEXT_INDEX", strategyId: "RAG_TREE_FULLTEXT_INDEX_V1" }),
          expect.objectContaining({ nodeId: "tree-vector-index", stageType: "TREE_VECTOR_INDEX", strategyId: "RAG_TREE_VECTOR_INDEX_V1" }),
          expect.objectContaining({ nodeId: "persist-tree", stageType: "PERSIST_TREE", strategyId: "RAG_PERSIST_TREE" }),
          expect.objectContaining({ nodeId: "persist-source", stageType: "PERSIST_SOURCE", strategyId: "RAG_PERSIST_SOURCE" }),
        ]),
      }),
      profileId: "support-notes-ingest-v1",
      profileType: "INGEST",
      primaryCorpusId: "project-doc",
      requiredSourceVersion: "v1",
    }),
  ]));

  const createBodies = requestBodies(requests, "/api/v1/rag/profiles", "POST")
    .filter((body) => body.profileType === "INGEST");
  const createBody = createBodies.find((body) => body.profileId === "support-notes-ingest-v1");
  expect(createBody).toEqual(expect.objectContaining({
    pipeline: expect.objectContaining({
      mode: "DAG",
      nodes: expect.arrayContaining([
        expect.objectContaining({ enabled: true, nodeId: "prepare-payload", stageType: "PREPARE_PAYLOAD", strategyId: "RAG_TEXT_CONTRACT_PAYLOAD_V1" }),
        expect.objectContaining({ enabled: true, nodeId: "split-text", stageType: "SPLIT_TEXT", strategyId: "RAG_SPLIT_PLAIN_TEXT_V1" }),
        expect.objectContaining({ enabled: true, nodeId: "summarize-tree", stageType: "SUMMARIZE_TREE", strategyId: "RAG_SUMMARIZE_TREE_V1" }),
        expect.objectContaining({ enabled: true, nodeId: "tree-fulltext-index", stageType: "TREE_FULLTEXT_INDEX", strategyId: "RAG_TREE_FULLTEXT_INDEX_V1" }),
        expect.objectContaining({ enabled: true, nodeId: "tree-vector-index", stageType: "TREE_VECTOR_INDEX", strategyId: "RAG_TREE_VECTOR_INDEX_V1" }),
        expect.objectContaining({ enabled: true, nodeId: "persist-tree", stageType: "PERSIST_TREE", strategyId: "RAG_PERSIST_TREE" }),
        expect.objectContaining({ enabled: true, nodeId: "persist-source", stageType: "PERSIST_SOURCE", strategyId: "RAG_PERSIST_SOURCE" }),
      ]),
    }),
    profileId: "support-notes-ingest-v1",
    status: "enabled",
    sourceInputPolicy: expect.objectContaining({
      supportedSourceTypes: ["RUNBOOK", "INCIDENT_NOTE"],
    }),
  }));
  expect(pipelineNode(createBody, "SPLIT_TEXT")).toEqual(expect.objectContaining({
    config: expect.objectContaining({
      splitter: expect.objectContaining({
        maxTokens: 800,
        overlapTokens: 80,
        type: "plainText",
      }),
    }),
    strategyId: "RAG_SPLIT_PLAIN_TEXT_V1",
  }));
  expect(createBody).not.toHaveProperty("pipelineStages");

  const updateBodies = requestBodies(requests, "/api/v1/rag/profiles/support-notes-ingest-v1", "PUT");
  const updateBody = updateBodies.find((body) => body.profileId === "support-notes-ingest-v1");
  expect(updateBody).toEqual(expect.objectContaining({
    description: "Ingest support notes, incident runbooks, and QA handbooks.",
    pipeline: expect.objectContaining({
      nodes: expect.arrayContaining([
        expect.objectContaining({ enabled: true, nodeId: "split-text", stageType: "SPLIT_TEXT", strategyId: "RAG_SPLIT_MARKDOWN_HEADING_V1" }),
        expect.objectContaining({ enabled: true, nodeId: "vector-index", stageType: "VECTOR_INDEX", strategyId: "RAG_VECTOR_INDEX" }),
      ]),
    }),
    profileId: "support-notes-ingest-v1",
    status: "enabled",
  }));
  expect(pipelineNode(updateBody, "SPLIT_TEXT")).toEqual(expect.objectContaining({
    config: expect.objectContaining({
      splitter: expect.objectContaining({
        headingDepth: 6,
        maxTokens: 800,
        overlapTokens: 80,
        type: "markdownHeading",
      }),
    }),
    strategyId: "RAG_SPLIT_MARKDOWN_HEADING_V1",
  }));
  expect(updateBody).not.toHaveProperty("pipelineStages");
});

test("preserves legacy split strategy when editing existing ingest profiles", async ({ page }) => {
  const requests: CapturedRequest[] = [];
  await installRagManagementApiMock(page, requests);

  await page.goto(`${frontendBaseUrl}/rag/profiles/ingest/project-doc-ingest-v1/edit`);
  await expect(page.getByRole("heading", { name: /Edit Ingest Profile|编辑写入配置/ })).toBeVisible();
  const splitStrategySelect = page.getByLabel(label.splitStrategy);
  await expect(splitStrategySelect).toContainText("RAG_SPLIT_PLAIN_TEXT_V1");
  await expect(splitStrategySelect).toContainText("RAG_SPLIT_MARKDOWN_HEADING_V1");
  await expect(splitStrategySelect).toContainText("RAG_SPLIT_TEXT");
  await expect(splitStrategySelect).toHaveValue("RAG_SPLIT_TEXT");
  await expect(page.getByLabel(/Text split splitter type|文本切分切分器类型/)).toHaveValue("plainText");

  await page.getByRole("button", { name: buttonName.saveProfile }).click();

  await expect(page).toHaveURL(/\/rag\?tab=profiles(&profileType=ingest)?$/);
  await expect(notifications(page, /project-doc-ingest-v1 (saved|已保存)/)).toBeVisible();

  const updateBodies = requestBodies(requests, "/api/v1/rag/profiles/project-doc-ingest-v1", "PUT");
  const updateBody = updateBodies.find((body) => body.profileId === "project-doc-ingest-v1");
  expect(pipelineNode(updateBody, "SPLIT_TEXT")).toEqual(expect.objectContaining({
    config: expect.objectContaining({
      splitter: expect.objectContaining({
        type: "plainText",
      }),
    }),
    strategyId: "RAG_SPLIT_TEXT",
  }));
});

test("workbench displays summary tree hit metadata", async ({ page }) => {
  const requests: CapturedRequest[] = [];
  await installRagManagementApiMock(page, requests);

  await page.goto(`${frontendBaseUrl}/rag?tab=workbench`);
  await expect(page.getByRole("heading", { name: /RAG Management|知识检索管理/ })).toBeVisible();
  await page.getByRole("tab", { name: /Retrieval Test|检索测试/ }).click();
  await page.getByRole("textbox", { name: /^(Query|查询)$/ }).fill("How does summary tree retrieval surface metadata?");
  await page.getByRole("button", { name: /Run Search|执行检索/ }).click();

  await expect(page.getByText("TREE_VECTOR")).toBeVisible();
  await expect(page.getByText("tree-runtime-workbench")).toBeVisible();
  await expect(page.getByText("Runtime Workbench Summary")).toBeVisible();
  await expect(page.getByText("0.880")).toBeVisible();

  const searchBodies = requestBodies(requests, "/api/v1/rag/search", "POST");
  expect(searchBodies).toEqual(expect.arrayContaining([
    expect.objectContaining({
      query: "How does summary tree retrieval surface metadata?",
    }),
  ]));
});

test("retrieval profile keeps summary tree route types", async ({ page }) => {
  const requests: CapturedRequest[] = [];
  await installRagManagementApiMock(page, requests);

  await page.goto(`${frontendBaseUrl}/rag/profiles/retrieval/new`);
  await expect(page.getByRole("heading", { name: /Create Retrieval Profile|新建检索配置/ })).toBeVisible();

  await page.getByLabel(/Profile ID|配置标识/).fill("project-doc-tree-related-v1");
  await page.getByLabel(/Primary Corpus|主语料库/).selectOption("project-doc");
  await page.getByLabel(/Description|描述/).fill("Search project documentation summary tree nodes.");
  await page.getByLabel(/Route 1 Corpus|路由 1 语料库/).selectOption("project-doc");
  await page.getByLabel(/Route 1 Retrieval Strategy|路由 1 检索策略/).selectOption("TREE_FULLTEXT_V1");
  await page.getByRole("button", { name: /Add Route|添加路由/ }).click();
  await page.getByLabel(/Route 2 Corpus|路由 2 语料库/).selectOption("project-doc");
  await page.getByLabel(/Route 2 Retrieval Strategy|路由 2 检索策略/).selectOption("TREE_VECTOR_V1");
  await page.getByRole("button", { name: /Save Profile|保存配置/ }).click();

  await expect(page).toHaveURL(/\/rag\?tab=profiles/);

  const createBodies = requestBodies(requests, "/api/v1/rag/profiles", "POST")
    .filter((body) => body.profileId === "project-doc-tree-related-v1");
  expect(createBodies).toEqual(expect.arrayContaining([
    expect.objectContaining({
      pipeline: expect.objectContaining({
        nodes: expect.arrayContaining([
          expect.objectContaining({
            config: expect.objectContaining({ corpusId: "project-doc", routeType: "TREE_FULLTEXT" }),
            stageType: "ROUTE_SEARCH",
            strategyId: "TREE_FULLTEXT_V1",
          }),
          expect.objectContaining({
            config: expect.objectContaining({ corpusId: "project-doc", routeType: "TREE_VECTOR" }),
            stageType: "ROUTE_SEARCH",
            strategyId: "TREE_VECTOR_V1",
          }),
        ]),
      }),
      profileType: "RETRIEVAL",
    }),
  ]));
});

test("creates and edits multi-corpus retrieval profiles through management APIs", async ({ page }) => {
  const requests: CapturedRequest[] = [];
  await installRagManagementApiMock(page, requests);

  await page.goto(`${frontendBaseUrl}/rag?tab=profiles`);
  await expect(page.getByRole("button", { name: "Add Retrieval Profile" })).toBeVisible();
  await page.getByRole("button", { name: "Add Retrieval Profile" }).click();

  await expect(page).toHaveURL(/\/rag\/profiles\/retrieval\/new$/);
  await expect(page.getByRole("heading", { name: "Create Retrieval Profile" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Pipeline Nodes" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Pipeline Edges" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Stage Config" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Backend Payload Preview" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Verify Result" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Routes" })).toHaveCount(0);

  await page.getByLabel("Profile ID").fill("support-fusion-v1");
  await page.getByLabel("Status").selectOption("Ready");
  await page.getByLabel("Primary Corpus").selectOption("memory-card");
  await page.getByLabel("Description").fill("Fuse memory and project documentation support answers.");
  await page.getByLabel("Node 1 corpus").selectOption("memory-card");
  await page.getByLabel("Node 1 stage type").selectOption("FULLTEXT");
  await page.getByLabel("Node 1 strategy ID").selectOption("FULLTEXT_SQLITE_FTS5_SIMPLE_V1");
  await page.getByRole("button", { name: "Add Pipeline Node" }).click();
  await page.getByLabel("Node 2 corpus").selectOption("project-doc");
  await page.getByLabel("Node 2 stage type").selectOption("VECTOR");
  await page.getByLabel("Node 2 strategy ID").selectOption("VECTOR_SQLITE_VEC0_COSINE_V1");
  await page.getByRole("button", { name: "Add Pipeline Node" }).click();
  await page.getByLabel("Node 3 corpus").selectOption("project-doc");
  await page.getByLabel("Node 3 stage type").selectOption("TREE_FULLTEXT");
  await page.getByLabel("Node 3 strategy ID").selectOption("TREE_FULLTEXT_V1");
  await page.getByRole("button", { name: "Add Pipeline Node" }).click();
  await page.getByLabel("Node 4 corpus").selectOption("project-doc");
  await page.getByLabel("Node 4 stage type").selectOption("TREE_VECTOR");
  await page.getByLabel("Node 4 strategy ID").selectOption("TREE_VECTOR_V1");
  await page.getByRole("button", { name: "Add Filter" }).click();
  await page.getByLabel("Filter 1 field").selectOption("scopeCode");
  await page.getByLabel("Filter 1 operator").selectOption("eq");
  await page.getByLabel("Filter 1 value").fill("TRON_CORE");
  await page.getByRole("button", { name: "Save Profile" }).click();

  await expect(page).toHaveURL(/\/rag\?tab=profiles$/);
  await expect(notifications(page, "support-fusion-v1 created")).toBeVisible();
  await expect(page.getByRole("button", { name: "Edit support-fusion-v1" })).toBeVisible();

  await page.getByRole("button", { name: "Edit support-fusion-v1" }).click();
  await expect(page).toHaveURL(/\/rag\/profiles\/retrieval\/support-fusion-v1\/edit$/);
  await expect(page.getByLabel("Node 1 corpus")).toHaveValue("memory-card");
  await expect(page.getByLabel("Node 2 corpus")).toHaveValue("project-doc");
  await expect(page.getByLabel("Filter 1 value")).toHaveValue("TRON_CORE");

  await page.getByLabel("Description").fill("Fuse memory, documentation, and curated support signals.");
  await page.getByLabel("Topic Policy").selectOption("EMBEDDING_MMR_V1");
  await page.getByRole("button", { name: "Save Profile" }).click();

  await expect(page).toHaveURL(/\/rag\?tab=profiles$/);
  await expect(notifications(page, "support-fusion-v1 saved")).toBeVisible();
  await page.getByRole("tab", { name: "Workbench" }).click();
  await expect(page.getByLabel("Retrieval Profile")).toContainText("support-fusion-v1");

  const verifyBodies = requestBodies(requests, "/api/v1/rag/profiles/verify", "POST")
    .filter((body) => body.profileType === "RETRIEVAL");
  expect(verifyBodies).toEqual(expect.arrayContaining([
    expect.objectContaining({
      corpusIds: ["memory-card", "project-doc"],
      pipeline: expect.objectContaining({
        mode: "DAG",
        nodes: expect.arrayContaining([
          expect.objectContaining({ config: expect.objectContaining({ corpusId: "memory-card" }), nodeId: "memory-card-fulltext-1", stageType: "FULLTEXT", strategyId: "FULLTEXT_SQLITE_FTS5_SIMPLE_V1" }),
          expect.objectContaining({ config: expect.objectContaining({ corpusId: "project-doc" }), nodeId: "project-doc-vector-2", stageType: "VECTOR", strategyId: "VECTOR_SQLITE_VEC0_COSINE_V1" }),
          expect.objectContaining({ config: expect.objectContaining({ corpusId: "project-doc", routeType: "TREE_FULLTEXT" }), nodeId: "project-doc-tree-fulltext-3", stageType: "TREE_FULLTEXT", strategyId: "TREE_FULLTEXT_V1" }),
          expect.objectContaining({ config: expect.objectContaining({ corpusId: "project-doc", routeType: "TREE_VECTOR" }), nodeId: "project-doc-tree-vector-4", stageType: "TREE_VECTOR", strategyId: "TREE_VECTOR_V1" }),
        ]),
      }),
      primaryCorpusId: "memory-card",
      profileId: "support-fusion-v1",
      profileType: "RETRIEVAL",
    }),
  ]));

  const createBodies = requestBodies(requests, "/api/v1/rag/profiles", "POST")
    .filter((body) => body.profileType === "RETRIEVAL");
  const createBody = createBodies.find((body) => body.profileId === "support-fusion-v1");
  expect(createBody).toEqual(expect.objectContaining({
    corpusIds: ["memory-card", "project-doc"],
    filters: [expect.objectContaining({ field: "scopeCode", operator: "eq", value: "TRON_CORE" })],
    fusionPolicy: "RRF",
    pipeline: expect.objectContaining({
      edges: expect.arrayContaining([
        expect.objectContaining({ from: "memory-card-fulltext-1", to: "project-doc-vector-2" }),
        expect.objectContaining({ from: "project-doc-vector-2", to: "project-doc-tree-fulltext-3" }),
        expect.objectContaining({ from: "project-doc-tree-fulltext-3", to: "project-doc-tree-vector-4" }),
      ]),
      mode: "DAG",
    }),
    profileId: "support-fusion-v1",
  }));
  expect(createBody).not.toHaveProperty("routes");

  const updateBodies = requestBodies(requests, "/api/v1/rag/profiles/support-fusion-v1", "PUT");
  const updateBody = updateBodies.find((body) => body.profileId === "support-fusion-v1");
  expect(updateBody).toEqual(expect.objectContaining({
    corpusIds: ["memory-card", "project-doc"],
    description: "Fuse memory, documentation, and curated support signals.",
    pipeline: expect.objectContaining({ mode: "DAG" }),
    profileId: "support-fusion-v1",
    status: "enabled",
    topicPolicy: "EMBEDDING_MMR_V1",
  }));
  expect(updateBody).not.toHaveProperty("routes");
});

test("runs ingest and search from the workbench through runtime APIs", async ({ page }) => {
  const requests: CapturedRequest[] = [];
  await installRagManagementApiMock(page, requests);

  await page.goto(`${frontendBaseUrl}/rag?tab=workbench`);
  await expect(page.getByRole("heading", { name: "RAG Management" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Ingest Request" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Retrieval Test" })).toBeVisible();

  await expect(page.getByLabel("Corpus")).toContainText("memory-card");
  await expect(page.getByLabel("Ingest Profile")).toContainText("project-doc-ingest-v1");
  await expect(page.getByLabel("Retrieval Profile")).toContainText("project-doc-related-v1");

  await page.getByLabel("Corpus").selectOption("project-doc");
  await page.getByLabel("Ingest Profile").selectOption("project-doc-ingest-v1");
  await page.getByLabel("Target Scope").fill("workspace:TRON_CORE");
  await page.getByLabel("Source Code").fill("docs/features/rag-storage-runtime-workbench");
  await page.getByLabel("Source Type").selectOption("MARKDOWN");
  await expect(page.getByLabel("Source Version")).toHaveValue("v1");
  await page.getByLabel("Language").fill("en-US");
  await page.getByLabel("Metadata Source Path").fill("docs/features/rag-storage-runtime-workbench");
  await page.getByRole("button", { name: "Add Business Attribute" }).click();
  await page.getByLabel("Business Attribute 1 Field").selectOption("documentCode");
  await page.getByLabel("Business Attribute 1 Value").fill("rag-storage-runtime-workbench");
  await page.getByRole("button", { name: "Add Business Attribute" }).click();
  await page.getByLabel("Business Attribute 2 Field").selectOption("repoCode");
  await page.getByLabel("Business Attribute 2 Value").fill("tron");
  await page.getByLabel("Text Content").fill("Workbench ingest verifies backend runtime writes from the frontend.");
  await page.getByRole("button", { name: "Submit Ingest" }).click();

  await expect(page.getByText("Ingest completed for docs/features/rag-storage-runtime-workbench.").first()).toBeVisible();
  await expect(page.getByText("Text Units")).toBeVisible();
  await expect(page.getByText("3").last()).toBeVisible();
  await expect(page.getByRole("heading", { name: "Ingest Stage Results" })).toBeVisible();
  await expect(page.getByText("prepare-payload")).toBeVisible();
  await expect(page.getByText("PREPARE_PAYLOAD")).toBeVisible();
  await expect(page.getByText("RAG_TEXT_CONTRACT_PAYLOAD_V1")).toBeVisible();
  await expect(page.getByText("COMPLETED · 2ms")).toBeVisible();
  await expect(page.getByText("request.payload")).toBeVisible();
  await expect(page.getByText("payload.prepared, source.metadata")).toBeVisible();
  await expect(page.getByText("split-text")).toBeVisible();
  await expect(page.getByText("SPLIT_TEXT · RAG_SPLIT_TEXT")).toBeVisible();
  await expect(page.getByText("COMPLETED · 3ms")).toBeVisible();
  await expect(page.getByText("payload.prepared").first()).toBeVisible();
  await expect(page.getByText("textUnits.generated")).toBeVisible();
  await expect(page.getByText("Warning splitter used default overlap")).toBeVisible();

  await page.getByLabel("Retrieval Profile").selectOption("project-doc-related-v1");
  await page.getByLabel("Query", { exact: true }).fill("How does the runtime workbench call backend search?");
  await page.getByLabel("Context Strategy").selectOption("DOC_QA_CONTEXT_PACK");
  await page.getByRole("button", { name: "Add Query Field" }).click();
  await page.getByLabel("Query Field 1").selectOption("documentCode");
  await page.getByRole("button", { name: "Add Query Field" }).click();
  await page.getByLabel("Query Field 2").selectOption("repoCode");
  await page.getByRole("button", { name: "Add Filter Condition" }).click();
  await page.getByLabel("Filter 1 Field").selectOption("scopeCode");
  await page.getByLabel("Filter 1 Operator").selectOption("eq");
  await page.getByLabel("Filter 1 Value").fill("TRON_CORE");
  await page.getByLabel("Limit").fill("5");
  await page.getByLabel("Token Budget").fill("2048");
  await page.getByRole("button", { name: "Enable trace" }).click();
  await expect(page.getByRole("button", { name: "Enable trace" })).toHaveAttribute("aria-pressed", "false");
  await page.getByRole("button", { name: "Enable trace" }).click();
  await expect(page.getByRole("button", { name: "Enable trace" })).toHaveAttribute("aria-pressed", "true");
  await page.getByRole("button", { name: "Run Search" }).click();

  await expect(page.getByText("Search returned 1 hit.").first()).toBeVisible();
  await expect(page.getByText("trace-workbench-001")).toBeVisible();
  await expect(page.getByText("RAG Storage Runtime Workbench").first()).toBeVisible();
  await expect(page.getByText("TREE_VECTOR")).toBeVisible();
  await expect(page.getByText("tree-runtime-workbench")).toBeVisible();
  await expect(page.getByText("Runtime Workbench Summary")).toBeVisible();
  await expect(page.getByText("0.880")).toBeVisible();
  await expect(page.getByText("rag-storage-runtime-workbench", { exact: true })).toBeVisible();
  await expect(page.getByText("tron", { exact: true }).first()).toBeVisible();
  await expect(page.getByText("RAG workbench can execute retrieval through the backend API.")).toBeVisible();
  await expect(page.getByText("Workbench search result from mocked runtime.")).toBeVisible();
  await expect(page.getByRole("heading", { name: "Search Stage Results" })).toBeVisible();
  await expect(page.getByText("route-search-fulltext")).toBeVisible();
  await expect(page.getByText("ROUTE_SEARCH · FULLTEXT_SQLITE_FTS5_SIMPLE_V1")).toBeVisible();
  await expect(page.getByText("COMPLETED · 5ms")).toBeVisible();
  await expect(page.getByText("query.effective")).toBeVisible();
  await expect(page.getByText("hits.route")).toHaveCount(2);
  await expect(page.getByText("context-pack")).toBeVisible();
  await expect(page.getByText("CONTEXT_PACK · DOC_QA_CONTEXT_PACK")).toBeVisible();
  await expect(page.getByText("COMPLETED_WITH_WARNINGS · 7ms")).toBeVisible();
  await expect(page.getByText("context.pack")).toBeVisible();
  await expect(page.getByText("Warning rerank disabled; using lexical order")).toBeVisible();

  await page.getByLabel("Query", { exact: true }).fill("legacy fallback search without stage results");
  await page.getByRole("button", { name: "Run Search" }).click();
  await expect(page.getByText("Search returned 1 hit.").first()).toBeVisible();
  await expect(page.getByText("RAG Storage Runtime Workbench").first()).toBeVisible();
  await expect(page.getByRole("heading", { name: "Search Stage Results" })).toHaveCount(0);

  const ingestBodies = requestBodies(requests, "/api/v1/rag/ingest", "POST");
  expect(ingestBodies).toEqual(expect.arrayContaining([
    expect.objectContaining({
      accessContext: expect.objectContaining({
        agentCode: "AGENT_CODEX",
        userCode: "USR-AUTH-0001",
        workspaceCode: "TRON_CORE",
      }),
      corpusId: "project-doc",
      ingestProfileId: "project-doc-ingest-v1",
      payload: expect.objectContaining({
        domainFields: {
          documentCode: "rag-storage-runtime-workbench",
          repoCode: "tron",
        },
        source: expect.objectContaining({
          code: "docs/features/rag-storage-runtime-workbench",
          type: "MARKDOWN",
          version: "v1",
        }),
      }),
      targetScope: {
        scopeCode: "TRON_CORE",
        scopeType: "SPACE",
      },
    }),
  ]));
  const ingestResponses = responseBodies(requests, "/api/v1/rag/ingest", "POST");
  expect(ingestResponses).toEqual(expect.arrayContaining([
    expect.objectContaining({
      stageResults: expect.arrayContaining([
        expect.objectContaining({
          durationMs: 2,
          inputArtifactKeys: ["request.payload"],
          nodeId: "prepare-payload",
          outputArtifactKeys: ["payload.prepared", "source.metadata"],
          stageType: "PREPARE_PAYLOAD",
          status: "COMPLETED",
          strategyId: "RAG_TEXT_CONTRACT_PAYLOAD_V1",
        }),
        expect.objectContaining({
          durationMs: 3,
          inputArtifactKeys: ["payload.prepared"],
          nodeId: "split-text",
          outputArtifactKeys: ["textUnits.generated"],
          stageType: "SPLIT_TEXT",
          status: "COMPLETED",
          strategyId: "RAG_SPLIT_TEXT",
          warnings: ["splitter used default overlap"],
        }),
      ]),
    }),
  ]));

  const searchBodies = requestBodies(requests, "/api/v1/rag/search", "POST");
  expect(searchBodies).toEqual(expect.arrayContaining([
    expect.objectContaining({
      accessContext: expect.objectContaining({
        agentCode: "AGENT_CODEX",
        userCode: "USR-AUTH-0001",
        workspaceCode: "TRON_CORE",
      }),
      context: expect.objectContaining({
        returnFields: ["documentCode", "repoCode"],
        strategy: "DOC_QA_CONTEXT_PACK",
      }),
      filters: {
        scopeCode: {
          op: "eq",
          value: "TRON_CORE",
        },
      },
      limit: 5,
      profileId: "project-doc-related-v1",
      query: "How does the runtime workbench call backend search?",
      tokenBudget: 2048,
      trace: { enabled: true },
    }),
  ]));
  const searchResponses = responseBodies(requests, "/api/v1/rag/search", "POST");
  const detailedSearchResponse = searchResponses.find((response) => (
    response.query === "How does the runtime workbench call backend search?"
  ));
  expect(detailedSearchResponse).toEqual(expect.objectContaining({
    stageResults: expect.arrayContaining([
      expect.objectContaining({
        durationMs: 5,
        inputArtifactKeys: ["query.effective"],
        nodeId: "route-search-fulltext",
        outputArtifactKeys: ["hits.route"],
        stageType: "ROUTE_SEARCH",
        status: "COMPLETED",
        strategyId: "FULLTEXT_SQLITE_FTS5_SIMPLE_V1",
      }),
      expect.objectContaining({
        durationMs: 7,
        inputArtifactKeys: ["hits.route"],
        nodeId: "context-pack",
        outputArtifactKeys: ["context.pack"],
        stageType: "CONTEXT_PACK",
        status: "COMPLETED_WITH_WARNINGS",
        strategyId: "DOC_QA_CONTEXT_PACK",
        warnings: ["rerank disabled; using lexical order"],
      }),
    ]),
    traceCode: "trace-workbench-001",
  }));
  const legacySearchResponse = searchResponses.find((response) => (
    response.query === "legacy fallback search without stage results"
  ));
  expect(legacySearchResponse).toBeDefined();
  expect(legacySearchResponse).not.toHaveProperty("stageResults");
});
