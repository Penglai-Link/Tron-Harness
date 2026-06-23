export type RagTab = "corpora" | "profiles" | "workbench" | "data" | "diagnostics";
export type DataView = "sources" | "textUnits" | "warnings" | "traces";
export type BadgeTone = "neutral" | "accent" | "success" | "warning" | "error";
export type RagStorageBackend = "SQLITE" | "GRAPH";
export type RagStorageTargetLayout = "TABLE" | "GRAPH_SINGLE_NODE" | "GRAPH_TREE";
export type RagStorageSegmentKey = "source" | "textUnit" | "fulltext" | "vector" | "tree" | "diagnostics";
export type RagCorpusStatusCode = "enabled" | "reserved" | "disabled";
export type RagFilterFieldType = "string" | "integer" | "boolean";

export interface RagCorpusCapabilities {
  fulltext: boolean;
  graph: boolean;
  reserved: boolean;
  tree: boolean;
  vector: boolean;
}

export interface RagCorpusFilterSchema {
  fields: Record<string, {
    column: string;
    operators?: string[];
    type: RagFilterFieldType;
  }>;
}

export interface RagCorpusFieldMapping {
  source: Record<string, string>;
  textUnit: Record<string, string>;
}

export interface RagSqliteStorageTargetBinding {
  backend: "SQLITE";
  layout: "TABLE";
  table?: string;
}

export interface RagGraphStorageTargetBinding {
  backend: "GRAPH";
  codeStrategy?: string;
  layout: "GRAPH_SINGLE_NODE" | "GRAPH_TREE";
  namespace?: string;
  nodeLabel?: string;
}

export type RagStoragePhysicalTarget = RagSqliteStorageTargetBinding | RagGraphStorageTargetBinding;

export type RagStorageTargetBinding = { id: string } & RagStoragePhysicalTarget;

export interface RagTargetedStorageBinding {
  bindings: Partial<Record<RagStorageSegmentKey, string>>;
  mode: "TARGETED";
  targets: RagStorageTargetBinding[];
}

export type RagStorageBinding = RagTargetedStorageBinding;
export type RagCorpusStorageBinding = RagStorageBinding;

export interface RagCorpusConfig {
  capabilities: RagCorpusCapabilities;
  corpusId: string;
  description: string;
  displayName: string;
  fieldMapping: RagCorpusFieldMapping;
  filterSchema: RagCorpusFilterSchema;
  status: RagCorpusStatusCode;
  storageBinding: RagCorpusStorageBinding;
}

export interface CorpusRecord {
  code: string;
  config?: RagCorpusConfig;
  description: string;
  filters: string[];
  fulltextHealth: string;
  ingestProfile: string;
  lastIndexed: string;
  name: string;
  retrievalProfile: string;
  scope: string;
  sources: number;
  status: "Active" | "Indexing" | "Warning" | "Reserved";
  storageBinding: string;
  strategies: string[];
  textUnits: number;
  vectorCoverage: number;
  warnings: number;
}

export type RagProfileStatus = "Ready" | "Draft" | "Needs Review";
export type RagProfileType = "Ingest" | "Retrieval";
export type RagPipelineMode = "DAG";
export type RagIngestStageId =
  | "validateSource"
  | "preparePayload"
  | "resolveSource"
  | "parseDocument"
  | "splitText"
  | "extractTree"
  | "summarizeTree"
  | "fulltextIndex"
  | "vectorIndex"
  | "treeFulltextIndex"
  | "treeVectorIndex"
  | "persistTree"
  | "persistSource";
export type RagRetrievalRouteType = "FULLTEXT" | "VECTOR" | "TREE_FULLTEXT" | "TREE_VECTOR" | "GRAPH" | "STRUCTURED_FILTER";

export interface RagPipelineNode {
  config?: Record<string, unknown>;
  enabled: boolean;
  failurePolicy?: string;
  joinPolicy?: string;
  nodeId: string;
  stageType: string;
  strategyId: string;
}

export interface RagPipelineEdge {
  from: string;
  to: string;
}

export interface RagProfilePipeline {
  edges: RagPipelineEdge[];
  mode: RagPipelineMode;
  nodes: RagPipelineNode[];
}

export interface RagPipelineTemplate {
  description?: string;
  displayName: string;
  pipeline: RagProfilePipeline;
  templateId: string;
}

export interface RagPipelineStrategyDefinition {
  configSchema?: Record<string, unknown>;
  displayName?: string;
  requiredCapabilities?: string[];
  requiredStorageSegments?: string[];
  strategyId: string;
}

export interface RagPipelineStageOrchestration {
  actsAsBarrier?: boolean;
  mustRunAfterStageTypes: string[];
  mustRunBeforeStageTypes: string[];
  required?: boolean;
}

export interface RagPipelineStageDefinition {
  allowedStrategies: string[];
  defaultFailurePolicy?: string;
  defaultJoinPolicy?: string;
  displayName?: string;
  inputArtifactKeys: string[];
  orchestration?: RagPipelineStageOrchestration;
  outputArtifactKeys: string[];
  parallelSupport?: boolean;
  profileTypes: Array<"INGEST" | "RETRIEVAL">;
  repeatable?: boolean;
  stageType: string;
  strategies: RagPipelineStrategyDefinition[];
}

export interface RagPipelineCatalog {
  failurePolicies: string[];
  joinPolicies: string[];
  profileType: "INGEST" | "RETRIEVAL";
  stages: RagPipelineStageDefinition[];
  templates: RagPipelineTemplate[];
}

export interface RagPipelineStageResult {
  durationMs?: number;
  inputArtifactKeys: string[];
  nodeId: string;
  outputArtifactKeys: string[];
  stageType: string;
  status: string;
  strategyId: string;
  warnings: string[];
}

export interface RagIngestStageConfig {
  enabled: boolean;
  id: RagIngestStageId;
}

export interface RagIngestStageBinding {
  config?: Record<string, unknown>;
  strategyId: string;
}

export interface RagIngestProfileConfig {
  dag: {
    edges: Array<{ from: RagIngestStageId; to: RagIngestStageId }>;
    nodes: RagIngestStageId[];
  };
  embeddingModelCode: string;
  embeddingVersion: string;
  graphWritePolicy: string;
  requiredSourceVersion: string;
  sourceInputPolicy: {
    supportedSourceTypes: string[];
  };
  stages: Partial<Record<RagIngestStageId, RagIngestStageBinding>>;
}

export interface RagRetrievalRouteConfig {
  corpusId: string;
  enabled: boolean;
  routeId?: string;
  routeType: RagRetrievalRouteType;
  strategyId: string;
}

export interface RagRetrievalProfileFilter {
  field: string;
  operator: string;
  value: string;
}

export interface RagRetrievalProfileConfig {
  contextStrategies: string[];
  dedupPolicy: string;
  filters: RagRetrievalProfileFilter[];
  fusionPolicy: string;
  graphExpansionPolicy: string;
  rerankEnabled: boolean;
  rerankModelCode: string;
  routesConfig: RagRetrievalRouteConfig[];
  topicPolicy: string;
}

export interface RagProfileRecordBase {
  corpus: string;
  corpusIds: string[];
  description: string;
  id: string;
  pipeline?: RagProfilePipeline;
  primaryCorpusId: string;
  routes: string[];
  status: RagProfileStatus;
  strategy: string;
  type: RagProfileType;
}

export interface IngestProfileRecord extends RagProfileRecordBase {
  config: RagIngestProfileConfig;
  type: "Ingest";
}

export interface RetrievalProfileRecord extends RagProfileRecordBase {
  config: RagRetrievalProfileConfig;
  type: "Retrieval";
}

export type ProfileRecord = IngestProfileRecord | RetrievalProfileRecord;

export interface SourceRecord {
  code: string;
  corpus: string;
  indexedAt: string;
  owner: string;
  sourceType: string;
  status: "Active" | "Indexing" | "Warning";
  textUnits: number;
  version: string;
  warnings: number;
}

export interface TextUnitRecord {
  corpus: string;
  embeddingStatus: "Indexed" | "Pending" | "Skipped";
  id: string;
  preview: string;
  source: string;
  tokens: number;
}

export interface WarningRecord {
  age: string;
  corpus: string;
  message: string;
  severity: "Low" | "Medium" | "High";
  source: string;
  stage: string;
}

export interface TraceRecord {
  age: string;
  hits: number;
  id: string;
  profile: string;
  query: string;
  route: string;
  tokens: number;
}

export const ragTabs = [
  { label: "语料库", value: "corpora" },
  { label: "配置档案", value: "profiles" },
  { label: "调试台", value: "workbench" },
  { label: "数据", value: "data" },
  { label: "诊断", value: "diagnostics" },
] as const;

export const dataViews = [
  { label: "来源", value: "sources" },
  { label: "文本单元", value: "textUnits" },
  { label: "告警", value: "warnings" },
  { label: "检索追踪", value: "traces" },
] as const;

export const pipelineStages = ["source", "parse", "normalize", "split", "extractTree", "summarizeTree", "persist", "fulltextIndex", "vectorIndex", "treeFulltextIndex", "treeVectorIndex"] as const;

const storageSegmentOrder: RagStorageSegmentKey[] = ["source", "textUnit", "fulltext", "vector", "tree", "diagnostics"];
const storageSegmentLabels: Record<RagStorageSegmentKey, string> = {
  diagnostics: "诊断",
  fulltext: "全文",
  source: "来源",
  textUnit: "文本单元",
  tree: "摘要树",
  vector: "向量",
};

export function isTargetedStorageBinding(binding: unknown): binding is RagTargetedStorageBinding {
  if (!binding || typeof binding !== "object") return false;
  const candidate = binding as Partial<RagTargetedStorageBinding>;
  return candidate.mode === "TARGETED" && Array.isArray(candidate.targets) && Boolean(candidate.bindings);
}

export function normalizeRagStorageBinding(binding: RagStorageBinding): RagTargetedStorageBinding {
  if (!isTargetedStorageBinding(binding)) {
    throw new Error("RAG storage binding must use TARGETED mode with bindings and targets.");
  }
  return binding;
}

export function formatRagStorageTarget(target?: RagStorageTargetBinding) {
  if (!target) return "未绑定";
  if (target.backend === "GRAPH") {
    return `graph.${target.namespace ?? target.nodeLabel ?? "未绑定"}/${target.nodeLabel ?? "节点"}`;
  }
  return `sqlite.${target.table ?? "未绑定"}`;
}

export function formatRagStorageBinding(binding: RagStorageBinding) {
  const normalizedBinding = normalizeRagStorageBinding(binding);
  const targetMap = new Map(normalizedBinding.targets.map((target) => [target.id, target]));
  return storageSegmentOrder
    .map((segmentKey) => `${storageSegmentLabels[segmentKey]}=${formatRagStorageTarget(targetMap.get(normalizedBinding.bindings[segmentKey] ?? ""))}`)
    .join(" | ");
}
