import {
  useEffect,
  useState,
} from "react";
import {
  ChevronDown,
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
  formatRagStatusLabel,
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
    label: "写入请求",
    summary: "把来源载荷写入选中的语料库。",
    value: "ingest",
  },
  {
    label: "检索测试",
    summary: "使用查询字段和筛选条件运行检索配置。",
    value: "retrieval",
  },
];
const workbenchScopeOptions: Array<{ label: string; value: WorkbenchScopeType }> = [
  { label: "工作区", value: "SPACE" },
  { label: "用户", value: "USER" },
  { label: "智能体", value: "AGENT" },
];

const workbenchFieldHints = {
  businessField: "写入 payload.domainFields 的业务字段，来自当前语料库字段映射。",
  businessValue: "业务字段对应的值，会按语料库筛选结构中的类型进行转换。",
  contextStrategy: "检索结果组装上下文包时使用的策略，来自当前检索配置。",
  corpus: "本次写入请求要写入的目标语料库。",
  documentFormat: "文档解析格式，例如 PDF、DOCX 或 MARKDOWN。需要与写入配置的解析策略匹配。",
  documentUri: "文档来源地址。项目文档通常使用 project-doc:// 开头。",
  filterField: "本次检索请求附加的筛选字段，来自检索配置绑定语料库的筛选结构。",
  filterOperator: "筛选字段的比较方式，例如 eq 表示等于，in 表示属于列表。",
  filterValue: "筛选字段的查询值。in 操作符可使用逗号分隔多个值。",
  ingestProfile: "本次写入使用的配置，决定来源契约、解析、切分和索引策略。",
  language: "写入 metadata.language 的语言标记，默认 zh-CN。",
  limit: "本次检索最多返回的命中数量。",
  metadataSourcePath: "写入 metadata.sourcePath 的来源路径，便于追踪文档在项目中的位置。",
  query: "本次检索测试的自然语言查询内容。",
  retrievalProfile: "本次检索使用的配置，决定路由、融合、去重和上下文策略。",
  returnField: "要求检索结果额外返回的字段，来自语料库字段映射。",
  scopeType: "请求目标作用域。系统会根据类型自动填充作用域编码。",
  sourceCode: "本次写入来源的唯一编码。同一编码和版本会用于幂等更新。",
  sourceText: "直接文本写入时使用的正文内容。文档引用写入会改用文档地址和格式。",
  sourceType: "来源类型，必须符合所选写入配置的输入契约。",
  sourceVersion: "来源版本，通常为 v1；当写入配置要求固定版本时不可编辑。",
  tokenBudget: "上下文包可使用的最大令牌预算，影响返回给模型的内容长度。",
} satisfies Record<string, string>;

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
    language: "zh-CN",
    metadataSourcePath: "",
    sourceCode,
    sourceType,
    sourceVersion,
    text: `${corpusId || "RAG"} 的调试台写入示例。`,
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
  return error instanceof Error ? error.message : "RAG 调试台请求失败。";
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
  return Number.isFinite(score) ? score!.toFixed(3) : "暂无";
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
      throw new Error(`筛选字段 ${field} 必须是整数。`);
    }
    return parsed;
  }
  if (valueType === "boolean") {
    const normalized = trimmed.toLowerCase();
    if (["true", "1", "yes"].includes(normalized)) return true;
    if (["false", "0", "no"].includes(normalized)) return false;
    throw new Error(`筛选字段 ${field} 必须是 true 或 false。`);
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
      throw new Error(`业务属性 ${index + 1} 必须填写字段和值。`);
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
  return keys && keys.length > 0 ? keys.join(", ") : "无";
}

function StageResultsList({
  results,
  title,
}: {
  results?: RagIngestResponseApi["stageResults"];
  title: string;
}) {
  if (!results || results.length === 0) return null;
  const warningCount = results.reduce((count, result) => count + (result.warnings?.length ?? 0), 0);
  const totalDurationMs = results.reduce((duration, result) => duration + (result.durationMs ?? 0), 0);

  return (
    <details className="group mt-5 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] text-sm">
      <summary className="flex cursor-pointer list-none flex-wrap items-center justify-between gap-3 p-4 marker:hidden [&::-webkit-details-marker]:hidden">
        <div className="flex min-w-0 items-center gap-2">
          <ChevronDown
            className="shrink-0 transition-transform group-open:rotate-180"
            size={16}
            aria-hidden="true"
          />
          <h2 className="truncate text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">{title}</h2>
        </div>
        <div className="flex flex-wrap items-center justify-end gap-2">
          <span className="rounded-full border border-[var(--color-outline-variant)] px-2.5 py-1 text-[12px] text-[var(--color-on-surface-variant)]">
            {results.length} 个阶段 · {totalDurationMs}ms
          </span>
          {warningCount > 0 ? (
            <span className="rounded-full border border-[var(--color-warning)]/30 px-2.5 py-1 text-[12px] text-[var(--color-warning)]">
              {warningCount} 个告警
            </span>
          ) : null}
          <span className="text-[12px] text-[var(--color-on-surface-variant)]">展开查看</span>
        </div>
      </summary>
      <div className="space-y-3 border-t border-[var(--color-outline-variant)] p-3">
        {results.map((result, index) => {
          const warnings = result.warnings ?? [];
          return (
            <article
              className="rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] p-4"
              key={`${result.nodeId}-${index}`}
            >
              <div>
                <p className="font-mono text-[12px] text-[var(--color-on-surface)]">{result.nodeId}</p>
                <p className="mt-1 font-mono text-[12px] text-[var(--color-on-surface-variant)]">
                  {result.stageType} · {result.strategyId}
                </p>
              </div>
              <div className="mt-3 flex flex-wrap items-center gap-2">
                <span className="rounded-full border border-[var(--color-outline-variant)] px-2.5 py-1 font-mono text-[12px] text-[var(--color-on-surface-variant)]">
                  {formatRagStatusLabel(result.status)} · {result.durationMs ?? 0}ms
                </span>
              </div>
              <dl className="mt-3 grid gap-2 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-3 sm:grid-cols-2">
                <div>
                  <dt className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">输入产物</dt>
                  <dd className="mt-1 break-all font-mono text-[12px]">{formatArtifactKeys(result.inputArtifactKeys)}</dd>
                </div>
                <div>
                  <dt className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">输出产物</dt>
                  <dd className="mt-1 break-all font-mono text-[12px]">{formatArtifactKeys(result.outputArtifactKeys)}</dd>
                </div>
              </dl>
              {warnings.length > 0 ? (
                <p className="mt-3 text-[12px] text-[var(--color-warning)]">
                  告警：{warnings.join(", ")}
                </p>
              ) : null}
            </article>
          );
        })}
      </div>
    </details>
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
  const [query, setQuery] = useState("RAG 上下文包应该如何遵守 tokenBudget？");
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
        throw new Error(`筛选条件 ${index + 1} 必须填写字段、操作符和值。`);
      }

      const schema = filterFieldSchema[field];
      const valueType = schema?.type ?? "string";
      const value = operator === "in"
        ? parseCommaSeparatedValues(rawValue).map((item) => parseFilterValue(item, valueType, field))
        : parseFilterValue(rawValue, valueType, field);
      if (operator === "in" && Array.isArray(value) && value.length === 0) {
        throw new Error(`筛选条件 ${index + 1} 至少需要一个值。`);
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
      const message = "来源编码、类型和版本不能为空。";
      setIngestFeedback(message);
      onNotice(message);
      return;
    }
    if (usesDocumentPayload && (!sourcePayloadForm.documentUri.trim() || !sourcePayloadForm.documentFormat.trim())) {
      const message = "文档地址和格式不能为空。";
      setIngestFeedback(message);
      onNotice(message);
      return;
    }
    if (!usesDocumentPayload && !sourcePayloadForm.text.trim()) {
      const message = "文本内容不能为空。";
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
      const message = `${result.sourceCode || corpusId} 写入完成。`;
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
      const message = `检索返回 ${hitCount} 条命中。`;
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
        <div className="grid gap-1 sm:grid-cols-2" role="tablist" aria-label="调试台模式">
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
          title="写入请求"
        />
        <div className="mt-4 grid gap-3">
          <Field label="语料库" hint={workbenchFieldHints.corpus}>
            <SelectInput
              aria-label="语料库"
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
          <Field label="写入配置" hint={workbenchFieldHints.ingestProfile}>
            <SelectInput
              aria-label="写入配置"
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
          <Field label="作用域类型" hint={workbenchFieldHints.scopeType}>
            <SelectInput
              aria-label="作用域类型"
              value={targetScopeType}
              onChange={(event) => setTargetScopeType(event.target.value as WorkbenchScopeType)}
            >
              {workbenchScopeOptions.map((option) => (
                <option key={option.value} value={option.value}>{option.label}</option>
              ))}
            </SelectInput>
            <p className="mt-1 text-[12px] text-[var(--color-on-surface-variant)]">
              作用域编码会自动解析：<span className="font-mono">{targetScope.scopeCode}</span>
            </p>
          </Field>
          <div className="grid gap-3 sm:grid-cols-2">
            <Field label="来源编码" hint={workbenchFieldHints.sourceCode}>
              <TextInput aria-label="来源编码" value={sourcePayloadForm.sourceCode} onChange={(event) => updateSourcePayloadForm({ sourceCode: event.target.value })} />
            </Field>
            <Field label="来源类型" hint={workbenchFieldHints.sourceType}>
              {sourceTypeOptions.length > 0 ? (
                <SelectInput
                  aria-label="来源类型"
                  value={sourcePayloadForm.sourceType}
                  onChange={(event) => updateSourcePayloadForm({ sourceType: event.target.value })}
                >
                  {sourceTypeOptions.map((sourceType) => (
                    <option key={sourceType} value={sourceType}>{sourceType}</option>
                  ))}
                </SelectInput>
              ) : (
                <TextInput aria-label="来源类型" value={sourcePayloadForm.sourceType} onChange={(event) => updateSourcePayloadForm({ sourceType: event.target.value })} />
              )}
            </Field>
            <Field label="来源版本" hint={workbenchFieldHints.sourceVersion}>
              <TextInput
                aria-label="来源版本"
                disabled={Boolean(requiredSourceVersion)}
                value={sourcePayloadForm.sourceVersion}
                onChange={(event) => updateSourcePayloadForm({ sourceVersion: event.target.value })}
              />
            </Field>
            <Field label="语言" hint={workbenchFieldHints.language}>
              <TextInput aria-label="语言" value={sourcePayloadForm.language} onChange={(event) => updateSourcePayloadForm({ language: event.target.value })} />
            </Field>
          </div>
          <Field label="元数据来源路径" hint={workbenchFieldHints.metadataSourcePath}>
            <TextInput aria-label="元数据来源路径" value={sourcePayloadForm.metadataSourcePath} onChange={(event) => updateSourcePayloadForm({ metadataSourcePath: event.target.value })} />
          </Field>
          <div className="rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-3">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">业务属性</p>
              <Button
                disabled={businessFieldOptions.length === 0}
                size="sm"
                type="button"
                variant="secondary"
                onClick={() => setBusinessAttributeRows((current) => [...current, createWorkbenchBusinessAttributeRow(current.length)])}
              >
                添加业务属性
              </Button>
            </div>
            {businessAttributeRows.length > 0 ? (
              <div className="mt-3 grid gap-3">
                {businessAttributeRows.map((row, index) => (
                  <div className="grid gap-3 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] p-3 xl:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_auto]" key={row.formId}>
                    <Field label={`业务属性 ${index + 1} 字段`} hint={workbenchFieldHints.businessField}>
                      <SelectInput
                        aria-label={`业务属性 ${index + 1} 字段`}
                        value={row.field}
                        onChange={(event) => setBusinessAttributeRows((current) => current.map((item) => (
                          item.formId === row.formId ? { ...item, field: event.target.value } : item
                        )))}
                      >
                        <option value="">选择字段</option>
                        {businessFieldOptions.map((option) => (
                          <option key={option.field} value={option.field}>{option.field}</option>
                        ))}
                      </SelectInput>
                    </Field>
                    <Field label={`业务属性 ${index + 1} 值`} hint={workbenchFieldHints.businessValue}>
                      <TextInput
                        aria-label={`业务属性 ${index + 1} 值`}
                        value={row.value}
                        onChange={(event) => setBusinessAttributeRows((current) => current.map((item) => (
                          item.formId === row.formId ? { ...item, value: event.target.value } : item
                        )))}
                      />
                    </Field>
                    <Button
                      aria-label={`移除业务属性 ${index + 1}`}
                      className="self-end"
                      size="sm"
                      type="button"
                      variant="ghost"
                      onClick={() => setBusinessAttributeRows((current) => current.filter((item) => item.formId !== row.formId))}
                    >
                      移除
                    </Button>
                  </div>
                ))}
              </div>
            ) : null}
          </div>
          {usesDocumentPayload ? (
            <div className="grid gap-3 sm:grid-cols-2">
              <Field label="文档地址" hint={workbenchFieldHints.documentUri}>
                <TextInput aria-label="文档地址" value={sourcePayloadForm.documentUri} onChange={(event) => updateSourcePayloadForm({ documentUri: event.target.value })} />
              </Field>
              <Field label="文档格式" hint={workbenchFieldHints.documentFormat}>
                <TextInput aria-label="文档格式" value={sourcePayloadForm.documentFormat} onChange={(event) => updateSourcePayloadForm({ documentFormat: event.target.value })} />
              </Field>
            </div>
          ) : (
            <Field label="文本内容" hint={workbenchFieldHints.sourceText}>
              <TextArea aria-label="文本内容" className="min-h-28" value={sourcePayloadForm.text} onChange={(event) => updateSourcePayloadForm({ text: event.target.value })} />
            </Field>
          )}
          <Button disabled={ingestLoading || !corpusId || !ingestProfileId} onClick={handleIngestSubmit}>
            {ingestLoading ? "正在提交写入..." : "提交写入"}
          </Button>
        </div>
        {ingestResult ? (
          <div className="mt-5 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4">
            <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">写入结果</p>
            <div className="mt-3 grid gap-2 text-sm">
              <InspectorRow label="状态" value={formatRagStatusLabel(ingestResult.status ?? "unknown")} />
              <InspectorRow label="来源编码" value={ingestResult.sourceCode ?? sourcePayloadForm.sourceCode} />
              <InspectorRow label="文本单元" value={String(ingestResult.textUnitCount ?? 0)} />
              <InspectorRow label="全文索引" value={formatRagStatusLabel(ingestResult.fulltextStatus ?? "not started")} />
              <InspectorRow label="向量索引" value={formatRagStatusLabel(ingestResult.embeddingStatus ?? "not started")} />
            </div>
          </div>
        ) : null}
        <StageResultsList results={ingestResult?.stageResults} title="写入阶段结果" />
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
          title="检索测试"
        />
        <div className="mt-4 grid gap-3">
          <Field label="检索配置" hint={workbenchFieldHints.retrievalProfile}>
            <SelectInput
              aria-label="检索配置"
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
          <Field label="查询" hint={workbenchFieldHints.query}>
            <TextArea aria-label="查询" className="min-h-24" value={query} onChange={(event) => setQuery(event.target.value)} />
          </Field>
          <Field label="上下文策略" hint={workbenchFieldHints.contextStrategy}>
            <SelectInput aria-label="上下文策略" value={contextStrategy} onChange={(event) => setContextStrategy(event.target.value)}>
              {contextStrategyOptions.map((strategy) => (
                <option key={strategy} value={strategy}>{strategy}</option>
              ))}
            </SelectInput>
          </Field>
          <div className="rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-3">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">查询字段</p>
              <Button
                disabled={returnFieldOptions.length === 0}
                size="sm"
                type="button"
                variant="secondary"
                onClick={() => setReturnFieldRows((current) => [...current, createWorkbenchReturnFieldRow(current.length)])}
              >
                添加查询字段
              </Button>
            </div>
            {returnFieldRows.length > 0 ? (
              <div className="mt-3 grid gap-3">
                {returnFieldRows.map((row, index) => (
                  <div className="grid gap-3 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] p-3 xl:grid-cols-[minmax(0,1fr)_auto]" key={row.formId}>
                    <Field label={`查询字段 ${index + 1}`} hint={workbenchFieldHints.returnField}>
                      <SelectInput
                        aria-label={`查询字段 ${index + 1}`}
                        value={row.field}
                        onChange={(event) => setReturnFieldRows((current) => current.map((item) => (
                          item.formId === row.formId ? { ...item, field: event.target.value } : item
                        )))}
                      >
                        <option value="">选择字段</option>
                        {returnFieldOptions.map((option) => (
                          <option key={option.field} value={option.field}>{option.field}</option>
                        ))}
                      </SelectInput>
                    </Field>
                    <Button
                      aria-label={`移除返回字段 ${index + 1}`}
                      className="self-end"
                      size="sm"
                      type="button"
                      variant="ghost"
                      onClick={() => setReturnFieldRows((current) => current.filter((item) => item.formId !== row.formId))}
                    >
                      移除
                    </Button>
                  </div>
                ))}
              </div>
            ) : null}
          </div>
          <div className="rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-3">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">筛选条件</p>
              <Button
                size="sm"
                type="button"
                variant="secondary"
                onClick={() => setFilterRows((current) => [...current, createWorkbenchFilterRow(current.length)])}
              >
                添加筛选条件
              </Button>
            </div>
            {filterRows.length > 0 ? (
              <div className="mt-3 grid gap-3">
                {filterRows.map((row, index) => {
                  const selectedField = filterFieldSchema[row.field];
                  const operators = selectedField?.operators?.length ? selectedField.operators : [...workbenchFilterOperators];
                  return (
                    <div className="grid gap-3 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] p-3 xl:grid-cols-[minmax(0,1fr)_120px_minmax(0,1fr)_auto]" key={row.formId}>
                      <Field label={`筛选条件 ${index + 1} 字段`} hint={workbenchFieldHints.filterField}>
                        <SelectInput
                          aria-label={`筛选条件 ${index + 1} 字段`}
                          value={row.field}
                          onChange={(event) => setFilterRows((current) => current.map((item) => (
                            item.formId === row.formId ? { ...item, field: event.target.value } : item
                          )))}
                        >
                          <option value="">选择字段</option>
                          {filterFieldOptions.map((option) => (
                            <option key={option.field} value={option.field}>{option.field}</option>
                          ))}
                        </SelectInput>
                      </Field>
                      <Field label={`筛选条件 ${index + 1} 操作符`} hint={workbenchFieldHints.filterOperator}>
                        <SelectInput
                          aria-label={`筛选条件 ${index + 1} 操作符`}
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
                      <Field label={`筛选条件 ${index + 1} 值`} hint={workbenchFieldHints.filterValue}>
                        <TextInput
                          aria-label={`筛选条件 ${index + 1} 值`}
                          value={row.value}
                          onChange={(event) => setFilterRows((current) => current.map((item) => (
                            item.formId === row.formId ? { ...item, value: event.target.value } : item
                          )))}
                        />
                      </Field>
                      <Button
                        aria-label={`移除筛选条件 ${index + 1}`}
                        className="self-end"
                        size="sm"
                        type="button"
                        variant="ghost"
                        onClick={() => setFilterRows((current) => current.filter((item) => item.formId !== row.formId))}
                      >
                        移除
                      </Button>
                    </div>
                  );
                })}
              </div>
            ) : null}
          </div>
          <div className="grid gap-3 sm:grid-cols-2">
            <Field label="返回数量" hint={workbenchFieldHints.limit}>
              <TextInput aria-label="返回数量" type="number" value={limit} onChange={(event) => setLimit(event.target.value)} />
            </Field>
            <Field label="令牌预算" hint={workbenchFieldHints.tokenBudget}>
              <TextInput aria-label="令牌预算" type="number" value={tokenBudget} onChange={(event) => setTokenBudget(event.target.value)} />
            </Field>
          </div>
          <ToggleSwitch checked={traceEnabled} label="启用追踪" onCheckedChange={onTraceEnabledChange} />
          <Button disabled={searchLoading || retrievalProfileDetailLoading || !retrievalProfileId || !query.trim()} onClick={handleSearchSubmit}>
            {searchLoading ? "正在检索..." : "执行检索"}
          </Button>
        </div>
        <div className="mt-5 grid gap-3">
          <PreviewLine title="首条命中" value={searchResult?.hits?.[0]?.title ?? searchResult?.hits?.[0]?.sourceCode ?? "等待中"} />
          <PreviewLine title="上下文包" value={String(searchResult?.contextPack?.tokenCount ?? 0)} />
          <PreviewLine title="追踪编码" value={searchResult?.traceCode ?? (traceEnabled ? "开启" : "关闭")} />
        </div>
        <StageResultsList results={searchResult?.stageResults} title="检索阶段结果" />
        {searchHits.length > 0 ? (
          <div className="mt-5 space-y-3">
            <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">检索结果</p>
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
                        {hit.title ?? hit.sourceCode ?? `命中 ${index + 1}`}
                      </p>
                      <p className="mt-1 break-all font-mono text-[12px] text-[var(--color-on-surface-variant)]">
                        {[hit.corpusId, hit.sourceCode, hit.sourceVersion].filter(Boolean).join(" · ")}
                      </p>
                    </div>
                    <span className="rounded-full border border-[var(--color-outline-variant)] px-2.5 py-1 font-mono text-[12px] text-[var(--color-on-surface-variant)]">
                      得分 {formatScore(hit.score)}
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
            <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">上下文包条目</p>
            {contextPackItems.map((item, index) => (
              <article
                className="rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4 text-sm"
                key={`${item.sourceCode ?? "context"}-${index}`}
              >
                <div className="flex flex-wrap items-center justify-between gap-3">
                  <p className="break-all font-mono text-[12px] text-[var(--color-on-surface-variant)]">
                    {item.sourceCode ?? `上下文条目 ${index + 1}`}
                  </p>
                  <span className="font-mono text-[12px] text-[var(--color-on-surface-variant)]">
                    {item.tokens ?? 0} 令牌
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
