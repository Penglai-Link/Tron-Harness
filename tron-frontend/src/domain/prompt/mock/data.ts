import type {
  PlatformPromptCreatePayload,
  PlatformPromptUpdatePayload,
  PromptCategory,
  PromptDetailPayload,
  PromptDuplicatePayload,
  PromptPage,
  PromptPageQuery,
  PromptRecord,
  PromptTranslation,
  PromptTranslationPayload,
  PromptVariable,
  PromptRuntimeProvider,
  WorkspacePromptCreatePayload,
  WorkspacePromptUpdatePayload,
} from "@/domain/prompt/model";
import {
  analyzePromptTemplate,
  normalizePromptVariables,
  reconcilePromptVariables,
} from "@/domain/prompt/model";

const seedTimestamp = "2026-07-13T02:00:00.000Z";
let nextPromptSequence = 1000;

const runtimeVariableProviders: Record<string, PromptRuntimeProvider> = {
  child_summaries: "RAG_TREE_CHILD_SUMMARIES",
  command: "TERMINAL_EXPLAIN_COMMAND",
  current_command: "TERMINAL_CURRENT_COMMAND",
  output: "TERMINAL_EXPLAIN_OUTPUT",
  persistence_mode: "AGENT_DOMAIN_PERSISTENCE_MODE",
  source_text: "RAG_TREE_SOURCE_TEXT",
  task_context: "TASK_CONTEXT",
  title: "RAG_TREE_TITLE",
  user_message: "CHAT_USER_MESSAGE",
  user_request: "TERMINAL_USER_REQUEST",
  visible_domains: "AGENT_VISIBLE_DOMAINS",
};

function platformPrompt(
  promptCode: string,
  displayName: string,
  category: PromptCategory,
  promptBody: string,
  builtIn = false,
  runtimeRequired = builtIn,
): PromptRecord {
  const timestamp = seedTimestamp;
  return {
    promptCode,
    scopeKind: "PLATFORM",
    workspaceCode: null,
    displayName,
    category,
    promptBody,
    description: `${displayName} shared platform prompt.`,
    enabled: true,
    sourceKind: "PLATFORM",
    sourceLabel: "Platform",
    sourceWorkspaceCode: null,
    sourceWorkspaceName: null,
    sourceWorkspacePath: [],
    allowInheritance: null,
    variables: seedPromptVariables(promptBody),
    translations: [
      {
        locale: "en-US",
        displayName,
        promptBody,
        description: `${displayName} shared platform prompt.`,
        status: "PUBLISHED",
        sourceVersion: 1,
        updatedAt: timestamp,
      },
      {
        locale: "zh-CN",
        displayName: `${displayName}（中文）`,
        promptBody,
        description: `${displayName} 的中文提示词。`,
        status: "PUBLISHED",
        sourceVersion: 1,
        updatedAt: timestamp,
      },
    ],
    builtIn,
    runtimeRequired,
    catalogKey: builtIn ? promptCode : null,
    version: 1,
    createdAt: seedTimestamp,
    updatedAt: seedTimestamp,
  };
}

function initialPlatformPrompts() {
  const prompts = [
    platformPrompt(
      "PROMPT-734600000000000001",
      "Workspace Router",
      "SYSTEM",
      "Route {{user_request}} to the most appropriate enabled workspace model.",
    ),
    platformPrompt(
      "PROMPT-734600000000000002",
      "Review Persona",
      "PERSONA",
      "Review {{change_summary}} for correctness, regressions, and missing tests.",
    ),
  ];
  const builtinSeeds: Array<[string, string, PromptCategory, string, boolean]> = [
    ["SYS_AGENT_FINAL_OUTPUT", "Agent final output guidance", "SYSTEM", "Final assistant output contract: answer with only the final result and do not expose chain-of-thought.", true],
    ["SYS_CHAT_ASK_USER", "Interactive user input contract", "SYSTEM", "When material clarification is required, call chat_ask_user and stop the turn.", true],
    ["SYS_AGENT_DOMAIN_DISCOVERY", "Domain discovery guidance", "SYSTEM", "Tool domain discovery is enabled. Visible domains: {{visible_domains}}. Persistence mode: {{persistence_mode}}.", true],
    ["SYS_AGENT_STEER_CONTINUITY", "Steer continuity guidance", "SYSTEM", "Preserve unresolved earlier requests when steering messages arrive during a run.", true],
    ["SYS_MEMORY_COMPACTION", "Short-term memory compaction", "SYSTEM", "Generate a short-term memory compaction checkpoint. Return final text only.", true],
    ["SYS_MEMORY_GOAL_JUDGE", "Goal judge system prompt", "SYSTEM", "Judge whether the active session goal should continue and return valid JSON.", true],
    ["SYS_MEMORY_GOAL_JUDGE_RETRY", "Goal judge retry prompt", "SYSTEM", "Re-evaluate the same goal judge input and return only valid JSON.", true],
    ["SYS_CHAT_SESSION_TITLE", "Chat title generation", "TEMPLATE", "Generate a concise chat title from the user's first message. {{user_message}}", false],
    ["SYS_TERMINAL_CHAT", "Terminal chat assistant", "SYSTEM", "You are a terminal assistant. Emit safe executable command blocks only when needed.", false],
    ["SYS_TERMINAL_EXPLAIN", "Terminal output explanation", "TEMPLATE", "Explain command {{command}} and output {{output}} without executing it.", false],
    ["SYS_TERMINAL_COMMAND_GENERATE", "Terminal command rewrite", "TEMPLATE", "Generate one safe Shell command from {{current_command}}.", false],
    ["SYS_TERMINAL_COMMAND_EXACT", "Exact terminal command instruction", "TEMPLATE", "Return exactly one executable shell command for {{user_request}}.", false],
    ["SYS_TASK_EXECUTION", "Task execution instruction", "TEMPLATE", "Execute the following task and provide the final result: {{task_context}}", true],
    ["SYS_RAG_TREE_SUMMARY", "RAG tree summary", "TEMPLATE", "Summarize {{title}} from {{source_text}} and {{child_summaries}}.", true],
  ];
  return [
    ...prompts,
    ...builtinSeeds.map(([code, name, category, body, runtimeRequired]) => platformPrompt(code, name, category, body, true, runtimeRequired)),
  ];
}

function initialWorkspacePrompts(): PromptRecord[] {
  return [
    {
      ...platformPrompt(
        "PROMPT-734600000000000101",
        "Tron Incident Summary",
        "TEMPLATE",
        "Summarize {{incident_summary}} for {{audience}} and list the next actions.",
      ),
      scopeKind: "WORKSPACE" as const,
      workspaceCode: "TRON_CORE",
      sourceKind: "LOCAL" as const,
      sourceLabel: "Local",
      sourceWorkspaceCode: "TRON_CORE",
      sourceWorkspaceName: "Tron Core",
      sourceWorkspacePath: [{ workspaceCode: "TRON_CORE", name: "Tron Core" }],
      allowInheritance: true,
    },
  ];
}

let platformPrompts = initialPlatformPrompts();
let workspacePrompts = initialWorkspacePrompts();

function now() {
  return new Date().toISOString();
}

function generatePromptCode() {
  nextPromptSequence += 1;
  return `PROMPT-MOCK-${nextPromptSequence}`;
}

function seedPromptVariables(promptBody: string) {
  return reconcilePromptVariables(promptBody, []).map((variable) => ({
    ...variable,
    description: `Variable ${variable.name}`,
    ...(runtimeVariableProviders[variable.name]
      ? {
        sourceKind: "RUNTIME" as const,
        managedVariableKey: null,
        runtimeProvider: runtimeVariableProviders[variable.name],
      }
      : {
        sourceKind: "MANAGED" as const,
        managedVariableKey: null,
        runtimeProvider: null,
      }),
  }));
}

function promptVariables(promptBody: string, variables: PromptVariable[]) {
  return normalizePromptVariables(promptBody, variables);
}

function inheritedPrompt(record: PromptRecord, workspaceCode: string): PromptRecord {
  return {
    ...record,
    scopeKind: "WORKSPACE",
    workspaceCode,
    sourceKind: "INHERITED",
    sourceLabel: "Platform",
    allowInheritance: null,
  };
}

function workspaceCatalog(workspaceCode: string) {
  const localRecords = workspacePrompts.filter((record) => record.workspaceCode === workspaceCode);
  const localCodeSet = new Set(localRecords.map((record) => record.promptCode));
  return [
    ...localRecords,
    ...platformPrompts
      .filter((record) => !localCodeSet.has(record.promptCode))
      .map((record) => inheritedPrompt(record, workspaceCode)),
  ];
}

function filterAndPage(records: PromptRecord[], query: PromptPageQuery): PromptPage<PromptRecord> {
  const normalizedSearch = query.search?.trim().toLowerCase() ?? "";
  const filtered = records.filter((record) => {
    if (query.category && record.category !== query.category) return false;
    if (query.sourceKind && record.sourceKind !== query.sourceKind) return false;
    if (query.enabled !== undefined && record.enabled !== query.enabled) return false;
    if (!normalizedSearch) return true;
    return [record.promptCode, record.displayName, record.description ?? "", record.sourceLabel]
      .some((value) => value.toLowerCase().includes(normalizedSearch));
  });
  const current = Math.max(1, query.current || 1);
  const size = Math.min(100, Math.max(1, query.size || 15));
  const start = (current - 1) * size;
  return {
    current,
    size,
    total: filtered.length,
    records: filtered.slice(start, start + size),
  };
}

export function resetPromptMockState() {
  nextPromptSequence = 1000;
  platformPrompts = initialPlatformPrompts();
  workspacePrompts = initialWorkspacePrompts();
}

export function listWorkspacePromptsMock(query: PromptPageQuery & { workspaceCode: string }) {
  return filterAndPage(workspaceCatalog(query.workspaceCode), query);
}

export function listPlatformPromptsMock(query: PromptPageQuery) {
  return filterAndPage(platformPrompts, query);
}

export function getWorkspacePromptMock(payload: PromptDetailPayload & { workspaceCode: string }) {
  const record = workspaceCatalog(payload.workspaceCode).find((item) => item.promptCode === payload.promptCode);
  if (!record) throw new Error("prompt was not found");
  return record;
}

export function getPlatformPromptMock(payload: PromptDetailPayload) {
  const record = platformPrompts.find((item) => item.promptCode === payload.promptCode);
  if (!record) throw new Error("prompt was not found");
  return record;
}

export function createWorkspacePromptMock(payload: WorkspacePromptCreatePayload) {
  const timestamp = now();
  const record: PromptRecord = {
    ...payload,
    promptCode: generatePromptCode(),
    scopeKind: "WORKSPACE",
    sourceKind: "LOCAL",
    sourceLabel: "Local",
    sourceWorkspaceCode: payload.workspaceCode,
    sourceWorkspaceName: payload.workspaceCode,
    sourceWorkspacePath: [{ workspaceCode: payload.workspaceCode, name: payload.workspaceCode }],
    translations: materializeTranslations(payload.translations, timestamp, 1),
    variables: promptVariables(payload.promptBody, payload.variables),
    version: 1,
    createdAt: timestamp,
    updatedAt: timestamp,
  };
  workspacePrompts = [record, ...workspacePrompts];
  return record;
}

export function createPlatformPromptMock(payload: PlatformPromptCreatePayload) {
  const timestamp = now();
  const record: PromptRecord = {
    ...payload,
    promptCode: generatePromptCode(),
    scopeKind: "PLATFORM",
    workspaceCode: null,
    sourceKind: "PLATFORM",
    sourceLabel: "Platform",
    sourceWorkspaceCode: null,
    sourceWorkspaceName: null,
    sourceWorkspacePath: [],
    allowInheritance: null,
    translations: materializeTranslations(payload.translations, timestamp, 1),
    variables: promptVariables(payload.promptBody, payload.variables),
    version: 1,
    createdAt: timestamp,
    updatedAt: timestamp,
  };
  platformPrompts = [record, ...platformPrompts];
  return record;
}

function updateRecord(
  record: PromptRecord,
  payload: PlatformPromptUpdatePayload | WorkspacePromptUpdatePayload,
): PromptRecord {
  if (payload.expectedVersion !== record.version) {
    throw new Error("prompt version conflict");
  }
  const updatedAt = now();
  return {
    ...record,
    allowInheritance: record.sourceKind === "LOCAL" && "allowInheritance" in payload
      ? (payload.allowInheritance ?? record.allowInheritance)
      : record.allowInheritance,
    description: payload.description || null,
    displayName: payload.displayName,
    enabled: payload.enabled,
    promptBody: payload.promptBody,
    variables: promptVariables(payload.promptBody, payload.variables),
    translations: payload.translations?.length
      ? materializeTranslations(payload.translations, updatedAt, record.version + 1)
      : record.translations,
    version: record.version + 1,
    updatedAt,
  };
}

function materializeTranslations(
  translations: PromptTranslationPayload[] | undefined,
  updatedAt: string,
  sourceVersion: number,
): PromptTranslation[] {
  return (translations ?? []).map((translation) => ({
    ...translation,
    sourceVersion,
    updatedAt,
  }));
}

export function updateWorkspacePromptMock(payload: WorkspacePromptUpdatePayload) {
  const ownedIndex = workspacePrompts.findIndex((record) => (
    record.workspaceCode === payload.workspaceCode && record.promptCode === payload.promptCode
  ));
  if (ownedIndex >= 0) {
    const updated = updateRecord(workspacePrompts[ownedIndex], payload);
    workspacePrompts = workspacePrompts.map((record, index) => index === ownedIndex ? updated : record);
    return updated;
  }

  const inherited = getWorkspacePromptMock(payload);
  const updated: PromptRecord = {
    ...updateRecord(inherited, payload),
    sourceKind: "OVERRIDE",
    sourceLabel: "Local override",
    sourceWorkspaceCode: payload.workspaceCode,
    sourceWorkspaceName: payload.workspaceCode,
    sourceWorkspacePath: [{ workspaceCode: payload.workspaceCode, name: payload.workspaceCode }],
  };
  workspacePrompts = [updated, ...workspacePrompts];
  return updated;
}

export function updatePlatformPromptMock(payload: PlatformPromptUpdatePayload) {
  const index = platformPrompts.findIndex((record) => record.promptCode === payload.promptCode);
  if (index < 0) throw new Error("prompt was not found");
  const updated = updateRecord(platformPrompts[index], payload);
  platformPrompts = platformPrompts.map((record, recordIndex) => recordIndex === index ? updated : record);
  return updated;
}

export function duplicateWorkspacePromptMock(payload: PromptDuplicatePayload & { workspaceCode: string }) {
  const source = getWorkspacePromptMock(payload);
  return createWorkspacePromptMock({
    allowInheritance: true,
    category: payload.targetCategory,
    description: source.description ?? "",
    displayName: `${source.displayName} Copy`,
    enabled: source.enabled,
    promptBody: source.promptBody,
    variables: source.variables,
    workspaceCode: payload.workspaceCode,
  });
}

export function duplicatePlatformPromptMock(payload: PromptDuplicatePayload) {
  const source = getPlatformPromptMock(payload);
  return createPlatformPromptMock({
    category: payload.targetCategory,
    description: source.description ?? "",
    displayName: `${source.displayName} Copy`,
    enabled: source.enabled,
    promptBody: source.promptBody,
    variables: source.variables,
  });
}

export function deleteWorkspacePromptMock(payload: PromptDetailPayload & { workspaceCode: string }) {
  const before = workspacePrompts.length;
  workspacePrompts = workspacePrompts.filter((record) => !(
    record.workspaceCode === payload.workspaceCode && record.promptCode === payload.promptCode
  ));
  if (workspacePrompts.length === before) throw new Error("only local prompts can be deleted");
  return { promptCode: payload.promptCode };
}

export function deletePlatformPromptMock(payload: PromptDetailPayload) {
  const before = platformPrompts.length;
  platformPrompts = platformPrompts.filter((record) => record.promptCode !== payload.promptCode);
  if (platformPrompts.length === before) throw new Error("prompt was not found");
  workspacePrompts = workspacePrompts.filter((record) => record.promptCode !== payload.promptCode);
  return { promptCode: payload.promptCode };
}

export function revertWorkspacePromptMock(payload: PromptDetailPayload & { workspaceCode: string }) {
  const existing = workspacePrompts.find((record) => (
    record.workspaceCode === payload.workspaceCode
    && record.promptCode === payload.promptCode
    && record.sourceKind === "OVERRIDE"
  ));
  if (!existing) throw new Error("prompt override was not found");
  workspacePrompts = workspacePrompts.filter((record) => record !== existing);
  return getWorkspacePromptMock(payload);
}

export function setWorkspacePromptEnabledMock(payload: PromptDetailPayload & { workspaceCode: string; enabled: boolean; expectedVersion: number }) {
  const current = getWorkspacePromptMock(payload);
  if (current.version !== payload.expectedVersion) throw new Error("prompt version conflict");
  return updateWorkspacePromptMock({
    description: current.description ?? "",
    displayName: current.displayName,
    enabled: payload.enabled,
    promptBody: current.promptBody,
    promptCode: current.promptCode,
    expectedVersion: payload.expectedVersion,
    variables: current.variables,
    workspaceCode: payload.workspaceCode,
  });
}

export function translatePromptDraftMock(payload: {
  sourceLocale: "en-US" | "zh-CN";
  targetLocale: "en-US" | "zh-CN";
  displayName: string;
  promptBody: string;
  description?: string | null;
}): PromptTranslation {
  const marker = payload.targetLocale === "zh-CN" ? "中文翻译草稿" : "English translation draft";
  return {
    locale: payload.targetLocale,
    displayName: `${payload.displayName} · ${marker}`,
    promptBody: `${payload.promptBody}\n\n[${marker}]`,
    description: payload.description ?? null,
    status: "DRAFT",
    sourceVersion: 1,
    updatedAt: now(),
  };
}

export function setPlatformPromptEnabledMock(payload: PromptDetailPayload & { enabled: boolean; expectedVersion: number }) {
  const current = getPlatformPromptMock(payload);
  if (current.version !== payload.expectedVersion) throw new Error("prompt version conflict");
  return updatePlatformPromptMock({
    description: current.description ?? "",
    displayName: current.displayName,
    enabled: payload.enabled,
    promptBody: current.promptBody,
    promptCode: current.promptCode,
    expectedVersion: payload.expectedVersion,
    variables: current.variables,
  });
}

export function setWorkspacePromptAllowInheritanceMock(payload: PromptDetailPayload & { workspaceCode: string; allowInheritance: boolean; expectedVersion: number }) {
  const index = workspacePrompts.findIndex((record) => (
    record.workspaceCode === payload.workspaceCode
    && record.promptCode === payload.promptCode
    && record.sourceKind === "LOCAL"
  ));
  if (index < 0) throw new Error("only local prompts can change inheritance");
  if (workspacePrompts[index].version !== payload.expectedVersion) throw new Error("prompt version conflict");
  const updated = {
    ...workspacePrompts[index],
    allowInheritance: payload.allowInheritance,
    updatedAt: now(),
    version: workspacePrompts[index].version + 1,
  };
  workspacePrompts = workspacePrompts.map((record, recordIndex) => recordIndex === index ? updated : record);
  return updated;
}

export function analyzePromptTemplateMock(promptBody: string) {
  const analysis = analyzePromptTemplate(promptBody);
  return {
    invalidVariables: analysis.invalidVariables,
    variables: analysis.variables.map((name) => ({ name, type: "STRING" as const })),
  };
}

export function exportWorkspacePromptsMock(query: PromptPageQuery & { workspaceCode: string }) {
  const page = listWorkspacePromptsMock({ ...query, current: 1, size: 100 });
  return {
    fileName: `${query.workspaceCode.toLowerCase()}-prompts.json`,
    content: JSON.stringify(page.records, null, 2),
  };
}
