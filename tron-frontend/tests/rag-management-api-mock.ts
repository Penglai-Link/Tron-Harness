import type {
  Page,
  Request,
  Route,
} from "@playwright/test";

export type CapturedRequest = {
  body: unknown;
  method: string;
  pathname: string;
  query: string;
  responseData?: unknown;
};

type RagStorageTargetApi =
  | {
      backend: "GRAPH";
      codeStrategy?: string;
      id: string;
      layout: "GRAPH_SINGLE_NODE" | "GRAPH_TREE";
      namespace?: string;
      nodeLabel?: string;
    }
  | {
      backend: "SQLITE";
      id: string;
      layout: "TABLE";
      table?: string;
    };

type RagStorageBindingApi = {
  bindings: Partial<Record<"diagnostics" | "source" | "textUnit" | "fulltext" | "vector" | "tree", string>>;
  mode: "TARGETED";
  targets: RagStorageTargetApi[];
};

type RagCorpusDetailApi = {
  capabilities: {
    fulltext: boolean;
    graph: boolean;
    reserved: boolean;
    tree: boolean;
    vector: boolean;
  };
  corpusId: string;
  description: string;
  displayName: string;
  fieldMapping: {
    source: Record<string, string>;
    textUnit: Record<string, string>;
  };
  filterSchema: {
    fields: Record<string, {
      column: string;
      operators?: string[];
      type: "boolean" | "integer" | "string";
    }>;
  };
  ingestProfileId: string;
  metadataGroups: Array<Record<string, unknown>>;
  retrievalProfileId: string;
  runtimeProjection: {
    compatible: boolean;
    issues: string[];
    storageBinding: RagStorageBindingApi;
  };
  status: string;
  storageBinding: RagStorageBindingApi;
  storageSummary: string;
};

type RagPipelineStageApi = {
  config?: Record<string, unknown>;
  enabled: boolean;
  id: string;
  strategyId: string;
};

type RagPipelineNodeApi = {
  config?: Record<string, unknown>;
  enabled?: boolean;
  failurePolicy?: string;
  joinPolicy?: string;
  nodeId: string;
  stageType: string;
  strategyId: string;
};

type RagPipelineEdgeApi = {
  from: string;
  to: string;
};

type RagPipelineApi = {
  edges: RagPipelineEdgeApi[];
  mode: "DAG";
  nodes: RagPipelineNodeApi[];
};

type RagPipelineCatalogApi = {
  failurePolicies: string[];
  joinPolicies: string[];
  profileType: "INGEST" | "RETRIEVAL";
  stages: Array<{
    allowedStrategies: string[];
    defaultFailurePolicy: string;
    defaultJoinPolicy: string;
    displayName: string;
    inputArtifactKeys: string[];
    outputArtifactKeys: string[];
    profileTypes: Array<"INGEST" | "RETRIEVAL">;
    repeatable: boolean;
    stageType: string;
    strategies: Array<{
      configSchema: Record<string, unknown>;
      displayName: string;
      strategyId: string;
    }>;
  }>;
};

type RagRouteApi = {
  corpusId: string;
  enabled: boolean;
  routeId: string;
  routeType: string;
  strategyId: string;
};

type RagFilterApi = {
  field: string;
  operator: string;
  value: string;
};

type RagProfileDetailApi = {
  contextStrategies?: string[];
  corpusIds: string[];
  dedupPolicy?: string;
  degradePolicy?: string;
  description: string;
  embeddingModelCode?: string;
  embeddingVersion?: string;
  filters?: RagFilterApi[];
  fusionPolicy?: string;
  graphExpansionPolicy?: string;
  graphWritePolicy?: string;
  pipeline?: RagPipelineApi;
  pipelineStages?: RagPipelineStageApi[];
  primaryCorpusId: string;
  profileId: string;
  profileType: "INGEST" | "RETRIEVAL";
  rerankEnabled?: boolean;
  rerankModelCode?: string;
  requiredSourceVersion: string;
  routes?: RagRouteApi[];
  sourceInputPolicy?: {
    supportedSourceTypes?: string[];
  };
  status: string;
  topicPolicy?: string;
};

type RagCatalogCorpusApi = {
  capabilities: RagCorpusDetailApi["capabilities"];
  corpusId: string;
  displayName: string;
  filterSchema: RagCorpusDetailApi["filterSchema"];
  status: string;
  storageSummary: string;
};

type RagProfileCatalogApi = {
  contextStrategies: string[];
  corpora: RagCatalogCorpusApi[];
  dedupPolicyTypes: string[];
  fusionPolicyTypes: string[];
  graphExpansionPolicies: string[];
  ingestStageDefinitions: string[];
  ingestStageStrategies: Record<string, string[]>;
  retrievalRouteTypes: Array<{ corpusId: string; routeTypes: string[] }>;
  retrievalStrategies: Array<{ corpusId: string; routeType: string; strategyIds: string[] }>;
  topicPolicyTypes: string[];
};

type RagValidationErrorApi = {
  code?: string;
  field?: string;
  message: string;
};

type RagManagementState = {
  corpora: Map<string, RagCorpusDetailApi>;
  profiles: Map<string, RagProfileDetailApi>;
  profileCatalog: RagProfileCatalogApi;
};

type RagRuntimeAccessContextApi = {
  agentCode?: string;
  userCode?: string;
  workspaceCode?: string;
};

type RagRuntimeScopeApi = {
  scopeCode?: string;
  scopeType?: string;
};

function makeSqliteTarget(id: string, table: string): RagStorageTargetApi {
  return {
    backend: "SQLITE",
    id,
    layout: "TABLE",
    table,
  };
}

function makeGraphTarget(id: string, namespace: string, nodeLabel: string, layout: "GRAPH_SINGLE_NODE" | "GRAPH_TREE" = "GRAPH_SINGLE_NODE"): RagStorageTargetApi {
  return {
    backend: "GRAPH",
    codeStrategy: "CORPUS_SOURCE_VERSION_SCOPE_HASH",
    id,
    layout,
    namespace,
    nodeLabel,
  };
}

function formatStorageTarget(target: RagStorageTargetApi | undefined) {
  if (!target) return "unbound";
  if (target.backend === "GRAPH") {
    return `graph.${target.namespace ?? target.nodeLabel ?? "unbound"}/${target.nodeLabel ?? "node"}`;
  }
  return `sqlite.${target.table ?? "unbound"}`;
}

function formatStorageSummary(binding: RagStorageBindingApi) {
  const targetMap = new Map(binding.targets.map((target) => [target.id, target]));
  return ["source", "textUnit", "fulltext", "vector", "tree"]
    .map((segmentKey) => {
      const targetId = segmentKey === "fulltext"
        ? (binding.bindings.fulltext ?? binding.bindings.textUnit)
        : binding.bindings[segmentKey as keyof typeof binding.bindings];
      return `${segmentKey}=${formatStorageTarget(targetMap.get(targetId ?? ""))}`;
    })
    .join(" | ");
}

function createProjectDocCorpus(): RagCorpusDetailApi {
  const storageBinding: RagStorageBindingApi = {
    bindings: {
      diagnostics: "project-doc-diagnostics",
      source: "source-store",
      tree: "tree-store",
      textUnit: "content-store",
      vector: "content-store",
    },
    mode: "TARGETED",
    targets: [
      makeSqliteTarget("source-store", "rag_project_doc_sources"),
      makeSqliteTarget("content-store", "rag_project_doc_text_units"),
      makeSqliteTarget("tree-store", "rag_project_doc_tree_nodes"),
      makeSqliteTarget("project-doc-diagnostics", "rag_project_doc_diagnostics"),
    ],
  };

  return {
    capabilities: {
      fulltext: true,
      graph: false,
      reserved: false,
      tree: true,
      vector: true,
    },
    corpusId: "project-doc",
    description: "Project documentation sources and text units.",
    displayName: "Project Document",
    fieldMapping: {
      source: {
        documentCode: "document_code",
        metadata: "metadata",
        repoCode: "repo_code",
        scopeCode: "scope_code",
        scopeType: "scope_type",
        sourceCode: "source_code",
        sourceType: "source_type",
        sourceVersion: "source_version",
        status: "status",
        title: "title",
      },
      textUnit: {
        documentCode: "document_code",
        scopeCode: "scope_code",
        scopeType: "scope_type",
        sourceCode: "source_code",
        sourceVersion: "source_version",
        text: "text",
        textUnitStatus: "status",
      },
    },
    filterSchema: {
      fields: {
        documentCode: { column: "document_code", operators: ["eq", "in"], type: "string" },
        repoCode: { column: "repo_code", operators: ["eq", "in"], type: "string" },
        scopeCode: { column: "scope_code", operators: ["eq", "in"], type: "string" },
        scopeType: { column: "scope_type", operators: ["eq", "in"], type: "string" },
      },
    },
    ingestProfileId: "project-doc-ingest-v1",
    metadataGroups: [{ groupId: "provenance", labels: ["scopeType", "scopeCode"] }],
    retrievalProfileId: "project-doc-related-v1",
    runtimeProjection: {
      compatible: true,
      issues: [],
      storageBinding,
    },
    status: "enabled",
    storageBinding,
    storageSummary: formatStorageSummary(storageBinding),
  };
}

function createMemoryCardCorpus(): RagCorpusDetailApi {
  const storageBinding: RagStorageBindingApi = {
    bindings: {
      diagnostics: "memory-card-diagnostics",
      fulltext: "content-store",
      source: "content-store",
      tree: "content-store",
      textUnit: "content-store",
      vector: "content-store",
    },
    mode: "TARGETED",
    targets: [
      makeSqliteTarget("content-store", "rag_memory_text_units"),
      makeSqliteTarget("memory-card-diagnostics", "rag_memory_card_diagnostics"),
    ],
  };

  return {
    capabilities: {
      fulltext: true,
      graph: false,
      reserved: false,
      tree: true,
      vector: true,
    },
    corpusId: "memory-card",
    description: "User, agent, and workspace scoped memory cards.",
    displayName: "Memory Card",
    fieldMapping: {
      source: {
        importanceScore: "importance_score",
        metadata: "source_metadata",
        scopeCode: "scope_code",
        scopeType: "scope_type",
        sourceCode: "source_code",
        sourceType: "source_type",
        sourceVersion: "source_version",
        status: "source_status",
        title: "title",
      },
      textUnit: {
        scopeCode: "scope_code",
        scopeType: "scope_type",
        sourceCode: "source_code",
        sourceVersion: "source_version",
        text: "text",
        textUnitStatus: "status",
      },
    },
    filterSchema: {
      fields: {
        memoryType: { column: "memory_type", operators: ["eq", "in"], type: "string" },
        scopeCode: { column: "scope_code", operators: ["eq", "in"], type: "string" },
        scopeType: { column: "scope_type", operators: ["eq", "in"], type: "string" },
      },
    },
    ingestProfileId: "memory-card-ingest-v1",
    metadataGroups: [{ groupId: "governance", labels: ["memoryType"] }],
    retrievalProfileId: "memory-related-v1",
    runtimeProjection: {
      compatible: true,
      issues: [],
      storageBinding,
    },
    status: "enabled",
    storageBinding,
    storageSummary: formatStorageSummary(storageBinding),
  };
}

function createSupportTreeGraphCorpus(): RagCorpusDetailApi {
  const storageBinding: RagStorageBindingApi = {
    bindings: {
      diagnostics: "support-tree-diagnostics",
      source: "tree-graph-store",
      tree: "tree-graph-store",
      textUnit: "tree-graph-store",
    },
    mode: "TARGETED",
    targets: [
      makeGraphTarget("tree-graph-store", "rag_support_tree", "RagSupportTreeNode", "GRAPH_TREE"),
      makeGraphTarget("support-tree-diagnostics", "rag_support_tree", "RagSupportTreeDiagnostics"),
    ],
  };

  return {
    capabilities: {
      fulltext: true,
      graph: true,
      reserved: false,
      tree: true,
      vector: true,
    },
    corpusId: "support-tree-graph",
    description: "Support knowledge summary tree backed by graph storage.",
    displayName: "Support Graph Tree",
    fieldMapping: {
      source: {
        scopeCode: "scopeCode",
        scopeType: "scopeType",
        sourceCode: "sourceCode",
        sourceType: "sourceType",
        sourceVersion: "sourceVersion",
        status: "status",
        title: "title",
      },
      textUnit: {
        scopeCode: "scopeCode",
        scopeType: "scopeType",
        sourceCode: "sourceCode",
        sourceVersion: "sourceVersion",
        text: "text",
      },
    },
    filterSchema: {
      fields: {
        scopeCode: { column: "scopeCode", operators: ["eq", "in"], type: "string" },
        scopeType: { column: "scopeType", operators: ["eq", "in"], type: "string" },
      },
    },
    ingestProfileId: "",
    metadataGroups: [{ groupId: "tree", labels: ["scopeCode", "scopeType"] }],
    retrievalProfileId: "",
    runtimeProjection: {
      compatible: true,
      issues: [],
      storageBinding,
    },
    status: "enabled",
    storageBinding,
    storageSummary: formatStorageSummary(storageBinding),
  };
}

function createSeedProfileCatalog(corpora: RagCatalogCorpusApi[]): RagProfileCatalogApi {
  return {
    contextStrategies: ["RESULT_LIST", "TOKEN_BUDGET_PACK", "MEMORY_CONTEXT_PACK", "DOC_QA_CONTEXT_PACK"],
    corpora,
    dedupPolicyTypes: ["TEXT_UNIT", "CANONICAL_TEXT_UNIT"],
    fusionPolicyTypes: ["RRF", "SUM_SCORE"],
    graphExpansionPolicies: ["ONE_HOP", "TWO_HOP"],
    ingestStageDefinitions: [
      "validateSource",
      "preparePayload",
      "resolveSource",
      "parseDocument",
      "splitText",
      "extractTree",
      "summarizeTree",
      "fulltextIndex",
      "vectorIndex",
      "treeFulltextIndex",
      "treeVectorIndex",
      "persistTree",
      "persistSource",
    ],
    ingestStageStrategies: {
      extractTree: ["RAG_MARKDOWN_TREE_EXTRACT_V1"],
      fulltextIndex: ["RAG_FULLTEXT_INDEX"],
      parseDocument: ["RAG_PARSE_MARKDOWN_DOCUMENT"],
      persistTree: ["RAG_PERSIST_TREE_GRAPH_V1"],
      persistSource: ["RAG_PERSIST_SOURCE"],
      preparePayload: ["RAG_TEXT_CONTRACT_PAYLOAD_V1", "RAG_DOCUMENT_REF_CONTRACT_PAYLOAD_V1"],
      resolveSource: ["PROJECT_DOC_RESOLVE_SOURCE"],
      splitText: ["RAG_SPLIT_PLAIN_TEXT_V1", "RAG_SPLIT_MARKDOWN_HEADING_V1", "RAG_SPLIT_TEXT"],
      summarizeTree: ["RAG_TREE_EXTRACTIVE_SUMMARY_V1"],
      treeFulltextIndex: ["RAG_TREE_FULLTEXT_INDEX"],
      treeVectorIndex: ["RAG_TREE_VECTOR_INDEX"],
      validateSource: ["RAG_VALIDATE_SOURCE"],
      vectorIndex: ["RAG_VECTOR_INDEX"],
    },
    retrievalRouteTypes: corpora.map((corpus) => ({
      corpusId: corpus.corpusId,
      routeTypes: corpus.corpusId === "support-tree-graph"
        ? ["TREE_FULLTEXT", "TREE_VECTOR"]
        : ["FULLTEXT", "VECTOR", "TREE_FULLTEXT", "TREE_VECTOR", "STRUCTURED_FILTER"],
    })),
    retrievalStrategies: corpora.flatMap((corpus) => {
      if (corpus.corpusId === "support-tree-graph") {
        return [
          {
            corpusId: corpus.corpusId,
            routeType: "TREE_FULLTEXT",
            strategyIds: ["TREE_FULLTEXT_V1"],
          },
          {
            corpusId: corpus.corpusId,
            routeType: "TREE_VECTOR",
            strategyIds: ["TREE_VECTOR_V1"],
          },
        ];
      }

      return [
        {
          corpusId: corpus.corpusId,
          routeType: "FULLTEXT",
          strategyIds: ["FULLTEXT_SQLITE_FTS5_SIMPLE_V1"],
        },
        {
          corpusId: corpus.corpusId,
          routeType: "VECTOR",
          strategyIds: ["VECTOR_SQLITE_VEC0_COSINE_V1"],
        },
        {
          corpusId: corpus.corpusId,
          routeType: "TREE_FULLTEXT",
          strategyIds: ["TREE_FULLTEXT_V1"],
        },
        {
          corpusId: corpus.corpusId,
          routeType: "TREE_VECTOR",
          strategyIds: ["TREE_VECTOR_V1"],
        },
        {
          corpusId: corpus.corpusId,
          routeType: "STRUCTURED_FILTER",
          strategyIds: ["STRUCTURED_FILTER_SQLITE_V1"],
        },
      ];
    }),
    topicPolicyTypes: ["NONE", "EMBEDDING_MMR_V1"],
  };
}

function createStrategy(strategyId: string, configSchema: Record<string, unknown> = {}) {
  return {
    configSchema,
    displayName: strategyId,
    strategyId,
  };
}

function createSeedPipelineCatalog(profileType: "INGEST" | "RETRIEVAL"): RagPipelineCatalogApi {
  if (profileType === "INGEST") {
    const stage = (
      stageType: string,
      inputArtifactKeys: string[],
      outputArtifactKeys: string[],
      strategies: Array<ReturnType<typeof createStrategy>>,
      orchestration: {
        actsAsBarrier?: boolean;
        mustRunAfterStageTypes?: string[];
        mustRunBeforeStageTypes?: string[];
        required?: boolean;
      } = {},
    ) => ({
      allowedStrategies: strategies.map((strategy) => strategy.strategyId),
      defaultFailurePolicy: "FAIL_PIPELINE",
      defaultJoinPolicy: "ALL_SUCCESS",
      displayName: stageType,
      inputArtifactKeys,
      orchestration: {
        actsAsBarrier: orchestration.actsAsBarrier ?? false,
        mustRunAfterStageTypes: orchestration.mustRunAfterStageTypes ?? [],
        mustRunBeforeStageTypes: orchestration.mustRunBeforeStageTypes ?? [],
        required: orchestration.required ?? false,
      },
      outputArtifactKeys,
      profileTypes: ["INGEST"] as const,
      repeatable: false,
      stageType,
      strategies,
    });
    return {
      failurePolicies: ["FAIL_PIPELINE", "SKIP_DOWNSTREAM", "CONTINUE_WITH_WARNING"],
      joinPolicies: ["ALL_SUCCESS", "ANY_SUCCESS", "BEST_EFFORT"],
      profileType,
      stages: [
        stage("VALIDATE_SOURCE", ["request.payload"], ["source.validated"], [createStrategy("RAG_VALIDATE_SOURCE")]),
        stage("PREPARE_PAYLOAD", ["request.payload"], ["payload.prepared"], [
          createStrategy("RAG_TEXT_CONTRACT_PAYLOAD_V1"),
          createStrategy("RAG_DOCUMENT_REF_CONTRACT_PAYLOAD_V1"),
        ]),
        stage("RESOLVE_SOURCE", ["payload.prepared"], ["source.resolved"], [createStrategy("PROJECT_DOC_RESOLVE_SOURCE", { maxBytes: { type: "integer" } })]),
        stage("PARSE_DOCUMENT", ["source.resolved"], ["document.parsed"], [createStrategy("RAG_PARSE_MARKDOWN_DOCUMENT", { format: { type: "string" } })]),
        stage("SPLIT_TEXT", ["payload.prepared"], ["textUnits.generated"], [
          createStrategy("RAG_SPLIT_PLAIN_TEXT_V1", { splitter: { type: "object" } }),
          createStrategy("RAG_SPLIT_MARKDOWN_HEADING_V1", { splitter: { type: "object" } }),
          createStrategy("RAG_SPLIT_TEXT", { splitter: { type: "object" } }),
        ]),
        stage("EXTRACT_TREE", ["tree.hints", "textUnits.generated"], ["tree.extracted", "graph.related"], [createStrategy("RAG_MARKDOWN_TREE_EXTRACT_V1")], {
          mustRunBeforeStageTypes: ["SUMMARIZE_TREE"],
          mustRunAfterStageTypes: ["SPLIT_TEXT"],
        }),
        stage("SUMMARIZE_TREE", ["tree.extracted"], ["tree.summarized"], [createStrategy("RAG_TREE_EXTRACTIVE_SUMMARY_V1")], {
          mustRunBeforeStageTypes: ["TREE_FULLTEXT_INDEX", "TREE_VECTOR_INDEX"],
          mustRunAfterStageTypes: ["EXTRACT_TREE"],
        }),
        stage("FULLTEXT_INDEX", ["textUnits.generated"], ["index.fulltext"], [createStrategy("RAG_FULLTEXT_INDEX")], {
          mustRunBeforeStageTypes: ["PERSIST_SOURCE"],
        }),
        stage("VECTOR_INDEX", ["textUnits.generated"], ["index.vector"], [createStrategy("RAG_VECTOR_INDEX")], {
          mustRunBeforeStageTypes: ["PERSIST_SOURCE"],
        }),
        stage("TREE_FULLTEXT_INDEX", ["tree.summarized"], ["index.treeFulltext"], [createStrategy("RAG_TREE_FULLTEXT_INDEX")], {
          mustRunBeforeStageTypes: ["PERSIST_TREE", "PERSIST_SOURCE"],
          mustRunAfterStageTypes: ["SUMMARIZE_TREE"],
        }),
        stage("TREE_VECTOR_INDEX", ["tree.summarized"], ["index.treeVector"], [createStrategy("RAG_TREE_VECTOR_INDEX")], {
          mustRunBeforeStageTypes: ["PERSIST_TREE", "PERSIST_SOURCE"],
          mustRunAfterStageTypes: ["SUMMARIZE_TREE"],
        }),
        stage("PERSIST_TREE", ["tree.summarized"], ["tree.writePayload", "tree.persisted"], [createStrategy("RAG_PERSIST_TREE_GRAPH_V1")], {
          mustRunBeforeStageTypes: ["PERSIST_SOURCE"],
          mustRunAfterStageTypes: ["SUMMARIZE_TREE", "TREE_FULLTEXT_INDEX", "TREE_VECTOR_INDEX"],
        }),
        stage("PERSIST_SOURCE", ["textUnits.generated"], ["source.persisted"], [createStrategy("RAG_PERSIST_SOURCE")]),
      ],
    };
  }

  const retrievalStage = (stageType: string, outputArtifactKeys: string[], strategies: string[]) => ({
    allowedStrategies: strategies,
    defaultFailurePolicy: "CONTINUE_WITH_WARNING",
    defaultJoinPolicy: "BEST_EFFORT",
    displayName: stageType,
    inputArtifactKeys: ["query.original", "filters.request"],
    outputArtifactKeys,
    profileTypes: ["RETRIEVAL"] as const,
    repeatable: true,
    stageType,
    strategies: strategies.map((strategyId) => createStrategy(strategyId, { corpusId: { type: "string" } })),
  });
  return {
    failurePolicies: ["FAIL_PIPELINE", "SKIP_DOWNSTREAM", "CONTINUE_WITH_WARNING"],
    joinPolicies: ["ALL_SUCCESS", "ANY_SUCCESS", "BEST_EFFORT"],
    profileType,
    stages: [
      retrievalStage("FULLTEXT", ["hits.fulltext"], ["FULLTEXT_SQLITE_FTS5_SIMPLE_V1", "FULLTEXT_GRAPH_SINGLE_NODE_V1"]),
      retrievalStage("VECTOR", ["hits.vector"], ["VECTOR_SQLITE_VEC0_COSINE_V1", "VECTOR_GRAPH_SINGLE_NODE_V1"]),
      retrievalStage("TREE_FULLTEXT", ["hits.treeFulltext"], ["TREE_FULLTEXT_V1"]),
      retrievalStage("TREE_VECTOR", ["hits.treeVector"], ["TREE_VECTOR_V1"]),
      retrievalStage("GRAPH", ["hits.graph"], ["FULLTEXT_GRAPH_SINGLE_NODE_V1", "VECTOR_GRAPH_SINGLE_NODE_V1"]),
      retrievalStage("STRUCTURED_FILTER", ["filters.structured"], ["STRUCTURED_FILTER_SQLITE_V1"]),
    ],
  };
}

function createProjectDocIngestProfile(): RagProfileDetailApi {
  return {
    corpusIds: ["project-doc"],
    description: "Primary project documentation ingest profile.",
    embeddingModelCode: "text-embedding-3-large",
    embeddingVersion: "v1",
    graphWritePolicy: "",
    pipelineStages: [
      { enabled: true, id: "preparePayload", strategyId: "RAG_TEXT_CONTRACT_PAYLOAD_V1" },
      { enabled: true, id: "splitText", strategyId: "RAG_SPLIT_TEXT", config: { splitter: { maxTokens: 800, overlapTokens: 80, type: "plainText" } } },
      { enabled: true, id: "persistSource", strategyId: "RAG_PERSIST_SOURCE" },
    ],
    primaryCorpusId: "project-doc",
    profileId: "project-doc-ingest-v1",
    profileType: "INGEST",
    requiredSourceVersion: "v1",
    sourceInputPolicy: {
      supportedSourceTypes: ["DOC", "MARKDOWN"],
    },
    status: "enabled",
  };
}

function createProjectDocRetrievalProfile(): RagProfileDetailApi {
  return {
    contextStrategies: ["RESULT_LIST", "TOKEN_BUDGET_PACK", "DOC_QA_CONTEXT_PACK"],
    corpusIds: ["project-doc"],
    dedupPolicy: "TEXT_UNIT",
    degradePolicy: "RESULT_LIST",
    description: "Project documentation retrieval profile.",
    filters: [],
    fusionPolicy: "RRF",
    graphExpansionPolicy: "",
    primaryCorpusId: "project-doc",
    profileId: "project-doc-related-v1",
    profileType: "RETRIEVAL",
    requiredSourceVersion: "v1",
    routes: [
      { corpusId: "project-doc", enabled: true, routeType: "FULLTEXT", strategyId: "FULLTEXT_SQLITE_FTS5_SIMPLE_V1" },
      { corpusId: "project-doc", enabled: true, routeType: "VECTOR", strategyId: "VECTOR_SQLITE_VEC0_COSINE_V1" },
    ],
    status: "enabled",
    topicPolicy: "NONE",
  };
}

function createSeedState(): RagManagementState {
  const corpora = new Map<string, RagCorpusDetailApi>();
  [createProjectDocCorpus(), createMemoryCardCorpus(), createSupportTreeGraphCorpus()].forEach((corpus) => {
    corpora.set(corpus.corpusId, corpus);
  });
  const catalogCorpora = Array.from(corpora.values()).map((corpus) => ({
    capabilities: corpus.capabilities,
    corpusId: corpus.corpusId,
    displayName: corpus.displayName,
    filterSchema: corpus.filterSchema,
    status: corpus.status,
    storageSummary: corpus.storageSummary,
  }));
  const profiles = new Map<string, RagProfileDetailApi>();
  [createProjectDocIngestProfile(), createProjectDocRetrievalProfile()].forEach((profile) => {
    profiles.set(profile.profileId, profile);
  });
  return {
    corpora,
    profileCatalog: createSeedProfileCatalog(catalogCorpora),
    profiles,
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function readRequestBody(request: Request): unknown {
  const rawBody = request.postData();
  if (!rawBody) return undefined;
  try {
    return JSON.parse(rawBody);
  } catch {
    return rawBody;
  }
}

function normalizeProfileType(value: unknown): "INGEST" | "RETRIEVAL" {
  return String(value).trim().toUpperCase() === "INGEST" ? "INGEST" : "RETRIEVAL";
}

function normalizeProfileStatus(value: unknown) {
  const normalized = String(value).trim().toLowerCase();
  if (normalized === "enabled" || normalized === "ready") return "enabled";
  if (normalized === "reserved" || normalized === "needs review" || normalized === "needs_review") return "reserved";
  if (normalized === "disabled" || normalized === "draft") return "disabled";
  return "";
}

function readString(value: unknown) {
  return typeof value === "string" ? value : "";
}

function readStringArray(value: unknown) {
  return Array.isArray(value)
    ? value.filter((item): item is string => typeof item === "string").map((item) => item.trim()).filter(Boolean)
    : [];
}

function readStageRows(value: unknown): RagPipelineStageApi[] {
  if (!Array.isArray(value)) return [];
  return value
    .filter(isRecord)
    .map((row) => ({
      config: isRecord(row.config) ? row.config : undefined,
      enabled: Boolean(row.enabled),
      id: readString(row.id),
      strategyId: readString(row.strategyId),
    }));
}

function readRoutes(value: unknown): RagRouteApi[] {
  if (!Array.isArray(value)) return [];
  return value
    .filter(isRecord)
    .map((row, index) => ({
      corpusId: readString(row.corpusId),
      enabled: Boolean(row.enabled),
      routeId: readString(row.routeId) || `route-${index + 1}`,
      routeType: readString(row.routeType),
      strategyId: readString(row.strategyId),
    }));
}

const ingestStageTypesById: Record<string, string> = {
  extractTree: "EXTRACT_TREE",
  fulltextIndex: "FULLTEXT_INDEX",
  parseDocument: "PARSE_DOCUMENT",
  persistTree: "PERSIST_TREE",
  persistSource: "PERSIST_SOURCE",
  preparePayload: "PREPARE_PAYLOAD",
  resolveSource: "RESOLVE_SOURCE",
  splitText: "SPLIT_TEXT",
  summarizeTree: "SUMMARIZE_TREE",
  treeFulltextIndex: "TREE_FULLTEXT_INDEX",
  treeVectorIndex: "TREE_VECTOR_INDEX",
  validateSource: "VALIDATE_SOURCE",
  vectorIndex: "VECTOR_INDEX",
};
const ingestStageIdsByType = Object.fromEntries(
  Object.entries(ingestStageTypesById).map(([stageId, stageType]) => [stageType, stageId]),
);

function kebabNodeId(value: string) {
  return value
    .trim()
    .toLowerCase()
    .replace(/_/g, "-")
    .replace(/[^a-z0-9-]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

function pipelineEdges(nodeIds: string[]): RagPipelineEdgeApi[] {
  return nodeIds.slice(0, -1).map((nodeId, index) => ({ from: nodeId, to: nodeIds[index + 1] }));
}

function readPipeline(value: unknown): RagPipelineApi | undefined {
  if (!isRecord(value) || !Array.isArray(value.nodes)) return undefined;
  const nodes = value.nodes
    .filter(isRecord)
    .map((node) => ({
      config: isRecord(node.config) ? node.config : undefined,
      enabled: node.enabled === undefined ? true : Boolean(node.enabled),
      failurePolicy: readString(node.failurePolicy) || undefined,
      joinPolicy: readString(node.joinPolicy) || undefined,
      nodeId: readString(node.nodeId),
      stageType: readString(node.stageType),
      strategyId: readString(node.strategyId),
    }))
    .filter((node) => node.nodeId && node.stageType);
  const edges = Array.isArray(value.edges)
    ? value.edges
      .filter(isRecord)
      .map((edge) => ({ from: readString(edge.from), to: readString(edge.to) }))
      .filter((edge) => edge.from && edge.to)
    : pipelineEdges(nodes.filter((node) => node.enabled !== false).map((node) => node.nodeId));
  return {
    edges,
    mode: "DAG",
    nodes,
  };
}

function pipelineFromLegacyStages(stages: RagPipelineStageApi[]): RagPipelineApi {
  const nodes = stages
    .filter((stage) => stage.enabled)
    .map((stage) => ({
      config: stage.config,
      enabled: true,
      failurePolicy: "FAIL_PIPELINE",
      joinPolicy: "ALL_SUCCESS",
      nodeId: kebabNodeId(ingestStageTypesById[stage.id] ?? stage.id),
      stageType: ingestStageTypesById[stage.id] ?? stage.id,
      strategyId: stage.strategyId,
    }));
  return {
    edges: pipelineEdges(nodes.map((node) => node.nodeId)),
    mode: "DAG",
    nodes,
  };
}

function pipelineFromLegacyRoutes(routes: RagRouteApi[]): RagPipelineApi {
  const nodes = routes
    .filter((route) => route.enabled !== false)
    .map((route, index) => {
      const nodeId = readString(route.routeId) || `${kebabNodeId(route.corpusId || "route")}-${kebabNodeId(route.routeType || "retrieval")}-${index + 1}`;
      return {
        config: {
          corpusId: route.corpusId,
          routeType: route.routeType,
        },
        enabled: true,
        failurePolicy: "CONTINUE_WITH_WARNING",
        joinPolicy: "BEST_EFFORT",
        nodeId,
        stageType: route.routeType,
        strategyId: route.strategyId,
      };
    });
  return {
    edges: pipelineEdges(nodes.map((node) => node.nodeId)),
    mode: "DAG",
    nodes,
  };
}

function readFilters(value: unknown): RagFilterApi[] {
  if (!Array.isArray(value)) return [];
  return value
    .filter(isRecord)
    .map((row) => ({
      field: readString(row.field),
      operator: readString(row.operator),
      value: readString(row.value),
    }));
}

function readAccessContext(value: unknown): RagRuntimeAccessContextApi {
  const record = isRecord(value) ? value : {};
  return {
    agentCode: readString(record.agentCode),
    userCode: readString(record.userCode),
    workspaceCode: readString(record.workspaceCode),
  };
}

function readTargetScope(value: unknown): RagRuntimeScopeApi {
  const record = isRecord(value) ? value : {};
  return {
    scopeCode: readString(record.scopeCode),
    scopeType: readString(record.scopeType),
  };
}

function readStorageBinding(value: unknown): RagStorageBindingApi {
  if (!isRecord(value) || !Array.isArray(value.targets) || !isRecord(value.bindings)) {
    return {
      bindings: {},
      mode: "TARGETED",
      targets: [],
    };
  }

  return {
    bindings: {
      diagnostics: readString(value.bindings.diagnostics),
      fulltext: readString(value.bindings.fulltext),
      source: readString(value.bindings.source),
      textUnit: readString(value.bindings.textUnit),
      tree: readString(value.bindings.tree),
      vector: readString(value.bindings.vector),
    },
    mode: "TARGETED",
    targets: value.targets
      .filter(isRecord)
      .map((target) => (
        readString(target.backend).toUpperCase() === "GRAPH"
          ? {
              backend: "GRAPH" as const,
              codeStrategy: readString(target.codeStrategy) || undefined,
              id: readString(target.id),
              layout: readString(target.layout) === "GRAPH_TREE" ? "GRAPH_TREE" as const : "GRAPH_SINGLE_NODE" as const,
              namespace: readString(target.namespace) || undefined,
              nodeLabel: readString(target.nodeLabel) || undefined,
            }
          : {
              backend: "SQLITE" as const,
              id: readString(target.id),
              layout: "TABLE" as const,
              table: readString(target.table) || undefined,
            }
      )),
  };
}

function buildRuntimeProjection(binding: RagStorageBindingApi, compatible: boolean, issues: string[]) {
  return {
    compatible,
    issues,
    storageBinding: binding,
  };
}

function collectBoundStorageTargets(binding: RagStorageBindingApi) {
  const targetMap = new Map(binding.targets.map((target) => [target.id, target]));
  return ["source", "textUnit", "fulltext", "vector", "tree"]
    .map((segmentKey) => targetMap.get(binding.bindings[segmentKey as keyof typeof binding.bindings] ?? ""))
    .filter((target): target is RagStorageTargetApi => Boolean(target));
}

function validateCorpusPayload(
  payload: unknown,
  state: RagManagementState,
  currentCorpusId?: string,
): { errors: RagValidationErrorApi[]; runtimeProjection: RagCorpusDetailApi["runtimeProjection"] } {
  const record = isRecord(payload) ? payload : {};
  const corpusId = readString(record.corpusId);
  const displayName = readString(record.displayName);
  const capabilities = isRecord(record.capabilities) ? record.capabilities : {};
  const graphEnabled = Boolean(capabilities.graph);
  const storageBinding = readStorageBinding(record.storageBinding);
  const errors: RagValidationErrorApi[] = [];

  if (!corpusId) {
    errors.push({ field: "code", message: "Corpus code is required." });
  } else if (state.corpora.has(corpusId) && corpusId !== currentCorpusId) {
    errors.push({ field: "code", message: "Corpus code must be unique." });
  }

  if (!displayName) {
    errors.push({ field: "name", message: "Corpus name is required." });
  }

  const boundTargets = collectBoundStorageTargets(storageBinding);
  const graphBoundTargets = boundTargets.filter((target) => target.backend === "GRAPH");
  if (graphEnabled && graphBoundTargets.length === 0) {
    errors.push({
      code: "RAG_STORAGE_GRAPH_BINDING_REQUIRED",
      field: "storageBinding",
      message: "Graph enabled requires at least one non-diagnostics segment to bind to a GRAPH target.",
    });
  }

  const runtimeProjection = buildRuntimeProjection(storageBinding, errors.length === 0, errors.map((error) => error.message));

  return {
    errors,
    runtimeProjection,
  };
}

function upsertCorpus(payload: unknown, state: RagManagementState, runtimeProjection: RagCorpusDetailApi["runtimeProjection"]): RagCorpusDetailApi {
  const record = isRecord(payload) ? payload : {};
  const corpusId = readString(record.corpusId);
  const previous = state.corpora.get(corpusId);
  const storageBinding = readStorageBinding(record.storageBinding);
  const nextCorpus: RagCorpusDetailApi = {
    capabilities: isRecord(record.capabilities)
      ? {
          fulltext: Boolean(record.capabilities.fulltext),
          graph: Boolean(record.capabilities.graph),
          reserved: Boolean(record.capabilities.reserved),
          tree: Boolean(record.capabilities.tree),
          vector: Boolean(record.capabilities.vector),
        }
      : {
          fulltext: true,
          graph: false,
          reserved: false,
          tree: true,
          vector: true,
        },
    corpusId,
    description: readString(record.description),
    displayName: readString(record.displayName),
    fieldMapping: isRecord(record.fieldMapping)
      ? {
          source: isRecord(record.fieldMapping.source) ? Object.fromEntries(Object.entries(record.fieldMapping.source).map(([key, value]) => [key, readString(value)])) : {},
          textUnit: isRecord(record.fieldMapping.textUnit) ? Object.fromEntries(Object.entries(record.fieldMapping.textUnit).map(([key, value]) => [key, readString(value)])) : {},
        }
      : { source: {}, textUnit: {} },
    filterSchema: isRecord(record.filterSchema) && isRecord(record.filterSchema.fields)
      ? {
          fields: Object.fromEntries(
            Object.entries(record.filterSchema.fields)
              .filter(([, value]) => isRecord(value))
              .map(([key, value]) => [key, {
                column: readString(value.column),
                operators: Array.isArray(value.operators) ? value.operators.filter((item): item is string => typeof item === "string") : ["eq", "in"],
                type: (readString(value.type) || "string") as "boolean" | "integer" | "string",
              }]),
          ),
        }
      : { fields: {} },
    ingestProfileId: previous?.ingestProfileId ?? "",
    metadataGroups: Array.isArray(record.metadataGroups)
      ? record.metadataGroups.filter(isRecord)
      : (previous?.metadataGroups ?? []),
    retrievalProfileId: previous?.retrievalProfileId ?? "",
    runtimeProjection,
    status: readString(record.status) || "enabled",
    storageBinding,
    storageSummary: formatStorageSummary(storageBinding),
  };

  state.corpora.set(nextCorpus.corpusId, nextCorpus);
  return nextCorpus;
}

function validateProfilePayload(
  payload: unknown,
  state: RagManagementState,
  currentProfileId?: string,
): RagValidationErrorApi[] {
  const record = isRecord(payload) ? payload : {};
  const profileId = readString(record.profileId);
  const profileType = normalizeProfileType(record.profileType);
  const primaryCorpusId = readString(record.primaryCorpusId);
  const status = normalizeProfileStatus(record.status);
  const errors: RagValidationErrorApi[] = [];

  if (!profileId) {
    errors.push({ field: "id", message: "Profile ID is required." });
  } else if (state.profiles.has(profileId) && profileId !== currentProfileId) {
    errors.push({ field: "id", message: "Profile ID must be unique." });
  }

  if (!primaryCorpusId) {
    errors.push({ field: "primaryCorpusId", message: "Primary corpus is required." });
  }

  if (!status) {
    errors.push({ field: "status", message: "Profile status must use enabled, reserved, or disabled." });
  } else if (status !== "enabled") {
    errors.push({ field: "status", message: "Current RAG runtime only compiles enabled profiles." });
  }

  if (profileType === "INGEST") {
    const pipelineStages = readStageRows(record.pipelineStages);
    const pipeline = readPipeline(record.pipeline) ?? pipelineFromLegacyStages(pipelineStages);
    const enabledNodes = pipeline.nodes.filter((node) => node.enabled !== false);
    if (enabledNodes.length === 0) {
      errors.push({ field: "pipeline", message: "At least one pipeline node must be enabled." });
    }
    if (!enabledNodes.some((node) => node.stageType === "PREPARE_PAYLOAD") || !enabledNodes.some((node) => node.stageType === "PERSIST_SOURCE")) {
      errors.push({ field: "pipeline", message: "PREPARE_PAYLOAD and PERSIST_SOURCE must both be enabled." });
    }
    enabledNodes.forEach((node, index) => {
      if (!node.strategyId.trim()) {
        errors.push({ field: `pipeline.nodes[${index}].strategyId`, message: "Strategy ID is required." });
      }
    });
    if (!isRecord(record.sourceInputPolicy) || readStringArray(record.sourceInputPolicy.supportedSourceTypes).length === 0) {
      errors.push({ field: "sourceInputPolicy.supportedSourceTypes", message: "Supported source types are required." });
    }
    return errors;
  }

  const routes = readRoutes(record.routes);
  const pipeline = readPipeline(record.pipeline) ?? pipelineFromLegacyRoutes(routes);
  const enabledNodes = pipeline.nodes.filter((node) => node.enabled !== false);
  const routeStageTypes = new Set(["FULLTEXT", "VECTOR", "TREE_FULLTEXT", "TREE_VECTOR", "GRAPH", "STRUCTURED_FILTER", "ROUTE_SEARCH"]);
  const routeNodes = enabledNodes.filter((node) => routeStageTypes.has(node.stageType));
  if (enabledNodes.length === 0) {
    errors.push({ field: "pipeline", message: "At least one pipeline node must be enabled." });
  }
  if (primaryCorpusId && !routeNodes.some((node) => isRecord(node.config) && readString(node.config.corpusId) === primaryCorpusId)) {
    errors.push({ field: "pipeline", message: "Primary corpus must appear in at least one enabled node." });
  }
  routeNodes.forEach((node, index) => {
    const config = isRecord(node.config) ? node.config : {};
    if (!readString(config.corpusId).trim()) {
      errors.push({ field: `pipeline.nodes[${index}].config.corpusId`, message: "Node corpus is required." });
    }
    if (!node.stageType.trim()) {
      errors.push({ field: `pipeline.nodes[${index}].stageType`, message: "Stage type is required." });
    }
    if (!node.strategyId.trim()) {
      errors.push({ field: `pipeline.nodes[${index}].strategyId`, message: "Strategy ID is required." });
    }
  });

  const contextStrategies = readStringArray(record.contextStrategies);
  if (contextStrategies.length === 0) {
    errors.push({ field: "contextStrategies", message: "At least one context strategy must be enabled." });
  }

  const dedupPolicy = readString(record.dedupPolicy) || "TEXT_UNIT";
  if (!["TEXT_UNIT", "CANONICAL_TEXT_UNIT"].includes(dedupPolicy)) {
    errors.push({ field: "dedupPolicy", message: "Dedup policy must use TEXT_UNIT or CANONICAL_TEXT_UNIT." });
  }

  const topicPolicy = readString(record.topicPolicy) || "NONE";
  if (!["NONE", "EMBEDDING_MMR_V1"].includes(topicPolicy)) {
    errors.push({ field: "topicPolicy", message: "Topic policy must use NONE or EMBEDDING_MMR_V1." });
  }

  const graphExpansionPolicy = readString(record.graphExpansionPolicy);
  if (graphExpansionPolicy && !routeNodes.some((node) => node.stageType === "GRAPH")) {
    errors.push({ field: "graphExpansionPolicy", message: "Graph expansion requires at least one enabled GRAPH route." });
  }

  readFilters(record.filters).forEach((filter, index) => {
    if (!filter.field.trim()) {
      errors.push({ field: `filters[${index}].field`, message: "Filter field is required." });
    }
    if (!filter.operator.trim()) {
      errors.push({ field: `filters[${index}].operator`, message: "Filter operator is required." });
    }
    if (!filter.value.trim()) {
      errors.push({ field: `filters[${index}].value`, message: "Filter value is required." });
    }
  });

  return errors;
}

function upsertProfile(payload: unknown, state: RagManagementState): RagProfileDetailApi {
  const record = isRecord(payload) ? payload : {};
  const profileType = normalizeProfileType(record.profileType);
  const profileId = readString(record.profileId);
  const pipeline = readPipeline(record.pipeline);
  const nextProfile: RagProfileDetailApi = profileType === "INGEST"
    ? {
        corpusIds: readStringArray(record.corpusIds),
        description: readString(record.description),
        embeddingModelCode: readString(record.embeddingModelCode),
        embeddingVersion: readString(record.embeddingVersion),
        graphWritePolicy: readString(record.graphWritePolicy),
        pipeline: pipeline ?? pipelineFromLegacyStages(readStageRows(record.pipelineStages)),
        primaryCorpusId: readString(record.primaryCorpusId),
        profileId,
        profileType,
        requiredSourceVersion: readString(record.requiredSourceVersion) || "v1",
        sourceInputPolicy: isRecord(record.sourceInputPolicy) ? {
          supportedSourceTypes: readStringArray(record.sourceInputPolicy.supportedSourceTypes),
        } : { supportedSourceTypes: [] },
        status: normalizeProfileStatus(record.status) || "disabled",
      }
    : {
        contextStrategies: readStringArray(record.contextStrategies),
        corpusIds: readStringArray(record.corpusIds),
        dedupPolicy: readString(record.dedupPolicy),
        degradePolicy: readString(record.degradePolicy),
        description: readString(record.description),
        filters: readFilters(record.filters),
        fusionPolicy: readString(record.fusionPolicy),
        graphExpansionPolicy: readString(record.graphExpansionPolicy),
        pipeline: pipeline ?? pipelineFromLegacyRoutes(readRoutes(record.routes)),
        primaryCorpusId: readString(record.primaryCorpusId),
        profileId,
        profileType,
        requiredSourceVersion: readString(record.requiredSourceVersion) || "v1",
        rerankEnabled: Boolean(record.rerankEnabled),
        rerankModelCode: readString(record.rerankModelCode),
        status: normalizeProfileStatus(record.status) || "disabled",
        topicPolicy: readString(record.topicPolicy),
      };

  state.profiles.set(profileId, nextProfile);
  return nextProfile;
}

function listCorpora(state: RagManagementState) {
  return Array.from(state.corpora.values())
    .sort((left, right) => left.displayName.localeCompare(right.displayName))
    .map((corpus) => ({
      capabilities: corpus.capabilities,
      corpusId: corpus.corpusId,
      description: corpus.description,
      displayName: corpus.displayName,
      ingestProfileId: corpus.ingestProfileId,
      metadataGroups: corpus.metadataGroups,
      retrievalProfileId: corpus.retrievalProfileId,
      status: corpus.status,
      storageSummary: corpus.storageSummary,
    }));
}

function listProfiles(state: RagManagementState, requestedType?: "INGEST" | "RETRIEVAL") {
  return Array.from(state.profiles.values())
    .filter((profile) => !requestedType || profile.profileType === requestedType)
    .sort((left, right) => left.profileId.localeCompare(right.profileId));
}

function fulfillJson(route: Route, data: unknown, status = 200) {
  return route.fulfill({
    body: JSON.stringify({
      code: "SUCCESS",
      data,
      message: "ok",
    }),
    contentType: "application/json",
    status,
  });
}

function fulfillError(route: Route, status: number, code: string, message: string, data?: unknown) {
  return route.fulfill({
    body: JSON.stringify({
      code,
      data: data ?? {},
      message,
    }),
    contentType: "application/json",
    status,
  });
}

export async function installRagManagementApiMock(page: Page, requests: CapturedRequest[]) {
  const state = createSeedState();

  await page.route("**/api/v1/rag/**", async (route) => {
    const request = route.request();
    const url = new URL(request.url());
    const pathname = url.pathname;
    const method = request.method();
    const body = readRequestBody(request);

    const capturedRequest: CapturedRequest = {
      body,
      method,
      pathname,
      query: url.searchParams.toString(),
    };
    requests.push(capturedRequest);

    if (pathname === "/api/v1/rag/corpora" && method === "GET") {
      await fulfillJson(route, listCorpora(state));
      return;
    }

    if (pathname === "/api/v1/rag/ingest" && method === "POST") {
      const record = isRecord(body) ? body : {};
      const payload = isRecord(record.payload) ? record.payload : {};
      const source = isRecord(payload.source) ? payload.source : {};
      const accessContext = readAccessContext(record.accessContext);
      const targetScope = readTargetScope(record.targetScope);
      const sourceCode = readString(source.code) || "workbench-source";
      const responseData = {
        corpusId: readString(record.corpusId),
        embeddingStatus: "INDEXED",
        fulltextStatus: "INDEXED",
        profileId: readString(record.ingestProfileId),
        sourceCode,
        sourceVersion: readString(source.version) || "v1",
        stageResults: [
          {
            durationMs: 2,
            inputArtifactKeys: ["request.payload"],
            nodeId: "prepare-payload",
            outputArtifactKeys: ["payload.prepared", "source.metadata"],
            stageType: "PREPARE_PAYLOAD",
            status: "COMPLETED",
            strategyId: "RAG_TEXT_CONTRACT_PAYLOAD_V1",
            warnings: [],
          },
          {
            durationMs: 3,
            inputArtifactKeys: ["payload.prepared"],
            nodeId: "split-text",
            outputArtifactKeys: ["textUnits.generated"],
            stageType: "SPLIT_TEXT",
            status: "COMPLETED",
            strategyId: "RAG_SPLIT_TEXT",
            warnings: ["splitter used default overlap"],
          },
        ],
        status: "success",
        textUnitCount: targetScope.scopeCode === "TRON_CORE" && accessContext.workspaceCode === "TRON_CORE" ? 3 : 1,
        warnings: [],
      };
      capturedRequest.responseData = responseData;
      await fulfillJson(route, responseData);
      return;
    }

    if (pathname === "/api/v1/rag/search" && method === "POST") {
      const record = isRecord(body) ? body : {};
      const query = readString(record.query);
      const traceEnabled = isRecord(record.trace) ? Boolean(record.trace.enabled) : false;
      const responseData = {
        contextPack: {
          items: [
            {
              metadata: { route: "VECTOR" },
              sourceCode: "docs/features/rag-storage-runtime-workbench",
              text: "Workbench search result from mocked runtime.",
              tokens: 42,
            },
          ],
          tokenCount: 42,
        },
        contextStrategy: "DOC_QA_CONTEXT_PACK",
        hits: [
          {
            corpusId: "project-doc",
            documentCode: "rag-storage-runtime-workbench",
            fields: {
              matchedBy: "TREE_VECTOR",
              scopeCode: "TRON_CORE",
              treeNodeCode: "tree-runtime-workbench",
              treeNodeTitle: "Runtime Workbench Summary",
              treeScore: 0.88,
            },
            metadata: {
              matchedBy: "TREE_VECTOR",
              scopeCode: "TRON_CORE",
              treeNodeCode: "tree-runtime-workbench",
              treeNodeTitle: "Runtime Workbench Summary",
              treeScore: 0.88,
            },
            repoCode: "tron",
            score: 0.91,
            sourceCode: "docs/features/rag-storage-runtime-workbench",
            sourceVersion: "v1",
            text: "RAG workbench can execute retrieval through the backend API.",
            title: "RAG Storage Runtime Workbench",
          },
        ],
        profileId: readString(record.profileId),
        query,
        ...(query.includes("legacy fallback") ? {} : {
          stageResults: [
            {
              durationMs: 5,
              inputArtifactKeys: ["query.effective"],
              nodeId: "route-search-fulltext",
              outputArtifactKeys: ["hits.route"],
              stageType: "ROUTE_SEARCH",
              status: "COMPLETED",
              strategyId: "FULLTEXT_SQLITE_FTS5_SIMPLE_V1",
              warnings: [],
            },
            {
              durationMs: 7,
              inputArtifactKeys: ["hits.route"],
              nodeId: "context-pack",
              outputArtifactKeys: ["context.pack"],
              stageType: "CONTEXT_PACK",
              status: "COMPLETED_WITH_WARNINGS",
              strategyId: "DOC_QA_CONTEXT_PACK",
              warnings: ["rerank disabled; using lexical order"],
            },
          ],
        }),
        traceCode: traceEnabled ? "trace-workbench-001" : null,
        warnings: [],
      };
      capturedRequest.responseData = responseData;
      await fulfillJson(route, responseData);
      return;
    }

    if (pathname === "/api/v1/rag/corpora/catalog" && method === "GET") {
      await fulfillJson(route, {
        capabilityRules: [
          { capability: "graph", rule: "Graph capability requires a bound GRAPH target." },
        ],
        codeStrategies: ["CORPUS_SOURCE_VERSION_SCOPE_HASH"],
        compatibilityRules: [],
        contractFields: {
          source: [
            { column: "source_code", logicalName: "sourceCode" },
            { column: "source_version", logicalName: "sourceVersion" },
            { column: "scope_type", logicalName: "scopeType" },
            { column: "scope_code", logicalName: "scopeCode" },
            { column: "title", logicalName: "title" },
          ],
          textUnit: [
            { column: "source_code", logicalName: "sourceCode" },
            { column: "source_version", logicalName: "sourceVersion" },
            { column: "scope_type", logicalName: "scopeType" },
            { column: "scope_code", logicalName: "scopeCode" },
            { column: "text", logicalName: "text" },
          ],
        },
        filterFieldTypes: ["string", "integer", "boolean"],
        filterOperators: ["eq", "in", "gt", "gte", "lt", "lte"],
        storageBackends: ["SQLITE", "GRAPH"],
      });
      return;
    }

    if (pathname === "/api/v1/rag/corpora/verify" && method === "POST") {
      const record = isRecord(body) ? body : {};
      const currentCorpusId = readString(record.corpusId);
      const existing = state.corpora.get(currentCorpusId);
      const { errors, runtimeProjection } = validateCorpusPayload(body, state, existing?.corpusId);
      await fulfillJson(route, {
        errors,
        runtimeProjection,
        valid: errors.length === 0,
      });
      return;
    }

    if (pathname === "/api/v1/rag/corpora" && method === "POST") {
      const { errors, runtimeProjection } = validateCorpusPayload(body, state);
      if (errors.length > 0) {
        await fulfillError(route, 400, errors[0]?.code ?? "RAG_MANAGEMENT_REQUEST_INVALID", errors[0]?.message ?? "Corpus request invalid", { errors });
        return;
      }
      await fulfillJson(route, upsertCorpus(body, state, runtimeProjection));
      return;
    }

    const corpusMatch = pathname.match(/^\/api\/v1\/rag\/corpora\/([^/]+)$/);
    if (corpusMatch && method === "GET") {
      const corpusId = decodeURIComponent(corpusMatch[1] ?? "");
      const corpus = state.corpora.get(corpusId);
      if (!corpus) {
        await fulfillError(route, 404, "RAG_CORPUS_NOT_FOUND", "Corpus not found");
        return;
      }
      await fulfillJson(route, corpus);
      return;
    }

    if (corpusMatch && method === "PUT") {
      const corpusId = decodeURIComponent(corpusMatch[1] ?? "");
      if (!state.corpora.has(corpusId)) {
        await fulfillError(route, 404, "RAG_CORPUS_NOT_FOUND", "Corpus not found");
        return;
      }
      const { errors, runtimeProjection } = validateCorpusPayload(body, state, corpusId);
      if (errors.length > 0) {
        await fulfillError(route, 400, errors[0]?.code ?? "RAG_MANAGEMENT_REQUEST_INVALID", errors[0]?.message ?? "Corpus request invalid", { errors });
        return;
      }
      await fulfillJson(route, upsertCorpus(body, state, runtimeProjection));
      return;
    }

    if (pathname === "/api/v1/rag/profiles" && method === "GET") {
      const profileTypeParam = readString(url.searchParams.get("profileType"));
      const requestedType = profileTypeParam ? normalizeProfileType(profileTypeParam) : undefined;
      await fulfillJson(route, listProfiles(state, requestedType));
      return;
    }

    if (pathname === "/api/v1/rag/profiles/catalog" && method === "GET") {
      await fulfillJson(route, state.profileCatalog);
      return;
    }

    if (pathname === "/api/v1/rag/pipeline/catalog" && method === "GET") {
      const profileType = normalizeProfileType(url.searchParams.get("profileType"));
      await fulfillJson(route, createSeedPipelineCatalog(profileType));
      return;
    }

    if (pathname === "/api/v1/rag/profiles/verify" && method === "POST") {
      const record = isRecord(body) ? body : {};
      const currentProfileId = readString(record.profileId);
      const existing = state.profiles.get(currentProfileId);
      const errors = validateProfilePayload(body, state, existing?.profileId);
      await fulfillJson(route, {
        errors,
        valid: errors.length === 0,
      });
      return;
    }

    if (pathname === "/api/v1/rag/profiles" && method === "POST") {
      const errors = validateProfilePayload(body, state);
      if (errors.length > 0) {
        await fulfillError(route, 400, "RAG_MANAGEMENT_REQUEST_INVALID", errors[0]?.message ?? "Profile request invalid", { errors });
        return;
      }
      await fulfillJson(route, upsertProfile(body, state));
      return;
    }

    const profileMatch = pathname.match(/^\/api\/v1\/rag\/profiles\/([^/]+)$/);
    if (profileMatch && method === "GET") {
      const profileId = decodeURIComponent(profileMatch[1] ?? "");
      const profile = state.profiles.get(profileId);
      if (!profile) {
        await fulfillError(route, 404, "RAG_PROFILE_NOT_FOUND", "Profile not found");
        return;
      }
      await fulfillJson(route, profile);
      return;
    }

    if (profileMatch && method === "PUT") {
      const profileId = decodeURIComponent(profileMatch[1] ?? "");
      if (!state.profiles.has(profileId)) {
        await fulfillError(route, 404, "RAG_PROFILE_NOT_FOUND", "Profile not found");
        return;
      }
      const errors = validateProfilePayload(body, state, profileId);
      if (errors.length > 0) {
        await fulfillError(route, 400, "RAG_MANAGEMENT_REQUEST_INVALID", errors[0]?.message ?? "Profile request invalid", { errors });
        return;
      }
      await fulfillJson(route, upsertProfile(body, state));
      return;
    }

    if (pathname === "/api/v1/rag/filters/validate" && method === "POST") {
      await fulfillJson(route, {
        fieldErrors: [],
        normalizedFilters: readFilters(isRecord(body) ? body.filters : []),
        valid: true,
      });
      return;
    }

    await fulfillError(route, 404, "RAG_ROUTE_NOT_FOUND", `${method} ${pathname} not mocked`);
  });

  return state;
}
