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
  enableFulltextIndex: /Enable FULLTEXT_INDEX node|启用 全文索引 节点/,
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

test("pipeline flow shows actual outgoing targets for parallel branches", async ({ page }) => {
  const requests: CapturedRequest[] = [];
  await installRagManagementApiMock(page, requests);

  await page.goto(`${frontendBaseUrl}/rag/profiles/ingest/new?returnProfileType=ingest`);
  await expect(page.getByRole("heading", { name: /Pipeline Flow|流水线流程/ })).toBeVisible();
  await openAdvancedNodeControls(page);
  await page.getByRole("button", { name: buttonName.enableSplitText }).click();
  await page.getByRole("button", { name: buttonName.enableExtractTree }).click();
  await page.getByRole("button", { name: buttonName.enableSummarizeTree }).click();
  await page.getByRole("button", { name: buttonName.enableFulltextIndex }).click();
  await page.getByRole("button", { name: buttonName.enableVectorIndex }).click();
  await page.getByRole("button", { name: buttonName.enablePersistSource }).click();

  const flow = page.getByTestId("pipeline-flow");
  const fulltextNode = flow.getByTestId("pipeline-flow-node-fulltext-index");
  await expect(fulltextNode).toContainText("→ persist-source");
  await expect(fulltextNode).not.toContainText("summarize-tree");
  await expect(flow.getByTestId("pipeline-flow-node-vector-index")).toContainText("→ persist-source");
  await expect(flow.getByTestId("pipeline-flow-node-extract-tree")).toContainText("→ summarize-tree");
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
  await page.getByLabel(/EXTRACT_TREE (strategy ID|策略标识)/).selectOption("RAG_MARKDOWN_TREE_EXTRACT_V1");
  await page.getByLabel(/SUMMARIZE_TREE (strategy ID|策略标识)/).selectOption("RAG_TREE_EXTRACTIVE_SUMMARY_V1");
  await page.getByLabel(/TREE_FULLTEXT_INDEX (strategy ID|策略标识)/).selectOption("RAG_TREE_FULLTEXT_INDEX");
  await page.getByLabel(/TREE_VECTOR_INDEX (strategy ID|策略标识)/).selectOption("RAG_TREE_VECTOR_INDEX");
  await page.getByLabel(/PERSIST_TREE (strategy ID|策略标识)/).selectOption("RAG_PERSIST_TREE_GRAPH_V1");
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
          expect.objectContaining({ from: "persist-tree", to: "persist-source" }),
        ]),
        mode: "DAG",
        nodes: expect.arrayContaining([
          expect.objectContaining({ nodeId: "prepare-payload", stageType: "PREPARE_PAYLOAD", strategyId: "RAG_TEXT_CONTRACT_PAYLOAD_V1" }),
          expect.objectContaining({ nodeId: "split-text", stageType: "SPLIT_TEXT", strategyId: "RAG_SPLIT_PLAIN_TEXT_V1" }),
          expect.objectContaining({ nodeId: "extract-tree", stageType: "EXTRACT_TREE", strategyId: "RAG_MARKDOWN_TREE_EXTRACT_V1" }),
          expect.objectContaining({ nodeId: "summarize-tree", stageType: "SUMMARIZE_TREE", strategyId: "RAG_TREE_EXTRACTIVE_SUMMARY_V1" }),
          expect.objectContaining({ nodeId: "tree-fulltext-index", stageType: "TREE_FULLTEXT_INDEX", strategyId: "RAG_TREE_FULLTEXT_INDEX" }),
          expect.objectContaining({ nodeId: "tree-vector-index", stageType: "TREE_VECTOR_INDEX", strategyId: "RAG_TREE_VECTOR_INDEX" }),
          expect.objectContaining({ nodeId: "persist-tree", stageType: "PERSIST_TREE", strategyId: "RAG_PERSIST_TREE_GRAPH_V1" }),
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
      edges: expect.arrayContaining([
        expect.objectContaining({ from: "persist-tree", to: "persist-source" }),
      ]),
      mode: "DAG",
      nodes: expect.arrayContaining([
        expect.objectContaining({ enabled: true, nodeId: "prepare-payload", stageType: "PREPARE_PAYLOAD", strategyId: "RAG_TEXT_CONTRACT_PAYLOAD_V1" }),
        expect.objectContaining({ enabled: true, nodeId: "split-text", stageType: "SPLIT_TEXT", strategyId: "RAG_SPLIT_PLAIN_TEXT_V1" }),
        expect.objectContaining({ enabled: true, nodeId: "summarize-tree", stageType: "SUMMARIZE_TREE", strategyId: "RAG_TREE_EXTRACTIVE_SUMMARY_V1" }),
        expect.objectContaining({ enabled: true, nodeId: "tree-fulltext-index", stageType: "TREE_FULLTEXT_INDEX", strategyId: "RAG_TREE_FULLTEXT_INDEX" }),
        expect.objectContaining({ enabled: true, nodeId: "tree-vector-index", stageType: "TREE_VECTOR_INDEX", strategyId: "RAG_TREE_VECTOR_INDEX" }),
        expect.objectContaining({ enabled: true, nodeId: "persist-tree", stageType: "PERSIST_TREE", strategyId: "RAG_PERSIST_TREE_GRAPH_V1" }),
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

test("API catalog exposes summary tree strategies for graph tree corpora", async ({ page }) => {
  const requests: CapturedRequest[] = [];
  await installRagManagementApiMock(page, requests);

  await page.goto(`${frontendBaseUrl}/rag/profiles/retrieval/new`);
  await expect(page.getByRole("heading", { name: /Create Retrieval Profile|新建检索配置/ })).toBeVisible();
  const supportTreeDetail = await page.evaluate(async () => {
    const response = await fetch("/api/v1/rag/corpora/support-tree-graph");
    return response.json();
  });
  expect(supportTreeDetail.data.capabilities).toEqual(expect.objectContaining({
    fulltext: true,
    tree: true,
    vector: true,
  }));

  await page.getByLabel(/Profile ID|配置标识/).fill("support-tree-graph-related-v1");
  await page.getByLabel(/Primary Corpus|主语料库/).selectOption("support-tree-graph");
  await page.getByLabel(/Description|描述/).fill("Search support summary tree graph nodes.");
  await page.getByLabel(/Route 1 Corpus|路由 1 语料库/).selectOption("support-tree-graph");
  await expect(page.getByLabel(/Route 1 Retrieval Strategy|路由 1 检索策略/)).toContainText("TREE_FULLTEXT_V1");
  await page.getByLabel(/Route 1 Retrieval Strategy|路由 1 检索策略/).selectOption("TREE_FULLTEXT_V1");
  await page.getByRole("button", { name: /Add Route|添加路由/ }).click();
  await page.getByLabel(/Route 2 Corpus|路由 2 语料库/).selectOption("support-tree-graph");
  await expect(page.getByLabel(/Route 2 Retrieval Strategy|路由 2 检索策略/)).toContainText("TREE_VECTOR_V1");
  await page.getByLabel(/Route 2 Retrieval Strategy|路由 2 检索策略/).selectOption("TREE_VECTOR_V1");
  await page.getByRole("button", { name: /Save Profile|保存配置/ }).click();

  await expect(page).toHaveURL(/\/rag\?tab=profiles&profileType=retrieval$/);

  const createBodies = requestBodies(requests, "/api/v1/rag/profiles", "POST")
    .filter((body) => body.profileId === "support-tree-graph-related-v1");
  expect(createBodies).toEqual(expect.arrayContaining([
    expect.objectContaining({
      pipeline: expect.objectContaining({
        nodes: expect.arrayContaining([
          expect.objectContaining({
            config: expect.objectContaining({ corpusId: "support-tree-graph", routeType: "TREE_FULLTEXT" }),
            stageType: "ROUTE_SEARCH",
            strategyId: "TREE_FULLTEXT_V1",
          }),
          expect.objectContaining({
            config: expect.objectContaining({ corpusId: "support-tree-graph", routeType: "TREE_VECTOR" }),
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

  await page.goto(`${frontendBaseUrl}/rag?tab=profiles&profileType=retrieval`);
  await expect(page.getByRole("button", { name: /Add Retrieval Profile|新增检索配置/ })).toBeVisible();
  await page.getByRole("button", { name: /Add Retrieval Profile|新增检索配置/ }).click();

  await expect(page).toHaveURL(/\/rag\/profiles\/retrieval\/new(\?.*)?$/);
  await expect(page.getByRole("heading", { name: /Create Retrieval Profile|新建检索配置/ })).toBeVisible();
  await expect(page.getByRole("heading", { name: /Retrieval Routes|检索路由/ })).toBeVisible();
  await expect(page.getByRole("heading", { name: /Retrieval Flow|检索流程/ })).toBeVisible();
  await expect(page.getByText(/Backend Payload Preview|开发者预览/)).toBeVisible();
  await expect(page.getByRole("heading", { name: /Verify Result|校验结果/ })).toBeVisible();

  await page.getByLabel(label.profileId).fill("support-fusion-v1");
  await page.getByLabel(/Status|状态/).selectOption("Ready");
  await page.getByLabel(label.primaryCorpus).selectOption("memory-card");
  await page.getByLabel(label.description).fill("Fuse memory and project documentation support answers.");
  await page.getByLabel(/Route 1 Corpus|路由 1 语料库/).selectOption("memory-card");
  await page.getByLabel(/Route 1 Retrieval Strategy|路由 1 检索策略/).selectOption("FULLTEXT_SQLITE_FTS5_SIMPLE_V1");
  await page.getByRole("button", { name: /Add Route|添加路由/ }).click();
  await page.getByLabel(/Route 2 Corpus|路由 2 语料库/).selectOption("project-doc");
  await page.getByLabel(/Route 2 Retrieval Strategy|路由 2 检索策略/).selectOption("VECTOR_SQLITE_VEC0_COSINE_V1");
  await page.getByRole("button", { name: /Add Route|添加路由/ }).click();
  await page.getByLabel(/Route 3 Corpus|路由 3 语料库/).selectOption("project-doc");
  await page.getByLabel(/Route 3 Retrieval Strategy|路由 3 检索策略/).selectOption("TREE_FULLTEXT_V1");
  await page.getByRole("button", { name: /Add Route|添加路由/ }).click();
  await page.getByLabel(/Route 4 Corpus|路由 4 语料库/).selectOption("project-doc");
  await page.getByLabel(/Route 4 Retrieval Strategy|路由 4 检索策略/).selectOption("TREE_VECTOR_V1");
  await page.getByRole("button", { name: /Add Filter|添加筛选条件/ }).click();
  await page.getByLabel(/Filter 1 field|筛选条件 1 字段/).selectOption("scopeCode");
  await page.getByLabel(/Filter 1 operator|筛选条件 1 操作符/).selectOption("eq");
  await page.getByLabel(/Filter 1 value|筛选条件 1 值/).fill("TRON_CORE");
  await page.getByRole("button", { name: buttonName.saveProfile }).click();

  await expect(page).toHaveURL(/\/rag\?tab=profiles&profileType=retrieval$/);
  await expect(notifications(page, /support-fusion-v1 (created|已创建)|检索配置已创建/)).toBeVisible();
  await expect(page.getByRole("button", { name: /Edit support-fusion-v1|编辑 support-fusion-v1/ })).toBeVisible();

  await page.getByRole("button", { name: /Edit support-fusion-v1|编辑 support-fusion-v1/ }).click();
  await expect(page).toHaveURL(/\/rag\/profiles\/retrieval\/support-fusion-v1\/edit(\?.*)?$/);
  await expect(page.getByLabel(/Route 1 Corpus|路由 1 语料库/)).toHaveValue("memory-card");
  await expect(page.getByLabel(/Route 2 Corpus|路由 2 语料库/)).toHaveValue("project-doc");
  await expect(page.getByLabel(/Filter 1 value|筛选条件 1 值/)).toHaveValue("TRON_CORE");

  await page.getByLabel(label.description).fill("Fuse memory, documentation, and curated support signals.");
  await page.getByLabel(/Topic Policy|主题策略/).selectOption("EMBEDDING_MMR_V1");
  await page.getByRole("button", { name: buttonName.saveProfile }).click();

  await expect(page).toHaveURL(/\/rag\?tab=profiles&profileType=retrieval$/);
  await expect(notifications(page, /support-fusion-v1 (saved|已保存)|配置已更新/)).toBeVisible();
  await page.getByRole("tab", { name: /Workbench|调试台/ }).click();
  await page.getByRole("tab", { name: /Retrieval Test|检索测试/ }).click();
  await expect(page.getByLabel("检索配置", { exact: true })).toContainText("support-fusion-v1");

  const verifyBodies = requestBodies(requests, "/api/v1/rag/profiles/verify", "POST")
    .filter((body) => body.profileType === "RETRIEVAL");
  expect(verifyBodies).toEqual(expect.arrayContaining([
    expect.objectContaining({
      corpusIds: ["memory-card", "project-doc"],
      pipeline: expect.objectContaining({
        mode: "DAG",
        nodes: expect.arrayContaining([
          expect.objectContaining({ config: expect.objectContaining({ corpusId: "memory-card", routeType: "FULLTEXT" }), nodeId: "memory-card-retrieval-1", stageType: "ROUTE_SEARCH", strategyId: "FULLTEXT_SQLITE_FTS5_SIMPLE_V1" }),
          expect.objectContaining({ config: expect.objectContaining({ corpusId: "project-doc", routeType: "VECTOR" }), nodeId: "project-doc-retrieval-2", stageType: "ROUTE_SEARCH", strategyId: "VECTOR_SQLITE_VEC0_COSINE_V1" }),
          expect.objectContaining({ config: expect.objectContaining({ corpusId: "project-doc", routeType: "TREE_FULLTEXT" }), nodeId: "project-doc-retrieval-3", stageType: "ROUTE_SEARCH", strategyId: "TREE_FULLTEXT_V1" }),
          expect.objectContaining({ config: expect.objectContaining({ corpusId: "project-doc", routeType: "TREE_VECTOR" }), nodeId: "project-doc-retrieval-4", stageType: "ROUTE_SEARCH", strategyId: "TREE_VECTOR_V1" }),
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
        expect.objectContaining({ from: "memory-card-retrieval-1", to: "fuse-results" }),
        expect.objectContaining({ from: "project-doc-retrieval-2", to: "fuse-results" }),
        expect.objectContaining({ from: "project-doc-retrieval-3", to: "fuse-results" }),
        expect.objectContaining({ from: "project-doc-retrieval-4", to: "fuse-results" }),
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
  await expect(page.getByRole("heading", { name: "知识检索管理" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "写入请求" })).toBeVisible();
  await expect(page.getByRole("tab", { name: /检索测试/ })).toBeVisible();

  await expect(page.getByLabel("语料库", { exact: true })).toContainText("memory-card");
  await expect(page.getByLabel("写入配置", { exact: true })).toContainText("project-doc-ingest-v1");

  await page.getByLabel("语料库", { exact: true }).selectOption("project-doc");
  await page.getByLabel("写入配置", { exact: true }).selectOption("project-doc-ingest-v1");
  await expect(page.getByLabel("作用域类型", { exact: true })).toHaveValue("SPACE");
  await page.getByLabel("来源编码", { exact: true }).fill("docs/features/rag-storage-runtime-workbench");
  await page.getByLabel("来源类型", { exact: true }).selectOption("MARKDOWN");
  await expect(page.getByLabel("来源版本", { exact: true })).toHaveValue("v1");
  await page.getByLabel("语言", { exact: true }).fill("en-US");
  await page.getByLabel("元数据来源路径", { exact: true }).fill("docs/features/rag-storage-runtime-workbench");
  await page.getByRole("button", { name: "添加业务属性" }).click();
  await page.getByLabel("业务属性 1 字段", { exact: true }).selectOption("documentCode");
  await page.getByLabel("业务属性 1 值", { exact: true }).fill("rag-storage-runtime-workbench");
  await page.getByRole("button", { name: "添加业务属性" }).click();
  await page.getByLabel("业务属性 2 字段", { exact: true }).selectOption("repoCode");
  await page.getByLabel("业务属性 2 值", { exact: true }).fill("tron");
  await page.getByLabel("文本内容", { exact: true }).fill("Workbench ingest verifies backend runtime writes from the frontend.");
  await page.getByRole("button", { name: "提交写入" }).click();

  await expect(page.getByText("docs/features/rag-storage-runtime-workbench 写入完成。").first()).toBeVisible();
  const ingestResultPanel = page.locator("div").filter({ hasText: /^写入结果/ }).first();
  await expect(ingestResultPanel.getByText("文本单元")).toBeVisible();
  await expect(ingestResultPanel).toContainText(/文本单元\s*\d+/);
  const ingestStageHeading = page.getByRole("heading", { name: "写入阶段结果" });
  await expect(ingestStageHeading).toBeVisible();
  await ingestStageHeading.click();
  await expect(page.getByText("prepare-payload")).toBeVisible();
  await expect(page.getByText("PREPARE_PAYLOAD")).toBeVisible();
  await expect(page.getByText("RAG_TEXT_CONTRACT_PAYLOAD_V1")).toBeVisible();
  await expect(page.getByText("已完成 · 2ms")).toBeVisible();
  await expect(page.getByText("request.payload")).toBeVisible();
  await expect(page.getByText("payload.prepared, source.metadata")).toBeVisible();
  await expect(page.getByText("split-text")).toBeVisible();
  await expect(page.getByText("SPLIT_TEXT · RAG_SPLIT_TEXT")).toBeVisible();
  await expect(page.getByText("已完成 · 3ms")).toBeVisible();
  await expect(page.getByText("payload.prepared").first()).toBeVisible();
  await expect(page.getByText("textUnits.generated")).toBeVisible();
  await expect(page.getByText("告警：splitter used default overlap")).toBeVisible();

  await page.getByRole("tab", { name: /检索测试/ }).click();
  await page.getByLabel("检索配置", { exact: true }).selectOption("project-doc-related-v1");
  await page.getByLabel("查询", { exact: true }).fill("How does the runtime workbench call backend search?");
  await page.getByLabel("上下文策略", { exact: true }).selectOption("DOC_QA_CONTEXT_PACK");
  await page.getByRole("button", { name: "添加查询字段" }).click();
  await page.getByLabel("查询字段 1", { exact: true }).selectOption("documentCode");
  await page.getByRole("button", { name: "添加查询字段" }).click();
  await page.getByLabel("查询字段 2", { exact: true }).selectOption("repoCode");
  await page.getByRole("button", { name: "添加筛选条件" }).click();
  await page.getByLabel("筛选条件 1 字段", { exact: true }).selectOption("scopeCode");
  await page.getByLabel("筛选条件 1 操作符", { exact: true }).selectOption("eq");
  await page.getByLabel("筛选条件 1 值", { exact: true }).fill("TRON_CORE");
  await page.getByLabel("返回数量", { exact: true }).fill("5");
  await page.getByLabel("令牌预算", { exact: true }).fill("2048");
  await page.getByRole("button", { name: "启用追踪" }).click();
  await expect(page.getByRole("button", { name: "启用追踪" })).toHaveAttribute("aria-pressed", "false");
  await page.getByRole("button", { name: "启用追踪" }).click();
  await expect(page.getByRole("button", { name: "启用追踪" })).toHaveAttribute("aria-pressed", "true");
  await page.getByRole("button", { name: "执行检索" }).click();

  await expect(page.getByText("检索返回 1 条命中。").first()).toBeVisible();
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
  const searchStageHeading = page.getByRole("heading", { name: "检索阶段结果" });
  await expect(searchStageHeading).toBeVisible();
  await searchStageHeading.click();
  await expect(page.getByText("route-search-fulltext")).toBeVisible();
  await expect(page.getByText("ROUTE_SEARCH · FULLTEXT_SQLITE_FTS5_SIMPLE_V1")).toBeVisible();
  await expect(page.getByText("已完成 · 5ms")).toBeVisible();
  await expect(page.getByText("query.effective")).toBeVisible();
  await expect(page.getByText("hits.route")).toHaveCount(2);
  await expect(page.getByText("context-pack")).toBeVisible();
  await expect(page.getByText("CONTEXT_PACK · DOC_QA_CONTEXT_PACK")).toBeVisible();
  await expect(page.getByText("COMPLETED_WITH_WARNINGS · 7ms")).toBeVisible();
  await expect(page.getByText("context.pack")).toBeVisible();
  await expect(page.getByText("告警：rerank disabled; using lexical order")).toBeVisible();

  await page.getByLabel("查询", { exact: true }).fill("legacy fallback search without stage results");
  await page.getByRole("button", { name: "执行检索" }).click();
  await expect(page.getByText("检索返回 1 条命中。").first()).toBeVisible();
  await expect(page.getByText("RAG Storage Runtime Workbench").first()).toBeVisible();
  await expect(page.getByRole("heading", { name: "检索阶段结果" })).toHaveCount(0);

  const ingestBodies = requestBodies(requests, "/api/v1/rag/ingest", "POST");
  expect(ingestBodies).toEqual(expect.arrayContaining([
    expect.objectContaining({
      accessContext: expect.objectContaining({
        agentCode: "AGENT_CODEX",
        userCode: "USR-AUTH-0001",
        workspaceCode: "ENTERPRISE",
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
        scopeCode: "ENTERPRISE",
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
        workspaceCode: "ENTERPRISE",
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
