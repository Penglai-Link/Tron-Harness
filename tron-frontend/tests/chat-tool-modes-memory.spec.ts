/**
 * End-to-end acceptance for the two-tool gateway (`tool_search` / `tool_execute`)
 * across async long tasks, short-term memory tools and parallel tool calls.
 *
 * Everything runs against the real acceptance backend (`make be-ui-acceptance`),
 * a local OpenAI-compatible stub model and the real frontend dev server. The
 * stub records every model request body so the spec can assert the stable
 * `tools` invariant, and it drives a small state machine so the framework gets
 * the tool results it needs to continue the run.
 */
import { expect, test } from "@playwright/test";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { createServer as createHttpServer, type Server } from "node:http";
import { createServer } from "node:net";
import { fileURLToPath } from "node:url";
import { spawnManagedProcess, stopManagedProcess, type ManagedProcess } from "./support/managed-process";
import { routeActiveProfileApi, seedResourceUiShell, startResourceUiFrontend, stopResourceUiFrontend, type ResourceUiFrontend } from "./support/resource-ui-harness";

type StubMessage = { role?: string; content?: unknown; tool_calls?: StubToolCall[]; tool_call_id?: string };
type StubToolCall = { id?: string; function?: { name?: string; arguments?: unknown } };
type ModelRequest = {
  messages: StubMessage[];
  tools: string[];
  rawToolCount: number;
  stream: boolean;
  hasGatewayTools: boolean;
};
type StubCall = { tool: string; args: Record<string, unknown>; bind?: (id: string) => void };
type Decision = { calls?: StubCall[]; text?: string };
type CapturedToolResult = { callId: string; tool: string; content: string };
/** Embedding requests the backend sent to the stub (the memory corpus vector channel). */
type EmbeddingRequest = { path: string; model: string; inputCount: number };

const RAW_TOOL_NAMES = ["tool_search", "tool_execute", "bash_exec", "bash_task_read", "bash_task_stop", "memory_save", "memory_search"];
const EXPECTED_TOOL_NAMES = ["tool_execute", "tool_search"];
const LONG_COMMAND = "sleep 12 && echo acc-long-done";
const PARALLEL_COMMANDS = ["echo acc-parallel-a", "echo acc-parallel-b"];
const memoryMarker = `ACC_MEMORY_MARKER_${Date.now()}`;
const savedMemoryText = `${memoryMarker} prefers concise updates`;
const TAGS = ["e2e-long-task", "e2e-memory", "e2e-parallel"];
/** Must match the memory-corpus `embeddingDimension` the spec configures. */
const EMBEDDING_DIMENSION = 1024;
const MEMORY_CORPUS_ID = "memory-card";

const modelRequests: ModelRequest[] = [];
/** tag -> messages of the latest turn (everything after the last user message). */
const turnSnapshots = new Map<string, StubMessage[]>();
/** tag -> tool calls the stub handed to the framework, with their results. */
const stubCalls = new Map<string, Array<{ callId: string; tool: string }>>();
/** tag -> tool results observed in the follow-up model request. */
const toolResultSnapshots = new Map<string, CapturedToolResult[]>();
/** Every embedding request the backend made against the stub, across the whole run. */
const embeddingRequests: EmbeddingRequest[] = [];

const longTaskState = { execCallId: "", readCallIds: [] as string[], reads: 0 };
const memoryState = { saveCallId: "", searchCallId: "" };
const parallelState = { aCallId: "", bCallId: "" };

let callSequence = 0;
let backend: ManagedProcess | undefined;
let frontend: ResourceUiFrontend | undefined;
let provider: Server | undefined;
let directory = "";
let api = "";

async function post(path: string, body: unknown) {
  const response = await fetch(`${api}${path}`, { method: "POST", headers: { "Content-Type": "application/json", Authorization: "Bearer profile-test-token" }, body: JSON.stringify(body) });
  const payload = await response.json();
  if (!response.ok) throw new Error(`${path}: ${JSON.stringify(payload)}`);
  return payload.data;
}

async function put(path: string, body: unknown) {
  const response = await fetch(`${api}${path}`, { method: "PUT", headers: { "Content-Type": "application/json", Authorization: "Bearer profile-test-token" }, body: JSON.stringify(body) });
  const payload = await response.json();
  if (!response.ok) throw new Error(`${path}: ${JSON.stringify(payload)}`);
  return payload.data;
}

function stringifyContent(value: unknown) {
  return typeof value === "string" ? value : JSON.stringify(value ?? "");
}

function lastUserIndex(messages: StubMessage[]) {
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    if (messages[index]?.role === "user") return index;
  }
  return -1;
}

function tagOfTurn(turn: StubMessage[]) {
  const users = turn.filter((message) => message.role === "user");
  const text = users.map((message) => stringifyContent(message.content)).join("\n");
  return TAGS.find((tag) => text.includes(tag)) ?? "";
}

function toolResultFor(turn: StubMessage[], callId: string) {
  const message = turn.find((entry) => entry.role === "tool" && entry.tool_call_id === callId);
  return message ? stringifyContent(message.content) : "";
}

function parseTaskId(text: string) {
  const match = text.match(/"taskId"\s*:\s*"([^"]+)"/);
  return match ? match[1] : "";
}

function longTaskDecision(turn: StubMessage[]): Decision {
  if (!longTaskState.execCallId) {
    return { calls: [{ tool: "bash_exec", args: { command: LONG_COMMAND }, bind: (id) => { longTaskState.execCallId = id; } }] };
  }
  const execResult = toolResultFor(turn, longTaskState.execCallId);
  const taskId = parseTaskId(execResult);
  if (!taskId) return { text: "e2e-long-task-no-task-id" };
  const lastReadId = longTaskState.readCallIds[longTaskState.readCallIds.length - 1];
  const lastReadResult = lastReadId ? toolResultFor(turn, lastReadId) : "";
  if (!lastReadResult || (!lastReadResult.includes("acc-long-done") && longTaskState.reads < 3)) {
    longTaskState.reads += 1;
    return { calls: [{ tool: "bash_task_read", args: { taskId, waitMs: 10_000 }, bind: (id) => { longTaskState.readCallIds.push(id); } }] };
  }
  return { text: "e2e-long-task-done" };
}

function memoryDecision(turn: StubMessage[]): Decision {
  if (!memoryState.saveCallId) {
    return {
      calls: [{
        tool: "memory_save",
        args: { memoryType: "USER_PREFERENCE", text: savedMemoryText, title: "Acceptance preference", scope: "USER" },
        bind: (id) => { memoryState.saveCallId = id; },
      }],
    };
  }
  if (!toolResultFor(turn, memoryState.saveCallId)) return { text: "e2e-memory-save-pending" };
  if (!memoryState.searchCallId) {
    return {
      calls: [{ tool: "memory_search", args: { query: memoryMarker, scope: "USER", limit: 10 }, bind: (id) => { memoryState.searchCallId = id; } }],
    };
  }
  if (!toolResultFor(turn, memoryState.searchCallId)) return { text: "e2e-memory-search-pending" };
  return { text: "e2e-memory-done" };
}

function parallelDecision(turn: StubMessage[]): Decision {
  if (!parallelState.aCallId) {
    return {
      calls: [
        { tool: "bash_exec", args: { command: PARALLEL_COMMANDS[0] }, bind: (id) => { parallelState.aCallId = id; } },
        { tool: "bash_exec", args: { command: PARALLEL_COMMANDS[1] }, bind: (id) => { parallelState.bCallId = id; } },
      ],
    };
  }
  const first = toolResultFor(turn, parallelState.aCallId);
  const second = toolResultFor(turn, parallelState.bCallId);
  return { text: first && second ? "e2e-parallel-done" : "e2e-parallel-pending" };
}

function decide(turn: StubMessage[], tag: string): Decision {
  if (tag === "e2e-long-task") return longTaskDecision(turn);
  if (tag === "e2e-memory") return memoryDecision(turn);
  if (tag === "e2e-parallel") return parallelDecision(turn);
  return { text: "e2e-aux-reply" };
}

function recordTurn(tag: string, turn: StubMessage[]) {
  turnSnapshots.set(tag, turn);
  const calls = stubCalls.get(tag) ?? [];
  toolResultSnapshots.set(tag, calls.map((call) => ({
    callId: call.callId,
    tool: call.tool,
    content: toolResultFor(turn, call.callId),
  })));
}

const usage = { prompt_tokens: 240, completion_tokens: 32, total_tokens: 272 };

function writeStreamText(response: import("node:http").ServerResponse, text: string) {
  response.writeHead(200, { "Content-Type": "text/event-stream" });
  response.write(`data: ${JSON.stringify({ id: "e2e", choices: [{ index: 0, delta: { role: "assistant", content: text }, finish_reason: null }] })}\n\n`);
  response.end(`data: ${JSON.stringify({ id: "e2e", choices: [{ index: 0, delta: {}, finish_reason: "stop" }], usage })}\n\ndata: [DONE]\n\n`);
}

function writeStreamToolCalls(response: import("node:http").ServerResponse, calls: Array<{ id: string; name: string; args: unknown }>) {
  response.writeHead(200, { "Content-Type": "text/event-stream" });
  calls.forEach((call, index) => {
    const delta = { role: "assistant", tool_calls: [{ index, id: call.id, type: "function", function: { name: call.name, arguments: JSON.stringify(call.args) } }] };
    response.write(`data: ${JSON.stringify({ id: "e2e", choices: [{ index: 0, delta, finish_reason: null }] })}\n\n`);
  });
  response.end(`data: ${JSON.stringify({ id: "e2e", choices: [{ index: 0, delta: {}, finish_reason: "tool_calls" }], usage })}\n\ndata: [DONE]\n\n`);
}

function writeJsonReply(response: import("node:http").ServerResponse, text: string, calls: Array<{ id: string; name: string; args: unknown }> = []) {
  response.writeHead(200, { "Content-Type": "application/json" });
  response.end(JSON.stringify({
    id: "e2e",
    choices: [{
      index: 0,
      message: calls.length
        ? { role: "assistant", content: null, tool_calls: calls.map((call) => ({ id: call.id, type: "function", function: { name: call.name, arguments: JSON.stringify(call.args) } })) }
        : { role: "assistant", content: text },
      finish_reason: calls.length ? "tool_calls" : "stop",
    }],
    usage,
  }));
}

test.beforeAll(async () => {
  test.setTimeout(240_000);
  directory = await mkdtemp(join(tmpdir(), "tron-chat-tool-modes-"));
  provider = createHttpServer(async (request, response) => {
    let body = ""; for await (const chunk of request) body += chunk;
    const payload = JSON.parse(body);
    const requestPath = request.url ?? "";
    // The memory RAG vector channel calls this stub's OpenAI-compatible
    // `/v1/embeddings` endpoint (the spec registers the stub as a platform
    // EMBEDDING model and points the built-in memory corpus at it). Embedding
    // calls carry no gateway `tools` array, so they must not enter the chat
    // request log the two-tool invariant inspects.
    if (requestPath.includes("embeddings")) {
      const inputs: unknown[] = Array.isArray(payload.input) ? payload.input : [payload.input];
      embeddingRequests.push({ path: requestPath, model: String(payload.model ?? ""), inputCount: inputs.length });
      const embedding = new Array<number>(EMBEDDING_DIMENSION).fill(0);
      embedding[0] = 1; embedding[1] = 0.25;
      response.writeHead(200, { "Content-Type": "application/json" });
      response.end(JSON.stringify({
        object: "list",
        model: payload.model ?? "acceptance-embedding",
        data: inputs.map((_, index) => ({ object: "embedding", index, embedding })),
        usage: { prompt_tokens: 8, total_tokens: 8 },
      }));
      return;
    }
    const messages: StubMessage[] = Array.isArray(payload.messages) ? payload.messages : [];
    const rawTools: unknown[] = Array.isArray(payload.tools) ? payload.tools : [];
    const toolNames = rawTools.map((tool) => (tool as { function?: { name?: string } })?.function?.name ?? "");
    const hasGatewayTools = toolNames.includes("tool_execute") && toolNames.includes("tool_search");
    const stream = Boolean(payload.stream);
    modelRequests.push({ messages, tools: toolNames, rawToolCount: rawTools.length, stream, hasGatewayTools });

    const userIndex = lastUserIndex(messages);
    const turn = userIndex >= 0 ? messages.slice(userIndex) : messages;
    const tag = hasGatewayTools ? tagOfTurn(turn) : "";
    if (tag) recordTurn(tag, turn);
    const decision = hasGatewayTools && tag ? decide(turn, tag) : { text: "e2e-aux-reply" };
    const calls = (decision.calls ?? []).map((call) => {
      callSequence += 1;
      const id = `stub-${tag || "aux"}-${callSequence}`;
      call.bind?.(id);
      if (tag) {
        const recorded = stubCalls.get(tag) ?? [];
        recorded.push({ callId: id, tool: call.tool });
        stubCalls.set(tag, recorded);
      }
      return { id, name: "tool_execute", args: { tool: call.tool, arguments: call.args } };
    });

    if (stream) {
      if (calls.length) writeStreamToolCalls(response, calls);
      else writeStreamText(response, decision.text ?? "e2e-idle");
    } else {
      writeJsonReply(response, decision.text ?? "e2e-idle", calls);
    }
  });
  await new Promise<void>((resolveReady) => provider!.listen(0, "127.0.0.1", resolveReady));
  const providerAddress = provider.address();
  if (!providerAddress || typeof providerAddress === "string") throw new Error("provider address missing");
  const port = await new Promise<number>((resolvePort, reject) => {
    const server = createServer(); server.on("error", reject); server.listen(0, "127.0.0.1", () => {
      const address = server.address(); if (!address || typeof address === "string") throw new Error("missing port");
      server.close(() => resolvePort(address.port));
    });
  });
  api = `http://127.0.0.1:${port}`;
  let logs = "";
  backend = spawnManagedProcess("make", ["be-ui-acceptance", `CARGO=${process.env.TRON_ACCEPTANCE_CARGO || "cargo"}`], {
    cwd: resolve(fileURLToPath(import.meta.url), "../../.."), env: { ...process.env, CARGO_NET_OFFLINE: process.env.CARGO_NET_OFFLINE || "true", CARGO_PROFILE_DEV_DEBUG: process.env.CARGO_PROFILE_DEV_DEBUG || "0", CARGO_INCREMENTAL: process.env.CARGO_INCREMENTAL || "0", CARGO_BUILD_JOBS: "2", TRON_ACCEPTANCE_DIRECTORY: directory, TRON_ACCEPTANCE_PORT: String(port) }, stdio: "pipe",
  });
  backend.stdout.on("data", (data) => logs += data.toString());
  backend.stderr.on("data", (data) => logs += data.toString());
  const deadline = Date.now() + 210_000;
  let ready = false;
  while (Date.now() < deadline) {
    if (backend.exitCode !== null) throw new Error(logs);
    try { ready = (await fetch(`${api}/api/v1/health`)).ok; } catch { /* backend compiling */ }
    if (ready) break;
    await new Promise((resolveDelay) => setTimeout(resolveDelay, 250));
  }
  if (!ready) throw new Error(logs);
  const agentResponse = await fetch(`${api}/api/v1/agents/workspaces/TRON_CORE/internal`, {
    headers: { Authorization: "Bearer profile-test-token" },
  });
  expect(agentResponse.ok).toBe(true);
  const agents = (await agentResponse.json()).data;
  // The acceptance workspace seeds main-agent plus its platform internal agents.
  expect(agents.map((agent: { agentCode: string }) => agent.agentCode)).toContain("main-agent");

  await post("/api/v1/admin/platform/models", {
    displayName: "Framework Acceptance", vendor: "OpenAI", protocol: "OPENAI_COMPATIBLE", modelType: "TEXT", modelCapabilities: ["TEXT_GENERATION"],
    baseUrl: `http://127.0.0.1:${providerAddress.port}/v1`, apiPath: "/chat/completions", apiKeyRef: "sk-acceptance", modelName: "acceptance-model", enabled: true,
    thinkingEnabled: false, structuredOutputEnabled: false, temperature: 0.2, maxTokens: 1024, contextWindowTokens: 128000, topP: 1, maxRetries: 0,
  });
  // Offline acceptance has no reachable Person-AI embedding provider, while the
  // seeded `memory-card` corpus pins `embeddingModelCode:
  // "PERSON_AI:ROUTE:EmbeddingAuto"` (tron-backend/migrations/0001_init.sql).
  // Register the local stub as an OpenAI-compatible EMBEDDING platform model and
  // repoint the corpus at it through the public management APIs, so the memory
  // save/search round trip really runs (and really persists) offline.
  const embeddingModel = await post("/api/v1/admin/platform/models", {
    displayName: "Framework Acceptance Embedding", vendor: "OpenAI", protocol: "OPENAI_COMPATIBLE_EMBEDDING", modelType: "EMBEDDING", modelCapabilities: ["TEXT_VECTOR"],
    baseUrl: `http://127.0.0.1:${providerAddress.port}/v1`, apiPath: "/embeddings", apiKeyRef: "sk-acceptance-embedding", modelName: "acceptance-embedding", enabled: true,
    thinkingEnabled: false, structuredOutputEnabled: false, maxRetries: 0,
  });
  const embeddingModelCode = String(embeddingModel.modelCode ?? "");
  if (!embeddingModelCode) throw new Error(`embedding model create returned no modelCode: ${JSON.stringify(embeddingModel)}`);
  await post(`/api/v1/admin/platform/models/${encodeURIComponent(embeddingModelCode)}/set-default`, {});
  const memorySettings = await put(`/api/v1/rag/corpora/${MEMORY_CORPUS_ID}/settings`, {
    fulltext: true, vector: true, graph: false,
    embeddingModelCode, embeddingDimension: EMBEDDING_DIMENSION,
    rerankModelCode: null, graphModelCode: null, imageDescriptionModelCode: null,
  });
  if (memorySettings?.embeddingModelCode !== embeddingModelCode) {
    throw new Error(`memory corpus did not accept ${embeddingModelCode}: ${JSON.stringify(memorySettings)}`);
  }
  frontend = await startResourceUiFrontend("/chats", {
    VITE_TRON_DATA_SOURCE: "mock", VITE_TRON_CHAT_DATA_SOURCE: "api", VITE_TRON_MODEL_DATA_SOURCE: "api", VITE_TRON_AGENT_DATA_SOURCE: "api",
    VITE_TRON_API_BASE_URL: "", TRON_FRONTEND_API_PROXY_TARGET: api,
  });
});

test.afterAll(async () => {
  await stopResourceUiFrontend(frontend); await stopManagedProcess(backend);
  await new Promise<void>((resolveClose) => provider ? provider.close(() => resolveClose()) : resolveClose());
  if (directory) await rm(directory, { recursive: true, force: true });
});

function resetScenarioState(tag: string) {
  modelRequests.length = 0;
  stubCalls.set(tag, []);
  turnSnapshots.delete(tag);
  toolResultSnapshots.delete(tag);
  if (tag === "e2e-long-task") { longTaskState.execCallId = ""; longTaskState.readCallIds = []; longTaskState.reads = 0; }
  if (tag === "e2e-memory") { memoryState.saveCallId = ""; memoryState.searchCallId = ""; }
  if (tag === "e2e-parallel") { parallelState.aCallId = ""; parallelState.bCallId = ""; }
}

function scenarioRequests() {
  return modelRequests.filter((request) => request.hasGatewayTools);
}

function report(label: string, value: unknown) {
  console.log(`[E2E] ${label}: ${typeof value === "string" ? value : JSON.stringify(value, null, 2)}`);
}

async function expandWorkTranscript(transcript: import("@playwright/test").Locator) {
  for (const selector of ["button.ef-chat__rail-summary[aria-expanded=\"false\"]", "button.ef-chat__rail-entry-toggle[aria-expanded=\"false\"]"]) {
    for (let guard = 0; guard < 12; guard += 1) {
      const collapsed = transcript.locator(selector);
      if (await collapsed.count() === 0) break;
      await collapsed.first().click();
    }
  }
}

/** Opens every tool card, then its Parameters panel, so payloads are on screen. */
async function expandToolPayloads(transcript: import("@playwright/test").Locator) {
  for (const selector of ["button.ef-chat__tool-toggle[aria-expanded=\"false\"]", "button.ef-chat__tool-subtoggle[aria-expanded=\"false\"]"]) {
    for (let guard = 0; guard < 24; guard += 1) {
      const collapsed = transcript.locator(selector);
      if (await collapsed.count() === 0) break;
      await collapsed.first().click();
    }
  }
}

async function openChat(page: import("@playwright/test").Page) {
  await page.setViewportSize({ width: 1440, height: 1000 });
  await seedResourceUiShell(page, "zh-CN");
  await routeActiveProfileApi(page);
  await page.goto(`${frontend!.baseUrl}/chats`);
  await page.getByRole("button", { name: "新会话", exact: true }).click();
  await page.getByRole("button", { name: "选择模型", exact: true }).click();
  await page.getByRole("menuitemradio", { name: "Framework Acceptance", exact: true }).click();
}

test("long command moves to a task id and is read back through the task tool", async ({ page }, testInfo) => {
  test.setTimeout(150_000);
  resetScenarioState("e2e-long-task");
  await openChat(page);
  const input = page.locator('[data-theme-part="chat-input"]');
  const transcript = page.getByRole("region", { name: "Chat transcript", exact: true });
  await input.fill("e2e-long-task: run the long command and read the task");
  await input.press("Enter");
  await expect(transcript).toContainText("e2e-long-task-done", { timeout: 120_000 });

  const results = toolResultSnapshots.get("e2e-long-task") ?? [];
  const execResult = results.find((entry) => entry.tool === "bash_exec")?.content ?? "";
  const reads = results.filter((entry) => entry.tool === "bash_task_read");
  const taskId = parseTaskId(execResult);
  report("long-task bash_exec result", execResult);
  report("long-task task id", taskId);
  report("long-task bash_task_read result", reads.map((entry) => entry.content));

  await expandWorkTranscript(transcript);
  await expandToolPayloads(transcript);
  const titles = await transcript.locator(".ef-chat__tool-title").allInnerTexts();
  const transcriptText = await transcript.innerText();
  report("long-task tool card titles", titles);
  report("long-task transcript text", transcriptText);
  await page.screenshot({ animations: "disabled", path: testInfo.outputPath("long-task.png") });

  // Product rule: a command still running after 10s comes back as a task id
  // with empty stdout, never as the finished synchronous output.
  // (`command`/`args` echo the request line; only `stdout` is command output.)
  expect(taskId).toMatch(/^cmd-/);
  expect(JSON.parse(execResult).status).toBe("running");
  expect(JSON.parse(execResult).stdout).toBe("");
  expect(JSON.parse(execResult).success).toBeNull();
  expect(reads.length).toBeGreaterThan(0);
  expect(reads[reads.length - 1]!.content).toContain("acc-long-done");
  expect(JSON.parse(reads[reads.length - 1]!.content).stdout).toContain("acc-long-done");

  // The work rail paints a card as each tool finishes, so poll the live DOM
  // instead of the earlier snapshot: reading titles once can race the renderer.
  const liveTitles = async () => (await transcript.locator(".ef-chat__tool-title").allInnerTexts()).join("\n");
  await expect.poll(liveTitles, { timeout: 20_000 }).toContain("sleep 12");
  await expect.poll(liveTitles, { timeout: 20_000 }).toContain(taskId);
  const settled = await transcript.locator(".ef-chat__tool-title").allInnerTexts();
  expect(settled.some((title) => title.includes("执行") && title.includes("sleep 12"))).toBe(true);
  expect(settled.some((title) => title.includes("读取") && title.includes(taskId))).toBe(true);
  for (const rawName of RAW_TOOL_NAMES) {
    expect(transcriptText, `transcript must not contain raw tool name ${rawName}`).not.toContain(rawName);
  }
  assertGatewayToolsInvariant();
});

test("short-term memory save and search render as human tool cards", async ({ page }, testInfo) => {
  test.setTimeout(120_000);
  resetScenarioState("e2e-memory");
  await openChat(page);
  const input = page.locator('[data-theme-part="chat-input"]');
  const transcript = page.getByRole("region", { name: "Chat transcript", exact: true });
  await input.fill("e2e-memory: save then search the acceptance marker");
  await input.press("Enter");
  await expect(transcript).toContainText("e2e-memory-done", { timeout: 90_000 });

  const results = toolResultSnapshots.get("e2e-memory") ?? [];
  const saveResult = results.find((entry) => entry.tool === "memory_save")?.content ?? "";
  const searchResult = results.find((entry) => entry.tool === "memory_search")?.content ?? "";
  report("memory_save result", saveResult);
  report("memory_search result", searchResult);

  await expandWorkTranscript(transcript);
  await expandToolPayloads(transcript);
  const titles = await transcript.locator(".ef-chat__tool-title").allInnerTexts();
  const transcriptText = await transcript.innerText();
  report("memory tool card titles", titles);
  report("memory transcript text", transcriptText);
  await page.screenshot({ animations: "disabled", path: testInfo.outputPath("memory-tools.png") });

  expect(titles.some((title) => title.includes("保存记忆"))).toBe(true);
  expect(titles.some((title) => title.includes("记忆") && title.includes(memoryMarker))).toBe(true);
  for (const rawName of RAW_TOOL_NAMES) {
    expect(transcriptText, `transcript must not contain raw tool name ${rawName}`).not.toContain(rawName);
  }
  assertGatewayToolsInvariant();

  // Persistence proof, running fully offline: the suite registers the local stub
  // as an OpenAI-compatible EMBEDDING platform model and repoints the built-in
  // `memory-card` corpus at it, so `memory_save` embeds through the stub, writes
  // a real memory row and `memory_search` reads that exact row back.
  //
  // `memory_save` answers with the new memory's identity and index diagnostics
  // only (`MemorySaveResponse` in tron-backend/src/application/memory/dto.rs has
  // no text field), so the stored text is proven by reading it back verbatim:
  // the search hit must carry the memoryCode `memory_save` just returned, and its
  // `text` must equal the saved text exactly. These assertions stay strict on
  // purpose: a green run requires a real embedding request and a real storage
  // round trip.
  report("memory embedding requests", embeddingRequests);
  expect(embeddingRequests.length, "memory save/search must reach the stub embedding endpoint").toBeGreaterThan(0);

  const save = JSON.parse(saveResult);
  expect(save.success).toBe(true);
  expect(save.action).toBe("CREATED");
  expect(typeof save.memoryCode).toBe("string");
  expect(save.memoryCode.length).toBeGreaterThan(0);
  expect(save.scope).toBe("USER");
  expect(save.rag.status).toBe("ACTIVE");
  expect(save.rag.textUnitCount).toBeGreaterThan(0);
  expect(save.rag.fulltextStatus).toBe("INDEXED");
  expect(save.rag.embeddingStatus).toBe("INDEXED");

  const searched = JSON.parse(searchResult);
  expect(searched.success).toBe(true);
  expect(searched.count).toBeGreaterThan(0);
  const saved = searched.items.find((item: { memoryCode?: string }) => item.memoryCode === save.memoryCode);
  expect(saved, `memory_search must return the memory_save row ${save.memoryCode}`).toBeTruthy();
  expect(saved.text).toBe(savedMemoryText);
  expect(searchResult).toContain(memoryMarker);
  expect(searchResult).toContain(savedMemoryText);
});

test("parallel tool calls in one assistant message both complete and group", async ({ page }, testInfo) => {
  test.setTimeout(120_000);
  resetScenarioState("e2e-parallel");
  await openChat(page);
  const input = page.locator('[data-theme-part="chat-input"]');
  const transcript = page.getByRole("region", { name: "Chat transcript", exact: true });
  await input.fill("e2e-parallel: run both commands at once");
  await input.press("Enter");
  await expect(transcript).toContainText("e2e-parallel-done", { timeout: 90_000 });

  const results = toolResultSnapshots.get("e2e-parallel") ?? [];
  const resultText = results.map((entry) => entry.content).join("\n");
  report("parallel tool results", results.map((entry) => entry.content));

  expect(results).toHaveLength(2);
  expect(resultText).toContain("acc-parallel-a");
  expect(resultText).toContain("acc-parallel-b");

  await expandWorkTranscript(transcript);
  await expandToolPayloads(transcript);
  const titles = await transcript.locator(".ef-chat__tool-title").allInnerTexts();
  const groupSummaries = await transcript.locator(".ef-chat__rail-entry-summary").allInnerTexts();
  const transcriptText = await transcript.innerText();
  report("parallel tool card titles", titles);
  report("parallel group summaries", groupSummaries);
  report("parallel transcript text", transcriptText);
  await page.screenshot({ animations: "disabled", path: testInfo.outputPath("parallel-tools.png") });

  expect(titles.some((title) => title.includes("acc-parallel-a"))).toBe(true);
  expect(titles.some((title) => title.includes("acc-parallel-b"))).toBe(true);
  expect(groupSummaries.some((summary) => summary.includes("执行") && summary.includes("2"))).toBe(true);
  for (const rawName of RAW_TOOL_NAMES) {
    expect(transcriptText, `transcript must not contain raw tool name ${rawName}`).not.toContain(rawName);
  }
  assertGatewayToolsInvariant();
});

/**
 * The stable two-tool invariant: every model request of the run exposes exactly
 * `tool_execute` and `tool_search`, in that order — no per-request expansion.
 */
function assertGatewayToolsInvariant() {
  report("model requests (count, tools arrays)", { count: modelRequests.length, tools: modelRequests.map((request) => request.tools) });
  expect(modelRequests.length).toBeGreaterThan(0);
  modelRequests.forEach((request, index) => {
    expect(request.tools, `model request ${index + 1} must expose exactly the gateway pair`).toEqual(EXPECTED_TOOL_NAMES);
  });
}