import type {
  PromptCategory,
  PromptCreatePayload,
  PromptDetailPayload,
  PromptDuplicatePayload,
  PromptPage,
  PromptPageQuery,
  PromptRecord,
  PromptRuntimeProvider,
  PromptTranslation,
  PromptTranslationPayload,
  PromptUpdatePayload,
  PromptVariable,
} from "@/domain/prompt/model";
import { analyzePromptTemplate, normalizePromptVariables, reconcilePromptVariables } from "@/domain/prompt/model";
import { agentRolePromptSeeds } from "./agentRolePromptSeeds";

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

function makePrompt(
  promptCode: string,
  displayName: string,
  category: PromptCategory,
  promptBody: string,
  builtIn = false,
  runtimeRequired = builtIn,
): PromptRecord {
  const description = `${displayName} shared platform prompt.`;
  return {
    promptCode,
    displayName,
    category,
    promptBody,
    description,
    enabled: true,
    variables: seedPromptVariables(promptBody),
    translations: [
      { locale: "en-US", displayName, promptBody, description, status: "PUBLISHED", sourceVersion: 1, updatedAt: seedTimestamp },
      { locale: "zh-CN", displayName: `${displayName}（中文）`, promptBody, description: `${displayName} 的中文提示词。`, status: "PUBLISHED", sourceVersion: 1, updatedAt: seedTimestamp },
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
    makePrompt("PROMPT-734600000000000002", "Review Persona", "PERSONA", "Review {{change_summary}} for correctness, regressions, and missing tests."),
  ];
  const builtinSeeds: Array<[string, string, PromptCategory, string, boolean]> = [
    ["SYS_AGENT_FINAL_OUTPUT", "Agent final output guidance", "SYSTEM", "Final assistant output contract: answer with only the final result and do not expose chain-of-thought.", true],
    ["SYS_CHAT_ASK_USER", "Interactive user input contract", "SYSTEM", "When material clarification is required, call chat_ask_user and stop the turn.", true],
    ["SYS_AGENT_DOMAIN_DISCOVERY", "Domain discovery guidance", "SYSTEM", "You may call only tools explicitly listed in the current tool list. Before using a visible but not yet expanded domain, first call `discovery_expand_tool_domain` and pass that domain's `domainCode`; after receiving the updated tool list, call a tool provided in it. Never guess tool names or request parameters, never call hidden tools directly before expanding the domain, and carefully review the tool-call examples.", true],
    ["SYS_CHAT_SESSION_TITLE", "Chat title generation", "TEMPLATE", "Generate a concise chat title from the user's first message. {{user_message}}", false],
    ["SYS_TERMINAL_COMMAND_EXACT", "Exact terminal command instruction", "TEMPLATE", "Return exactly one executable shell command for {{user_request}}.", false],
    ["SYS_TASK_EXECUTION", "Task execution instruction", "TEMPLATE", "Execute the following task and provide the final result: {{task_context}}", true],
  ];
  return [
    ...prompts,
    ...agentRolePromptSeeds.map((seed) => {
      const original = seed.translations["en-US"];
      return {
        ...makePrompt(seed.promptCode, original.displayName, "AGENT_ROLE", original.promptBody, true, false),
        description: original.description,
        catalogKey: seed.catalogKey,
        translations: Object.entries(seed.translations).map(([locale, translation]) => ({
          ...translation,
          locale: locale as "en-US" | "zh-CN",
          status: "PUBLISHED" as const,
          sourceVersion: seed.version,
          updatedAt: seedTimestamp,
        })),
      };
    }),
    ...builtinSeeds.map(([code, name, category, body, runtimeRequired]) => {
      const prompt = makePrompt(code, name, category, body, true, runtimeRequired);
      if (code === "SYS_AGENT_DOMAIN_DISCOVERY") {
        prompt.version = 2;
        prompt.translations = (prompt.translations ?? []).map((translation) => ({
          ...translation,
          promptBody: translation.locale === "zh-CN"
            ? "你只能调用当前工具列表中明确提供的工具。使用可见但尚未展开的领域前，先调用 `discovery_expand_tool_domain` 并传入该领域的 domainCode；收到更新后的工具列表后，再调用其中提供的工具。绝不要猜测工具名和请求参数，也不要在展开领域前直接调用隐藏工具，注意仔细看工具调用示例说明。"
            : body,
          sourceVersion: 2,
        }));
      }
      return prompt;
    }),
  ];
}

let platformPrompts = initialPlatformPrompts();

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
      ? { sourceKind: "RUNTIME" as const, managedVariableKey: null, runtimeProvider: runtimeVariableProviders[variable.name] }
      : { sourceKind: "MANAGED" as const, managedVariableKey: null, runtimeProvider: null }),
  }));
}

function promptVariables(promptBody: string, variables: PromptVariable[]) {
  return normalizePromptVariables(promptBody, variables);
}

function filterAndPage(records: PromptRecord[], query: PromptPageQuery): PromptPage<PromptRecord> {
  const normalizedSearch = query.search?.trim().toLowerCase() ?? "";
  const filtered = records.filter((record) => {
    if (query.category && record.category !== query.category) return false;
    if (query.enabled !== undefined && record.enabled !== query.enabled) return false;
    if (!normalizedSearch) return true;
    return [record.promptCode, record.displayName, record.description ?? "",
      ...(record.translations ?? []).flatMap((translation) => [translation.displayName, translation.description ?? ""])]
      .some((value) => value.toLowerCase().includes(normalizedSearch));
  });
  const current = Math.max(1, query.current || 1);
  const size = Math.min(100, Math.max(1, query.size || 15));
  const start = (current - 1) * size;
  return { current, size, total: filtered.length, records: filtered.slice(start, start + size) };
}

export function resetPromptMockState() {
  nextPromptSequence = 1000;
  platformPrompts = initialPlatformPrompts();
}

export function listPlatformPromptsMock(query: PromptPageQuery) {
  return filterAndPage(platformPrompts, query);
}

export function getPlatformPromptMock(payload: PromptDetailPayload) {
  const record = platformPrompts.find((item) => item.promptCode === payload.promptCode);
  if (!record) throw new Error("prompt was not found");
  return record;
}

function materializeTranslations(translations: PromptTranslationPayload[] | undefined, updatedAt: string, sourceVersion: number): PromptTranslation[] {
  return (translations ?? []).map((translation) => ({ ...translation, sourceVersion, updatedAt }));
}

export function createPlatformPromptMock(payload: PromptCreatePayload) {
  const timestamp = now();
  const record: PromptRecord = {
    ...payload,
    promptCode: generatePromptCode(),
    translations: materializeTranslations(payload.translations, timestamp, 1),
    variables: promptVariables(payload.promptBody, payload.variables),
    version: 1,
    createdAt: timestamp,
    updatedAt: timestamp,
  };
  platformPrompts = [record, ...platformPrompts];
  return record;
}

function updateRecord(record: PromptRecord, payload: PromptUpdatePayload): PromptRecord {
  if (payload.expectedVersion !== record.version) throw new Error("prompt version conflict");
  const updatedAt = now();
  return {
    ...record,
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

export function updatePlatformPromptMock(payload: PromptUpdatePayload) {
  const index = platformPrompts.findIndex((record) => record.promptCode === payload.promptCode);
  if (index < 0) throw new Error("prompt was not found");
  const updated = updateRecord(platformPrompts[index], payload);
  platformPrompts = platformPrompts.map((record, recordIndex) => recordIndex === index ? updated : record);
  return updated;
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

export function deletePlatformPromptMock(payload: PromptDetailPayload) {
  const before = platformPrompts.length;
  platformPrompts = platformPrompts.filter((record) => record.promptCode !== payload.promptCode);
  if (platformPrompts.length === before) throw new Error("prompt was not found");
  return { promptCode: payload.promptCode };
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

export function analyzePromptTemplateMock(promptBody: string) {
  const analysis = analyzePromptTemplate(promptBody);
  return { invalidVariables: analysis.invalidVariables, variables: analysis.variables.map((name) => ({ name, type: "STRING" as const })) };
}
