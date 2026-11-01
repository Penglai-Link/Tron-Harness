import { gatewayArguments, singleGatewayResult } from "./support/gateway-stub";
import { expect, test, type Page } from "@playwright/test";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { createServer as createHttpServer, type Server } from "node:http";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnManagedProcess, stopManagedProcess, type ManagedProcess } from "./support/managed-process";
import { seedResourceUiShell, startResourceUiFrontend, stopResourceUiFrontend, type ResourceUiFrontend } from "./support/resource-ui-harness";

let backend: ManagedProcess | undefined;
let frontend: ResourceUiFrontend | undefined;
let provider: Server | undefined;
let directory = "";
let api = "";
let foregroundStarted = false;
let backgroundTaskCode = "";
let questionIssued = false;
let releaseForeground = () => {};
let backendLogs = "";
const evidence = new WeakMap<Page, { errors: string[]; messages: string[]; chatRequests: unknown[]; chatResponses: { status: number; url: string; body: string }[] }>();
const providerRequests: unknown[] = [];
let dudeToolResults: Record<string, string> = {};
let dudeToolStatuses: Record<string, {status: string; isError: boolean}> = {};
let dudeDelegationResults: Record<string, string> = {};
let dudeChildResults: Record<string, string> = {};
let dudeIdentityResult = "";
let dudeSharedResults: Record<string, {status: string; isError: boolean; content: string}> = {};
let dudePersonAiResults: Record<string, {status: string; isError: boolean; content: string}> = {};
const personaiFixtureEnabled = process.env.TRON_TEST_PERSONAI_REMOTE === "1";
const personaiRequests: Array<{path:string;productCode:string | undefined;authorized:boolean}> = [];

// Match common::app_with_state_setup rather than the resource-only shell's
// USR-RESOURCE-0001/resource-ui-token identity. Product endpoints remain real.
async function routeAcceptanceProfile(page: Page) {
  const profile = { profileId: "PROFILE-ACCEPTANCE", personAiUserId: "USR-AUTH-0001",
    personAiUsername: "profile.operator", status: "ACTIVE", createdAt: "2026-09-27T00:00:00Z",
    lastActivatedAt: null, avatarUrl: null };
  await page.route(/\/api\/v1\/(?:bootstrap\/status|profiles(?:\/active\/token|\/events)?)(?:\?.*)?$/, async route => {
    const path = new URL(route.request().url()).pathname;
    if (path === "/api/v1/profiles/events") return route.fulfill({ status: 204 });
    const data = path === "/api/v1/bootstrap/status"
      ? { hasProfiles: true, activeProfileId: profile.profileId, activeProfile: profile, avatarUrl: null, account: null }
      : path === "/api/v1/profiles" ? [profile] : { token: "profile-test-token" };
    await route.fulfill({ json: { code: "SUCCESS", data, message: "ok" } });
  });
}

test.beforeEach(async ({ page }) => {
  const captured = { errors: [] as string[], messages: [] as string[], chatRequests: [] as unknown[], chatResponses: [] as { status: number; url: string; body: string }[] };
  evidence.set(page, captured);
  page.on("pageerror", error => captured.errors.push(error.stack ?? error.message));
  page.on("console", message => { if (message.type() === "error" || message.type() === "warning") captured.messages.push(`${message.type()}: ${message.text()}`); });
  page.on("requestfailed", failed => captured.messages.push(`requestfailed: ${failed.method()} ${failed.url()} ${failed.failure()?.errorText}`));
  page.on("request", outgoing => {
    if (outgoing.method() === "POST" && /\/api\/v1\/(?:chats|agents)\//.test(outgoing.url())) {
      captured.chatRequests.push({ url: outgoing.url(), body: outgoing.postData() });
    }
  });
  page.on("response", response => {
    if (response.status() >= 400) captured.messages.push(`HTTP ${response.status()}: ${response.url()}`);
    if (response.request().method() === "POST" && /\/api\/v1\/(?:chats|agents)\//.test(response.url())) {
      // For a streaming chat this resolves when it closes, including a 200
      // stream that ends with RUN_ERROR before ever contacting the provider.
      void response.text().then(body => captured.chatResponses.push({ status: response.status(), url: response.url(), body: body.slice(-500000) }))
        .catch(error => captured.messages.push(`response body: ${response.url()} ${error}`));
    }
  });
});

test.afterEach(async ({ page }, testInfo) => {
  if (testInfo.status === testInfo.expectedStatus) return;
  const captured = evidence.get(page);
  const snapshot = await page.locator("body").ariaSnapshot({ timeout: 2000 }).catch(error => `snapshot failed: ${error}`);
  const html = await page.content().catch(error => `content failed: ${error}`);
  const diagnosticsPath = testInfo.outputPath("dude-page-diagnostics.json");
  await writeFile(diagnosticsPath, JSON.stringify({
    url: page.url(), ...captured, providerRequests, snapshot, html: html.slice(0, 20000), frontend: frontend?.logs(), backend: backendLogs.slice(-16000),
  }, null, 2));
  await testInfo.attach("dude-page-diagnostics", { contentType: "application/json", path: diagnosticsPath });
  const screenshotPath = testInfo.outputPath("dude-failure-screen.png");
  const screenshot = await page.screenshot({ path: screenshotPath, animations: "disabled", timeout: 3000 }).catch(() => null);
  if (screenshot) await testInfo.attach("dude-failure-screen", { contentType: "image/png", path: screenshotPath });
});

async function request(method: string, path: string, body?: unknown) {
  const response = await fetch(`${api}${path}`, { method, headers: { "Content-Type": "application/json", Authorization: "Bearer profile-test-token" }, body: body === undefined ? undefined : JSON.stringify(body) });
  const payload = await response.json();
  if (!response.ok) throw new Error(`${method} ${path}: ${JSON.stringify(payload)}`);
  return payload.data;
}

test.beforeAll(async () => {
  test.setTimeout(300_000);
  directory = await mkdtemp(join(tmpdir(), "tron-dude-acceptance-"));
  const skillDirectory = join(directory, "skills/.agents/skills/dude-ui-assignment");
  await mkdir(skillDirectory, { recursive: true });
  await writeFile(join(skillDirectory, "SKILL.md"), "---\nname: dude-ui-assignment\ndescription: Dude resource assignment acceptance\n---\nUse assigned resources.");
  await mkdir(join(skillDirectory, "references"), { recursive: true });
  await writeFile(join(skillDirectory, "references/details.md"), "Dude acceptance skill resource");
  await writeFile(join(directory, "dude-image.png"), Buffer.from("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAIAAACQd1PeAAAADElEQVR4nGP4z8AAAAMBAQDJ/pLvAAAAAElFTkSuQmCC", "base64"));
  const pdfText = "BT /F1 12 Tf 10 100 Td (Dude acceptance PDF text) Tj ET";
  const objects = ["<< /Type /Catalog /Pages 2 0 R >>", "<< /Type /Pages /Kids [3 0 R] /Count 1 >>", "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>", "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>", `<< /Length ${Buffer.byteLength(pdfText)} >>\nstream\n${pdfText}\nendstream`];
  let pdf = "%PDF-1.4\n"; const offsets = [0];
  objects.forEach((object,index) => {offsets.push(Buffer.byteLength(pdf)); pdf += `${index + 1} 0 obj\n${object}\nendobj\n`;});
  const xref = Buffer.byteLength(pdf);
  pdf += `xref\n0 ${offsets.length}\n0000000000 65535 f \n${offsets.slice(1).map(offset => `${String(offset).padStart(10,"0")} 00000 n \n`).join("")}trailer\n<< /Size ${offsets.length} /Root 1 0 R >>\nstartxref\n${xref}\n%%EOF\n`;
  await writeFile(join(directory, "dude.pdf"), pdf);
  provider = createHttpServer(async (incoming, response) => {
    if (incoming.method === "GET" && incoming.url === "/acceptance-web") {
      response.writeHead(200, {"Content-Type":"text/html"});
      response.end("<title>Dude acceptance page</title><p>Dude acceptance web content</p>");
      return;
    }
    const requestPath = incoming.url ?? "";
    if (personaiFixtureEnabled && requestPath.startsWith("/api/v1/")) {
      personaiRequests.push({path:requestPath,productCode:incoming.headers["x-product-code"] as string | undefined,authorized:incoming.headers.authorization === "Bearer acceptance-token"});
      let data: unknown = [];
      if (requestPath.startsWith("/api/v1/search")) data = {query:"Rust",count:1,results:[{title:"Rust",url:"https://rust-lang.org",snippet:"Dude search result",source:"fixture"}],providerCode:"fixture"};
      else if (requestPath.startsWith("/api/v1/images/generations")) data = {id:"image-task",media_type:0,status:0,progress:0};
      else if (requestPath.startsWith("/api/v1/multimedia/tasks/image-task")) data = {id:"image-task",media_type:0,status:2,progress:100,result_url:"https://example.invalid/image.png"};
      else if (requestPath.startsWith("/api/v1/aigc/detections/")) data = {id:"detection",detection_type:requestPath.endsWith("/image") ? 1 : 0,model_code:"detector",status:0,ai_confidence:0.9};
      else if (requestPath.includes("media_type=")) {const kind = Number(new URL(requestPath,"http://127.0.0.1").searchParams.get("media_type"));data = [{model_id:`media-${kind}`,model_code:`media-${kind}`,media_type:kind}];}
      response.writeHead(200, {"Content-Type":"application/json"});
      response.end(JSON.stringify({code:0,data}));
      return;
    }
    let body = "";
    for await (const chunk of incoming) body += chunk;
    const payload = JSON.parse(body);
    if (incoming.url?.includes("embeddings")) {
      const inputs = Array.isArray(payload.input) ? payload.input : [payload.input];
      const embedding = Array<number>(1024).fill(0); embedding[0] = 1;
      response.writeHead(200, { "Content-Type": "application/json" });
      response.end(JSON.stringify({ object:"list", model:payload.model, data:inputs.map((_: unknown, index:number) => ({ object:"embedding", index, embedding })), usage:{prompt_tokens:8,total_tokens:8} }));
      return;
    }
    const input = JSON.stringify(payload.messages ?? []);
    // The platform conversation persists between tests, so older scenario
    // markers remain in the transcript. Dispatch to the newest scenario.
    const scenario = ["acceptance-askuser", "acceptance-dude-tools", "acceptance-dude-shared", "acceptance-dude-personai", "acceptance-dude-a2ui", "acceptance-dude-delegation", "acceptance-dude-child", "acceptance-dude-identity", "acceptance-background-work", "acceptance-foreground-hold"]
      .map(marker => ({ marker, index: input.lastIndexOf(marker) }))
      .filter(item => item.index >= 0)
      .sort((a, b) => b.index - a.index)[0]?.marker;
    // Context compression uses a dedicated nonstream request with this exact
    // input contract and expects {summary: nonempty string}, not a chat answer.
    let summaryRequest: { previous_summary: string; history: unknown[]; summary_max_utf8_bytes: number } | undefined;
    try {
      const lastContent = payload.messages?.at(-1)?.content;
      const candidate = typeof lastContent === "string" ? JSON.parse(lastContent) : undefined;
      if (typeof candidate?.previous_summary === "string" && Array.isArray(candidate.history)
        && Number.isInteger(candidate.summary_max_utf8_bytes) && candidate.summary_max_utf8_bytes > 0) {
        summaryRequest = candidate;
      }
    } catch { /* Ordinary chat input is not a summary request. */ }
    providerRequests.push({ path: incoming.url, model: payload.model, stream: payload.stream,
      purpose: summaryRequest ? "conversation_summary" : "chat",
      foreground: input.includes("acceptance-foreground-hold"), background: input.includes("acceptance-background-work"),
      messageCount: payload.messages?.length ?? 0, lastMessage: JSON.stringify(payload.messages?.at(-1) ?? null).slice(0, 2000) });
    if (payload.stream && !summaryRequest && scenario === "acceptance-askuser" && !questionIssued) {
      questionIssued = true;
      response.writeHead(200, { "Content-Type": "text/event-stream" });
      response.write(`data: ${JSON.stringify({ id: "dude-question", choices: [{ index: 0, delta: { tool_calls: [{ index: 0, id: "dude-question-call", type: "function", function: { name: "tool_execute", arguments: JSON.stringify({ tool: "chat_ask_user", arguments: { intro: "Choose the acceptance target", questions: [{ question: "Which target should I use?", options: [{ label: "Acceptance Local" }, { label: "Acceptance Remote" }] }] } }) } }] }, finish_reason: "tool_calls" }], usage: { prompt_tokens: 120, completion_tokens: 20, total_tokens: 140 } })}\n\n`);
      response.end("data: [DONE]\n\n");
      return;
    }
    if (payload.stream && !summaryRequest && scenario === "acceptance-dude-tools") {
      const result = (id: string) => payload.messages?.find((message: {role:string;tool_call_id?:string}) => message.role === "tool" && message.tool_call_id === id);
      const parsed = (id: string) => JSON.parse(singleGatewayResult(result(id).content));
      const steps: Array<{ id:string; tool:string; args:() => Record<string, unknown> }> = [
        { id:"dude-memory-save", tool:"dude_memory_save", args:() => ({scope:"USER",memoryType:"USER_PREFERENCE",text:"Dude acceptance memory",title:"Dude acceptance"}) },
        { id:"dude-memory-search", tool:"dude_memory_search", args:() => ({scope:"USER",query:"Dude acceptance memory"}) },
        { id:"dude-memory-delete", tool:"dude_memory_delete", args:() => ({memoryCode:parsed("dude-memory-save").memoryCode,reason:"Acceptance cleanup"}) },
        { id:"dude-task-create", tool:"dude_task_create", args:() => ({title:"Dude acceptance task"}) },
        { id:"dude-task-page", tool:"dude_task_page", args:() => ({search:"Dude acceptance task"}) },
        { id:"dude-task-update", tool:"dude_task_update", args:() => ({taskCode:parsed("dude-task-create").taskCode,title:"Dude updated task"}) },
        { id:"dude-task-delete", tool:"dude_task_delete", args:() => ({taskCode:parsed("dude-task-create").taskCode}) },
        { id:"dude-task-restore", tool:"dude_task_restore", args:() => ({taskCode:parsed("dude-task-create").taskCode}) },
        { id:"dude-soul-read", tool:"dude_soul_read", args:() => ({}) },
        { id:"dude-session-todo", tool:"dude_todo_update", args:() => ({todos:[{position:0,content:"Dude session todo",status:"in_progress",priority:"medium"}]}) },
      ];
      const pending = steps.find(step => !result(step.id));
      if (pending) {
        response.writeHead(200, {"Content-Type":"text/event-stream"});
        response.end(`data: ${JSON.stringify({choices:[{index:0,delta:{tool_calls:[{index:0,id:pending.id,type:"function",function:{name:"tool_execute",arguments:JSON.stringify(gatewayArguments("tool_execute",{tool:pending.tool,arguments:pending.args()}))}}]},finish_reason:"tool_calls"}]})}\n\ndata: [DONE]\n\n`);
        return;
      }
      dudeToolResults = Object.fromEntries(steps.map(step => [step.id, singleGatewayResult(result(step.id).content)]));
      dudeToolStatuses = Object.fromEntries(steps.map(step => [step.id, JSON.parse(result(step.id).content).results[0]]));
      response.writeHead(200, {"Content-Type":"text/event-stream"});
      response.end(`data: ${JSON.stringify({choices:[{index:0,delta:{role:"assistant",content:"dude-tools-complete"},finish_reason:"stop"}]})}\n\ndata: [DONE]\n\n`);
      return;
    }
    if (payload.stream && !summaryRequest && scenario === "acceptance-dude-shared") {
      const result = (id: string) => payload.messages?.find((message: {role:string;tool_call_id?:string}) => message.role === "tool" && message.tool_call_id === id);
      const address = provider?.address();
      if (!address || typeof address === "string") throw new Error("provider address missing");
      const steps = [
        {tool:"skill_page",args:{search:"dude-ui-assignment"}},
        {tool:"skill_view",args:{skillName:"dude-ui-assignment"}},
        {tool:"skill_load",args:{name:"dude-ui-assignment"}},
        {tool:"skill_read_resource",args:{name:"dude-ui-assignment",path:"references/details.md"}},
        {tool:"skill_diagnostics",args:{}},
        {tool:"web_read",args:{url:`http://127.0.0.1:${address.port}/acceptance-web`}},
        {tool:"media_read_image",args:{path:join(directory,"dude-image.png")}},
        {tool:"media_read_pdf",args:{path:join(directory,"dude.pdf"),startPage:1,endPage:1}},
      ];
      const pending = steps.find(step => !result(`dude-shared-${step.tool}`));
      response.writeHead(200, {"Content-Type":"text/event-stream"});
      if (pending) {
        response.end(`data: ${JSON.stringify({choices:[{index:0,delta:{tool_calls:[{index:0,id:`dude-shared-${pending.tool}`,type:"function",function:{name:"tool_execute",arguments:JSON.stringify(gatewayArguments("tool_execute",{tool:pending.tool,arguments:pending.args}))}}]},finish_reason:"tool_calls"}]})}\n\ndata: [DONE]\n\n`);
      } else {
        dudeSharedResults = Object.fromEntries(steps.map(step => {
          const item = result(`dude-shared-${step.tool}`);
          const parsed = JSON.parse(item.content);
          const envelope = Array.isArray(parsed) ? JSON.parse(parsed.find(block => block.type === "text").text) : parsed;
          return [step.tool, {...envelope.results[0], content:singleGatewayResult(JSON.stringify(envelope))}];
        }));
        response.end(`data: ${JSON.stringify({choices:[{index:0,delta:{role:"assistant",content:"dude-shared-complete"},finish_reason:"stop"}]})}\n\ndata: [DONE]\n\n`);
      }
      return;
    }
    if (payload.stream && !summaryRequest && scenario === "acceptance-dude-a2ui") {
      const result = (id:string) => payload.messages?.find((message:{role:string;tool_call_id?:string}) => message.role === "tool" && message.tool_call_id === id);
      const form = Boolean(result("dude-a2ui-display"));
      const surfaceId = form ? "dude-acceptance-form" : "dude-acceptance-report";
      const messages: unknown[] = [
        {version:"v0.9.1",createSurface:{surfaceId,catalogId:"https://eliteforge.dev/a2ui/v0.9.1/agent"}},
        {version:"v0.9.1",updateComponents:{surfaceId,components:form ? [
          {id:"root",component:"Column",children:["name","submit"]},
          {id:"name",component:"TextField",label:"Dude acceptance name",value:{path:"/response/name"}},
          {id:"submit",component:"Button",child:"submit-label",action:{event:{name:"submit",context:{response:{path:"/response"}}}}},
          {id:"submit-label",component:"Text",text:"Submit Dude acceptance"},
        ] : [{id:"root",component:"Text",text:"Dude acceptance report"}]}},
      ];
      if (form) messages.push({version:"v0.9.1",updateDataModel:{surfaceId,value:{response:{name:""}}}});
      response.writeHead(200, {"Content-Type":"text/event-stream"});
      if (!result("dude-a2ui-form")) {
        const tool = form ? "a2ui_request_input" : "a2ui_publish";
        const args = form ? {mode:"SubmitOnce",messages,responseSchema:{type:"object",required:["name"],properties:{name:{type:"string",minLength:1}},additionalProperties:false}} : {mode:"Display",messages};
        response.end(`data: ${JSON.stringify({choices:[{index:0,delta:{tool_calls:[{index:0,id:form ? "dude-a2ui-form" : "dude-a2ui-display",type:"function",function:{name:"tool_execute",arguments:JSON.stringify(gatewayArguments("tool_execute",{tool,arguments:args}))}}]},finish_reason:"tool_calls"}]})}\n\ndata: [DONE]\n\n`);
      } else response.end(`data: ${JSON.stringify({choices:[{index:0,delta:{role:"assistant",content:"dude-a2ui-complete"},finish_reason:"stop"}]})}\n\ndata: [DONE]\n\n`);
      return;
    }
    if (payload.stream && !summaryRequest && scenario === "acceptance-dude-personai") {
      const result = (id: string) => payload.messages?.find((message: {role:string;tool_call_id?:string}) => message.role === "tool" && message.tool_call_id === id);
      const steps = [
        {tool:"web_search",args:{query:"Rust",count:1}},
        {tool:"personai_generate_image",args:{prompt:"A small blue square"}},
        {tool:"personai_query_image",args:{task_id:"image-task"}},
        {tool:"personai_detect_text",args:{text:"Acceptance fixture text"}},
        {tool:"personai_detect_image",args:{image_url:"https://example.invalid/image.png"}},
      ];
      const pending = steps.find(step => !result(`dude-personai-${step.tool}`));
      response.writeHead(200, {"Content-Type":"text/event-stream"});
      if (pending) {
        response.end(`data: ${JSON.stringify({choices:[{index:0,delta:{tool_calls:[{index:0,id:`dude-personai-${pending.tool}`,type:"function",function:{name:"tool_execute",arguments:JSON.stringify(gatewayArguments("tool_execute",{tool:pending.tool,arguments:pending.args}))}}]},finish_reason:"tool_calls"}]})}\n\ndata: [DONE]\n\n`);
      } else {
        dudePersonAiResults = Object.fromEntries(steps.map(step => {const item = result(`dude-personai-${step.tool}`);return [step.tool, {...JSON.parse(item.content).results[0], content:singleGatewayResult(item.content)}];}));
        response.end(`data: ${JSON.stringify({choices:[{index:0,delta:{role:"assistant",content:"dude-personai-complete"},finish_reason:"stop"}]})}\n\ndata: [DONE]\n\n`);
      }
      return;
    }
    if (payload.stream && !summaryRequest && scenario === "acceptance-dude-delegation") {
      const result = (id: string) => payload.messages?.find((message: {role:string;tool_call_id?:string}) => message.role === "tool" && message.tool_call_id === id);
      const parsed = (id: string) => JSON.parse(singleGatewayResult(result(id).content));
      const session = () => parsed("delegate-create").session_id;
      const steps: Array<{ id:string; tool:string; args:() => Record<string, unknown> }> = [
        { id:"delegate-create", tool:"thread_create", args:() => ({agentCode:"main-agent",task:"Work on the isolated acceptance task",title:"Acceptance delegation"}) },
        { id:"delegate-wait", tool:"thread_wait", args:() => ({session_id:session()}) },
        { id:"delegate-read", tool:"thread_read", args:() => ({session_id:session()}) },
        { id:"delegate-send", tool:"thread_send", args:() => ({session_id:session(),message:"Continue the isolated task"}) },
        { id:"delegate-wait-again", tool:"thread_wait", args:() => ({session_id:session()}) },
        { id:"delegate-list", tool:"thread_list", args:() => ({}) },
        { id:"delegate-close", tool:"thread_close", args:() => ({session_id:session()}) },
      ];
      const pending = steps.find(step => !result(step.id));
      if (pending) {
        response.writeHead(200, {"Content-Type":"text/event-stream"});
        response.end(`data: ${JSON.stringify({choices:[{index:0,delta:{tool_calls:[{index:0,id:pending.id,type:"function",function:{name:"tool_execute",arguments:JSON.stringify(gatewayArguments("tool_execute",{tool:pending.tool,arguments:pending.args()}))}}]},finish_reason:"tool_calls"}]})}\n\ndata: [DONE]\n\n`);
        return;
      }
      dudeDelegationResults = Object.fromEntries(steps.map(step => [step.id, singleGatewayResult(result(step.id).content)]));
      response.writeHead(200, {"Content-Type":"text/event-stream"});
      response.end(`data: ${JSON.stringify({choices:[{index:0,delta:{role:"assistant",content:"dude-delegation-complete"},finish_reason:"stop"}]})}\n\ndata: [DONE]\n\n`);
      return;
    }
    if (payload.stream && !summaryRequest && scenario === "acceptance-dude-child") {
      const result = (id: string) => payload.messages?.find((message: {role:string;tool_call_id?:string}) => message.role === "tool" && message.tool_call_id === id);
      const parsed = (id: string) => JSON.parse(singleGatewayResult(result(id).content));
      const child = () => parsed("child-spawn").child_run_id;
      const steps: Array<{ id:string; tool:string; args:() => Record<string, unknown> }> = [
        { id:"child-agent-get", tool:"agent_get", args:() => ({agentCode:"main-agent"}) },
        { id:"child-spawn", tool:"agent_spawn", args:() => ({agentCode:"main-agent",task:"Inspect this isolated child task",background:true}) },
        { id:"child-wait", tool:"child_wait", args:() => ({child_run_id:child()}) },
        { id:"child-read", tool:"child_read", args:() => ({child_run_id:child()}) },
        { id:"child-list", tool:"child_list", args:() => ({}) },
      ];
      const pending = steps.find(step => !result(step.id));
      if (pending) {
        response.writeHead(200, {"Content-Type":"text/event-stream"});
        response.end(`data: ${JSON.stringify({choices:[{index:0,delta:{tool_calls:[{index:0,id:pending.id,type:"function",function:{name:"tool_execute",arguments:JSON.stringify(gatewayArguments("tool_execute",{tool:pending.tool,arguments:pending.args()}))}}]},finish_reason:"tool_calls"}]})}\n\ndata: [DONE]\n\n`);
        return;
      }
      dudeChildResults = Object.fromEntries(steps.map(step => [step.id, singleGatewayResult(result(step.id).content)]));
      response.writeHead(200, {"Content-Type":"text/event-stream"});
      response.end(`data: ${JSON.stringify({choices:[{index:0,delta:{role:"assistant",content:"dude-child-complete"},finish_reason:"stop"}]})}\n\ndata: [DONE]\n\n`);
      return;
    }
    if (payload.stream && !summaryRequest && scenario === "acceptance-dude-identity") {
      const saved = payload.messages?.find((message: {role:string;tool_call_id?:string}) => message.role === "tool" && message.tool_call_id === "dude-identity-update");
      response.writeHead(200, {"Content-Type":"text/event-stream"});
      if (!saved) {
        response.end(`data: ${JSON.stringify({choices:[{index:0,delta:{tool_calls:[{index:0,id:"dude-identity-update",type:"function",function:{name:"tool_execute",arguments:JSON.stringify(gatewayArguments("tool_execute",{tool:"dude_soul_update",arguments:{name:"Dude Acceptance"}}))}}]},finish_reason:"tool_calls"}]})}\n\ndata: [DONE]\n\n`);
      } else {
        dudeIdentityResult = singleGatewayResult(saved.content);
        response.end(`data: ${JSON.stringify({choices:[{index:0,delta:{role:"assistant",content:"dude-identity-complete"},finish_reason:"stop"}]})}\n\ndata: [DONE]\n\n`);
      }
      return;
    }
    if (payload.stream && !summaryRequest && scenario === "acceptance-background-work") {
      const toolResult = (id: string) => payload.messages?.find((message: {role:string;tool_call_id?:string}) => message.role === "tool" && message.tool_call_id === id);
      const recorded = toolResult("dude-result-call");
      const change = toolResult("dude-change-call");
      const notified = toolResult("dude-notify-call");
      if (!recorded || !change || !notified) {
        const changeId = change ? JSON.parse(singleGatewayResult(change.content)).changeId : null;
        const inner = !recorded || !change ? "dude_work_change" : "dude_notify";
        const args = !recorded ? {operation:"recordResult", taskCode:backgroundTaskCode, result:"dude-background-finished"} : !change ? { operation: "record", taskCode: backgroundTaskCode, changeKey:"acceptance-background", kind:"Progress", summary:"dude-background-finished", evidence:["acceptance:background-run"] } : {changeId,title:"Background result",body:"dude-background-finished"};
        const id = !recorded ? "dude-result-call" : !change ? "dude-change-call" : "dude-notify-call";
        response.writeHead(200, {"Content-Type":"text/event-stream"});
        response.end(`data: ${JSON.stringify({choices:[{index:0,delta:{tool_calls:[{index:0,id,type:"function",function:{name:"tool_execute",arguments:JSON.stringify(gatewayArguments("tool_execute",{tool:inner,arguments:args}))}}]},finish_reason:"tool_calls"}]})}\n\ndata: [DONE]\n\n`);
        return;
      }
    }
    if (payload.stream && !summaryRequest && scenario === "acceptance-foreground-hold") {
      foregroundStarted = true;
      await new Promise<void>(resolveGate => { releaseForeground = resolveGate; });
    }
    const summary = input.includes("dude-background-finished")
      ? "The Dude completed task acceptance-background-work with result dude-background-finished."
      : input.includes("dude-foreground-finished")
        ? "The user discussed acceptance-foreground-hold. The Dude replied dude-foreground-finished."
        : summaryRequest?.previous_summary || "No completed acceptance task was recorded.";
    // These fixture summaries are ASCII, so slicing characters also enforces the
    // framework's UTF-8 byte bound without splitting a multibyte character.
    const answer = summaryRequest
      ? JSON.stringify({ summary: summary.slice(0, summaryRequest.summary_max_utf8_bytes) })
      : scenario === "acceptance-background-work" ? "dude-background-finished" : scenario === "acceptance-foreground-hold" ? "dude-foreground-finished" : scenario === "acceptance-askuser" ? "dude-askuser-resumed" : "Dude acceptance summary";
    const usage = { prompt_tokens: 120, completion_tokens: 20, total_tokens: 140 };
    if (payload.stream) {
      response.writeHead(200, { "Content-Type": "text/event-stream" });
      response.write(`data: ${JSON.stringify({ id: "dude-acceptance", choices: [{ index: 0, delta: { role: "assistant", content: answer }, finish_reason: null }] })}\n\n`);
      response.end(`data: ${JSON.stringify({ id: "dude-acceptance", choices: [{ index: 0, delta: {}, finish_reason: "stop" }], usage })}\n\ndata: [DONE]\n\n`);
    } else {
      response.writeHead(200, { "Content-Type": "application/json" });
      response.end(JSON.stringify({ id: "dude-acceptance", choices: [{ index: 0, message: { role: "assistant", content: answer }, finish_reason: "stop" }], usage }));
    }
  });
  await new Promise<void>(ready => provider!.listen(0, "127.0.0.1", ready));
  const providerAddress = provider.address();
  if (!providerAddress || typeof providerAddress === "string") throw new Error("provider address missing");
  const port = await new Promise<number>((resolvePort, reject) => {
    const server = createServer(); server.on("error", reject); server.listen(0, "127.0.0.1", () => {
      const address = server.address(); if (!address || typeof address === "string") throw new Error("acceptance port missing");
      server.close(() => resolvePort(address.port));
    });
  });
  api = `http://127.0.0.1:${port}`;
  backendLogs = "";
  const backendBinary = process.env.TRON_ACCEPTANCE_SERVER;
  backend = spawnManagedProcess(backendBinary || "rtk", backendBinary ? [] : ["make", "be-ui-acceptance"], {
    cwd: resolve(fileURLToPath(import.meta.url), "../../.."),
    env: { ...process.env, CARGO_BUILD_JOBS: "2", TRON_ACCEPTANCE_DIRECTORY: directory, TRON_ACCEPTANCE_PORT: String(port), ...(personaiFixtureEnabled ? {TRON_ACCEPTANCE_PERSONAI_URL:`http://127.0.0.1:${providerAddress.port}`} : {}) }, stdio: "pipe",
  });
  backend.stdout.on("data", data => backendLogs += data.toString()); backend.stderr.on("data", data => backendLogs += data.toString());
  const deadline = Date.now() + 270_000;
  let ready = false;
  while (Date.now() < deadline) {
    if (backend.exitCode !== null) throw new Error(backendLogs);
    try { ready = (await fetch(`${api}/api/v1/health`)).ok; } catch { /* backend starting */ }
    if (ready) break;
    await new Promise(resolveDelay => setTimeout(resolveDelay, 250));
  }
  if (!ready) throw new Error(backendLogs);
  // Configure an executable loop through the same public management API as an
  // existing user. This fixture tests parallel execution, not bootstrap policy.

  const main = (await request("GET", "/api/v1/dudes")).find((dude: { isMain: boolean }) => dude.isMain);
  if (!main) throw new Error("default main Dude missing");
  await request("POST", "/api/v1/model-providers/configured", {
    displayName: "Dude Acceptance", protocol: "OpenAiChatCompletions",
    baseUrl: `http://127.0.0.1:${providerAddress.port}/v1`, apiKey: "synthetic-acceptance", requestHeaders: {}, enabled: true,
    models: [{ modelType: "Text", modelCode: null, modelId: "acceptance-model", name: "Dude Acceptance", contextWindowTokens: 128000,
      maxOutputTokens: 1024, reasoning: false, supportsDeveloperRole: false, inputCapabilities: ["Text"], enabled: true }],
  });
  const embeddingProvider = await request("POST", "/api/v1/model-providers/configured", {
    displayName:"Dude Acceptance Embedding", protocol:"OpenAiChatCompletions", baseUrl:`http://127.0.0.1:${providerAddress.port}/v1`, apiKey:"synthetic-acceptance", requestHeaders:{}, enabled:true,
    models:[{modelType:"TextEmbedding",modelCode:null,modelId:"dude-acceptance-embedding",name:"Dude Acceptance Embedding",dimension:1024,contextWindowTokens:null,maxOutputTokens:null,reasoning:false,supportsDeveloperRole:false,inputCapabilities:["Text"],enabled:true}],
  });
  await request("PUT", "/api/v1/rag/corpora/memory-card/settings", {fulltext:true,vector:true,graph:false,embeddingModelCode:embeddingProvider.models[0].modelCode,embeddingDimension:1024,rerankModelCode:null,graphModelCode:null,imageDescriptionModelCode:null});
  frontend = await startResourceUiFrontend("/dudes", {
    VITE_TRON_DATA_SOURCE: "mock", VITE_TRON_CHAT_DATA_SOURCE: "api", VITE_TRON_MODEL_DATA_SOURCE: "api", VITE_TRON_AGENT_DATA_SOURCE: "api",
    VITE_TRON_API_BASE_URL: "", TRON_FRONTEND_API_PROXY_TARGET: api,
  });
});

test.afterAll(async () => {
  releaseForeground();
  await stopResourceUiFrontend(frontend); await stopManagedProcess(backend);
  await new Promise<void>(closed => provider ? provider.close(() => closed()) : closed());
  if (directory) await rm(directory, { recursive: true, force: true });
});

test("main Dude answers AskUser and continues chatting without selecting a workspace", async ({ page }) => {
  test.setTimeout(120_000);
  await seedResourceUiShell(page, "en-US"); await routeAcceptanceProfile(page);
  await page.goto(`${frontend!.baseUrl}/notifications`);
  const resolved = page.waitForResponse(response => response.url().endsWith("/api/v1/dudes/main/session") && response.request().method() === "POST");
  await page.getByRole("button", { name: "Platform main Dude conversation", exact: true }).click();
  const session = (await (await resolved).json()).data;
  expect(session.workspaceCode).toBeNull();
  const input = page.locator('[data-theme-part="chat-input"]');
  await input.fill("acceptance-askuser: ask me to choose a target");
  await page.getByRole("button", { name: "Send", exact: true }).click();
  await page.getByText("Acceptance Local", { exact: true }).click({ timeout: 60_000 });
  await expect(page.getByRole("radio", { name: /Acceptance Local/ })).toBeChecked();
  const resumed = page.waitForResponse(response => response.request().method() === "POST"
    && response.url().includes(`/api/v1/chats/sessions/${session.sessionCode}/user-input-requests/`));
  await page.getByRole("button", { name: "Confirm", exact: true }).click();
  const response = await resumed;
  expect(response.request().headers()["x-dude-code"]).toBe(session.ownerDudeCode);
  expect(response.status()).toBe(200);
  expect(await response.text()).not.toContain("RUN_ERROR");
  await expect(page.getByRole("region", { name: "Chat transcript", exact: true })).toContainText("dude-askuser-resumed");
  await input.fill("Continue after answering");
  const continued = page.waitForResponse(response => response.request().method() === "POST"
    && new URL(response.url()).pathname === `/api/v1/chats/sessions/${session.sessionCode}/messages/stream`);
  await page.getByRole("button", { name: "Send", exact: true }).click();
  const continuation = await continued;
  expect(continuation.request().headers()["x-dude-code"]).toBe(session.ownerDudeCode);
  expect(continuation.status()).toBe(200);
  expect(await continuation.text()).not.toContain("RUN_ERROR");
  expect(evidence.get(page)!.messages.filter(message => message.includes("/api/v1/apps/workspaces//options"))).toEqual([]);
  expect(evidence.get(page)!.errors).toEqual([]);
});

test("Dude runs its memory and task tools without a workspace", async ({ page }) => {
  test.setTimeout(180_000);
  await seedResourceUiShell(page, "en-US"); await routeAcceptanceProfile(page);
  await page.goto(`${frontend!.baseUrl}/notifications`);
  await page.getByRole("button", { name:"Platform main Dude conversation", exact:true }).click();
  const input = page.locator('[data-theme-part="chat-input"]');
  await input.fill("acceptance-dude-tools: manage this test memory and task");
  const response = page.waitForResponse(result => result.request().method() === "POST" && result.url().includes("/messages/stream"));
  await page.getByRole("button", {name:"Send",exact:true}).click();
  const stream = await (await response).text();
  expect(stream).not.toContain("RUN_ERROR");
  await expect(page.getByRole("region", {name:"Chat transcript",exact:true})).toContainText("dude-tools-complete", {timeout:120_000});
  for (const tool of ["dude_memory_save","dude_memory_search","dude_memory_delete","dude_task_create","dude_task_page","dude_task_update","dude_task_delete","dude_task_restore","dude_soul_read","dude_todo_update"]) {
    expect(stream, `expected ${tool} in the real conversation`).toContain(tool);
  }
  expect(stream).not.toContain('"isError":true');
  for (const [tool, status] of Object.entries(dudeToolStatuses)) {
    expect(status.status, tool).toBe("Completed");
    expect(status.isError, tool).toBe(false);
  }
  expect(Object.keys(dudeToolStatuses)).toHaveLength(10);
  expect(dudeToolResults["dude-memory-search"]).toContain("Dude acceptance memory");
  expect(dudeToolResults["dude-task-page"]).toContain("Dude acceptance task");
  expect(dudeToolResults["dude-session-todo"]).toContain("Dude session todo");
  const todoResult = JSON.parse(dudeToolResults["dude-session-todo"]);
  const todoResponse = await fetch(`${api}/api/v1/chats/sessions/get-todos?sessionCode=${encodeURIComponent(todoResult.sessionCode)}`, {
    headers: { Authorization: "Bearer profile-test-token", "X-Dude-Code": "main-dude" },
  });
  expect(todoResponse.ok, await todoResponse.clone().text()).toBe(true);
  const todos = (await todoResponse.json()).data;
  expect(todos.some((item: { content: string }) => item.content === "Dude session todo")).toBe(true);
  expect(evidence.get(page)!.errors).toEqual([]);
});

test("Dude uses its assigned skills and web reader without a workspace", async ({page}) => {
  test.setTimeout(180_000);
  await request("PUT", "/api/v1/agents/registry/internal/main-dude/skills", {skillNames:["dude-ui-assignment"]});
  await seedResourceUiShell(page, "en-US"); await routeAcceptanceProfile(page);
  await page.goto(`${frontend!.baseUrl}/notifications`);
  await page.getByRole("button", {name:"Platform main Dude conversation", exact:true}).click();
  await page.locator('[data-theme-part="chat-input"]').fill("acceptance-dude-shared: use assigned skill and read a page");
  await page.getByRole("button", {name:"Send",exact:true}).click();
  await expect(page.getByRole("region", {name:"Chat transcript",exact:true})).toContainText("dude-shared-complete", {timeout:120_000});
  for (const [tool, result] of Object.entries(dudeSharedResults)) {
    expect(result.status, tool).toBe("Completed");
    expect(result.isError, tool).toBe(false);
  }
  expect(Object.keys(dudeSharedResults)).toHaveLength(8);
  expect(dudeSharedResults.skill_read_resource.content).toContain("Dude acceptance skill resource");
  expect(dudeSharedResults.web_read.content).toContain("Dude acceptance web content");
  expect(dudeSharedResults.media_read_pdf.content).toContain("Dude acceptance PDF text");
});

test("Dude executes Person-AI tools without a workspace", async ({page}) => {
  test.skip(!personaiFixtureEnabled, "requires the isolated local Person-AI fixture");
  test.setTimeout(180_000);
  await seedResourceUiShell(page, "en-US"); await routeAcceptanceProfile(page);
  await page.goto(`${frontend!.baseUrl}/notifications`);
  await page.getByRole("button", {name:"Platform main Dude conversation", exact:true}).click();
  await page.locator('[data-theme-part="chat-input"]').fill("acceptance-dude-personai: exercise remote tools");
  await page.getByRole("button", {name:"Send",exact:true}).click();
  await expect(page.getByRole("region", {name:"Chat transcript",exact:true})).toContainText("dude-personai-complete", {timeout:120_000});
  for (const [tool, result] of Object.entries(dudePersonAiResults)) {
    expect(result.status, tool).toBe("Completed"); expect(result.isError, tool).toBe(false);
  }
  expect(Object.keys(dudePersonAiResults)).toHaveLength(5);
  expect(dudePersonAiResults.web_search.content).toContain("Dude search result");
  expect(dudePersonAiResults.personai_query_image.content).toContain("https://example.invalid/image.png");
  for (const path of ["/api/v1/search", "/api/v1/images/generations", "/api/v1/multimedia/tasks/image-task", "/api/v1/aigc/detections/text", "/api/v1/aigc/detections/image"]) {
    expect(personaiRequests.find(item => item.path.startsWith(path)), path).toMatchObject({productCode:"tron",authorized:true});
  }
});

test("Dude publishes an interactive surface without a workspace", async ({page}) => {
  test.setTimeout(180_000);
  await seedResourceUiShell(page, "en-US"); await routeAcceptanceProfile(page);
  await page.goto(`${frontend!.baseUrl}/notifications`);
  await page.getByRole("button", {name:"Platform main Dude conversation", exact:true}).click();
  await page.locator('[data-theme-part="chat-input"]').fill("acceptance-dude-a2ui: display a report and request input");
  await page.getByRole("button", {name:"Send",exact:true}).click();
  await expect(page.getByText("Dude acceptance report", {exact:true})).toBeVisible({timeout:30_000});
  await page.getByLabel("Dude acceptance name").fill("Dude result");
  await page.getByRole("button", {name:"Submit Dude acceptance"}).click();
  await expect(page.getByRole("region", {name:"Chat transcript",exact:true})).toContainText("dude-a2ui-complete", {timeout:120_000});
  expect(evidence.get(page)!.errors).toEqual([]);
});

test("Dude delegates to another agent and manages the thread without a workspace", async ({ page }) => {
  test.setTimeout(180_000);
  await seedResourceUiShell(page, "en-US"); await routeAcceptanceProfile(page);
  await page.goto(`${frontend!.baseUrl}/notifications`);
  await page.getByRole("button", { name:"Platform main Dude conversation", exact:true }).click();
  const input = page.locator('[data-theme-part="chat-input"]');
  await input.fill("acceptance-dude-delegation: delegate the isolated task");
  const response = page.waitForResponse(result => result.request().method() === "POST" && result.url().includes("/messages/stream"));
  await page.getByRole("button", {name:"Send",exact:true}).click();
  const stream = await (await response).text();
  expect(stream).not.toContain("RUN_ERROR");
  await expect(page.getByRole("region", {name:"Chat transcript",exact:true})).toContainText("dude-delegation-complete", {timeout:120_000});
  for (const tool of ["thread_create","thread_wait","thread_read","thread_send","thread_list","thread_close"]) {
    expect(stream, `expected ${tool} in the Dude conversation`).toContain(tool);
  }
  const created = JSON.parse(dudeDelegationResults["delegate-create"]);
  expect(created.session_id).toBeTruthy();
  const first = JSON.parse(dudeDelegationResults["delegate-wait"]);
  const followup = JSON.parse(dudeDelegationResults["delegate-wait-again"]);
  expect(first.status).toBe("Completed");
  expect(followup.status).toBe("Completed");
  expect(followup.run_id).not.toBe(first.run_id);
  expect(JSON.parse(dudeDelegationResults["delegate-send"]).session_id).toBe(created.session_id);
  const conversation = JSON.parse(dudeDelegationResults["delegate-read"]).conversation.messages;
  expect(conversation.some((message: { provenance?: { sender?: { kind?: string } }; content?: { text?: string }[] }) =>
    message.provenance?.sender?.kind === "DUDE" && message.content?.some(block => block.text === "Work on the isolated acceptance task"))).toBe(true);
  expect(conversation.some((message: { provenance?: { sender?: { kind?: string; id?: string } } }) =>
    message.provenance?.sender?.kind === "AGENT" && message.provenance.sender.id === "main-agent")).toBe(true);
  expect(dudeDelegationResults["delegate-list"]).toContain(created.session_id);
  expect(JSON.parse(dudeDelegationResults["delegate-close"]).closed).toBe(true);
  expect(evidence.get(page)!.errors).toEqual([]);
});

test("Dude launches and inspects a background child without a workspace", async ({ page }) => {
  test.setTimeout(180_000);
  await seedResourceUiShell(page, "en-US"); await routeAcceptanceProfile(page);
  await page.goto(`${frontend!.baseUrl}/notifications`);
  await page.getByRole("button", { name:"Platform main Dude conversation", exact:true }).click();
  await page.locator('[data-theme-part="chat-input"]').fill("acceptance-dude-child: launch the isolated child");
  const response = page.waitForResponse(result => result.request().method() === "POST" && result.url().includes("/messages/stream"));
  await page.getByRole("button", {name:"Send",exact:true}).click();
  const stream = await (await response).text();
  expect(stream).not.toContain("RUN_ERROR");
  await expect(page.getByRole("region", {name:"Chat transcript",exact:true})).toContainText("dude-child-complete", {timeout:120_000});
  for (const tool of ["agent_get","agent_spawn","child_wait","child_read","child_list"]) expect(stream).toContain(tool);
  expect(JSON.parse(dudeChildResults["child-agent-get"]).agentCode).toBe("main-agent");
  const childId = JSON.parse(dudeChildResults["child-spawn"]).child_run_id;
  expect(childId).toBeTruthy();
  const childResult = JSON.parse(dudeChildResults["child-wait"]);
  expect(childResult.child_run_id).toBe(childId);
  expect(childResult.status).toBe("Completed");
  const childHistory = JSON.parse(dudeChildResults["child-read"]);
  expect(childHistory.summary.child_run_id).toBe(childId);
  expect(childHistory.summary.agent_code).toBe("main-agent");
  expect(childHistory.conversation.messages.some((message: { provenance?: { sender?: { kind?: string } }; content?: { text?: string }[] }) =>
    message.provenance?.sender?.kind === "DUDE" && message.content?.some(block => block.text === "Inspect this isolated child task"))).toBe(true);
  expect(JSON.parse(dudeChildResults["child-list"]).children.some((item: { child_run_id: string }) => item.child_run_id === childId)).toBe(true);
  expect(evidence.get(page)!.errors).toEqual([]);
});

test("Dude updates its own identity when the user asks", async ({ page }) => {
  test.setTimeout(120_000);
  await seedResourceUiShell(page, "en-US"); await routeAcceptanceProfile(page);
  await page.goto(`${frontend!.baseUrl}/notifications`);
  await page.getByRole("button", {name:"Platform main Dude conversation",exact:true}).click();
  await page.locator('[data-theme-part="chat-input"]').fill("acceptance-dude-identity: please change your name to Dude Acceptance");
  const response = page.waitForResponse(result => result.request().method() === "POST" && result.url().includes("/messages/stream"));
  await page.getByRole("button", {name:"Send",exact:true}).click();
  const stream = await (await response).text();
  expect(stream).not.toContain("RUN_ERROR");
  expect(stream).toContain("dude_soul_update");
  await expect(page.getByRole("region", {name:"Chat transcript",exact:true})).toContainText("dude-identity-complete", {timeout:90_000});
  expect(JSON.parse(dudeIdentityResult).name).toBe("Dude Acceptance");
  const soul = await request("GET", "/api/v1/dudes/main-dude/soul");
  expect(soul.name).toBe("Dude Acceptance");
  expect(evidence.get(page)!.errors).toEqual([]);
});

test("robot opens the persistent platform conversation while task work runs in parallel", async ({ page }, testInfo) => {
  test.setTimeout(180_000);
  await page.setViewportSize({ width: 1440, height: 1000 });
  await seedResourceUiShell(page, "en-US"); await routeAcceptanceProfile(page);
  const errors = evidence.get(page)!.errors;
  // The robot belongs to the main application shell. Notifications provides
  // that shell without a second chat composer competing with the robot's one.
  await page.goto(`${frontend!.baseUrl}/notifications`);
  await expect(page.getByRole("button", { name: "Platform main Dude conversation", exact: true }),
    `Robot unavailable. Page errors: ${JSON.stringify(errors)}. Frontend: ${frontend!.logs()}`)
    .toBeVisible({ timeout: 10_000 });
  const resolveResponse = page.waitForResponse(response => response.url().endsWith("/api/v1/dudes/main/session") && response.request().method() === "POST");
  await page.getByRole("button", { name: "Platform main Dude conversation", exact: true }).click();
  const session = (await (await resolveResponse).json()).data;
  expect(session.workspaceCode).toBeNull(); expect(session.sessionCode).toBeTruthy();
  const input = page.locator('[data-theme-part="chat-input"]');
  let backgroundSession = "";
  await input.fill("acceptance-foreground-hold: continue discussing this with me");
  const sentRequest = page.waitForRequest(outgoing => outgoing.method() === "POST"
    && new URL(outgoing.url()).pathname === `/api/v1/chats/sessions/${session.sessionCode}/messages/stream`);
  // History can still be loading while the composer accepts a draft. The Send
  // button waits for session readiness; an immediate Enter can be ignored then.
  await page.getByRole("button", { name: "Send", exact: true }).click();
  const sent = await sentRequest;
  const traceparent = sent.headers()["traceparent"];
  expect(traceparent).toMatch(/^00-[0-9a-f]{32}-[0-9a-f]{16}-01$/);
  expect(sent.headers()["x-dude-code"]).toBe(session.ownerDudeCode);
  const [, foregroundTrace, browserSpanId] = traceparent.split("-");
  try {
    // Native startup includes persisted context and resource preparation. The
    // controlled provider stays held throughout background task completion.
    await expect.poll(() => foregroundStarted, { timeout: 60_000 }).toBe(true);
    const backgroundTask = await request("POST", "/api/v1/tasks", { workspaceCode: null, ownerDudeCode: session.ownerDudeCode, title: "acceptance-background-work", description: "acceptance-background-work: finish this while we are still chatting", dueAt: null, priority: "Medium", taskType: "General", sourceType: "Internal", assigneeUserCodes: [], agentCodes: [], appCodes: [] });
    backgroundTaskCode = backgroundTask.taskCode;
    await expect.poll(async () => {
      const events = await request("GET", `/api/v1/dudes/${session.ownerDudeCode}/events`);
      const completed = events.find((event: { source: string; status: string; result?: string }) => event.source === "TaskReminder" && event.status === "Completed" && event.result?.includes("dude-background-finished"));
      backgroundSession = completed?.sessionCode ?? "";
      return Boolean(completed);
    }, { timeout: 60_000 }).toBe(true);
    await expect(page.getByRole("region", { name: "Chat transcript", exact: true })).not.toContainText("dude-background-finished");
    await expect(page.getByRole("region", { name: "Chat transcript", exact: true })).not.toContainText("dude-foreground-finished");
  } finally { releaseForeground(); }
  await expect(page.getByRole("region", { name: "Chat transcript", exact: true })).toContainText("dude-foreground-finished");
  // Query actual local telemetry exported by the browser and native runtime.
  // A trace header alone would not prove that SDK-owned tasks kept the context.
  let foregroundDetail: any;
  await expect.poll(async () => {
    try {
      foregroundDetail = await request("GET", `/api/v1/observability/traces/${foregroundTrace}`);
      const spans = foregroundDetail.spans;
      return spans.some((span: any) => span.spanId === browserSpanId && span.attributes["service.name"] === "tron-frontend")
        && spans.some((span: any) => span.parentSpanId === browserSpanId && span.attributes["http.request.method"] === "POST")
        && spans.some((span: any) => span.name === "tron.session.run")
        && spans.some((span: any) => span.name === "agent.model");
    } catch { return false; }
  }, { timeout: 30_000 }).toBe(true);
  expect(foregroundDetail.trace.sessionCode).toBe(session.sessionCode);
  expect(foregroundDetail.trace.workspaceCode).toBe("");
  expect(foregroundDetail.spans.find((span: any) => span.name === "tron.session.run").attributes["tron.dude.code"]).toBe(session.ownerDudeCode);
  let backgroundDetail: any;
  await expect.poll(async () => {
    const traces = await request("POST", "/api/v1/observability/traces/query", { sessionCode: backgroundSession, current: 1, size: 15 });
    for (const trace of traces.records) {
      const detail = await request("GET", `/api/v1/observability/traces/${trace.traceId}`);
      if (detail.spans.some((span: any) => span.name === "tron.dude.execute") && detail.spans.some((span: any) => span.name === "agent.model")) {
        backgroundDetail = detail; return true;
      }
    }
    return false;
  }, { timeout: 30_000 }).toBe(true);
  expect(backgroundDetail.trace.traceId).not.toBe(foregroundTrace);
  const eventSpan = backgroundDetail.spans.find((span: any) => span.name === "tron.dude.execute");
  expect(eventSpan.parentSpanId).toBeNull();
  expect(eventSpan.attributes["tron.dude.event.source"]).toBe("TaskReminder");
  expect(eventSpan.attributes["tron.dude.code"]).toBe(session.ownerDudeCode);
  expect(backgroundDetail.spans.find((span: any) => span.name === "tron.session.run").parentSpanId).toBe(eventSpan.spanId);
  await writeFile(testInfo.outputPath("dude-traces.json"), JSON.stringify({ foreground: foregroundDetail, background: backgroundDetail }, null, 2));
  await page.getByRole("button", { name: "Platform main Dude conversation", exact: true }).click();
  const secondResolve = page.waitForResponse(response => response.url().endsWith("/api/v1/dudes/main/session") && response.request().method() === "POST");
  await page.getByRole("button", { name: "Platform main Dude conversation", exact: true }).click();
  expect((await (await secondResolve).json()).data.sessionCode).toBe(session.sessionCode);
  await expect(page.getByRole("region", { name: "Chat transcript", exact: true })).toContainText("dude-foreground-finished");
  expect(errors).toEqual([]);
  await page.screenshot({ animations: "disabled", path: testInfo.outputPath("platform-dude-parallel-work.png") });
  await page.getByRole("button", { name: "Platform main Dude conversation", exact: true }).click();
  await page.goto(`${frontend!.baseUrl}/notifications`);
  await page.getByRole("tab", {name:"All",exact:true}).click();
  const notice = page.locator("article").filter({ hasText: "dude-background-finished" });
  await notice.getByRole("button", { name: "Mark read", exact: true }).click();
  await expect(notice.getByRole("button", { name: "Mark unread", exact: true })).toBeVisible();
  await page.reload();
  await page.getByRole("tab", {name:"All",exact:true}).click();
  await expect(notice.getByRole("button", { name: "Mark unread", exact: true })).toBeVisible();
  await notice.click();
  await page.getByRole("dialog").getByRole("button", {name:"Open related content",exact:true}).click();
  await expect(page).toHaveURL(new RegExp(`task=${backgroundTaskCode}.*tab=tasks`));
  // Work-change notifications target the task; inspect that work session separately.
  await page.goto(`${frontend!.baseUrl}/dudes?dude=${session.ownerDudeCode}&session=${backgroundSession}&tab=history`);
  const workHistory = page.getByRole("region", { name: "Chat transcript", exact: true });
  await expect(workHistory).toContainText("dude-background-finished", {timeout:30_000});
  await expect(workHistory).not.toContainText("dude-foreground-finished");
  const traceLink = page.getByRole("button", { name: "View traces", exact: true });
  await expect(traceLink).toHaveAttribute("href", `/settings/observability/traces?category=Agent&sessionCode=${backgroundSession}`);
  expect(errors).toEqual([]);
  await page.screenshot({ animations: "disabled", path: testInfo.outputPath("dude-notification-work-history.png") });
  const traceQuery = page.waitForResponse(response => response.request().method() === "POST"
    && new URL(response.url()).pathname === "/api/v1/observability/traces/query"
    && response.request().postDataJSON()?.sessionCode === backgroundSession);
  await traceLink.click();
  const queryResponse = await traceQuery;
  expect(queryResponse.request().postDataJSON().workspaceCode).toBeNull();
  expect((await queryResponse.json()).data.records.some((trace: any) => trace.traceId === backgroundDetail.trace.traceId)).toBe(true);
  await expect(page.getByRole("button", { name: backgroundDetail.trace.traceId, exact: true })).toBeVisible();
  await page.getByRole("button", { name: backgroundDetail.trace.traceId, exact: true }).click();
  await expect(page.getByRole("button", { name: "tron.dude.execute", exact: true })).toBeVisible();
  const modelSpans = backgroundDetail.spans.filter((span: any) => span.name === "agent.model");
  expect(modelSpans.length).toBeGreaterThan(0);
  for (const model of new Set<string>(modelSpans.map((span: any) => span.attributes["gen_ai.request.model"]))) {
    await expect(page.getByRole("button", { name: model, exact: true })).toHaveCount(
      modelSpans.filter((span: any) => span.attributes["gen_ai.request.model"] === model).length,
    );
  }
  await page.getByRole("button", { name: "tron.dude.execute", exact: true }).click();
  await expect(page.getByText("tron.dude.event.source", { exact: true })).toBeVisible();
  await page.getByRole("tab", {name:"Waterfall", exact:true}).click();
  await expect(page.getByRole("img", { name: "Waterfall", exact: true }).locator("canvas")).toBeVisible();
  await page.screenshot({ animations: "disabled", path: testInfo.outputPath("dude-trace-explorer.png") });
  expect(errors).toEqual([]);
});

test("Dude creation stays in agent management with a clear consumption notice", async ({ page }, testInfo) => {
  await page.setViewportSize({ width: 1440, height: 1000 });
  await seedResourceUiShell(page, "en-US"); await routeAcceptanceProfile(page);
  await page.goto(`${frontend!.baseUrl}/settings/agents`);
  await expect(page.getByRole("region", {name:"Dude",exact:true}).getByRole("button", { name: "New", exact: true }),
    `Dude manager unavailable. Page errors: ${JSON.stringify(evidence.get(page)!.errors)}. Frontend: ${frontend!.logs()}`)
    .toBeVisible({ timeout: 10_000 });
  await page.getByRole("region", {name:"Dude",exact:true}).getByRole("button", { name: "New", exact: true }).click();
  const dialog = page.getByRole("dialog");
  await expect(dialog).toContainText(/points|credits|consumption/i);
  await expect(dialog).not.toContainText(/Points budget|Session strategy|Notification preferences|Event subscriptions/i);
  await expect(dialog.getByText("Group", { exact: true })).toHaveCount(0);
  await expect(dialog.getByRole("switch", {name:"Pause",exact:true})).toBeChecked();
  await page.screenshot({ animations: "disabled", path: testInfo.outputPath("dude-creation-consumption-notice.png") });
  await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
  await expect(page.getByRole("button", { name: /Set as main|Make main/i })).toHaveCount(0);
  const library = await request("POST", "/api/v1/knowledge-bases", { name: "Dude assignment acceptance", description: "" });
  const skills = page.waitForResponse(response => response.request().method() === "GET"
    && new URL(response.url()).pathname === "/api/v1/agents/registry/internal/main-dude/skills");
  const knowledge = page.waitForResponse(response => response.request().method() === "GET"
    && new URL(response.url()).pathname === "/api/v1/knowledge-bases/agent-grants/main-dude");
  await page.getByRole("region", {name:"Dude",exact:true}).getByRole("button", {name:/main-dude$/}).click();
  await dialog.getByRole("tab", {name:"Capability settings",exact:true}).click();
  expect((await skills).status()).toBe(200);
  expect((await knowledge).status()).toBe(200);
  await dialog.getByRole("tab", {name:"Scheduling settings",exact:true}).click();
  await expect(dialog.getByRole("spinbutton", { name: /Heartbeat/i })).toHaveValue("5");
  await dialog.getByRole("tab", {name:"Capability settings",exact:true}).click();
  await expect(dialog.getByRole("alert")).toHaveCount(0);
  await dialog.getByRole("button", { name: /^Select skills/ }).click();
  const skillDialog = page.getByRole("dialog").filter({ has: page.getByRole("button", { name: "Confirm selection", exact: true }) });
  const skillName = "dude-ui-assignment";
  const selectedSkill = skillDialog.getByRole("checkbox", { name: skillName, exact: true });
  await selectedSkill.check();
  await skillDialog.getByRole("button", { name: "Confirm selection", exact: true }).click();
  await dialog.getByRole("button", { name: /^Choose knowledge bases/ }).click();
  const knowledgeDialog = page.getByRole("dialog").filter({ has: page.getByRole("switch", { name: "Dude assignment acceptance", exact: true }) });
  await knowledgeDialog.getByRole("switch", { name: "Dude assignment acceptance", exact: true }).click();
  await knowledgeDialog.getByRole("button", { name: "Confirm", exact: true }).click();
  await dialog.getByRole("button", { name: "Save", exact: true }).click();
  await expect(dialog).toHaveCount(0);
  const installed = await request("GET", "/api/v1/agents/registry/internal/main-dude/skills");
  expect(installed.some((skill: any) => skill.skillName === skillName && skill.assigned)).toBe(true);
  const granted = await request("GET", "/api/v1/knowledge-bases/agent-grants/main-dude");
  expect(granted.some((item: any) => item.id === library.id && item.granted)).toBe(true);
  expect(evidence.get(page)!.errors).toEqual([]);
});
