import { expect, test } from "@playwright/test";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { createServer as createHttpServer, type Server } from "node:http";
import { createServer } from "node:net";
import { fileURLToPath } from "node:url";
import { spawnManagedProcess, stopManagedProcess, type ManagedProcess } from "./support/managed-process";
import { routeActiveProfileApi, seedResourceUiShell, startResourceUiFrontend, stopResourceUiFrontend, type ResourceUiFrontend } from "./support/resource-ui-harness";

let backend: ManagedProcess | undefined;
let frontend: ResourceUiFrontend | undefined;
let provider: Server | undefined;
let directory = "";
let api = "";
let questionIssued = false;
const requests: Array<{ messages: Array<{ role: string; content: unknown }>; stream?: boolean }> = [];
async function post(path: string, body: unknown) {
  const response = await fetch(`${api}${path}`, { method: "POST", headers: { "Content-Type": "application/json", Authorization: "Bearer profile-test-token" }, body: JSON.stringify(body) });
  const payload = await response.json();
  if (!response.ok) throw new Error(`${path}: ${JSON.stringify(payload)}`);
  return payload.data;
}
test.beforeAll(async () => {
  test.setTimeout(240_000);
  directory = await mkdtemp(join(tmpdir(), "tron-chat-framework-"));
  provider = createHttpServer(async (request, response) => {
    let body = ""; for await (const chunk of request) body += chunk;
    const payload = JSON.parse(body);
    requests.push(payload);
    const lastUser = [...(payload.messages || [])].reverse().find((message: {role: string}) => message.role === "user");
    const content = JSON.stringify(lastUser?.content || "");
    const questionTurn = JSON.stringify(payload.messages).includes("acceptance-question");
    const askQuestion = payload.stream && questionTurn && !questionIssued;
    if (askQuestion) questionIssued = true;
    const answer = questionTurn ? "framework-question-resumed" : content.includes("acceptance-turn-two") ? "framework-second-reply" : content.includes("acceptance-turn-one") ? "framework-first-reply" : "Conversation acceptance summary";
    const usage = { prompt_tokens: 240, completion_tokens: 32, total_tokens: 272 };
    if (payload.stream) {
      response.writeHead(200, { "Content-Type": "text/event-stream" });
      response.write(`data: ${JSON.stringify({ id: "acceptance", choices: [{ index: 0, delta: askQuestion ? { role: "assistant", tool_calls: [{ index: 0, id: "question-call", type: "function", function: { name: "tool_execute", arguments: JSON.stringify({ tool: "chat_ask_user", arguments: { intro: "Choose the acceptance target", questions: [{ question: "Which target should I use?", options: [{ label: "Acceptance Local" }, { label: "Acceptance Remote" }] }] } }) } }] } : { role: "assistant", content: answer }, finish_reason: null }] })}\n\n`);
      response.end(`data: ${JSON.stringify({ id: "acceptance", choices: [{ index: 0, delta: {}, finish_reason: askQuestion ? "tool_calls" : "stop" }], usage })}\n\ndata: [DONE]\n\n`);
    } else {
      response.writeHead(200, { "Content-Type": "application/json" });
      response.end(JSON.stringify({ id: "acceptance", choices: [{ index: 0, message: { role: "assistant", content: answer }, finish_reason: "stop" }], usage }));
    }
  });
  await new Promise<void>(resolveReady => provider!.listen(0, "127.0.0.1", resolveReady));
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
    cwd: resolve(fileURLToPath(import.meta.url), "../../.."), env: { ...process.env, CARGO_PROFILE_DEV_DEBUG: process.env.CARGO_PROFILE_DEV_DEBUG || "0", CARGO_INCREMENTAL: process.env.CARGO_INCREMENTAL || "0", CARGO_BUILD_JOBS: "2", TRON_ACCEPTANCE_DIRECTORY: directory, TRON_ACCEPTANCE_PORT: String(port) }, stdio: "pipe",
  });
  backend.stdout.on("data", data => logs += data.toString()); backend.stderr.on("data", data => logs += data.toString());
  const deadline = Date.now() + 210_000;
  let ready = false;
  while (Date.now() < deadline) {
    if (backend.exitCode !== null) throw new Error(logs);
    try { ready = (await fetch(`${api}/api/v1/health`)).ok; } catch { /* backend compiling */ }
    if (ready) break;
    await new Promise(resolveDelay => setTimeout(resolveDelay, 250));
  }
  if (!ready) throw new Error(logs);
  const agentResponse = await fetch(`${api}/api/v1/agents/workspaces/TRON_CORE/internal`, {
    headers: { Authorization: "Bearer profile-test-token" },
  });
  expect(agentResponse.ok).toBe(true);
  const agents = (await agentResponse.json()).data;
  expect(agents.map((agent: { agentCode: string }) => agent.agentCode)).toEqual(expect.arrayContaining(["main-agent"]));

  await post("/api/v1/admin/platform/models", {
    displayName: "Framework Acceptance", vendor: "OpenAI", protocol: "OPENAI_COMPATIBLE", modelType: "TEXT", modelCapabilities: ["TEXT_GENERATION"],
    baseUrl: `http://127.0.0.1:${providerAddress.port}/v1`, apiPath: "/chat/completions", apiKeyRef: "sk-acceptance", modelName: "acceptance-model", enabled: true,
    thinkingEnabled: false, structuredOutputEnabled: false, temperature: 0.2, maxTokens: 1024, contextWindowTokens: 128000, topP: 1, maxRetries: 0,
  });
  frontend = await startResourceUiFrontend("/chats", {
    VITE_TRON_DATA_SOURCE: "mock", VITE_TRON_CHAT_DATA_SOURCE: "api", VITE_TRON_MODEL_DATA_SOURCE: "api", VITE_TRON_AGENT_DATA_SOURCE: "api",
    VITE_TRON_API_BASE_URL: "", TRON_FRONTEND_API_PROXY_TARGET: api,
  });
});
test.afterAll(async () => {
  await stopResourceUiFrontend(frontend); await stopManagedProcess(backend);
  await new Promise<void>(resolveClose => provider ? provider.close(() => resolveClose()) : resolveClose());
  if (directory) await rm(directory, { recursive: true, force: true });
});

async function inspectContextPreviews(page: import("@playwright/test").Page, testInfo: import("@playwright/test").TestInfo, locale: "en-US" | "zh-CN") {
  const zh = locale === "zh-CN";
  const sidebar = page.getByTestId("tron-right-sidebar");
  const dialog = page.getByRole("dialog");
  const openBlock = async (name: string) => {
    const responsePromise = page.waitForResponse(response => response.url().includes("get-context-detail"));
    // The block rows are buttons whose text starts with the block order (for
    // example "8.Tool Definitions"); the protocol block also renders an info
    // tooltip button carrying the same accessible name, so match on the row
    // text to stay unambiguous.
    await sidebar.getByRole("button").filter({ hasText: new RegExp(name) }).first().click();
    const response = await responsePromise;
    expect(response.ok()).toBe(true);
    const data = (await response.json()).data;
    expect(data.budgetBreakdown).toBeUndefined();
    expect(data.blocks).toBeUndefined();
    return data;
  };
  const close = async () => dialog.getByRole("button", { name: zh ? "关闭预览" : "Close preview", exact: true }).click();
  const tool = await openBlock(zh ? "工具定义" : "Tool Definitions");
  // The tool gateway exposes exactly two stable tools to the model, so the
  // tool definitions block in the request context must contain that pair (and
  // nothing else) instead of the per-request expanded catalogue.
  expect(tool.tools.map((value: { toolName: string }) => value.toolName).sort()).toEqual(["tool_execute", "tool_search"]);
  await expect(dialog.getByRole("heading", { name: "tool_search", exact: true })).toBeVisible();
  await expect(dialog.locator("section").filter({ has: page.getByRole("heading", { name: "tool_search", exact: true }) }).getByRole("table")).toContainText("domainCode");
  await expect(dialog.getByRole("heading", { name: "tool_execute", exact: true })).toBeVisible();
  await page.screenshot({ animations: "disabled", path: testInfo.outputPath(`tool-definitions-${locale}.png`) });
  await close();
  // The current short-term memory sidebar no longer renders a related-sessions block.
  const prompt = await openBlock(zh ? "系统提示词" : "System Prompt");
  expect(prompt.messages.length).toBeGreaterThan(0);
  await expect(dialog.getByRole("heading", { name: zh ? "第 1 条消息 · 系统" : "Message 1 · System", exact: true })).toBeVisible();
  expect(await dialog.locator("article").count()).toBe(0);
  // The system prompt budget (/ 16k) is asserted on the sidebar row above; the
  // preview dialog renders the prompt messages themselves.
  await dialog.getByRole("button", { name: zh ? "原文" : "Raw JSON", exact: true }).click();
  await expect(dialog.locator("pre").last()).not.toContainText("budgetBreakdown");
  await expect(dialog.locator("pre").last()).toContainText('"parts"');
  await dialog.getByRole("button", { name: zh ? "预览" : "Preview", exact: true }).click();
  await page.screenshot({ animations: "disabled", path: testInfo.outputPath(`system-prompt-${locale}.png`) });
  await close();
  const history = await openBlock(zh ? "模型请求中的历史" : "Request History");
  expect(history.messages.length).toBeGreaterThan(0);
  await close();
}

test("public chat uses the real framework, restores history and shows its context window", async ({ page }, testInfo) => {
  test.setTimeout(60_000);
  await page.setViewportSize({ width: 1440, height: 1000 });
  await seedResourceUiShell(page, "en-US");
  await routeActiveProfileApi(page);
  const errors: string[] = [];
  page.on("pageerror", error => errors.push(error.message));
  await page.goto(`${frontend!.baseUrl}/chats`);
  await page.getByRole("button", { name: "New chat", exact: true }).click();
  await page.getByRole("button", { name: "Select chat model", exact: true }).click();
  await page.getByRole("menuitemradio", { name: "Framework Acceptance", exact: true }).click();
  const input = page.locator('[data-theme-part="chat-input"]');
  const transcript = page.getByRole("region", { name: "Chat transcript", exact: true });
  await input.fill("acceptance-turn-one: remember the word sapphire"); await input.press("Enter");
  await expect(transcript).toContainText("framework-first-reply");
  await expect(input).toBeEnabled();
  await expect(transcript).not.toContainText("tronMessagePart");
  await page.reload();
  await expect(transcript).toContainText("framework-first-reply");
  await expect(transcript).not.toContainText("tronMessagePart");
  await input.fill("acceptance-turn-two: use the earlier word"); await input.press("Enter");
  await expect(transcript).toContainText("framework-second-reply");
  expect(requests.some(request => request.stream && JSON.stringify(request.messages).includes("acceptance-turn-one") && JSON.stringify(request.messages).includes("acceptance-turn-two") && JSON.stringify(request.messages).includes("framework-first-reply"))).toBe(true);
  // The interactive-question contract now reaches the model through the gateway:
  // the payload the model actually receives must instruct the envelope call.
  expect(JSON.stringify(requests).replace(/\\"/g, '"')).toContain('{"tool":"chat_ask_user"');
  const open = page.getByRole("button", { name: "Open right sidebar", exact: true });
  if (await open.isVisible()) await open.click();
  const sidebar = page.getByTestId("tron-right-sidebar");
  await sidebar.getByRole("tab", { name: "Short-term memory", exact: true }).click();
  await expect(sidebar.getByText("Request History")).toBeVisible();
  await expect(sidebar.getByText("Session Total", { exact: true })).toBeVisible();
  // The sidebar renders the platform model identity of the active connection
  // (currently the generated model code) followed by the context window; the
  // seeded provider exposes a 128k window either way.
  await expect(sidebar).toContainText(/· 128k window/);
  await expect(sidebar).toContainText("Request History· 3 messages");
  await expect(sidebar.getByText("480", { exact: true })).toBeVisible();
  await expect(sidebar.getByText("240", { exact: true })).toBeVisible();
  await expect(sidebar).toContainText("/ 16k");
  await inspectContextPreviews(page, testInfo, "en-US");
  expect(errors).toEqual([]);
  await page.screenshot({ animations: "disabled", path: testInfo.outputPath("real-framework-chat-context.png") });
  await input.fill("acceptance-question: ask me to choose the target"); await input.press("Enter");
  // The question card renders the questions and their options; the interaction
  // intro is carried in the payload but not displayed by the current card.
  await expect(page.getByRole("region", { name: "Assistant questions", exact: true }).getByText("Which target should I use?", { exact: true })).toBeVisible();
  await page.reload();
  await expect(page.getByRole("region", { name: "Assistant questions", exact: true }).getByText("Which target should I use?", { exact: true })).toBeVisible();
  const answers = page.getByRole("region", { name: "Assistant questions", exact: true });
  await answers.getByText("Acceptance Local", { exact: true }).click();
  await expect(page.getByRole("radio", { name: /Acceptance Local/ })).toBeChecked();
  const nextStep = answers.getByRole("button", { name: "Continue", exact: true });
  if (await nextStep.isEnabled()) await nextStep.click();
  const confirmStep = answers.getByRole("button", { name: "Confirm", exact: true });
  if ((await confirmStep.count()) > 0 && (await confirmStep.isEnabled())) await confirmStep.click();
  await expect(transcript).toContainText("framework-question-resumed", { timeout: 30_000 });
  expect(errors).toEqual([]);
  await page.screenshot({ animations: "disabled", path: testInfo.outputPath("real-framework-question-resumed.png") });
});

test("context previews show native Framework content in Chinese", async ({ page }, testInfo) => {
  await page.setViewportSize({ width: 1440, height: 1000 });
  await seedResourceUiShell(page, "zh-CN");
  await routeActiveProfileApi(page);
  await page.goto(`${frontend!.baseUrl}/chats`);
  await page.getByRole("button", { name: "新会话", exact: true }).click();
  await page.getByRole("button", { name: "选择模型", exact: true }).click();
  await page.getByRole("menuitemradio", { name: "Framework Acceptance", exact: true }).click();
  const input = page.locator('[data-theme-part="chat-input"]');
  await input.fill("acceptance-turn-one: 检查短期记忆");
  await input.press("Enter");
  await expect(page.getByRole("region", { name: "Chat transcript", exact: true })).toContainText("framework-first-reply");
  // The zh run must receive the localized gateway contract (same envelope form).
  expect(JSON.stringify(requests).replace(/\\"/g, '"')).toContain('必须调用 `tool_execute`，参数为 `{"tool":"chat_ask_user"');
  const open = page.getByRole("button", { name: "打开右侧边栏", exact: true });
  if (await open.isVisible()) await open.click();
  await page.getByTestId("tron-right-sidebar").getByRole("tab", { name: "短期记忆", exact: true }).click();
  await inspectContextPreviews(page, testInfo, "zh-CN");
});
