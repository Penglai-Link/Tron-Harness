import {
  useEffect,
  useState,
} from "react";
import {
  Search,
  Upload,
} from "lucide-react";

import {
  Field,
  SelectInput,
  SurfaceCard,
  TextArea,
  TextInput,
  ToggleSwitch,
} from "@/shared";
import { Button } from "@/shared/ui/Button";

import {
  InspectorRow,
  PreviewLine,
  SectionTitle,
} from "@/domain/rag/components/RagShared";
import {
  runRagIngest,
  runRagSearch,
  type RagIngestResponseApi,
  type RagSearchResponseApi,
} from "@/domain/rag/api/ragManagementApi";
import { useRagProfileDetail } from "@/domain/rag/hooks/useRagProfileDetail";
import type {
  CorpusRecord as RagCorpusRecord,
  IngestProfileRecord as RagIngestProfileRecord,
  ProfileRecord as RagProfileRecord,
  RagFilterFieldType,
  RetrievalProfileRecord as RagRetrievalProfileRecord,
} from "@/domain/rag/model/RagModel";

type WorkbenchSourcePayloadForm = {
  documentFormat: string;
  documentUri: string;
  language: string;
  metadataSourcePath: string;
  sourceCode: string;
  sourceType: string;
  sourceVersion: string;
  text: string;
};

type WorkbenchBusinessAttributeRow = {
  field: string;
  formId: string;
  value: string;
};

type WorkbenchFilterRow = {
  field: string;
  formId: string;
  operator: string;
  value: string;
};

type WorkbenchReturnFieldRow = {
  field: string;
  formId: string;
};

type WorkbenchFieldOption = {
  field: string;
  type: RagFilterFieldType;
};

type WorkbenchPanel = "ingest" | "retrieval";
type WorkbenchScopeType = "AGENT" | "SPACE" | "USER";

const workbenchPanels: Array<{ label: string; summary: string; value: WorkbenchPanel }> = [
  {
    label: "Ingest Request",
    summary: "Write source payloads into a selected corpus.",
    value: "ingest",
  },
  {
    label: "Retrieval Test",
    summary: "Run retrieval profiles with query fields and filters.",
    value: "retrieval",
  },
];
const workbenchScopeOptions: Array<{ label: string; value: WorkbenchScopeType }> = [
  { label: "Workspace", value: "SPACE" },
  { label: "User", value: "USER" },
  { label: "Agent", value: "AGENT" },
];

const workbenchFilterOperators = ["eq", "in", "gt", "gte", "lt", "lte"] as const;
const workbenchReservedReturnFields = new Set([
  "corpusId",
  "metadata",
  "preview",
  "routeIds",
  "scopeCode",
  "scopeType",
  "score",
  "sourceCode",
  "sourceVersion",
  "targetCode",
  "targetType",
  "text",
  "title",
]);
const workbenchFixedPayloadFields = new Set([
  ...workbenchReservedReturnFields,
  "sourceStatus",
  "sourceType",
  "status",
  "textUnitStatus",
]);

function isDocumentPayloadProfile(profile?: RagIngestProfileRecord) {
  return profile?.config.stages.preparePayload?.strategyId === "RAG_DOCUMENT_REF_CONTRACT_PAYLOAD_V1";
}

function buildDefaultPayloadForm(corpusId: string, profile?: RagIngestProfileRecord): WorkbenchSourcePayloadForm {
  const sourceType = profile?.config.sourceInputPolicy.supportedSourceTypes[0] ?? "TEXT";
  const sourceVersion = profile?.config.requiredSourceVersion || "v1";
  const sourceCode = `${corpusId || "rag"}-workbench-source`;
  return {
    documentFormat: "MARKDOWN",
    documentUri: `project-doc://${sourceCode}`,
    language: "en-US",
    metadataSourcePath: "",
    sourceCode,
    sourceType,
    sourceVersion,
    text: `Workbench ingest sample for ${corpusId || "RAG"}.`,
  };
}

function buildPayloadFromForm(
  form: WorkbenchSourcePayloadForm,
  documentPayload: boolean,
  domainFields: Record<string, unknown>,
): Record<string, unknown> {
  const metadata: Record<string, string> = {};
  if (form.language.trim()) metadata.language = form.language.trim();
  if (form.metadataSourcePath.trim()) metadata.sourcePath = form.metadataSourcePath.trim();

  const payload: Record<string, unknown> = {
    metadata,
    source: {
      code: form.sourceCode.trim(),
      type: form.sourceType.trim(),
      version: form.sourceVersion.trim(),
    },
  };
  if (Object.keys(domainFields).length > 0) {
    payload.domainFields = domainFields;
  }

  if (documentPayload) {
    payload.document = {
      format: form.documentFormat.trim(),
      uri: form.documentUri.trim(),
    };
  } else {
    payload.text = form.text;
  }

  return payload;
}

function readErrorMessage(error: unknown) {
  return error instanceof Error ? error.message : "RAG workbench request failed.";
}

const workbenchAgentCode = "AGENT_CODEX";

function buildTargetScope(scopeType: WorkbenchScopeType, userCode: string, workspaceCode: string) {
  const normalizedWorkspaceCode = workspaceCode || "TRON_CORE";
  if (scopeType === "USER") {
    return {
      scopeCode: userCode || "USR-AUTH-0001",
      scopeType,
    };
  }
  if (scopeType === "AGENT") {
    return {
      scopeCode: workbenchAgentCode,
      scopeType,
    };
  }
  return {
    scopeCode: normalizedWorkspaceCode,
    scopeType,
  };
}

function formatScore(score: number | undefined) {
  return Number.isFinite(score) ? score!.toFixed(3) : "n/a";
}

function parseCommaSeparatedValues(value: string) {
  return value
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
}

function createWorkbenchFilterRow(index: number): WorkbenchFilterRow {
  return {
    field: "",
    formId: `filter-${index + 1}`,
    operator: "eq",
    value: "",
  };
}

function createWorkbenchBusinessAttributeRow(index: number): WorkbenchBusinessAttributeRow {
  return {
    field: "",
    formId: `business-attribute-${index + 1}`,
    value: "",
  };
}

function createWorkbenchReturnFieldRow(index: number): WorkbenchReturnFieldRow {
  return {
    field: "",
    formId: `return-field-${index + 1}`,
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function parseFilterValue(value: string, valueType: "boolean" | "integer" | "string", field: string): unknown {
  const trimmed = value.trim();
  if (valueType === "integer") {
    const parsed = Number.parseInt(trimmed, 10);
    if (!Number.isFinite(parsed)) {
      throw new Error(`Filter ${field} must be an integer.`);
    }
    return parsed;
  }
  if (valueType === "boolean") {
    const normalized = trimmed.toLowerCase();
    if (["true", "1", "yes"].includes(normalized)) return true;
    if (["false", "0", "no"].includes(normalized)) return false;
    throw new Error(`Filter ${field} must be true or false.`);
  }
  return trimmed;
}

function appendFilter(
  filters: Record<string, unknown>,
  field: string,
  filter: { op: string; value: unknown },
) {
  const existing = filters[field];
  if (!isRecord(existing)) {
    filters[field] = filter;
    return;
  }

  if (existing.op === "and" && Array.isArray(existing.value)) {
    existing.value.push(filter);
    return;
  }

  filters[field] = {
    op: "and",
    value: [existing, filter],
  };
}

function collectWorkbenchFieldOptions(
  corpora: RagCorpusRecord[],
  reservedFields: Set<string>,
): WorkbenchFieldOption[] {
  const optionsByField = new Map<string, WorkbenchFieldOption>();

  corpora.forEach((corpus) => {
    const schemaFields = corpus.config?.filterSchema.fields ?? {};
    const addField = (field: string, type?: RagFilterFieldType) => {
      const normalizedField = field.trim();
      if (!normalizedField || reservedFields.has(normalizedField) || optionsByField.has(normalizedField)) return;
      optionsByField.set(normalizedField, {
        field: normalizedField,
        type: type ?? schemaFields[normalizedField]?.type ?? "string",
      });
    };

    Object.keys(corpus.config?.fieldMapping.source ?? {}).forEach((field) => addField(field));
    Object.keys(corpus.config?.fieldMapping.textUnit ?? {}).forEach((field) => addField(field));
    Object.entries(schemaFields).forEach(([field, schema]) => addField(field, schema.type));
    corpus.filters.forEach((field) => addField(field));
  });

  return [...optionsByField.values()].sort((left, right) => left.field.localeCompare(right.field));
}

function buildDomainFields(
  rows: WorkbenchBusinessAttributeRow[],
  fieldSchema: Record<string, WorkbenchFieldOption>,
) {
  const domainFields: Record<string, unknown> = {};
  rows.forEach((row, index) => {
    const field = row.field.trim();
    const rawValue = row.value.trim();
    if (!field && !rawValue) return;
    if (!field || !rawValue) {
      throw new Error(`Business attribute ${index + 1} must include field and value.`);
    }
    const valueType = fieldSchema[field]?.type ?? "string";
    domainFields[field] = parseFilterValue(rawValue, valueType, field);
  });
  return domainFields;
}

function buildReturnFields(rows: WorkbenchReturnFieldRow[]) {
  return [...new Set(rows.map((row) => row.field.trim()).filter(Boolean))];
}

function formatFieldValue(value: unknown) {
  if (value === null || value === undefined) return "";
  if (typeof value === "string") return value;
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  return JSON.stringify(value);
}

function formatArtifactKeys(keys: string[] | undefined) {
  return keys && keys.length > 0 ? keys.join(", ") : "none";
}

function StageResultsList({
  results,
  title,
}: {
  results?: RagIngestResponseApi["stageResults"];
  title: string;
}) {
  if (!results || results.length === 0) return null;
  return (
    <div className="mt-5 space-y-3">
      <h2 className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">{title}</h2>
      {results.map((result, index) => {
        const warnings = result.warnings ?? [];
        return (
          <article
            className="rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4 text-sm"
            key={`${result.nodeId}-${index}`}
          >
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div>
                <p className="font-mono text-[12px] text-[var(--color-on-surface)]">{result.nodeId}</p>
                <p className="mt-1 font-mono text-[12px] text-[var(--color-on-surface-variant)]">
                  {result.stageType} · {result.strategyId}
                </p>
              </div>
              <span className="rounded-full border border-[var(--color-outline-variant)] px-2.5 py-1 font-mono text-[12px] text-[var(--color-on-surface-variant)]">
                {result.status} · {result.durationMs ?? 0}ms
              </span>
            </div>
            <dl className="mt-3 grid gap-2 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] p-3 sm:grid-cols-2">
              <div>
                <dt className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">Input Artifacts</dt>
                <dd className="mt-1 break-all font-mono text-[12px]">{formatArtifactKeys(result.inputArtifactKeys)}</dd>
              </div>
              <div>
                <dt className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">Output Artifacts</dt>
                <dd className="mt-1 break-all font-mono text-[12px]">{formatArtifactKeys(result.outputArtifactKeys)}</dd>
              </div>
            </dl>
            {warnings.length > 0 ? (
              <p className="mt-3 text-[12px] text-[var(--color-warning)]">
                {warnings.length === 1 ? "Warning" : "Warnings"} {warnings.join(", ")}
              </p>
            ) : null}
          </article>
        );
      })}
    </div>
  );
}

function workspaceCodeForAccessContext(
  targetScope: ReturnType<typeof buildTargetScope>,
  fallbackWorkspaceCode: string,
) {
  return targetScope.scopeType === "SPACE" && targetScope.scopeCode
    ? targetScope.scopeCode
    : fallbackWorkspaceCode || "TRON_CORE";
}

export function RagWorkbenchTab({
  corpora,
  profiles,
  traceEnabled,
  userCode,
  workspaceCode,
  onNotice,
  onTraceEnabledChange,
}: {
  corpora: RagCorpusRecord[];
  profiles: RagProfileRecord[];
  traceEnabled: boolean;
  userCode: string;
  workspaceCode: string;
  onNotice: (notice: string) => void;
  onTraceEnabledChange: (checked: boolean) => void;
}) {
  const visibleCorpora = corpora.filter((corpus) => corpus.status !== "Reserved");
  const ingestProfiles = profiles.filter((profile): profile is RagIngestProfileRecord => profile.type === "Ingest");
  const retrievalProfiles = profiles.filter((profile): profile is RagRetrievalProfileRecord => (
    profile.type === "Retrieval" && profile.status !== "Draft"
  ));
  const [corpusId, setCorpusId] = useState(visibleCorpora[0]?.code ?? "");
  const [ingestProfileId, setIngestProfileId] = useState(ingestProfiles[0]?.id ?? "");
  const [targetScopeType, setTargetScopeType] = useState<WorkbenchScopeType>("SPACE");
  const [sourcePayloadEdited, setSourcePayloadEdited] = useState(false);
  const [sourcePayloadForm, setSourcePayloadForm] = useState<WorkbenchSourcePayloadForm>(() => (
    buildDefaultPayloadForm(visibleCorpora[0]?.code ?? "", ingestProfiles[0])
  ));
  const [businessAttributeRows, setBusinessAttributeRows] = useState<WorkbenchBusinessAttributeRow[]>([]);
  const [retrievalProfileId, setRetrievalProfileId] = useState(retrievalProfiles[0]?.id ?? "");
  const [query, setQuery] = useState("How should the RAG context pack respect tokenBudget?");
  const [contextStrategy, setContextStrategy] = useState(retrievalProfiles[0]?.config.contextStrategies[0] ?? "DOC_QA_CONTEXT_PACK");
  const [returnFieldRows, setReturnFieldRows] = useState<WorkbenchReturnFieldRow[]>([]);
  const [filterRows, setFilterRows] = useState<WorkbenchFilterRow[]>([]);
  const [limit, setLimit] = useState("8");
  const [tokenBudget, setTokenBudget] = useState("4096");
  const [ingestLoading, setIngestLoading] = useState(false);
  const [searchLoading, setSearchLoading] = useState(false);
  const [ingestResult, setIngestResult] = useState<RagIngestResponseApi | null>(null);
  const [searchResult, setSearchResult] = useState<RagSearchResponseApi | null>(null);
  const [ingestFeedback, setIngestFeedback] = useState("");
  const [searchFeedback, setSearchFeedback] = useState("");
  const [activeWorkbenchPanel, setActiveWorkbenchPanel] = useState<WorkbenchPanel>("ingest");
  const { loading: retrievalProfileDetailLoading, profile: retrievalProfileDetail } = useRagProfileDetail(retrievalProfileId);
  const selectedIngestCorpus = corpora.find((corpus) => corpus.code === corpusId);
  const selectedIngestProfile = ingestProfiles.find((profile) => profile.id === ingestProfileId);
  const selectedRetrievalProfile = retrievalProfileDetail?.type === "Retrieval"
    ? retrievalProfileDetail
    : retrievalProfiles.find((profile) => profile.id === retrievalProfileId);
  const usesDocumentPayload = isDocumentPayloadProfile(selectedIngestProfile);
  const sourceTypeOptions = selectedIngestProfile?.config.sourceInputPolicy.supportedSourceTypes ?? [];
  const requiredSourceVersion = selectedIngestProfile?.config.requiredSourceVersion ?? "";
  const contextStrategyOptions = selectedRetrievalProfile?.config.contextStrategies.length
    ? selectedRetrievalProfile.config.contextStrategies
    : ["RESULT_LIST"];
  const selectedRetrievalCorpora = corpora.filter((corpus) => (
    selectedRetrievalProfile?.corpusIds.includes(corpus.code)
    || selectedRetrievalProfile?.primaryCorpusId === corpus.code
  ));
  const filterFieldOptions = selectedRetrievalCorpora
    .flatMap((corpus) => {
      const schemaFields = corpus.config?.filterSchema.fields;
      if (schemaFields) {
        return Object.entries(schemaFields).map(([field, schema]) => ({
          field,
          operators: schema.operators?.length ? schema.operators : [...workbenchFilterOperators],
          type: schema.type,
        }));
      }
      return corpus.filters.map((field) => ({
        field,
        operators: [...workbenchFilterOperators],
        type: "string" as const,
      }));
    })
    .filter((option, index, options) => options.findIndex((item) => item.field === option.field) === index)
    .sort((left, right) => left.field.localeCompare(right.field));
  const filterFieldSchema = Object.fromEntries(filterFieldOptions.map((option) => [option.field, option]));
  const businessFieldOptions = collectWorkbenchFieldOptions(
    selectedIngestCorpus ? [selectedIngestCorpus] : [],
    workbenchFixedPayloadFields,
  );
  const businessFieldSchema = Object.fromEntries(businessFieldOptions.map((option) => [option.field, option]));
  const returnFieldOptions = collectWorkbenchFieldOptions(selectedRetrievalCorpora, workbenchReservedReturnFields);
  const selectedReturnFields = buildReturnFields(returnFieldRows);
  const targetScope = buildTargetScope(targetScopeType, userCode, workspaceCode);

  useEffect(() => {
    if (!corpusId && visibleCorpora[0]?.code) {
      setCorpusId(visibleCorpora[0].code);
    }
  }, [corpusId, visibleCorpora]);

  useEffect(() => {
    if (!ingestProfileId && ingestProfiles[0]?.id) {
      setIngestProfileId(ingestProfiles[0].id);
    }
  }, [ingestProfileId, ingestProfiles]);

  useEffect(() => {
    if (!retrievalProfileId && retrievalProfiles[0]?.id) {
      setRetrievalProfileId(retrievalProfiles[0].id);
    }
  }, [retrievalProfileId, retrievalProfiles]);

  useEffect(() => {
    if (!sourcePayloadEdited) {
      setSourcePayloadForm(buildDefaultPayloadForm(corpusId, selectedIngestProfile));
    }
  }, [corpusId, selectedIngestProfile, sourcePayloadEdited]);

  useEffect(() => {
    if (!contextStrategyOptions.includes(contextStrategy)) {
      setContextStrategy(contextStrategyOptions[0] ?? "DOC_QA_CONTEXT_PACK");
    }
  }, [contextStrategy, contextStrategyOptions]);

  function updateSourcePayloadForm(patch: Partial<WorkbenchSourcePayloadForm>) {
    setSourcePayloadForm((current) => ({ ...current, ...patch }));
    setSourcePayloadEdited(true);
  }

  function buildSearchFilters() {
    const filters: Record<string, unknown> = {};
    filterRows.forEach((row, index) => {
      const field = row.field.trim();
      const operator = row.operator.trim();
      const rawValue = row.value.trim();
      if (!field && !operator && !rawValue) return;
      if (!field || !operator || !rawValue) {
        throw new Error(`Filter ${index + 1} must include field, operator, and value.`);
      }

      const schema = filterFieldSchema[field];
      const valueType = schema?.type ?? "string";
      const value = operator === "in"
        ? parseCommaSeparatedValues(rawValue).map((item) => parseFilterValue(item, valueType, field))
        : parseFilterValue(rawValue, valueType, field);
      if (operator === "in" && Array.isArray(value) && value.length === 0) {
        throw new Error(`Filter ${index + 1} must include at least one value.`);
      }
      appendFilter(filters, field, { op: operator, value });
    });
    return filters;
  }

  async function handleIngestSubmit() {
    setIngestFeedback("");
    let domainFields: Record<string, unknown>;
    try {
      domainFields = buildDomainFields(businessAttributeRows, businessFieldSchema);
    } catch (error) {
      const message = readErrorMessage(error);
      setIngestFeedback(message);
      onNotice(message);
      return;
    }
    const payload = buildPayloadFromForm(sourcePayloadForm, usesDocumentPayload, domainFields);
    if (!sourcePayloadForm.sourceCode.trim() || !sourcePayloadForm.sourceType.trim() || !sourcePayloadForm.sourceVersion.trim()) {
      const message = "Source code, type, and version are required.";
      setIngestFeedback(message);
      onNotice(message);
      return;
    }
    if (usesDocumentPayload && (!sourcePayloadForm.documentUri.trim() || !sourcePayloadForm.documentFormat.trim())) {
      const message = "Document URI and format are required.";
      setIngestFeedback(message);
      onNotice(message);
      return;
    }
    if (!usesDocumentPayload && !sourcePayloadForm.text.trim()) {
      const message = "Text content is required.";
      setIngestFeedback(message);
      onNotice(message);
      return;
    }

    setIngestLoading(true);
    try {
      const result = await runRagIngest({
        accessContext: {
          agentCode: workbenchAgentCode,
          userCode,
          workspaceCode: workspaceCodeForAccessContext(targetScope, workspaceCode),
        },
        corpusId,
        ingestProfileId,
        payload,
        targetScope,
      });
      setIngestResult(result);
      const message = `Ingest completed for ${result.sourceCode || corpusId}.`;
      setIngestFeedback(message);
      onNotice(message);
    } catch (error) {
      const message = readErrorMessage(error);
      setIngestFeedback(message);
      onNotice(message);
    } finally {
      setIngestLoading(false);
    }
  }

  async function handleSearchSubmit() {
    setSearchFeedback("");
    let filters: Record<string, unknown>;
    try {
      filters = buildSearchFilters();
    } catch (error) {
      const message = readErrorMessage(error);
      setSearchFeedback(message);
      onNotice(message);
      return;
    }
    setSearchLoading(true);
    try {
      const result = await runRagSearch({
        accessContext: {
          agentCode: workbenchAgentCode,
          userCode,
          workspaceCode: workspaceCodeForAccessContext(targetScope, workspaceCode),
        },
        context: {
          returnFields: selectedReturnFields,
          strategy: contextStrategy,
        },
        filters,
        limit: Number.parseInt(limit, 10) || 10,
        profileId: retrievalProfileId,
        query,
        tokenBudget: Number.parseInt(tokenBudget, 10) || 1200,
        trace: {
          enabled: traceEnabled,
        },
      });
      setSearchResult(result);
      const hitCount = result.hits?.length ?? 0;
      const message = `Search returned ${hitCount} ${hitCount === 1 ? "hit" : "hits"}.`;
      setSearchFeedback(message);
      onNotice(message);
    } catch (error) {
      const message = readErrorMessage(error);
      setSearchFeedback(message);
      onNotice(message);
    } finally {
      setSearchLoading(false);
    }
  }

  const searchHits = searchResult?.hits ?? [];
  const contextPackItems = searchResult?.contextPack?.items ?? [];

  return (
    <div className="grid gap-5">
      <div className="rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-1">
        <div className="grid gap-1 sm:grid-cols-2" role="tablist" aria-label="Workbench mode">
          {workbenchPanels.map((panel) => {
            const selected = activeWorkbenchPanel === panel.value;
            return (
              <button
                aria-controls={`rag-workbench-${panel.value}-panel`}
                aria-selected={selected}
                className={`rounded-xl px-4 py-3 text-left transition ${
                  selected
                    ? "bg-[var(--color-elevated-surface)] shadow-sm"
                    : "text-[var(--color-on-surface-variant)] hover:bg-[var(--color-surface)]"
                }`}
                id={`rag-workbench-${panel.value}-tab`}
                key={panel.value}
                role="tab"
                type="button"
                onClick={() => setActiveWorkbenchPanel(panel.value)}
              >
                <span className="block font-medium">{panel.label}</span>
                <span className="mt-1 block text-[13px] text-[var(--color-on-surface-variant)]">{panel.summary}</span>
              </button>
            );
          })}
        </div>
      </div>

      {activeWorkbenchPanel === "ingest" ? (
      <SurfaceCard>
        <div
          aria-labelledby="rag-workbench-ingest-tab"
          id="rag-workbench-ingest-panel"
          role="tabpanel"
        >
        <SectionTitle
          icon={<Upload size={17} aria-hidden="true" />}
          title="Ingest Request"
        />
        <div className="mt-4 grid gap-3">
          <Field label="Corpus">
            <SelectInput
              aria-label="Corpus"
              value={corpusId}
              onChange={(event) => {
                setCorpusId(event.target.value);
                setBusinessAttributeRows([]);
                setSourcePayloadEdited(false);
              }}
            >
              {visibleCorpora.map((corpus) => (
                <option key={corpus.code} value={corpus.code}>{corpus.code}</option>
              ))}
            </SelectInput>
          </Field>
          <Field label="Ingest Profile">
            <SelectInput
              aria-label="Ingest Profile"
              value={ingestProfileId}
              onChange={(event) => {
                setIngestProfileId(event.target.value);
                setBusinessAttributeRows([]);
                setSourcePayloadEdited(false);
              }}
            >
              {ingestProfiles.map((profile) => (
                <option key={profile.id} value={profile.id}>{profile.id}</option>
              ))}
            </SelectInput>
          </Field>
          <Field label="Scope Type">
            <SelectInput
              aria-label="Scope Type"
              value={targetScopeType}
              onChange={(event) => setTargetScopeType(event.target.value as WorkbenchScopeType)}
            >
              {workbenchScopeOptions.map((option) => (
                <option key={option.value} value={option.value}>{option.label}</option>
              ))}
            </SelectInput>
            <p className="mt-1 text-[12px] text-[var(--color-on-surface-variant)]">
              Scope code is resolved automatically: <span className="font-mono">{targetScope.scopeCode}</span>
            </p>
          </Field>
          <div className="grid gap-3 sm:grid-cols-2">
            <Field label="Source Code">
              <TextInput aria-label="Source Code" value={sourcePayloadForm.sourceCode} onChange={(event) => updateSourcePayloadForm({ sourceCode: event.target.value })} />
            </Field>
            <Field label="Source Type">
              {sourceTypeOptions.length > 0 ? (
                <SelectInput
                  aria-label="Source Type"
                  value={sourcePayloadForm.sourceType}
                  onChange={(event) => updateSourcePayloadForm({ sourceType: event.target.value })}
                >
                  {sourceTypeOptions.map((sourceType) => (
                    <option key={sourceType} value={sourceType}>{sourceType}</option>
                  ))}
                </SelectInput>
              ) : (
                <TextInput aria-label="Source Type" value={sourcePayloadForm.sourceType} onChange={(event) => updateSourcePayloadForm({ sourceType: event.target.value })} />
              )}
            </Field>
            <Field label="Source Version">
              <TextInput
                aria-label="Source Version"
                disabled={Boolean(requiredSourceVersion)}
                value={sourcePayloadForm.sourceVersion}
                onChange={(event) => updateSourcePayloadForm({ sourceVersion: event.target.value })}
              />
            </Field>
            <Field label="Language">
              <TextInput aria-label="Language" value={sourcePayloadForm.language} onChange={(event) => updateSourcePayloadForm({ language: event.target.value })} />
            </Field>
          </div>
          <Field label="Metadata Source Path">
            <TextInput aria-label="Metadata Source Path" value={sourcePayloadForm.metadataSourcePath} onChange={(event) => updateSourcePayloadForm({ metadataSourcePath: event.target.value })} />
          </Field>
          <div className="rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-3">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">Business Attributes</p>
              <Button
                disabled={businessFieldOptions.length === 0}
                size="sm"
                type="button"
                variant="secondary"
                onClick={() => setBusinessAttributeRows((current) => [...current, createWorkbenchBusinessAttributeRow(current.length)])}
              >
                Add Business Attribute
              </Button>
            </div>
            {businessAttributeRows.length > 0 ? (
              <div className="mt-3 grid gap-3">
                {businessAttributeRows.map((row, index) => (
                  <div className="grid gap-3 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] p-3 xl:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_auto]" key={row.formId}>
                    <Field label={`Business Attribute ${index + 1} Field`}>
                      <SelectInput
                        aria-label={`Business Attribute ${index + 1} Field`}
                        value={row.field}
                        onChange={(event) => setBusinessAttributeRows((current) => current.map((item) => (
                          item.formId === row.formId ? { ...item, field: event.target.value } : item
                        )))}
                      >
                        <option value="">Select field</option>
                        {businessFieldOptions.map((option) => (
                          <option key={option.field} value={option.field}>{option.field}</option>
                        ))}
                      </SelectInput>
                    </Field>
                    <Field label={`Business Attribute ${index + 1} Value`}>
                      <TextInput
                        aria-label={`Business Attribute ${index + 1} Value`}
                        value={row.value}
                        onChange={(event) => setBusinessAttributeRows((current) => current.map((item) => (
                          item.formId === row.formId ? { ...item, value: event.target.value } : item
                        )))}
                      />
                    </Field>
                    <Button
                      aria-label={`Remove business attribute ${index + 1}`}
                      className="self-end"
                      size="sm"
                      type="button"
                      variant="ghost"
                      onClick={() => setBusinessAttributeRows((current) => current.filter((item) => item.formId !== row.formId))}
                    >
                      Remove
                    </Button>
                  </div>
                ))}
              </div>
            ) : null}
          </div>
          {usesDocumentPayload ? (
            <div className="grid gap-3 sm:grid-cols-2">
              <Field label="Document URI">
                <TextInput aria-label="Document URI" value={sourcePayloadForm.documentUri} onChange={(event) => updateSourcePayloadForm({ documentUri: event.target.value })} />
              </Field>
              <Field label="Document Format">
                <TextInput aria-label="Document Format" value={sourcePayloadForm.documentFormat} onChange={(event) => updateSourcePayloadForm({ documentFormat: event.target.value })} />
              </Field>
            </div>
          ) : (
            <Field label="Text Content">
              <TextArea aria-label="Text Content" className="min-h-28" value={sourcePayloadForm.text} onChange={(event) => updateSourcePayloadForm({ text: event.target.value })} />
            </Field>
          )}
          <Button disabled={ingestLoading || !corpusId || !ingestProfileId} onClick={handleIngestSubmit}>
            {ingestLoading ? "Submitting Ingest..." : "Submit Ingest"}
          </Button>
        </div>
        {ingestResult ? (
          <div className="mt-5 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4">
            <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">Ingest Result</p>
            <div className="mt-3 grid gap-2 text-sm">
              <InspectorRow label="Status" value={ingestResult.status ?? "unknown"} />
              <InspectorRow label="Source Code" value={ingestResult.sourceCode ?? sourcePayloadForm.sourceCode} />
              <InspectorRow label="Text Units" value={String(ingestResult.textUnitCount ?? 0)} />
              <InspectorRow label="Fulltext" value={ingestResult.fulltextStatus ?? "not started"} />
              <InspectorRow label="Vector" value={ingestResult.embeddingStatus ?? "not started"} />
            </div>
          </div>
        ) : null}
        <StageResultsList results={ingestResult?.stageResults} title="Ingest Stage Results" />
        {ingestFeedback ? (
          <p className="mt-4 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] px-4 py-3 text-sm text-[var(--color-on-surface)]">
            {ingestFeedback}
          </p>
        ) : null}
        </div>
      </SurfaceCard>
      ) : (
      <SurfaceCard>
        <div
          aria-labelledby="rag-workbench-retrieval-tab"
          id="rag-workbench-retrieval-panel"
          role="tabpanel"
        >
        <SectionTitle
          icon={<Search size={17} aria-hidden="true" />}
          title="Retrieval Test"
        />
        <div className="mt-4 grid gap-3">
          <Field label="Retrieval Profile">
            <SelectInput
              aria-label="Retrieval Profile"
              value={retrievalProfileId}
              onChange={(event) => {
                setRetrievalProfileId(event.target.value);
                setContextStrategy("RESULT_LIST");
                setFilterRows([]);
                setReturnFieldRows([]);
              }}
            >
              {retrievalProfiles.map((profile) => (
                <option key={profile.id} value={profile.id}>{profile.id}</option>
              ))}
            </SelectInput>
          </Field>
          <Field label="Query">
            <TextArea aria-label="Query" className="min-h-24" value={query} onChange={(event) => setQuery(event.target.value)} />
          </Field>
          <Field label="Context Strategy">
            <SelectInput aria-label="Context Strategy" value={contextStrategy} onChange={(event) => setContextStrategy(event.target.value)}>
              {contextStrategyOptions.map((strategy) => (
                <option key={strategy} value={strategy}>{strategy}</option>
              ))}
            </SelectInput>
          </Field>
          <div className="rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-3">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">Query Fields</p>
              <Button
                disabled={returnFieldOptions.length === 0}
                size="sm"
                type="button"
                variant="secondary"
                onClick={() => setReturnFieldRows((current) => [...current, createWorkbenchReturnFieldRow(current.length)])}
              >
                Add Query Field
              </Button>
            </div>
            {returnFieldRows.length > 0 ? (
              <div className="mt-3 grid gap-3">
                {returnFieldRows.map((row, index) => (
                  <div className="grid gap-3 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] p-3 xl:grid-cols-[minmax(0,1fr)_auto]" key={row.formId}>
                    <Field label={`Query Field ${index + 1}`}>
                      <SelectInput
                        aria-label={`Query Field ${index + 1}`}
                        value={row.field}
                        onChange={(event) => setReturnFieldRows((current) => current.map((item) => (
                          item.formId === row.formId ? { ...item, field: event.target.value } : item
                        )))}
                      >
                        <option value="">Select field</option>
                        {returnFieldOptions.map((option) => (
                          <option key={option.field} value={option.field}>{option.field}</option>
                        ))}
                      </SelectInput>
                    </Field>
                    <Button
                      aria-label={`Remove return selection ${index + 1}`}
                      className="self-end"
                      size="sm"
                      type="button"
                      variant="ghost"
                      onClick={() => setReturnFieldRows((current) => current.filter((item) => item.formId !== row.formId))}
                    >
                      Remove
                    </Button>
                  </div>
                ))}
              </div>
            ) : null}
          </div>
          <div className="rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-3">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">Filter Conditions</p>
              <Button
                size="sm"
                type="button"
                variant="secondary"
                onClick={() => setFilterRows((current) => [...current, createWorkbenchFilterRow(current.length)])}
              >
                Add Filter Condition
              </Button>
            </div>
            {filterRows.length > 0 ? (
              <div className="mt-3 grid gap-3">
                {filterRows.map((row, index) => {
                  const selectedField = filterFieldSchema[row.field];
                  const operators = selectedField?.operators?.length ? selectedField.operators : [...workbenchFilterOperators];
                  return (
                    <div className="grid gap-3 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] p-3 xl:grid-cols-[minmax(0,1fr)_120px_minmax(0,1fr)_auto]" key={row.formId}>
                      <Field label={`Filter ${index + 1} Field`}>
                        <SelectInput
                          aria-label={`Filter ${index + 1} Field`}
                          value={row.field}
                          onChange={(event) => setFilterRows((current) => current.map((item) => (
                            item.formId === row.formId ? { ...item, field: event.target.value } : item
                          )))}
                        >
                          <option value="">Select field</option>
                          {filterFieldOptions.map((option) => (
                            <option key={option.field} value={option.field}>{option.field}</option>
                          ))}
                        </SelectInput>
                      </Field>
                      <Field label={`Filter ${index + 1} Operator`}>
                        <SelectInput
                          aria-label={`Filter ${index + 1} Operator`}
                          value={row.operator}
                          onChange={(event) => setFilterRows((current) => current.map((item) => (
                            item.formId === row.formId ? { ...item, operator: event.target.value } : item
                          )))}
                        >
                          {operators.map((operator) => (
                            <option key={operator} value={operator}>{operator}</option>
                          ))}
                        </SelectInput>
                      </Field>
                      <Field label={`Filter ${index + 1} Value`}>
                        <TextInput
                          aria-label={`Filter ${index + 1} Value`}
                          value={row.value}
                          onChange={(event) => setFilterRows((current) => current.map((item) => (
                            item.formId === row.formId ? { ...item, value: event.target.value } : item
                          )))}
                        />
                      </Field>
                      <Button
                        aria-label={`Remove filter ${index + 1}`}
                        className="self-end"
                        size="sm"
                        type="button"
                        variant="ghost"
                        onClick={() => setFilterRows((current) => current.filter((item) => item.formId !== row.formId))}
                      >
                        Remove
                      </Button>
                    </div>
                  );
                })}
              </div>
            ) : null}
          </div>
          <div className="grid gap-3 sm:grid-cols-2">
            <Field label="Limit">
              <TextInput aria-label="Limit" type="number" value={limit} onChange={(event) => setLimit(event.target.value)} />
            </Field>
            <Field label="Token Budget">
              <TextInput aria-label="Token Budget" type="number" value={tokenBudget} onChange={(event) => setTokenBudget(event.target.value)} />
            </Field>
          </div>
          <ToggleSwitch checked={traceEnabled} label="Enable trace" onCheckedChange={onTraceEnabledChange} />
          <Button disabled={searchLoading || retrievalProfileDetailLoading || !retrievalProfileId || !query.trim()} onClick={handleSearchSubmit}>
            {searchLoading ? "Running Search..." : "Run Search"}
          </Button>
        </div>
        <div className="mt-5 grid gap-3">
          <PreviewLine title="Top Hit" value={searchResult?.hits?.[0]?.title ?? searchResult?.hits?.[0]?.sourceCode ?? "waiting"} />
          <PreviewLine title="Context Pack" value={String(searchResult?.contextPack?.tokenCount ?? 0)} />
          <PreviewLine title="Trace Code" value={searchResult?.traceCode ?? (traceEnabled ? "on" : "off")} />
        </div>
        <StageResultsList results={searchResult?.stageResults} title="Search Stage Results" />
        {searchHits.length > 0 ? (
          <div className="mt-5 space-y-3">
            <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">Search Results</p>
            {searchHits.map((hit, index) => {
              const returnedFields = selectedReturnFields
                .map((field) => [field, hit[field]] as const)
                .filter(([, value]) => value !== undefined && value !== null);
              return (
                <article
                  className="rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4 text-sm"
                  key={`${hit.corpusId ?? "hit"}-${hit.sourceCode ?? index}-${hit.sourceVersion ?? "v"}`}
                >
                  <div className="flex flex-wrap items-start justify-between gap-3">
                    <div className="min-w-0">
                      <p className="font-medium text-[var(--color-on-surface)]">
                        {hit.title ?? hit.sourceCode ?? `Hit ${index + 1}`}
                      </p>
                      <p className="mt-1 break-all font-mono text-[12px] text-[var(--color-on-surface-variant)]">
                        {[hit.corpusId, hit.sourceCode, hit.sourceVersion].filter(Boolean).join(" · ")}
                      </p>
                    </div>
                    <span className="rounded-full border border-[var(--color-outline-variant)] px-2.5 py-1 font-mono text-[12px] text-[var(--color-on-surface-variant)]">
                      score {formatScore(hit.score)}
                    </span>
                  </div>
                  {returnedFields.length > 0 ? (
                    <dl className="mt-3 grid gap-2 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] p-3 sm:grid-cols-2">
                      {returnedFields.map(([field, value]) => (
                        <div key={field}>
                          <dt className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">{field}</dt>
                          <dd className="mt-1 break-all text-[var(--color-on-surface)]">{formatFieldValue(value)}</dd>
                        </div>
                      ))}
                    </dl>
                  ) : null}
                  {hit.text ? (
                    <p className="mt-3 line-clamp-4 text-[var(--color-on-surface-variant)]">
                      {hit.text}
                    </p>
                  ) : null}
                </article>
              );
            })}
          </div>
        ) : null}
        {contextPackItems.length > 0 ? (
          <div className="mt-5 space-y-3">
            <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">Context Pack Items</p>
            {contextPackItems.map((item, index) => (
              <article
                className="rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4 text-sm"
                key={`${item.sourceCode ?? "context"}-${index}`}
              >
                <div className="flex flex-wrap items-center justify-between gap-3">
                  <p className="break-all font-mono text-[12px] text-[var(--color-on-surface-variant)]">
                    {item.sourceCode ?? `Context item ${index + 1}`}
                  </p>
                  <span className="font-mono text-[12px] text-[var(--color-on-surface-variant)]">
                    {item.tokens ?? 0} tokens
                  </span>
                </div>
                {item.text ? (
                  <p className="mt-3 line-clamp-3 text-[var(--color-on-surface-variant)]">
                    {item.text}
                  </p>
                ) : null}
              </article>
            ))}
          </div>
        ) : null}
        {searchFeedback ? (
          <p className="mt-4 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] px-4 py-3 text-sm text-[var(--color-on-surface)]">
            {searchFeedback}
          </p>
        ) : null}
        </div>
      </SurfaceCard>
      )}
    </div>
  );
}
