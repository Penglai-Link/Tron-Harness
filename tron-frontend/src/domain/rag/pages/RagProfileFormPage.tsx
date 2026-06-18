import {
  Fragment,
  type FormEvent,
  useEffect,
  useMemo,
  useState,
} from "react";
import {
  useNavigate,
  useParams,
  useSearchParams,
} from "react-router";
import {
  Plus,
  Save,
  Trash2,
  X,
} from "lucide-react";

import {
  Field,
  PageHeader,
  SelectInput,
  SurfaceCard,
  TextArea,
  TextInput,
  ToggleSwitch,
  useToast,
} from "@/shared";
import { Button } from "@/shared/ui/Button";

import {
  getRagMockCorpora,
  getRagMockProfiles,
  listRagMockContextStrategies,
  listRagMockDedupPolicyTypes,
  listRagMockFusionPolicyTypes,
  listRagMockIngestStageStrategies,
  listRagMockTopicPolicyTypes,
  listRagMockRetrievalRouteTypes,
  listRagMockRetrievalStrategies,
  saveRagMockProfile,
} from "@/domain/rag/mock/data";
import {
  buildRagIngestProfileMutationPayload,
  buildRagRetrievalProfileMutationPayload,
  createRagProfile,
  fetchRagPipelineCatalog,
  fetchRagProfileCatalog,
  normalizeRagPipelineCatalog,
  normalizeRagProfileCatalog,
  normalizeRagValidationIssues,
  updateRagProfile,
  validationIssueField,
  validationIssueMessage,
  verifyRagProfile,
  type RagManagementValidationIssue,
  type RagProfileCatalogNormalized,
  type RagProfileVerifyResultApi,
  type RagRetrievalProfileHiddenFields,
} from "@/domain/rag/api/ragManagementApi";
import { formatRagStatusLabel } from "@/domain/rag/components/RagShared";
import { useRagProfileDetail } from "@/domain/rag/hooks/useRagProfileDetail";
import type {
  CorpusRecord,
  RagIngestStageId,
  RagPipelineCatalog,
  RagPipelineStageDefinition,
  RagPipelineTemplate,
  RagProfilePipeline,
  RagProfileStatus,
} from "@/domain/rag/model/RagModel";
import {
  applyIngestStageStrategyDefaults,
  buildIngestPipelineFromForm,
  buildIngestProfileFormState,
  buildIngestProfilePayloadPreview,
  buildIngestProfileRecordFromForm,
  buildRetrievalProfileFormState,
  buildRetrievalProfilePayloadPreview,
  buildRetrievalProfileRecordFromForm,
  createRetrievalFilterFormRow,
  createRetrievalRouteFormRow,
  ingestStageIdToStageType,
  ingestStageNodeId,
  ingestStageTypeToStageId,
  listFilterOperators,
  listProfileStatuses,
  listRetrievalFilterFieldOptions,
  mineruParseStrategyId,
  type RagIngestProfileFormErrors,
  type RagIngestProfileFormState,
  type RagRetrievalFilterFormRow,
  type RagRetrievalProfileFormErrors,
  type RagRetrievalProfileFormState,
  type RagRetrievalRouteFormRow,
  validateIngestProfileForm,
  validateRetrievalProfileForm,
} from "@/domain/rag/model/ragProfileForm";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

type ProfileReturnType = "ingest" | "retrieval";

const profileTypeStorageKey = "tron.rag.profiles.profileType";

function profileReturnTypeFromSearch(value: string | null): ProfileReturnType | null {
  const normalizedValue = value?.toLowerCase();
  return normalizedValue === "ingest" || normalizedValue === "retrieval" ? normalizedValue : null;
}

function rememberProfileReturnType(value: ProfileReturnType) {
  if (typeof window === "undefined") return;
  try {
    window.sessionStorage.setItem(profileTypeStorageKey, value);
  } catch {
    // 浏览器存储不可用时，返回 URL 仍然是状态来源。
  }
}

function profileFormReturnPath(profileType: ProfileReturnType, searchParams?: URLSearchParams) {
  const returnProfileType = profileReturnTypeFromSearch(searchParams?.get("returnProfileType") ?? null) ?? profileType;
  return `/rag?tab=profiles&profileType=${returnProfileType}`;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

type CorpusStorageBackend = "GRAPH" | "SQLITE";

function inferCorpusStorageBackends(corpusCode: string, corpusOptions: CorpusRecord[]) {
  const corpus = corpusOptions.find((candidate) => candidate.code === corpusCode);
  const backends = new Set<CorpusStorageBackend>();
  if (!corpus) return backends;

  const storageText = `${corpus.storageBinding} ${corpus.strategies.join(" ")}`.toLowerCase();
  if (storageText.includes("graph")) backends.add("GRAPH");
  if (storageText.includes("sqlite")) backends.add("SQLITE");
  if (backends.size === 0 && corpus.config) {
    const { storageBinding } = corpus.config;
    storageBinding.targets.forEach((target) => backends.add(target.backend));
  }
  if (backends.size === 0 && corpus.status !== "Reserved") backends.add("SQLITE");
  return backends;
}

function hasGraphBackedCorpus(corpusCode: string, corpusOptions: CorpusRecord[]) {
  return inferCorpusStorageBackends(corpusCode, corpusOptions).has("GRAPH");
}

function inferStrategyBackend(strategyId: string): CorpusStorageBackend | undefined {
  const normalized = strategyId.toUpperCase();
  if (normalized.includes("GRAPH")) return "GRAPH";
  if (normalized.includes("SQLITE")) return "SQLITE";
  return undefined;
}

function isGraphRetrievalStrategy(strategyId: string) {
  const normalized = strategyId.toUpperCase();
  return normalized === "GRAPH" || normalized.includes("GRAPH") && !normalized.includes("GRAPH_EXPANSION");
}

function isRetrievalRouteStrategy(strategyId: string) {
  const normalized = strategyId.toUpperCase();
  if (normalized.includes("GRAPH_EXPANSION")) return false;
  return normalized === "FULLTEXT"
    || normalized === "VECTOR"
    || normalized === "GRAPH"
    || normalized.includes("FULLTEXT")
    || normalized.includes("VECTOR")
    || normalized.includes("STRUCTURED_FILTER");
}

function filterRouteStrategyOptions(
  corpusCode: string,
  corpusOptions: CorpusRecord[],
  strategyIds: string[],
) {
  if (!corpusCode) return [];
  const backends = inferCorpusStorageBackends(corpusCode, corpusOptions);
  return Array.from(new Set(strategyIds.map((strategyId) => strategyId.trim()).filter(Boolean)))
    .filter(isRetrievalRouteStrategy)
    .filter((strategyId) => {
      const strategyBackend = inferStrategyBackend(strategyId);
      if (strategyBackend === "GRAPH") return backends.has("GRAPH");
      if (strategyBackend === "SQLITE") return backends.has("SQLITE");
      return backends.has("SQLITE");
    });
}

function listFallbackRouteStrategies(corpusCode: string, corpusOptions: CorpusRecord[]) {
  return listRagMockRetrievalRouteTypes(corpusCode, corpusOptions)
    .flatMap((routeType) => listRagMockRetrievalStrategies(routeType, corpusCode, corpusOptions));
}

function flattenIngestErrors(errors: RagIngestProfileFormErrors) {
  const messages = Object.values(errors).filter((message): message is string => typeof message === "string");

  Object.values(errors.stageRows ?? {}).forEach((row) => {
    messages.push(...Object.values(row).filter((value): value is string => Boolean(value)));
  });

  return messages;
}

function flattenRetrievalErrors(errors: RagRetrievalProfileFormErrors) {
  const messages = Object.values(errors)
    .filter((value): value is string => typeof value === "string");

  Object.values(errors.routeRows ?? {}).forEach((row) => {
    messages.push(...Object.values(row).filter((value): value is string => Boolean(value)));
  });

  Object.values(errors.filterRows ?? {}).forEach((row) => {
    messages.push(...Object.values(row).filter((value): value is string => Boolean(value)));
  });

  return messages;
}

function SummaryList({ rows }: { rows: Array<{ label: string; value: string }> }) {
  return (
    <div className="grid gap-2 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4 text-sm">
      {rows.map((row) => (
        <div className="flex items-center justify-between gap-4" key={row.label}>
          <span className="text-[var(--color-on-surface-variant)]">{row.label}</span>
          <span className="text-right font-medium">{row.value}</span>
        </div>
      ))}
    </div>
  );
}

function ValidationSummary({ errors }: { errors: string[] }) {
  if (errors.length === 0) return null;

  return (
    <SurfaceCard>
      <h2 className="text-base font-semibold">校验摘要</h2>
      <div className="mt-4 grid gap-2 rounded-xl border border-[var(--color-error-container)] bg-[var(--color-error-container)] p-4 text-sm text-[var(--color-on-error-container)]">
        {errors.map((error) => <p key={error}>{error}</p>)}
      </div>
    </SurfaceCard>
  );
}

function JsonPreview({ value }: { value: unknown }) {
  return (
    <pre className="overflow-x-auto rounded-lg bg-[var(--color-elevated-surface)] p-3 text-[12px] leading-5 text-[var(--color-on-surface)]">
      {JSON.stringify(value, null, 2)}
    </pre>
  );
}

function findPipelineStageDefinition(
  pipelineCatalog: RagPipelineCatalog | null,
  stageType: string,
): RagPipelineStageDefinition | undefined {
  return pipelineCatalog?.stages.find((stage) => stage.stageType === stageType);
}

function strategyOptionsForStage(stageDefinition?: RagPipelineStageDefinition) {
  if (!stageDefinition) return [];
  if (stageDefinition.allowedStrategies.length > 0) return stageDefinition.allowedStrategies;
  return stageDefinition.strategies.map((strategy) => strategy.strategyId);
}

function strategySchemaForNode(stageDefinition: RagPipelineStageDefinition | undefined, strategyId: string) {
  return stageDefinition?.strategies.find((strategy) => strategy.strategyId === strategyId)?.configSchema ?? {};
}

function strategyDefinitionForNode(stageDefinition: RagPipelineStageDefinition | undefined, strategyId: string) {
  return stageDefinition?.strategies.find((strategy) => strategy.strategyId === strategyId);
}

const pipelineStageDisplayNames: Record<string, string> = {
  BUILD_CONTEXT: "构建上下文",
  DEDUP_RESULTS: "结果去重",
  EXTRACT_TREE: "提取结构树",
  FULLTEXT_INDEX: "全文索引",
  FUSE_RESULTS: "结果融合",
  PARSE_DOCUMENT: "文档解析",
  PERSIST_SOURCE: "持久化来源",
  PERSIST_TREE: "持久化结构树",
  PREPARE_PAYLOAD: "准备载荷",
  RESOLVE_SOURCE: "解析来源",
  ROUTE_SEARCH: "路由检索",
  SPLIT_TEXT: "文本切分",
  VALIDATE_SOURCE: "校验来源",
  VECTOR_INDEX: "向量索引",
};

function localizePipelineStageName(value: string) {
  const normalized = value.trim().replace(/[\s-]+/g, "_").toUpperCase();
  return pipelineStageDisplayNames[normalized] ?? value;
}

function strategyOptionLabel(stageDefinition: RagPipelineStageDefinition | undefined, strategyId: string) {
  void stageDefinition;
  return strategyId;
}

const templateDisplayText: Record<string, { description: string; name: string }> = {
  "document-reference": {
    description: "校验文档来源引用，解析并切分文本后写入全文和向量索引。",
    name: "文档引用写入",
  },
  "graph-tree-enhanced": {
    description: "构建全文和向量索引，同时为图语料库提取并持久化结构树分支。",
    name: "图结构增强写入",
  },
  "mineru-document-reference": {
    description: "解析 PDF 或 DOCX 项目文档引用，通过 MinerU 转换为 Markdown 后构建全文和向量索引。",
    name: "MinerU 文档引用写入",
  },
  "text-standard": {
    description: "准备文本载荷、切分文本单元、构建全文和向量索引，然后持久化来源。",
    name: "标准文本写入",
  },
};

function templateName(template: RagPipelineTemplate) {
  return templateDisplayText[template.templateId]?.name ?? template.displayName;
}

function templateDescription(template: RagPipelineTemplate) {
  return templateDisplayText[template.templateId]?.description ?? template.description;
}

function capabilityLabel(capability: string) {
  const labels: Record<string, string> = {
    fulltext: "全文",
    graph: "图",
    vector: "向量",
  };
  return labels[capability.trim().toLowerCase()] ?? capability;
}

const ingestFieldHints = {
  description: "显示在配置列表和详情中的可读描述。",
  embeddingModelCode: "向量嵌入使用的可选模型编码。留空时使用工作区默认嵌入模型。",
  embeddingVersion: "写入向量索引条目的版本标记，默认值为 v1。",
  graphWritePolicy: "图存储语料库可使用的图写入策略 JSON 内容。不写入图数据时留空。",
  parseDocumentFormat: "解析器接受的文档格式。Markdown 解析器使用 MARKDOWN；MinerU 使用逗号分隔列表，例如 PDF, DOCX。",
  parseDocumentUnsupportedFormatPolicy: "不支持格式的处理策略。FAIL_FAST 会在解析前拒绝请求。",
  profileId: "写入接口请求使用的稳定配置标识。",
  primaryCorpus: "该写入配置写入的语料库，同时用于限定可用阶段策略。",
  requiredSourceVersion: "该配置要求的来源版本。默认 v1 与当前写入请求契约一致。",
  resolveSourceAllowedUriSchemes: "来源解析器接受的地址协议，多个值用逗号分隔。项目文档引用应使用 project-doc。",
  resolveSourceMaxBytes: "来源解析器读取的最大文档字节数。文档引用配置对 PDF/DOCX 默认使用 52428800 字节。",
  splitTextHeadingDepth: "用于生成分块路径的最大 Markdown 标题层级。markdownHeading 的有效范围是 1 到 6。",
  splitTextMaxTokens: "每个文本单元的目标最大令牌数，默认 800。",
  splitTextOverlapTokens: "相邻文本单元之间的重叠令牌数，默认 80。",
  splitTextType: "切分器实现。文档解析后建议使用 markdownHeading；直接文本载荷可使用 plainText。",
  status: "当前只有 Ready 状态的配置会保存为运行时可用的 RAG 配置。",
  strategy: "该阶段的执行策略。选择策略后会自动填充下方推荐配置默认值。",
  supportedSourceTypes: "该配置接受的来源类型，多个值用逗号分隔，例如 PROJECT_DOC_REF、MEMORY_CARD 或 SKILL。",
} satisfies Record<string, string>;

const retrievalFieldHints = {
  contextStrategy: "检索结果组装上下文包时使用的策略。至少需要启用一个策略供工作台和接口调用。",
  dedupPolicy: "多路召回结果合并后如何去重，避免同一来源片段重复进入上下文。",
  description: "显示在配置列表和详情中的可读描述。",
  filterField: "该配置固定附加的筛选字段，必须来自主语料库和路由语料库的可筛选字段。",
  filterOperator: "筛选条件使用的比较方式，例如 eq 表示等于，in 表示属于列表。",
  filterValue: "筛选条件的固定值。保存后每次使用该检索配置都会带上这个条件。",
  fusionPolicy: "多条检索路由返回结果后的融合方式。",
  graphExpansionPolicy: "启用图检索路由时用于扩展邻接节点或结构关系的策略。",
  primaryCorpus: "该检索配置的默认语料库，也用于推导可选筛选字段。",
  profileId: "检索接口请求使用的稳定配置标识。",
  rerankModelCode: "用于结果重排的模型编码。启用重排后才会生效，留空时使用默认配置。",
  routeCorpus: "该路由实际查询的语料库。不同语料库会影响可选检索策略。",
  routeStrategy: "该路由使用的召回策略，决定走全文、向量、图或结构化检索。",
  status: "当前只有 Ready 状态的检索配置会作为运行时可用配置。",
  topicPolicy: "上下文包按主题聚合或排序时使用的策略。",
} satisfies Record<string, string>;

function parseDocumentFormatHint(strategyId: string) {
  return strategyId.trim() === mineruParseStrategyId
    ? "MinerU 接受的格式列表，多个值用逗号分隔。默认值为 PDF, DOCX。"
    : ingestFieldHints.parseDocumentFormat;
}

function stageDisplayName(pipelineCatalog: RagPipelineCatalog | null, stageType: string) {
  const displayName = findPipelineStageDefinition(pipelineCatalog, stageType)?.displayName || stageType;
  return localizePipelineStageName(displayName);
}

function pipelineExecutionLevels(pipeline: RagProfilePipeline) {
  const nodes = pipeline.nodes.filter((node) => node.enabled);
  const nodeIds = new Set(nodes.map((node) => node.nodeId));
  const incoming = Object.fromEntries(nodes.map((node) => [node.nodeId, 0]));
  const outgoing: Record<string, string[]> = {};

  pipeline.edges.forEach((edge) => {
    if (!nodeIds.has(edge.from) || !nodeIds.has(edge.to)) return;
    incoming[edge.to] = (incoming[edge.to] ?? 0) + 1;
    outgoing[edge.from] = [...(outgoing[edge.from] ?? []), edge.to];
  });

  let ready = nodes.filter((node) => incoming[node.nodeId] === 0).map((node) => node.nodeId);
  const visited = new Set<string>();
  const levels: string[][] = [];

  while (ready.length > 0) {
    const level = ready.filter((nodeId) => !visited.has(nodeId));
    if (level.length === 0) break;
    level.forEach((nodeId) => visited.add(nodeId));
    levels.push(level);

    const nextReady: string[] = [];
    level.forEach((nodeId) => {
      (outgoing[nodeId] ?? []).forEach((nextNodeId) => {
        incoming[nextNodeId] = Math.max((incoming[nextNodeId] ?? 1) - 1, 0);
        if (incoming[nextNodeId] === 0) nextReady.push(nextNodeId);
      });
    });
    ready = nextReady;
  }

  const byId = Object.fromEntries(nodes.map((node) => [node.nodeId, node]));
  return levels.map((level) => level.map((nodeId) => byId[nodeId]).filter(Boolean));
}

function templateRequiredCapabilities(
  template: RagPipelineTemplate,
  pipelineCatalog: RagPipelineCatalog | null,
) {
  const capabilities = new Set<string>();
  template.pipeline.nodes.forEach((node) => {
    const stageDefinition = findPipelineStageDefinition(pipelineCatalog, node.stageType);
    const strategy = strategyDefinitionForNode(stageDefinition, node.strategyId);
    strategy?.requiredCapabilities?.forEach((capability) => capabilities.add(capability));
  });
  return Array.from(capabilities);
}

function applyIngestTemplateToForm(
  currentForm: RagIngestProfileFormState,
  template: RagPipelineTemplate,
): RagIngestProfileFormState {
  const nodesByStageId = Object.fromEntries(
    template.pipeline.nodes
      .map((node) => [ingestStageTypeToStageId(node.stageType), node] as const)
      .filter(([stageId]) => Boolean(stageId)),
  ) as Partial<Record<RagIngestStageId, RagPipelineTemplate["pipeline"]["nodes"][number]>>;

  let nextForm: RagIngestProfileFormState = {
    ...currentForm,
    pipelineStages: currentForm.pipelineStages.map((stage) => {
      const templateNode = nodesByStageId[stage.id];
      return {
        ...stage,
        enabled: Boolean(templateNode),
        strategyId: templateNode?.strategyId ?? stage.strategyId,
      };
    }),
  };

  template.pipeline.nodes.forEach((node) => {
    const stageId = ingestStageTypeToStageId(node.stageType);
    if (!stageId) return;
    nextForm = applyIngestStageStrategyDefaults(nextForm, stageId, node.strategyId);
  });

  return nextForm;
}

function PipelineEdgesTable({ edges }: { edges: Array<{ from: string; to: string }> }) {
  return (
    <div className="overflow-x-auto rounded-xl border border-[var(--color-outline-variant)]">
      <table className="min-w-full divide-y divide-[var(--color-outline-variant)] text-left text-sm">
        <thead className="bg-[var(--color-surface-container-low)] text-[11px] uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
          <tr>
            <th className="px-4 py-3 font-semibold">来源节点</th>
            <th className="px-4 py-3 font-semibold">目标节点</th>
          </tr>
        </thead>
        <tbody className="divide-y divide-[var(--color-outline-variant)]">
          {edges.length > 0 ? edges.map((edge) => (
            <tr key={`${edge.from}-${edge.to}`}>
              <td className="px-4 py-3 font-mono text-[12px]">{edge.from}</td>
              <td className="px-4 py-3 font-mono text-[12px]">{edge.to}</td>
            </tr>
          )) : (
            <tr>
              <td className="px-4 py-3 text-[var(--color-on-surface-variant)]" colSpan={2}>至少启用两个相连节点后才会生成有向无环图边。</td>
            </tr>
          )}
        </tbody>
      </table>
    </div>
  );
}

function PipelineEdgesCard({ edges }: { edges: Array<{ from: string; to: string }> }) {
  return (
    <SurfaceCard>
      <h2 className="text-base font-semibold">流水线边</h2>
      <div className="mt-4">
        <PipelineEdgesTable edges={edges} />
      </div>
    </SurfaceCard>
  );
}

function IngestTemplateCard({
  graphBackedCorpus,
  onApply,
  pipelineCatalog,
  selectedTemplateId,
  templates,
}: {
  graphBackedCorpus: boolean;
  onApply: (template: RagPipelineTemplate) => void;
  pipelineCatalog: RagPipelineCatalog | null;
  selectedTemplateId: string;
  templates: RagPipelineTemplate[];
}) {
  return (
    <SurfaceCard>
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="text-base font-semibold">流水线模板</h2>
          <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">
            先选择后端声明的写入模式，仅在需要时再调整具体阶段。
          </p>
        </div>
      </div>
      <div className="mt-4 grid gap-3 lg:grid-cols-3">
        {templates.length > 0 ? templates.map((template) => {
          const capabilities = templateRequiredCapabilities(template, pipelineCatalog);
          const requiresGraph = capabilities.includes("graph");
          const disabled = requiresGraph && !graphBackedCorpus;
          const selected = selectedTemplateId === template.templateId;
          return (
            <button
              aria-pressed={selected}
              className={`relative rounded-xl border p-4 text-left transition ${
                selected
                  ? "border-[var(--color-primary)] bg-[var(--color-surface-container-low)] shadow-[inset_4px_0_0_var(--color-primary)]"
                  : "border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] hover:border-[var(--color-outline)]"
              } ${disabled ? "cursor-not-allowed opacity-55" : ""}`}
              disabled={disabled}
              key={template.templateId}
              type="button"
              onClick={() => onApply(template)}
            >
              <div className="flex items-center justify-between gap-3">
                <p className="font-medium">{templateName(template)}</p>
                <span className={`rounded-full border px-2 py-0.5 font-mono text-[11px] ${
                  selected
                    ? "border-[var(--color-primary)] bg-[var(--color-primary-container)] text-[var(--color-on-primary-container)]"
                    : "border-[var(--color-outline-variant)]"
                }`}>
                    {selected ? "已选择" : `${template.pipeline.nodes.length} 个节点`}
                  </span>
              </div>
              {templateDescription(template) ? (
                <p className="mt-2 text-sm text-[var(--color-on-surface-variant)]">{templateDescription(template)}</p>
              ) : null}
              <div className="mt-3 flex flex-wrap gap-2">
                {capabilities.length > 0 ? capabilities.map((capability) => (
                  <span className={`rounded-full px-2 py-0.5 font-mono text-[11px] ${
                    selected
                      ? "bg-[var(--color-primary-container)] text-[var(--color-on-primary-container)]"
                      : "bg-[var(--color-surface)]"
                  }`} key={capability}>
                    {capabilityLabel(capability)}
                  </span>
                )) : (
                  <span className="rounded-full bg-[var(--color-surface)] px-2 py-0.5 text-[11px]">基础</span>
                )}
              </div>
              {disabled ? (
                <p className="mt-3 text-[12px] text-[var(--color-error)]">选择图存储语料库后才能使用此模板。</p>
              ) : null}
            </button>
          );
        }) : (
          <div className="rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4 text-sm text-[var(--color-on-surface-variant)] lg:col-span-3">
            暂无可用的后端模板。请使用下方高级节点控制。
          </div>
        )}
      </div>
    </SurfaceCard>
  );
}

function PipelineFlowCard({
  pipeline,
  pipelineCatalog,
}: {
  pipeline: RagProfilePipeline;
  pipelineCatalog: RagPipelineCatalog | null;
}) {
  const levels = pipelineExecutionLevels(pipeline);
  const nodeCount = pipeline.nodes.filter((node) => node.enabled).length;

  return (
    <SurfaceCard>
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="text-base font-semibold">流水线流程</h2>
          <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">
            保存后的流水线仍是有向无环图；同一列中的分支可以并行运行。
          </p>
        </div>
        <span className="rounded-full border border-[var(--color-outline-variant)] px-3 py-1 text-sm">{nodeCount} 个已启用节点</span>
      </div>
      {levels.length > 0 ? (
        <div className="mt-5 overflow-x-auto">
          <div className="flex min-w-max items-stretch gap-3">
            {levels.map((level, levelIndex) => (
              <div className="flex items-center gap-3" key={`level-${levelIndex}`}>
                <div className="grid min-w-52 gap-3">
                  {level.length > 1 ? (
                    <span className="w-fit rounded-full bg-[var(--color-surface-container-high)] px-2 py-0.5 text-[11px] uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
                      并行
                    </span>
                  ) : null}
                  {level.map((node) => {
                    return (
                      <div className="rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-3" key={node.nodeId}>
                        <p className="font-medium">{stageDisplayName(pipelineCatalog, node.stageType)}</p>
                        <p className="mt-1 font-mono text-[11px] text-[var(--color-on-surface-variant)]">{node.strategyId || "未设置策略"}</p>
                      </div>
                    );
                  })}
                </div>
                {levelIndex < levels.length - 1 ? (
                  <span className="text-xl text-[var(--color-on-surface-variant)]">→</span>
                ) : null}
              </div>
            ))}
          </div>
        </div>
      ) : (
        <p className="mt-4 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4 text-sm text-[var(--color-on-surface-variant)]">
          应用模板或启用节点后即可预览有向无环图。
        </p>
      )}
      <details className="mt-4 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4">
        <summary className="cursor-pointer text-sm font-medium">边详情</summary>
        <div className="mt-3">
          <PipelineEdgesTable edges={pipeline.edges} />
        </div>
      </details>
    </SurfaceCard>
  );
}

function VerifyResultCard({ result }: { result: RagProfileVerifyResultApi | null }) {
  const issues = normalizeRagValidationIssues(result?.errors);
  return (
    <SurfaceCard>
      <h2 className="text-base font-semibold">校验结果</h2>
      <div className="mt-4 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4 text-sm">
        {result ? (
          <>
            <p className="font-medium">{result.valid === false || issues.length > 0 ? "后端拒绝了该配置。" : "后端校验通过。"}</p>
            {issues.length > 0 ? (
              <div className="mt-3 grid gap-2 text-[var(--color-error)]">
                {issues.map((issue, index) => (
                  <p key={`${validationIssueField(issue)}-${index}`}>{validationIssueMessage(issue)}</p>
                ))}
              </div>
            ) : null}
          </>
        ) : (
          <p className="text-[var(--color-on-surface-variant)]">保存配置前会先执行后端校验。</p>
        )}
      </div>
    </SurfaceCard>
  );
}

function mapIngestApiErrors(
  issues: RagManagementValidationIssue[],
  form: RagIngestProfileFormState,
): RagIngestProfileFormErrors {
  const nextErrors: RagIngestProfileFormErrors = {};
  const enabledStages = form.pipelineStages.filter((stage) => stage.enabled);

  issues.forEach((issue) => {
    const field = validationIssueField(issue);
    const message = validationIssueMessage(issue);

    if (field === "id" || field === "profileId") {
      nextErrors.id = nextErrors.id ?? message;
      return;
    }

    if (field === "primaryCorpusId") {
      nextErrors.primaryCorpusId = nextErrors.primaryCorpusId ?? message;
      return;
    }

    if (field === "status") {
      nextErrors.status = nextErrors.status ?? message;
      return;
    }

    if (field === "sourceInputPolicy" || field === "sourceInputPolicy.supportedSourceTypes") {
      nextErrors.supportedSourceTypesText = nextErrors.supportedSourceTypesText ?? message;
      return;
    }

    if (field === "pipeline") {
      nextErrors.pipeline = nextErrors.pipeline ?? message;
      return;
    }

    const stageMatch = field.match(/^pipelineStages\[(\d+)\]\.strategyId$/);
    if (stageMatch) {
      const stage = enabledStages[Number(stageMatch[1])];
      if (!stage) return;
      nextErrors.stageRows = {
        ...(nextErrors.stageRows ?? {}),
        [stage.id]: {
          ...(nextErrors.stageRows?.[stage.id] ?? {}),
          strategyId: message,
        },
      };
    }
  });

  return nextErrors;
}

function mapRetrievalApiErrors(
  issues: RagManagementValidationIssue[],
  form: RagRetrievalProfileFormState,
): RagRetrievalProfileFormErrors {
  const nextErrors: RagRetrievalProfileFormErrors = {};

  issues.forEach((issue) => {
    const field = validationIssueField(issue);
    const message = validationIssueMessage(issue);

    if (field === "id" || field === "profileId") {
      nextErrors.id = nextErrors.id ?? message;
      return;
    }

    if (field === "primaryCorpusId") {
      nextErrors.primaryCorpusId = nextErrors.primaryCorpusId ?? message;
      return;
    }

    if (field === "status") {
      nextErrors.status = nextErrors.status ?? message;
      return;
    }

    if (field === "routes") {
      nextErrors.routes = nextErrors.routes ?? message;
      return;
    }

    if (field === "filters") {
      nextErrors.filters = nextErrors.filters ?? message;
      return;
    }

    if (field === "contextStrategies") {
      nextErrors.contextStrategies = nextErrors.contextStrategies ?? message;
      return;
    }

    if (field === "graphExpansionPolicy") {
      nextErrors.graphExpansionPolicy = nextErrors.graphExpansionPolicy ?? message;
      return;
    }

    const routeMatch = field.match(/^routes\[(\d+)\]\.(corpusId|routeType|strategyId)$/);
    if (routeMatch) {
      const route = form.routes[Number(routeMatch[1])];
      const routeField = routeMatch[2] as "corpusId" | "routeType" | "strategyId";
      if (!route) return;
      nextErrors.routeRows = {
        ...(nextErrors.routeRows ?? {}),
        [route.formId]: {
          ...(nextErrors.routeRows?.[route.formId] ?? {}),
          [routeField]: message,
        },
      };
      return;
    }

    const filterMatch = field.match(/^filters\[(\d+)\]\.(field|operator|value)$/);
    if (filterMatch) {
      const filter = form.filters[Number(filterMatch[1])];
      const filterField = filterMatch[2] as "field" | "operator" | "value";
      if (!filter) return;
      nextErrors.filterRows = {
        ...(nextErrors.filterRows ?? {}),
        [filter.formId]: {
          ...(nextErrors.filterRows?.[filter.formId] ?? {}),
          [filterField]: message,
        },
      };
    }
  });

  return nextErrors;
}

function defaultRetrievalHiddenFields(): RagRetrievalProfileHiddenFields {
  return {
    degradePolicy: "RESULT_LIST",
    requiredSourceVersion: "v1",
  };
}

function PayloadPreview({ payload }: { payload: unknown }) {
  return (
    <SurfaceCard>
      <details>
        <summary className="cursor-pointer text-base font-semibold">开发者预览</summary>
        <p className="mt-2 text-sm text-[var(--color-on-surface-variant)]">
          这里展示发送给配置保存和校验接口的完整管理载荷。
        </p>
        <pre className="mt-4 overflow-x-auto rounded-xl bg-[var(--color-surface-container-low)] p-4 text-[13px] leading-6 text-[var(--color-on-surface)]">
          {JSON.stringify(payload, null, 2)}
        </pre>
      </details>
    </SurfaceCard>
  );
}

function RetrievalFlowCard({ form }: { form: RagRetrievalProfileFormState }) {
  const enabledRoutes = form.routes.filter((route) => route.enabled && route.strategyId.trim());
  const contextStrategy = form.contextStrategies[0] || "RESULT_LIST";
  const steps = [
    {
      label: "路由检索",
      value: enabledRoutes.length > 0 ? `${enabledRoutes.length} 条已启用路由` : "未选择路由",
    },
    { label: "结果融合", value: form.fusionPolicy || "SUM_SCORE" },
    { label: "结果去重", value: form.dedupPolicy || "TEXT_UNIT" },
    { label: "构建上下文", value: contextStrategy },
  ];

  return (
    <SurfaceCard>
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="text-base font-semibold">检索流程</h2>
          <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">
            保存后的配置会并行执行路由检索，然后进行结果处理和上下文构建。
          </p>
        </div>
        <span className="rounded-full border border-[var(--color-outline-variant)] px-3 py-1 text-sm">
          {enabledRoutes.length} 条路由
        </span>
      </div>
      <div className="mt-5 grid gap-3 lg:grid-cols-[1fr_auto_1fr_auto_1fr_auto_1fr] lg:items-center">
        {steps.map((step, index) => (
          <Fragment key={step.label}>
            <div className="rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4">
              <p className="text-sm font-semibold">{step.label}</p>
              <p className="mt-2 break-words font-mono text-[12px] text-[var(--color-on-surface-variant)]">{step.value}</p>
            </div>
            {index < steps.length - 1 ? (
              <span className="hidden text-center text-[var(--color-on-surface-variant)] lg:block">→</span>
            ) : null}
          </Fragment>
        ))}
      </div>
    </SurfaceCard>
  );
}

function RouteRow({
  canRemove,
  corpusOptions,
  error,
  index,
  onRemove,
  onUpdate,
  row,
  strategyOptions,
}: {
  canRemove: boolean;
  corpusOptions: ReturnType<typeof getRagMockCorpora>;
  error?: { corpusId?: string; routeType?: string; strategyId?: string };
  index: number;
  onRemove: () => void;
  onUpdate: (patch: Partial<RagRetrievalRouteFormRow>) => void;
  row: RagRetrievalRouteFormRow;
  strategyOptions: string[];
}) {
  return (
    <div className="space-y-3 rounded-lg border border-[var(--color-outline-variant)] p-3">
      <div className={`grid gap-3 ${canRemove ? "lg:grid-cols-[minmax(0,1fr)_minmax(0,1.4fr)_auto] lg:items-end" : "lg:grid-cols-2"}`}>
        <Field label="语料库" error={error?.corpusId} hint={retrievalFieldHints.routeCorpus}>
          <SelectInput
            aria-label={`路由 ${index + 1} 语料库`}
            className="w-full"
            value={row.corpusId}
            onChange={(event) => onUpdate({ corpusId: event.target.value, strategyId: "" })}
          >
            <option value="">选择语料库</option>
            {corpusOptions.map((corpus) => (
              <option key={corpus.code} value={corpus.code}>{corpus.code}</option>
            ))}
          </SelectInput>
        </Field>
        <Field label="检索策略" error={error?.strategyId} hint={retrievalFieldHints.routeStrategy}>
          <SelectInput
            aria-label={`路由 ${index + 1} 检索策略`}
            className="w-full font-mono"
            value={row.strategyId}
            onChange={(event) => onUpdate({ strategyId: event.target.value })}
          >
            <option value="">{strategyOptions.length > 0 ? "选择路由策略" : "暂无支持的策略"}</option>
            {strategyOptions.map((strategyId) => <option key={strategyId} value={strategyId}>{strategyId}</option>)}
          </SelectInput>
        </Field>
        {canRemove ? (
          <div className="flex items-center justify-end lg:pb-1">
            <Button aria-label={`移除路由 ${index + 1}`} size="icon" type="button" variant="ghost" onClick={onRemove}>
              <Trash2 size={16} aria-hidden="true" />
            </Button>
          </div>
        ) : null}
      </div>
      {row.corpusId && strategyOptions.length === 0 ? (
        <p className="text-[12px] text-[var(--color-on-surface-variant)]">
          该语料库暂无后端支持的检索策略。
        </p>
      ) : null}
      <ToggleSwitch checked={row.enabled} label={`启用路由 ${index + 1}`} onCheckedChange={(checked) => onUpdate({ enabled: checked })} />
    </div>
  );
}

function FilterRow({
  canRemove,
  error,
  fieldOptions,
  index,
  onRemove,
  onUpdate,
  operatorOptions,
  row,
}: {
  canRemove: boolean;
  error?: { field?: string; operator?: string; value?: string };
  fieldOptions: string[];
  index: number;
  onRemove: () => void;
  onUpdate: (patch: Partial<RagRetrievalFilterFormRow>) => void;
  operatorOptions: string[];
  row: RagRetrievalFilterFormRow;
}) {
  return (
    <div className="space-y-3 rounded-lg border border-[var(--color-outline-variant)] p-3">
      <div className={`grid gap-3 ${canRemove ? "lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_minmax(0,1fr)_auto] lg:items-end" : "lg:grid-cols-3"}`}>
        <Field label="字段" error={error?.field} hint={retrievalFieldHints.filterField}>
          <SelectInput
            aria-label={`筛选条件 ${index + 1} 字段`}
            className="w-full"
            value={row.field}
            onChange={(event) => onUpdate({ field: event.target.value })}
          >
            <option value="">选择已映射字段</option>
            {fieldOptions.map((field) => <option key={field} value={field}>{field}</option>)}
          </SelectInput>
        </Field>
        <Field label="操作符" error={error?.operator} hint={retrievalFieldHints.filterOperator}>
          <SelectInput
            aria-label={`筛选条件 ${index + 1} 操作符`}
            className="w-full"
            value={row.operator}
            onChange={(event) => onUpdate({ operator: event.target.value })}
          >
            {operatorOptions.map((operator) => <option key={operator} value={operator}>{operator}</option>)}
          </SelectInput>
        </Field>
        <Field label="值" error={error?.value} hint={retrievalFieldHints.filterValue}>
          <TextInput
            aria-label={`筛选条件 ${index + 1} 值`}
            className="w-full font-mono"
            value={row.value}
            onChange={(event) => onUpdate({ value: event.target.value })}
          />
        </Field>
        {canRemove ? (
          <div className="flex items-center justify-end lg:pb-1">
            <Button aria-label={`移除筛选条件 ${index + 1}`} size="icon" type="button" variant="ghost" onClick={onRemove}>
              <Trash2 size={16} aria-hidden="true" />
            </Button>
          </div>
        ) : null}
      </div>
    </div>
  );
}

export function RagIngestProfileFormPage() {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const toast = useToast();
  const dataSource = getServiceDataSource("rag");
  const isApiMode = dataSource === "api";
  const { profileId } = useParams<{ profileId: string }>();
  const mode = profileId ? "edit" : "create";
  const { loading: detailLoading, profile: detailProfile } = useRagProfileDetail(profileId ?? "");
  const existingProfile = detailProfile?.type === "Ingest" ? detailProfile : undefined;
  const [catalog, setCatalog] = useState<RagProfileCatalogNormalized | null>(null);
  const [pipelineCatalog, setPipelineCatalog] = useState<RagPipelineCatalog | null>(null);
  const [catalogLoading, setCatalogLoading] = useState(isApiMode);
  const [form, setForm] = useState<RagIngestProfileFormState>(() => buildIngestProfileFormState());
  const [errors, setErrors] = useState<RagIngestProfileFormErrors>({});
  const [selectedTemplateId, setSelectedTemplateId] = useState("");
  const [verifyResult, setVerifyResult] = useState<RagProfileVerifyResultApi | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const title = mode === "create" ? "新建写入配置" : "编辑写入配置";
  const returnPath = profileFormReturnPath("ingest", searchParams);
  const payloadPreview = useMemo(
    () => (isApiMode ? buildRagIngestProfileMutationPayload(form, pipelineCatalog) : buildIngestProfilePayloadPreview(form, pipelineCatalog)),
    [form, isApiMode, pipelineCatalog],
  );
  const validationMessages = useMemo(() => flattenIngestErrors(errors), [errors]);
  const profileRecords = useMemo(() => (isApiMode ? [] : getRagMockProfiles()), [isApiMode]);
  const existingIds = useMemo(() => profileRecords.map((profile) => profile.id), [profileRecords]);
  const corpusOptions = useMemo(
    () => (isApiMode ? (catalog?.corpora ?? []) : getRagMockCorpora()),
    [catalog?.corpora, isApiMode],
  );
  const graphBackedCorpus = useMemo(() => hasGraphBackedCorpus(form.primaryCorpusId, corpusOptions), [corpusOptions, form.primaryCorpusId]);
  const requiredStageIds = useMemo(
    () => (pipelineCatalog?.stages ?? [])
      .filter((stage) => stage.orchestration?.required)
      .map((stage) => ingestStageTypeToStageId(stage.stageType))
      .filter((stageId): stageId is RagIngestStageId => Boolean(stageId)),
    [pipelineCatalog],
  );
  const enabledStages = useMemo(() => form.pipelineStages.filter((stage) => stage.enabled), [form.pipelineStages]);
  const ingestPipeline = useMemo(() => buildIngestPipelineFromForm(form, pipelineCatalog), [form, pipelineCatalog]);
  const ingestTemplates = pipelineCatalog?.templates ?? [];
  const allowedStageStrategies = useMemo(
    () => Object.fromEntries(
      enabledStages.map((stage) => {
        const fallbackStrategies = listRagMockIngestStageStrategies(stage.id, form.primaryCorpusId);
        const catalogStrategies = catalog?.stageStrategiesByCorpus[form.primaryCorpusId]?.[stage.id] ?? [];
        const pipelineStrategies = strategyOptionsForStage(findPipelineStageDefinition(pipelineCatalog, ingestStageIdToStageType(stage.id)));
        return [stage.id, isApiMode && pipelineStrategies.length > 0
          ? pipelineStrategies
          : isApiMode && catalogStrategies.length > 0
            ? catalogStrategies
            : fallbackStrategies];
      }),
    ) as Partial<Record<RagIngestStageId, string[]>>,
    [catalog?.stageStrategiesByCorpus, enabledStages, form.primaryCorpusId, isApiMode, pipelineCatalog],
  );
  const loading = isApiMode && (catalogLoading || (mode === "edit" && detailLoading && !existingProfile));

  useEffect(() => {
    rememberProfileReturnType("ingest");
  }, []);

  useEffect(() => {
    if (requiredStageIds.length === 0) return;
    setForm((current) => {
      let changed = false;
      const pipelineStages = current.pipelineStages.map((stage) => {
        if (!requiredStageIds.includes(stage.id) || stage.enabled) return stage;
        changed = true;
        return { ...stage, enabled: true };
      });
      return changed ? { ...current, pipelineStages } : current;
    });
  }, [requiredStageIds]);

  useEffect(() => {
    if (!isApiMode) {
      setCatalogLoading(false);
      return;
    }

    const controller = new AbortController();
    setCatalogLoading(true);

    Promise.all([
      fetchRagProfileCatalog("INGEST", controller.signal),
      fetchRagPipelineCatalog("INGEST", controller.signal),
    ])
      .then(([profileCatalog, nextPipelineCatalog]) => {
        setCatalog(normalizeRagProfileCatalog(profileCatalog));
        setPipelineCatalog(normalizeRagPipelineCatalog(nextPipelineCatalog));
      })
      .catch((error: unknown) => {
        if (controller.signal.aborted) return;
        toast.error(error instanceof Error ? error.message : "配置目录加载失败");
      })
      .finally(() => {
        if (!controller.signal.aborted) setCatalogLoading(false);
      });

    return () => controller.abort();
  }, [isApiMode, toast]);

  useEffect(() => {
    if (mode === "create") {
      setForm(buildIngestProfileFormState());
      setSelectedTemplateId("");
      return;
    }
    if (!existingProfile) return;
    setForm(buildIngestProfileFormState(existingProfile));
    setSelectedTemplateId("custom");
  }, [existingProfile, mode]);

  useEffect(() => {
    if (mode !== "create" || selectedTemplateId || ingestTemplates.length === 0) return;
    const firstTemplate = ingestTemplates[0];
    setForm((currentForm) => {
      if (currentForm.pipelineStages.some((stage) => stage.enabled)) return currentForm;
      return applyIngestTemplateToForm(currentForm, firstTemplate);
    });
    setSelectedTemplateId(firstTemplate.templateId);
  }, [ingestTemplates, mode, selectedTemplateId]);

  function updateForm<K extends keyof RagIngestProfileFormState>(key: K, value: RagIngestProfileFormState[K]) {
    setForm((currentForm) => ({ ...currentForm, [key]: value }));
    setErrors((currentErrors) => {
      if (!(key in currentErrors)) return currentErrors;
      const nextErrors = { ...currentErrors };
      delete nextErrors[key as keyof RagIngestProfileFormErrors];
      return nextErrors;
    });
  }

  function updateStage(stageId: string, enabled: boolean) {
    setForm((currentForm) => ({
      ...currentForm,
      pipelineStages: currentForm.pipelineStages.map((stage) => (stage.id === stageId ? { ...stage, enabled } : stage)),
    }));
    setSelectedTemplateId("custom");
    setErrors((currentErrors) => {
      const nextErrors = { ...currentErrors };
      if (nextErrors.stageRows?.[stageId as keyof typeof nextErrors.stageRows]) {
        nextErrors.stageRows = { ...nextErrors.stageRows };
        delete nextErrors.stageRows[stageId as keyof typeof nextErrors.stageRows];
        if (Object.keys(nextErrors.stageRows).length === 0) delete nextErrors.stageRows;
      }
      delete nextErrors.pipeline;
      return nextErrors;
    });
  }

  function updateStageStrategy(stageId: RagIngestStageId, strategyId: string) {
    setForm((currentForm) => applyIngestStageStrategyDefaults(currentForm, stageId, strategyId));
    setSelectedTemplateId("custom");
    setErrors((currentErrors) => {
      if (!currentErrors.stageRows?.[stageId as keyof typeof currentErrors.stageRows]) return currentErrors;
      const nextErrors = { ...currentErrors };
      nextErrors.stageRows = { ...nextErrors.stageRows };
      delete nextErrors.stageRows[stageId as keyof typeof nextErrors.stageRows];
      if (Object.keys(nextErrors.stageRows).length === 0) delete nextErrors.stageRows;
      return nextErrors;
    });
  }

  function applyTemplate(template: RagPipelineTemplate) {
    setForm((currentForm) => applyIngestTemplateToForm(currentForm, template));
    setSelectedTemplateId(template.templateId);
    setErrors((currentErrors) => {
      const nextErrors = { ...currentErrors };
      delete nextErrors.pipeline;
      delete nextErrors.stageRows;
      return nextErrors;
    });
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setVerifyResult(null);
    const nextErrors = validateIngestProfileForm(form, existingIds, existingProfile?.id, allowedStageStrategies, pipelineCatalog);
    setErrors(nextErrors);
    if (Object.keys(nextErrors).length > 0) return;

    if (!isApiMode) {
      saveRagMockProfile(buildIngestProfileRecordFromForm(form, pipelineCatalog));
      toast.success(mode === "create" ? "写入配置已创建。" : "配置已更新。");
      navigate(returnPath);
      return;
    }

    setSubmitting(true);
    try {
      const payload = buildRagIngestProfileMutationPayload(form, pipelineCatalog);
      const verification = await verifyRagProfile(payload);
      setVerifyResult(verification);
      const issues = normalizeRagValidationIssues(verification.errors);
      if (verification.valid === false || issues.length > 0) {
        setErrors(mapIngestApiErrors(issues, form));
        toast.error(validationIssueMessage(issues[0] ?? { message: "请先处理配置表单错误再保存。" }));
        return;
      }

      const saved = mode === "create"
        ? await createRagProfile(payload)
        : await updateRagProfile(profileId ?? form.id.trim(), payload);
      const savedId = typeof saved.profileId === "string" && saved.profileId.trim()
        ? saved.profileId.trim()
        : form.id.trim();
      toast.success(`${savedId} 已${mode === "create" ? "创建" : "保存"}`);
      navigate(returnPath);
    } catch (error) {
      const errorData = typeof error === "object" && error !== null && "data" in error
        ? (error as { data?: unknown }).data
        : undefined;
      const issues = isRecord(errorData) ? normalizeRagValidationIssues(errorData.errors) : [];
      if (issues.length > 0) {
        setErrors(mapIngestApiErrors(issues, form));
      }
      toast.error(error instanceof Error ? error.message : "写入配置保存失败");
    } finally {
      setSubmitting(false);
    }
  }

  if (mode === "edit" && loading) {
    return (
      <>
        <PageHeader
          title="正在加载写入配置"
          actions={(
            <Button variant="secondary" onClick={() => navigate(returnPath)}>
              <X size={16} aria-hidden="true" />
              取消
            </Button>
          )}
        />
        <SurfaceCard>
          <h2 className="text-base font-semibold">正在加载配置</h2>
          <p className="mt-3 text-sm text-[var(--color-on-surface-variant)]">正在加载写入配置管理数据...</p>
        </SurfaceCard>
      </>
    );
  }

  if (mode === "edit" && !existingProfile) {
    return (
      <>
        <PageHeader
          title="编辑写入配置"
          actions={(
            <Button variant="secondary" onClick={() => navigate(returnPath)}>
              <X size={16} aria-hidden="true" />
              取消
            </Button>
          )}
        />
        <SurfaceCard>
          <h2 className="text-base font-semibold">未找到配置</h2>
          <p className="mt-3 text-sm text-[var(--color-on-surface-variant)]">
            {isApiMode
              ? "无法从 RAG 管理接口加载请求的写入配置。"
              : "模拟注册表中不存在请求的写入配置。"}
          </p>
        </SurfaceCard>
      </>
    );
  }

  return (
    <>
      <PageHeader
        title={title}
        actions={(
          <>
            <Button disabled={loading || submitting} variant="secondary" onClick={() => navigate(returnPath)}>
              <X size={16} aria-hidden="true" />
              取消
            </Button>
            <Button aria-busy={submitting} disabled={loading || submitting} type="submit" form="rag-ingest-profile-form">
              <Save size={16} aria-hidden="true" />
              {submitting ? "正在保存..." : "保存配置"}
            </Button>
          </>
        )}
      />

      <form className="grid gap-5" id="rag-ingest-profile-form" onSubmit={handleSubmit}>
        <SurfaceCard>
          <h2 className="text-base font-semibold">基础信息</h2>
          <div className="mt-4 grid gap-4 lg:grid-cols-2">
            <Field label="配置标识" error={errors.id} hint={ingestFieldHints.profileId}>
              <TextInput
                aria-label="配置标识"
                className="font-mono"
                disabled={mode === "edit"}
                value={form.id}
                onChange={(event) => updateForm("id", event.target.value)}
              />
            </Field>
            <Field label="状态" error={errors.status} hint={ingestFieldHints.status}>
              <SelectInput aria-label="状态" value={form.status} onChange={(event) => updateForm("status", event.target.value as RagProfileStatus)}>
                {listProfileStatuses().map((status) => <option key={status} value={status}>{formatRagStatusLabel(status)}</option>)}
              </SelectInput>
            </Field>
            <Field label="主语料库" error={errors.primaryCorpusId} hint={ingestFieldHints.primaryCorpus}>
              <SelectInput aria-label="主语料库" value={form.primaryCorpusId} onChange={(event) => updateForm("primaryCorpusId", event.target.value)}>
                <option value="">选择语料库</option>
                {corpusOptions.map((corpus) => <option key={corpus.code} value={corpus.code}>{corpus.code}</option>)}
              </SelectInput>
            </Field>
            <Field label="描述" hint={ingestFieldHints.description}>
              <TextArea aria-label="描述" className="min-h-28" value={form.description} onChange={(event) => updateForm("description", event.target.value)} />
            </Field>
          </div>
        </SurfaceCard>

        <IngestTemplateCard
          graphBackedCorpus={graphBackedCorpus}
          pipelineCatalog={pipelineCatalog}
          selectedTemplateId={selectedTemplateId}
          templates={ingestTemplates}
          onApply={applyTemplate}
        />

        <PipelineFlowCard pipeline={ingestPipeline} pipelineCatalog={pipelineCatalog} />

        <SurfaceCard>
          <details>
            <summary className="cursor-pointer text-base font-semibold">高级节点控制</summary>
            <p className="mt-2 text-sm text-[var(--color-on-surface-variant)]">
              当选中的模板需要自定义阶段组合时使用这些控制项。
            </p>
            <div className="mt-4 grid gap-3">
              {form.pipelineStages.map((stage) => {
                const stageType = ingestStageIdToStageType(stage.id);
                const stageDefinition = findPipelineStageDefinition(pipelineCatalog, stageType);
                const required = requiredStageIds.includes(stage.id);
                return (
                  <div className="flex items-center justify-between gap-4 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] px-4 py-3" key={stage.id}>
                    <div>
                      <div className="flex flex-wrap items-center gap-2">
                        <span className="font-medium">{stageDisplayName(pipelineCatalog, stageType)}</span>
                        {required ? (
                          <span className="rounded-full bg-[var(--color-primary-container)] px-2 py-0.5 text-[11px] text-[var(--color-on-primary-container)]">必需</span>
                        ) : null}
                        {stageDefinition?.orchestration?.actsAsBarrier ? (
                          <span className="rounded-full bg-[var(--color-surface)] px-2 py-0.5 text-[11px] text-[var(--color-on-surface-variant)]">控制屏障</span>
                        ) : null}
                      </div>
                      <p className="mt-1 font-mono text-[12px] text-[var(--color-on-surface-variant)]">{ingestStageNodeId(stage.id)} · {stageType}</p>
                      {(stageDefinition?.orchestration?.mustRunBeforeStageTypes.length ?? 0) > 0 ? (
                        <p className="mt-1 text-[12px] text-[var(--color-on-surface-variant)]">
                          必须早于 {stageDefinition?.orchestration?.mustRunBeforeStageTypes.join(", ")}
                        </p>
                      ) : null}
                    </div>
                    <ToggleSwitch checked={stage.enabled} disabled={required} label={`启用 ${stageDisplayName(pipelineCatalog, stageType)} 节点`} onCheckedChange={(checked) => updateStage(stage.id, checked)} />
                  </div>
                );
              })}
              {errors.pipeline ? <p className="text-[12px] text-[var(--color-error)]">{errors.pipeline}</p> : null}
            </div>
          </details>
        </SurfaceCard>

        <SurfaceCard>
          <h2 className="text-base font-semibold">输入契约</h2>
          <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">
            这些字段定义调试台和写入接口对该配置可接受的输入。
          </p>
          <div className="mt-4 grid gap-4 lg:grid-cols-2">
            <Field label="支持的来源类型" error={errors.supportedSourceTypesText} hint={ingestFieldHints.supportedSourceTypes}>
              <TextInput aria-label="支持的来源类型" className="font-mono" value={form.supportedSourceTypesText} onChange={(event) => updateForm("supportedSourceTypesText", event.target.value)} />
            </Field>
            <Field label="要求的来源版本" hint={ingestFieldHints.requiredSourceVersion}>
              <TextInput aria-label="要求的来源版本" className="font-mono" value={form.requiredSourceVersion} onChange={(event) => updateForm("requiredSourceVersion", event.target.value)} />
            </Field>
          </div>
        </SurfaceCard>

        <SurfaceCard>
          <h2 className="text-base font-semibold">阶段策略</h2>
          <div className="mt-4 grid gap-4">
            {enabledStages.length === 0 ? (
              <p className="text-sm text-[var(--color-on-surface-variant)]">请先在上方启用一个或多个阶段，再配置阶段策略。</p>
            ) : null}
            {enabledStages.map((stage) => {
              const stageType = ingestStageIdToStageType(stage.id);
              const stageDefinition = findPipelineStageDefinition(pipelineCatalog, stageType);
              const configSchema = strategySchemaForNode(stageDefinition, stage.strategyId);
              return (
              <div className="rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4" key={stage.id}>
                <div className="flex flex-wrap items-start justify-between gap-3">
                  <div>
                    <p className="font-medium">{stageDisplayName(pipelineCatalog, stageType)}</p>
                    <p className="mt-1 font-mono text-[12px] text-[var(--color-on-surface-variant)]">{stageType}</p>
                  </div>
                  <span className="rounded-full border border-[var(--color-outline-variant)] px-2.5 py-1 font-mono text-[11px] text-[var(--color-on-surface-variant)]">{ingestStageNodeId(stage.id)}</span>
                </div>
                <div className="mt-3 grid gap-4 lg:grid-cols-2">
                  <Field label="策略" error={errors.stageRows?.[stage.id]?.strategyId} hint={ingestFieldHints.strategy}>
                    <SelectInput
                      aria-label={`${stageType} 策略标识`}
                      value={stage.strategyId}
                      onChange={(event) => updateStageStrategy(stage.id, event.target.value)}
                    >
                      <option value="">{(allowedStageStrategies[stage.id]?.length ?? 0) > 0 ? "选择阶段策略" : "暂无支持的策略"}</option>
                      {(allowedStageStrategies[stage.id] ?? []).map((strategyId) => (
                        <option key={strategyId} value={strategyId}>{strategyOptionLabel(stageDefinition, strategyId)}</option>
                      ))}
                    </SelectInput>
                  </Field>

                  {stage.id === "resolveSource" ? (
                    <>
                      <Field label="允许的地址协议" hint={ingestFieldHints.resolveSourceAllowedUriSchemes}>
                        <TextInput
                          aria-label="解析来源允许的地址协议"
                          className="font-mono"
                          value={form.resolveSourceAllowedUriSchemesText}
                          onChange={(event) => updateForm("resolveSourceAllowedUriSchemesText", event.target.value)}
                        />
                      </Field>
                      <Field label="最大字节数" error={errors.resolveSourceMaxBytes} hint={ingestFieldHints.resolveSourceMaxBytes}>
                        <TextInput
                          aria-label="解析来源最大字节数"
                          className="font-mono"
                          value={form.resolveSourceMaxBytes}
                          onChange={(event) => updateForm("resolveSourceMaxBytes", event.target.value)}
                        />
                      </Field>
                    </>
                  ) : null}

                  {stage.id === "parseDocument" ? (
                    <>
                      <Field
                        label={stage.strategyId.trim() === mineruParseStrategyId ? "支持的格式" : "文档格式"}
                        error={errors.parseDocumentFormat}
                        hint={parseDocumentFormatHint(stage.strategyId)}
                      >
                        <TextInput
                          aria-label={stage.strategyId.trim() === mineruParseStrategyId ? "文档解析支持的格式" : "文档解析格式"}
                          className="font-mono"
                          value={form.parseDocumentFormat}
                          onChange={(event) => updateForm("parseDocumentFormat", event.target.value)}
                        />
                      </Field>
                      <Field label="不支持格式策略" hint={ingestFieldHints.parseDocumentUnsupportedFormatPolicy}>
                        <TextInput
                          aria-label="文档解析不支持格式策略"
                          className="font-mono"
                          value={form.parseDocumentUnsupportedFormatPolicy}
                          onChange={(event) => updateForm("parseDocumentUnsupportedFormatPolicy", event.target.value)}
                        />
                      </Field>
                    </>
                  ) : null}

                  {stage.id === "splitText" ? (
                    <>
                      <Field label="切分器类型" error={errors.splitTextType} hint={ingestFieldHints.splitTextType}>
                        <TextInput
                          aria-label="文本切分切分器类型"
                          className="font-mono"
                          value={form.splitTextType}
                          onChange={(event) => updateForm("splitTextType", event.target.value)}
                        />
                      </Field>
                      <Field label="最大令牌数" error={errors.splitTextMaxTokens} hint={ingestFieldHints.splitTextMaxTokens}>
                        <TextInput
                          aria-label="文本切分最大令牌数"
                          className="font-mono"
                          value={form.splitTextMaxTokens}
                          onChange={(event) => updateForm("splitTextMaxTokens", event.target.value)}
                        />
                      </Field>
                      <Field label="重叠令牌数" error={errors.splitTextOverlapTokens} hint={ingestFieldHints.splitTextOverlapTokens}>
                        <TextInput
                          aria-label="文本切分重叠令牌数"
                          className="font-mono"
                          value={form.splitTextOverlapTokens}
                          onChange={(event) => updateForm("splitTextOverlapTokens", event.target.value)}
                        />
                      </Field>
                      <Field label="标题深度" error={errors.splitTextHeadingDepth} hint={ingestFieldHints.splitTextHeadingDepth}>
                        <TextInput
                          aria-label="文本切分标题深度"
                          className="font-mono"
                          value={form.splitTextHeadingDepth}
                          onChange={(event) => updateForm("splitTextHeadingDepth", event.target.value)}
                        />
                      </Field>
                    </>
                  ) : null}

                  {stage.id === "vectorIndex" ? (
                    <>
                      <Field label="嵌入模型编码" hint={ingestFieldHints.embeddingModelCode}>
                        <TextInput
                          aria-label="嵌入模型编码"
                          className="font-mono"
                          value={form.embeddingModelCode}
                          onChange={(event) => updateForm("embeddingModelCode", event.target.value)}
                        />
                      </Field>
                      <Field label="嵌入版本" hint={ingestFieldHints.embeddingVersion}>
                        <TextInput
                          aria-label="嵌入版本"
                          className="font-mono"
                          value={form.embeddingVersion}
                          onChange={(event) => updateForm("embeddingVersion", event.target.value)}
                        />
                      </Field>
                    </>
                  ) : null}
                </div>
                {(allowedStageStrategies[stage.id]?.length ?? 0) === 0 ? (
                  <p className="mt-3 text-[12px] text-[var(--color-on-surface-variant)]">
                    该阶段和语料库暂无可用策略。
                  </p>
                ) : null}
                <details className="mt-4 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface)] p-3">
                  <summary className="cursor-pointer text-sm font-medium">开发者契约</summary>
                  <div className="mt-3 grid gap-3">
                    <div className="grid gap-2 text-[12px] text-[var(--color-on-surface-variant)] lg:grid-cols-2">
                      <p><span className="font-semibold">输入：</span>{stageDefinition?.inputArtifactKeys.join(", ") || "无"}</p>
                      <p><span className="font-semibold">输出：</span>{stageDefinition?.outputArtifactKeys.join(", ") || "无"}</p>
                    </div>
                    <JsonPreview value={configSchema} />
                  </div>
                </details>
              </div>
            );
            })}
            {graphBackedCorpus ? (
              <Field label="图写入策略" hint={ingestFieldHints.graphWritePolicy}>
                <TextInput aria-label="图写入策略" className="font-mono" value={form.graphWritePolicy} onChange={(event) => updateForm("graphWritePolicy", event.target.value)} />
              </Field>
            ) : null}
          </div>
        </SurfaceCard>

        <SurfaceCard>
          <h2 className="text-base font-semibold">流水线摘要</h2>
          <div className="mt-4">
            <SummaryList rows={[
              { label: "主语料库", value: form.primaryCorpusId || "未设置" },
              { label: "已启用阶段", value: String(form.pipelineStages.filter((stage) => stage.enabled).length) },
              {
                label: "载荷策略",
                value: form.pipelineStages.find((stage) => stage.id === "preparePayload")?.strategyId || "未设置",
              },
            ]} />
          </div>
        </SurfaceCard>

        <PayloadPreview payload={payloadPreview} />
        <VerifyResultCard result={verifyResult} />
        <ValidationSummary errors={validationMessages} />
      </form>
    </>
  );
}

export function RagRetrievalProfileFormPage() {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const toast = useToast();
  const dataSource = getServiceDataSource("rag");
  const isApiMode = dataSource === "api";
  const { profileId } = useParams<{ profileId: string }>();
  const mode = profileId ? "edit" : "create";
  const { detail: apiDetail, loading: detailLoading, profile: detailProfile } = useRagProfileDetail(profileId ?? "");
  const existingProfile = detailProfile?.type === "Retrieval" ? detailProfile : undefined;
  const [catalog, setCatalog] = useState<RagProfileCatalogNormalized | null>(null);
  const [pipelineCatalog, setPipelineCatalog] = useState<RagPipelineCatalog | null>(null);
  const [catalogLoading, setCatalogLoading] = useState(isApiMode);
  const [hiddenFields, setHiddenFields] = useState<RagRetrievalProfileHiddenFields>(() => defaultRetrievalHiddenFields());
  const [form, setForm] = useState<RagRetrievalProfileFormState>(() => buildRetrievalProfileFormState());
  const [errors, setErrors] = useState<RagRetrievalProfileFormErrors>({});
  const [verifyResult, setVerifyResult] = useState<RagProfileVerifyResultApi | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const title = mode === "create" ? "新建检索配置" : "编辑检索配置";
  const returnPath = profileFormReturnPath("retrieval", searchParams);
  const operatorOptions = useMemo(() => listFilterOperators(), []);
  const profileRecords = useMemo(() => (isApiMode ? [] : getRagMockProfiles()), [isApiMode]);
  const existingIds = useMemo(() => profileRecords.map((profile) => profile.id), [profileRecords]);
  const corpusOptions = useMemo(
    () => (isApiMode ? (catalog?.corpora ?? []) : getRagMockCorpora()),
    [catalog?.corpora, isApiMode],
  );
  const contextStrategyOptions = useMemo(
    () => (isApiMode ? catalog?.contextStrategies ?? [] : listRagMockContextStrategies()),
    [catalog?.contextStrategies, isApiMode],
  );
  const fusionPolicyOptions = useMemo(
    () => (isApiMode ? catalog?.fusionPolicyTypes ?? [] : listRagMockFusionPolicyTypes()),
    [catalog?.fusionPolicyTypes, isApiMode],
  );
  const dedupPolicyOptions = useMemo(
    () => (isApiMode ? catalog?.dedupPolicyTypes ?? [] : listRagMockDedupPolicyTypes()),
    [catalog?.dedupPolicyTypes, isApiMode],
  );
  const topicPolicyOptions = useMemo(
    () => (isApiMode ? catalog?.topicPolicyTypes ?? [] : listRagMockTopicPolicyTypes()),
    [catalog?.topicPolicyTypes, isApiMode],
  );
  const payloadPreview = useMemo(
    () => (isApiMode ? buildRagRetrievalProfileMutationPayload(form, hiddenFields) : buildRetrievalProfilePayloadPreview(form)),
    [form, hiddenFields, isApiMode],
  );
  const validationMessages = useMemo(() => flattenRetrievalErrors(errors), [errors]);
  const filterFieldOptions = useMemo(() => listRetrievalFilterFieldOptions(corpusOptions, form.routes), [corpusOptions, form.routes]);
  const routeOptionsByFormId = useMemo(
    () => Object.fromEntries(
      form.routes.map((route) => {
        const fallbackStrategies = listFallbackRouteStrategies(route.corpusId, corpusOptions);
        const catalogStrategiesByType = catalog?.routeStrategiesByCorpus[route.corpusId] ?? {};
        const pipelineStrategies = strategyOptionsForStage(findPipelineStageDefinition(pipelineCatalog, "ROUTE_SEARCH"));
        const catalogStrategies = Object.values(catalogStrategiesByType).flat();
        const corpusStrategies = corpusOptions.find((corpus) => corpus.code === route.corpusId)?.strategies ?? [];
        const candidateStrategies = isApiMode
          ? [...pipelineStrategies, ...catalogStrategies, ...corpusStrategies]
          : [...fallbackStrategies, ...corpusStrategies];
        return [route.formId, filterRouteStrategyOptions(
          route.corpusId,
          corpusOptions,
          candidateStrategies.length > 0 ? candidateStrategies : fallbackStrategies,
        )];
      }),
    ) as Record<string, string[]>,
    [catalog?.routeStrategiesByCorpus, corpusOptions, form.routes, isApiMode, pipelineCatalog],
  );
  const enabledGraphRouteExists = useMemo(
    () => form.routes.some((route) => route.enabled && isGraphRetrievalStrategy(route.strategyId)),
    [form.routes],
  );
  const loading = isApiMode && (catalogLoading || (mode === "edit" && detailLoading && !existingProfile));

  useEffect(() => {
    rememberProfileReturnType("retrieval");
  }, []);

  useEffect(() => {
    if (!isApiMode) {
      setCatalogLoading(false);
      return;
    }

    const controller = new AbortController();
    setCatalogLoading(true);

    Promise.all([
      fetchRagProfileCatalog("RETRIEVAL", controller.signal),
      fetchRagPipelineCatalog("RETRIEVAL", controller.signal),
    ])
      .then(([profileCatalog, nextPipelineCatalog]) => {
        setCatalog(normalizeRagProfileCatalog(profileCatalog));
        setPipelineCatalog(normalizeRagPipelineCatalog(nextPipelineCatalog));
      })
      .catch((error: unknown) => {
        if (controller.signal.aborted) return;
        toast.error(error instanceof Error ? error.message : "配置目录加载失败");
      })
      .finally(() => {
        if (!controller.signal.aborted) setCatalogLoading(false);
      });

    return () => controller.abort();
  }, [isApiMode, toast]);

  useEffect(() => {
    if (mode === "create") {
      setForm(buildRetrievalProfileFormState());
      setHiddenFields(defaultRetrievalHiddenFields());
      return;
    }
    if (!existingProfile) return;
    setForm(buildRetrievalProfileFormState(existingProfile));
    if (isApiMode) {
      setHiddenFields({
        degradePolicy: typeof apiDetail?.degradePolicy === "string" && apiDetail.degradePolicy.trim()
          ? apiDetail.degradePolicy.trim()
          : "RESULT_LIST",
        requiredSourceVersion: typeof apiDetail?.requiredSourceVersion === "string" && apiDetail.requiredSourceVersion.trim()
          ? apiDetail.requiredSourceVersion.trim()
          : "v1",
      });
    }
  }, [apiDetail?.degradePolicy, apiDetail?.profileId, apiDetail?.requiredSourceVersion, existingProfile, isApiMode, mode]);

  function updateForm<K extends keyof RagRetrievalProfileFormState>(key: K, value: RagRetrievalProfileFormState[K]) {
    setForm((currentForm) => ({ ...currentForm, [key]: value }));
    setErrors((currentErrors) => {
      if (!(key in currentErrors)) return currentErrors;
      const nextErrors = { ...currentErrors };
      delete nextErrors[key as keyof RagRetrievalProfileFormErrors];
      return nextErrors;
    });
  }

  function toggleContextStrategy(strategyId: string, enabled: boolean) {
    setForm((currentForm) => ({
      ...currentForm,
      contextStrategies: enabled
        ? Array.from(new Set([...currentForm.contextStrategies, strategyId]))
        : currentForm.contextStrategies.filter((strategy) => strategy !== strategyId),
    }));
    setErrors((currentErrors) => {
      if (!currentErrors.contextStrategies) return currentErrors;
      const nextErrors = { ...currentErrors };
      delete nextErrors.contextStrategies;
      return nextErrors;
    });
  }

  function updateRoute(formId: string, patch: Partial<RagRetrievalRouteFormRow>) {
    setForm((currentForm) => ({
      ...currentForm,
      routes: currentForm.routes.map((route) => (route.formId === formId ? { ...route, ...patch } : route)),
    }));
    setErrors((currentErrors) => {
      if (!currentErrors.routeRows?.[formId] && !currentErrors.routes && !currentErrors.graphExpansionPolicy) return currentErrors;
      const nextErrors = { ...currentErrors };
      if (nextErrors.routeRows) {
        nextErrors.routeRows = { ...nextErrors.routeRows };
        delete nextErrors.routeRows[formId];
        if (Object.keys(nextErrors.routeRows).length === 0) delete nextErrors.routeRows;
      }
      delete nextErrors.routes;
      delete nextErrors.graphExpansionPolicy;
      return nextErrors;
    });
  }

  function addRoute() {
    setForm((currentForm) => ({
      ...currentForm,
      routes: [...currentForm.routes, createRetrievalRouteFormRow(currentForm.routes.length)],
    }));
  }

  function removeRoute(formId: string) {
    setForm((currentForm) => ({
      ...currentForm,
      routes: currentForm.routes.filter((route) => route.formId !== formId),
    }));
    setErrors((currentErrors) => {
      const nextErrors = { ...currentErrors };
      if (nextErrors.routeRows) {
        nextErrors.routeRows = { ...nextErrors.routeRows };
        delete nextErrors.routeRows[formId];
        if (Object.keys(nextErrors.routeRows).length === 0) delete nextErrors.routeRows;
      }
      delete nextErrors.routes;
      delete nextErrors.graphExpansionPolicy;
      return nextErrors;
    });
  }

  function updateFilter(formId: string, patch: Partial<RagRetrievalFilterFormRow>) {
    setForm((currentForm) => ({
      ...currentForm,
      filters: currentForm.filters.map((filter) => (filter.formId === formId ? { ...filter, ...patch } : filter)),
    }));
    setErrors((currentErrors) => {
      if (!currentErrors.filterRows?.[formId] && !currentErrors.filters) return currentErrors;
      const nextErrors = { ...currentErrors };
      if (nextErrors.filterRows) {
        nextErrors.filterRows = { ...nextErrors.filterRows };
        delete nextErrors.filterRows[formId];
        if (Object.keys(nextErrors.filterRows).length === 0) delete nextErrors.filterRows;
      }
      delete nextErrors.filters;
      return nextErrors;
    });
  }

  function addFilter() {
    setForm((currentForm) => ({
      ...currentForm,
      filters: [...currentForm.filters, createRetrievalFilterFormRow(currentForm.filters.length)],
    }));
  }

  function removeFilter(formId: string) {
    setForm((currentForm) => ({
      ...currentForm,
      filters: currentForm.filters.filter((filter) => filter.formId !== formId),
    }));
    setErrors((currentErrors) => {
      const nextErrors = { ...currentErrors };
      if (nextErrors.filterRows) {
        nextErrors.filterRows = { ...nextErrors.filterRows };
        delete nextErrors.filterRows[formId];
        if (Object.keys(nextErrors.filterRows).length === 0) delete nextErrors.filterRows;
      }
      delete nextErrors.filters;
      return nextErrors;
    });
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setVerifyResult(null);
    const nextErrors = validateRetrievalProfileForm(form, existingIds, corpusOptions, existingProfile?.id, routeOptionsByFormId);
    setErrors(nextErrors);
    if (Object.keys(nextErrors).length > 0) return;

    if (!isApiMode) {
      saveRagMockProfile(buildRetrievalProfileRecordFromForm(form));
      toast.success(mode === "create" ? "检索配置已创建。" : "配置已更新。");
      navigate(returnPath);
      return;
    }

    setSubmitting(true);
    try {
      const payload = buildRagRetrievalProfileMutationPayload(form, hiddenFields);
      const verification = await verifyRagProfile(payload);
      setVerifyResult(verification);
      const issues = normalizeRagValidationIssues(verification.errors);
      if (verification.valid === false || issues.length > 0) {
        setErrors(mapRetrievalApiErrors(issues, form));
        toast.error(validationIssueMessage(issues[0] ?? { message: "请先处理配置表单错误再保存。" }));
        return;
      }

      const saved = mode === "create"
        ? await createRagProfile(payload)
        : await updateRagProfile(profileId ?? form.id.trim(), payload);
      const savedId = typeof saved.profileId === "string" && saved.profileId.trim()
        ? saved.profileId.trim()
        : form.id.trim();
      toast.success(`${savedId} 已${mode === "create" ? "创建" : "保存"}`);
      navigate(returnPath);
    } catch (error) {
      const errorData = typeof error === "object" && error !== null && "data" in error
        ? (error as { data?: unknown }).data
        : undefined;
      const issues = isRecord(errorData) ? normalizeRagValidationIssues(errorData.errors) : [];
      if (issues.length > 0) {
        setErrors(mapRetrievalApiErrors(issues, form));
      }
      toast.error(error instanceof Error ? error.message : "检索配置保存失败");
    } finally {
      setSubmitting(false);
    }
  }

  if (mode === "edit" && loading) {
    return (
      <>
        <PageHeader
          title="正在加载检索配置"
          actions={(
            <Button variant="secondary" onClick={() => navigate(returnPath)}>
              <X size={16} aria-hidden="true" />
              取消
            </Button>
          )}
        />
        <SurfaceCard>
          <h2 className="text-base font-semibold">正在加载配置</h2>
          <p className="mt-3 text-sm text-[var(--color-on-surface-variant)]">正在加载检索配置管理数据...</p>
        </SurfaceCard>
      </>
    );
  }

  if (mode === "edit" && !existingProfile) {
    return (
      <>
        <PageHeader
          title="编辑检索配置"
          actions={(
            <Button variant="secondary" onClick={() => navigate(returnPath)}>
              <X size={16} aria-hidden="true" />
              取消
            </Button>
          )}
        />
        <SurfaceCard>
          <h2 className="text-base font-semibold">未找到配置</h2>
          <p className="mt-3 text-sm text-[var(--color-on-surface-variant)]">
            {isApiMode
              ? "无法从 RAG 管理接口加载请求的检索配置。"
              : "模拟注册表中不存在请求的检索配置。"}
          </p>
        </SurfaceCard>
      </>
    );
  }

  return (
    <>
      <PageHeader
        title={title}
        actions={(
          <>
            <Button disabled={loading || submitting} variant="secondary" onClick={() => navigate(returnPath)}>
              <X size={16} aria-hidden="true" />
              取消
            </Button>
            <Button aria-busy={submitting} disabled={loading || submitting} type="submit" form="rag-retrieval-profile-form">
              <Save size={16} aria-hidden="true" />
              {submitting ? "正在保存..." : "保存配置"}
            </Button>
          </>
        )}
      />

      <form className="grid gap-5" id="rag-retrieval-profile-form" onSubmit={handleSubmit}>
        <SurfaceCard>
          <h2 className="text-base font-semibold">基础信息</h2>
          <div className="mt-4 grid gap-4 lg:grid-cols-2">
            <Field label="配置标识" error={errors.id} hint={retrievalFieldHints.profileId}>
              <TextInput
                aria-label="配置标识"
                className="font-mono"
                disabled={mode === "edit"}
                value={form.id}
                onChange={(event) => updateForm("id", event.target.value)}
              />
            </Field>
            <Field label="状态" error={errors.status} hint={retrievalFieldHints.status}>
              <SelectInput aria-label="状态" value={form.status} onChange={(event) => updateForm("status", event.target.value as RagProfileStatus)}>
                {listProfileStatuses().map((status) => <option key={status} value={status}>{formatRagStatusLabel(status)}</option>)}
              </SelectInput>
            </Field>
            <Field label="主语料库" error={errors.primaryCorpusId} hint={retrievalFieldHints.primaryCorpus}>
              <SelectInput aria-label="主语料库" value={form.primaryCorpusId} onChange={(event) => updateForm("primaryCorpusId", event.target.value)}>
                <option value="">选择语料库</option>
                {corpusOptions.map((corpus) => <option key={corpus.code} value={corpus.code}>{corpus.code}</option>)}
              </SelectInput>
            </Field>
            <Field label="描述" hint={retrievalFieldHints.description}>
              <TextArea aria-label="描述" className="min-h-28" value={form.description} onChange={(event) => updateForm("description", event.target.value)} />
            </Field>
          </div>
        </SurfaceCard>

        <SurfaceCard>
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div>
              <h2 className="text-base font-semibold">检索路由</h2>
              <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">
                每条路由选择一个语料库和检索策略；策略决定使用全文、向量、图或结构化召回。
              </p>
            </div>
            <Button type="button" variant="secondary" onClick={addRoute}>
              <Plus size={16} aria-hidden="true" />
              添加路由
            </Button>
          </div>
          <div className="mt-4 grid gap-3">
            {form.routes.map((route, index) => (
              <RouteRow
                canRemove={true}
                corpusOptions={corpusOptions}
                error={errors.routeRows?.[route.formId]}
                index={index}
                key={route.formId}
                row={route}
                strategyOptions={routeOptionsByFormId[route.formId] ?? []}
                onRemove={() => removeRoute(route.formId)}
                onUpdate={(patch) => updateRoute(route.formId, patch)}
              />
            ))}
            {errors.routes ? <p className="text-[12px] text-[var(--color-error)]">{errors.routes}</p> : null}
          </div>
        </SurfaceCard>

        <RetrievalFlowCard form={form} />

        <SurfaceCard>
          <h2 className="text-base font-semibold">结果策略</h2>
          <div className="mt-4 grid gap-4 lg:grid-cols-2">
            <Field label="融合策略" hint={retrievalFieldHints.fusionPolicy}>
              <SelectInput aria-label="融合策略" className="font-mono" value={form.fusionPolicy} onChange={(event) => updateForm("fusionPolicy", event.target.value)}>
                {fusionPolicyOptions.map((policy) => <option key={policy} value={policy}>{policy}</option>)}
              </SelectInput>
            </Field>
            <Field label="去重策略" hint={retrievalFieldHints.dedupPolicy}>
              <SelectInput aria-label="去重策略" className="font-mono" value={form.dedupPolicy} onChange={(event) => updateForm("dedupPolicy", event.target.value)}>
                {dedupPolicyOptions.map((policy) => <option key={policy} value={policy}>{policy}</option>)}
              </SelectInput>
            </Field>
            <Field label="主题策略" hint={retrievalFieldHints.topicPolicy}>
              <SelectInput aria-label="主题策略" className="font-mono" value={form.topicPolicy} onChange={(event) => updateForm("topicPolicy", event.target.value)}>
                {topicPolicyOptions.map((policy) => <option key={policy} value={policy}>{policy}</option>)}
              </SelectInput>
            </Field>
            <Field label="重排模型编码" hint={retrievalFieldHints.rerankModelCode}>
              <TextInput aria-label="重排模型编码" className="font-mono" value={form.rerankModelCode} onChange={(event) => updateForm("rerankModelCode", event.target.value)} />
            </Field>
          </div>
          <div className="mt-4">
            <ToggleSwitch checked={form.rerankEnabled} label="启用重排" onCheckedChange={(checked) => updateForm("rerankEnabled", checked)} />
          </div>
        </SurfaceCard>

        <SurfaceCard>
          <h2 className="text-base font-semibold">上下文策略</h2>
          <div className="mt-4 grid gap-3">
            {contextStrategyOptions.map((strategyId) => (
              <div className="flex items-center justify-between gap-4 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] px-4 py-3" key={strategyId}>
                <span className="font-mono text-sm">{strategyId}</span>
                <ToggleSwitch
                  checked={form.contextStrategies.includes(strategyId)}
                  label={`启用上下文策略 ${strategyId}`}
                  onCheckedChange={(checked) => toggleContextStrategy(strategyId, checked)}
                />
              </div>
            ))}
            {errors.contextStrategies ? <p className="text-[12px] text-[var(--color-error)]">{errors.contextStrategies}</p> : null}
          </div>
        </SurfaceCard>

        <SurfaceCard>
          <h2 className="text-base font-semibold">图扩展</h2>
          <div className="mt-4">
            <Field label="图扩展策略" error={errors.graphExpansionPolicy} hint={retrievalFieldHints.graphExpansionPolicy}>
              <TextInput
                aria-label="图扩展策略"
                className="font-mono"
                placeholder={enabledGraphRouteExists ? "" : "选择已启用的图策略后才能使用该策略"}
                value={form.graphExpansionPolicy}
                onChange={(event) => updateForm("graphExpansionPolicy", event.target.value)}
              />
            </Field>
          </div>
        </SurfaceCard>

        <SurfaceCard>
          <div className="flex flex-wrap items-start justify-between gap-3">
            <h2 className="text-base font-semibold">配置筛选条件</h2>
            <Button type="button" variant="secondary" onClick={addFilter}>
              <Plus size={16} aria-hidden="true" />
              添加筛选条件
            </Button>
          </div>
          <div className="mt-4 grid gap-3">
            {form.filters.map((filter, index) => (
              <FilterRow
                canRemove={true}
                error={errors.filterRows?.[filter.formId]}
                fieldOptions={filterFieldOptions}
                index={index}
                key={filter.formId}
                operatorOptions={operatorOptions}
                row={filter}
                onRemove={() => removeFilter(filter.formId)}
                onUpdate={(patch) => updateFilter(filter.formId, patch)}
              />
            ))}
            {errors.filters ? <p className="text-[12px] text-[var(--color-error)]">{errors.filters}</p> : null}
          </div>
        </SurfaceCard>

        <PayloadPreview payload={payloadPreview} />
        <VerifyResultCard result={verifyResult} />
        <ValidationSummary errors={validationMessages} />
      </form>
    </>
  );
}
