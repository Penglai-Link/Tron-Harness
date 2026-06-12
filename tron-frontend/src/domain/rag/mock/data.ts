import type {
  CorpusRecord,
  IngestProfileRecord,
  ProfileRecord,
  RagCorpusConfig,
  RagCorpusStatusCode,
  RagIngestProfileConfig,
  RagIngestStageId,
  RagProfileStatus,
  RagRetrievalProfileConfig,
  RagRetrievalProfileFilter,
  RagRetrievalRouteConfig,
  RagRetrievalRouteType,
  RagStorageBackend,
  RagStorageBinding,
  RagStorageTargetBinding,
  RetrievalProfileRecord,
  SourceRecord,
  TextUnitRecord,
  TraceRecord,
  WarningRecord,
} from "@/domain/rag/model/RagModel";
import {
  formatRagStorageBinding,
  normalizeRagStorageBinding,
} from "@/domain/rag/model/RagModel";

export const corpora: CorpusRecord[] = [
  {
    code: "project-doc",
    description: "Project documentation sources and text units.",
    filters: ["scopeType", "scopeCode", "documentCode", "repoCode"],
    fulltextHealth: "Healthy",
    ingestProfile: "project-doc-ingest-v1",
    lastIndexed: "8 min ago",
    name: "Project Document",
    retrievalProfile: "project-doc-related-v1",
    scope: "workspace",
    sources: 184,
    status: "Active",
    storageBinding: "sqlite.rag_project_doc_sources",
    strategies: ["FULLTEXT_SQLITE_FTS5_SIMPLE_V1", "VECTOR_SQLITE_VEC0_COSINE_V1"],
    textUnits: 68420,
    vectorCoverage: 97,
    warnings: 2,
  },
  {
    code: "memory-card",
    description: "User, agent, and space scoped memory cards.",
    filters: ["scopeType", "scopeCode", "memoryType", "memoryStatus"],
    fulltextHealth: "Healthy",
    ingestProfile: "memory-card-ingest-v1",
    lastIndexed: "3 min ago",
    name: "Memory Card",
    retrievalProfile: "memory-related-v1",
    scope: "user + workspace",
    sources: 923,
    status: "Active",
    storageBinding: "sqlite.rag_memory_text_units",
    strategies: [
      "STRUCTURED_FILTER_SQLITE_V1",
      "FULLTEXT_SQLITE_FTS5_SIMPLE_V1",
      "VECTOR_SQLITE_VEC0_COSINE_V1",
    ],
    textUnits: 142086,
    vectorCoverage: 94,
    warnings: 4,
  },
  {
    code: "skill",
    description: "Workspace-scoped skill names and descriptions.",
    filters: ["scopeType", "scopeCode", "skillName"],
    fulltextHealth: "Healthy",
    ingestProfile: "skill-ingest-v1",
    lastIndexed: "11 min ago",
    name: "Skill",
    retrievalProfile: "skill-related-v1",
    scope: "workspace",
    sources: 96,
    status: "Active",
    storageBinding: "sqlite.rag_skill_text_units",
    strategies: ["FULLTEXT_SQLITE_FTS5_SIMPLE_V1", "VECTOR_SQLITE_VEC0_COSINE_V1"],
    textUnits: 18640,
    vectorCoverage: 93,
    warnings: 1,
  },
  {
    code: "skill-graph",
    description: "Graph-backed workspace-scoped skill nodes and related graph data.",
    filters: ["scopeType", "scopeCode", "skillName"],
    fulltextHealth: "Backfill",
    ingestProfile: "skill-graph-ingest-v1",
    lastIndexed: "22 min ago",
    name: "Skill Graph",
    retrievalProfile: "skill-graph-related-v1",
    scope: "workspace",
    sources: 78,
    status: "Indexing",
    storageBinding: "graph.rag_skill",
    strategies: ["FULLTEXT_GRAPH_SINGLE_NODE_V1", "VECTOR_GRAPH_SINGLE_NODE_V1", "GRAPH_EXPANSION_POLICY"],
    textUnits: 12840,
    vectorCoverage: 81,
    warnings: 6,
  },
  {
    code: "code-graph",
    description: "Reserved corpus for code entity retrieval once semantic projection is available.",
    filters: ["repository", "language", "symbolType"],
    fulltextHealth: "Reserved",
    ingestProfile: "",
    lastIndexed: "Not enabled",
    name: "Code Graph",
    retrievalProfile: "code-graph-reserved-v1",
    scope: "repository",
    sources: 0,
    status: "Reserved",
    storageBinding: "graph.code_graph",
    strategies: ["GRAPH", "SUMMARY_TREE"],
    textUnits: 0,
    vectorCoverage: 0,
    warnings: 0,
  },
];

export const RAG_MOCK_CORPORA_STORAGE_KEY = "tron.rag.mock.corpora";

const corpusStatuses = new Set<CorpusRecord["status"]>(["Active", "Indexing", "Warning", "Reserved"]);

const statusToCode: Record<CorpusRecord["status"], RagCorpusStatusCode> = {
  Active: "enabled",
  Indexing: "enabled",
  Reserved: "reserved",
  Warning: "enabled",
};

function sanitizeKebabToken(value: string) {
  return value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

function sanitizeSnakeToken(value: string) {
  return sanitizeKebabToken(value).replace(/-/g, "_");
}

function toPascalCase(value: string) {
  return sanitizeKebabToken(value)
    .split("-")
    .filter(Boolean)
    .map((token) => `${token.slice(0, 1).toUpperCase()}${token.slice(1)}`)
    .join("");
}

function appendImplicitDiagnosticsTarget(
  corpus: CorpusRecord,
  sourceTargetId: string,
  targets: RagStorageTargetBinding[],
  bindings: Partial<Record<"source" | "textUnit" | "fulltext" | "vector" | "diagnostics", string>>,
): RagStorageBinding {
  const sourceTarget = targets.find((target) => target.id === sourceTargetId);
  if (!sourceTarget) {
    return {
      bindings: {
        source: sourceTargetId,
      },
      mode: "TARGETED",
      targets,
    };
  }

  const corpusIdentifier = sanitizeKebabToken(corpus.code) || sanitizeKebabToken(corpus.name) || "temporary-corpus";
  const diagnosticsId = `${corpusIdentifier}-diagnostics`;
  const diagnosticsTarget: RagStorageTargetBinding = sourceTarget.backend === "GRAPH"
    ? {
        backend: "GRAPH",
        codeStrategy: sourceTarget.codeStrategy,
        id: diagnosticsId,
        layout: "GRAPH_SINGLE_NODE",
        namespace: sourceTarget.namespace,
        nodeLabel: `${toPascalCase(corpusIdentifier)}Diagnostics`,
      }
    : {
        backend: "SQLITE",
        id: diagnosticsId,
        layout: "TABLE",
        table: `rag_${sanitizeSnakeToken(corpusIdentifier)}_diagnostics`,
      };

  return {
    bindings: {
      ...bindings,
      diagnostics: diagnosticsTarget.id,
    },
    mode: "TARGETED",
    targets: [...targets, diagnosticsTarget],
  };
}

function defaultStorageBinding(corpus: CorpusRecord): RagStorageBinding {
  if (corpus.code === "project-doc") {
    return appendImplicitDiagnosticsTarget(corpus, "source-store", [
      { backend: "SQLITE", id: "source-store", layout: "TABLE", table: "rag_project_doc_sources" },
      { backend: "SQLITE", id: "content-store", layout: "TABLE", table: "rag_project_doc_text_units" },
    ], {
      fulltext: "content-store",
      source: "source-store",
      textUnit: "content-store",
      vector: "content-store",
    });
  }
  if (corpus.code === "memory-card") {
    return appendImplicitDiagnosticsTarget(corpus, "content-store", [
      { backend: "SQLITE", id: "content-store", layout: "TABLE", table: "rag_memory_text_units" },
    ], {
      fulltext: "content-store",
      source: "content-store",
      textUnit: "content-store",
      vector: "content-store",
    });
  }
  if (corpus.code === "skill") {
    return appendImplicitDiagnosticsTarget(corpus, "content-store", [
      { backend: "SQLITE", id: "content-store", layout: "TABLE", table: "rag_skill_text_units" },
    ], {
      fulltext: "content-store",
      source: "content-store",
      textUnit: "content-store",
      vector: "content-store",
    });
  }
  if (corpus.code === "skill-graph") {
    return appendImplicitDiagnosticsTarget(corpus, "graph-store", [
      {
        backend: "GRAPH",
        codeStrategy: "CORPUS_SOURCE_VERSION_SCOPE_HASH",
        id: "graph-store",
        layout: "GRAPH_SINGLE_NODE",
        namespace: "rag_skill",
        nodeLabel: "RagSkill",
      },
    ], {
      fulltext: "graph-store",
      source: "graph-store",
      textUnit: "graph-store",
      vector: "graph-store",
    });
  }
  return corpus.storageBinding.startsWith("graph.")
    ? appendImplicitDiagnosticsTarget(corpus, "graph-store", [
      {
        backend: "GRAPH",
        codeStrategy: "CORPUS_SOURCE_VERSION_SCOPE_HASH",
        id: "graph-store",
        layout: "GRAPH_SINGLE_NODE",
        namespace: corpus.code.replaceAll("-", "_"),
        nodeLabel: corpus.name.replaceAll(/\s+/g, ""),
      },
    ], {
      fulltext: "graph-store",
      source: "graph-store",
      textUnit: "graph-store",
      vector: "graph-store",
    })
    : appendImplicitDiagnosticsTarget(corpus, "source-store", [
      {
        backend: "SQLITE",
        id: "source-store",
        layout: "TABLE",
        table: corpus.storageBinding.replace(/^sqlite\./, "") || "rag_memory_sources",
      },
      {
        backend: "SQLITE",
        id: "content-store",
        layout: "TABLE",
        table: "rag_memory_text_units",
      },
    ], {
      fulltext: "content-store",
      source: "source-store",
      textUnit: "content-store",
      vector: "content-store",
    });
}

function defaultFilterSchema(corpus: CorpusRecord): RagCorpusConfig["filterSchema"] {
  if (corpus.code === "project-doc") {
    return {
      fields: {
        scopeType: { column: "scope_type", operators: ["eq", "in"], type: "string" },
        scopeCode: { column: "scope_code", operators: ["eq", "in"], type: "string" },
        sourceCode: { column: "source_code", operators: ["eq", "in"], type: "string" },
        sourceVersion: { column: "source_version", operators: ["eq", "in"], type: "string" },
        status: { column: "status", operators: ["eq", "in"], type: "string" },
        documentCode: { column: "document_code", operators: ["eq", "in"], type: "string" },
        documentType: { column: "document_type", operators: ["eq", "in"], type: "string" },
        path: { column: "path", operators: ["eq", "in"], type: "string" },
        repoCode: { column: "repo_code", operators: ["eq", "in"], type: "string" },
        moduleCode: { column: "module_code", operators: ["eq", "in"], type: "string" },
        moduleName: { column: "module_name", operators: ["eq", "in"], type: "string" },
      },
    };
  }
  if (corpus.code === "memory-card") {
    return {
      fields: {
        scopeType: { column: "scope_type", operators: ["eq", "in"], type: "string" },
        scopeCode: { column: "scope_code", operators: ["eq", "in"], type: "string" },
        sourceCode: { column: "source_code", operators: ["eq", "in"], type: "string" },
        sourceVersion: { column: "source_version", operators: ["eq", "in"], type: "string" },
        status: { column: "source_status", operators: ["eq", "in"], type: "string" },
        sourceStatus: { column: "source_status", operators: ["eq", "in"], type: "string" },
        textUnitStatus: { column: "status", operators: ["eq", "in"], type: "string" },
        memoryType: { column: "memory_type", operators: ["eq", "in"], type: "string" },
        memoryStatus: { column: "memory_status", operators: ["eq", "in"], type: "string" },
        reviewStatus: { column: "review_status", operators: ["eq", "in"], type: "string" },
        version: { column: "version", operators: ["eq", "in"], type: "string" },
        supersededBy: { column: "superseded_by", operators: ["eq", "in"], type: "string" },
        conflictGroup: { column: "conflict_group", operators: ["eq", "in"], type: "string" },
        duplicateGroup: { column: "duplicate_group", operators: ["eq", "in"], type: "string" },
        injectable: { column: "injectable", operators: ["eq", "in"], type: "boolean" },
        importanceScore: { column: "importance_score", operators: ["eq", "in", "gt", "gte", "lt", "lte"], type: "integer" },
        confidenceScore: { column: "confidence_score", operators: ["eq", "in", "gt", "gte", "lt", "lte"], type: "integer" },
        qualityScore: { column: "quality_score", operators: ["eq", "in", "gt", "gte", "lt", "lte"], type: "integer" },
        pinLevel: { column: "pin_level", operators: ["eq", "in", "gt", "gte", "lt", "lte"], type: "integer" },
        expiresAt: { column: "expires_at", operators: ["eq", "in", "gt", "gte", "lt", "lte"], type: "integer" },
        derivedSourceType: { column: "derived_source_type", operators: ["eq", "in"], type: "string" },
        derivedSourceCode: { column: "derived_source_code", operators: ["eq", "in"], type: "string" },
      },
    };
  }
  if (corpus.code === "skill") {
    return {
      fields: {
        scopeType: { column: "scope_type", operators: ["eq", "in"], type: "string" },
        scopeCode: { column: "scope_code", operators: ["eq", "in"], type: "string" },
        sourceCode: { column: "source_code", operators: ["eq", "in"], type: "string" },
        sourceVersion: { column: "source_version", operators: ["eq", "in"], type: "string" },
        status: { column: "source_status", operators: ["eq", "in"], type: "string" },
        sourceStatus: { column: "source_status", operators: ["eq", "in"], type: "string" },
        textUnitStatus: { column: "status", operators: ["eq", "in"], type: "string" },
        skillName: { column: "skill_name", operators: ["eq", "in"], type: "string" },
      },
    };
  }
  if (corpus.code === "skill-graph") {
    return {
      fields: {
        scopeType: { column: "scopeType", operators: ["eq", "in"], type: "string" },
        scopeCode: { column: "scopeCode", operators: ["eq", "in"], type: "string" },
        sourceCode: { column: "sourceCode", operators: ["eq", "in"], type: "string" },
        sourceVersion: { column: "sourceVersion", operators: ["eq", "in"], type: "string" },
        status: { column: "sourceStatus", operators: ["eq", "in"], type: "string" },
        sourceStatus: { column: "sourceStatus", operators: ["eq", "in"], type: "string" },
        textUnitStatus: { column: "textUnitStatus", operators: ["eq", "in"], type: "string" },
        skillName: { column: "skillName", operators: ["eq", "in"], type: "string" },
      },
    };
  }
  return {
    fields: Object.fromEntries(
      corpus.filters.map((filter) => [filter, { column: filter, operators: ["eq", "in"], type: "string" as const }]),
    ),
  };
}

function defaultFieldMapping(corpus: CorpusRecord): RagCorpusConfig["fieldMapping"] {
  if (corpus.code === "project-doc") {
    return {
      source: {
        sourceCode: "source_code",
        sourceVersion: "source_version",
        sourceType: "source_type",
        scopeType: "scope_type",
        scopeCode: "scope_code",
        status: "status",
        title: "title",
        contentHash: "content_hash",
        metadata: "metadata",
        documentCode: "document_code",
        documentType: "document_type",
        path: "path",
        repoCode: "repo_code",
        moduleCode: "module_code",
        moduleName: "module_name",
      },
      textUnit: {
        sourceCode: "source_code",
        sourceVersion: "source_version",
        scopeType: "scope_type",
        scopeCode: "scope_code",
        documentCode: "document_code",
        text: "text",
      },
    };
  }
  if (corpus.code === "memory-card") {
    return {
      source: {
        sourceCode: "source_code",
        sourceVersion: "source_version",
        sourceType: "source_type",
        scopeType: "scope_type",
        scopeCode: "scope_code",
        status: "source_status",
        sourceStatus: "source_status",
        title: "title",
        sourceContentHash: "source_content_hash",
        sourceMetadata: "source_metadata",
        memoryType: "memory_type",
        memoryStatus: "memory_status",
        reviewStatus: "review_status",
        version: "version",
        supersededBy: "superseded_by",
        conflictGroup: "conflict_group",
        duplicateGroup: "duplicate_group",
        injectable: "injectable",
        importanceScore: "importance_score",
        confidenceScore: "confidence_score",
        qualityScore: "quality_score",
        pinLevel: "pin_level",
        expiresAt: "expires_at",
        derivedSourceType: "derived_source_type",
        derivedSourceCode: "derived_source_code",
      },
      textUnit: {
        sourceCode: "source_code",
        sourceVersion: "source_version",
        scopeType: "scope_type",
        scopeCode: "scope_code",
        textUnitStatus: "status",
        text: "text",
      },
    };
  }
  if (corpus.code === "skill") {
    return {
      source: {
        sourceCode: "source_code",
        sourceVersion: "source_version",
        sourceType: "source_type",
        scopeType: "scope_type",
        scopeCode: "scope_code",
        status: "source_status",
        sourceStatus: "source_status",
        title: "title",
        sourceContentHash: "source_content_hash",
        sourceMetadata: "source_metadata",
        skillName: "skill_name",
        skillDescription: "skill_description",
      },
      textUnit: {
        sourceCode: "source_code",
        sourceVersion: "source_version",
        scopeType: "scope_type",
        scopeCode: "scope_code",
        textUnitStatus: "status",
        text: "text",
      },
    };
  }
  if (corpus.code === "skill-graph") {
    return {
      source: {
        sourceCode: "sourceCode",
        sourceVersion: "sourceVersion",
        sourceType: "sourceType",
        scopeType: "scopeType",
        scopeCode: "scopeCode",
        status: "sourceStatus",
        sourceStatus: "sourceStatus",
        title: "title",
        sourceMetadata: "sourceMetadataJson",
        skillName: "skillName",
        skillDescription: "skillDescription",
      },
      textUnit: {
        sourceCode: "sourceCode",
        sourceVersion: "sourceVersion",
        scopeType: "scopeType",
        scopeCode: "scopeCode",
        targetCode: "targetCode",
        textUnitStatus: "textUnitStatus",
        skillName: "skillName",
        skillDescription: "skillDescription",
        text: "text",
      },
    };
  }
  if (corpus.code === "code-graph") {
    return {
      source: {},
      textUnit: {},
    };
  }
  return {
    source: {
      sourceCode: "source_code",
      sourceVersion: "source_version",
    },
    textUnit: {
      sourceCode: "source_code",
      sourceVersion: "source_version",
      text: "text",
    },
  };
}

export function buildDefaultRagCorpusConfig(corpus: CorpusRecord): RagCorpusConfig {
  return {
    capabilities: {
      fulltext: corpus.strategies.includes("FULLTEXT"),
      graph: corpus.strategies.some((strategy) => strategy.includes("GRAPH")),
      reserved: corpus.status === "Reserved",
      vector: corpus.strategies.some((strategy) => strategy.includes("VECTOR")),
    },
    corpusId: corpus.code,
    description: corpus.description,
    displayName: corpus.name,
    fieldMapping: defaultFieldMapping(corpus),
    filterSchema: defaultFilterSchema(corpus),
    status: statusToCode[corpus.status],
    storageBinding: defaultStorageBinding(corpus),
  };
}

function strategiesFromConfig(corpus: CorpusRecord, config: RagCorpusConfig) {
  const strategies = new Set(corpus.strategies);
  if (config.capabilities.fulltext) strategies.add("FULLTEXT");
  if (config.capabilities.vector) strategies.add("VECTOR");
  if (config.capabilities.graph) strategies.add("GRAPH");
  return Array.from(strategies);
}

function withStructuredConfig(corpus: CorpusRecord): CorpusRecord {
  const config = corpus.config ?? buildDefaultRagCorpusConfig(corpus);
  const normalizedBinding = normalizeRagStorageBinding(config.storageBinding);
  return {
    ...corpus,
    code: config.corpusId || corpus.code,
    config: {
      ...config,
      storageBinding: normalizedBinding,
    },
    description: config.description || corpus.description,
    filters: Object.keys(config.filterSchema.fields),
    name: config.displayName || corpus.name,
    storageBinding: formatRagStorageBinding(normalizedBinding),
    strategies: strategiesFromConfig(corpus, config),
  };
}

function canUseBrowserStorage() {
  return typeof window !== "undefined" && Boolean(window.localStorage);
}

function isCorpusRecord(value: unknown): value is CorpusRecord {
  if (!value || typeof value !== "object") return false;

  const record = value as Partial<CorpusRecord>;
  return typeof record.code === "string"
    && typeof record.name === "string"
    && typeof record.description === "string"
    && typeof record.storageBinding === "string"
    && typeof record.ingestProfile === "string"
    && typeof record.retrievalProfile === "string"
    && typeof record.fulltextHealth === "string"
    && typeof record.lastIndexed === "string"
    && typeof record.scope === "string"
    && typeof record.sources === "number"
    && typeof record.textUnits === "number"
    && typeof record.vectorCoverage === "number"
    && typeof record.warnings === "number"
    && Array.isArray(record.filters)
    && Array.isArray(record.strategies)
    && Boolean(record.status && corpusStatuses.has(record.status));
}

function readStoredRagCorpora(): CorpusRecord[] {
  if (!canUseBrowserStorage()) return [];

  try {
    const rawValue = window.localStorage.getItem(RAG_MOCK_CORPORA_STORAGE_KEY);
    if (!rawValue) return [];
    const parsedValue: unknown = JSON.parse(rawValue);
    return Array.isArray(parsedValue) ? parsedValue.filter(isCorpusRecord) : [];
  } catch {
    return [];
  }
}

export function getRagMockCorpora(): CorpusRecord[] {
  const storedCorpora = readStoredRagCorpora().map(withStructuredConfig);
  const storedByCode = new Map(storedCorpora.map((corpus) => [corpus.code, corpus]));
  const seedCodes = new Set(corpora.map((corpus) => corpus.code));
  const mergedSeedCorpora = corpora.map((corpus) => storedByCode.get(corpus.code) ?? withStructuredConfig(corpus));
  const createdCorpora = storedCorpora.filter((corpus) => !seedCodes.has(corpus.code));

  return [...mergedSeedCorpora, ...createdCorpora];
}

export function findRagMockCorpus(corpusCode: string) {
  return getRagMockCorpora().find((corpus) => corpus.code === corpusCode);
}

export function saveRagMockCorpus(corpus: CorpusRecord) {
  const structuredCorpus = withStructuredConfig(corpus);
  if (!canUseBrowserStorage()) return structuredCorpus;

  const storedCorpora = readStoredRagCorpora();
  const existingIndex = storedCorpora.findIndex((storedCorpus) => storedCorpus.code === structuredCorpus.code);
  const nextCorpora = existingIndex >= 0
    ? storedCorpora.map((storedCorpus, index) => (index === existingIndex ? structuredCorpus : storedCorpus))
    : [...storedCorpora, structuredCorpus];

  window.localStorage.setItem(RAG_MOCK_CORPORA_STORAGE_KEY, JSON.stringify(nextCorpora));
  return structuredCorpus;
}

export const RAG_MOCK_PROFILES_STORAGE_KEY = "tron.rag.mock.profiles";

type RagMockStrategyStatus = "Enabled" | "Reserved" | "Unsupported";

export interface RagMockIngestStageStrategy {
  status: RagMockStrategyStatus;
  strategyId: string;
  supportedCorpusCodes?: string[];
  supportedStageIds: RagIngestStageId[];
}

export interface RagMockRetrievalStrategy {
  backends: RagStorageBackend[];
  routeKinds: RagRetrievalRouteType[];
  status: RagMockStrategyStatus;
  strategyId: string;
}

export interface RagMockStrategyRegistry {
  contextStrategies: string[];
  ingestStageStrategies: RagMockIngestStageStrategy[];
  policyOptions: {
    dedupTypes: string[];
    degradePolicies: string[];
    fusionTypes: string[];
    topicTypes: string[];
  };
  retrievalStrategies: RagMockRetrievalStrategy[];
}

const ragMockStrategyRegistry: RagMockStrategyRegistry = {
  contextStrategies: [
    "RESULT_LIST",
    "TOKEN_BUDGET_PACK",
    "MEMORY_CONTEXT_PACK",
    "DOC_QA_CONTEXT_PACK",
  ],
  ingestStageStrategies: [
    {
      status: "Enabled",
      strategyId: "RAG_VALIDATE_SOURCE",
      supportedStageIds: ["validateSource"],
    },
    {
      status: "Enabled",
      strategyId: "RAG_TEXT_CONTRACT_PAYLOAD_V1",
      supportedCorpusCodes: ["memory-card", "skill", "skill-graph"],
      supportedStageIds: ["preparePayload"],
    },
    {
      status: "Enabled",
      strategyId: "RAG_DOCUMENT_REF_CONTRACT_PAYLOAD_V1",
      supportedCorpusCodes: ["project-doc"],
      supportedStageIds: ["preparePayload"],
    },
    {
      status: "Enabled",
      strategyId: "PROJECT_DOC_RESOLVE_SOURCE",
      supportedCorpusCodes: ["project-doc"],
      supportedStageIds: ["resolveSource"],
    },
    {
      status: "Enabled",
      strategyId: "RAG_PARSE_MARKDOWN_DOCUMENT",
      supportedStageIds: ["parseDocument"],
    },
    {
      status: "Enabled",
      strategyId: "RAG_SPLIT_TEXT",
      supportedStageIds: ["splitText"],
    },
    {
      status: "Enabled",
      strategyId: "RAG_FULLTEXT_INDEX",
      supportedStageIds: ["fulltextIndex"],
    },
    {
      status: "Enabled",
      strategyId: "RAG_VECTOR_INDEX",
      supportedStageIds: ["vectorIndex"],
    },
    {
      status: "Enabled",
      strategyId: "RAG_PERSIST_SOURCE",
      supportedStageIds: ["persistSource"],
    },
  ],
  policyOptions: {
    dedupTypes: ["TEXT_UNIT", "CANONICAL_TEXT_UNIT"],
    degradePolicies: ["FAIL_FAST", "BEST_EFFORT"],
    fusionTypes: ["RRF", "SUM_SCORE"],
    topicTypes: ["NONE", "EMBEDDING_MMR_V1"],
  },
  retrievalStrategies: [
    {
      backends: ["SQLITE"],
      routeKinds: ["FULLTEXT"],
      status: "Enabled",
      strategyId: "FULLTEXT",
    },
    {
      backends: ["SQLITE"],
      routeKinds: ["FULLTEXT"],
      status: "Enabled",
      strategyId: "FULLTEXT_SQLITE_LIKE_V1",
    },
    {
      backends: ["SQLITE"],
      routeKinds: ["FULLTEXT"],
      status: "Enabled",
      strategyId: "FULLTEXT_SQLITE_FTS5_SIMPLE_V1",
    },
    {
      backends: ["SQLITE"],
      routeKinds: ["VECTOR"],
      status: "Enabled",
      strategyId: "VECTOR",
    },
    {
      backends: ["SQLITE"],
      routeKinds: ["VECTOR"],
      status: "Enabled",
      strategyId: "VECTOR_SQLITE_COSINE_V1",
    },
    {
      backends: ["SQLITE"],
      routeKinds: ["VECTOR"],
      status: "Enabled",
      strategyId: "VECTOR_SQLITE_VEC0_COSINE_V1",
    },
    {
      backends: ["GRAPH"],
      routeKinds: ["FULLTEXT", "GRAPH"],
      status: "Enabled",
      strategyId: "FULLTEXT_GRAPH_SINGLE_NODE_V1",
    },
    {
      backends: ["GRAPH"],
      routeKinds: ["VECTOR", "GRAPH"],
      status: "Enabled",
      strategyId: "VECTOR_GRAPH_SINGLE_NODE_V1",
    },
    {
      backends: ["SQLITE"],
      routeKinds: ["STRUCTURED_FILTER"],
      status: "Enabled",
      strategyId: "STRUCTURED_FILTER_SQLITE_V1",
    },
  ],
};

function inferCorpusStorageBackend(corpusCode: string, corpusOptions: CorpusRecord[]): RagStorageBackend | undefined {
  const corpus = corpusOptions.find((candidate) => candidate.code === corpusCode);
  if (!corpus) return undefined;
  return corpus.storageBinding.startsWith("graph.") ? "GRAPH" : "SQLITE";
}

export function getRagMockStrategyRegistry(): RagMockStrategyRegistry {
  return ragMockStrategyRegistry;
}

export function listRagMockIngestStageStrategies(stageId: RagIngestStageId, corpusCode?: string) {
  return ragMockStrategyRegistry.ingestStageStrategies
    .filter((strategy) => strategy.status === "Enabled")
    .filter((strategy) => strategy.supportedStageIds.includes(stageId))
    .filter((strategy) => !corpusCode || !strategy.supportedCorpusCodes || strategy.supportedCorpusCodes.includes(corpusCode))
    .map((strategy) => strategy.strategyId);
}

export function listRagMockRetrievalRouteTypes(corpusCode: string, corpusOptions: CorpusRecord[]) {
  const backend = inferCorpusStorageBackend(corpusCode, corpusOptions);
  if (!backend) return ["FULLTEXT", "VECTOR", "GRAPH", "STRUCTURED_FILTER"] satisfies RagRetrievalRouteType[];
  if (backend === "GRAPH") return ["GRAPH"] satisfies RagRetrievalRouteType[];

  return Array.from(new Set(
    ragMockStrategyRegistry.retrievalStrategies
      .filter((strategy) => strategy.status === "Enabled")
      .filter((strategy) => strategy.backends.includes(backend))
      .flatMap((strategy) => strategy.routeKinds),
  ))
    .filter((routeType) => routeType !== "GRAPH") as RagRetrievalRouteType[];
}

export function listRagMockRetrievalStrategies(
  routeType: RagRetrievalRouteType | "",
  corpusCode: string,
  corpusOptions: CorpusRecord[],
) {
  if (!routeType) return [];
  const backend = inferCorpusStorageBackend(corpusCode, corpusOptions);
  if (!backend) return [];

  return ragMockStrategyRegistry.retrievalStrategies
    .filter((strategy) => strategy.status === "Enabled")
    .filter((strategy) => strategy.backends.includes(backend))
    .filter((strategy) => backend === "GRAPH"
      ? routeType === "GRAPH" && strategy.routeKinds.includes("GRAPH")
      : routeType !== "GRAPH" && strategy.routeKinds.includes(routeType))
    .map((strategy) => strategy.strategyId);
}

export function listRagMockFusionPolicyTypes() {
  return ragMockStrategyRegistry.policyOptions.fusionTypes;
}

export function listRagMockDedupPolicyTypes() {
  return ragMockStrategyRegistry.policyOptions.dedupTypes;
}

export function listRagMockTopicPolicyTypes() {
  return ragMockStrategyRegistry.policyOptions.topicTypes;
}

export function listRagMockContextStrategies() {
  return ragMockStrategyRegistry.contextStrategies;
}

const profileStatuses = new Set<RagProfileStatus>(["Ready", "Draft", "Needs Review"]);
const defaultIngestStageOrder: RagIngestStageId[] = [
  "validateSource",
  "preparePayload",
  "resolveSource",
  "parseDocument",
  "splitText",
  "fulltextIndex",
  "vectorIndex",
  "persistSource",
];

type BuildIngestStageBinding = {
  config?: Record<string, unknown>;
  strategyId: string;
};

type BuildIngestConfigOverrides = {
  embeddingModelCode?: string;
  embeddingVersion?: string;
  graphWritePolicy?: string;
  requiredSourceVersion?: string;
  sourceInputPolicy?: {
    supportedSourceTypes: string[];
  };
  stageBindings?: Partial<Record<RagIngestStageId, BuildIngestStageBinding>>;
};

function buildIngestStageEdges(nodes: RagIngestStageId[]) {
  return nodes.slice(0, -1).map((node, index) => ({ from: node, to: nodes[index + 1] }));
}

function buildIngestConfig(
  enabledStageIds: RagIngestStageId[],
  overrides: BuildIngestConfigOverrides = {},
): RagIngestProfileConfig {
  const stages = enabledStageIds.reduce<NonNullable<RagIngestProfileConfig["stages"]>>((accumulator, stageId) => {
    const binding = overrides.stageBindings?.[stageId] ?? { strategyId: "" };
    accumulator[stageId] = binding.config ? { strategyId: binding.strategyId, config: binding.config } : { strategyId: binding.strategyId };
    return accumulator;
  }, {});

  return {
    dag: {
      edges: buildIngestStageEdges(enabledStageIds),
      nodes: enabledStageIds,
    },
    embeddingModelCode: overrides.embeddingModelCode ?? "text-embedding-3-large",
    embeddingVersion: overrides.embeddingVersion ?? "v1",
    graphWritePolicy: overrides.graphWritePolicy ?? "",
    requiredSourceVersion: overrides.requiredSourceVersion ?? "v1",
    sourceInputPolicy: overrides.sourceInputPolicy ?? { supportedSourceTypes: [] },
    stages,
  };
}

function buildRetrievalRouteConfig(corpusId: string, routeType: RagRetrievalRouteConfig["routeType"], strategyId: string, enabled = true): RagRetrievalRouteConfig {
  return {
    corpusId,
    enabled,
    routeType,
    strategyId,
  };
}

function buildRetrievalConfig(
  routesConfig: RagRetrievalRouteConfig[],
  overrides: Partial<RagRetrievalProfileConfig> = {},
): RagRetrievalProfileConfig {
  return {
    contextStrategies: overrides.contextStrategies ?? [...ragMockStrategyRegistry.contextStrategies],
    dedupPolicy: overrides.dedupPolicy ?? "TEXT_UNIT",
    filters: overrides.filters ?? [],
    fusionPolicy: overrides.fusionPolicy ?? "RRF",
    graphExpansionPolicy: overrides.graphExpansionPolicy ?? "",
    rerankEnabled: overrides.rerankEnabled ?? false,
    rerankModelCode: overrides.rerankModelCode ?? "",
    routesConfig,
    topicPolicy: overrides.topicPolicy ?? "NONE",
  };
}

function summarizeProfileCorpora(profile: ProfileRecord) {
  return profile.corpusIds.join(" + ");
}

function summarizeProfileStrategy(profile: ProfileRecord) {
  if (profile.type === "Ingest") {
    return profile.config.stages.preparePayload?.strategyId
      ?? profile.config.stages.splitText?.strategyId
      ?? profile.config.stages.parseDocument?.strategyId
      ?? profile.config.stages.persistSource?.strategyId
      ?? "";
  }

  const strategyParts = [profile.config.fusionPolicy];
  if (profile.config.contextStrategies.length > 0) {
    strategyParts.push(profile.config.contextStrategies[0]);
  }
  if (profile.config.graphExpansionPolicy.trim()) {
    strategyParts.push("GRAPH_EXPANSION");
  }
  return strategyParts.filter(Boolean).join(" + ");
}

function summarizeProfileRoutes(profile: ProfileRecord) {
  if (profile.type === "Ingest") {
    return profile.config.dag.nodes;
  }

  return profile.config.routesConfig
    .filter((route) => route.enabled)
    .map((route) => route.strategyId);
}

function withStructuredProfile(profile: ProfileRecord): ProfileRecord {
  if (profile.type === "Ingest") {
    const corpusIds = Array.from(new Set([profile.primaryCorpusId, ...profile.corpusIds].filter(Boolean)));
    return {
      ...profile,
      corpus: corpusIds.join(" + "),
      corpusIds,
      routes: summarizeProfileRoutes(profile),
      strategy: summarizeProfileStrategy(profile),
    };
  }

  const routeCorpusIds = profile.config.routesConfig.map((route) => route.corpusId).filter(Boolean);
  const corpusIds = Array.from(new Set([profile.primaryCorpusId, ...profile.corpusIds, ...routeCorpusIds].filter(Boolean)));
  return {
    ...profile,
    corpus: corpusIds.join(" + "),
    corpusIds,
    routes: summarizeProfileRoutes(profile),
    strategy: summarizeProfileStrategy(profile),
  };
}

function buildSeedIngestProfile(
  profile: Omit<IngestProfileRecord, "corpus" | "corpusIds" | "routes" | "strategy">,
): IngestProfileRecord {
  return withStructuredProfile({
    ...profile,
    corpus: profile.primaryCorpusId,
    corpusIds: [profile.primaryCorpusId],
    routes: [],
    strategy: "",
  }) as IngestProfileRecord;
}

function buildSeedRetrievalProfile(
  profile: Omit<RetrievalProfileRecord, "corpus" | "corpusIds" | "routes" | "strategy">,
): RetrievalProfileRecord {
  return withStructuredProfile({
    ...profile,
    corpus: profile.primaryCorpusId,
    corpusIds: [profile.primaryCorpusId],
    routes: [],
    strategy: "",
  }) as RetrievalProfileRecord;
}

const seedProfiles: ProfileRecord[] = [
  buildSeedIngestProfile({
    config: buildIngestConfig(["validateSource", "preparePayload", "resolveSource", "parseDocument", "splitText", "fulltextIndex", "vectorIndex", "persistSource"], {
      sourceInputPolicy: { supportedSourceTypes: ["PROJECT_DOC_REF"] },
      stageBindings: {
        fulltextIndex: { strategyId: "RAG_FULLTEXT_INDEX" },
        parseDocument: {
          strategyId: "RAG_PARSE_MARKDOWN_DOCUMENT",
          config: {
            format: "MARKDOWN",
            unsupportedFormatPolicy: "FAIL_FAST",
          },
        },
        persistSource: { strategyId: "RAG_PERSIST_SOURCE" },
        preparePayload: { strategyId: "RAG_DOCUMENT_REF_CONTRACT_PAYLOAD_V1" },
        resolveSource: {
          strategyId: "PROJECT_DOC_RESOLVE_SOURCE",
          config: {
            allowedUriSchemes: ["project-doc"],
            maxBytes: 1_048_576,
          },
        },
        splitText: {
          strategyId: "RAG_SPLIT_TEXT",
          config: {
            splitter: {
              headingDepth: 6,
              maxTokens: 800,
              overlapTokens: 80,
              type: "markdownHeading",
            },
          },
        },
        validateSource: { strategyId: "RAG_VALIDATE_SOURCE" },
        vectorIndex: { strategyId: "RAG_VECTOR_INDEX" },
      },
    }),
    description: "Parse docs, normalize frontmatter, split by sections, then index fulltext and vectors.",
    id: "project-doc-ingest-v1",
    primaryCorpusId: "project-doc",
    status: "Ready",
    type: "Ingest",
  }),
  buildSeedIngestProfile({
    config: buildIngestConfig(["validateSource", "preparePayload", "splitText", "fulltextIndex", "vectorIndex", "persistSource"], {
      sourceInputPolicy: { supportedSourceTypes: ["MEMORY_CARD"] },
      stageBindings: {
        fulltextIndex: { strategyId: "RAG_FULLTEXT_INDEX" },
        persistSource: { strategyId: "RAG_PERSIST_SOURCE" },
        preparePayload: { strategyId: "RAG_TEXT_CONTRACT_PAYLOAD_V1" },
        splitText: {
          strategyId: "RAG_SPLIT_TEXT",
          config: {
            splitter: {
              maxTokens: 800,
              overlapTokens: 80,
              type: "plainText",
            },
          },
        },
        validateSource: { strategyId: "RAG_VALIDATE_SOURCE" },
        vectorIndex: { strategyId: "RAG_VECTOR_INDEX" },
      },
    }),
    description: "Store compact memory fragments with user, agent, and workspace isolation filters.",
    id: "memory-card-ingest-v1",
    primaryCorpusId: "memory-card",
    status: "Ready",
    type: "Ingest",
  }),
  buildSeedIngestProfile({
    config: buildIngestConfig(["validateSource", "preparePayload", "splitText", "fulltextIndex", "vectorIndex", "persistSource"], {
      sourceInputPolicy: { supportedSourceTypes: ["SKILL"] },
      stageBindings: {
        fulltextIndex: { strategyId: "RAG_FULLTEXT_INDEX" },
        persistSource: { strategyId: "RAG_PERSIST_SOURCE" },
        preparePayload: { strategyId: "RAG_TEXT_CONTRACT_PAYLOAD_V1" },
        splitText: {
          strategyId: "RAG_SPLIT_TEXT",
          config: {
            splitter: {
              maxTokens: 800,
              overlapTokens: 80,
              type: "plainText",
            },
          },
        },
        validateSource: { strategyId: "RAG_VALIDATE_SOURCE" },
        vectorIndex: { strategyId: "RAG_VECTOR_INDEX" },
      },
    }),
    description: "Persist workspace skill descriptions into the embedded skill text-unit corpus.",
    id: "skill-ingest-v1",
    primaryCorpusId: "skill",
    status: "Ready",
    type: "Ingest",
  }),
  buildSeedIngestProfile({
    config: buildIngestConfig(["validateSource", "preparePayload", "splitText", "fulltextIndex", "vectorIndex", "persistSource"], {
      graphWritePolicy: "{\"maxRelatedNodes\":50,\"maxRelatedEdges\":100}",
      sourceInputPolicy: { supportedSourceTypes: ["SKILL"] },
      stageBindings: {
        fulltextIndex: { strategyId: "RAG_FULLTEXT_INDEX" },
        persistSource: { strategyId: "RAG_PERSIST_SOURCE" },
        preparePayload: { strategyId: "RAG_TEXT_CONTRACT_PAYLOAD_V1" },
        splitText: {
          strategyId: "RAG_SPLIT_TEXT",
          config: {
            splitter: {
              maxTokens: 1_200,
              overlapTokens: 0,
              type: "singleNode",
            },
          },
        },
        validateSource: { strategyId: "RAG_VALIDATE_SOURCE" },
        vectorIndex: { strategyId: "RAG_VECTOR_INDEX" },
      },
    }),
    description: "Write skill payloads into graph-backed nodes after preparing fulltext and vector artifacts.",
    id: "skill-graph-ingest-v1",
    primaryCorpusId: "skill-graph",
    status: "Needs Review",
    type: "Ingest",
  }),
  buildSeedRetrievalProfile({
    config: buildRetrievalConfig([
      buildRetrievalRouteConfig("project-doc", "FULLTEXT", "FULLTEXT_SQLITE_FTS5_SIMPLE_V1"),
      buildRetrievalRouteConfig("project-doc", "VECTOR", "VECTOR_SQLITE_VEC0_COSINE_V1"),
    ], {
      contextStrategies: ["RESULT_LIST", "DOC_QA_CONTEXT_PACK"],
      dedupPolicy: "TEXT_UNIT",
      filters: [],
      fusionPolicy: "RRF",
      rerankEnabled: false,
      topicPolicy: "NONE",
    }),
    description: "Hybrid document search with lexical recall and vector retrieval over project-doc text units.",
    id: "project-doc-related-v1",
    primaryCorpusId: "project-doc",
    status: "Ready",
    type: "Retrieval",
  }),
  buildSeedRetrievalProfile({
    config: buildRetrievalConfig([
      buildRetrievalRouteConfig("memory-card", "STRUCTURED_FILTER", "STRUCTURED_FILTER_SQLITE_V1"),
      buildRetrievalRouteConfig("memory-card", "FULLTEXT", "FULLTEXT_SQLITE_FTS5_SIMPLE_V1"),
      buildRetrievalRouteConfig("memory-card", "VECTOR", "VECTOR_SQLITE_VEC0_COSINE_V1"),
    ], {
      contextStrategies: ["RESULT_LIST", "TOKEN_BUDGET_PACK", "MEMORY_CONTEXT_PACK"],
      dedupPolicy: "TEXT_UNIT",
      filters: [
        { field: "scopeType", operator: "eq", value: "workspace" },
        { field: "scopeCode", operator: "eq", value: "TRON_CORE" },
      ],
      fusionPolicy: "RRF",
      rerankEnabled: true,
      rerankModelCode: "gpt-4.1-mini-rerank",
      topicPolicy: "NONE",
    }),
    description: "Memory-focused retrieval with structured filters, lexical recall, and vector scoring.",
    id: "memory-related-v1",
    primaryCorpusId: "memory-card",
    status: "Ready",
    type: "Retrieval",
  }),
  buildSeedRetrievalProfile({
    config: buildRetrievalConfig([
      buildRetrievalRouteConfig("skill", "FULLTEXT", "FULLTEXT_SQLITE_FTS5_SIMPLE_V1"),
      buildRetrievalRouteConfig("skill", "VECTOR", "VECTOR_SQLITE_VEC0_COSINE_V1"),
    ], {
      contextStrategies: ["RESULT_LIST", "TOKEN_BUDGET_PACK"],
      dedupPolicy: "TEXT_UNIT",
      fusionPolicy: "RRF",
      rerankEnabled: false,
      topicPolicy: "NONE",
    }),
    description: "Skill retrieval over the embedded SQLite skill corpus.",
    id: "skill-related-v1",
    primaryCorpusId: "skill",
    status: "Ready",
    type: "Retrieval",
  }),
  buildSeedRetrievalProfile({
    config: buildRetrievalConfig([
      buildRetrievalRouteConfig("skill-graph", "GRAPH", "FULLTEXT_GRAPH_SINGLE_NODE_V1"),
      buildRetrievalRouteConfig("skill-graph", "GRAPH", "VECTOR_GRAPH_SINGLE_NODE_V1"),
    ], {
      contextStrategies: ["RESULT_LIST", "TOKEN_BUDGET_PACK"],
      dedupPolicy: "TEXT_UNIT",
      fusionPolicy: "RRF",
      graphExpansionPolicy: "GRAPH_EXPANSION_POLICY",
      rerankEnabled: false,
      topicPolicy: "NONE",
    }),
    description: "Graph-backed skill retrieval with graph fulltext, vector scoring, and related-node expansion.",
    id: "skill-graph-related-v1",
    primaryCorpusId: "skill-graph",
    status: "Needs Review",
    type: "Retrieval",
  }),
  buildSeedRetrievalProfile({
    config: buildRetrievalConfig([
      buildRetrievalRouteConfig("memory-card", "FULLTEXT", "FULLTEXT_SQLITE_FTS5_SIMPLE_V1"),
      buildRetrievalRouteConfig("memory-card", "VECTOR", "VECTOR_SQLITE_VEC0_COSINE_V1"),
      buildRetrievalRouteConfig("project-doc", "FULLTEXT", "FULLTEXT_SQLITE_FTS5_SIMPLE_V1"),
      buildRetrievalRouteConfig("project-doc", "VECTOR", "VECTOR_SQLITE_VEC0_COSINE_V1"),
    ], {
      contextStrategies: ["RESULT_LIST", "TOKEN_BUDGET_PACK", "MEMORY_CONTEXT_PACK", "DOC_QA_CONTEXT_PACK"],
      dedupPolicy: "TEXT_UNIT",
      fusionPolicy: "RRF",
      rerankEnabled: true,
      rerankModelCode: "gpt-4.1-mini-rerank",
      topicPolicy: "EMBEDDING_MMR_V1",
    }),
    description: "Cross-corpus fusion profile for memory and project documents with RRF and topic diversity.",
    id: "memory-project-doc-fusion-v1",
    primaryCorpusId: "memory-card",
    status: "Ready",
    type: "Retrieval",
  }),
  buildSeedRetrievalProfile({
    config: buildRetrievalConfig([
      buildRetrievalRouteConfig("skill-graph", "GRAPH", "FULLTEXT_GRAPH_SINGLE_NODE_V1"),
      buildRetrievalRouteConfig("skill-graph", "GRAPH", "VECTOR_GRAPH_SINGLE_NODE_V1"),
      buildRetrievalRouteConfig("memory-card", "FULLTEXT", "FULLTEXT_SQLITE_FTS5_SIMPLE_V1"),
      buildRetrievalRouteConfig("memory-card", "VECTOR", "VECTOR_SQLITE_VEC0_COSINE_V1"),
      buildRetrievalRouteConfig("project-doc", "FULLTEXT", "FULLTEXT_SQLITE_FTS5_SIMPLE_V1"),
      buildRetrievalRouteConfig("project-doc", "VECTOR", "VECTOR_SQLITE_VEC0_COSINE_V1"),
    ], {
      contextStrategies: ["RESULT_LIST", "TOKEN_BUDGET_PACK", "MEMORY_CONTEXT_PACK", "DOC_QA_CONTEXT_PACK"],
      dedupPolicy: "TEXT_UNIT",
      fusionPolicy: "RRF",
      graphExpansionPolicy: "GRAPH_EXPANSION_POLICY",
      rerankEnabled: true,
      rerankModelCode: "gpt-4.1-mini-rerank",
      topicPolicy: "EMBEDDING_MMR_V1",
    }),
    description: "Three-way retrieval profile that fuses graph, memory, and project document corpora.",
    id: "skill-memory-project-fusion-v1",
    primaryCorpusId: "skill-graph",
    status: "Needs Review",
    type: "Retrieval",
  }),
  buildSeedRetrievalProfile({
    config: buildRetrievalConfig([
      buildRetrievalRouteConfig("code-graph", "GRAPH", "FULLTEXT_GRAPH_SINGLE_NODE_V1"),
    ], {
      contextStrategies: ["RESULT_LIST"],
      dedupPolicy: "TEXT_UNIT",
      fusionPolicy: "SUM_SCORE",
      graphExpansionPolicy: "SUMMARY_TREE",
      rerankEnabled: false,
      topicPolicy: "NONE",
    }),
    description: "Reserved profile placeholder until code graph semantic projection is implemented.",
    id: "code-graph-reserved-v1",
    primaryCorpusId: "code-graph",
    status: "Draft",
    type: "Retrieval",
  }),
];

function isRetrievalFilter(value: unknown): value is RagRetrievalProfileFilter {
  if (!value || typeof value !== "object") return false;
  const filter = value as Partial<RagRetrievalProfileFilter>;
  return typeof filter.field === "string"
    && typeof filter.operator === "string"
    && typeof filter.value === "string";
}

function isRetrievalRouteConfig(value: unknown): value is RagRetrievalRouteConfig {
  if (!value || typeof value !== "object") return false;
  const route = value as Partial<RagRetrievalRouteConfig>;
  return typeof route.corpusId === "string"
    && typeof route.routeType === "string"
    && typeof route.strategyId === "string"
    && typeof route.enabled === "boolean";
}

function isIngestStageBinding(value: unknown) {
  if (!value || typeof value !== "object") return false;
  const binding = value as { strategyId?: unknown };
  return typeof binding.strategyId === "string";
}

function isIngestProfileRecord(value: unknown): value is IngestProfileRecord {
  if (!value || typeof value !== "object") return false;
  const record = value as Partial<IngestProfileRecord>;
  return record.type === "Ingest"
    && typeof record.id === "string"
    && typeof record.description === "string"
    && typeof record.primaryCorpusId === "string"
    && typeof record.status === "string"
    && profileStatuses.has(record.status)
    && Array.isArray(record.corpusIds)
    && record.config !== undefined
    && typeof record.config === "object"
    && record.config.dag !== undefined
    && Array.isArray(record.config.dag.nodes)
    && Array.isArray(record.config.dag.edges)
    && record.config.stages !== undefined
    && typeof record.config.stages === "object"
    && Object.values(record.config.stages).every(isIngestStageBinding)
    && record.config.sourceInputPolicy !== undefined
    && typeof record.config.sourceInputPolicy === "object"
    && Array.isArray(record.config.sourceInputPolicy.supportedSourceTypes);
}

function isRetrievalProfileRecord(value: unknown): value is RetrievalProfileRecord {
  if (!value || typeof value !== "object") return false;
  const record = value as Partial<RetrievalProfileRecord>;
  return record.type === "Retrieval"
    && typeof record.id === "string"
    && typeof record.description === "string"
    && typeof record.primaryCorpusId === "string"
    && typeof record.status === "string"
    && profileStatuses.has(record.status)
    && Array.isArray(record.corpusIds)
    && record.config !== undefined
    && typeof record.config === "object"
    && Array.isArray(record.config.contextStrategies)
    && record.config.contextStrategies.every((strategy) => typeof strategy === "string")
    && Array.isArray(record.config.routesConfig)
    && record.config.routesConfig.every(isRetrievalRouteConfig)
    && Array.isArray(record.config.filters)
    && record.config.filters.every(isRetrievalFilter);
}

function isProfileRecord(value: unknown): value is ProfileRecord {
  return isIngestProfileRecord(value) || isRetrievalProfileRecord(value);
}

type LegacyIngestProfileRecord = Omit<IngestProfileRecord, "config"> & {
  config: {
    documentFormat?: string;
    embeddingModelCode?: string;
    embeddingVersion?: string;
    graphWritePolicy?: string;
    payloadStrategy?: string;
    pipelineStages?: Array<{ enabled?: boolean; id?: string }>;
    primaryStrategy?: string;
    requiredSourceVersion?: string;
    sourceResolverMaxBytes?: string;
    supportedSourceTypes?: string[];
  };
};

type LegacyRetrievalProfileRecord = Omit<RetrievalProfileRecord, "config"> & {
  config: Partial<RagRetrievalProfileConfig>;
};

const legacyStageIdMap: Partial<Record<string, RagIngestStageId>> = {
  fulltextIndex: "fulltextIndex",
  extractTree: "extractTree",
  parse: "parseDocument",
  persistTree: "persistTree",
  persistSource: "persistSource",
  preparePayload: "preparePayload",
  resolveSource: "resolveSource",
  split: "splitText",
  validateSource: "validateSource",
  vectorIndex: "vectorIndex",
};

function normalizeStoredIngestProfile(value: unknown): IngestProfileRecord | undefined {
  if (isIngestProfileRecord(value)) return withStructuredProfile(value) as IngestProfileRecord;
  if (!value || typeof value !== "object") return undefined;

  const record = value as Partial<LegacyIngestProfileRecord>;
  if (record.type !== "Ingest" || typeof record.id !== "string" || typeof record.primaryCorpusId !== "string" || !record.config || typeof record.config !== "object") {
    return undefined;
  }
  const legacyConfig = record.config;

  const enabledStageIds = Array.isArray(legacyConfig.pipelineStages)
    ? legacyConfig.pipelineStages
      .filter((stage) => stage.enabled)
      .map((stage) => (stage.id ? legacyStageIdMap[stage.id] : undefined))
      .filter((stageId): stageId is RagIngestStageId => Boolean(stageId))
    : [];

  const stages = enabledStageIds.reduce<NonNullable<RagIngestProfileConfig["stages"]>>((accumulator, stageId) => {
    const defaultStrategyId = ({
      extractTree: "RAG_MARKDOWN_TREE_EXTRACT_V1",
      fulltextIndex: "RAG_FULLTEXT_INDEX",
      parseDocument: "RAG_PARSE_MARKDOWN_DOCUMENT",
      persistTree: "RAG_PERSIST_TREE_GRAPH_V1",
      persistSource: "RAG_PERSIST_SOURCE",
      preparePayload: legacyConfig.payloadStrategy?.trim() || "RAG_TEXT_CONTRACT_PAYLOAD_V1",
      resolveSource: "PROJECT_DOC_RESOLVE_SOURCE",
      splitText: legacyConfig.primaryStrategy?.trim() || "RAG_SPLIT_TEXT",
      validateSource: "RAG_VALIDATE_SOURCE",
      vectorIndex: "RAG_VECTOR_INDEX",
    } satisfies Record<RagIngestStageId, string>)[stageId];

    if (stageId === "resolveSource") {
      accumulator[stageId] = {
        config: {
          maxBytes: Number.parseInt(legacyConfig.sourceResolverMaxBytes ?? "1048576", 10) || 1_048_576,
        },
        strategyId: defaultStrategyId,
      };
      return accumulator;
    }

    if (stageId === "parseDocument") {
      accumulator[stageId] = {
        config: {
          format: (legacyConfig.documentFormat ?? "MARKDOWN").toUpperCase(),
          unsupportedFormatPolicy: "FAIL_FAST",
        },
        strategyId: defaultStrategyId,
      };
      return accumulator;
    }

    accumulator[stageId] = { strategyId: defaultStrategyId };
    return accumulator;
  }, {});

  return withStructuredProfile({
    config: {
      dag: {
        edges: buildIngestStageEdges(enabledStageIds),
        nodes: enabledStageIds,
      },
      embeddingModelCode: legacyConfig.embeddingModelCode ?? "",
      embeddingVersion: legacyConfig.embeddingVersion ?? "v1",
      graphWritePolicy: legacyConfig.graphWritePolicy ?? "",
      requiredSourceVersion: legacyConfig.requiredSourceVersion ?? "v1",
      sourceInputPolicy: {
        supportedSourceTypes: legacyConfig.supportedSourceTypes ?? [],
      },
      stages,
    },
    corpus: record.primaryCorpusId,
    corpusIds: Array.isArray(record.corpusIds) ? record.corpusIds.filter((corpusId): corpusId is string => typeof corpusId === "string") : [record.primaryCorpusId],
    description: typeof record.description === "string" ? record.description : "",
    id: record.id,
    primaryCorpusId: record.primaryCorpusId,
    routes: [],
    status: profileStatuses.has(record.status as RagProfileStatus) ? record.status as RagProfileStatus : "Draft",
    strategy: "",
    type: "Ingest",
  }) as IngestProfileRecord;
}

function normalizeLegacyRetrievalDedupPolicy(policy: string | undefined) {
  if (!policy) return "TEXT_UNIT";
  if (policy === "KEEP_HIGHEST_SCORE") return "TEXT_UNIT";
  if (policy === "TEXT_UNIT" || policy === "CANONICAL_TEXT_UNIT") return policy;
  return "TEXT_UNIT";
}

function normalizeLegacyRetrievalFusionPolicy(policy: string | undefined) {
  if (!policy) return "SUM_SCORE";
  if (policy === "RRF" || policy === "SUM_SCORE") return policy;
  if (policy === "RESULT_LIST" || policy === "DOC_QA_CONTEXT_PACK" || policy === "MEMORY_CONTEXT_PACK") return "SUM_SCORE";
  return "SUM_SCORE";
}

function normalizeLegacyRetrievalTopicPolicy(policy: string | undefined) {
  if (!policy || policy === "NONE") return "NONE";
  if (policy === "DIVERSITY") return "EMBEDDING_MMR_V1";
  if (policy === "EMBEDDING_MMR_V1") return policy;
  return "NONE";
}

function normalizeLegacyContextStrategies(
  contextStrategies: string[] | undefined,
  legacyFusionPolicy?: string,
) {
  const supportedStrategies = new Set(ragMockStrategyRegistry.contextStrategies);
  const filteredStrategies = (contextStrategies ?? []).filter((strategy) => supportedStrategies.has(strategy));
  if (filteredStrategies.length > 0) {
    return Array.from(new Set(filteredStrategies));
  }

  if (legacyFusionPolicy && supportedStrategies.has(legacyFusionPolicy)) {
    return [legacyFusionPolicy];
  }

  return [...ragMockStrategyRegistry.contextStrategies];
}

function normalizeStoredRetrievalProfile(value: unknown): RetrievalProfileRecord | undefined {
  if (isRetrievalProfileRecord(value)) {
    return withStructuredProfile({
      ...value,
      config: {
        ...value.config,
        contextStrategies: normalizeLegacyContextStrategies(value.config.contextStrategies, value.config.fusionPolicy),
        dedupPolicy: normalizeLegacyRetrievalDedupPolicy(value.config.dedupPolicy),
        fusionPolicy: normalizeLegacyRetrievalFusionPolicy(value.config.fusionPolicy),
        topicPolicy: normalizeLegacyRetrievalTopicPolicy(value.config.topicPolicy),
      },
    }) as RetrievalProfileRecord;
  }

  if (!value || typeof value !== "object") return undefined;
  const record = value as Partial<LegacyRetrievalProfileRecord>;
  if (record.type !== "Retrieval" || typeof record.id !== "string" || typeof record.primaryCorpusId !== "string" || !record.config || typeof record.config !== "object") {
    return undefined;
  }

  const routesConfig = Array.isArray(record.config.routesConfig)
    ? record.config.routesConfig.filter(isRetrievalRouteConfig)
    : [];
  const filters = Array.isArray(record.config.filters)
    ? record.config.filters.filter(isRetrievalFilter)
    : [];

  return withStructuredProfile({
    config: {
      contextStrategies: normalizeLegacyContextStrategies(record.config.contextStrategies, record.config.fusionPolicy),
      dedupPolicy: normalizeLegacyRetrievalDedupPolicy(record.config.dedupPolicy),
      filters,
      fusionPolicy: normalizeLegacyRetrievalFusionPolicy(record.config.fusionPolicy),
      graphExpansionPolicy: typeof record.config.graphExpansionPolicy === "string" ? record.config.graphExpansionPolicy : "",
      rerankEnabled: typeof record.config.rerankEnabled === "boolean" ? record.config.rerankEnabled : false,
      rerankModelCode: typeof record.config.rerankModelCode === "string" ? record.config.rerankModelCode : "",
      routesConfig,
      topicPolicy: normalizeLegacyRetrievalTopicPolicy(record.config.topicPolicy),
    },
    corpus: record.primaryCorpusId,
    corpusIds: Array.isArray(record.corpusIds) ? record.corpusIds.filter((corpusId): corpusId is string => typeof corpusId === "string") : [record.primaryCorpusId],
    description: typeof record.description === "string" ? record.description : "",
    id: record.id,
    primaryCorpusId: record.primaryCorpusId,
    routes: [],
    status: profileStatuses.has(record.status as RagProfileStatus) ? record.status as RagProfileStatus : "Draft",
    strategy: "",
    type: "Retrieval",
  }) as RetrievalProfileRecord;
}

function readStoredRagProfiles(): ProfileRecord[] {
  if (!canUseBrowserStorage()) return [];

  try {
    const rawValue = window.localStorage.getItem(RAG_MOCK_PROFILES_STORAGE_KEY);
    if (!rawValue) return [];
    const parsedValue: unknown = JSON.parse(rawValue);
    if (!Array.isArray(parsedValue)) return [];
    return parsedValue.flatMap<ProfileRecord>((value) => {
      const normalizedIngest = normalizeStoredIngestProfile(value);
      if (normalizedIngest) return [normalizedIngest];
      const normalizedRetrieval = normalizeStoredRetrievalProfile(value);
      return normalizedRetrieval ? [normalizedRetrieval] : [];
    });
  } catch {
    return [];
  }
}

export function getRagMockProfiles(): ProfileRecord[] {
  const storedProfiles = readStoredRagProfiles().map(withStructuredProfile);
  const storedById = new Map(storedProfiles.map((profile) => [profile.id, profile]));
  const seedIds = new Set(seedProfiles.map((profile) => profile.id));
  const mergedSeedProfiles = seedProfiles.map((profile) => storedById.get(profile.id) ?? profile);
  const createdProfiles = storedProfiles.filter((profile) => !seedIds.has(profile.id));
  return [...mergedSeedProfiles, ...createdProfiles];
}

export function findRagMockProfile(profileId: string) {
  return getRagMockProfiles().find((profile) => profile.id === profileId);
}

export function saveRagMockProfile(profile: ProfileRecord) {
  const structuredProfile = withStructuredProfile(profile);
  if (!canUseBrowserStorage()) return structuredProfile;

  const storedProfiles = readStoredRagProfiles();
  const existingIndex = storedProfiles.findIndex((storedProfile) => storedProfile.id === structuredProfile.id);
  const nextProfiles = existingIndex >= 0
    ? storedProfiles.map((storedProfile, index) => (index === existingIndex ? structuredProfile : storedProfile))
    : [...storedProfiles, structuredProfile];

  window.localStorage.setItem(RAG_MOCK_PROFILES_STORAGE_KEY, JSON.stringify(nextProfiles));
  return structuredProfile;
}

export const sources: SourceRecord[] = [
  {
    code: "docs/features/rag-search-context-pack",
    corpus: "project-doc",
    indexedAt: "8 min ago",
    owner: "TRON_CORE",
    sourceType: "feature-spec",
    status: "Active",
    textUnits: 348,
    version: "2026-06-04",
    warnings: 0,
  },
  {
    code: "memory/session-8842-alpha",
    corpus: "memory-card",
    indexedAt: "3 min ago",
    owner: "main-agent",
    sourceType: "memory-fragment",
    status: "Active",
    textUnits: 92,
    version: "v12",
    warnings: 1,
  },
  {
    code: "skills/product-design",
    corpus: "skill-graph",
    indexedAt: "22 min ago",
    owner: "TRON_CORE",
    sourceType: "skill",
    status: "Indexing",
    textUnits: 128,
    version: "v1",
    warnings: 3,
  },
  {
    code: "docs/features/rag-ingest-dag",
    corpus: "project-doc",
    indexedAt: "17 min ago",
    owner: "TRON_CORE",
    sourceType: "feature-spec",
    status: "Warning",
    textUnits: 283,
    version: "2026-06-03",
    warnings: 2,
  },
];

export const textUnits: TextUnitRecord[] = [
  {
    corpus: "project-doc",
    embeddingStatus: "Indexed",
    id: "tu-project-doc-1042",
    preview: "RAG search returns hits, context packs, warnings, and a trace code when tracing is enabled.",
    source: "docs/features/rag-search-context-pack",
    tokens: 186,
  },
  {
    corpus: "memory-card",
    embeddingStatus: "Indexed",
    id: "tu-memory-8821",
    preview: "Workspace retrieval must honor userCode, agentCode, and workspaceCode access filters.",
    source: "memory/session-8842-alpha",
    tokens: 92,
  },
  {
    corpus: "skill-graph",
    embeddingStatus: "Pending",
    id: "tu-skill-239",
    preview: "Use get-context before ideation and produce exactly three visual options.",
    source: "skills/product-design",
    tokens: 128,
  },
  {
    corpus: "project-doc",
    embeddingStatus: "Skipped",
    id: "tu-project-doc-1130",
    preview: "A warning is attached when payload metadata cannot be normalized into the filter schema.",
    source: "docs/features/rag-ingest-dag",
    tokens: 54,
  },
];

export const warnings: WarningRecord[] = [
  {
    age: "17 min",
    corpus: "project-doc",
    message: "Source metadata missed optional language, defaulted to en-US.",
    severity: "Low",
    source: "docs/features/rag-ingest-dag",
    stage: "normalize",
  },
  {
    age: "22 min",
    corpus: "skill-graph",
    message: "Vector index is still backfilling 19 percent of skill text units.",
    severity: "Medium",
    source: "skills/product-design",
    stage: "vectorIndex",
  },
  {
    age: "44 min",
    corpus: "memory-card",
    message: "One memory source produced an empty split and was skipped.",
    severity: "Medium",
    source: "memory/session-8842-alpha",
    stage: "split",
  },
];

export const traces: TraceRecord[] = [
  {
    age: "2 min",
    hits: 8,
    id: "trace-rag-9128",
    profile: "project-doc-related-v1",
    query: "RAG context pack token budget",
    route: "FULLTEXT + VECTOR",
    tokens: 3880,
  },
  {
    age: "9 min",
    hits: 5,
    id: "trace-rag-9084",
    profile: "memory-related-v1",
    query: "workspace auth mock failure",
    route: "VECTOR",
    tokens: 1410,
  },
  {
    age: "31 min",
    hits: 12,
    id: "trace-rag-9011",
    profile: "skill-graph-related-v1",
    query: "product design prototype flow",
    route: "GRAPH",
    tokens: 2250,
  },
];
