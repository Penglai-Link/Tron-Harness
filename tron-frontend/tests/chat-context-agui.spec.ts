import { expect, test, type Page, type Route } from "@playwright/test";
import type { ChatAguiStreamEvent, ChatMessage, ChatSession } from "@tron/chat-core";
import {
  routeResourceUiShellApi,
  seedResourceUiShell,
  startResourceUiFrontend,
  stopResourceUiFrontend,
  type ResourceUiFrontend,
} from "./support/resource-ui-harness";

let frontend: ResourceUiFrontend;

test.beforeAll(async () => {
  const mockDomains = ["ADMIN", "AGENT", "APP", "AUTOMATION", "KNOWLEDGE_GRAPH", "MCP", "MEMORY", "MODEL", "OBSERVABILITY", "PROMPT", "RAG", "RESOURCE", "SKILL", "TASK", "VARIABLE"];
  frontend = await startResourceUiFrontend("/chats", {
    ...Object.fromEntries(mockDomains.map((domain) => [`VITE_TRON_${domain}_DATA_SOURCE`, "mock"])),
    VITE_TRON_API_BASE_URL: "",
    VITE_TRON_DATA_SOURCE: "mock",
    VITE_TRON_CHAT_DATA_SOURCE: "api",
  });
});

test.afterAll(async () => stopResourceUiFrontend(frontend));

async function fulfillJson(route: Route, data: unknown) {
  await route.fulfill({ json: { code: "SUCCESS", message: "ok", data } });
}

async function mockChat(page: Page) {
  const now = Date.now();
  const session: ChatSession = {
    id: 1, sessionCode: "SESSION-CONTEXT", workspaceCode: "TRON_CORE", title: "Context checks",
    sourceType: "MANUAL", agentCode: "main-agent", status: "ACTIVE",
    createdAt: new Date(now).toISOString(), updatedAt: new Date(now).toISOString(),
  };
  let messages: ChatMessage[] = [];
  let events: ChatAguiStreamEvent[] = [];
  const userMessage = (content: string): ChatMessage => ({
    id: 1, messageCode: "MSG-CONTEXT", sessionCode: session.sessionCode, role: "USER",
    actor: "User", channel: "MAIN_AGENT", content, createdAt: new Date(now).toISOString(),
  });
  const event = (type: string, extra: Partial<ChatAguiStreamEvent> = {}): ChatAguiStreamEvent => ({
    type, threadId: session.sessionCode, runId: "RUN-CONTEXT", timestamp: now + 1, ...extra,
  });

  await routeResourceUiShellApi(page);
  await page.route("**/api/v1/chats/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path.endsWith("/session-groups")) {
      return fulfillJson(route, [{ taskCode: null, taskTitle: "Workspace chat", sessions: [session] }]);
    }
    if (path.endsWith("/get-active-run")) {
      const active = events.length > 0 && !events.some((entry) => entry.type === "RUN_ERROR" || entry.type === "RUN_FINISHED");
      return fulfillJson(route, {
        sessionCode: session.sessionCode,
        activeRun: active ? { runCode: "RUN-CONTEXT", state: "RUNNING", runtimeAdapterKey: "TRON", supportsSteer: false } : null,
      });
    }
    if (path.endsWith("/messages/stream")) {
      messages = [userMessage(route.request().postDataJSON().content)];
      events = [
        event("RUN_STARTED"),
        event("RUN_ERROR", {
          timestamp: now + 2, code: "tool_schema_over_budget",
          message: "TOOL_SCHEMA_OVER_BUDGET: estimatedTokens=27294, limitTokens=21597, toolCount=89; tool exposure was not changed",
        }),
      ];
      return route.fulfill({
        contentType: "text/event-stream",
        body: events.map((entry) => `event: ${entry.type.toLowerCase()}\ndata: ${JSON.stringify(entry)}\n\n`).join(""),
      });
    }
    if (path.endsWith("/messages")) return fulfillJson(route, messages);
    if (path.endsWith("/user-input-requests") || path.endsWith("/get-todos")) return fulfillJson(route, []);
    if (/\/get-(latest-compaction|latest-goal|goal)$/.test(path)) return fulfillJson(route, null);
    if (path.endsWith(`/sessions/${session.sessionCode}`)) return fulfillJson(route, session);
    return route.fallback();
  });
  await page.route("**/api/v1/agents/agui/streams/**", async (route) => {
    const protocol = new URL(route.request().url()).pathname.endsWith("/protocol-events");
    return fulfillJson(route, protocol ? events.map((data, index) => ({ sequence: index + 1, sseEvent: data.type.toLowerCase(), data })) : { events: [] });
  });
  return {
    url: `${frontend.baseUrl}/chats?sessionCode=${session.sessionCode}`,
    compact(status: "started" | "completed") {
      messages = [userMessage("Compact context")];
      events = [event("RUN_STARTED"), event("CUSTOM", { name: "tron.short_term_memory.compaction.started" })];
      if (status === "completed") events.push(
        event("CUSTOM", { name: "tron.short_term_memory.compaction.completed", timestamp: now + 2 }),
        event("RUN_FINISHED", { timestamp: now + 3 }),
      );
    },
  };
}

for (const locale of ["en-US", "zh-CN"] as const) {
  for (const viewport of [{ width: 1440, height: 900 }, { width: 1920, height: 1080 }]) {
    test(`chat displays live errors and replays them: ${locale} ${viewport.width}`, async ({ page }, testInfo) => {
      await page.setViewportSize(viewport);
      await seedResourceUiShell(page, locale);
      const fixture = await mockChat(page);
      const pageErrors: string[] = [];
      page.on("pageerror", (error) => pageErrors.push(error.message));
      await page.goto(fixture.url);
      await page.waitForLoadState("networkidle");
      const transcript = page.getByRole("region", { name: "Chat transcript", exact: true });
      const input = page.locator(".tron-chat__textarea");
      await expect(input).toBeEnabled();
      await input.fill("Inspect tools");
      await input.press("Enter");
      const title = locale === "zh-CN" ? "工具定义超过上下文预算" : "Tool definitions exceed the context budget";
      const alert = transcript.getByRole("alert");
      await expect(alert).toContainText(title);
      await expect(alert).toContainText("27,294");
      await expect(alert).toContainText("21,597");
      await expect(alert).toContainText("89");
      await expect(transcript.locator("button.tron-chat__rail-summary")).toHaveCount(0);
      await expect(transcript.getByRole("article")).toHaveCount(1);
      await page.reload();
      await expect(alert).toContainText(title);
      await expect(alert).toHaveCount(1);
      await expect.poll(() => alert.evaluate((node) => node.scrollWidth - node.clientWidth)).toBeLessThanOrEqual(1);
      await expect.poll(() => page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
      expect(pageErrors).toEqual([]);
      await page.screenshot({ path: testInfo.outputPath("chat-context-error.png") });
    });
  }
}

test("chat updates compaction in place and retains the completed state after refresh", async ({ page }, testInfo) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await seedResourceUiShell(page, "zh-CN");
  const fixture = await mockChat(page);
  fixture.compact("started");
  await page.goto(fixture.url);
  await page.waitForLoadState("networkidle");
  const transcript = page.getByRole("region", { name: "Chat transcript", exact: true });
  const status = transcript.getByRole("status");
  await expect(status).toContainText("正在压缩上下文");
  fixture.compact("completed");
  await expect(status).toContainText("上下文已压缩");
  await expect(status).toHaveCount(1);
  await expect(transcript.locator("button.tron-chat__rail-summary")).toHaveCount(0);
  await page.reload();
  await expect(status).toContainText("上下文已压缩");
  await expect(status).toHaveCount(1);
  await page.screenshot({ path: testInfo.outputPath("chat-context-completed.png") });
});
