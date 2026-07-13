import type {
  PlatformPromptCreatePayload,
  PlatformPromptUpdatePayload,
  PromptCategory,
  PromptDetailPayload,
  PromptDuplicatePayload,
  PromptPage,
  PromptPageQuery,
  PromptPreviewPayload,
  PromptRecord,
  WorkspacePromptCreatePayload,
  WorkspacePromptUpdatePayload,
} from "@/domain/prompt/model";
import {
  analyzePromptTemplate,
  renderPromptTemplate,
} from "@/domain/prompt/model/promptTemplate";

const seedTimestamp = "2026-07-13T02:00:00.000Z";
let nextPromptSequence = 1000;

function platformPrompt(
  promptCode: string,
  displayName: string,
  category: PromptCategory,
  promptBody: string,
): PromptRecord {
  return {
    promptCode,
    scopeKind: "PLATFORM",
    workspaceCode: null,
    displayName,
    category,
    promptBody,
    description: `${displayName} shared platform prompt.`,
    modelCode: "MODEL-734600000000000001",
    temperature: 0.2,
    maxOutputTokens: 2048,
    enabled: true,
    sourceKind: "PLATFORM",
    sourceLabel: "Platform",
    sourceWorkspaceCode: null,
    sourceWorkspaceName: null,
    sourceWorkspacePath: [],
    allowInheritance: null,
    variables: analyzePromptTemplate(promptBody).variables.map((name) => ({ name, type: "STRING" as const })),
    version: 1,
    createdAt: seedTimestamp,
    updatedAt: seedTimestamp,
  };
}

function initialPlatformPrompts() {
  return [
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

function promptVariables(promptBody: string) {
  const analysis = analyzePromptTemplate(promptBody);
  if (analysis.invalidVariables.length) {
    throw new Error(`Invalid prompt variables: ${analysis.invalidVariables.join(", ")}`);
  }
  return analysis.variables.map((name) => ({ name, type: "STRING" as const }));
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
    variables: promptVariables(payload.promptBody),
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
    variables: promptVariables(payload.promptBody),
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
  return {
    ...record,
    allowInheritance: record.sourceKind === "LOCAL" && "allowInheritance" in payload
      ? (payload.allowInheritance ?? record.allowInheritance)
      : record.allowInheritance,
    description: payload.description || null,
    displayName: payload.displayName,
    enabled: payload.enabled,
    maxOutputTokens: payload.maxOutputTokens,
    modelCode: payload.modelCode,
    promptBody: payload.promptBody,
    temperature: payload.temperature,
    variables: promptVariables(payload.promptBody),
    version: record.version + 1,
    updatedAt: now(),
  };
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
    maxOutputTokens: source.maxOutputTokens,
    modelCode: source.modelCode,
    promptBody: source.promptBody,
    temperature: source.temperature,
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
    maxOutputTokens: source.maxOutputTokens,
    modelCode: source.modelCode,
    promptBody: source.promptBody,
    temperature: source.temperature,
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
    maxOutputTokens: current.maxOutputTokens,
    modelCode: current.modelCode,
    promptBody: current.promptBody,
    promptCode: current.promptCode,
    expectedVersion: payload.expectedVersion,
    temperature: current.temperature,
    workspaceCode: payload.workspaceCode,
  });
}

export function setPlatformPromptEnabledMock(payload: PromptDetailPayload & { enabled: boolean; expectedVersion: number }) {
  const current = getPlatformPromptMock(payload);
  if (current.version !== payload.expectedVersion) throw new Error("prompt version conflict");
  return updatePlatformPromptMock({
    description: current.description ?? "",
    displayName: current.displayName,
    enabled: payload.enabled,
    maxOutputTokens: current.maxOutputTokens,
    modelCode: current.modelCode,
    promptBody: current.promptBody,
    promptCode: current.promptCode,
    expectedVersion: payload.expectedVersion,
    temperature: current.temperature,
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

export function testPromptPreviewMock(payload: PromptPreviewPayload) {
  let promptBody = payload.promptBody;
  if (promptBody === undefined) {
    if (!payload.promptCode) throw new Error("promptCode or promptBody is required");
    promptBody = payload.workspaceCode
      ? getWorkspacePromptMock({ promptCode: payload.promptCode, workspaceCode: payload.workspaceCode }).promptBody
      : getPlatformPromptMock({ promptCode: payload.promptCode }).promptBody;
  }
  const result = renderPromptTemplate(promptBody, payload.variables);
  if (result.errors.length) {
    throw new Error(result.errors.map((error) => `${error.kind}: ${error.variableName}`).join(", "));
  }
  return {
    modelCode: payload.modelCode ?? (payload.workspaceCode && payload.promptCode
      ? getWorkspacePromptMock({ promptCode: payload.promptCode, workspaceCode: payload.workspaceCode }).modelCode
      : payload.promptCode
        ? getPlatformPromptMock({ promptCode: payload.promptCode }).modelCode
        : null),
    renderedPrompt: result.renderedPrompt,
    output: `Mock output for ${result.renderedPrompt.slice(0, 120)}`,
  };
}

export function exportWorkspacePromptsMock(query: PromptPageQuery & { workspaceCode: string }) {
  const page = listWorkspacePromptsMock({ ...query, current: 1, size: 100 });
  return {
    fileName: `${query.workspaceCode.toLowerCase()}-prompts.json`,
    content: JSON.stringify(page.records, null, 2),
  };
}
