import { gatewayArguments, singleGatewayResult } from "./support/gateway-stub";
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
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
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

const RAW_TOOL_NAMES = ["tool_search", "tool_execute", "command_exec", "command_exec_read", "command_exec_stop", "memory_save", "memory_search"];
const EXPECTED_TOOL_NAMES = ["tool_execute", "tool_search"];
const LONG_COMMAND = "sleep 12 && echo acc-long-done";
const PARALLEL_COMMANDS = ["echo acc-parallel-a", "echo acc-parallel-b"];
const memoryMarker = `ACC_MEMORY_MARKER_${Date.now()}`;
const savedMemoryText = `${memoryMarker} prefers concise updates`;
const TAGS = ["e2e-long-task", "e2e-memory", "e2e-memory-delete", "e2e-parallel", "e2e-todo", "e2e-workspace-task", "e2e-variables", "e2e-app", "e2e-mcp", "e2e-prompt", "e2e-automation", "e2e-resource", "e2e-platform-resource", "e2e-resource-connections", "e2e-files", "e2e-office", "e2e-media", "e2e-terminal", "e2e-attachment", "e2e-web", "e2e-command-control", "e2e-skills", "e2e-rag", "e2e-rag-graph", "e2e-rag-quality", "e2e-a2ui-display", "e2e-a2ui-form", "e2e-personai"];
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
const todoState = { callId: "" };
const workspaceTaskCalls = new Map<string, string>();
const variableCalls = new Map<string, string>();
const appCalls = new Map<string, string>();
const mcpCalls = new Map<string, string>();
const promptCalls = new Map<string, string>();
const automationCalls = new Map<string, string>();
const resourceCalls = new Map<string, string>();
const connectionCalls = new Map<string, string>();
const fileCalls = new Map<string, string>();
const officeCalls = new Map<string, string>();
const memoryDeleteCalls = new Map<string, string>();
const mediaCalls = new Map<string, string>();
const terminalCalls = new Map<string, string>();
const attachmentCalls = new Map<string, string>();
const webCalls = new Map<string, string>();
const commandControlCalls = new Map<string, string>();
const skillCalls = new Map<string, string>();
const ragCalls = new Map<string, string>();
const a2uiCalls = new Map<string, string>();
const personaiCalls = new Map<string, string>();
const personaiRequests: Array<{path:string;productCode:string | undefined;authorized:boolean}> = [];
const personaiFixtureEnabled = process.env.TRON_TEST_PERSONAI_REMOTE === "1";
let acceptanceLibraryId = "";
let acceptanceChatModelCode = "";
let acceptanceEmbeddingModelCode = "";
let slowQualityEmbeddings = false;
let acceptanceAttachmentCode = "";

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
    if (messages[index]?.role === "user" && TAGS.some(tag => stringifyContent(messages[index].content).includes(tag))) return index;
  }
  return -1;
}

function tagOfTurn(turn: StubMessage[]) {
  const users = turn.filter((message) => message.role === "user");
  const text = users.map((message) => stringifyContent(message.content)).join("\n");
  return TAGS.filter((tag) => text.includes(tag)).sort((a, b) => b.length - a.length)[0] ?? "";
}

function toolResultFor(turn: StubMessage[], callId: string) {
  const message = turn.find((entry) => entry.role === "tool" && entry.tool_call_id === callId);
  return message ? singleGatewayResult(stringifyContent(message.content)) : "";
}

function parseTaskId(text: string) {
  const match = text.match(/"taskId"\s*:\s*"([^"]+)"/);
  return match ? match[1] : "";
}

function longTaskDecision(turn: StubMessage[]): Decision {
  if (!longTaskState.execCallId) {
    return { calls: [{ tool: "command_exec", args: { command: LONG_COMMAND }, bind: (id) => { longTaskState.execCallId = id; } }] };
  }
  const execResult = toolResultFor(turn, longTaskState.execCallId);
  const taskId = parseTaskId(execResult);
  if (!taskId) return { text: "e2e-long-task-no-task-id" };
  const lastReadId = longTaskState.readCallIds[longTaskState.readCallIds.length - 1];
  const lastReadResult = lastReadId ? toolResultFor(turn, lastReadId) : "";
  if (!lastReadResult || (!lastReadResult.includes("acc-long-done") && longTaskState.reads < 3)) {
    longTaskState.reads += 1;
    return { calls: [{ tool: "command_exec_read", args: { taskId, waitMs: 10_000 }, bind: (id) => { longTaskState.readCallIds.push(id); } }] };
  }
  return { text: "e2e-long-task-done" };
}

function memoryDecision(turn: StubMessage[]): Decision {
  if (!memoryState.saveCallId) {
    return {
      calls: [{
        tool: "memory_save",
        args: { workspaceCode: "TRON_CORE", memoryType: "USER_PREFERENCE", text: savedMemoryText, title: "Acceptance preference", scope: "USER" },
        bind: (id) => { memoryState.saveCallId = id; },
      }],
    };
  }
  if (!toolResultFor(turn, memoryState.saveCallId)) return { text: "e2e-memory-save-pending" };
  if (!memoryState.searchCallId) {
    return {
      calls: [{ tool: "memory_search", args: { workspaceCode: "TRON_CORE", query: memoryMarker, scope: "USER", limit: 10 }, bind: (id) => { memoryState.searchCallId = id; } }],
    };
  }
  if (!toolResultFor(turn, memoryState.searchCallId)) return { text: "e2e-memory-search-pending" };
  return { text: "e2e-memory-done" };
}

function parallelDecision(turn: StubMessage[]): Decision {
  if (!parallelState.aCallId) {
    return {
      calls: [
        { tool: "command_exec", args: { command: PARALLEL_COMMANDS[0] }, bind: (id) => { parallelState.aCallId = id; } },
        { tool: "command_exec", args: { command: PARALLEL_COMMANDS[1] }, bind: (id) => { parallelState.bCallId = id; } },
      ],
    };
  }
  const first = toolResultFor(turn, parallelState.aCallId);
  const second = toolResultFor(turn, parallelState.bCallId);
  return { text: first && second ? "e2e-parallel-done" : "e2e-parallel-pending" };
}

function decide(turn: StubMessage[], tag: string): Decision {
  if (tag === "e2e-personai") {
    const steps = [
      {tool:"web_search",args:{query:"Rust",count:1}},
      {tool:"personai_generate_image",args:{prompt:"A small blue square"}},
      {tool:"personai_query_image",args:{task_id:"image-task"}},
      {tool:"personai_detect_text",args:{text:"Acceptance fixture text"}},
      {tool:"personai_detect_image",args:{image_url:"https://example.invalid/image.png"}},
    ];
    const next = steps.find(step => !personaiCalls.has(step.tool));
    if (next) return {calls:[{...next,bind:id => {personaiCalls.set(next.tool,id);}}]};
    return {text:toolResultFor(turn,personaiCalls.get("personai_detect_image")!) ? "e2e-personai-done" : "e2e-personai-pending"};
  }
  if (tag === "e2e-a2ui-display" || tag === "e2e-a2ui-form") {
    const form = tag === "e2e-a2ui-form";
    const surfaceId = form ? "acceptance-form" : "acceptance-report";
    const messages: unknown[] = [
      {version:"v0.9.1",createSurface:{surfaceId,catalogId:"https://eliteforge.dev/a2ui/v0.9.1/agent"}},
      {version:"v0.9.1",updateComponents:{surfaceId,components:form ? [
        {id:"root",component:"Column",children:["name","submit"]},
        {id:"name",component:"TextField",label:"Acceptance name",value:{path:"/response/name"}},
        {id:"submit",component:"Button",child:"submit-label",action:{event:{name:"submit",context:{response:{path:"/response"}}}}},
        {id:"submit-label",component:"Text",text:"Submit acceptance"},
      ] : [{id:"root",component:"Text",text:"Acceptance A2UI report"}]}},
    ];
    if (form) messages.push({version:"v0.9.1",updateDataModel:{surfaceId,value:{response:{name:""}}}});
    const tool = form ? "a2ui_request_input" : "a2ui_publish";
    if (!a2uiCalls.has(tag)) return {calls:[{tool,args:form ? {mode:"SubmitOnce",messages,responseSchema:{type:"object",required:["name"],properties:{name:{type:"string",minLength:1}},additionalProperties:false}} : {mode:"Display",messages},bind:id => {a2uiCalls.set(tag,id);}}]};
    return {text:toolResultFor(turn,a2uiCalls.get(tag)!) ? `${tag}-done` : `${tag}-pending`};
  }
  if (tag === "e2e-long-task") return longTaskDecision(turn);
  if (tag === "e2e-memory") return memoryDecision(turn);
  if (tag === "e2e-memory-delete") {
    const saved = memoryDeleteCalls.get("save");
    const memoryCode = saved && toolResultFor(turn,saved) ? JSON.parse(toolResultFor(turn,saved)).memoryCode : "";
    const steps = [
      {key:"save",tool:"memory_save",args:{workspaceCode:"TRON_CORE",scope:"USER",memoryType:"USER_PREFERENCE",text:"Acceptance memory for deletion",title:"Acceptance deletion"}},
      {key:"delete",tool:"memory_delete",args:{workspaceCode:"TRON_CORE",memoryCode,reason:"Acceptance cleanup"}},
      {key:"search",tool:"memory_search",args:{workspaceCode:"TRON_CORE",scope:"USER",query:"Acceptance memory for deletion",limit:10}},
    ];
    const next = steps.find(step => !memoryDeleteCalls.has(step.key));
    if (next) return {calls:[{tool:next.tool,args:next.args,bind:id => {memoryDeleteCalls.set(next.key,id);}}]};
    return {text:toolResultFor(turn,memoryDeleteCalls.get("search")!) ? "e2e-memory-delete-done" : "e2e-memory-delete-pending"};
  }
  if (tag === "e2e-parallel") return parallelDecision(turn);
  if (tag === "e2e-todo") {
    if (!todoState.callId) return { calls: [{ tool: "todo_update", args: { workspaceCode: "TRON_CORE", todos: [{ position: 1, content: "Acceptance task", status: "in_progress", priority: "high" }] }, bind: (id) => { todoState.callId = id; } }] };
    return { text: toolResultFor(turn, todoState.callId) ? "e2e-todo-done" : "e2e-todo-pending" };
  }
  if (tag === "e2e-workspace-task") {
    const created = workspaceTaskCalls.get("task_create");
    const taskCode = created && toolResultFor(turn, created) ? JSON.parse(toolResultFor(turn, created)).taskCode : "";
    const steps = [
      { tool: "workspace_get", args: {workspaceCode:"TRON_CORE"} },
      { tool: "workspace_page", args: {workspaceCode:"TRON_CORE"} },
      { tool: "task_create", args: {workspaceCode:"TRON_CORE", title:"Ordinary acceptance task"} },
      { tool: "task_page", args: {workspaceCode:"TRON_CORE", search:"Ordinary acceptance task"} },
      { tool: "task_update", args: {workspaceCode:"TRON_CORE", taskCode, title:"Updated ordinary task"} },
      { tool: "task_delete", args: {workspaceCode:"TRON_CORE", taskCode} },
      { tool: "task_restore", args: {workspaceCode:"TRON_CORE", taskCode} },
    ];
    const next = steps.find(step => !workspaceTaskCalls.has(step.tool));
    if (next) return { calls: [{ ...next, bind: id => { workspaceTaskCalls.set(next.tool, id); } }] };
    return { text: toolResultFor(turn, workspaceTaskCalls.get("task_restore")!) ? "e2e-workspace-task-done" : "e2e-workspace-task-pending" };
  }
  if (tag === "e2e-variables") {
    const steps = ["variable", "platform_variable"].flatMap(prefix => {
      const workspace = prefix === "variable" ? { workspaceCode:"TRON_CORE" } : {};
      const key = prefix === "variable" ? "ACCEPTANCE_WORKSPACE" : "ACCEPTANCE_PLATFORM";
      return [
        {tool:`${prefix}_create`, args:{...workspace,key,value:"fixture",isSensitive:false}},
        {tool:`${prefix}_page`, args:{...workspace,search:key}},
        {tool:`${prefix}_update`, args:{...workspace,key,description:"updated by acceptance"}},
        {tool:`${prefix}_delete`, args:{...workspace,key}},
      ];
    });
    const next = steps.find(step => !variableCalls.has(step.tool));
    if (next) return {calls:[{...next,bind:id => { variableCalls.set(next.tool,id); }}]};
    return {text:toolResultFor(turn,variableCalls.get("platform_variable_delete")!) ? "e2e-variables-done" : "e2e-variables-pending"};
  }
  if (tag === "e2e-app") {
    const created = appCalls.get("app_create");
    const appCode = created && toolResultFor(turn, created) ? JSON.parse(toolResultFor(turn, created)).appCode : "";
    const steps = [
      {tool:"app_create",args:{workspaceCode:"TRON_CORE",name:"Acceptance app",localDirectory:directory,runtimeRole:"frontend"}},
      {tool:"app_page",args:{workspaceCode:"TRON_CORE",search:"Acceptance app"}},
      {tool:"app_update",args:{workspaceCode:"TRON_CORE",appCode,name:"Updated acceptance app"}},
      {tool:"app_delete",args:{workspaceCode:"TRON_CORE",appCode}},
      {tool:"app_restore",args:{workspaceCode:"TRON_CORE",appCode}},
    ];
    const next = steps.find(step => !appCalls.has(step.tool));
    if (next) return {calls:[{...next,bind:id => { appCalls.set(next.tool,id); }}]};
    return {text:toolResultFor(turn,appCalls.get("app_restore")!) ? "e2e-app-done" : "e2e-app-pending"};
  }
  if (tag === "e2e-mcp") {
    const serverCode = "acceptance-mcp";
    const connection = {serverCode,name:"Acceptance MCP",transportKind:"streamable_http",url:"http://127.0.0.1:1/mcp",authKind:"none"};
    const steps = [
      {tool:"platform_mcp_create",args:connection},
      {tool:"platform_mcp_page",args:{search:serverCode}},
      {tool:"platform_mcp_get",args:{serverCode}},
      {tool:"platform_mcp_update",args:{...connection,name:"Updated acceptance MCP"}},
      {tool:"platform_mcp_delete",args:{serverCode}},
    ];
    const next = steps.find(step => !mcpCalls.has(step.tool));
    if (next) return {calls:[{...next,bind:id => { mcpCalls.set(next.tool,id); }}]};
    return {text:toolResultFor(turn,mcpCalls.get("platform_mcp_delete")!) ? "e2e-mcp-done" : "e2e-mcp-pending"};
  }
  if (tag === "e2e-prompt") {
    const created = promptCalls.get("platform_prompt_create");
    const promptCode = created && toolResultFor(turn,created) ? JSON.parse(toolResultFor(turn,created)).promptCode : "";
    const detail = promptCalls.get("platform_prompt_get");
    const version = detail && toolResultFor(turn,detail) ? JSON.parse(toolResultFor(turn,detail)).version : 0;
    const steps = [
      {tool:"platform_prompt_create",args:{displayName:"Acceptance prompt",category:"TEMPLATE",promptBody:"Hello",variables:[]}},
      {tool:"platform_prompt_page",args:{search:"Acceptance prompt"}},
      {tool:"platform_prompt_get",args:{promptCode}},
      {tool:"platform_prompt_update",args:{promptCode,expectedVersion:version,displayName:"Updated acceptance prompt",promptBody:"Hi",variables:[]}},
      {tool:"platform_prompt_delete",args:{promptCode}},
    ];
    const next = steps.find(step => !promptCalls.has(step.tool));
    if (next) return {calls:[{...next,bind:id => {promptCalls.set(next.tool,id);}}]};
    return {text:toolResultFor(turn,promptCalls.get("platform_prompt_delete")!) ? "e2e-prompt-done" : "e2e-prompt-pending"};
  }
  if (tag === "e2e-automation") {
    const created = automationCalls.get("automation_create");
    const automationCode = created && toolResultFor(turn,created) ? JSON.parse(toolResultFor(turn,created)).automationCode : "";
    const workspaceCode = "TRON_CORE";
    const steps = [
      {tool:"automation_create",args:{workspaceCode,name:"Acceptance automation",prompt:"Summarize the test",cronExpression:"0 9 * * *",status:"paused"}},
      {tool:"automation_page",args:{workspaceCode,search:"Acceptance automation"}},
      {tool:"automation_get",args:{workspaceCode,automationCode}},
      {tool:"automation_update",args:{workspaceCode,automationCode,name:"Updated acceptance automation"}},
      {tool:"automation_delete",args:{workspaceCode,automationCode}},
      {tool:"automation_restore",args:{workspaceCode,automationCode}},
      {tool:"automation_execute",args:{workspaceCode,automationCode}},
    ];
    const next = steps.find(step => !automationCalls.has(step.tool));
    if (next) return {calls:[{...next,bind:id => {automationCalls.set(next.tool,id);}}]};
    return {text:toolResultFor(turn,automationCalls.get("automation_execute")!) ? "e2e-automation-done" : "e2e-automation-pending"};
  }
  if (tag === "e2e-resource" || tag === "e2e-platform-resource") {
    const prefix = tag === "e2e-resource" ? "resource" : "platform_resource";
    const workspace = tag === "e2e-resource" ? {workspaceCode:"TRON_CORE"} : {};
    const resourceCode = "acceptance-resource";
    const steps = [
      {tool:"fs_write_text",args:{path:"acceptance-source.txt",content:"acceptance resource",mode:"create"}},
      {tool:`${prefix}_create`,args:{...workspace,resourceCode,kind:"File",name:"Acceptance resource",sourcePath:"acceptance-source.txt",...(tag === "e2e-resource" ? {allowInheritance:false} : {})}},
      {tool:`${prefix}_query`,args:{...workspace,search:resourceCode}},
      {tool:`${prefix}_get`,args:{...workspace,resourceCode}},
      {tool:`${prefix}_update`,args:{...workspace,resourceCode,name:"Updated acceptance resource"}},
      {tool:`${prefix}_binary`,args:{...workspace,resourceCode,action:"download",targetPath:"acceptance-downloaded.txt"}},
      {tool:`${prefix}_delete`,args:{...workspace,resourceCode}},
    ];
    const next = steps.find(step => !resourceCalls.has(step.tool));
    if (next) return {calls:[{...next,bind:id => {resourceCalls.set(next.tool,id);}}]};
    return {text:toolResultFor(turn,resourceCalls.get(`${prefix}_delete`)!) ? `${tag}-done` : `${tag}-pending`};
  }
  if (tag === "e2e-resource-connections") {
    const workspaceCode = "TRON_CORE";
    const steps = [
      {tool:"variable_create",args:{workspaceCode,key:"ACCEPTANCE_CREDENTIAL",value:"fixture",isSensitive:true}},
      {tool:"resource_data_source_create",args:{workspaceCode,resourceCode:"acceptance-mysql",kind:"MysqlConnection",name:"Acceptance MySQL",allowInheritance:false,attributes:{connectionMode:"SINGLE",endpoints:[{host:"mysql.invalid",port:3306}],database:"tron",credentials:{username:"test",password:{source:"DIRECT",value:"fixture-password"}},tls:{mode:"DISABLED"}}}},
      {tool:"resource_data_source_update",args:{workspaceCode,resourceCode:"acceptance-mysql",description:"Updated data source"}},
      {tool:"resource_server_create",args:{workspaceCode,serverCode:"acceptance-server",name:"Acceptance server",host:"server.invalid",port:22,allowInheritance:false,initialAccount:{accountCode:"acceptance-ops",name:"Operations",username:"ops",authenticationType:"Password",passwordVariableKey:"ACCEPTANCE_CREDENTIAL",allowInheritance:false}}},
      {tool:"resource_server_account_create",args:{workspaceCode,serverCode:"acceptance-server",account:{accountCode:"acceptance-deploy",name:"Deploy",username:"deploy",authenticationType:"PrivateKey",privateKeyVariableKey:"ACCEPTANCE_CREDENTIAL",allowInheritance:false}}},
    ];
    const next = steps.find(step => !connectionCalls.has(step.tool));
    if (next) return {calls:[{...next,bind:id => {connectionCalls.set(next.tool,id);}}]};
    return {text:toolResultFor(turn,connectionCalls.get("resource_server_account_create")!) ? "e2e-resource-connections-done" : "e2e-resource-connections-pending"};
  }
  if (tag === "e2e-files") {
    const read = fileCalls.get("fs_read_text");
    const version = read && toolResultFor(turn,read) ? JSON.parse(toolResultFor(turn,read)).version : "";
    const steps = [
      {tool:"fs_write_text",args:{path:"acceptance.txt",content:"needle\n",mode:"create"}},
      {tool:"fs_read_text",args:{path:"acceptance.txt"}},
      {tool:"fs_list",args:{path:"."}},
      {tool:"fs_stat",args:{path:"acceptance.txt"}},
      {tool:"search_grep",args:{pattern:"needle",glob:["*.txt"]}},
      {tool:"search_glob",args:{patterns:["*.txt"]}},
      {tool:"search_path",args:{patterns:["acceptance.txt"]}},
      {tool:"fs_apply_unified_patch",args:{patch:"--- acceptance.txt\n+++ acceptance.txt\n@@ -1 +1 @@\n-needle\n+patched\n",expectedVersions:{"acceptance.txt":version}}},
      {tool:"fs_apply_patch",args:{path:"acceptance.txt",action:"replace",target:"patched",content:"final"}},
    ];
    const next = steps.find(step => !fileCalls.has(step.tool));
    if (next) return {calls:[{...next,bind:id => {fileCalls.set(next.tool,id);}}]};
    return {text:toolResultFor(turn,fileCalls.get("fs_apply_patch")!) ? "e2e-files-done" : "e2e-files-pending"};
  }
  if (tag === "e2e-office") {
    const steps = [
      {key:"word-create",tool:"word_create",args:{path:"office-acceptance.docx",paragraphs:[{text:"Acceptance draft"}]}},
      {key:"word-read",tool:"word_read",args:{path:"office-acceptance.docx"}},
      {key:"word-replace",tool:"word_replace_text",args:{path:"office-acceptance.docx",outputPath:"office-final.docx",find:"draft",replace:"final",expectedCount:1}},
      {key:"word-read-final",tool:"word_read",args:{path:"office-final.docx"}},
      {key:"excel-write",tool:"excel_write",args:{path:"office-acceptance.xlsx",sheets:[{name:"Sheet1",cells:[{row:1,column:1,value:42},{row:2,column:1,formula:"SUM(A1:A1)"}]}]}},
      {key:"excel-read",tool:"excel_read",args:{path:"office-acceptance.xlsx",sheet:"Sheet1",rows:2,columns:1}},
    ];
    const next = steps.find(step => !officeCalls.has(step.key));
    if (next) return {calls:[{tool:next.tool,args:next.args,bind:id => {officeCalls.set(next.key,id);}}]};
    return {text:toolResultFor(turn,officeCalls.get("excel-read")!) ? "e2e-office-done" : "e2e-office-pending"};
  }
  if (tag === "e2e-media") {
    const steps = [
      {tool:"media_read_image",args:{path:join(directory,"acceptance-image.png")}},
      {tool:"media_read_pdf",args:{path:join(directory,"acceptance.pdf"),startPage:1,endPage:1}},
    ];
    const next = steps.find(step => !mediaCalls.has(step.tool));
    if (next) return {calls:[{...next,bind:id => {mediaCalls.set(next.tool,id);}}]};
    return {text:toolResultFor(turn,mediaCalls.get("media_read_pdf")!) ? "e2e-media-done" : "e2e-media-pending"};
  }
  if (tag === "e2e-terminal") {
    const started = terminalCalls.get("start");
    const terminalId = started && toolResultFor(turn,started) ? JSON.parse(toolResultFor(turn,started)).terminalId : "";
    const steps = [
      {key:"start",tool:"terminal_start",args:{command:"read answer; printf 'terminal-answer=%s\\n' \"$answer\"; sleep 30"}},
      {key:"list",tool:"terminal_list",args:{}},
      {key:"write",tool:"terminal_write",args:{terminalId,data:"acceptance-value\n"}},
      {key:"read",tool:"terminal_read",args:{terminalId}},
      {key:"stop",tool:"terminal_stop",args:{terminalId}},
    ];
    const next = steps.find(step => !terminalCalls.has(step.key));
    if (next) return {calls:[{tool:next.tool,args:next.args,bind:id => {terminalCalls.set(next.key,id);}}]};
    return {text:toolResultFor(turn,terminalCalls.get("stop")!) ? "e2e-terminal-done" : "e2e-terminal-pending"};
  }
  if (tag === "e2e-attachment") {
    const steps = [
      {tool:"chat_attachment_read_text",args:{attachmentCode:acceptanceAttachmentCode}},
      {tool:"chat_attachment_parse_json",args:{attachmentCode:acceptanceAttachmentCode,jsonPath:"/topics/1"}},
    ];
    const next = steps.find(step => !attachmentCalls.has(step.tool));
    if (next) return {calls:[{...next,bind:id => {attachmentCalls.set(next.tool,id);}}]};
    return {text:toolResultFor(turn,attachmentCalls.get("chat_attachment_parse_json")!) ? "e2e-attachment-done" : "e2e-attachment-pending"};
  }
  if (tag === "e2e-web") {
    const address = provider?.address();
    if (!address || typeof address === "string") throw new Error("web fixture address missing");
    if (!webCalls.has("read")) return {calls:[{tool:"web_read",args:{url:`http://127.0.0.1:${address.port}/acceptance-web`},bind:id => {webCalls.set("read",id);}}]};
    return {text:toolResultFor(turn,webCalls.get("read")!) ? "e2e-web-done" : "e2e-web-pending"};
  }
  if (tag === "e2e-command-control") {
    const started = commandControlCalls.get("start");
    const taskId = started && toolResultFor(turn,started) ? JSON.parse(toolResultFor(turn,started)).taskId : "";
    const steps = [
      {key:"start",tool:"command_exec",args:{command:"read answer; printf 'command-answer=%s\\n' \"$answer\"; sleep 30",interactive:true}},
      {key:"list",tool:"command_exec_list",args:{}},
      {key:"write",tool:"command_exec_write",args:{taskId,data:"acceptance-value\n",waitMs:1000}},
      {key:"read",tool:"command_exec_read",args:{taskId,waitMs:1000}},
      {key:"stop",tool:"command_exec_stop",args:{taskId}},
    ];
    const next = steps.find(step => !commandControlCalls.has(step.key));
    if (next) return {calls:[{tool:next.tool,args:next.args,bind:id => {commandControlCalls.set(next.key,id);}}]};
    return {text:toolResultFor(turn,commandControlCalls.get("stop")!) ? "e2e-command-control-done" : "e2e-command-control-pending"};
  }
  if (tag === "e2e-skills") {
    const steps = [
      {tool:"skill_page",args:{search:"acceptance-skill"}},
      {tool:"skill_view",args:{skillName:"acceptance-skill"}},
      {tool:"skill_load",args:{name:"acceptance-skill"}},
      {tool:"skill_read_resource",args:{name:"acceptance-skill",path:"references/details.md"}},
      {tool:"skill_diagnostics",args:{}},
    ];
    const next = steps.find(step => !skillCalls.has(step.tool));
    if (next) return {calls:[{...next,bind:id => {skillCalls.set(next.tool,id);}}]};
    return {text:toolResultFor(turn,skillCalls.get("skill_diagnostics")!) ? "e2e-skills-done" : "e2e-skills-pending"};
  }
  if (tag === "e2e-rag") {
    if (!ragCalls.has("search")) return {calls:[{tool:"rag_search",args:{workspaceCode:"TRON_CORE",query:"orchid acceptance evidence",corpusIds:[acceptanceLibraryId]},bind:id => {ragCalls.set("search",id);}}]};
    return {text:toolResultFor(turn,ragCalls.get("search")!) ? "e2e-rag-done" : "e2e-rag-pending"};
  }
  if (tag === "e2e-rag-graph") {
    const search = ragCalls.get("graph-search");
    const firstNode = search && toolResultFor(turn,search) ? JSON.parse(toolResultFor(turn,search)).nodes?.[0]?.id : "";
    const steps = [
      {key:"graph-search",tool:"rag_graph_search",args:{workspaceCode:"TRON_CORE",query:"Alpha",corpusIds:[acceptanceLibraryId]}},
      {key:"graph-expand",tool:"rag_graph_expand",args:{workspaceCode:"TRON_CORE",nodeIds:[firstNode],corpusIds:[acceptanceLibraryId],options:{incomingDepth:0,outgoingDepth:1}}},
    ];
    const next = steps.find(step => !ragCalls.has(step.key));
    if (next) return {calls:[{tool:next.tool,args:next.args,bind:id => {ragCalls.set(next.key,id);}}]};
    return {text:toolResultFor(turn,ragCalls.get("graph-expand")!) ? "e2e-rag-graph-done" : "e2e-rag-graph-pending"};
  }
  if (tag === "e2e-rag-quality") {
    const sourceCall = ragCalls.get("quality-sources");
    const sources = sourceCall && toolResultFor(turn,sourceCall) ? JSON.parse(toolResultFor(turn,sourceCall)) : null;
    const cases = (sources?.records ?? []).slice(0,8).map((record: {documentId:string;blockId:string;start:number;end:number}, index:number) => ({id:`quality-${index}`,query:`orchid${index}`,partition:index % 2 ? "holdout" : "training",expected:[{documentId:record.documentId,span:{blockId:record.blockId,start:record.start,end:record.end}}]}));
    const startCall = ragCalls.get("quality-start");
    const runId = startCall && toolResultFor(turn,startCall) ? JSON.parse(toolResultFor(turn,startCall)).id : "";
    const workspace = {workspaceCode:"TRON_CORE",libraryId:acceptanceLibraryId};
    const steps = [
      {key:"quality-sources",tool:"rag_quality_sources",args:workspace},
      {key:"quality-cases",tool:"rag_quality_cases",args:{...workspace,sourceFingerprint:sources?.sourceFingerprint,cases}},
      {key:"quality-start",tool:"rag_quality_start",args:{...workspace,requestId:"acceptance-quality",options:{topK:1,maxTrials:1}}},
      {key:"quality-status",tool:"rag_quality_status",args:{...workspace,runId}},
      {key:"quality-cancel",tool:"rag_quality_cancel",args:{...workspace,runId}},
      {key:"quality-status-after",tool:"rag_quality_status",args:{...workspace,runId,waitSeconds:10}},
    ];
    const next = steps.find(step => !ragCalls.has(step.key));
    if (next?.key === "quality-start") slowQualityEmbeddings = true;
    if (next) return {calls:[{tool:next.tool,args:next.args,bind:id => {ragCalls.set(next.key,id);}}]};
    return {text:toolResultFor(turn,ragCalls.get("quality-status-after")!) ? "e2e-rag-quality-done" : "e2e-rag-quality-pending"};
  }
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
  const skillDirectory = join(directory, "skills/.agents/skills/acceptance-skill");
  await mkdir(join(skillDirectory, "references"), {recursive:true});
  await writeFile(join(skillDirectory, "SKILL.md"), "---\nname: acceptance-skill\ndescription: Isolated skill acceptance\n---\nRead references/details.md for acceptance instructions.\n");
  await writeFile(join(skillDirectory, "references/details.md"), "Acceptance skill resource content.\n");
  await writeFile(join(directory, "acceptance-image.png"), Buffer.from("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAIAAACQd1PeAAAADElEQVR4nGP4z8AAAAMBAQDJ/pLvAAAAAElFTkSuQmCC", "base64"));
  const pageContent = "BT /F1 12 Tf 10 100 Td (Acceptance PDF text) Tj ET";
  const objects = [
    "<< /Type /Catalog /Pages 2 0 R >>",
    "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>",
    "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    `<< /Length ${Buffer.byteLength(pageContent)} >>\nstream\n${pageContent}\nendstream`,
  ];
  let pdf = "%PDF-1.4\n";
  const offsets = [0];
  objects.forEach((object, index) => { offsets.push(Buffer.byteLength(pdf)); pdf += `${index + 1} 0 obj\n${object}\nendobj\n`; });
  const xref = Buffer.byteLength(pdf);
  pdf += `xref\n0 ${offsets.length}\n0000000000 65535 f \n${offsets.slice(1).map(offset => `${String(offset).padStart(10, "0")} 00000 n \n`).join("")}trailer\n<< /Size ${offsets.length} /Root 1 0 R >>\nstartxref\n${xref}\n%%EOF\n`;
  await writeFile(join(directory, "acceptance.pdf"), pdf);
  provider = createHttpServer(async (request, response) => {
    if (request.method === "GET" && request.url === "/acceptance-web") {
      response.writeHead(200, {"Content-Type":"text/html; charset=utf-8"});
      response.end("<html><head><title>Acceptance page</title><script>hidden script</script></head><body><p>Acceptance web content</p></body></html>");
      return;
    }
    const requestPath = request.url ?? "";
    if (personaiFixtureEnabled && requestPath.startsWith("/api/v1/")) {
      personaiRequests.push({path:requestPath,productCode:request.headers["x-product-code"] as string | undefined,authorized:request.headers.authorization === "Bearer acceptance-token"});
      let data: unknown = [];
      if (requestPath.startsWith("/api/v1/search")) data = {query:"Rust",count:1,results:[{title:"Rust",url:"https://rust-lang.org",snippet:"Acceptance search result",source:"fixture"}],providerCode:"fixture"};
      else if (requestPath.startsWith("/api/v1/images/generations")) data = {id:"image-task",media_type:0,status:0,progress:0};
      else if (requestPath.startsWith("/api/v1/multimedia/tasks/image-task")) data = {id:"image-task",media_type:0,status:2,progress:100,result_url:"https://example.invalid/image.png"};
      else if (requestPath.startsWith("/api/v1/aigc/detections/")) data = {id:"detection",detection_type:requestPath.endsWith("/image") ? 1 : 0,model_code:"detector",status:0,ai_confidence:0.9};
      else if (requestPath.includes("media_type=")) {const kind = Number(new URL(requestPath,"http://127.0.0.1").searchParams.get("media_type"));data = [{model_id:`media-${kind}`,model_code:`media-${kind}`,media_type:kind}];}
      response.writeHead(200, {"Content-Type":"application/json"});
      response.end(JSON.stringify({code:0,data}));
      return;
    }
    let body = ""; for await (const chunk of request) body += chunk;
    const payload = JSON.parse(body);
    // The memory RAG vector channel calls this stub's OpenAI-compatible
    // `/v1/embeddings` endpoint (the spec registers the stub as a platform
    // EMBEDDING model and points the built-in memory corpus at it). Embedding
    // calls carry no gateway `tools` array, so they must not enter the chat
    // request log the two-tool invariant inspects.
    if (requestPath.includes("embeddings")) {
      if (slowQualityEmbeddings) await new Promise(resolveDelay => setTimeout(resolveDelay, 10_000));
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
    if (!rawTools.length && JSON.stringify(messages).includes("从下述正文抽取实体和关系") && JSON.stringify(messages).includes("Alpha calls Beta.")) {
      writeJsonReply(response, JSON.stringify({nodes:[{ref:"alpha",name:"Alpha",type:"Service",evidence:"Alpha calls Beta."},{ref:"beta",name:"Beta",type:"Service",evidence:"Alpha calls Beta."}],edges:[{from:"alpha",to:"beta",relationship:"CALLS",evidence:"Alpha calls Beta."}]}), []);
      return;
    }
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
      return { id, name: "tool_execute", args: gatewayArguments("tool_execute", { tool: call.tool, arguments: call.args }) };
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
  const backendBinary = process.env.TRON_ACCEPTANCE_SERVER;
  backend = spawnManagedProcess(backendBinary || "make", backendBinary ? [] : ["be-ui-acceptance", `CARGO=${process.env.TRON_ACCEPTANCE_CARGO || "cargo"}`], {
    cwd: resolve(fileURLToPath(import.meta.url), "../../.."), env: { ...process.env, CARGO_NET_OFFLINE: process.env.CARGO_NET_OFFLINE || "true", CARGO_PROFILE_DEV_DEBUG: process.env.CARGO_PROFILE_DEV_DEBUG || "0", CARGO_INCREMENTAL: process.env.CARGO_INCREMENTAL || "0", CARGO_BUILD_JOBS: "2", TRON_ACCEPTANCE_DIRECTORY: directory, TRON_ACCEPTANCE_PORT: String(port), TRON_ACCEPTANCE_MOCK_RESOURCE_CONNECTIONS: "1", ...(personaiFixtureEnabled ? {TRON_ACCEPTANCE_PERSONAI_URL:`http://127.0.0.1:${providerAddress.port}`} : {}) }, stdio: "pipe",
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

  const chatModel = await post("/api/v1/model-providers/configured", {
    displayName: "Framework Acceptance", protocol: "OpenAiChatCompletions",
    baseUrl: `http://127.0.0.1:${providerAddress.port}/v1`, apiKey: "sk-acceptance", requestHeaders: {}, enabled: true,
    models: [{ modelType: "Text", modelCode: null, modelId: "acceptance-model", name: "Framework Acceptance", contextWindowTokens: 128000,
      maxOutputTokens: 1024, reasoning: false, supportsDeveloperRole: false, inputCapabilities: ["Text"], enabled: true }],
  });
  acceptanceChatModelCode = String(chatModel.models?.[0]?.modelCode ?? "");
  // Offline acceptance has no reachable Person-AI embedding provider, while the
  // seeded `memory-card` corpus pins `embeddingModelCode:
  // "PERSON_AI:ROUTE:EmbeddingAuto"` (tron-backend/migrations/0001_init.sql).
  // Register the local stub as a provider with an Embedding model and
  // repoint the corpus at it through the public management APIs, so the memory
  // save/search round trip really runs (and really persists) offline.
  const embeddingModel = await post("/api/v1/model-providers/configured", {
    displayName: "Framework Acceptance Embedding", protocol: "OpenAiChatCompletions",
    baseUrl: `http://127.0.0.1:${providerAddress.port}/v1`, apiKey: "sk-acceptance-embedding", requestHeaders: {}, enabled: true,
    models: [{ modelType: "TextEmbedding", dimension: 1024, modelCode: null, modelId: "acceptance-embedding", name: "Framework Acceptance Embedding", contextWindowTokens: null,
      maxOutputTokens: null, reasoning: false, supportsDeveloperRole: false, inputCapabilities: ["Text"], enabled: true }],
  });
  const embeddingModelCode = String(embeddingModel.models?.[0]?.modelCode ?? "");
  acceptanceEmbeddingModelCode = embeddingModelCode;
  if (!embeddingModelCode) throw new Error(`embedding model create returned no modelCode: ${JSON.stringify(embeddingModel)}`);
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

test.afterEach(async ({page}, info) => {
  if (info.status !== info.expectedStatus) {
    await writeFile(info.outputPath("failed-page.txt"), await page.locator("body").ariaSnapshot());
    await page.screenshot({path: info.outputPath("failed-page.png")});
  }
});

function resetScenarioState(tag: string) {
  modelRequests.length = 0;
  stubCalls.set(tag, []);
  turnSnapshots.delete(tag);
  toolResultSnapshots.delete(tag);
  if (tag === "e2e-long-task") { longTaskState.execCallId = ""; longTaskState.readCallIds = []; longTaskState.reads = 0; }
  if (tag === "e2e-memory") { memoryState.saveCallId = ""; memoryState.searchCallId = ""; }
  if (tag === "e2e-parallel") { parallelState.aCallId = ""; parallelState.bCallId = ""; }
  if (tag === "e2e-todo") todoState.callId = "";
  if (tag === "e2e-workspace-task") workspaceTaskCalls.clear();
  if (tag === "e2e-variables") variableCalls.clear();
  if (tag === "e2e-app") appCalls.clear();
  if (tag === "e2e-mcp") mcpCalls.clear();
  if (tag === "e2e-prompt") promptCalls.clear();
  if (tag === "e2e-automation") automationCalls.clear();
  if (tag === "e2e-resource" || tag === "e2e-platform-resource") resourceCalls.clear();
  if (tag === "e2e-resource-connections") connectionCalls.clear();
  if (tag === "e2e-files") fileCalls.clear();
}

test("ordinary chat executes Person-AI tools through the product host", async ({page}) => {
  test.skip(!personaiFixtureEnabled, "requires the isolated local Person-AI fixture");
  test.setTimeout(150_000);
  resetScenarioState("e2e-personai");
  personaiCalls.clear();
  personaiRequests.length = 0;
  await openChat(page);
  const transcript = page.getByRole("region", {name:"Chat transcript",exact:true});
  await page.locator('[data-theme-part="chat-input"]').fill("e2e-personai: test remote tools");
  await page.locator('[data-theme-part="chat-input"]').press("Enter");
  await expect(transcript).toContainText("e2e-personai-done", {timeout:120_000});
  const results = toolResultSnapshots.get("e2e-personai") ?? [];
  expect(results.map(result => result.tool)).toEqual(["web_search","personai_generate_image","personai_query_image","personai_detect_text","personai_detect_image"]);
  expect(results[0].content).toContain("Acceptance search result");
  expect(results[1].content).toContain("image-task");
  expect(results[2].content).toContain("https://example.invalid/image.png");
  for (const result of results.slice(3)) expect(result.content).toContain("ai_confidence");
  for (const path of ["/api/v1/search", "/api/v1/images/generations", "/api/v1/multimedia/tasks/image-task", "/api/v1/aigc/detections/text", "/api/v1/aigc/detections/image"]) {
    const request = personaiRequests.find(item => item.path.startsWith(path));
    expect(request, path).toMatchObject({productCode:"tron",authorized:true});
  }
  assertGatewayToolsInvariant();
});

test("ordinary chat publishes an A2UI surface", async ({ page }) => {
  test.setTimeout(120_000);
  resetScenarioState("e2e-a2ui-display");
  a2uiCalls.delete("e2e-a2ui-display");
  await openChat(page);
  const transcript = page.getByRole("region", { name: "Chat transcript", exact: true });
  const stream = page.waitForResponse(response => response.url().includes("/messages/stream") && response.request().method() === "POST");
  await page.locator('[data-theme-part="chat-input"]').fill("e2e-a2ui-display: show the report");
  await page.locator('[data-theme-part="chat-input"]').press("Enter");
  await expect(transcript).toContainText("e2e-a2ui-display-done", {timeout:90_000});
  const result = toolResultSnapshots.get("e2e-a2ui-display")?.find(item => item.tool === "a2ui_publish");
  expect(result?.content).toContain("A2UI surface published");
  expect((await (await stream).text())).toContain('"name":"agent.a2ui"');
  await expect(transcript.getByText("Acceptance A2UI report", {exact:true})).toBeVisible();
  await page.reload();
  await expect(page.getByRole("region", {name:"Chat transcript",exact:true}).getByText("Acceptance A2UI report", {exact:true})).toBeVisible({timeout:30_000});
});

test("ordinary chat submits an A2UI form and resumes", async ({ page }) => {
  test.setTimeout(120_000);
  resetScenarioState("e2e-a2ui-form");
  a2uiCalls.delete("e2e-a2ui-form");
  await openChat(page);
  const transcript = page.getByRole("region", { name: "Chat transcript", exact: true });
  await page.locator('[data-theme-part="chat-input"]').fill("e2e-a2ui-form: request a name");
  await page.locator('[data-theme-part="chat-input"]').press("Enter");
  const input = page.getByRole("textbox", {name:"Acceptance name",exact:true});
  await expect(input).toBeVisible({timeout:90_000});
  await input.fill("Alice Acceptance");
  await page.getByRole("button", {name:"Submit acceptance",exact:true}).click();
  await expect(transcript).toContainText("e2e-a2ui-form-done", {timeout:90_000});
  expect(JSON.stringify(modelRequests.at(-1)?.messages)).toContain("Alice Acceptance");
});

test("ordinary chat can update its session todo list", async ({ page }) => {
  test.setTimeout(120_000);
  resetScenarioState("e2e-todo");
  await openChat(page);
  const transcript = page.getByRole("region", { name: "Chat transcript", exact: true });
  await page.locator('[data-theme-part="chat-input"]').fill("e2e-todo: record the acceptance task");
  await page.locator('[data-theme-part="chat-input"]').press("Enter");
  await expect(transcript).toContainText("e2e-todo-done", { timeout: 90_000 });
  const result = (toolResultSnapshots.get("e2e-todo") ?? []).find(item => item.tool === "todo_update")?.content ?? "";
  expect(result).toContain("Acceptance task");
  const todoResult = JSON.parse(result);
  expect(todoResult.count).toBe(1);
  const persisted = await fetch(`${api}/api/v1/chats/sessions/get-todos?sessionCode=${encodeURIComponent(todoResult.sessionCode)}`, {
    headers: { Authorization: "Bearer profile-test-token", "X-Workspace-Code": "TRON_CORE" },
  });
  expect(persisted.ok).toBe(true);
  expect((await persisted.json()).data.some((item: { content: string }) => item.content === "Acceptance task")).toBe(true);
  assertGatewayToolsInvariant();
});

test("ordinary chat manages workspace tasks through the tool gateway", async ({ page }) => {
  test.setTimeout(150_000);
  resetScenarioState("e2e-workspace-task");
  await openChat(page);
  const transcript = page.getByRole("region", { name: "Chat transcript", exact: true });
  await page.locator('[data-theme-part="chat-input"]').fill("e2e-workspace-task: manage an acceptance task");
  await page.locator('[data-theme-part="chat-input"]').press("Enter");
  await expect(transcript).toContainText("e2e-workspace-task-done", { timeout: 120_000 });
  const results = toolResultSnapshots.get("e2e-workspace-task") ?? [];
  expect(results.map(item => item.tool)).toEqual(["workspace_get", "workspace_page", "task_create", "task_page", "task_update", "task_delete", "task_restore"]);
  for (const result of results) expect(() => JSON.parse(result.content), result.tool).not.toThrow();
  expect(results.find(item => item.tool === "task_page")?.content).toContain("Ordinary acceptance task");
  const created = JSON.parse(results.find(item => item.tool === "task_create")!.content);
  const restored = JSON.parse(results.find(item => item.tool === "task_restore")!.content);
  expect(restored.taskCode).toBe(created.taskCode);
  expect(restored.archivedAt).toBeNull();
  const response = await fetch(`${api}/api/v1/tasks/${encodeURIComponent(created.taskCode)}`, {headers:{Authorization:"Bearer profile-test-token"}});
  expect(response.ok).toBe(true);
  expect((await response.json()).data.title).toBe("Updated ordinary task");
  assertGatewayToolsInvariant();
});

test("ordinary chat manages workspace and platform variables", async ({ page }) => {
  test.setTimeout(150_000);
  resetScenarioState("e2e-variables");
  await openChat(page);
  const transcript = page.getByRole("region", {name:"Chat transcript",exact:true});
  await page.locator('[data-theme-part="chat-input"]').fill("e2e-variables: manage acceptance variables");
  await page.locator('[data-theme-part="chat-input"]').press("Enter");
  await expect(transcript).toContainText("e2e-variables-done", {timeout:120_000});
  const results = toolResultSnapshots.get("e2e-variables") ?? [];
  expect(results.map(item => item.tool)).toEqual(["variable_create","variable_page","variable_update","variable_delete","platform_variable_create","platform_variable_page","platform_variable_update","platform_variable_delete"]);
  for (const result of results) expect(() => JSON.parse(result.content), result.tool).not.toThrow();
  expect(results.find(item => item.tool === "variable_page")?.content).toContain("ACCEPTANCE_WORKSPACE");
  expect(results.find(item => item.tool === "platform_variable_page")?.content).toContain("ACCEPTANCE_PLATFORM");
  assertGatewayToolsInvariant();
});

test("ordinary chat manages workspace applications", async ({ page }) => {
  test.setTimeout(150_000);
  resetScenarioState("e2e-app");
  await openChat(page);
  const transcript = page.getByRole("region", {name:"Chat transcript",exact:true});
  await page.locator('[data-theme-part="chat-input"]').fill("e2e-app: manage an acceptance application");
  await page.locator('[data-theme-part="chat-input"]').press("Enter");
  await expect(transcript).toContainText("e2e-app-done", {timeout:120_000});
  const results = toolResultSnapshots.get("e2e-app") ?? [];
  expect(results.map(item => item.tool)).toEqual(["app_create","app_page","app_update","app_delete","app_restore"]);
  for (const result of results) expect(() => JSON.parse(result.content), result.tool).not.toThrow();
  expect(results.find(item => item.tool === "app_page")?.content).toContain("Acceptance app");
  assertGatewayToolsInvariant();
});

test("ordinary chat manages platform MCP configuration", async ({ page }) => {
  test.setTimeout(150_000);
  resetScenarioState("e2e-mcp");
  await openChat(page);
  const transcript = page.getByRole("region", {name:"Chat transcript",exact:true});
  await page.locator('[data-theme-part="chat-input"]').fill("e2e-mcp: manage the acceptance server");
  await page.locator('[data-theme-part="chat-input"]').press("Enter");
  await expect(transcript).toContainText("e2e-mcp-done", {timeout:120_000});
  const results = toolResultSnapshots.get("e2e-mcp") ?? [];
  expect(results.map(item => item.tool)).toEqual(["platform_mcp_create","platform_mcp_page","platform_mcp_get","platform_mcp_update","platform_mcp_delete"]);
  for (const result of results) expect(() => JSON.parse(result.content), result.tool).not.toThrow();
  expect(results.find(item => item.tool === "platform_mcp_get")?.content).toContain("Acceptance MCP");
  assertGatewayToolsInvariant();
});

test("ordinary chat manages platform prompts", async ({ page }) => {
  test.setTimeout(150_000);
  resetScenarioState("e2e-prompt");
  await openChat(page);
  const transcript = page.getByRole("region", {name:"Chat transcript",exact:true});
  await page.locator('[data-theme-part="chat-input"]').fill("e2e-prompt: manage the acceptance prompt");
  await page.locator('[data-theme-part="chat-input"]').press("Enter");
  await expect(transcript).toContainText("e2e-prompt-done", {timeout:120_000});
  const results = toolResultSnapshots.get("e2e-prompt") ?? [];
  expect(results.map(item => item.tool)).toEqual(["platform_prompt_create","platform_prompt_page","platform_prompt_get","platform_prompt_update","platform_prompt_delete"]);
  for (const result of results) expect(() => JSON.parse(result.content), result.tool).not.toThrow();
  expect(results.find(item => item.tool === "platform_prompt_get")?.content).toContain("Acceptance prompt");
  assertGatewayToolsInvariant();
});

test("ordinary chat manages workspace automations", async ({ page }) => {
  test.setTimeout(150_000);
  resetScenarioState("e2e-automation");
  await openChat(page);
  const transcript = page.getByRole("region", {name:"Chat transcript",exact:true});
  await page.locator('[data-theme-part="chat-input"]').fill("e2e-automation: manage the acceptance automation");
  await page.locator('[data-theme-part="chat-input"]').press("Enter");
  await expect(transcript).toContainText("e2e-automation-done", {timeout:120_000});
  const results = toolResultSnapshots.get("e2e-automation") ?? [];
  expect(results.map(item => item.tool)).toEqual(["automation_create","automation_page","automation_get","automation_update","automation_delete","automation_restore","automation_execute"]);
  for (const result of results) expect(() => JSON.parse(result.content), result.tool).not.toThrow();
  expect(results.find(item => item.tool === "automation_get")?.content).toContain("Acceptance automation");
  const executed = JSON.parse(results.find(item => item.tool === "automation_execute")!.content);
  expect(executed.lastExecutionCode).toBeTruthy();
  await expect.poll(async () => {
    const response = await fetch(`${api}/api/v1/automations/${executed.automationCode}/executions`, {headers:{Authorization:"Bearer profile-test-token"}});
    if (!response.ok) throw new Error(`execution lookup failed: ${response.status}`);
    const data = (await response.json()).data;
    return data.records.find((record:{executionCode:string}) => record.executionCode === executed.lastExecutionCode)?.status;
  }, {timeout:60_000}).toBe("Success");
  assertGatewayToolsInvariant();
});

for (const [tag, prefix] of [["e2e-resource", "resource"], ["e2e-platform-resource", "platform_resource"]] as const) {
  test(`ordinary chat manages ${prefix} files`, async ({ page }) => {
    test.setTimeout(150_000);
    resetScenarioState(tag);
    await openChat(page);
    const transcript = page.getByRole("region", {name:"Chat transcript",exact:true});
    await page.locator('[data-theme-part="chat-input"]').fill(`${tag}: manage the acceptance resource`);
    await page.locator('[data-theme-part="chat-input"]').press("Enter");
    await expect(transcript).toContainText(`${tag}-done`, {timeout:120_000});
    const results = toolResultSnapshots.get(tag) ?? [];
    expect(results.map(item => item.tool)).toEqual(["fs_write_text",`${prefix}_create`,`${prefix}_query`,`${prefix}_get`,`${prefix}_update`,`${prefix}_binary`,`${prefix}_delete`]);
    for (const result of results) expect(() => JSON.parse(result.content), `${result.tool}: ${result.content}`).not.toThrow();
    expect(results.find(item => item.tool === `${prefix}_get`)?.content).toContain("Acceptance resource");
    assertGatewayToolsInvariant();
  });
}

test("ordinary chat manages workspace connection resources", async ({ page }) => {
  test.setTimeout(150_000);
  resetScenarioState("e2e-resource-connections");
  await openChat(page);
  const transcript = page.getByRole("region", {name:"Chat transcript",exact:true});
  await page.locator('[data-theme-part="chat-input"]').fill("e2e-resource-connections: manage acceptance connections");
  await page.locator('[data-theme-part="chat-input"]').press("Enter");
  await expect(transcript).toContainText("e2e-resource-connections-done", {timeout:120_000});
  const results = toolResultSnapshots.get("e2e-resource-connections") ?? [];
  expect(results.map(item => item.tool)).toEqual(["variable_create","resource_data_source_create","resource_data_source_update","resource_server_create","resource_server_account_create"]);
  for (const result of results) expect(() => JSON.parse(result.content), `${result.tool}: ${result.content}`).not.toThrow();
  expect(results.find(item => item.tool === "resource_data_source_create")?.content).not.toContain("fixture-password");
  assertGatewayToolsInvariant();
});

test("ordinary chat reads, searches and patches local files", async ({ page }) => {
  test.setTimeout(150_000);
  resetScenarioState("e2e-files");
  await openChat(page);
  const transcript = page.getByRole("region", {name:"Chat transcript",exact:true});
  await page.locator('[data-theme-part="chat-input"]').fill("e2e-files: edit the acceptance file");
  await page.locator('[data-theme-part="chat-input"]').press("Enter");
  await expect(transcript).toContainText("e2e-files-done", {timeout:120_000});
  const results = toolResultSnapshots.get("e2e-files") ?? [];
  expect(results.map(item => item.tool)).toEqual(["fs_write_text","fs_read_text","fs_list","fs_stat","search_grep","search_glob","search_path","fs_apply_unified_patch","fs_apply_patch"]);
  for (const result of results) expect(() => JSON.parse(result.content), `${result.tool}: ${result.content}`).not.toThrow();
  expect(results.find(item => item.tool === "search_grep")?.content).toContain("acceptance.txt");
  expect(JSON.parse(results.find(item => item.tool === "fs_apply_unified_patch")!.content).applied).toBe(true);
  expect(JSON.parse(results.find(item => item.tool === "fs_apply_patch")!.content).replacements).toBe(1);
  assertGatewayToolsInvariant();
});

test("ordinary chat creates and edits Word and Excel documents", async ({ page }) => {
  test.setTimeout(150_000);
  resetScenarioState("e2e-office");
  officeCalls.clear();
  await openChat(page);
  const transcript = page.getByRole("region", {name:"Chat transcript",exact:true});
  await page.locator('[data-theme-part="chat-input"]').fill("e2e-office: create and edit the acceptance documents");
  await page.locator('[data-theme-part="chat-input"]').press("Enter");
  await expect(transcript).toContainText("e2e-office-done", {timeout:120_000});
  const results = toolResultSnapshots.get("e2e-office") ?? [];
  expect(results.map(item => item.tool)).toEqual(["word_create","word_read","word_replace_text","word_read","excel_write","excel_read"]);
  for (const result of results) expect(() => JSON.parse(result.content), `${result.tool}: ${result.content}`).not.toThrow();
  expect(JSON.parse(results[1].content).paragraphs[0].text).toBe("Acceptance draft");
  expect(JSON.parse(results[3].content).paragraphs[0].text).toBe("Acceptance final");
  const spreadsheet = JSON.parse(results[5].content);
  expect(spreadsheet.cells[0].value).toBe("42");
  expect(spreadsheet.cells[1].formula).toBe("SUM(A1:A1)");
  assertGatewayToolsInvariant();
});

test("ordinary chat deletes its saved memory", async ({ page }) => {
  test.setTimeout(150_000);
  resetScenarioState("e2e-memory-delete");
  memoryDeleteCalls.clear();
  await openChat(page);
  const transcript = page.getByRole("region", {name:"Chat transcript",exact:true});
  await page.locator('[data-theme-part="chat-input"]').fill("e2e-memory-delete: save and delete the acceptance memory");
  await page.locator('[data-theme-part="chat-input"]').press("Enter");
  await expect(transcript).toContainText("e2e-memory-delete-done", {timeout:120_000});
  const results = toolResultSnapshots.get("e2e-memory-delete") ?? [];
  expect(results.map(item => item.tool)).toEqual(["memory_save","memory_delete","memory_search"]);
  const saved = JSON.parse(results[0].content);
  const deleted = JSON.parse(results[1].content);
  const searched = JSON.parse(results[2].content);
  expect(saved.success).toBe(true);
  expect(deleted.success).toBe(true);
  expect(JSON.stringify(searched)).not.toContain(saved.memoryCode);
  assertGatewayToolsInvariant();
});

test("ordinary chat reads local image and PDF content", async ({ page }) => {
  test.setTimeout(150_000);
  resetScenarioState("e2e-media");
  mediaCalls.clear();
  await openChat(page);
  const transcript = page.getByRole("region", {name:"Chat transcript",exact:true});
  await page.locator('[data-theme-part="chat-input"]').fill("e2e-media: inspect the acceptance image and PDF");
  await page.locator('[data-theme-part="chat-input"]').press("Enter");
  await expect(transcript).toContainText("e2e-media-done", {timeout:120_000});
  const results = toolResultSnapshots.get("e2e-media") ?? [];
  expect(results.map(item => item.tool)).toEqual(["media_read_image","media_read_pdf"]);
  const imageBlocks = JSON.parse(results[0].content);
  const imageResult = JSON.parse(imageBlocks[0].text).results[0];
  expect(imageResult.isError).toBe(false);
  expect(imageResult.result.width).toBe(1);
  expect(imageResult.attachmentCount).toBe(1);
  expect(imageBlocks.some((block: {type: string; media?: {mime_type: string}}) => block.type === "media" && block.media?.mime_type === "image/png")).toBe(true);
  expect(JSON.parse(results[1].content).pages[0].text).toContain("Acceptance PDF text");
  assertGatewayToolsInvariant();
});

test("ordinary chat starts, lists, writes to, reads and stops a terminal", async ({ page }) => {
  test.setTimeout(150_000);
  resetScenarioState("e2e-terminal");
  terminalCalls.clear();
  await openChat(page);
  const transcript = page.getByRole("region", {name:"Chat transcript",exact:true});
  await page.locator('[data-theme-part="chat-input"]').fill("e2e-terminal: run an interactive acceptance command");
  await page.locator('[data-theme-part="chat-input"]').press("Enter");
  await expect(transcript).toContainText("e2e-terminal-done", {timeout:120_000});
  const results = toolResultSnapshots.get("e2e-terminal") ?? [];
  expect(results.map(item => item.tool)).toEqual(["terminal_start","terminal_list","terminal_write","terminal_read","terminal_stop"]);
  const started = JSON.parse(results[0].content);
  expect(started.terminalId).toBeTruthy();
  expect(JSON.parse(results[1].content).some((item: {terminalId:string}) => item.terminalId === started.terminalId)).toBe(true);
  expect(JSON.parse(results[2].content).written).toBe(true);
  expect(JSON.parse(results[3].content).output).toContain("terminal-answer=acceptance-value");
  expect(JSON.parse(results[4].content).stopped).toBe(true);
  assertGatewayToolsInvariant();
});

test("ordinary chat reads and parses an uploaded session attachment", async ({ page }) => {
  test.setTimeout(150_000);
  resetScenarioState("e2e-attachment");
  attachmentCalls.clear();
  acceptanceAttachmentCode = "";
  await openChat(page);
  const transcript = page.getByRole("region", {name:"Chat transcript",exact:true});
  const input = page.locator('[data-theme-part="chat-input"]');
  await input.fill("Initialize the acceptance session");
  await input.press("Enter");
  await expect(transcript).toContainText("e2e-aux-reply", {timeout:60_000});
  const uploaded = page.waitForResponse(response => response.request().method() === "POST" && /\/api\/v1\/chats\/sessions\/[^/]+\/attachments$/.test(new URL(response.url()).pathname));
  await page.locator('input[type="file"]').setInputFiles({name:"acceptance.json",mimeType:"application/json",buffer:Buffer.from('{"topics":["alpha","beta"]}')});
  const uploadResponse = await uploaded;
  expect(uploadResponse.ok()).toBe(true);
  const attachment = (await uploadResponse.json()).data;
  acceptanceAttachmentCode = attachment.resourceCode || attachment.id;
  expect(acceptanceAttachmentCode).toBeTruthy();
  await input.fill("e2e-attachment: read the uploaded JSON file");
  await input.press("Enter");
  await expect(transcript).toContainText("e2e-attachment-done", {timeout:120_000});
  const results = toolResultSnapshots.get("e2e-attachment") ?? [];
  expect(results.map(item => item.tool)).toEqual(["chat_attachment_read_text","chat_attachment_parse_json"]);
  expect(JSON.parse(results[0].content).content).toBe('{"topics":["alpha","beta"]}');
  expect(JSON.parse(results[1].content).selection.value).toBe("beta");
  assertGatewayToolsInvariant();
});

test("ordinary chat reads a web page through the tool gateway", async ({ page }) => {
  test.setTimeout(150_000);
  resetScenarioState("e2e-web");
  webCalls.clear();
  await openChat(page);
  const transcript = page.getByRole("region", {name:"Chat transcript",exact:true});
  await page.locator('[data-theme-part="chat-input"]').fill("e2e-web: read the acceptance page");
  await page.locator('[data-theme-part="chat-input"]').press("Enter");
  await expect(transcript).toContainText("e2e-web-done", {timeout:120_000});
  const results = toolResultSnapshots.get("e2e-web") ?? [];
  expect(results.map(item => item.tool)).toEqual(["web_read"]);
  const read = JSON.parse(results[0].content);
  expect(read.title).toBe("Acceptance page");
  expect(read.text).toContain("Acceptance web content");
  expect(read.text).not.toContain("hidden script");
  assertGatewayToolsInvariant();
});

test("ordinary chat lists, writes to and stops an interactive command", async ({ page }) => {
  test.setTimeout(150_000);
  resetScenarioState("e2e-command-control");
  commandControlCalls.clear();
  await openChat(page);
  const transcript = page.getByRole("region", {name:"Chat transcript",exact:true});
  await page.locator('[data-theme-part="chat-input"]').fill("e2e-command-control: interact with the acceptance command");
  await page.locator('[data-theme-part="chat-input"]').press("Enter");
  await expect(transcript).toContainText("e2e-command-control-done", {timeout:120_000});
  const results = toolResultSnapshots.get("e2e-command-control") ?? [];
  expect(results.map(item => item.tool)).toEqual(["command_exec","command_exec_list","command_exec_write","command_exec_read","command_exec_stop"]);
  const started = JSON.parse(results[0].content);
  expect(started.taskId).toBeTruthy();
  expect(results[1].content).toContain(started.taskId);
  expect(results[2].content + results[3].content).toContain("command-answer=acceptance-value");
  expect(JSON.parse(results[4].content).status).not.toBe("running");
  assertGatewayToolsInvariant();
});

test("ordinary chat inspects and loads an assigned skill and its resource", async ({ page }) => {
  test.setTimeout(150_000);
  resetScenarioState("e2e-skills");
  skillCalls.clear();
  await put("/api/v1/agents/registry/internal/main-agent/skills", {skillNames:["acceptance-skill"]});
  await openChat(page);
  const transcript = page.getByRole("region", {name:"Chat transcript",exact:true});
  await page.locator('[data-theme-part="chat-input"]').fill("e2e-skills: inspect the assigned acceptance skill");
  await page.locator('[data-theme-part="chat-input"]').press("Enter");
  await expect(transcript).toContainText("e2e-skills-done", {timeout:120_000});
  const results = toolResultSnapshots.get("e2e-skills") ?? [];
  expect(results.map(item => item.tool)).toEqual(["skill_page","skill_view","skill_load","skill_read_resource","skill_diagnostics"]);
  expect(results[0].content).toContain("acceptance-skill");
  expect(results[1].content).toContain("acceptance-skill");
  expect(results[2].content).toContain("references/details.md");
  expect(results[3].content).toContain("Acceptance skill resource content");
  expect(JSON.parse(results[4].content).diagnostics).toEqual([]);
  assertGatewayToolsInvariant();
});

test("ordinary chat searches an authorized indexed knowledge base", async ({ page }) => {
  test.setTimeout(150_000);
  resetScenarioState("e2e-rag");
  ragCalls.clear();
  const library = await post("/api/v1/knowledge-bases", {name:"Acceptance search library",description:"Isolated test",indexSettings:{fulltext:true,vector:false,graph:false,embeddingModelCode:null,rerankModelCode:null,graphModelCode:null}});
  acceptanceLibraryId = library.id;
  await put("/api/v1/knowledge-bases/agent-grants/main-agent", {libraryIds:[acceptanceLibraryId]});
  const form = new FormData();
  form.set("workspaceCode", "TRON_CORE");
  form.set("file", new Blob(["# Acceptance\n\nOrchid acceptance evidence proves the search tool can retrieve this document."], {type:"text/markdown"}), "acceptance.md");
  const upload = await fetch(`${api}/api/v1/knowledge-bases/${acceptanceLibraryId}/documents`, {method:"POST",headers:{Authorization:"Bearer profile-test-token"},body:form});
  expect(upload.ok).toBe(true);
  const document = (await upload.json()).data;
  await expect.poll(async () => {
    const response = await fetch(`${api}/api/v1/knowledge-bases/${acceptanceLibraryId}?workspaceCode=TRON_CORE`, {headers:{Authorization:"Bearer profile-test-token"}});
    const detail = (await response.json()).data;
    const current = detail.documents.find((item: {id:string}) => item.id === document.id);
    if (current?.status === "FAILED") throw new Error(`acceptance document indexing failed: ${JSON.stringify(current)}`);
    return current?.status;
  }, {timeout:60_000}).toBe("READY");
  await openChat(page);
  const transcript = page.getByRole("region", {name:"Chat transcript",exact:true});
  await page.locator('[data-theme-part="chat-input"]').fill("e2e-rag: find the acceptance evidence");
  await page.locator('[data-theme-part="chat-input"]').press("Enter");
  await expect(transcript).toContainText("e2e-rag-done", {timeout:120_000});
  const results = toolResultSnapshots.get("e2e-rag") ?? [];
  expect(results.map(item => item.tool)).toEqual(["rag_search"]);
  const search = JSON.parse(results[0].content);
  expect(search.authorizedCorpusIds).toContain(acceptanceLibraryId);
  expect(search.evidence.some((item: {text:string}) => item.text.includes("Orchid acceptance evidence"))).toBe(true);
  assertGatewayToolsInvariant();
});

test("ordinary chat searches and expands an authorized knowledge graph", async ({ page }) => {
  test.setTimeout(150_000);
  resetScenarioState("e2e-rag-graph");
  ragCalls.clear();
  expect(acceptanceChatModelCode).toBeTruthy();
  const library = await post("/api/v1/knowledge-bases", {name:"Acceptance graph library",description:"Isolated graph test",indexSettings:{fulltext:true,vector:false,graph:true,embeddingModelCode:acceptanceEmbeddingModelCode,graphModelCode:acceptanceChatModelCode,rerankModelCode:null}});
  acceptanceLibraryId = library.id;
  await put("/api/v1/knowledge-bases/agent-grants/main-agent", {libraryIds:[acceptanceLibraryId]});
  const form = new FormData();
  form.set("workspaceCode", "TRON_CORE");
  form.set("file", new Blob(["# Services\n\nAlpha calls Beta."], {type:"text/markdown"}), "graph.md");
  const upload = await fetch(`${api}/api/v1/knowledge-bases/${acceptanceLibraryId}/documents`, {method:"POST",headers:{Authorization:"Bearer profile-test-token"},body:form});
  expect(upload.ok).toBe(true);
  const document = (await upload.json()).data;
  await expect.poll(async () => {
    const response = await fetch(`${api}/api/v1/knowledge-bases/${acceptanceLibraryId}?workspaceCode=TRON_CORE`, {headers:{Authorization:"Bearer profile-test-token"}});
    const current = (await response.json()).data.documents.find((item: {id:string}) => item.id === document.id);
    if (current?.status === "FAILED") throw new Error(`acceptance graph indexing failed: ${JSON.stringify(current)}`);
    return current?.status;
  }, {timeout:60_000}).toBe("READY");
  await openChat(page);
  const transcript = page.getByRole("region", {name:"Chat transcript",exact:true});
  await page.locator('[data-theme-part="chat-input"]').fill("e2e-rag-graph: inspect the acceptance graph");
  await page.locator('[data-theme-part="chat-input"]').press("Enter");
  await expect(transcript).toContainText("e2e-rag-graph-done", {timeout:120_000});
  const results = toolResultSnapshots.get("e2e-rag-graph") ?? [];
  expect(results.map(item => item.tool)).toEqual(["rag_graph_search","rag_graph_expand"]);
  const search = JSON.parse(results[0].content);
  const expansion = JSON.parse(results[1].content);
  expect(search.nodes.some((node: {name:string}) => node.name === "Alpha")).toBe(true);
  expect(JSON.stringify(expansion)).toContain("Beta");
  assertGatewayToolsInvariant();
});

test("ordinary chat manages a knowledge quality evaluation", async ({ page }) => {
  test.setTimeout(150_000);
  resetScenarioState("e2e-rag-quality");
  ragCalls.clear();
  slowQualityEmbeddings = false;
  const library = await post("/api/v1/knowledge-bases", {name:"Acceptance quality library",description:"Isolated quality test",indexSettings:{fulltext:true,vector:true,graph:false,embeddingModelCode:acceptanceEmbeddingModelCode,rerankModelCode:null,graphModelCode:null}});
  acceptanceLibraryId = library.id;
  await put("/api/v1/knowledge-bases/agent-grants/main-agent", {libraryIds:[acceptanceLibraryId]});
  const body = Array.from({length:8}, (_,i) => `# Topic ${i}\n\norchid${i} has unique conditions, requirements, exceptions and supporting evidence for the evaluation.\n\n`).join("");
  const form = new FormData();
  form.set("workspaceCode", "TRON_CORE");
  form.set("file", new Blob([body], {type:"text/markdown"}), "quality.md");
  const upload = await fetch(`${api}/api/v1/knowledge-bases/${acceptanceLibraryId}/documents`, {method:"POST",headers:{Authorization:"Bearer profile-test-token"},body:form});
  expect(upload.ok).toBe(true);
  const document = (await upload.json()).data;
  await expect.poll(async () => {
    const response = await fetch(`${api}/api/v1/knowledge-bases/${acceptanceLibraryId}?workspaceCode=TRON_CORE`, {headers:{Authorization:"Bearer profile-test-token"}});
    const current = (await response.json()).data.documents.find((item: {id:string}) => item.id === document.id);
    if (current?.status === "FAILED") throw new Error(`acceptance quality indexing failed: ${JSON.stringify(current)}`);
    return current?.status;
  }, {timeout:60_000}).toBe("READY");
  await openChat(page);
  const transcript = page.getByRole("region", {name:"Chat transcript",exact:true});
  await page.locator('[data-theme-part="chat-input"]').fill("e2e-rag-quality: evaluate the acceptance library");
  await page.locator('[data-theme-part="chat-input"]').press("Enter");
  await expect(transcript).toContainText("e2e-rag-quality-done", {timeout:120_000});
  const results = toolResultSnapshots.get("e2e-rag-quality") ?? [];
  expect(results.map(item => item.tool)).toEqual(["rag_quality_sources","rag_quality_cases","rag_quality_start","rag_quality_status","rag_quality_cancel","rag_quality_status"]);
  const sources = JSON.parse(results[0].content);
  const dataset = JSON.parse(results[1].content);
  const started = JSON.parse(results[2].content);
  const status = JSON.parse(results[3].content);
  const cancelled = JSON.parse(results[4].content);
  const finalStatus = JSON.parse(results[5].content);
  report("quality run statuses", {started:started.status, beforeCancel:status.status, afterCancel:cancelled.status});
  expect(sources.records.length).toBeGreaterThanOrEqual(8);
  expect(dataset.cases).toHaveLength(8);
  expect(started.id).toBeTruthy();
  expect(status.id).toBe(started.id);
  expect(cancelled.id).toBe(started.id);
  expect(["QUEUED", "RUNNING", "PUBLISHING", "CANCEL_REQUESTED"]).toContain(status.status);
  expect(cancelled.status).toBe("CANCEL_REQUESTED");
  expect(finalStatus.status).toBe("CANCELLED");
  slowQualityEmbeddings = false;
  assertGatewayToolsInvariant();
});

function scenarioRequests() {
  return modelRequests.filter((request) => request.hasGatewayTools);
}

function report(label: string, value: unknown) {
  console.log(`[E2E] ${label}: ${typeof value === "string" ? value : JSON.stringify(value, null, 2)}`);
}

async function expandButtons(transcript: import("@playwright/test").Locator, selectors: string[]) {
  // Freeze the class-based positions. Filtering by aria-expanded races with
  // late run updates and can make a button disappear between count and click.
  for (const selector of selectors) {
    const buttons = transcript.locator(selector);
    const count = await buttons.count();
    for (let index = 0; index < count; index += 1) {
      const button = buttons.nth(index);
      if (await button.getAttribute("aria-expanded") === "false") await button.click({timeout: 5_000});
      await expect(button).toHaveAttribute("aria-expanded", "true");
    }
  }
}
async function expandWorkTranscript(transcript: import("@playwright/test").Locator) {
  // The final text arrives before RUN_FINISHED. Wait for the completed rail
  // before expanding; completion deliberately collapses the live work rail.
  const summary = transcript.locator("button.ef-chat__rail-summary");
  await expect(summary).toContainText("已工作");
  await expect(summary).toHaveAttribute("aria-expanded", "false");
  await expandButtons(transcript, ["button.ef-chat__rail-summary", "button.ef-chat__rail-entry-toggle"]);
}

/** Opens every tool card, then its Parameters panel, so payloads are on screen. */
async function expandToolPayloads(transcript: import("@playwright/test").Locator) {
  await expandButtons(transcript, ["button.ef-chat__tool-toggle", "button.ef-chat__tool-subtoggle"]);
}

async function openChat(page: import("@playwright/test").Page) {
  await page.setViewportSize({ width: 1440, height: 1000 });
  await seedResourceUiShell(page, "zh-CN");
  await routeActiveProfileApi(page);
  await page.goto(`${frontend!.baseUrl}/chats`);
  await page.locator('[data-menu-code="CHAT_NEW_SESSION"]').getByRole("button", { name: "新会话", exact: true }).click();
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
  const execResult = results.find((entry) => entry.tool === "command_exec")?.content ?? "";
  const reads = results.filter((entry) => entry.tool === "command_exec_read");
  const taskId = parseTaskId(execResult);
  report("long-task command_exec result", execResult);
  report("long-task task id", taskId);
  report("long-task command_exec_read result", reads.map((entry) => entry.content));

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
  // as an OpenAI-compatible provider-owned Embedding model and repoints the built-in
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
 * Each streamed conversation request exposes the stable gateway pair. Auxiliary
 * non-stream requests, such as title generation, do not use tools.
 */
function assertGatewayToolsInvariant() {
  const conversationRequests = modelRequests.filter((request) => request.stream);
  report("streamed model requests (count, tools arrays)", { count: conversationRequests.length, tools: conversationRequests.map((request) => request.tools) });
  expect(conversationRequests.length).toBeGreaterThan(0);
  conversationRequests.forEach((request, index) => {
    expect(request.tools, `conversation request ${index + 1} must expose exactly the gateway pair`).toEqual(EXPECTED_TOOL_NAMES);
  });
}
