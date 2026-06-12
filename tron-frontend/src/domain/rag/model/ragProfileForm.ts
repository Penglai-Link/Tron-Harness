import type {
  CorpusRecord,
  IngestProfileRecord,
  RagIngestStageId,
  RagPipelineCatalog,
  RagPipelineEdge,
  RagPipelineNode,
  RagPipelineStageDefinition,
  RagProfilePipeline,
  RagProfileStatus,
  RagRetrievalRouteType,
  RetrievalProfileRecord,
} from "@/domain/rag/model/RagModel";

const defaultIngestStageOrder: RagIngestStageId[] = [
  "validateSource",
  "preparePayload",
  "resolveSource",
  "parseDocument",
  "splitText",
  "fulltextIndex",
  "vectorIndex",
  "extractTree",
  "persistTree",
  "persistSource",
];

const defaultRouteTypes: Array<RagRetrievalRouteType> = ["FULLTEXT", "VECTOR", "GRAPH", "STRUCTURED_FILTER"];
const defaultProfileStatuses: RagProfileStatus[] = ["Draft", "Ready", "Needs Review"];
const defaultFilterOperators = ["eq", "in", "gt", "gte", "lt", "lte"];
const initialIngestArtifactKeys = new Set(["request.payload", "request.accessContext"]);
const ingestStageTypesById: Record<RagIngestStageId, string> = {
  fulltextIndex: "FULLTEXT_INDEX",
  extractTree: "EXTRACT_TREE",
  parseDocument: "PARSE_DOCUMENT",
  persistTree: "PERSIST_TREE",
  persistSource: "PERSIST_SOURCE",
  preparePayload: "PREPARE_PAYLOAD",
  resolveSource: "RESOLVE_SOURCE",
  splitText: "SPLIT_TEXT",
  validateSource: "VALIDATE_SOURCE",
  vectorIndex: "VECTOR_INDEX",
};
const ingestStageIdsByType = Object.fromEntries(
  Object.entries(ingestStageTypesById).map(([stageId, stageType]) => [stageType, stageId]),
) as Record<string, RagIngestStageId>;

export interface RagIngestProfileFormState {
  description: string;
  embeddingModelCode: string;
  embeddingVersion: string;
  graphWritePolicy: string;
  id: string;
  parseDocumentFormat: string;
  parseDocumentUnsupportedFormatPolicy: string;
  pipelineStages: Array<{ enabled: boolean; id: RagIngestStageId; strategyId: string }>;
  primaryCorpusId: string;
  requiredSourceVersion: string;
  resolveSourceAllowedUriSchemesText: string;
  resolveSourceMaxBytes: string;
  splitTextHeadingDepth: string;
  splitTextMaxTokens: string;
  splitTextOverlapTokens: string;
  splitTextType: string;
  status: RagProfileStatus;
  supportedSourceTypesText: string;
}

export interface RagRetrievalRouteFormRow {
  corpusId: string;
  enabled: boolean;
  formId: string;
  routeId?: string;
  routeType: RagRetrievalRouteType | "";
  strategyId: string;
}

export interface RagRetrievalFilterFormRow {
  field: string;
  formId: string;
  operator: string;
  value: string;
}

export interface RagRetrievalProfileFormState {
  contextStrategies: string[];
  dedupPolicy: string;
  description: string;
  filters: RagRetrievalFilterFormRow[];
  fusionPolicy: string;
  graphExpansionPolicy: string;
  id: string;
  primaryCorpusId: string;
  rerankEnabled: boolean;
  rerankModelCode: string;
  routes: RagRetrievalRouteFormRow[];
  status: RagProfileStatus;
  topicPolicy: string;
}

export interface RagIngestProfileFormErrors {
  id?: string;
  parseDocumentFormat?: string;
  pipeline?: string;
  primaryCorpusId?: string;
  resolveSourceMaxBytes?: string;
  status?: string;
  splitTextHeadingDepth?: string;
  splitTextMaxTokens?: string;
  splitTextOverlapTokens?: string;
  splitTextType?: string;
  stageRows?: Partial<Record<RagIngestStageId, { strategyId?: string }>>;
  supportedSourceTypesText?: string;
}

export interface RagRetrievalProfileFormErrors {
  contextStrategies?: string;
  filters?: string;
  filterRows?: Record<string, { field?: string; operator?: string; value?: string }>;
  graphExpansionPolicy?: string;
  id?: string;
  primaryCorpusId?: string;
  routes?: string;
  routeRows?: Record<string, { corpusId?: string; routeType?: string; strategyId?: string }>;
  status?: string;
}

export function listProfileStatuses() {
  return defaultProfileStatuses;
}

export function listRetrievalRouteTypes() {
  return defaultRouteTypes;
}

export function listFilterOperators() {
  return defaultFilterOperators;
}

function nextFormId(prefix: string, index: number) {
  return `${prefix}-${index + 1}`;
}

function splitCommaSeparatedTokens(value: string) {
  return value
    .split(",")
    .map((token) => token.trim())
    .filter(Boolean);
}

function trimOrUndefined(value: string) {
  const trimmed = value.trim();
  return trimmed ? trimmed : undefined;
}

function parseOptionalInteger(value: string) {
  const trimmed = trimOrUndefined(value);
  if (!trimmed) return undefined;
  const parsed = Number.parseInt(trimmed, 10);
  return Number.isFinite(parsed) ? parsed : undefined;
}

function buildFallbackIngestStageEdges(nodes: RagIngestStageId[]) {
  return nodes.slice(0, -1).map((node, index) => ({ from: node, to: nodes[index + 1] }));
}

function buildIngestStageEdges(nodes: RagIngestStageId[], pipelineCatalog?: RagPipelineCatalog | null) {
  if (!pipelineCatalog?.stages.length) {
    return buildFallbackIngestStageEdges(nodes);
  }

  const enabled = new Set(nodes);
  const stageDefinitionById = new Map(
    nodes
      .map((stageId) => {
        const stageType = ingestStageIdToStageType(stageId);
        const definition = pipelineCatalog.stages.find((stage) => stage.stageType === stageType);
        return definition ? [stageId, definition] as const : undefined;
      })
      .filter((entry): entry is readonly [RagIngestStageId, RagPipelineStageDefinition] => Boolean(entry)),
  );
  const providersByArtifact = new Map<string, RagIngestStageId[]>();
  stageDefinitionById.forEach((definition, stageId) => {
    definition.outputArtifactKeys.forEach((artifactKey) => {
      const providers = providersByArtifact.get(artifactKey) ?? [];
      providers.push(stageId);
      providersByArtifact.set(artifactKey, providers);
    });
  });

  const edges = new Map<string, { from: RagIngestStageId; to: RagIngestStageId }>();
  const addEdge = (from: RagIngestStageId | undefined, to: RagIngestStageId | undefined) => {
    if (!from || !to || from === to || !enabled.has(from) || !enabled.has(to)) return;
    edges.set(`${from}->${to}`, { from, to });
  };

  stageDefinitionById.forEach((definition, stageId) => {
    definition.inputArtifactKeys
      .filter((artifactKey) => !initialIngestArtifactKeys.has(artifactKey))
      .forEach((artifactKey) => {
        providersByArtifact.get(artifactKey)?.forEach((providerStageId) => addEdge(providerStageId, stageId));
      });

    definition.orchestration?.mustRunBeforeStageTypes
      .map(ingestStageTypeToStageId)
      .forEach((targetStageId) => addEdge(stageId, targetStageId));

    definition.orchestration?.mustRunAfterStageTypes
      .map(ingestStageTypeToStageId)
      .forEach((sourceStageId) => addEdge(sourceStageId, stageId));
  });

  return Array.from(edges.values()).sort((left, right) => {
    const leftFrom = defaultIngestStageOrder.indexOf(left.from);
    const rightFrom = defaultIngestStageOrder.indexOf(right.from);
    if (leftFrom !== rightFrom) return leftFrom - rightFrom;
    return defaultIngestStageOrder.indexOf(left.to) - defaultIngestStageOrder.indexOf(right.to);
  });
}

function requiredIngestStageIds(pipelineCatalog?: RagPipelineCatalog | null) {
  if (!pipelineCatalog?.stages.length) return [] as RagIngestStageId[];
  return pipelineCatalog.stages
    .filter((stage) => stage.orchestration?.required)
    .map((stage) => ingestStageTypeToStageId(stage.stageType))
    .filter((stageId): stageId is RagIngestStageId => Boolean(stageId));
}

function sanitizeNodeIdPart(value: string) {
  return value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9_-]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

function buildPipelineEdges(nodes: RagPipelineNode[]): RagPipelineEdge[] {
  const enabledNodeIds = nodes.filter((node) => node.enabled).map((node) => node.nodeId);
  return enabledNodeIds.slice(0, -1).map((nodeId, index) => ({ from: nodeId, to: enabledNodeIds[index + 1] }));
}

function buildRetrievalPipelineEdges(
  routeNodes: RagPipelineNode[],
  fuseNodeId: string,
  dedupNodeId: string,
  contextNodeId: string,
): RagPipelineEdge[] {
  return [
    ...routeNodes.map((route) => ({ from: route.nodeId, to: fuseNodeId })),
    { from: fuseNodeId, to: dedupNodeId },
    { from: dedupNodeId, to: contextNodeId },
  ];
}

export function ingestStageIdToStageType(stageId: RagIngestStageId) {
  return ingestStageTypesById[stageId];
}

export function ingestStageTypeToStageId(stageType: string) {
  return ingestStageIdsByType[stageType.trim().toUpperCase()];
}

export function ingestStageNodeId(stageId: RagIngestStageId) {
  return sanitizeNodeIdPart(ingestStageIdToStageType(stageId).replaceAll("_", "-"));
}

export function retrievalNodeId(route: { corpusId?: string; routeId?: string; routeType?: string }, index: number) {
  const explicit = route.routeId?.trim();
  if (explicit) return explicit;
  const corpusPart = sanitizeNodeIdPart(route.corpusId ?? "") || "route";
  const typePart = sanitizeNodeIdPart(route.routeType ?? "") || "retrieval";
  return `${corpusPart}-${typePart}-${index + 1}`;
}

export function createRetrievalRouteFormRow(index = 0): RagRetrievalRouteFormRow {
  return {
    corpusId: "",
    enabled: true,
    formId: nextFormId("route", index),
    routeType: "",
    strategyId: "",
  };
}

function inferRetrievalRouteType(strategyId: string): RagRetrievalRouteType {
  const normalized = strategyId.trim().toUpperCase();
  if (normalized.includes("STRUCTURED_FILTER")) return "STRUCTURED_FILTER";
  if (normalized.includes("VECTOR")) return "VECTOR";
  if (normalized.includes("FULLTEXT")) return "FULLTEXT";
  if (normalized.includes("GRAPH")) return "GRAPH";
  return "FULLTEXT";
}

export function createRetrievalFilterFormRow(index = 0): RagRetrievalFilterFormRow {
  return {
    field: "",
    formId: nextFormId("filter", index),
    operator: "eq",
    value: "",
  };
}

export function buildIngestProfileFormState(existing?: IngestProfileRecord): RagIngestProfileFormState {
  if (existing) {
    const enabledStages = new Set(existing.config.dag.nodes);
    const resolveSourceConfig = existing.config.stages.resolveSource?.config;
    const parseDocumentConfig = existing.config.stages.parseDocument?.config;
    const splitTextConfig = existing.config.stages.splitText?.config;
    const splitterConfig = splitTextConfig && typeof splitTextConfig === "object" && "splitter" in splitTextConfig
      ? splitTextConfig.splitter as Record<string, unknown>
      : undefined;

    return {
      description: existing.description,
      embeddingModelCode: existing.config.embeddingModelCode,
      embeddingVersion: existing.config.embeddingVersion,
      graphWritePolicy: existing.config.graphWritePolicy,
      id: existing.id,
      parseDocumentFormat: typeof parseDocumentConfig?.format === "string" ? parseDocumentConfig.format : "MARKDOWN",
      parseDocumentUnsupportedFormatPolicy: typeof parseDocumentConfig?.unsupportedFormatPolicy === "string" ? parseDocumentConfig.unsupportedFormatPolicy : "FAIL_FAST",
      pipelineStages: defaultIngestStageOrder.map((id) => ({
        enabled: enabledStages.has(id),
        id,
        strategyId: existing.config.stages[id]?.strategyId ?? "",
      })),
      primaryCorpusId: existing.primaryCorpusId,
      requiredSourceVersion: existing.config.requiredSourceVersion,
      resolveSourceAllowedUriSchemesText: Array.isArray(resolveSourceConfig?.allowedUriSchemes)
        ? resolveSourceConfig.allowedUriSchemes.filter((value): value is string => typeof value === "string").join(", ")
        : "",
      resolveSourceMaxBytes: typeof resolveSourceConfig?.maxBytes === "number" ? String(resolveSourceConfig.maxBytes) : "1048576",
      splitTextHeadingDepth: typeof splitterConfig?.headingDepth === "number" ? String(splitterConfig.headingDepth) : "6",
      splitTextMaxTokens: typeof splitterConfig?.maxTokens === "number" ? String(splitterConfig.maxTokens) : "800",
      splitTextOverlapTokens: typeof splitterConfig?.overlapTokens === "number" ? String(splitterConfig.overlapTokens) : "80",
      splitTextType: typeof splitterConfig?.type === "string" ? splitterConfig.type : "plainText",
      status: existing.status,
      supportedSourceTypesText: existing.config.sourceInputPolicy.supportedSourceTypes.join(", "),
    };
  }

  return {
    description: "",
    embeddingModelCode: "",
    embeddingVersion: "v1",
    graphWritePolicy: "",
    id: "",
    parseDocumentFormat: "MARKDOWN",
    parseDocumentUnsupportedFormatPolicy: "FAIL_FAST",
    pipelineStages: defaultIngestStageOrder.map((id) => ({ enabled: false, id, strategyId: "" })),
    primaryCorpusId: "",
    requiredSourceVersion: "v1",
    resolveSourceAllowedUriSchemesText: "",
    resolveSourceMaxBytes: "1048576",
    splitTextHeadingDepth: "6",
    splitTextMaxTokens: "800",
    splitTextOverlapTokens: "80",
    splitTextType: "plainText",
    status: "Ready",
    supportedSourceTypesText: "",
  };
}

export function buildRetrievalProfileFormState(existing?: RetrievalProfileRecord): RagRetrievalProfileFormState {
  if (existing) {
    return {
      contextStrategies: existing.config.contextStrategies,
      dedupPolicy: existing.config.dedupPolicy,
      description: existing.description,
      filters: existing.config.filters.map((filter, index) => ({
        ...filter,
        formId: nextFormId("filter", index),
      })),
      fusionPolicy: existing.config.fusionPolicy,
      graphExpansionPolicy: existing.config.graphExpansionPolicy,
      id: existing.id,
      primaryCorpusId: existing.primaryCorpusId,
      rerankEnabled: existing.config.rerankEnabled,
      rerankModelCode: existing.config.rerankModelCode,
      routes: existing.config.routesConfig.map((route, index) => ({
        ...route,
        formId: nextFormId("route", index),
      })),
      status: existing.status,
      topicPolicy: existing.config.topicPolicy,
    };
  }

  return {
    contextStrategies: ["RESULT_LIST", "TOKEN_BUDGET_PACK", "MEMORY_CONTEXT_PACK", "DOC_QA_CONTEXT_PACK"],
    dedupPolicy: "TEXT_UNIT",
    description: "",
    filters: [],
    fusionPolicy: "RRF",
    graphExpansionPolicy: "",
    id: "",
    primaryCorpusId: "",
    rerankEnabled: false,
    rerankModelCode: "",
    routes: [createRetrievalRouteFormRow()],
    status: "Ready",
    topicPolicy: "NONE",
  };
}

export function buildIngestProfileRecordFromForm(
  form: RagIngestProfileFormState,
  pipelineCatalog?: RagPipelineCatalog | null,
): IngestProfileRecord {
  const enabledStages = form.pipelineStages.filter((stage) => stage.enabled).map((stage) => stage.id);
  const stages = enabledStages.reduce<NonNullable<IngestProfileRecord["config"]["stages"]>>((accumulator, stageId) => {
    const stage = form.pipelineStages.find((currentStage) => currentStage.id === stageId);
    const strategyId = stage?.strategyId.trim() ?? "";
    if (!strategyId) return accumulator;

    if (stageId === "resolveSource") {
      const allowedUriSchemes = splitCommaSeparatedTokens(form.resolveSourceAllowedUriSchemesText);
      const maxBytes = parseOptionalInteger(form.resolveSourceMaxBytes);
      accumulator[stageId] = {
        config: {
          ...(allowedUriSchemes.length > 0 ? { allowedUriSchemes } : {}),
          ...(maxBytes !== undefined ? { maxBytes } : {}),
        },
        strategyId,
      };
      return accumulator;
    }

    if (stageId === "parseDocument") {
      accumulator[stageId] = {
        config: {
          ...(trimOrUndefined(form.parseDocumentFormat) ? { format: form.parseDocumentFormat.trim() } : {}),
          ...(trimOrUndefined(form.parseDocumentUnsupportedFormatPolicy)
            ? { unsupportedFormatPolicy: form.parseDocumentUnsupportedFormatPolicy.trim() }
            : {}),
        },
        strategyId,
      };
      return accumulator;
    }

    if (stageId === "splitText") {
      const headingDepth = parseOptionalInteger(form.splitTextHeadingDepth);
      const maxTokens = parseOptionalInteger(form.splitTextMaxTokens);
      const overlapTokens = parseOptionalInteger(form.splitTextOverlapTokens);
      accumulator[stageId] = {
        config: {
          splitter: {
            ...(trimOrUndefined(form.splitTextType) ? { type: form.splitTextType.trim() } : {}),
            ...(maxTokens !== undefined ? { maxTokens } : {}),
            ...(overlapTokens !== undefined ? { overlapTokens } : {}),
            ...(headingDepth !== undefined ? { headingDepth } : {}),
          },
        },
        strategyId,
      };
      return accumulator;
    }

    accumulator[stageId] = { strategyId };
    return accumulator;
  }, {});

  return {
    config: {
      dag: {
        edges: buildIngestStageEdges(enabledStages, pipelineCatalog),
        nodes: enabledStages,
      },
      embeddingModelCode: form.embeddingModelCode.trim(),
      embeddingVersion: form.embeddingVersion.trim(),
      graphWritePolicy: form.graphWritePolicy.trim(),
      requiredSourceVersion: form.requiredSourceVersion.trim(),
      sourceInputPolicy: {
        supportedSourceTypes: splitCommaSeparatedTokens(form.supportedSourceTypesText),
      },
      stages,
    },
    corpus: form.primaryCorpusId.trim(),
    corpusIds: [form.primaryCorpusId.trim()].filter(Boolean),
    description: form.description.trim(),
    id: form.id.trim(),
    primaryCorpusId: form.primaryCorpusId.trim(),
    routes: [],
    status: form.status,
    strategy: "",
    type: "Ingest",
  };
}

export function buildIngestPipelineFromForm(
  form: RagIngestProfileFormState,
  pipelineCatalog?: RagPipelineCatalog | null,
): RagProfilePipeline {
  const record = buildIngestProfileRecordFromForm(form, pipelineCatalog);
  const enabledStageIds = form.pipelineStages.filter((stage) => stage.enabled).map((stage) => stage.id);
  const nodes = form.pipelineStages
    .filter((stage) => stage.enabled)
    .map<RagPipelineNode>((stage) => ({
      config: record.config.stages[stage.id]?.config,
      enabled: true,
      failurePolicy: "FAIL_PIPELINE",
      joinPolicy: "ALL_SUCCESS",
      nodeId: ingestStageNodeId(stage.id),
      stageType: ingestStageIdToStageType(stage.id),
      strategyId: stage.strategyId.trim(),
    }));
  const nodeIdsByStageId = Object.fromEntries(nodes.map((node) => [ingestStageTypeToStageId(node.stageType), node.nodeId]));

  return {
    edges: buildIngestStageEdges(enabledStageIds, pipelineCatalog)
      .map((edge) => ({
        from: nodeIdsByStageId[edge.from],
        to: nodeIdsByStageId[edge.to],
      }))
      .filter((edge): edge is { from: string; to: string } => Boolean(edge.from && edge.to)),
    mode: "DAG",
    nodes,
  };
}

export function buildRetrievalProfileRecordFromForm(form: RagRetrievalProfileFormState): RetrievalProfileRecord {
  const enabledCorpusIds = Array.from(new Set(form.routes.map((route) => route.corpusId.trim()).filter(Boolean)));
  const corpusIds = Array.from(new Set([form.primaryCorpusId.trim(), ...enabledCorpusIds].filter(Boolean)));

  return {
    config: {
      contextStrategies: Array.from(new Set(form.contextStrategies.map((strategy) => strategy.trim()).filter(Boolean))),
      dedupPolicy: form.dedupPolicy.trim(),
      filters: form.filters
        .map((filter) => ({
          field: filter.field.trim(),
          operator: filter.operator.trim(),
          value: filter.value.trim(),
        }))
        .filter((filter) => filter.field && filter.operator && filter.value),
      fusionPolicy: form.fusionPolicy.trim(),
      graphExpansionPolicy: form.graphExpansionPolicy.trim(),
      rerankEnabled: form.rerankEnabled,
      rerankModelCode: form.rerankModelCode.trim(),
      routesConfig: form.routes
        .map((route) => ({
          corpusId: route.corpusId.trim(),
          enabled: route.enabled,
          routeId: route.routeId?.trim(),
          routeType: inferRetrievalRouteType(route.strategyId),
          strategyId: route.strategyId.trim(),
        }))
        .filter((route) => route.corpusId && route.strategyId),
      topicPolicy: form.topicPolicy.trim(),
    },
    corpus: corpusIds.join(" + "),
    corpusIds,
    description: form.description.trim(),
    id: form.id.trim(),
    primaryCorpusId: form.primaryCorpusId.trim(),
    routes: [],
    status: form.status,
    strategy: "",
    type: "Retrieval",
  };
}

export function buildRetrievalPipelineFromForm(form: RagRetrievalProfileFormState): RagProfilePipeline {
  const routeNodes = form.routes
    .filter((route) => route.enabled)
    .map<RagPipelineNode>((route, index) => ({
      config: {
        corpusId: route.corpusId.trim(),
        routeType: inferRetrievalRouteType(route.strategyId),
      },
      enabled: true,
      failurePolicy: "CONTINUE_WITH_WARNING",
      joinPolicy: "BEST_EFFORT",
      nodeId: retrievalNodeId(route, index),
      stageType: "ROUTE_SEARCH",
      strategyId: route.strategyId.trim(),
    }));
  const fuseNodeId = "fuse-results";
  const dedupNodeId = "dedup-results";
  const contextNodeId = "build-context";
  const nodes = [
    ...routeNodes,
    {
      enabled: true,
      failurePolicy: "FAIL_PIPELINE",
      joinPolicy: "ALL_SUCCESS",
      nodeId: fuseNodeId,
      stageType: "FUSE_RESULTS",
      strategyId: form.fusionPolicy.trim() || "SUM_SCORE",
    },
    {
      enabled: true,
      failurePolicy: "FAIL_PIPELINE",
      joinPolicy: "ALL_SUCCESS",
      nodeId: dedupNodeId,
      stageType: "DEDUP_RESULTS",
      strategyId: form.dedupPolicy.trim() || "TEXT_UNIT",
    },
    {
      enabled: true,
      failurePolicy: "FAIL_PIPELINE",
      joinPolicy: "ALL_SUCCESS",
      nodeId: contextNodeId,
      stageType: "BUILD_CONTEXT",
      strategyId: form.contextStrategies[0]?.trim() || "RESULT_LIST",
    },
  ] satisfies RagPipelineNode[];

  return {
    edges: buildRetrievalPipelineEdges(routeNodes, fuseNodeId, dedupNodeId, contextNodeId),
    mode: "DAG",
    nodes,
  };
}

export function buildIngestProfilePayloadPreview(
  form: RagIngestProfileFormState,
  pipelineCatalog?: RagPipelineCatalog | null,
) {
  return {
    corpusIds: [form.primaryCorpusId.trim()].filter(Boolean),
    description: form.description.trim(),
    pipeline: buildIngestPipelineFromForm(form, pipelineCatalog),
    primaryCorpusId: form.primaryCorpusId.trim(),
    profileId: form.id.trim(),
    profileType: "Ingest",
    status: form.status,
  };
}

export function buildRetrievalProfilePayloadPreview(form: RagRetrievalProfileFormState) {
  const record = buildRetrievalProfileRecordFromForm(form);
  return {
    corpusIds: record.corpusIds,
    description: form.description.trim(),
    pipeline: buildRetrievalPipelineFromForm(form),
    primaryCorpusId: form.primaryCorpusId.trim(),
    profileId: form.id.trim(),
    profileType: "Retrieval",
    status: form.status,
  };
}

export function validateIngestProfileForm(
  form: RagIngestProfileFormState,
  existingIds: string[],
  currentProfileId?: string,
  allowedStageStrategies?: Partial<Record<RagIngestStageId, string[]>>,
  pipelineCatalog?: RagPipelineCatalog | null,
): RagIngestProfileFormErrors {
  const errors: RagIngestProfileFormErrors = {};
  const normalizedId = form.id.trim();
  if (!normalizedId) {
    errors.id = "Profile ID is required.";
  } else if (existingIds.some((existingId) => existingId === normalizedId && existingId !== currentProfileId)) {
    errors.id = "Profile ID must be unique.";
  }

  if (!form.primaryCorpusId.trim()) {
    errors.primaryCorpusId = "Primary corpus is required.";
  }

  if (form.status !== "Ready") {
    errors.status = "Current RAG runtime only saves Ready profiles.";
  }

  const enabledStages = form.pipelineStages.filter((stage) => stage.enabled);
  if (enabledStages.length === 0) {
    errors.pipeline = "At least one pipeline stage must be enabled.";
  } else if (pipelineCatalog?.stages.length) {
    const enabledStageIds = new Set(enabledStages.map((stage) => stage.id));
    const missingRequiredStages = requiredIngestStageIds(pipelineCatalog).filter((stageId) => !enabledStageIds.has(stageId));
    if (missingRequiredStages.length > 0) {
      errors.pipeline = `Required stages must stay enabled: ${missingRequiredStages.join(", ")}.`;
    }
  } else {
    const enabledStageIds = new Set(enabledStages.map((stage) => stage.id));
    if (!enabledStageIds.has("preparePayload") || !enabledStageIds.has("persistSource")) {
      errors.pipeline = "preparePayload and persistSource must both be enabled.";
    } else if (!enabledStageIds.has("splitText") || !enabledStageIds.has("fulltextIndex") || !enabledStageIds.has("vectorIndex")) {
      errors.pipeline = "persistSource requires splitText, fulltextIndex, and vectorIndex.";
    } else if (enabledStageIds.has("resolveSource") && !enabledStageIds.has("validateSource")) {
      errors.pipeline = "resolveSource requires validateSource.";
    } else if (enabledStageIds.has("parseDocument") && !enabledStageIds.has("resolveSource")) {
      errors.pipeline = "parseDocument requires resolveSource.";
    } else if (enabledStageIds.has("extractTree") && !enabledStageIds.has("splitText")) {
      errors.pipeline = "extractTree requires splitText.";
    } else if (enabledStageIds.has("persistTree") && !enabledStageIds.has("extractTree")) {
      errors.pipeline = "persistTree requires extractTree.";
    }
  }

  const stageRows: NonNullable<RagIngestProfileFormErrors["stageRows"]> = {};
  enabledStages.forEach((stage) => {
    if (!stage.strategyId.trim()) {
      stageRows[stage.id] = { strategyId: "Strategy ID is required." };
      return;
    }

    const allowedStrategies = allowedStageStrategies?.[stage.id];
    if (allowedStrategies && !allowedStrategies.includes(stage.strategyId.trim())) {
      stageRows[stage.id] = { strategyId: "Select one of the available stage strategies." };
    }
  });
  if (Object.keys(stageRows).length > 0) {
    errors.stageRows = stageRows;
  }

  if (enabledStages.some((stage) => stage.id === "resolveSource")) {
    const maxBytes = parseOptionalInteger(form.resolveSourceMaxBytes);
    if (trimOrUndefined(form.resolveSourceMaxBytes) && (maxBytes === undefined || maxBytes <= 0)) {
      errors.resolveSourceMaxBytes = "Max bytes must be a positive integer.";
    }
  }

  if (enabledStages.some((stage) => stage.id === "parseDocument") && !trimOrUndefined(form.parseDocumentFormat)) {
    errors.parseDocumentFormat = "Document format is required.";
  }

  if (enabledStages.some((stage) => stage.id === "splitText")) {
    const maxTokens = parseOptionalInteger(form.splitTextMaxTokens);
    const overlapTokens = parseOptionalInteger(form.splitTextOverlapTokens);
    const headingDepth = parseOptionalInteger(form.splitTextHeadingDepth);

    if (!trimOrUndefined(form.splitTextType)) {
      errors.splitTextType = "Splitter type is required.";
    }
    if (maxTokens === undefined || maxTokens <= 0) {
      errors.splitTextMaxTokens = "Max tokens must be a positive integer.";
    }
    if (overlapTokens === undefined || overlapTokens < 0) {
      errors.splitTextOverlapTokens = "Overlap tokens must be zero or greater.";
    }
    if (form.splitTextType.trim() === "markdownHeading" && (headingDepth === undefined || headingDepth < 1 || headingDepth > 6)) {
      errors.splitTextHeadingDepth = "Heading depth must be between 1 and 6.";
    }
  }

  if (splitCommaSeparatedTokens(form.supportedSourceTypesText).length === 0) {
    errors.supportedSourceTypesText = "Supported Source Types is required.";
  }

  return errors;
}

export function listRetrievalFilterFieldOptions(
  corpora: CorpusRecord[],
  routes: RagRetrievalRouteFormRow[],
) {
  const fields = new Set<string>();
  const corporaByCode = new Map(corpora.map((corpus) => [corpus.code, corpus]));

  routes.forEach((route) => {
    const corpus = corporaByCode.get(route.corpusId);
    if (!corpus) return;
    const schemaFields = corpus.config ? Object.keys(corpus.config.filterSchema.fields) : corpus.filters;
    schemaFields.forEach((field) => fields.add(field));
  });

  return Array.from(fields).sort((left, right) => left.localeCompare(right));
}

export function validateRetrievalProfileForm(
  form: RagRetrievalProfileFormState,
  existingIds: string[],
  corpora: CorpusRecord[],
  currentProfileId?: string,
  allowedRouteStrategies?: Record<string, string[]>,
): RagRetrievalProfileFormErrors {
  const errors: RagRetrievalProfileFormErrors = {};
  const normalizedId = form.id.trim();
  if (!normalizedId) {
    errors.id = "Profile ID is required.";
  } else if (existingIds.some((existingId) => existingId === normalizedId && existingId !== currentProfileId)) {
    errors.id = "Profile ID must be unique.";
  }

  if (!form.primaryCorpusId.trim()) {
    errors.primaryCorpusId = "Primary corpus is required.";
  }

  if (form.status !== "Ready") {
    errors.status = "Current RAG runtime only saves Ready profiles.";
  }

  if (form.contextStrategies.length === 0) {
    errors.contextStrategies = "At least one context strategy must be enabled.";
  }

  const corporaByCode = new Set(corpora.map((corpus) => corpus.code));
  const enabledRoutes = form.routes.filter((route) => route.enabled);
  const routeRows: NonNullable<RagRetrievalProfileFormErrors["routeRows"]> = {};

  form.routes.forEach((route) => {
    const rowError: NonNullable<RagRetrievalProfileFormErrors["routeRows"]>[string] = {};
    if (!route.corpusId.trim()) {
      rowError.corpusId = "Route corpus is required.";
    } else if (!corporaByCode.has(route.corpusId.trim())) {
      rowError.corpusId = "Route corpus must match an existing corpus.";
    }

    if (!route.strategyId.trim()) {
      rowError.strategyId = "Strategy ID is required.";
    } else {
      const routeOptions = allowedRouteStrategies?.[route.formId];
      if (routeOptions && !routeOptions.includes(route.strategyId.trim())) {
        rowError.strategyId = "Select one of the available route strategies.";
      }
    }

    if (Object.keys(rowError).length > 0) {
      routeRows[route.formId] = rowError;
    }
  });

  if (enabledRoutes.length === 0) {
    errors.routes = "At least one route must be enabled.";
  }

  if (form.primaryCorpusId.trim() && !enabledRoutes.some((route) => route.corpusId.trim() === form.primaryCorpusId.trim())) {
    errors.routes = errors.routes ?? "Primary corpus must appear in at least one enabled route.";
  }

  if (Object.keys(routeRows).length > 0) {
    errors.routeRows = routeRows;
  }

  if (form.graphExpansionPolicy.trim() && !enabledRoutes.some((route) => inferRetrievalRouteType(route.strategyId) === "GRAPH")) {
    errors.graphExpansionPolicy = "Graph expansion requires at least one enabled graph strategy.";
  }

  const availableFields = new Set(listRetrievalFilterFieldOptions(corpora, form.routes));
  const filterRows: NonNullable<RagRetrievalProfileFormErrors["filterRows"]> = {};
  form.filters.forEach((filter) => {
    const rowError: NonNullable<RagRetrievalProfileFormErrors["filterRows"]>[string] = {};
    if (!filter.field.trim()) {
      rowError.field = "Filter field is required.";
    } else if (!availableFields.has(filter.field.trim())) {
      rowError.field = "Filter field must belong to one of the selected route corpora.";
    }

    if (!filter.operator.trim()) {
      rowError.operator = "Filter operator is required.";
    }

    if (!filter.value.trim()) {
      rowError.value = "Filter value is required.";
    }

    if (Object.keys(rowError).length > 0) {
      filterRows[filter.formId] = rowError;
    }
  });

  if (Object.keys(filterRows).length > 0) {
    errors.filterRows = filterRows;
    errors.filters = "Every filter row must resolve to a field, operator, and value.";
  }

  return errors;
}
