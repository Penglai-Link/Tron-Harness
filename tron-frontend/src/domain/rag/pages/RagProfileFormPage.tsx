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
import { useTranslation } from "react-i18next";

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
import { i18n } from "@/i18n";

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
  treeLlmSummaryStrategyId,
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
import { SelectItem } from "@/components/ui/select";

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
    // URL state still handles the normal path if browser storage is unavailable.
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

function hasTreeCapableCorpus(corpusCode: string, corpusOptions: CorpusRecord[]) {
  const corpus = corpusOptions.find((candidate) => candidate.code === corpusCode);
  if (!corpus) return false;
  if (corpus.config?.capabilities.tree || corpus.config?.storageBinding.bindings.tree) return true;
  return corpus.strategies.some((strategy) => {
    const normalized = strategy.toUpperCase();
    return normalized.includes("TREE") || normalized.includes("SUMMARY_TREE");
  });
}

function inferStrategyBackend(strategyId: string): CorpusStorageBackend | undefined {
  const normalized = strategyId.toUpperCase();
  if (normalized.includes("GRAPH")) return "GRAPH";
  if (normalized.includes("SQLITE")) return "SQLITE";
  return undefined;
}

function isTreeRetrievalStrategy(strategyId: string) {
  const normalized = strategyId.toUpperCase();
  return normalized.includes("TREE_FULLTEXT") || normalized.includes("TREE_VECTOR");
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

function listRouteStrategyOptions(strategyIds: string[]) {
  return Array.from(new Set(strategyIds.map((strategyId) => strategyId.trim()).filter(Boolean)))
    .filter(isRetrievalRouteStrategy);
}

function filterRouteStrategyOptions(
  corpusCode: string,
  corpusOptions: CorpusRecord[],
  strategyIds: string[],
) {
  if (!corpusCode) return [];
  const supportsTree = hasTreeCapableCorpus(corpusCode, corpusOptions);
  const backends = inferCorpusStorageBackends(corpusCode, corpusOptions);
  return listRouteStrategyOptions(strategyIds)
    .filter((strategyId) => {
      if (isTreeRetrievalStrategy(strategyId)) return supportsTree;
      const strategyBackend = inferStrategyBackend(strategyId);
      if (strategyBackend === "GRAPH") return backends.has("GRAPH");
      if (strategyBackend === "SQLITE") return backends.has("SQLITE");
      return backends.has("SQLITE");
    });
}

function listCatalogRouteStrategyOptions(corpusCode: string, strategyIds: string[]) {
  return corpusCode ? listRouteStrategyOptions(strategyIds) : [];
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
  const { t } = useTranslation("pages");

  if (errors.length === 0) return null;

  return (
    <SurfaceCard>
      <h2 className="text-base font-semibold">
        {t("rag.profileForm.validationSummary.title", { defaultValue: "Validation summary" })}
      </h2>
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
  BUILD_CONTEXT: "Build context",
  DEDUP_RESULTS: "Deduplicate results",
  EXTRACT_TREE: "Extract structure tree",
  FULLTEXT_INDEX: "Fulltext index",
  FUSE_RESULTS: "Fuse results",
  PARSE_DOCUMENT: "Parse document",
  PERSIST_SOURCE: "Persist source",
  PERSIST_TREE: "Persist structure tree",
  PREPARE_PAYLOAD: "Prepare payload",
  RESOLVE_SOURCE: "Resolve source",
  ROUTE_SEARCH: "Route search",
  SPLIT_TEXT: "Split text",
  SUMMARIZE_TREE: "Summarize structure tree",
  TREE_FULLTEXT: "Tree fulltext recall",
  TREE_FULLTEXT_INDEX: "Tree fulltext index",
  TREE_VECTOR: "Tree vector recall",
  TREE_VECTOR_INDEX: "Tree vector index",
  VALIDATE_SOURCE: "Validate source",
  VECTOR_INDEX: "Vector index",
};

function localizePipelineStageName(value: string) {
  const normalized = value.trim().replace(/[\s-]+/g, "_").toUpperCase();
  return i18n.t(`pages:rag.profileForm.stageNames.${normalized}`, {
    defaultValue: pipelineStageDisplayNames[normalized] ?? value,
  });
}

function strategyOptionLabel(stageDefinition: RagPipelineStageDefinition | undefined, strategyId: string) {
  void stageDefinition;
  return strategyId;
}

const templateDisplayText: Record<string, { description: string; name: string }> = {
  "document-reference": {
    description: "Validate a document source reference, parse and split text, then write fulltext and vector indexes.",
    name: "Document reference ingest",
  },
  "graph-tree-enhanced": {
    description: "Build fulltext and vector indexes while extracting and persisting structure tree branches for graph corpora.",
    name: "Graph tree enhanced ingest",
  },
  "mineru-document-reference": {
    description: "Parse PDF or DOCX project document references with MinerU into Markdown, then build fulltext and vector indexes.",
    name: "MinerU document reference ingest",
  },
  "text-standard": {
    description: "Prepare text payloads, split text units, build fulltext and vector indexes, then persist the source.",
    name: "Standard text ingest",
  },
};

function templateName(template: RagPipelineTemplate) {
  return i18n.t(`pages:rag.profileForm.templates.${template.templateId}.name`, {
    defaultValue: templateDisplayText[template.templateId]?.name ?? template.displayName,
  });
}

function templateDescription(template: RagPipelineTemplate) {
  return i18n.t(`pages:rag.profileForm.templates.${template.templateId}.description`, {
    defaultValue: templateDisplayText[template.templateId]?.description ?? template.description,
  });
}

function capabilityLabel(capability: string) {
  const labels: Record<string, string> = {
    fulltext: "Fulltext",
    graph: "Graph",
    tree: "Summary tree",
    vector: "Vector",
  };
  const key = capability.trim().toLowerCase();
  return i18n.t(`pages:rag.profileForm.capabilities.${key}`, {
    defaultValue: labels[key] ?? capability,
  });
}

const ingestFieldHints = {
  description: "Readable description shown in profile lists and details.",
  embeddingModelCode: "Optional model code used for vector embeddings. Leave empty to use the workspace default embedding model.",
  embeddingVersion: "Version tag written to vector index entries. Defaults to v1.",
  graphWritePolicy: "Graph write policy JSON for graph-backed corpora. Leave empty when not writing graph data.",
  parseDocumentFormat: "Document formats accepted by the parser. Markdown parsers use MARKDOWN; MinerU uses comma-separated values such as PDF, DOCX.",
  parseDocumentUnsupportedFormatPolicy: "Policy for unsupported formats. FAIL_FAST rejects requests before parsing.",
  profileId: "Stable profile ID used by ingest API requests.",
  primaryCorpus: "Corpus written by this ingest profile. It also limits available stage strategies.",
  requiredSourceVersion: "Source version required by this profile. v1 matches the current ingest request contract.",
  resolveSourceAllowedUriSchemes: "URI schemes accepted by the source resolver. Separate multiple values with commas. Project document references should use project-doc.",
  resolveSourceMaxBytes: "Maximum document bytes read by the source resolver. Document reference profiles default to 52428800 bytes for PDF/DOCX.",
  splitTextHeadingDepth: "Maximum Markdown heading depth used to generate chunk paths. markdownHeading supports 1 to 6.",
  splitTextMaxTokens: "Target maximum tokens for each text unit. Defaults to 800.",
  splitTextOverlapTokens: "Overlapping tokens between adjacent text units. Defaults to 80.",
  splitTextType: "Splitter implementation. Use markdownHeading after document parsing and plainText for direct text payloads.",
  status: "Only Ready profiles are saved as runtime-available RAG profiles.",
  strategy: "Execution strategy for this stage. Selecting a strategy fills recommended defaults below.",
  supportedSourceTypes: "Source types accepted by this profile. Separate multiple values with commas, such as PROJECT_DOC_REF, MEMORY_CARD, or SKILL.",
  treeSummaryModelCode: "Optional model code used by the LLM tree summary stage. Leave empty to use the workspace default LLM model.",
} satisfies Record<string, string>;

const retrievalFieldHints = {
  contextStrategy: "Strategy used to assemble context packs from retrieval results. At least one strategy must be enabled for workbench and API calls.",
  dedupPolicy: "How multi-route recall results are deduplicated so the same source chunk does not repeat in context.",
  description: "Readable description shown in profile lists and details.",
  filterField: "Fixed filter field attached by this profile. It must come from filterable fields on the primary or route corpora.",
  filterOperator: "Comparison operator used by this filter, such as eq for equals or in for list membership.",
  filterValue: "Fixed filter value. Each request using this retrieval profile includes this condition after save.",
  fusionPolicy: "How results from multiple retrieval routes are fused.",
  graphExpansionPolicy: "Policy used to expand adjacent nodes or structural relations when graph retrieval is enabled.",
  primaryCorpus: "Default corpus for this retrieval profile. It is also used to infer available filter fields.",
  profileId: "Stable profile ID used by retrieval API requests.",
  rerankModelCode: "Model code used for reranking. It only applies when reranking is enabled; leave empty for defaults.",
  routeCorpus: "Corpus queried by this route. Different corpora affect available retrieval strategies.",
  routeStrategy: "Recall strategy used by this route, such as fulltext, vector, graph, or structured retrieval.",
  status: "Only Ready retrieval profiles are runtime-available.",
  topicPolicy: "Policy used to aggregate or order context packs by topic.",
} satisfies Record<string, string>;

function parseDocumentFormatHint(strategyId: string) {
  return strategyId.trim() === mineruParseStrategyId
    ? i18n.t("pages:rag.profileForm.hints.parseDocumentMineruFormat", {
      defaultValue: "Format list accepted by MinerU. Separate multiple values with commas. Defaults to PDF, DOCX.",
    })
    : ingestFieldHints.parseDocumentFormat;
}

function useProfileFormCopy() {
  const { t } = useTranslation("pages");
  const pageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`rag.profileForm.${key}`, { defaultValue, ...options })
  );
  const ingestHintT = (key: keyof typeof ingestFieldHints) => pageT(`hints.ingest.${key}`, ingestFieldHints[key]);
  const retrievalHintT = (key: keyof typeof retrievalFieldHints) => pageT(`hints.retrieval.${key}`, retrievalFieldHints[key]);
  return {
    ingestHintT,
    pageT,
    retrievalHintT,
    t,
  };
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

  const summarizeTreeNode = nodesByStageId.summarizeTree;
  const summarizeTreeConfig = summarizeTreeNode?.config;
  if (
    summarizeTreeNode?.strategyId === treeLlmSummaryStrategyId
    && summarizeTreeConfig
    && typeof summarizeTreeConfig.modelCode === "string"
  ) {
    nextForm = {
      ...nextForm,
      treeSummaryModelCode: summarizeTreeConfig.modelCode,
    };
  }

  template.pipeline.nodes.forEach((node) => {
    const stageId = ingestStageTypeToStageId(node.stageType);
    if (!stageId) return;
    nextForm = applyIngestStageStrategyDefaults(nextForm, stageId, node.strategyId);
  });

  return nextForm;
}

function PipelineEdgesTable({ edges }: { edges: Array<{ from: string; to: string }> }) {
  const { pageT } = useProfileFormCopy();

  return (
    <div className="overflow-x-auto rounded-xl border border-[var(--color-outline-variant)]">
      <table className="min-w-full divide-y divide-[var(--color-outline-variant)] text-left text-sm">
        <thead className="bg-[var(--color-surface-container-low)] text-[11px] uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
          <tr>
            <th className="px-4 py-3 font-semibold">{pageT("pipelineEdges.from", "Source node")}</th>
            <th className="px-4 py-3 font-semibold">{pageT("pipelineEdges.to", "Target node")}</th>
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
              <td className="px-4 py-3 text-[var(--color-on-surface-variant)]" colSpan={2}>{pageT("pipelineEdges.empty", "Enable at least two connected nodes to generate DAG edges.")}</td>
            </tr>
          )}
        </tbody>
      </table>
    </div>
  );
}

function PipelineEdgesCard({ edges }: { edges: Array<{ from: string; to: string }> }) {
  const { pageT } = useProfileFormCopy();

  return (
    <SurfaceCard>
      <h2 className="text-base font-semibold">{pageT("pipelineEdges.title", "Pipeline edges")}</h2>
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
  const { pageT } = useProfileFormCopy();

  return (
    <SurfaceCard>
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="text-base font-semibold">{pageT("templates.title", "Pipeline templates")}</h2>
          <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">
            {pageT("templates.description", "Choose a backend-declared ingest mode first, then adjust individual stages only when needed.")}
          </p>
        </div>
      </div>
      <div className="mt-4 grid min-w-0 gap-3 lg:grid-cols-[repeat(3,minmax(0,1fr))]">
        {templates.length > 0 ? templates.map((template) => {
          const capabilities = templateRequiredCapabilities(template, pipelineCatalog);
          const requiresGraph = capabilities.includes("graph");
          const disabled = requiresGraph && !graphBackedCorpus;
          const selected = selectedTemplateId === template.templateId;
          return (
            <button
              aria-pressed={selected}
              className={`relative min-w-0 rounded-xl border p-4 text-left transition ${selected
                ? "border-[var(--color-primary)] bg-[var(--color-surface-container-low)] shadow-[inset_4px_0_0_var(--color-primary)]"
                : "border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] hover:border-[var(--color-outline)]"
                } ${disabled ? "cursor-not-allowed opacity-55" : ""}`}
              disabled={disabled}
              key={template.templateId}
              type="button"
              onClick={() => onApply(template)}
            >
              <div className="flex min-w-0 items-center justify-between gap-3">
                <p className="min-w-0 break-words font-medium">{templateName(template)}</p>
                <span className={`rounded-full border px-2 py-0.5 font-mono text-[11px] ${selected
                  ? "border-[var(--color-primary)] bg-[var(--color-primary-container)] text-[var(--color-on-primary-container)]"
                  : "border-[var(--color-outline-variant)]"
                  }`}>
                  {selected ? pageT("templates.selected", "Selected") : pageT("templates.nodeCount", "{{count}} nodes", { count: template.pipeline.nodes.length })}
                </span>
              </div>
              {templateDescription(template) ? (
                <p className="mt-2 break-words text-sm text-[var(--color-on-surface-variant)]">{templateDescription(template)}</p>
              ) : null}
              <div className="mt-3 flex flex-wrap gap-2">
                {capabilities.length > 0 ? capabilities.map((capability) => (
                  <span className={`rounded-full px-2 py-0.5 font-mono text-[11px] ${selected
                    ? "bg-[var(--color-primary-container)] text-[var(--color-on-primary-container)]"
                    : "bg-[var(--color-surface)]"
                    }`} key={capability}>
                    {capabilityLabel(capability)}
                  </span>
                )) : (
                  <span className="rounded-full bg-[var(--color-surface)] px-2 py-0.5 text-[11px]">{pageT("templates.basic", "Basic")}</span>
                )}
              </div>
              {disabled ? (
                <p className="mt-3 text-[12px] text-[var(--color-error)]">{pageT("templates.graphRequired", "Select a graph-backed corpus before using this template.")}</p>
              ) : null}
            </button>
          );
        }) : (
          <div className="rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4 text-sm text-[var(--color-on-surface-variant)] lg:col-span-3">
            {pageT("templates.empty", "No backend templates are available. Use advanced node controls below.")}
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
  const { pageT } = useProfileFormCopy();
  const levels = pipelineExecutionLevels(pipeline);
  const nodeCount = pipeline.nodes.filter((node) => node.enabled).length;
  const enabledNodeIds = new Set(pipeline.nodes.filter((node) => node.enabled).map((node) => node.nodeId));
  const outgoingTargets = pipeline.edges.reduce<Record<string, string[]>>((targetsByNode, edge) => {
    if (!enabledNodeIds.has(edge.from) || !enabledNodeIds.has(edge.to)) return targetsByNode;
    targetsByNode[edge.from] = [...(targetsByNode[edge.from] ?? []), edge.to];
    return targetsByNode;
  }, {});

  return (
    <SurfaceCard data-testid="pipeline-flow">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="text-base font-semibold">{pageT("pipelineFlow.title", "Pipeline flow")}</h2>
          <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">
            {pageT("pipelineFlow.description", "Saved pipelines remain DAGs. Branches in the same column can run in parallel, and node cards show real outgoing edges.")}
          </p>
        </div>
        <span className="rounded-full border border-[var(--color-outline-variant)] px-3 py-1 text-sm">{pageT("pipelineFlow.enabledNodes", "{{count}} enabled nodes", { count: nodeCount })}</span>
      </div>
      {levels.length > 0 ? (
        <div className="mt-5 overflow-x-auto">
          <div className="flex min-w-max items-stretch gap-3">
            {levels.map((level, levelIndex) => (
              <div className="grid min-w-56 gap-3" key={`level-${levelIndex}`}>
                <div className="flex min-h-6 items-center justify-between gap-2">
                  {level.length > 1 ? (
                    <span className="w-fit rounded-full bg-[var(--color-surface-container-high)] px-2 py-0.5 text-[11px] uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
                      {pageT("pipelineFlow.parallel", "Parallel")}
                    </span>
                  ) : (
                    <span />
                  )}
                  <span className="font-mono text-[11px] text-[var(--color-on-surface-variant)]">
                    {pageT("pipelineFlow.level", "Level {{index}}", { index: levelIndex + 1 })}
                  </span>
                </div>
                {level.map((node) => {
                  const targets = outgoingTargets[node.nodeId] ?? [];
                  return (
                    <div
                      className="min-w-0 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-3"
                      data-testid={`pipeline-flow-node-${node.nodeId}`}
                      key={node.nodeId}
                    >
                      <p className="font-medium">{stageDisplayName(pipelineCatalog, node.stageType)}</p>
                      <p className="mt-1 break-all font-mono text-[11px] text-[var(--color-on-surface-variant)]">{node.strategyId || pageT("values.noStrategy", "No strategy set")}</p>
                      <div className="mt-3 border-t border-[var(--color-outline-variant)] pt-2">
                        <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">{pageT("pipelineFlow.targetNodes", "Target nodes")}</p>
                        <div className="mt-2 flex flex-wrap gap-1.5">
                          {targets.length > 0 ? targets.map((target) => (
                            <span className="rounded-full border border-[var(--color-outline-variant)] px-2 py-0.5 font-mono text-[11px] text-[var(--color-on-surface-variant)]" key={`${node.nodeId}-${target}`}>
                              → {target}
                            </span>
                          )) : (
                            <span className="rounded-full border border-[var(--color-outline-variant)] px-2 py-0.5 text-[11px] text-[var(--color-on-surface-variant)]">{pageT("pipelineFlow.endpoint", "Endpoint")}</span>
                          )}
                        </div>
                      </div>
                    </div>
                  );
                })}
              </div>
            ))}
          </div>
        </div>
      ) : (
        <p className="mt-4 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4 text-sm text-[var(--color-on-surface-variant)]">
          {pageT("pipelineFlow.empty", "Apply a template or enable nodes to preview the DAG.")}
        </p>
      )}
      <details className="mt-4 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4">
        <summary className="cursor-pointer text-sm font-medium">{pageT("pipelineFlow.edgeDetails", "Edge details")}</summary>
        <div className="mt-3">
          <PipelineEdgesTable edges={pipeline.edges} />
        </div>
      </details>
    </SurfaceCard>
  );
}

function VerifyResultCard({ result }: { result: RagProfileVerifyResultApi | null }) {
  const { pageT } = useProfileFormCopy();
  const issues = normalizeRagValidationIssues(result?.errors);
  return (
    <SurfaceCard>
      <h2 className="text-base font-semibold">{pageT("verify.title", "Verification result")}</h2>
      <div className="mt-4 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4 text-sm">
        {result ? (
          <>
            <p className="font-medium">{result.valid === false || issues.length > 0 ? pageT("verify.rejected", "The backend rejected this profile.") : pageT("verify.passed", "Backend verification passed.")}</p>
            {issues.length > 0 ? (
              <div className="mt-3 grid gap-2 text-[var(--color-error)]">
                {issues.map((issue, index) => (
                  <p key={`${validationIssueField(issue)}-${index}`}>{validationIssueMessage(issue)}</p>
                ))}
              </div>
            ) : null}
          </>
        ) : (
          <p className="text-[var(--color-on-surface-variant)]">{pageT("verify.pending", "Backend verification runs before saving the profile.")}</p>
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
  const { pageT } = useProfileFormCopy();

  return (
    <SurfaceCard>
      <details>
        <summary className="cursor-pointer text-base font-semibold">{pageT("payloadPreview.title", "Developer preview")}</summary>
        <p className="mt-2 text-sm text-[var(--color-on-surface-variant)]">
          {pageT("payloadPreview.description", "This shows the full management payload sent to save and verify APIs.")}
        </p>
        <pre className="mt-4 overflow-x-auto rounded-xl bg-[var(--color-surface-container-low)] p-4 text-[13px] leading-6 text-[var(--color-on-surface)]">
          {JSON.stringify(payload, null, 2)}
        </pre>
      </details>
    </SurfaceCard>
  );
}

function RetrievalFlowCard({ form }: { form: RagRetrievalProfileFormState }) {
  const { pageT } = useProfileFormCopy();
  const enabledRoutes = form.routes.filter((route) => route.enabled && route.strategyId.trim());
  const contextStrategy = form.contextStrategies[0] || "RESULT_LIST";
  const steps = [
    {
      label: pageT("retrievalFlow.routeSearch", "Route search"),
      value: enabledRoutes.length > 0 ? pageT("retrievalFlow.enabledRoutes", "{{count}} enabled routes", { count: enabledRoutes.length }) : pageT("retrievalFlow.noRoutes", "No routes selected"),
    },
    { label: pageT("retrievalFlow.fusion", "Result fusion"), value: form.fusionPolicy || "SUM_SCORE" },
    { label: pageT("retrievalFlow.dedup", "Result deduplication"), value: form.dedupPolicy || "TEXT_UNIT" },
    { label: pageT("retrievalFlow.context", "Build context"), value: contextStrategy },
  ];

  return (
    <SurfaceCard>
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="text-base font-semibold">{pageT("retrievalFlow.title", "Retrieval flow")}</h2>
          <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">
            {pageT("retrievalFlow.description", "Saved profiles run route retrieval in parallel, then process results and build context.")}
          </p>
        </div>
        <span className="rounded-full border border-[var(--color-outline-variant)] px-3 py-1 text-sm">
          {pageT("retrievalFlow.routeCount", "{{count}} routes", { count: enabledRoutes.length })}
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
  const { pageT, retrievalHintT } = useProfileFormCopy();

  return (
    <div className="space-y-3 rounded-lg border border-[var(--color-outline-variant)] p-3">
      <div className={`grid gap-3 ${canRemove ? "lg:grid-cols-[minmax(0,1fr)_minmax(0,1.4fr)_auto] lg:items-end" : "lg:grid-cols-2"}`}>
        <Field label={pageT("fields.corpus", "Corpus")} error={error?.corpusId} hint={retrievalHintT("routeCorpus")}>
          <SelectInput
            aria-label={pageT("routes.corpusAria", "Route {{index}} corpus", { index: index + 1 })}
            className="w-full"
            value={row.corpusId}
            onValueChange={(event) => onUpdate({ corpusId: event!, strategyId: "" })}
          >
            <SelectItem value="">{pageT("fields.selectCorpus", "Select corpus")}</SelectItem>
            {corpusOptions.map((corpus) => (
              <SelectItem key={corpus.code} value={corpus.code}>{corpus.code}</SelectItem>
            ))}
          </SelectInput>
        </Field>
        <Field label={pageT("fields.retrievalStrategy", "Retrieval strategy")} error={error?.strategyId} hint={retrievalHintT("routeStrategy")}>
          <SelectInput
            aria-label={pageT("routes.strategyAria", "Route {{index}} retrieval strategy", { index: index + 1 })}
            className="w-full font-mono"
            value={row.strategyId}
            onValueChange={(event) => onUpdate({ strategyId: event! })}
          >
            <SelectItem value="">{strategyOptions.length > 0 ? pageT("routes.selectStrategy", "Select route strategy") : pageT("routes.noStrategies", "No supported strategies")}</SelectItem>
            {strategyOptions.map((strategyId) => <SelectItem key={strategyId} value={strategyId}>{strategyId}</SelectItem>)}
          </SelectInput>
        </Field>
        {canRemove ? (
          <div className="flex items-center justify-end lg:pb-1">
            <Button aria-label={pageT("routes.removeAria", "Remove route {{index}}", { index: index + 1 })} size="icon" type="button" variant="ghost" onClick={onRemove}>
              <Trash2 size={16} aria-hidden="true" />
            </Button>
          </div>
        ) : null}
      </div>
      {row.corpusId && strategyOptions.length === 0 ? (
        <p className="text-[12px] text-[var(--color-on-surface-variant)]">
          {pageT("routes.noBackendStrategies", "This corpus has no backend-supported retrieval strategies.")}
        </p>
      ) : null}
      <ToggleSwitch checked={row.enabled} label={pageT("routes.enableLabel", "Enable route {{index}}", { index: index + 1 })} onCheckedChange={(checked) => onUpdate({ enabled: checked })} />
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
  const { pageT, retrievalHintT } = useProfileFormCopy();

  return (
    <div className="space-y-3 rounded-lg border border-[var(--color-outline-variant)] p-3">
      <div className={`grid gap-3 ${canRemove ? "lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_minmax(0,1fr)_auto] lg:items-end" : "lg:grid-cols-3"}`}>
        <Field label={pageT("fields.field", "Field")} error={error?.field} hint={retrievalHintT("filterField")}>
          <SelectInput
            aria-label={pageT("filters.fieldAria", "Filter {{index}} field", { index: index + 1 })}
            className="w-full"
            value={row.field}
            onValueChange={(event) => onUpdate({ field: event! })}
          >
            <SelectItem value="">{pageT("fields.selectMappedField", "Select mapped field")}</SelectItem>
            {fieldOptions.map((field) => <SelectItem key={field} value={field}>{field}</SelectItem>)}
          </SelectInput>
        </Field>
        <Field label={pageT("fields.operator", "Operator")} error={error?.operator} hint={retrievalHintT("filterOperator")}>
          <SelectInput
            aria-label={pageT("filters.operatorAria", "Filter {{index}} operator", { index: index + 1 })}
            className="w-full"
            value={row.operator}
            onValueChange={(event) => onUpdate({ operator: event! })}
          >
            {operatorOptions.map((operator) => <SelectItem key={operator} value={operator}>{operator}</SelectItem>)}
          </SelectInput>
        </Field>
        <Field label={pageT("fields.value", "Value")} error={error?.value} hint={retrievalHintT("filterValue")}>
          <TextInput
            aria-label={pageT("filters.valueAria", "Filter {{index}} value", { index: index + 1 })}
            className="w-full font-mono"
            value={row.value}
            onChange={(event) => onUpdate({ value: event.target.value })}
          />
        </Field>
        {canRemove ? (
          <div className="flex items-center justify-end lg:pb-1">
            <Button aria-label={pageT("filters.removeAria", "Remove filter {{index}}", { index: index + 1 })} size="icon" type="button" variant="ghost" onClick={onRemove}>
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
  const { pageT, ingestHintT, t } = useProfileFormCopy();
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
  const title = mode === "create" ? pageT("ingest.title.create", "New ingest profile") : pageT("ingest.title.edit", "Edit ingest profile");
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
        toast.error(error instanceof Error ? error.message : pageT("messages.catalogLoadFailed", "Profile catalog could not be loaded."));
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
      toast.success(mode === "create" ? pageT("messages.ingestCreated", "Ingest profile created.") : pageT("messages.profileUpdated", "Profile updated."));
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
        toast.error(validationIssueMessage(issues[0] ?? { message: pageT("messages.fixProfileErrors", "Resolve profile form errors before saving.") }));
        return;
      }

      const saved = mode === "create"
        ? await createRagProfile(payload)
        : await updateRagProfile(profileId ?? form.id.trim(), payload);
      const savedId = typeof saved.profileId === "string" && saved.profileId.trim()
        ? saved.profileId.trim()
        : form.id.trim();
      toast.success(pageT(mode === "create" ? "messages.profileCreatedWithId" : "messages.profileSavedWithId", mode === "create" ? "{{id}} created" : "{{id}} saved", { id: savedId }));
      navigate(returnPath);
    } catch (error) {
      const errorData = typeof error === "object" && error !== null && "data" in error
        ? (error as { data?: unknown }).data
        : undefined;
      const issues = isRecord(errorData) ? normalizeRagValidationIssues(errorData.errors) : [];
      if (issues.length > 0) {
        setErrors(mapIngestApiErrors(issues, form));
      }
      toast.error(error instanceof Error ? error.message : pageT("messages.ingestSaveFailed", "Ingest profile could not be saved."));
    } finally {
      setSubmitting(false);
    }
  }

  if (mode === "edit" && loading) {
    return (
      <>
        <PageHeader
          title={pageT("loading.ingestTitle", "Loading ingest profile")}
          actions={(
            <Button variant="secondary" onClick={() => navigate(returnPath)}>
              <X size={16} aria-hidden="true" />
              {pageT("actions.cancel", "Cancel")}
            </Button>
          )}
        />
        <SurfaceCard>
          <h2 className="text-base font-semibold">{pageT("loading.title", "Loading profile")}</h2>
          <p className="mt-3 text-sm text-[var(--color-on-surface-variant)]">{pageT("loading.ingestDescription", "Loading ingest profile management data...")}</p>
        </SurfaceCard>
      </>
    );
  }

  if (mode === "edit" && !existingProfile) {
    return (
      <>
        <PageHeader
          title={pageT("ingest.title.edit", "Edit ingest profile")}
          actions={(
            <Button variant="secondary" onClick={() => navigate(returnPath)}>
              <X size={16} aria-hidden="true" />
              {pageT("actions.cancel", "Cancel")}
            </Button>
          )}
        />
        <SurfaceCard>
          <h2 className="text-base font-semibold">{pageT("notFound.title", "Profile not found")}</h2>
          <p className="mt-3 text-sm text-[var(--color-on-surface-variant)]">
            {isApiMode
              ? pageT("notFound.ingestApi", "The requested ingest profile could not be loaded from the RAG management API.")
              : pageT("notFound.ingestMock", "The requested ingest profile does not exist in the mock registry.")}
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
              {pageT("actions.cancel", "Cancel")}
            </Button>
            <Button aria-busy={submitting} disabled={loading || submitting} type="submit" form="rag-ingest-profile-form">
              <Save size={16} aria-hidden="true" />
              {submitting ? pageT("actions.saving", "Saving...") : pageT("actions.saveProfile", "Save profile")}
            </Button>
          </>
        )}
      />

      <form className="grid min-w-0 gap-5" id="rag-ingest-profile-form" onSubmit={handleSubmit}>
        <SurfaceCard>
          <h2 className="text-base font-semibold">{pageT("sections.basicInfo", "Basic information")}</h2>
          <div className="mt-4 grid min-w-0 gap-4 lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
            <Field label={pageT("fields.profileId", "Profile ID")} error={errors.id} hint={ingestHintT("profileId")}>
              <TextInput
                aria-label={pageT("fields.profileId", "Profile ID")}
                className="font-mono"
                disabled={mode === "edit"}
                value={form.id}
                onChange={(event) => updateForm("id", event.target.value)}
              />
            </Field>
            <Field label={pageT("fields.status", "Status")} error={errors.status} hint={ingestHintT("status")}>
              <SelectInput aria-label={pageT("fields.status", "Status")} value={form.status} onValueChange={(event) => updateForm("status", event!)}>
                {listProfileStatuses().map((status) => <SelectItem key={status} value={status}>{formatRagStatusLabel(status, t)}</SelectItem>)}
              </SelectInput>
            </Field>
            <Field label={pageT("fields.primaryCorpus", "Primary corpus")} error={errors.primaryCorpusId} hint={ingestHintT("primaryCorpus")}>
              <SelectInput aria-label={pageT("fields.primaryCorpus", "Primary corpus")} value={form.primaryCorpusId} onValueChange={(event) => updateForm("primaryCorpusId", event!)}>
                <SelectItem value="">{pageT("fields.selectCorpus", "Select corpus")}</SelectItem>
                {corpusOptions.map((corpus) => <SelectItem key={corpus.code} value={corpus.code}>{corpus.code}</SelectItem>)}
              </SelectInput>
            </Field>
            <Field label={pageT("fields.description", "Description")} hint={ingestHintT("description")}>
              <TextArea aria-label={pageT("fields.description", "Description")} className="min-h-28" value={form.description} onChange={(event) => updateForm("description", event.target.value)} />
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
            <summary className="cursor-pointer text-base font-semibold">{pageT("ingest.advancedNodes.title", "Advanced node controls")}</summary>
            <p className="mt-2 text-sm text-[var(--color-on-surface-variant)]">
              {pageT("ingest.advancedNodes.description", "Use these controls when the selected template needs a custom stage combination.")}
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
                          <span className="rounded-full bg-[var(--color-primary-container)] px-2 py-0.5 text-[11px] text-[var(--color-on-primary-container)]">{pageT("ingest.advancedNodes.required", "Required")}</span>
                        ) : null}
                        {stageDefinition?.orchestration?.actsAsBarrier ? (
                          <span className="rounded-full bg-[var(--color-surface)] px-2 py-0.5 text-[11px] text-[var(--color-on-surface-variant)]">{pageT("ingest.advancedNodes.barrier", "Control barrier")}</span>
                        ) : null}
                      </div>
                      <p className="mt-1 font-mono text-[12px] text-[var(--color-on-surface-variant)]">{ingestStageNodeId(stage.id)} · {stageType}</p>
                      {(stageDefinition?.orchestration?.mustRunBeforeStageTypes.length ?? 0) > 0 ? (
                        <p className="mt-1 text-[12px] text-[var(--color-on-surface-variant)]">
                          {pageT("ingest.advancedNodes.mustRunBefore", "Must run before {{stages}}", { stages: stageDefinition?.orchestration?.mustRunBeforeStageTypes.join(", ") })}
                        </p>
                      ) : null}
                    </div>
                    <ToggleSwitch checked={stage.enabled} disabled={required} label={pageT("ingest.advancedNodes.enableNode", "Enable {{stage}} node", { stage: stageDisplayName(pipelineCatalog, stageType) })} onCheckedChange={(checked) => updateStage(stage.id, checked)} />
                  </div>
                );
              })}
              {errors.pipeline ? <p className="text-[12px] text-[var(--color-error)]">{errors.pipeline}</p> : null}
            </div>
          </details>
        </SurfaceCard>

        <SurfaceCard>
          <h2 className="text-base font-semibold">{pageT("ingest.inputContract.title", "Input contract")}</h2>
          <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">
            {pageT("ingest.inputContract.description", "These fields define the inputs accepted by the workbench and ingest API for this profile.")}
          </p>
          <div className="mt-4 grid gap-4 lg:grid-cols-2">
            <Field label={pageT("fields.supportedSourceTypes", "Supported source types")} error={errors.supportedSourceTypesText} hint={ingestHintT("supportedSourceTypes")}>
              <TextInput aria-label={pageT("fields.supportedSourceTypes", "Supported source types")} className="font-mono" value={form.supportedSourceTypesText} onChange={(event) => updateForm("supportedSourceTypesText", event.target.value)} />
            </Field>
            <Field label={pageT("fields.requiredSourceVersion", "Required source version")} hint={ingestHintT("requiredSourceVersion")}>
              <TextInput aria-label={pageT("fields.requiredSourceVersion", "Required source version")} className="font-mono" value={form.requiredSourceVersion} onChange={(event) => updateForm("requiredSourceVersion", event.target.value)} />
            </Field>
          </div>
        </SurfaceCard>

        <SurfaceCard>
          <h2 className="text-base font-semibold">{pageT("ingest.stageStrategies.title", "Stage strategies")}</h2>
          <div className="mt-4 grid gap-4">
            {enabledStages.length === 0 ? (
              <p className="text-sm text-[var(--color-on-surface-variant)]">{pageT("ingest.stageStrategies.empty", "Enable one or more stages above before configuring stage strategies.")}</p>
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
                    <Field label={pageT("fields.strategy", "Strategy")} error={errors.stageRows?.[stage.id]?.strategyId} hint={ingestHintT("strategy")}>
                      <SelectInput
                        aria-label={pageT("ingest.stageStrategies.strategyAria", "{{stage}} strategy ID", { stage: stageType })}
                        value={stage.strategyId}
                        onValueChange={(event) => updateStageStrategy(stage.id, event!)}
                      >
                        <SelectItem value="">{(allowedStageStrategies[stage.id]?.length ?? 0) > 0 ? pageT("ingest.stageStrategies.selectStrategy", "Select stage strategy") : pageT("ingest.stageStrategies.noStrategy", "No supported strategy")}</SelectItem>
                        {(allowedStageStrategies[stage.id] ?? []).map((strategyId) => (
                          <SelectItem key={strategyId} value={strategyId}>{strategyOptionLabel(stageDefinition, strategyId)}</SelectItem>
                        ))}
                      </SelectInput>
                    </Field>

                    {stage.id === "resolveSource" ? (
                      <>
                        <Field label={pageT("fields.allowedUriSchemes", "Allowed URI schemes")} hint={ingestHintT("resolveSourceAllowedUriSchemes")}>
                          <TextInput
                            aria-label={pageT("fields.allowedUriSchemes", "Allowed URI schemes")}
                            className="font-mono"
                            value={form.resolveSourceAllowedUriSchemesText}
                            onChange={(event) => updateForm("resolveSourceAllowedUriSchemesText", event.target.value)}
                          />
                        </Field>
                        <Field label={pageT("fields.maxBytes", "Max bytes")} error={errors.resolveSourceMaxBytes} hint={ingestHintT("resolveSourceMaxBytes")}>
                          <TextInput
                            aria-label={pageT("fields.maxBytes", "Max bytes")}
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
                          label={stage.strategyId.trim() === mineruParseStrategyId ? pageT("fields.supportedFormats", "Supported formats") : pageT("fields.documentFormat", "Document format")}
                          error={errors.parseDocumentFormat}
                          hint={parseDocumentFormatHint(stage.strategyId)}
                        >
                          <TextInput
                            aria-label={stage.strategyId.trim() === mineruParseStrategyId ? pageT("fields.supportedFormats", "Supported formats") : pageT("fields.documentFormat", "Document format")}
                            className="font-mono"
                            value={form.parseDocumentFormat}
                            onChange={(event) => updateForm("parseDocumentFormat", event.target.value)}
                          />
                        </Field>
                        <Field label={pageT("fields.unsupportedFormatPolicy", "Unsupported format policy")} hint={ingestHintT("parseDocumentUnsupportedFormatPolicy")}>
                          <TextInput
                            aria-label={pageT("fields.unsupportedFormatPolicy", "Unsupported format policy")}
                            className="font-mono"
                            value={form.parseDocumentUnsupportedFormatPolicy}
                            onChange={(event) => updateForm("parseDocumentUnsupportedFormatPolicy", event.target.value)}
                          />
                        </Field>
                      </>
                    ) : null}

                    {stage.id === "splitText" ? (
                      <>
                        <Field label={pageT("fields.splitterType", "Splitter type")} error={errors.splitTextType} hint={ingestHintT("splitTextType")}>
                          <TextInput
                            aria-label={pageT("fields.splitterType", "Splitter type")}
                            className="font-mono"
                            value={form.splitTextType}
                            onChange={(event) => updateForm("splitTextType", event.target.value)}
                          />
                        </Field>
                        <Field label={pageT("fields.maxTokens", "Max tokens")} error={errors.splitTextMaxTokens} hint={ingestHintT("splitTextMaxTokens")}>
                          <TextInput
                            aria-label={pageT("fields.maxTokens", "Max tokens")}
                            className="font-mono"
                            value={form.splitTextMaxTokens}
                            onChange={(event) => updateForm("splitTextMaxTokens", event.target.value)}
                          />
                        </Field>
                        <Field label={pageT("fields.overlapTokens", "Overlap tokens")} error={errors.splitTextOverlapTokens} hint={ingestHintT("splitTextOverlapTokens")}>
                          <TextInput
                            aria-label={pageT("fields.overlapTokens", "Overlap tokens")}
                            className="font-mono"
                            value={form.splitTextOverlapTokens}
                            onChange={(event) => updateForm("splitTextOverlapTokens", event.target.value)}
                          />
                        </Field>
                        <Field label={pageT("fields.headingDepth", "Heading depth")} error={errors.splitTextHeadingDepth} hint={ingestHintT("splitTextHeadingDepth")}>
                          <TextInput
                            aria-label={pageT("fields.headingDepth", "Heading depth")}
                            className="font-mono"
                            value={form.splitTextHeadingDepth}
                            onChange={(event) => updateForm("splitTextHeadingDepth", event.target.value)}
                          />
                        </Field>
                      </>
                    ) : null}

                    {stage.id === "summarizeTree" && stage.strategyId.trim() === treeLlmSummaryStrategyId ? (
                      <Field label={pageT("fields.treeSummaryModelCode", "Tree summary model code")} error={errors.treeSummaryModelCode} hint={ingestHintT("treeSummaryModelCode")}>
                        <TextInput
                          aria-label={pageT("fields.treeSummaryModelCode", "Tree summary model code")}
                          className="font-mono"
                          value={form.treeSummaryModelCode}
                          onChange={(event) => updateForm("treeSummaryModelCode", event.target.value)}
                        />
                      </Field>
                    ) : null}

                    {stage.id === "vectorIndex" ? (
                      <>
                        <Field label={pageT("fields.embeddingModelCode", "Embedding model code")} hint={ingestHintT("embeddingModelCode")}>
                          <TextInput
                            aria-label={pageT("fields.embeddingModelCode", "Embedding model code")}
                            className="font-mono"
                            value={form.embeddingModelCode}
                            onChange={(event) => updateForm("embeddingModelCode", event.target.value)}
                          />
                        </Field>
                        <Field label={pageT("fields.embeddingVersion", "Embedding version")} hint={ingestHintT("embeddingVersion")}>
                          <TextInput
                            aria-label={pageT("fields.embeddingVersion", "Embedding version")}
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
                      {pageT("ingest.stageStrategies.noAvailableStrategy", "No available strategy for this stage and corpus.")}
                    </p>
                  ) : null}
                  <details className="mt-4 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface)] p-3">
                    <summary className="cursor-pointer text-sm font-medium">{pageT("ingest.developerContract.title", "Developer contract")}</summary>
                    <div className="mt-3 grid gap-3">
                      <div className="grid gap-2 text-[12px] text-[var(--color-on-surface-variant)] lg:grid-cols-2">
                        <p><span className="font-semibold">{pageT("ingest.developerContract.input", "Input:")}</span>{stageDefinition?.inputArtifactKeys.join(", ") || pageT("values.none", "None")}</p>
                        <p><span className="font-semibold">{pageT("ingest.developerContract.output", "Output:")}</span>{stageDefinition?.outputArtifactKeys.join(", ") || pageT("values.none", "None")}</p>
                      </div>
                      <JsonPreview value={configSchema} />
                    </div>
                  </details>
                </div>
              );
            })}
            {graphBackedCorpus ? (
              <Field label={pageT("fields.graphWritePolicy", "Graph write policy")} hint={ingestHintT("graphWritePolicy")}>
                <TextInput aria-label={pageT("fields.graphWritePolicy", "Graph write policy")} className="font-mono" value={form.graphWritePolicy} onChange={(event) => updateForm("graphWritePolicy", event.target.value)} />
              </Field>
            ) : null}
          </div>
        </SurfaceCard>

        <SurfaceCard>
          <h2 className="text-base font-semibold">{pageT("ingest.summary.title", "Pipeline summary")}</h2>
          <div className="mt-4">
            <SummaryList rows={[
              { label: pageT("fields.primaryCorpus", "Primary corpus"), value: form.primaryCorpusId || pageT("values.notSet", "Not set") },
              { label: pageT("ingest.summary.enabledStages", "Enabled stages"), value: String(form.pipelineStages.filter((stage) => stage.enabled).length) },
              {
                label: pageT("ingest.summary.payloadStrategy", "Payload strategy"),
                value: form.pipelineStages.find((stage) => stage.id === "preparePayload")?.strategyId || pageT("values.notSet", "Not set"),
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
  const { pageT, retrievalHintT, t } = useProfileFormCopy();
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
  const title = mode === "create" ? pageT("retrieval.title.create", "New retrieval profile") : pageT("retrieval.title.edit", "Edit retrieval profile");
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
        const catalogStrategiesByCorpus = catalog?.routeStrategiesByCorpus ?? {};
        const hasCatalogStrategiesForCorpus = isApiMode
          && Boolean(route.corpusId)
          && Object.prototype.hasOwnProperty.call(catalogStrategiesByCorpus, route.corpusId);
        const catalogStrategiesByType = catalogStrategiesByCorpus[route.corpusId] ?? {};
        const pipelineStrategies = strategyOptionsForStage(findPipelineStageDefinition(pipelineCatalog, "ROUTE_SEARCH"));
        const catalogStrategies = Object.values(catalogStrategiesByType).flat();
        const corpusStrategies = corpusOptions.find((corpus) => corpus.code === route.corpusId)?.strategies ?? [];

        if (hasCatalogStrategiesForCorpus) {
          return [route.formId, listCatalogRouteStrategyOptions(route.corpusId, catalogStrategies)];
        }

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
        toast.error(error instanceof Error ? error.message : pageT("messages.catalogLoadFailed", "Profile catalog could not be loaded."));
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
      toast.success(mode === "create" ? pageT("messages.retrievalCreated", "Retrieval profile created.") : pageT("messages.profileUpdated", "Profile updated."));
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
        toast.error(validationIssueMessage(issues[0] ?? { message: pageT("messages.fixProfileErrors", "Resolve profile form errors before saving.") }));
        return;
      }

      const saved = mode === "create"
        ? await createRagProfile(payload)
        : await updateRagProfile(profileId ?? form.id.trim(), payload);
      const savedId = typeof saved.profileId === "string" && saved.profileId.trim()
        ? saved.profileId.trim()
        : form.id.trim();
      toast.success(pageT(mode === "create" ? "messages.profileCreatedWithId" : "messages.profileSavedWithId", mode === "create" ? "{{id}} created" : "{{id}} saved", { id: savedId }));
      navigate(returnPath);
    } catch (error) {
      const errorData = typeof error === "object" && error !== null && "data" in error
        ? (error as { data?: unknown }).data
        : undefined;
      const issues = isRecord(errorData) ? normalizeRagValidationIssues(errorData.errors) : [];
      if (issues.length > 0) {
        setErrors(mapRetrievalApiErrors(issues, form));
      }
      toast.error(error instanceof Error ? error.message : pageT("messages.retrievalSaveFailed", "Retrieval profile could not be saved."));
    } finally {
      setSubmitting(false);
    }
  }

  if (mode === "edit" && loading) {
    return (
      <>
        <PageHeader
          title={pageT("loading.retrievalTitle", "Loading retrieval profile")}
          actions={(
            <Button variant="secondary" onClick={() => navigate(returnPath)}>
              <X size={16} aria-hidden="true" />
              {pageT("actions.cancel", "Cancel")}
            </Button>
          )}
        />
        <SurfaceCard>
          <h2 className="text-base font-semibold">{pageT("loading.title", "Loading profile")}</h2>
          <p className="mt-3 text-sm text-[var(--color-on-surface-variant)]">{pageT("loading.retrievalDescription", "Loading retrieval profile management data...")}</p>
        </SurfaceCard>
      </>
    );
  }

  if (mode === "edit" && !existingProfile) {
    return (
      <>
        <PageHeader
          title={pageT("retrieval.title.edit", "Edit retrieval profile")}
          actions={(
            <Button variant="secondary" onClick={() => navigate(returnPath)}>
              <X size={16} aria-hidden="true" />
              {pageT("actions.cancel", "Cancel")}
            </Button>
          )}
        />
        <SurfaceCard>
          <h2 className="text-base font-semibold">{pageT("notFound.title", "Profile not found")}</h2>
          <p className="mt-3 text-sm text-[var(--color-on-surface-variant)]">
            {isApiMode
              ? pageT("notFound.retrievalApi", "The requested retrieval profile could not be loaded from the RAG management API.")
              : pageT("notFound.retrievalMock", "The requested retrieval profile does not exist in the mock registry.")}
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
              {pageT("actions.cancel", "Cancel")}
            </Button>
            <Button aria-busy={submitting} disabled={loading || submitting} type="submit" form="rag-retrieval-profile-form">
              <Save size={16} aria-hidden="true" />
              {submitting ? pageT("actions.saving", "Saving...") : pageT("actions.saveProfile", "Save profile")}
            </Button>
          </>
        )}
      />

      <form className="grid min-w-0 gap-5" id="rag-retrieval-profile-form" onSubmit={handleSubmit}>
        <SurfaceCard>
          <h2 className="text-base font-semibold">{pageT("sections.basicInfo", "Basic information")}</h2>
          <div className="mt-4 grid min-w-0 gap-4 lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
            <Field label={pageT("fields.profileId", "Profile ID")} error={errors.id} hint={retrievalHintT("profileId")}>
              <TextInput
                aria-label={pageT("fields.profileId", "Profile ID")}
                className="font-mono"
                disabled={mode === "edit"}
                value={form.id}
                onChange={(event) => updateForm("id", event.target.value)}
              />
            </Field>
            <Field label={pageT("fields.status", "Status")} error={errors.status} hint={retrievalHintT("status")}>
              <SelectInput aria-label={pageT("fields.status", "Status")} value={form.status} onValueChange={(event) => updateForm("status", event as RagProfileStatus)}>
                {listProfileStatuses().map((status) => <SelectItem key={status} value={status}>{formatRagStatusLabel(status, t)}</SelectItem>)}
              </SelectInput>
            </Field>
            <Field label={pageT("fields.primaryCorpus", "Primary corpus")} error={errors.primaryCorpusId} hint={retrievalHintT("primaryCorpus")}>
              <SelectInput aria-label={pageT("fields.primaryCorpus", "Primary corpus")} value={form.primaryCorpusId} onValueChange={(event) => updateForm("primaryCorpusId", event!)}>
                <SelectItem value="">{pageT("fields.selectCorpus", "Select corpus")}</SelectItem>
                {corpusOptions.map((corpus) => <SelectItem key={corpus.code} value={corpus.code}>{corpus.code}</SelectItem>)}
              </SelectInput>
            </Field>
            <Field label={pageT("fields.description", "Description")} hint={retrievalHintT("description")}>
              <TextArea aria-label={pageT("fields.description", "Description")} className="min-h-28" value={form.description} onChange={(event) => updateForm("description", event.target.value)} />
            </Field>
          </div>
        </SurfaceCard>

        <SurfaceCard>
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div>
              <h2 className="text-base font-semibold">{pageT("routes.title", "Retrieval routes")}</h2>
              <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">
                {pageT("routes.description", "Each route chooses a corpus and retrieval strategy; the strategy decides fulltext, vector, graph, or structured recall.")}
              </p>
            </div>
            <Button type="button" variant="secondary" onClick={addRoute}>
              <Plus size={16} aria-hidden="true" />
              {pageT("routes.add", "Add route")}
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
          <h2 className="text-base font-semibold">{pageT("retrieval.resultPolicy.title", "Result policy")}</h2>
          <div className="mt-4 grid gap-4 lg:grid-cols-2">
            <Field label={pageT("fields.fusionPolicy", "Fusion policy")} hint={retrievalHintT("fusionPolicy")}>
              <SelectInput aria-label={pageT("fields.fusionPolicy", "Fusion policy")} className="font-mono" value={form.fusionPolicy} onValueChange={(event) => updateForm("fusionPolicy", event!)}>
                {fusionPolicyOptions.map((policy) => <SelectItem key={policy} value={policy}>{policy}</SelectItem>)}
              </SelectInput>
            </Field>
            <Field label={pageT("fields.dedupPolicy", "Dedup policy")} hint={retrievalHintT("dedupPolicy")}>
              <SelectInput aria-label={pageT("fields.dedupPolicy", "Dedup policy")} className="font-mono" value={form.dedupPolicy} onValueChange={(event) => updateForm("dedupPolicy", event!)}>
                {dedupPolicyOptions.map((policy) => <SelectItem key={policy} value={policy}>{policy}</SelectItem>)}
              </SelectInput>
            </Field>
            <Field label={pageT("fields.topicPolicy", "Topic policy")} hint={retrievalHintT("topicPolicy")}>
              <SelectInput aria-label={pageT("fields.topicPolicy", "Topic policy")} className="font-mono" value={form.topicPolicy} onValueChange={(event) => updateForm("topicPolicy", event!)}>
                {topicPolicyOptions.map((policy) => <SelectItem key={policy} value={policy}>{policy}</SelectItem>)}
              </SelectInput>
            </Field>
            <Field label={pageT("fields.rerankModelCode", "Rerank model code")} hint={retrievalHintT("rerankModelCode")}>
              <TextInput aria-label={pageT("fields.rerankModelCode", "Rerank model code")} className="font-mono" value={form.rerankModelCode} onChange={(event) => updateForm("rerankModelCode", event.target.value)} />
            </Field>
          </div>
          <div className="mt-4">
            <ToggleSwitch checked={form.rerankEnabled} label={pageT("fields.enableRerank", "Enable rerank")} onCheckedChange={(checked) => updateForm("rerankEnabled", checked)} />
          </div>
        </SurfaceCard>

        <SurfaceCard>
          <h2 className="text-base font-semibold">{pageT("retrieval.contextStrategies.title", "Context strategies")}</h2>
          <div className="mt-4 grid gap-3">
            {contextStrategyOptions.map((strategyId) => (
              <div className="flex items-center justify-between gap-4 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] px-4 py-3" key={strategyId}>
                <span className="font-mono text-sm">{strategyId}</span>
                <ToggleSwitch
                  checked={form.contextStrategies.includes(strategyId)}
                  label={pageT("retrieval.contextStrategies.enableLabel", "Enable context strategy {{strategy}}", { strategy: strategyId })}
                  onCheckedChange={(checked) => toggleContextStrategy(strategyId, checked)}
                />
              </div>
            ))}
            {errors.contextStrategies ? <p className="text-[12px] text-[var(--color-error)]">{errors.contextStrategies}</p> : null}
          </div>
        </SurfaceCard>

        <SurfaceCard>
          <h2 className="text-base font-semibold">{pageT("retrieval.graphExpansion.title", "Graph expansion")}</h2>
          <div className="mt-4">
            <Field label={pageT("fields.graphExpansionPolicy", "Graph expansion policy")} error={errors.graphExpansionPolicy} hint={retrievalHintT("graphExpansionPolicy")}>
              <TextInput
                aria-label={pageT("fields.graphExpansionPolicy", "Graph expansion policy")}
                className="font-mono"
                placeholder={enabledGraphRouteExists ? "" : pageT("retrieval.graphExpansion.disabledPlaceholder", "Select an enabled graph strategy before using this policy")}
                value={form.graphExpansionPolicy}
                onChange={(event) => updateForm("graphExpansionPolicy", event.target.value)}
              />
            </Field>
          </div>
        </SurfaceCard>

        <SurfaceCard>
          <div className="flex flex-wrap items-start justify-between gap-3">
            <h2 className="text-base font-semibold">{pageT("filters.title", "Profile filters")}</h2>
            <Button type="button" variant="secondary" onClick={addFilter}>
              <Plus size={16} aria-hidden="true" />
              {pageT("filters.add", "Add filter")}
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
