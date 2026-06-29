import {
  expect,
  test,
  type Locator,
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

async function chooseSelectOption(page: Page, control: Locator, value: string) {
  await control.click();
  await expect(control).toHaveAttribute("aria-expanded", "true");
  await page.getByRole("option", { name: value, exact: true }).click();
  await expect(control).toHaveAttribute("aria-expanded", "false");
  await expect(control).toContainText(value);
}

async function expectSelectedOption(control: Locator, value: string) {
  await expect(control).toContainText(value);
}

async function closeSelect(page: Page, control: Locator) {
  await page.keyboard.press("Escape");
  await expect(control).toHaveAttribute("aria-expanded", "false");
}

async function expectSelectOptionAvailable(page: Page, control: Locator, value: string) {
  await control.click();
  await expect(control).toHaveAttribute("aria-expanded", "true");
  await expect(page.getByRole("option", { name: value, exact: true })).toBeVisible();
  await closeSelect(page, control);
}

async function expectSelectOptionUnavailable(page: Page, control: Locator, name: string | RegExp) {
  await control.click();
  await expect(control).toHaveAttribute("aria-expanded", "true");
  const option = typeof name === "string"
    ? page.getByRole("option", { name, exact: true })
    : page.getByRole("option", { name });
  await expect(option).toHaveCount(0);
  await closeSelect(page, control);
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
  const controls = page.getByTestId("advanced-node-controls");
  await expect(controls).toBeVisible();
  if ((await controls.getAttribute("open")) === null) await controls.locator("summary").click();
  await expect(controls).toHaveAttribute("open", "");
}

async function openAdvancedNodeConfig(page: Page, stageId: string) {
  await openAdvancedNodeControls(page);
  const nodeCard = page.getByTestId(`advanced-node-${stageId}`);
  const trigger = page.getByTestId(`advanced-node-trigger-${stageId}`);
  if (await trigger.isDisabled()) {
    const toggle = nodeCard.locator("button[aria-pressed]").first();
    await toggle.click();
  }
  if (await trigger.getAttribute("aria-expanded") !== "true") await trigger.click();
  await expect(page.getByTestId(`advanced-node-config-${stageId}`)).toBeVisible();
}

async function expectAdvancedNodeConfigClosed(page: Page, stageId: string) {
  await expect(page.getByTestId(`advanced-node-config-${stageId}`)).toHaveCount(0);
}

async function expectNoStageStrategyPlaceholder(page: Page, stageId: string, labelPattern: RegExp) {
  await openAdvancedNodeConfig(page, stageId);
  await expectSelectOptionUnavailable(page, page.getByLabel(labelPattern), /Select stage strategy|选择阶段策略/i);
}

async function openRetrievalAdvancedSettings(page: Page) {
  const advanced = page.getByTestId("retrieval-advanced-settings");
  await expect(advanced).toBeVisible();
  if ((await advanced.getAttribute("open")) === null) await advanced.locator("summary").click();
  await expect(advanced).toHaveAttribute("open", "");
}

async function openRetrievalFilters(page: Page) {
  const filters = page.getByTestId("retrieval-filters");
  await expect(filters).toBeVisible();
  if ((await filters.getAttribute("open")) === null) await filters.locator("summary").click();
  await expect(filters).toHaveAttribute("open", "");
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
  embeddingVersion: /Embedding version|嵌入版本/i,
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

test("pipeline flow draws connected edges for serial and parallel branches", async ({ page }) => {
  const requests: CapturedRequest[] = [];
  await installRagManagementApiMock(page, requests);

  await page.goto(`${frontendBaseUrl}/rag/profiles/ingest/new?returnProfileType=ingest`);
  const templateHeading = page.getByRole("heading", { name: /Pipeline templates|流水线模板/i });
  await expect(templateHeading).toBeVisible();
  await expect(page.getByRole("heading", { name: /Write pipeline|写入流水线/i })).toHaveCount(0);
  await expect(page.getByRole("button", { name: /Advanced pipeline settings|高级流水线设置/i })).toHaveCount(0);
  await expect(page.getByText(/Pipeline flow|流水线流向/i)).toBeVisible();
  await chooseSelectOption(page, page.getByLabel(label.primaryCorpus), "project-doc");
  await expect(page.getByTestId("advanced-node-controls")).not.toHaveAttribute("open", "");

  const flow = page.getByTestId("pipeline-flow");
  await expect(flow.locator("svg")).toBeHidden();
  await flow.getByTestId("pipeline-flow-summary").click();
  await expect(flow.locator("svg")).toBeVisible();
  await expect(flow).not.toContainText("Level 1");
  await expect(flow).not.toContainText("Parallel");
  const fulltextNode = flow.getByTestId("pipeline-flow-node-fulltext-index");
  const fulltextNodeGroup = flow.getByTestId("pipeline-flow-node-group-fulltext-index");
  await expect(fulltextNode).toContainText("Fulltext index");
  await expect(fulltextNode).not.toContainText("RAG_FULLTEXT_INDEX");
  await expect(fulltextNode).toHaveAttribute("title", /Strategy: RAG_FULLTEXT_INDEX/);
  await expect(fulltextNodeGroup).toHaveAttribute("aria-label", /Strategy: RAG_FULLTEXT_INDEX/);
  await expect(flow.getByTestId("pipeline-flow-edge-split-text-extract-tree")).toHaveCount(1);
  await expect(flow.getByTestId("pipeline-flow-edge-split-text-fulltext-index")).toHaveCount(1);
  await expect(flow.getByTestId("pipeline-flow-edge-fulltext-index-persist-source")).toHaveCount(1);
  await expect(fulltextNode).not.toContainText("summarize-tree");
  await expect(flow.getByTestId("pipeline-flow-edge-vector-index-persist-source")).toHaveCount(1);
  await expect(flow.getByTestId("pipeline-flow-edge-extract-tree-summarize-tree")).toHaveCount(1);

  await openAdvancedNodeControls(page);
  const vectorToggle = page.getByRole("button", { name: buttonName.enableVectorIndex });
  const vectorTrigger = page.getByTestId("advanced-node-trigger-vectorIndex");
  await expect(vectorToggle).toHaveAttribute("aria-pressed", "true");
  await vectorTrigger.click();
  await page.getByLabel(label.embeddingVersion).fill("custom-v2");
  await expect(page.getByLabel(label.embeddingVersion)).toHaveValue("custom-v2");
  await vectorToggle.click();
  await expect(vectorToggle).toHaveAttribute("aria-pressed", "false");
  await expect(vectorTrigger).toBeDisabled();
  await expect(vectorTrigger).toContainText(/Enable first|先启用/i);
  await expectAdvancedNodeConfigClosed(page, "vectorIndex");
  await vectorToggle.click();
  await expect(vectorToggle).toHaveAttribute("aria-pressed", "true");
  await expect(vectorTrigger).toBeEnabled();
  await expect(page.getByTestId("advanced-node-config-vectorIndex")).toBeVisible();
  await expect(page.getByLabel(label.embeddingVersion)).toHaveValue("v1");
  await page.getByTestId("advanced-node-trigger-fulltextIndex").click();
  await expect(page.getByTestId("advanced-node-config-fulltextIndex")).toBeVisible();
  await expectAdvancedNodeConfigClosed(page, "vectorIndex");
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
  await expect(page.getByRole("heading", { name: /Write pipeline|写入流水线/i })).toHaveCount(0);
  await expect(page.getByRole("heading", { name: /Pipeline templates|流水线模板/i })).toBeVisible();
  await expect(page.getByText(/Pipeline flow|流水线流向/i)).toBeVisible();
  await expect(page.getByRole("button", { name: /Advanced pipeline settings|高级流水线设置/i })).toHaveCount(0);
  await expect(page.getByTestId("advanced-node-controls")).not.toHaveAttribute("open", "");
  await expect(page.getByRole("heading", { name: /Stage (?:Config|strategies)|阶段策略/i })).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "Stage Bindings" })).toHaveCount(0);

  await page.getByLabel(label.profileId).fill("support-notes-ingest-v1");
  await chooseSelectOption(page, page.getByLabel(label.primaryCorpus), "project-doc");
  await page.getByLabel(label.description).fill("Ingest support notes and incident runbooks.");
  await expectNoStageStrategyPlaceholder(page, "validateSource", /VALIDATE_SOURCE (strategy ID|策略标识)/);
  await expectNoStageStrategyPlaceholder(page, "preparePayload", /PREPARE_PAYLOAD (strategy ID|策略标识)/);
  await chooseSelectOption(page, page.getByLabel(/PREPARE_PAYLOAD (strategy ID|策略标识)/), "RAG_TEXT_CONTRACT_PAYLOAD_V1");
  await expectNoStageStrategyPlaceholder(page, "resolveSource", /RESOLVE_SOURCE (strategy ID|策略标识)/);
  await openAdvancedNodeConfig(page, "splitText");
  await expectAdvancedNodeConfigClosed(page, "preparePayload");
  const splitStrategySelect = page.getByLabel(label.splitStrategy);
  await expectSelectOptionUnavailable(page, splitStrategySelect, /Select stage strategy|选择阶段策略/i);
  await expectSelectOptionAvailable(page, splitStrategySelect, "RAG_SPLIT_PLAIN_TEXT_V1");
  await expectSelectOptionAvailable(page, splitStrategySelect, "RAG_SPLIT_MARKDOWN_HEADING_V1");
  await expectSelectOptionAvailable(page, splitStrategySelect, "RAG_SPLIT_TEXT");
  await chooseSelectOption(page, splitStrategySelect, "RAG_SPLIT_MARKDOWN_HEADING_V1");
  await expectSelectedOption(page.getByLabel(label.splitterType), "markdownHeading");
  await expect(page.getByLabel(label.maxTokens)).toHaveValue("800");
  await expect(page.getByLabel(label.overlapTokens)).toHaveValue("80");
  await expectSelectedOption(page.getByLabel(label.headingDepth), "6");
  await chooseSelectOption(page, splitStrategySelect, "RAG_SPLIT_PLAIN_TEXT_V1");
  await expectSelectedOption(page.getByLabel(label.splitterType), "plainText");
  await expectNoStageStrategyPlaceholder(page, "parseDocument", /PARSE_DOCUMENT (strategy ID|策略标识)/);
  await expectNoStageStrategyPlaceholder(page, "fulltextIndex", /(^|[^_])FULLTEXT_INDEX (strategy ID|策略标识)/);
  await expectNoStageStrategyPlaceholder(page, "vectorIndex", label.vectorStrategy);
  await openAdvancedNodeConfig(page, "extractTree");
  await expectSelectOptionUnavailable(page, page.getByLabel(/EXTRACT_TREE (strategy ID|策略标识)/), /Select stage strategy|选择阶段策略/i);
  await chooseSelectOption(page, page.getByLabel(/EXTRACT_TREE (strategy ID|策略标识)/), "RAG_MARKDOWN_TREE_EXTRACT_V1");
  await openAdvancedNodeConfig(page, "summarizeTree");
  await expectSelectOptionUnavailable(page, page.getByLabel(/SUMMARIZE_TREE (strategy ID|策略标识)/), /Select stage strategy|选择阶段策略/i);
  await chooseSelectOption(page, page.getByLabel(/SUMMARIZE_TREE (strategy ID|策略标识)/), "RAG_TREE_LLM_SUMMARY_V1");
  await openAdvancedNodeConfig(page, "treeFulltextIndex");
  await expectSelectOptionUnavailable(page, page.getByLabel(/TREE_FULLTEXT_INDEX (strategy ID|策略标识)/), /Select stage strategy|选择阶段策略/i);
  await chooseSelectOption(page, page.getByLabel(/TREE_FULLTEXT_INDEX (strategy ID|策略标识)/), "RAG_TREE_FULLTEXT_INDEX");
  await openAdvancedNodeConfig(page, "treeVectorIndex");
  await expectSelectOptionUnavailable(page, page.getByLabel(/TREE_VECTOR_INDEX (strategy ID|策略标识)/), /Select stage strategy|选择阶段策略/i);
  await chooseSelectOption(page, page.getByLabel(/TREE_VECTOR_INDEX (strategy ID|策略标识)/), "RAG_TREE_VECTOR_INDEX");
  await openAdvancedNodeConfig(page, "persistTree");
  await expectSelectOptionUnavailable(page, page.getByLabel(/PERSIST_TREE (strategy ID|策略标识)/), /Select stage strategy|选择阶段策略/i);
  await chooseSelectOption(page, page.getByLabel(/PERSIST_TREE (strategy ID|策略标识)/), "RAG_PERSIST_TREE_GRAPH_V1");
  await openAdvancedNodeConfig(page, "persistSource");
  await expectSelectOptionUnavailable(page, page.getByLabel(/PERSIST_SOURCE (strategy ID|策略标识)/), /Select stage strategy|选择阶段策略/i);
  await chooseSelectOption(page, page.getByLabel(/PERSIST_SOURCE (strategy ID|策略标识)/), "RAG_PERSIST_SOURCE");
  await page.getByLabel(label.supportedSourceTypes).fill("RUNBOOK, INCIDENT_NOTE");
  await page.getByRole("button", { name: buttonName.saveProfile }).click();

  await expect(page).toHaveURL(/\/rag\?tab=profiles(&profileType=ingest)?$/);
  await expect(notifications(page, /support-notes-ingest-v1 (created|已创建)/)).toBeVisible();
  await expect(page.getByRole("button", { name: buttonName.editSupportNotesIngest })).toBeVisible();

  await page.getByRole("button", { name: buttonName.editSupportNotesIngest }).click();
  await expect(page).toHaveURL(/\/rag\/profiles\/ingest\/support-notes-ingest-v1\/edit(\?.*)?$/);
  await expect(page.getByLabel(label.profileId)).toBeDisabled();
  await expect(page.getByLabel(label.description)).toHaveValue("Ingest support notes and incident runbooks.");
  await expect(page.getByRole("heading", { name: /Write pipeline|写入流水线/i })).toHaveCount(0);
  await expect(page.getByText(/Pipeline flow|流水线流向/i)).toBeVisible();

  await page.getByLabel(label.description).fill("Ingest support notes, incident runbooks, and QA handbooks.");
  await openAdvancedNodeConfig(page, "splitText");
  await chooseSelectOption(page, page.getByLabel(label.splitStrategy), "RAG_SPLIT_MARKDOWN_HEADING_V1");
  await expectSelectedOption(page.getByLabel(label.splitterType), "markdownHeading");
  await expectSelectedOption(page.getByLabel(label.headingDepth), "6");
  await openAdvancedNodeConfig(page, "summarizeTree");
  const summarizeStrategySelect = page.getByLabel(/SUMMARIZE_TREE (strategy ID|策略标识)/);
  await expectSelectOptionAvailable(page, summarizeStrategySelect, "RAG_TREE_LLM_SUMMARY_V1");
  await chooseSelectOption(page, summarizeStrategySelect, "RAG_TREE_LLM_SUMMARY_V1");
  await openAdvancedNodeConfig(page, "vectorIndex");
  await chooseSelectOption(page, page.getByLabel(label.vectorStrategy), "RAG_VECTOR_INDEX");
  await page.getByRole("button", { name: buttonName.saveProfile }).click();

  await expect(page).toHaveURL(/\/rag\?tab=profiles(&profileType=ingest)?$/);
  await expect(notifications(page, /support-notes-ingest-v1 (saved|已保存)/)).toBeVisible();
  await page.getByRole("tab", { name: /Workbench|调试台/ }).click();
  await expectSelectOptionAvailable(page, page.getByLabel(label.ingestProfile), "support-notes-ingest-v1");

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
          expect.objectContaining({ nodeId: "summarize-tree", stageType: "SUMMARIZE_TREE", strategyId: "RAG_TREE_LLM_SUMMARY_V1" }),
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
        expect.objectContaining({ enabled: true, nodeId: "summarize-tree", stageType: "SUMMARIZE_TREE", strategyId: "RAG_TREE_LLM_SUMMARY_V1" }),
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
  const createSummaryNode = pipelineNode(createBody, "SUMMARIZE_TREE");
  expect(createSummaryNode).toEqual(expect.objectContaining({
    config: expect.any(Object),
    strategyId: "RAG_TREE_LLM_SUMMARY_V1",
  }));
  expect(createSummaryNode?.config as Record<string, unknown>).not.toHaveProperty("modelCode");
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
    config: expect.any(Object),
    strategyId: "RAG_TREE_LLM_SUMMARY_V1",
  }));
  expect(pipelineNode(updateBody, "SUMMARIZE_TREE")?.config as Record<string, unknown>).not.toHaveProperty("modelCode");
  expect(updateBody).not.toHaveProperty("pipelineStages");
});

test("preserves legacy split strategy when editing existing ingest profiles", async ({ page }) => {
  const requests: CapturedRequest[] = [];
  await installRagManagementApiMock(page, requests);

  await page.goto(`${frontendBaseUrl}/rag/profiles/ingest/project-doc-ingest-v1/edit`);
  await expect(page.getByRole("heading", { name: /Edit ingest profile|编辑写入配置/i })).toBeVisible();
  await openAdvancedNodeConfig(page, "splitText");
  const splitStrategySelect = page.getByLabel(label.splitStrategy);
  await expectSelectOptionAvailable(page, splitStrategySelect, "RAG_SPLIT_PLAIN_TEXT_V1");
  await expectSelectOptionAvailable(page, splitStrategySelect, "RAG_SPLIT_MARKDOWN_HEADING_V1");
  await expectSelectOptionAvailable(page, splitStrategySelect, "RAG_SPLIT_TEXT");
  await expectSelectedOption(splitStrategySelect, "RAG_SPLIT_TEXT");
  await expectSelectedOption(page.getByLabel(label.splitterType), "plainText");

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
  await chooseSelectOption(page, page.getByLabel(label.primaryCorpus), "project-doc");
  await page.getByLabel(label.description).fill("Search project documentation summary tree nodes.");
  await chooseSelectOption(page, page.getByLabel(routeCorpusLabel(1)), "project-doc");
  await chooseSelectOption(page, page.getByLabel(routeStrategyLabel(1)), "TREE_FULLTEXT_V1");
  await page.getByRole("button", { name: buttonName.addRoute }).click();
  await chooseSelectOption(page, page.getByLabel(routeCorpusLabel(2)), "project-doc");
  await chooseSelectOption(page, page.getByLabel(routeStrategyLabel(2)), "TREE_VECTOR_V1");
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
  await chooseSelectOption(page, page.getByLabel(label.primaryCorpus), "support-tree-graph");
  await page.getByLabel(label.description).fill("Search support summary tree graph nodes.");
  await chooseSelectOption(page, page.getByLabel(routeCorpusLabel(1)), "support-tree-graph");
  await expectSelectOptionAvailable(page, page.getByLabel(routeStrategyLabel(1)), "TREE_FULLTEXT_V1");
  await chooseSelectOption(page, page.getByLabel(routeStrategyLabel(1)), "TREE_FULLTEXT_V1");
  await page.getByRole("button", { name: buttonName.addRoute }).click();
  await chooseSelectOption(page, page.getByLabel(routeCorpusLabel(2)), "support-tree-graph");
  await expectSelectOptionAvailable(page, page.getByLabel(routeStrategyLabel(2)), "TREE_VECTOR_V1");
  await chooseSelectOption(page, page.getByLabel(routeStrategyLabel(2)), "TREE_VECTOR_V1");
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
  await expect(page.getByText(/Backend payload preview|Developer preview|开发者预览/i)).toHaveCount(0);
  await expect(page.getByTestId("retrieval-advanced-settings")).not.toHaveAttribute("open", "");
  await expect(page.getByTestId("retrieval-filters")).not.toHaveAttribute("open", "");
  await expect(page.getByRole("heading", { name: /Verification result|Verify Result|校验结果/i })).toHaveCount(0);

  await page.getByLabel(label.profileId).fill("support-fusion-v1");
  await chooseSelectOption(page, page.getByLabel(/Status|状态/), "Ready");
  await chooseSelectOption(page, page.getByLabel(label.primaryCorpus), "memory-card");
  await page.getByLabel(label.description).fill("Fuse memory and project documentation support answers.");
  await chooseSelectOption(page, page.getByLabel(routeCorpusLabel(1)), "memory-card");
  await chooseSelectOption(page, page.getByLabel(routeStrategyLabel(1)), "FULLTEXT_SQLITE_FTS5_SIMPLE_V1");
  await page.getByRole("button", { name: buttonName.addRoute }).click();
  await chooseSelectOption(page, page.getByLabel(routeCorpusLabel(2)), "project-doc");
  await chooseSelectOption(page, page.getByLabel(routeStrategyLabel(2)), "VECTOR_SQLITE_VEC0_COSINE_V1");
  await page.getByRole("button", { name: buttonName.addRoute }).click();
  await chooseSelectOption(page, page.getByLabel(routeCorpusLabel(3)), "project-doc");
  await chooseSelectOption(page, page.getByLabel(routeStrategyLabel(3)), "TREE_FULLTEXT_V1");
  await page.getByRole("button", { name: buttonName.addRoute }).click();
  await chooseSelectOption(page, page.getByLabel(routeCorpusLabel(4)), "project-doc");
  await chooseSelectOption(page, page.getByLabel(routeStrategyLabel(4)), "TREE_VECTOR_V1");
  await openRetrievalFilters(page);
  await page.getByRole("button", { name: buttonName.addFilter }).click();
  await chooseSelectOption(page, page.getByLabel(/Filter 1 field|筛选条件 1 字段/), "scopeCode");
  await chooseSelectOption(page, page.getByLabel(/Filter 1 operator|筛选条件 1 操作符/), "eq");
  await page.getByLabel(/Filter 1 value|筛选条件 1 值/).fill("TRON_CORE");
  await page.getByRole("button", { name: buttonName.saveProfile }).click();

  await expect(page).toHaveURL(/\/rag\?tab=profiles&profileType=retrieval$/);
  await expect(notifications(page, /support-fusion-v1 (created|已创建)|检索配置已创建/)).toBeVisible();
  await expect(page.getByRole("button", { name: /Edit support-fusion-v1|编辑 support-fusion-v1/ })).toBeVisible();

  await page.getByRole("button", { name: /Edit support-fusion-v1|编辑 support-fusion-v1/ }).click();
  await expect(page).toHaveURL(/\/rag\/profiles\/retrieval\/support-fusion-v1\/edit(\?.*)?$/);
  await expectSelectedOption(page.getByLabel(routeCorpusLabel(1)), "memory-card");
  await expectSelectedOption(page.getByLabel(routeCorpusLabel(2)), "project-doc");
  await openRetrievalFilters(page);
  await expect(page.getByLabel(/Filter 1 value|筛选条件 1 值/)).toHaveValue("TRON_CORE");

  await page.getByLabel(label.description).fill("Fuse memory, documentation, and curated support signals.");
  await openRetrievalAdvancedSettings(page);
  await chooseSelectOption(page, page.getByLabel(label.topicPolicy), "EMBEDDING_MMR_V1");
  await page.getByRole("button", { name: buttonName.saveProfile }).click();

  await expect(page).toHaveURL(/\/rag\?tab=profiles&profileType=retrieval$/);
  await expect(notifications(page, /support-fusion-v1 (saved|已保存)|配置已更新/)).toBeVisible();
  await page.getByRole("tab", { name: tabName.workbench }).click();
  await page.getByRole("tab", { name: tabName.retrievalTest }).click();
  await expectSelectOptionAvailable(page, page.getByRole("combobox", { name: label.retrievalProfile }), "support-fusion-v1");

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

  await chooseSelectOption(page, page.getByLabel("Corpus", { exact: true }), "project-doc");
  await chooseSelectOption(page, page.getByLabel("Ingest profile", { exact: true }), "project-doc-ingest-v1");
  await expectSelectedOption(page.getByLabel("Scope type", { exact: true }), "SPACE");
  await page.getByLabel("Source code", { exact: true }).fill("docs/features/rag-storage-runtime-workbench");
  await chooseSelectOption(page, page.getByLabel("Source type", { exact: true }), "MARKDOWN");
  await expect(page.getByLabel("Source version", { exact: true })).toHaveValue("v1");
  await page.getByLabel("Language", { exact: true }).fill("en-US");
  await page.getByLabel("Metadata source path", { exact: true }).fill("docs/features/rag-storage-runtime-workbench");
  await page.getByRole("button", { name: "Add business attribute" }).click();
  await chooseSelectOption(page, page.getByLabel("Business attribute 1 field", { exact: true }), "documentCode");
  await page.getByLabel("Business attribute 1 value", { exact: true }).fill("rag-storage-runtime-workbench");
  await page.getByRole("button", { name: "Add business attribute" }).click();
  await chooseSelectOption(page, page.getByLabel("Business attribute 2 field", { exact: true }), "repoCode");
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
  await chooseSelectOption(page, page.getByRole("combobox", { name: label.retrievalProfile }), "project-doc-related-v1");
  await page.getByLabel(label.query).fill("How does the runtime workbench call backend search?");
  await chooseSelectOption(page, page.getByLabel(label.contextStrategy), "DOC_QA_CONTEXT_PACK");
  await page.getByRole("button", { name: "Add return field" }).click();
  await chooseSelectOption(page, page.getByLabel("Return field 1", { exact: true }), "documentCode");
  await page.getByRole("button", { name: "Add return field" }).click();
  await chooseSelectOption(page, page.getByLabel("Return field 2", { exact: true }), "repoCode");
  await page.getByRole("button", { name: buttonName.addFilter }).click();
  await chooseSelectOption(page, page.getByLabel("Filter 1 field", { exact: true }), "scopeCode");
  await chooseSelectOption(page, page.getByLabel("Filter 1 operator", { exact: true }), "eq");
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
