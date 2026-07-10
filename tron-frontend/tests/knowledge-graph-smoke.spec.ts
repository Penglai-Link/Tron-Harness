import {
  expect,
  test,
  type Page,
  type Route,
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

test.describe.configure({ timeout: 60_000 });

type KnowledgeGraphFixture = ReturnType<typeof graphFixture>;

interface FrontendHarness {
  baseUrl: string;
  logs: () => string;
  stop: () => Promise<void>;
}

const workspaceSwitcher = {
  primarySpaces: [{
    childCount: 1,
    description: "Global governance and shared AI resource policy.",
    name: "Enterprise",
    status: "Active",
    workspaceCode: "ENTERPRISE",
  }],
  secondarySpaces: [
    {
      childCount: 1,
      depth: 1,
      description: "Global governance and shared AI resource policy.",
      effectiveUserCount: 1,
      label: "Enterprise",
      name: "Enterprise",
      parentWorkspaceCode: null,
      primaryWorkspaceCode: "ENTERPRISE",
      status: "Active",
      workspaceCode: "ENTERPRISE",
    },
    {
      childCount: 0,
      depth: 2,
      description: "Main Tron console workspace.",
      effectiveUserCount: 1,
      label: "Enterprise / Tron Core",
      name: "Tron Core",
      parentWorkspaceCode: "ENTERPRISE",
      primaryWorkspaceCode: "ENTERPRISE",
      status: "Active",
      workspaceCode: "TRON_CORE",
    },
  ],
};

async function seedTronCoreShellSelection(page: Page) {
  await page.addInitScript((snapshot) => {
    window.localStorage.setItem("tron-app-shell", JSON.stringify({
      state: {
        openSections: { Core: true },
        selectedPrimarySpaceId: "ENTERPRISE",
        selectedSecondarySpaceId: "TRON_CORE",
        sidebarCollapsed: false,
        workspaceSwitcherSnapshot: snapshot,
      },
      version: 4,
    }));
  }, workspaceSwitcher);
}

function delay(ms: number) {
  return new Promise((resolveDelay) => {
    setTimeout(resolveDelay, ms);
  });
}

async function reservePort() {
  return new Promise<number>((resolvePort, rejectPort) => {
    const server = createServer();

    server.on("error", rejectPort);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      if (!address || typeof address === "string") {
        server.close();
        rejectPort(new Error("Failed to reserve a frontend test port."));
        return;
      }

      const { port } = address;
      server.close(() => resolvePort(port));
    });
  });
}

async function waitForFrontend(url: string, process: ManagedProcess, logs: () => string) {
  const deadline = Date.now() + 30_000;
  let lastError: unknown;

  while (Date.now() < deadline) {
    if (process.exitCode !== null) {
      throw new Error(`Frontend dev server exited early.\n${logs()}`);
    }

    try {
      const response = await fetch(url);
      if (response.ok) return;
    } catch (error) {
      lastError = error;
    }

    await delay(250);
  }

  throw new Error(`Frontend dev server did not become ready: ${String(lastError)}\n${logs()}`);
}

async function startFrontend(env: NodeJS.ProcessEnv): Promise<FrontendHarness> {
  const port = await reservePort();
  const baseUrl = `http://127.0.0.1:${port}`;
  let frontendLogs = "";

  let frontendProcess: ManagedProcess | undefined = spawnManagedProcess(
    "npm",
    ["run", "dev", "--", "--host", "127.0.0.1", "--port", String(port)],
    {
      cwd: frontendRoot,
      env: {
        ...process.env,
        ...env,
      },
      stdio: "pipe",
    },
  );

  const appendFrontendLog = (chunk: Buffer) => {
    frontendLogs = `${frontendLogs}${chunk.toString()}`.slice(-4000);
  };
  frontendProcess.stdout.on("data", appendFrontendLog);
  frontendProcess.stderr.on("data", appendFrontendLog);

  try {
    await waitForFrontend(`${baseUrl}/knowledge-graph`, frontendProcess, () => frontendLogs);
  } catch (error) {
    await stopManagedProcess(frontendProcess);
    frontendProcess = undefined;
    throw error;
  }

  return {
    baseUrl,
    logs: () => frontendLogs,
    stop: async () => {
      try {
        await stopManagedProcess(frontendProcess);
      } finally {
        frontendProcess = undefined;
      }
    },
  };
}

function graphFixture(overrides: Partial<KnowledgeGraphFixture> = {}) {
  const scope = {
    appCode: "__NONE__",
    currentWorkspaceCode: "TRON_CORE",
    inheritanceMode: "WORKSPACE_LINEAGE",
    namedGraph: "WS-ROOT",
    namespace: "rag_skill",
    rootWorkspaceCode: "WS-ROOT",
    schema: "rag_skill",
    scopeModes: ["SPACE", "USER"],
    userCode: "U-100",
    visibleUserCodes: ["__NONE__", "U-100"],
    visibleWorkspaceCodes: ["WS-ROOT", "TRON_CORE"],
    workspaceCode: "TRON_CORE",
  };
  const baseNode = {
    createdAt: "2026-06-23T08:00:00Z",
    editable: false,
    inherited: true,
    originUserCode: null,
    originWorkspaceCode: "WS-ROOT",
    originWorkspaceName: "Root Workspace",
    scopeType: "SPACE",
    score: 0.98,
    updatedAt: "2026-06-23T08:00:00Z",
  };
  const baseEdge = {
    createdAt: "2026-06-23T08:00:00Z",
    editable: false,
    inherited: true,
    originUserCode: null,
    originWorkspaceCode: "WS-ROOT",
    originWorkspaceName: "Root Workspace",
    scopeType: "SPACE",
    updatedAt: "2026-06-23T08:00:00Z",
  };
  const nodes = [
    {
      ...baseNode,
      embeddingText: "Reviews RAG retrieval changes.",
      fullText: "Skill: rag-reviewer",
      labels: ["RagSkill"],
      nodeCode: "RAG_REVIEWER",
      properties: {
        embeddingJson: "[0.12,0.24]",
        skillName: "rag-reviewer",
        status: "ACTIVE",
        title: "RAG Reviewer",
      },
    },
    {
      ...baseNode,
      editable: true,
      inherited: false,
      labels: ["RagTool"],
      nodeCode: "GIT",
      originWorkspaceCode: "TRON_CORE",
      properties: { provider: "git", status: "ACTIVE" },
    },
    {
      ...baseNode,
      editable: true,
      inherited: false,
      labels: ["RagTag"],
      nodeCode: "USER_REVIEW_NOTE",
      originUserCode: "U-100",
      originWorkspaceCode: "TRON_CORE",
      properties: { category: "review", title: "User Review Note" },
      scopeType: "USER",
    },
    {
      ...baseNode,
      editable: true,
      inherited: false,
      labels: ["CustomNode"],
      nodeCode: "CUSTOM_LABEL_NODE_WITH_LONG_SUFFIX_1234567",
      originWorkspaceCode: "TRON_CORE",
      properties: { title: "Custom Label Node" },
    },
  ];
  const edges = [
    {
      ...baseEdge,
      edgeCode: "RAG_REVIEWER_REQUIRES_GIT",
      properties: { required: true },
      relationship: "REQUIRES_TOOL",
      sourceNodeCode: "RAG_REVIEWER",
      targetNodeCode: "GIT",
    },
    {
      ...baseEdge,
      editable: true,
      edgeCode: "RAG_REVIEWER_HAS_USER_NOTE",
      inherited: false,
      originUserCode: "U-100",
      originWorkspaceCode: "TRON_CORE",
      properties: {},
      relationship: "HAS_TAG",
      scopeType: "USER",
      sourceNodeCode: "RAG_REVIEWER",
      targetNodeCode: "USER_REVIEW_NOTE",
    },
  ];

  return {
    edges,
    facets: [
      { count: 1, label: "RagSkill", value: "RagSkill" },
      { count: 1, label: "RagTool", value: "RagTool" },
      { count: 1, label: "RagTag", value: "RagTag" },
    ],
    metrics: {
      edgeCount: 20,
      namespaceCount: 2,
      nodeCount: 12,
    },
    namespaces: ["rag_skill", "audit-graph"],
    nodes,
    paths: [],
    scope,
    window: {
      edgeTotal: 20,
      hasMore: true,
      limit: 100,
      nodeTotal: 12,
      returnedEdgeCount: edges.length,
      returnedNodeCount: nodes.length,
    },
    ...overrides,
  };
}

function apiEnvelope(data: unknown) {
  return {
    code: "0",
    data,
    message: "ok",
  };
}

async function fulfillGraph(route: Route, data: unknown) {
  await route.fulfill({
    contentType: "application/json",
    status: 200,
    body: JSON.stringify(apiEnvelope(data)),
  });
}

test.describe("Knowledge graph mock mode", () => {
  let harness: FrontendHarness;

  test.beforeAll(async () => {
    harness = await startFrontend({
      VITE_TRON_AUTH_DATA_SOURCE: "mock",
      VITE_TRON_DATA_SOURCE: "mock",
      VITE_TRON_KNOWLEDGE_GRAPH_DATA_SOURCE: "mock",
    });
  });

  test.afterAll(async () => {
    await harness.stop();
  });

  test("renders backend-shaped graph mock data and updates inspector from result selection", async ({ page }) => {
    await page.goto(`${harness.baseUrl}/knowledge-graph`);

    await expect(page.getByRole("heading", { name: "Knowledge Graph" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Explore" })).toHaveCount(0);
    await expect(page.getByRole("button", { name: "Run Traversal" })).toHaveCount(0);
    await expect(page.getByRole("button", { name: "Export", exact: true })).toHaveCount(0);
    await expect(page.getByText("Query Results")).toHaveCount(0);
    await expect(page.getByText(/\d+\s+namespaces/i)).toHaveCount(0);
    await expect(page.getByRole("heading", { name: "Node Inspector" })).toHaveCount(0);
    await expect(page.getByLabel("G6 semantic knowledge graph canvas")).toBeVisible();
    await expect(page.getByText("RagSkill").first()).toBeVisible();
    await expect(page.getByText("embeddingJson")).toHaveCount(0);

    await page.getByPlaceholder("Locate node by code or title...").fill("git");
    await expect(page.getByRole("listbox", { name: "Matching graph nodes" })).toBeVisible();
    await expect(page.getByRole("option", { name: /GIT RagTool/i })).toBeVisible();
    await page.getByRole("option", { name: /GIT RagTool/i }).click();
    await expect(page.getByRole("heading", { name: "GIT" })).toBeVisible();
    await expect(page.getByText("U-TRON-001").first()).toBeVisible();
    await expect(page.getByText("provider").first()).toBeVisible();
    await expect(page.getByText("git").first()).toBeVisible();
    await expect(page.getByText("priority").first()).toBeVisible();
  });

  test("adapts graph canvas chrome to matrix dark theme", async ({ page }) => {
    await page.addInitScript(() => {
      window.localStorage.setItem("tron.theme", "matrix-dark");
    });

    await page.goto(`${harness.baseUrl}/knowledge-graph`);

    const canvas = page.getByLabel("G6 semantic knowledge graph canvas");
    await expect(canvas).toBeVisible();
    await expect.poll(() => canvas.evaluate((element) => getComputedStyle(element).backgroundImage)).toContain("92, 255, 133");
    await expect.poll(() => canvas.evaluate((element) => getComputedStyle(element).backgroundImage)).not.toContain("255, 255, 255");

    const metrics = page.getByLabel("Graph canvas metrics");
    await expect(metrics).toBeVisible();
    await expect.poll(() => metrics.evaluate((element) => getComputedStyle(element).backgroundColor)).not.toContain("255, 255, 255");
  });
});

test.describe("Knowledge graph API mode", () => {
  let harness: FrontendHarness;

  test.beforeAll(async () => {
    harness = await startFrontend({
      VITE_TRON_AUTH_DATA_SOURCE: "mock",
      VITE_TRON_DATA_SOURCE: "mock",
      VITE_TRON_KNOWLEDGE_GRAPH_DATA_SOURCE: "api",
    });
  });

  test.afterAll(async () => {
    await harness.stop();
  });

  test("loads snapshot, queries server filters, expands explicitly, and shows scope fields", async ({ page }) => {
    await seedTronCoreShellSelection(page);

    const snapshot = graphFixture({ facets: [] });
    expect(snapshot.scope).toMatchObject({
      userCode: "U-100",
      visibleUserCodes: ["__NONE__", "U-100"],
    });
    const queryWindow = graphFixture({
      edges: [snapshot.edges[0]],
      facets: [
        { count: 1, label: "RagSkill", value: "RagSkill" },
        { count: 1, label: "RagTool", value: "RagTool" },
      ],
      nodes: [snapshot.nodes[0], snapshot.nodes[1]],
      window: {
        edgeTotal: 1,
        hasMore: false,
        limit: 100,
        nodeTotal: 2,
        returnedEdgeCount: 1,
        returnedNodeCount: 2,
      },
    });
    const expandWindow = graphFixture({
      edges: [
        snapshot.edges[0],
        {
          ...snapshot.edges[0],
          edgeCode: "RAG_REVIEWER_TO_EXPANDED",
          relationship: "LINKS",
          targetNodeCode: "EXPANDED_TOOL",
        },
      ],
      nodes: [
        snapshot.nodes[0],
        {
          ...snapshot.nodes[1],
          nodeCode: "EXPANDED_TOOL",
          properties: { provider: "expanded" },
        },
      ],
      window: {
        edgeTotal: 2,
        hasMore: false,
        limit: 100,
        nodeTotal: 2,
        returnedEdgeCount: 2,
        returnedNodeCount: 2,
      },
    });
    const queryBodies: unknown[] = [];
    const expandBodies: unknown[] = [];
    let namespaceRequested = false;
    let snapshotRequestCount = 0;

    await page.route("**/api/v1/knowledge-graph/list-namespaces?**", async (route, request) => {
      namespaceRequested = true;
      expect(new URL(request.url()).searchParams.get("workspace-code")).toBe("TRON_CORE");
      await fulfillGraph(route, ["audit-graph", "rag_skill"]);
    });
    await page.route("**/api/v1/knowledge-graph?**", async (route, request) => {
      if (request.method() !== "GET") {
        await route.fallback();
        return;
      }
      snapshotRequestCount += 1;
      expect(new URL(request.url()).searchParams.get("namespace")).toBe("rag_skill");
      expect(new URL(request.url()).searchParams.get("workspace-code")).toBe("TRON_CORE");
      await fulfillGraph(route, snapshot);
    });
    await page.route("**/api/v1/knowledge-graph/query", async (route, request) => {
      queryBodies.push(request.postDataJSON());
      await fulfillGraph(route, queryWindow);
    });
    await page.route("**/api/v1/knowledge-graph/expand", async (route, request) => {
      expandBodies.push(request.postDataJSON());
      await fulfillGraph(route, expandWindow);
    });

    await page.goto(`${harness.baseUrl}/knowledge-graph`);

    await expect(page.getByRole("heading", { name: "Knowledge Graph" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Explore" })).toHaveCount(0);
    await expect(page.getByRole("button", { name: "Run Traversal" })).toHaveCount(0);
    await expect(page.getByRole("button", { name: "Export", exact: true })).toHaveCount(0);
    await expect(page.getByText("Locate node").first()).toBeVisible();
    await expect(page.getByText("Namespace").first()).toBeVisible();
    await expect(page.getByText("Display field").first()).toBeVisible();
    await expect(page.getByText("Layout").first()).toBeVisible();
    await expect.poll(() => namespaceRequested).toBe(true);
    await expect.poll(() => snapshotRequestCount).toBe(1);
    await expect(page.getByText("Window 3 / 12 nodes")).toHaveCount(0);
    await expect(page.getByText("20 total edges")).toHaveCount(0);
    await expect(page.getByText(/\d+\s+namespaces/i)).toHaveCount(0);
    await expect(page.getByText("Query Results")).toHaveCount(0);
    await expect(page.getByRole("heading", { name: "Node Inspector" })).toHaveCount(0);
    await page.getByPlaceholder("Locate node by code or title...").fill("custom");
    await expect(page.getByRole("listbox", { name: "Matching graph nodes" })).toBeVisible();
    const longCodeResult = page.getByRole("option", { name: /Custom Label Node CUSTOM_LABEL_NODE_WITH_LONG_SUFFIX_1234567 CustomNode/i });
    await expect(longCodeResult).toBeVisible();
    await expect(longCodeResult.getByText("CUSTOM_LABEL_N...1234567")).toBeVisible();
    const longCodeLabelBox = await longCodeResult.getByText("CustomNode").boundingBox();
    const compactCodeBox = await longCodeResult.getByText("CUSTOM_LABEL_N...1234567").boundingBox();
    expect(longCodeLabelBox).not.toBeNull();
    expect(compactCodeBox).not.toBeNull();
    expect(Math.abs(
      (longCodeLabelBox!.y + longCodeLabelBox!.height / 2)
      - (compactCodeBox!.y + compactCodeBox!.height / 2),
    )).toBeLessThan(3);
    await longCodeResult.click();
    await expect(page.getByRole("heading", { name: "CUSTOM_LABEL_NODE_WITH_LONG_SUFFIX_1234567" })).toBeVisible();
    expect(queryBodies).toHaveLength(0);
    const graphNodeLabels = page.locator("[aria-label='Graph node labels']");
    await expect(graphNodeLabels).toContainText("title：RAG Reviewer");
    await expect(graphNodeLabels).toContainText("nodeCode：GIT");
    const graphLegend = page.getByLabel("Graph legend");
    await expect(graphLegend.getByText("CustomNode")).toBeVisible();
    await expect(graphLegend.getByText("Ontology")).toHaveCount(0);
    await page.getByLabel("Graph namespace").click();
    await expect(page.getByRole("option", { name: "audit-graph" })).toBeVisible();
    await page.keyboard.press("Escape");
    await page.getByLabel("Graph node label property").click();
    await expect(page.getByRole("option", { name: "title" })).toBeVisible();
    await expect(page.getByRole("option", { name: "skillName" })).toBeVisible();
    await expect(page.getByRole("option", { name: "provider" })).toBeVisible();
    await page.getByRole("option", { name: "skillName" }).click();
    await expect(graphNodeLabels).toContainText("skillName：rag-reviewer");
    await expect(graphNodeLabels).toContainText("title：User Review Note");
    await expect(graphNodeLabels).toContainText("nodeCode：GIT");
    await expect(page.getByLabel("Filter relationship")).toHaveCount(0);
    await page.getByRole("button", { name: "Toggle graph filters" }).click();
    await expect(page.getByLabel("Filter relationship")).toBeVisible();
    await expect(page.getByLabel("Filter node labels")).toBeVisible();
    await page.getByRole("button", { name: "Toggle graph filters" }).click();
    await page.getByRole("button", { name: "Refresh graph" }).click();
    await expect.poll(() => snapshotRequestCount).toBe(2);

    await page.getByPlaceholder("Locate node by code or title...").fill("git");
    await expect(page.getByRole("option", { name: /GIT RagTool/i })).toBeVisible();
    expect(queryBodies).toHaveLength(0);
    await page.getByRole("option", { name: /GIT RagTool/i }).click();
    await expect(page.getByRole("heading", { name: "GIT" })).toBeVisible();
    await page.getByLabel("Graph node label property").click();
    await expect(page.getByRole("option", { name: "category" })).toBeVisible();
    await page.keyboard.press("Escape");
    await page.getByRole("button", { name: "Toggle graph filters" }).click();
    await page.getByLabel("Filter node labels").click();
    await expect(page.getByRole("option", { name: /RagTag/i })).toBeVisible();
    await page.getByRole("option", { name: /RagTag/i }).click();
    await page.keyboard.press("Escape");
    await page.getByLabel("Filter relationship").click();
    await expect(page.getByRole("option", { name: "HAS_TAG" })).toBeVisible();
    await expect(page.getByRole("option", { name: "REQUIRES_TOOL" })).toBeVisible();
    await page.getByRole("option", { name: "HAS_TAG" }).click();
    await page.getByRole("option", { name: "REQUIRES_TOOL" }).click();
    await page.keyboard.press("Escape");
    await page.getByRole("button", { name: "Toggle graph filters" }).click();

    await expect.poll(() => queryBodies.length).toBeGreaterThan(0);
    await page.getByPlaceholder("Locate node by code or title...").fill("rag reviewer");
    await expect(page.getByRole("option", { name: /RAG Reviewer RAG_REVIEWER RagSkill/i })).toBeVisible();
    await page.getByRole("option", { name: /RAG Reviewer RAG_REVIEWER RagSkill/i }).click();
    await expect(page.getByText("scopeType").first()).toBeVisible();
    await expect(page.getByText("SPACE").first()).toBeVisible();
    await expect(page.getByText("originWorkspaceCode").first()).toBeVisible();
    await expect(page.getByText("WS-ROOT").first()).toBeVisible();
    await expect(page.getByText("inherited").first()).toBeVisible();
    await expect(page.getByText("editable").first()).toBeVisible();
    await expect(page.getByText("embeddingJson")).toHaveCount(0);
    await expect(page.getByText("[0.12,0.24]")).toHaveCount(0);

    await page.getByRole("button", { name: "Expand Node" }).click();
    await expect.poll(() => expandBodies.length).toBe(1);
    await page.getByPlaceholder("Locate node by code or title...").fill("expanded");
    await expect(page.getByRole("option", { name: /EXPANDED_TOOL RagTool/i })).toBeVisible();

    expect(queryBodies.at(-1)).toMatchObject({
      inheritanceMode: "WORKSPACE_LINEAGE",
      labels: ["RagTag"],
      namespace: "rag_skill",
      query: "",
      relationships: ["HAS_TAG", "REQUIRES_TOOL"],
      scopeModes: ["SPACE", "USER"],
      workspaceCode: "TRON_CORE",
    });
    expect(expandBodies[0]).toMatchObject({
      depth: 1,
      inheritanceMode: "WORKSPACE_LINEAGE",
      limit: 100,
      namespace: "rag_skill",
      nodeCode: "RAG_REVIEWER",
      relationships: [],
      scopeModes: ["SPACE", "USER"],
      workspaceCode: "TRON_CORE",
    });
  });
});
