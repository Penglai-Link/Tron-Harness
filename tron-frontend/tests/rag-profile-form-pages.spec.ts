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
  await page.locator("summary").filter({ hasText: /Advanced node controls|高级节点控制/i }).click();
}

function routeCorpusLabel(index: number) {
  return new RegExp(`Route ${index} corpus|路由 ${index} 语料库`, "i");
}

function routeStrategyLabel(index: number) {
  return new RegExp(`Route ${index} retrieval strategy|路由 ${index} 检索策略`, "i");
}

const label = {
  contextStrategy: /Context strategy|上下文策略/i,
  description: /Description|描述/i,
  headingDepth: /Heading depth|标题深度/i,
  ingestProfile: /Ingest profile|写入配置/i,
  limit: /Limit|限制数量|返回数量/i,
  maxTokens: /Max tokens|最大 Token 数/i,
  overlapTokens: /Overlap tokens|重叠 Token 数/i,
  primaryCorpus: /Primary corpus|主语料库/i,
  profileId: /Profile ID|配置(?: ID|标识)/i,
  query: /^Query$|^查询$/i,
  retrievalProfile: /Retrieval profile|检索配置/i,
  splitStrategy: /SPLIT_TEXT (strategy ID|策略标识)/,
  splitterType: /Splitter type|切分器类型/i,
  supportedSourceTypes: /Supported source types|支持(?:的)?来源类型/i,
  tokenBudget: /Token budget|Token 预算|令牌预算/i,
  topicPolicy: /Topic policy|主题策略/i,
  treeSummaryModelCode: /Tree summary model code|结构树摘要模型编码/,
  vectorStrategy: /(^|[^_])VECTOR_INDEX (strategy ID|策略标识)$/,
};

const buttonName = {
  addIngestProfile: /(?:Add|New) ingest profile|新增写入配置|新建写入配置/i,
  addFilter: /Add filter|添加筛选条件/i,
  addRetrievalProfile: /(?:Add|New) retrieval profile|新增检索配置|新建检索配置/i,
  addRoute: /Add route|添加路由/i,
  editProjectDocIngest: /Edit project-doc-ingest-v1|编辑 project-doc-ingest-v1/,
  editSupportNotesIngest: /Edit support-notes-ingest-v1|编辑 support-notes-ingest-v1/,
  enablePersistSource: /Enable (?:PERSIST_SOURCE|Persist source) node|启用 持久化来源 节点/i,
  enablePreparePayload: /Enable (?:PREPARE_PAYLOAD|Prepare payload) node|启用 (?:准备 payload|准备载荷) 节点/i,
  enableSplitText: /Enable (?:SPLIT_TEXT|Split text) node|启用 (?:切分文本|文本切分) 节点/i,
  enableExtractTree: /Enable (?:EXTRACT_TREE|Extract structure tree) node|启用 提取结构树 节点/i,
  enableFulltextIndex: /Enable (?:FULLTEXT_INDEX|Fulltext index) node|启用 全文索引 节点/i,
  enablePersistTree: /Enable (?:PERSIST_TREE|Persist structure tree) node|启用 持久化结构树 节点/i,
  enableSummarizeTree: /Enable (?:SUMMARIZE_TREE|Summarize structure tree) node|启用 摘要结构树 节点/i,
  enableTreeFulltextIndex: /Enable (?:TREE_FULLTEXT_INDEX|Tree fulltext index) node|启用 (?:树全文索引|结构树全文索引) 节点/i,
  enableTreeVectorIndex: /Enable (?:TREE_VECTOR_INDEX|Tree vector index) node|启用 (?:树向量索引|结构树向量索引) 节点/i,
  enableVectorIndex: /Enable (?:VECTOR_INDEX|Vector index) node|启用 向量索引 节点/i,
  runRetrieval: /Run retrieval|Run search|运行检索|执行检索/i,
  saveProfile: /Save profile|保存配置/i,
};

const headingName = {
  newRetrievalProfile: /(?:Create|New) retrieval profile|新建检索配置/i,
  ragManagement: /RAG Management|知识检索管理/i,
  retrievalFlow: /Retrieval flow|检索流程/i,
  retrievalRoutes: /Retrieval routes|检索路由/i,
};

const tabName = {
  retrievalTest: /Retrieval test|检索测试/i,
  workbench: /Workbench|调试台/i,
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
  await expect(page.getByRole("heading", { name: /Pipeline flow|流水线流向/i })).toBeVisible();
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

  await page.goto(`${frontendBaseUrl}/rag?tab=profiles&profileType=ingest`);
  await expect(page.getByRole("heading", { name: /RAG Management|知识检索管理/ })).toBeVisible();
  await expect(page.getByRole("heading", { name: /Profiles|配置档案/ })).toBeVisible();

  await page.getByRole("button", { name: buttonName.addIngestProfile }).click();
  await expect(page).toHaveURL(/\/rag\/profiles\/ingest\/new(\?.*)?$/);
  await expect(page.getByRole("heading", { name: /(?:Create|New) Ingest Profile|新建写入配置/i })).toBeVisible();
  await expect(page.getByRole("heading", { name: /Pipeline flow|流水线流向/i })).toBeVisible();
  await expect(page.getByRole("heading", { name: /Stage (?:Config|strategies)|阶段策略/i })).toBeVisible();
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
  await expect(page.getByLabel(label.splitterType)).toHaveValue("markdownHeading");
  await expect(page.getByLabel(label.maxTokens)).toHaveValue("800");
  await expect(page.getByLabel(label.overlapTokens)).toHaveValue("80");
  await expect(page.getByLabel(label.headingDepth)).toHaveValue("6");
  await splitStrategySelect.selectOption("RAG_SPLIT_PLAIN_TEXT_V1");
  await expect(page.getByLabel(label.splitterType)).toHaveValue("plainText");
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
  await expect(page.getByRole("heading", { name: /Pipeline flow|流水线流向/i })).toBeVisible();

  await page.getByLabel(label.description).fill("Ingest support notes, incident runbooks, and QA handbooks.");
  await openAdvancedNodeControls(page);
  await page.getByRole("button", { name: buttonName.enableVectorIndex }).click();
  await page.getByLabel(label.splitStrategy).selectOption("RAG_SPLIT_MARKDOWN_HEADING_V1");
  await expect(page.getByLabel(label.splitterType)).toHaveValue("markdownHeading");
  await expect(page.getByLabel(label.headingDepth)).toHaveValue("6");
  const summarizeStrategySelect = page.getByLabel(/SUMMARIZE_TREE (strategy ID|策略标识)/);
  await expect(summarizeStrategySelect).toContainText("RAG_TREE_LLM_SUMMARY_V1");
  await summarizeStrategySelect.selectOption("RAG_TREE_LLM_SUMMARY_V1");
  await page.getByLabel(label.treeSummaryModelCode).fill("summary-model-alpha");
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
        expect.objectContaining({ enabled: true, nodeId: "summarize-tree", stageType: "SUMMARIZE_TREE", strategyId: "RAG_TREE_LLM_SUMMARY_V1" }),
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
  expect(pipelineNode(updateBody, "SUMMARIZE_TREE")).toEqual(expect.objectContaining({
    config: expect.objectContaining({
      modelCode: "summary-model-alpha",
    }),
    strategyId: "RAG_TREE_LLM_SUMMARY_V1",
  }));
  expect(updateBody).not.toHaveProperty("pipelineStages");
});

test("preserves legacy split strategy when editing existing ingest profiles", async ({ page }) => {
  const requests: CapturedRequest[] = [];
  await installRagManagementApiMock(page, requests);

  await page.goto(`${frontendBaseUrl}/rag/profiles/ingest/project-doc-ingest-v1/edit`);
  await expect(page.getByRole("heading", { name: /Edit ingest profile|编辑写入配置/i })).toBeVisible();
  const splitStrategySelect = page.getByLabel(label.splitStrategy);
  await expect(splitStrategySelect).toContainText("RAG_SPLIT_PLAIN_TEXT_V1");
  await expect(splitStrategySelect).toContainText("RAG_SPLIT_MARKDOWN_HEADING_V1");
  await expect(splitStrategySelect).toContainText("RAG_SPLIT_TEXT");
  await expect(splitStrategySelect).toHaveValue("RAG_SPLIT_TEXT");
  await expect(page.getByLabel(label.splitterType)).toHaveValue("plainText");

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
  await expect(page.getByRole("heading", { name: headingName.ragManagement })).toBeVisible();
  await page.getByRole("tab", { name: tabName.retrievalTest }).click();
  await page.getByRole("textbox", { name: /^(Query|查询)$/ }).fill("How does summary tree retrieval surface metadata?");
  await page.getByRole("button", { name: buttonName.runRetrieval }).click();

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
  await expect(page.getByRole("heading", { name: headingName.newRetrievalProfile })).toBeVisible();

  await page.getByLabel(label.profileId).fill("project-doc-tree-related-v1");
  await page.getByLabel(label.primaryCorpus).selectOption("project-doc");
  await page.getByLabel(label.description).fill("Search project documentation summary tree nodes.");
  await page.getByLabel(routeCorpusLabel(1)).selectOption("project-doc");
  await page.getByLabel(routeStrategyLabel(1)).selectOption("TREE_FULLTEXT_V1");
  await page.getByRole("button", { name: buttonName.addRoute }).click();
  await page.getByLabel(routeCorpusLabel(2)).selectOption("project-doc");
  await page.getByLabel(routeStrategyLabel(2)).selectOption("TREE_VECTOR_V1");
  await page.getByRole("button", { name: buttonName.saveProfile }).click();

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
  await expect(page.getByRole("heading", { name: headingName.newRetrievalProfile })).toBeVisible();
  const supportTreeDetail = await page.evaluate(async () => {
    const response = await fetch("/api/v1/rag/corpora/support-tree-graph");
    return response.json();
  });
  expect(supportTreeDetail.data.capabilities).toEqual(expect.objectContaining({
    fulltext: true,
    tree: true,
    vector: true,
  }));

  await page.getByLabel(label.profileId).fill("support-tree-graph-related-v1");
  await page.getByLabel(label.primaryCorpus).selectOption("support-tree-graph");
  await page.getByLabel(label.description).fill("Search support summary tree graph nodes.");
  await page.getByLabel(routeCorpusLabel(1)).selectOption("support-tree-graph");
  await expect(page.getByLabel(routeStrategyLabel(1))).toContainText("TREE_FULLTEXT_V1");
  await page.getByLabel(routeStrategyLabel(1)).selectOption("TREE_FULLTEXT_V1");
  await page.getByRole("button", { name: buttonName.addRoute }).click();
  await page.getByLabel(routeCorpusLabel(2)).selectOption("support-tree-graph");
  await expect(page.getByLabel(routeStrategyLabel(2))).toContainText("TREE_VECTOR_V1");
  await page.getByLabel(routeStrategyLabel(2)).selectOption("TREE_VECTOR_V1");
  await page.getByRole("button", { name: buttonName.saveProfile }).click();

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
  await expect(page.getByRole("button", { name: buttonName.addRetrievalProfile })).toBeVisible();
  await page.getByRole("button", { name: buttonName.addRetrievalProfile }).click();

  await expect(page).toHaveURL(/\/rag\/profiles\/retrieval\/new(\?.*)?$/);
  await expect(page.getByRole("heading", { name: headingName.newRetrievalProfile })).toBeVisible();
  await expect(page.getByRole("heading", { name: headingName.retrievalRoutes })).toBeVisible();
  await expect(page.getByRole("heading", { name: headingName.retrievalFlow })).toBeVisible();
  await expect(page.getByText(/Backend payload preview|Developer preview|开发者预览/i)).toBeVisible();
  await expect(page.getByRole("heading", { name: /Verification result|Verify Result|校验结果/i })).toBeVisible();

  await page.getByLabel(label.profileId).fill("support-fusion-v1");
  await page.getByLabel(/Status|状态/).selectOption("Ready");
  await page.getByLabel(label.primaryCorpus).selectOption("memory-card");
  await page.getByLabel(label.description).fill("Fuse memory and project documentation support answers.");
  await page.getByLabel(routeCorpusLabel(1)).selectOption("memory-card");
  await page.getByLabel(routeStrategyLabel(1)).selectOption("FULLTEXT_SQLITE_FTS5_SIMPLE_V1");
  await page.getByRole("button", { name: buttonName.addRoute }).click();
  await page.getByLabel(routeCorpusLabel(2)).selectOption("project-doc");
  await page.getByLabel(routeStrategyLabel(2)).selectOption("VECTOR_SQLITE_VEC0_COSINE_V1");
  await page.getByRole("button", { name: buttonName.addRoute }).click();
  await page.getByLabel(routeCorpusLabel(3)).selectOption("project-doc");
  await page.getByLabel(routeStrategyLabel(3)).selectOption("TREE_FULLTEXT_V1");
  await page.getByRole("button", { name: buttonName.addRoute }).click();
  await page.getByLabel(routeCorpusLabel(4)).selectOption("project-doc");
  await page.getByLabel(routeStrategyLabel(4)).selectOption("TREE_VECTOR_V1");
  await page.getByRole("button", { name: buttonName.addFilter }).click();
  await page.getByLabel(/Filter 1 field|筛选条件 1 字段/).selectOption("scopeCode");
  await page.getByLabel(/Filter 1 operator|筛选条件 1 操作符/).selectOption("eq");
  await page.getByLabel(/Filter 1 value|筛选条件 1 值/).fill("TRON_CORE");
  await page.getByRole("button", { name: buttonName.saveProfile }).click();

  await expect(page).toHaveURL(/\/rag\?tab=profiles&profileType=retrieval$/);
  await expect(notifications(page, /support-fusion-v1 (created|已创建)|检索配置已创建/)).toBeVisible();
  await expect(page.getByRole("button", { name: /Edit support-fusion-v1|编辑 support-fusion-v1/ })).toBeVisible();

  await page.getByRole("button", { name: /Edit support-fusion-v1|编辑 support-fusion-v1/ }).click();
  await expect(page).toHaveURL(/\/rag\/profiles\/retrieval\/support-fusion-v1\/edit(\?.*)?$/);
  await expect(page.getByLabel(routeCorpusLabel(1))).toHaveValue("memory-card");
  await expect(page.getByLabel(routeCorpusLabel(2))).toHaveValue("project-doc");
  await expect(page.getByLabel(/Filter 1 value|筛选条件 1 值/)).toHaveValue("TRON_CORE");

  await page.getByLabel(label.description).fill("Fuse memory, documentation, and curated support signals.");
  await page.getByLabel(label.topicPolicy).selectOption("EMBEDDING_MMR_V1");
  await page.getByRole("button", { name: buttonName.saveProfile }).click();

  await expect(page).toHaveURL(/\/rag\?tab=profiles&profileType=retrieval$/);
  await expect(notifications(page, /support-fusion-v1 (saved|已保存)|配置已更新/)).toBeVisible();
  await page.getByRole("tab", { name: tabName.workbench }).click();
  await page.getByRole("tab", { name: tabName.retrievalTest }).click();
  await expect(page.getByRole("combobox", { name: label.retrievalProfile })).toContainText("support-fusion-v1");

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
  await expect(page.getByRole("heading", { name: headingName.ragManagement })).toBeVisible();
  await expect(page.getByRole("heading", { name: /Ingest request|写入请求/i })).toBeVisible();
  await expect(page.getByRole("tab", { name: tabName.retrievalTest })).toBeVisible();

  await expect(page.getByLabel("Corpus", { exact: true })).toContainText("memory-card");
  await expect(page.getByLabel("Ingest profile", { exact: true })).toContainText("project-doc-ingest-v1");

  await page.getByLabel("Corpus", { exact: true }).selectOption("project-doc");
  await page.getByLabel("Ingest profile", { exact: true }).selectOption("project-doc-ingest-v1");
  await expect(page.getByLabel("Scope type", { exact: true })).toHaveValue("SPACE");
  await page.getByLabel("Source code", { exact: true }).fill("docs/features/rag-storage-runtime-workbench");
  await page.getByLabel("Source type", { exact: true }).selectOption("MARKDOWN");
  await expect(page.getByLabel("Source version", { exact: true })).toHaveValue("v1");
  await page.getByLabel("Language", { exact: true }).fill("en-US");
  await page.getByLabel("Metadata source path", { exact: true }).fill("docs/features/rag-storage-runtime-workbench");
  await page.getByRole("button", { name: "Add business attribute" }).click();
  await page.getByLabel("Business attribute 1 field", { exact: true }).selectOption("documentCode");
  await page.getByLabel("Business attribute 1 value", { exact: true }).fill("rag-storage-runtime-workbench");
  await page.getByRole("button", { name: "Add business attribute" }).click();
  await page.getByLabel("Business attribute 2 field", { exact: true }).selectOption("repoCode");
  await page.getByLabel("Business attribute 2 value", { exact: true }).fill("tron");
  await page.getByLabel("Source text", { exact: true }).fill("Workbench ingest verifies backend runtime writes from the frontend.");
  await page.getByRole("button", { name: "Submit ingest" }).click();

  await expect(page.getByText("docs/features/rag-storage-runtime-workbench ingest completed.").first()).toBeVisible();
  const ingestResultPanel = page.locator("div").filter({ hasText: /^Ingest result/ }).first();
  await expect(ingestResultPanel.getByText("Text units")).toBeVisible();
  await expect(ingestResultPanel).toContainText(/Text units\s*\d+/);
  const ingestStageHeading = page.getByRole("heading", { name: "Ingest stage results" });
  await expect(ingestStageHeading).toBeVisible();
  await ingestStageHeading.click();
  await expect(page.getByText("prepare-payload")).toBeVisible();
  await expect(page.getByText("PREPARE_PAYLOAD")).toBeVisible();
  await expect(page.getByText("RAG_TEXT_CONTRACT_PAYLOAD_V1")).toBeVisible();
  await expect(page.getByText("Completed · 2ms")).toBeVisible();
  await expect(page.getByText("request.payload")).toBeVisible();
  await expect(page.getByText("payload.prepared, source.metadata")).toBeVisible();
  await expect(page.getByText("split-text")).toBeVisible();
  await expect(page.getByText("SPLIT_TEXT · RAG_SPLIT_TEXT")).toBeVisible();
  await expect(page.getByText("Completed · 3ms")).toBeVisible();
  await expect(page.getByText("payload.prepared").first()).toBeVisible();
  await expect(page.getByText("textUnits.generated")).toBeVisible();
  await expect(page.getByText("Warnings: splitter used default overlap")).toBeVisible();

  await page.getByRole("tab", { name: tabName.retrievalTest }).click();
  await page.getByRole("combobox", { name: label.retrievalProfile }).selectOption("project-doc-related-v1");
  await page.getByLabel(label.query).fill("How does the runtime workbench call backend search?");
  await page.getByLabel(label.contextStrategy).selectOption("DOC_QA_CONTEXT_PACK");
  await page.getByRole("button", { name: "Add return field" }).click();
  await page.getByLabel("Return field 1", { exact: true }).selectOption("documentCode");
  await page.getByRole("button", { name: "Add return field" }).click();
  await page.getByLabel("Return field 2", { exact: true }).selectOption("repoCode");
  await page.getByRole("button", { name: buttonName.addFilter }).click();
  await page.getByLabel("Filter 1 field", { exact: true }).selectOption("scopeCode");
  await page.getByLabel("Filter 1 operator", { exact: true }).selectOption("eq");
  await page.getByLabel("Filter 1 value", { exact: true }).fill("TRON_CORE");
  await page.getByLabel(label.limit).fill("5");
  await page.getByLabel(label.tokenBudget).fill("2048");
  await page.getByRole("button", { name: "Enable tracing" }).click();
  await expect(page.getByRole("button", { name: "Enable tracing" })).toHaveAttribute("aria-pressed", "false");
  await page.getByRole("button", { name: "Enable tracing" }).click();
  await expect(page.getByRole("button", { name: "Enable tracing" })).toHaveAttribute("aria-pressed", "true");
  await page.getByRole("button", { name: buttonName.runRetrieval }).click();

  await expect(page.getByText("Retrieval returned 1 hits.").first()).toBeVisible();
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
  const searchStageHeading = page.getByRole("heading", { name: "Retrieval stage results" });
  await expect(searchStageHeading).toBeVisible();
  await searchStageHeading.click();
  await expect(page.getByText("route-search-fulltext")).toBeVisible();
  await expect(page.getByText("ROUTE_SEARCH · FULLTEXT_SQLITE_FTS5_SIMPLE_V1")).toBeVisible();
  await expect(page.getByText("Completed · 5ms")).toBeVisible();
  await expect(page.getByText("query.effective")).toBeVisible();
  await expect(page.getByText("hits.route")).toHaveCount(2);
  await expect(page.getByText("context-pack")).toBeVisible();
  await expect(page.getByText("CONTEXT_PACK · DOC_QA_CONTEXT_PACK")).toBeVisible();
  await expect(page.getByText("COMPLETED_WITH_WARNINGS · 7ms")).toBeVisible();
  await expect(page.getByText("context.pack")).toBeVisible();
  await expect(page.getByText("Warnings: rerank disabled; using lexical order")).toBeVisible();

  await page.getByLabel(label.query).fill("legacy fallback search without stage results");
  await page.getByRole("button", { name: buttonName.runRetrieval }).click();
  await expect(page.getByText("Retrieval returned 1 hits.").first()).toBeVisible();
  await expect(page.getByText("RAG Storage Runtime Workbench").first()).toBeVisible();
  await expect(page.getByRole("heading", { name: "Retrieval stage results" })).toHaveCount(0);

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
