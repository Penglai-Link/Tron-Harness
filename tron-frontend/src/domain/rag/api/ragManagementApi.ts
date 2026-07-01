import { requestApi } from "@/shared/api/httpClient";

import {
  buildRagCorpusConfigFromForm,
  type RagCorpusFormState,
} from "@/domain/rag/model/ragCorpusForm";
import {
  formatRagStorageBinding,
  type CorpusRecord,
  type IngestProfileRecord,
  type RagCorpusConfig,
  type RagIngestStageId,
  type ProfileRecord,
  type RagCorpusCapabilities,
  type RagCorpusFieldMapping,
  type RagCorpusFilterSchema,
  type RagProfileStatus,
  type RagPipelineCatalog,
  type RagPipelineEdge,
  type RagPipelineNode,
  type RagPipelineStageDefinition,
  type RagPipelineStageResult,
  type RagPipelineTemplate,
  type RagProfilePipeline,
  type RagRetrievalRouteConfig,
  type RagStorageBinding,
  type RetrievalProfileRecord,
} from "@/domain/rag/model/RagModel";
import {
  buildIngestPipelineFromForm,
  buildIngestProfileRecordFromForm,
  buildRetrievalPipelineFromForm,
  buildRetrievalProfileRecordFromForm,
  inferRetrievalRouteType,
  ingestStageIdToStageType,
  ingestStageNodeId,
  ingestStageTypeToStageId,
  retrievalNodeId,
  type RagIngestProfileFormState,
  type RagRetrievalProfileFormState,
} from "@/domain/rag/model/ragProfileForm";

export const ragManagementApiEndpoints = {
  createCorpus: "POST /api/v1/rag/corpora",
  createProfile: "POST /api/v1/rag/profiles",
  deleteCorpus: "DELETE /api/v1/rag/corpora/{corpusId}",
  deleteProfile: "DELETE /api/v1/rag/profiles/{profileId}",
  getCorpusCatalog: "GET /api/v1/rag/corpora/catalog",
  getCorpusDetail: "GET /api/v1/rag/corpora/{corpusId}",
  getPipelineCatalog: "GET /api/v1/rag/pipeline/catalog",
  getProfileCatalog: "GET /api/v1/rag/profiles/catalog",
  getProfileDetail: "GET /api/v1/rag/profiles/{profileId}",
  listCorpora: "GET /api/v1/rag/corpora",
  listProfiles: "GET /api/v1/rag/profiles",
  runIngest: "POST /api/v1/rag/ingest",
  runSearch: "POST /api/v1/rag/search",
  updateCorpus: "PUT /api/v1/rag/corpora/{corpusId}",
  updateProfile: "PUT /api/v1/rag/profiles/{profileId}",
  validateFilters: "POST /api/v1/rag/filters/validate",
  verifyCorpus: "POST /api/v1/rag/corpora/verify",
  verifyProfile: "POST /api/v1/rag/profiles/verify",
} as const;

export interface RagManagementValidationIssue {
  code?: string;
  field?: string;
  message?: string;
  path?: string;
}

export interface RagCorpusSummaryApi {
  capabilities?: Partial<RagCorpusCapabilities>;
  corpusId: string;
  description?: string;
  displayName?: string;
  ingestProfileId?: string;
  metadataGroups?: Array<Record<string, unknown>>;
  retrievalProfileId?: string;
  status?: string;
  storageSummary?: string;
}

export interface RagCorpusDetailApi extends RagCorpusSummaryApi {
  fieldMapping?: RagCorpusFieldMapping;
  filterSchema?: RagCorpusFilterSchema;
  runtimeProjection?: {
    compatible?: boolean;
    issues?: string[];
    storageBinding?: RagStorageBinding;
  };
  storageBinding?: RagStorageBinding;
}

export interface RagCorpusCatalogApi {
  capabilityRules?: unknown[];
  codeStrategies?: string[];
  compatibilityRules?: unknown[];
  contractFields?: {
    source?: Array<{ column?: string; logicalName?: string }>;
    textUnit?: Array<{ column?: string; logicalName?: string }>;
  };
  filterFieldTypes?: string[];
  filterOperators?: string[];
  storageBackends?: string[];
}

export interface RagCorpusVerifyResultApi {
  errors?: RagManagementValidationIssue[];
  runtimeProjection?: RagCorpusDetailApi["runtimeProjection"];
  valid?: boolean;
}

export interface RagCorpusDeleteResultApi {
  corpusId?: string;
  deletedIngestProfileIds?: string[];
}

export interface RagProfilePipelineStageApi {
  config?: Record<string, unknown>;
  enabled?: boolean;
  id?: string;
  strategyId?: string;
}

export interface RagProfileRouteApi {
  corpusId?: string;
  enabled?: boolean;
  routeId?: string;
  routeType?: string;
  strategyId?: string;
}

export type RagProfilePipelineApi = RagProfilePipeline;
export type RagPipelineCatalogApi = RagPipelineCatalog;

export interface RagProfileFilterApi {
  field?: string;
  operator?: string;
  value?: string;
}

export interface RagProfileDetailApi {
  contextStrategies?: string[];
  corpusIds?: string[];
  dedupPolicy?: string;
  degradePolicy?: string;
  description?: string;
  embeddingModelCode?: string;
  embeddingVersion?: string;
  filters?: RagProfileFilterApi[];
  fusionPolicy?: string;
  graphExpansionPolicy?: string;
  graphWritePolicy?: string;
  pipeline?: RagProfilePipelineApi;
  pipelineStages?: RagProfilePipelineStageApi[];
  primaryCorpusId?: string;
  profileId: string;
  profileType?: string;
  rerankEnabled?: boolean;
  rerankModelCode?: string;
  requiredSourceVersion?: string;
  routes?: RagProfileRouteApi[];
  status?: string;
  topicPolicy?: string;
}

export interface RagProfileCatalogCorpusApi {
  capabilities?: Partial<RagCorpusCapabilities>;
  corpusId: string;
  displayName?: string;
  filterSchema?: RagCorpusFilterSchema;
  status?: string;
  storageSummary?: string;
}

export interface RagProfileCatalogApi {
  contextStrategies?: string[];
  corpora?: RagProfileCatalogCorpusApi[];
  dedupPolicyTypes?: string[];
  fusionPolicyTypes?: string[];
  graphExpansionPolicies?: string[];
  ingestStageDefinitions?: Array<{ id?: string; label?: string } | string>;
  ingestStageStrategies?: Array<{ corpusId?: string; stageId?: string; strategyIds?: string[] }> | Record<string, string[]>;
  retrievalRouteTypes?: Array<{ corpusId?: string; routeTypes?: string[] }> | string[];
  retrievalStrategies?: Array<{ corpusId?: string; routeType?: string; strategyIds?: string[] }> | Record<string, string[]>;
  topicPolicyTypes?: string[];
}

export interface RagProfileVerifyResultApi {
  errors?: RagManagementValidationIssue[];
  valid?: boolean;
}

export interface RagProfileDeleteResultApi {
  profileId?: string;
}

export interface RagFilterValidateInputApi {
  corpusId: string;
  filters: Record<string, unknown>;
}

export interface RagFilterValidateResultApi {
  fieldErrors?: RagManagementValidationIssue[];
  normalizedFilters?: Record<string, unknown>;
  valid?: boolean;
}

export interface RagRuntimeAccessContextApi {
  agentCode: string;
  userCode: string;
  workspaceCode: string;
}

export interface RagRuntimeScopeApi {
  scopeCode: string;
  scopeType: string;
}

export interface RagIngestRequestApi {
  accessContext: RagRuntimeAccessContextApi;
  corpusId: string;
  ingestProfileId: string;
  payload: Record<string, unknown>;
  targetScope: RagRuntimeScopeApi;
}

export interface RagIngestResponseApi {
  corpusId?: string;
  embeddingStatus?: string;
  fulltextStatus?: string;
  profileId?: string;
  sourceCode?: string;
  sourceVersion?: string;
  stageResults?: RagPipelineStageResult[];
  status?: string;
  textUnitCount?: number;
  warnings?: unknown[];
}

export interface RagSearchRequestApi {
  accessContext: RagRuntimeAccessContextApi;
  context: {
    graphExpansion?: {
      relationships: string[];
    };
    returnFields: string[];
    strategy: string;
  };
  filters: Record<string, unknown>;
  limit: number;
  profileId: string;
  query: string;
  tokenBudget: number;
  trace: {
    enabled: boolean;
  };
}

export interface RagSearchHitApi {
  [field: string]: unknown;
  corpusId?: string;
  fields?: Record<string, unknown>;
  metadata?: Record<string, unknown>;
  score?: number;
  sourceCode?: string;
  sourceVersion?: string;
  text?: string;
  title?: string;
}

export interface RagSearchResponseApi {
  contextPack?: {
    items?: Array<{
      metadata?: Record<string, unknown>;
      sourceCode?: string;
      text?: string;
      tokens?: number;
    }>;
    tokenCount?: number;
  } | null;
  contextStrategy?: string;
  hits?: RagSearchHitApi[];
  profileId?: string;
  query?: string;
  stageResults?: RagPipelineStageResult[];
  traceCode?: string | null;
  warnings?: unknown[];
}

export interface RagProfileCatalogNormalized {
  contextStrategies: string[];
  corpora: CorpusRecord[];
  dedupPolicyTypes: string[];
  fusionPolicyTypes: string[];
  graphExpansionPolicies: string[];
  pipelineCatalog?: RagPipelineCatalog;
  ingestStageDefinitions: Array<{ id: string; label: string }>;
  pipelineStageDefinitions: RagPipelineStageDefinition[];
  failurePolicies: string[];
  joinPolicies: string[];
  stageStrategiesByCorpus: Record<string, Record<string, string[]>>;
  topicPolicyTypes: string[];
  routeTypesByCorpus: Record<string, string[]>;
  routeStrategiesByCorpus: Record<string, Record<string, string[]>>;
}

export interface RagRetrievalProfileHiddenFields {
  degradePolicy: string;
  requiredSourceVersion: string;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function readString(value: unknown) {
  return typeof value === "string" ? value : "";
}

function readStringArray(value: unknown) {
  return Array.isArray(value)
    ? value.filter((item): item is string => typeof item === "string").map((item) => item.trim()).filter(Boolean)
    : [];
}

function readStrategyRecord(value: unknown) {
  if (!isRecord(value)) return {};
  return Object.fromEntries(
    Object.entries(value)
      .map(([key, strategyIds]) => [key.trim(), readStringArray(strategyIds)] as const)
      .filter(([key]) => Boolean(key)),
  );
}

function distributeGlobalStrategies(
  strategyMap: Record<string, string[]>,
  corpusIds: string[],
) {
  const entries = Object.entries(strategyMap);
  if (entries.length === 0) return {};
  return Object.fromEntries(
    corpusIds.map((corpusId) => [
      corpusId,
      Object.fromEntries(entries.map(([key, strategyIds]) => [key, strategyIds])),
    ]),
  ) as Record<string, Record<string, string[]>>;
}

function normalizeIngestStageDefinitions(value: unknown) {
  if (!Array.isArray(value)) return [];
  return value
    .map((row) => {
      if (typeof row === "string") {
        const id = row.trim();
        return { id, label: id };
      }
      if (!isRecord(row)) return { id: "", label: "" };
      return { id: readString(row.id), label: readString(row.label) || readString(row.id) };
    })
    .filter((row) => row.id);
}

function normalizeCorpusCapabilities(value: RagCorpusSummaryApi["capabilities"]): RagCorpusCapabilities {
  const record = isRecord(value) ? value : {};
  return {
    fulltext: Boolean(record.fulltext),
    graph: Boolean(record.graph),
    tree: Boolean(record.tree),
    vector: Boolean(record.vector),
  };
}

function normalizeCorpusStatus(status: string): CorpusRecord["status"] {
  const normalized = status.trim().toLowerCase();
  if (normalized === "reserved") return "Reserved";
  if (normalized === "disabled") return "Disabled";
  if (normalized === "indexing") return "Indexing";
  if (normalized === "warning") return "Warning";
  return "Active";
}

function normalizeCorpusStatusCode(status: string): RagCorpusConfig["status"] {
  const normalized = status.trim().toLowerCase();
  if (normalized === "reserved" || normalized === "disabled") return normalized;
  return "enabled";
}

function defaultProfileStatus(value: string): RagProfileStatus {
  const normalized = value.trim().toLowerCase();
  if (normalized === "enabled" || normalized === "ready") return "Ready";
  if (normalized === "reserved" || normalized === "needs review" || normalized === "needs_review") return "Needs Review";
  return "Draft";
}

function profileStatusToApiCode(status: RagProfileStatus) {
  if (status === "Ready") return "enabled";
  if (status === "Needs Review") return "reserved";
  return "disabled";
}

function summarizeIngestStrategy(pipelineStages: RagProfilePipelineStageApi[]) {
  return pipelineStages.find((stage) => stage.enabled && stage.strategyId)?.strategyId ?? "";
}

function summarizeRetrievalStrategy(profile: RagProfileDetailApi) {
  const parts = [profile.fusionPolicy ?? ""];
  const contextStrategy = readStringArray(profile.contextStrategies)[0];
  if (contextStrategy) parts.push(contextStrategy);
  if (readString(profile.graphExpansionPolicy)) parts.push("GRAPH_EXPANSION");
  return parts.filter(Boolean).join(" + ");
}

function summarizeRetrievalRoutes(routes: RagRetrievalRouteConfig[]) {
  return routes.filter((route) => route.enabled !== false).map((route) => route.strategyId);
}

function isRetrievalRoutePipelineNode(node: RagPipelineNode) {
  if (readString(node.stageType) === "ROUTE_SEARCH") return true;
  const config = isRecord(node.config) ? node.config : {};
  return Boolean(readString(config.corpusId) && readString(node.strategyId));
}

function retrievalRouteTypeFromPipelineNode(node: RagPipelineNode) {
  const config = isRecord(node.config) ? node.config : {};
  const configuredRouteType = readString(config.routeType);
  if (configuredRouteType) return configuredRouteType;
  const stageType = readString(node.stageType);
  return stageType && stageType !== "ROUTE_SEARCH" ? stageType : inferRetrievalRouteType(readString(node.strategyId));
}

function sanitizeRouteIdPart(value: string) {
  return value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9_-]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

function derivedRetrievalRouteId(route: { corpusId?: string; routeId?: string; routeType?: string }, index: number) {
  return retrievalNodeId(route, index);
}

function readPipelineNode(value: unknown): RagPipelineNode | undefined {
  if (!isRecord(value)) return undefined;
  const nodeId = readString(value.nodeId);
  const stageType = readString(value.stageType);
  if (!nodeId || !stageType) return undefined;
  return {
    config: isRecord(value.config) ? value.config : undefined,
    enabled: value.enabled === undefined ? true : Boolean(value.enabled),
    failurePolicy: readString(value.failurePolicy) || undefined,
    joinPolicy: readString(value.joinPolicy) || undefined,
    nodeId,
    stageType,
    strategyId: readString(value.strategyId),
  };
}

function readPipelineEdge(value: unknown): RagPipelineEdge | undefined {
  if (!isRecord(value)) return undefined;
  const from = readString(value.from);
  const to = readString(value.to);
  return from && to ? { from, to } : undefined;
}

function readProfilePipeline(value: unknown): RagProfilePipeline | undefined {
  if (!isRecord(value)) return undefined;
  const nodes = Array.isArray(value.nodes)
    ? value.nodes.map(readPipelineNode).filter((node): node is RagPipelineNode => Boolean(node))
    : [];
  const edges = Array.isArray(value.edges)
    ? value.edges.map(readPipelineEdge).filter((edge): edge is RagPipelineEdge => Boolean(edge))
    : [];
  if (nodes.length === 0 && edges.length === 0) return undefined;
  return {
    edges,
    mode: "DAG",
    nodes,
  };
}

function readPipelineTemplate(value: unknown): RagPipelineTemplate | undefined {
  if (!isRecord(value)) return undefined;
  const templateId = readString(value.templateId);
  const pipeline = readProfilePipeline(value.pipeline);
  if (!templateId || !pipeline) return undefined;
  return {
    description: readString(value.description),
    displayName: readString(value.displayName) || templateId,
    pipeline,
    templateId,
  };
}

function pipelineEdgesFromNodeIds(nodeIds: string[]) {
  return nodeIds.slice(0, -1).map((nodeId, index) => ({ from: nodeId, to: nodeIds[index + 1] }));
}

function pipelineFromLegacyIngestStages(pipelineStages: RagProfilePipelineStageApi[]): RagProfilePipeline {
  const nodes = pipelineStages
    .filter((stage) => stage.enabled)
    .map<RagPipelineNode>((stage) => {
      const stageId = readString(stage.id) as RagIngestStageId;
      const nodeId = ingestStageNodeId(stageId);
      return {
        config: isRecord(stage.config) ? stage.config : undefined,
        enabled: true,
        failurePolicy: "FAIL_PIPELINE",
        joinPolicy: "ALL_SUCCESS",
        nodeId,
        stageType: ingestStageIdToStageType(stageId),
        strategyId: readString(stage.strategyId),
      };
    });
  return {
    edges: pipelineEdgesFromNodeIds(nodes.map((node) => node.nodeId)),
    mode: "DAG",
    nodes,
  };
}

function pipelineFromLegacyRetrievalRoutes(routes: RagProfileRouteApi[]): RagProfilePipeline {
  const nodes = routes
    .filter((route) => route.enabled !== false)
    .map<RagPipelineNode>((route, index) => ({
      config: {
        corpusId: readString(route.corpusId),
        routeType: readString(route.routeType) || inferRetrievalRouteType(readString(route.strategyId)),
      },
      enabled: true,
      failurePolicy: "CONTINUE_WITH_WARNING",
      joinPolicy: "BEST_EFFORT",
      nodeId: derivedRetrievalRouteId(route, index),
      stageType: "ROUTE_SEARCH",
      strategyId: readString(route.strategyId),
    }));
  return {
    edges: pipelineEdgesFromNodeIds(nodes.map((node) => node.nodeId)),
    mode: "DAG",
    nodes,
  };
}

function fallbackCorpusFilterSchema(): RagCorpusFilterSchema {
  return { fields: {} };
}

function fallbackCorpusFieldMapping(): RagCorpusFieldMapping {
  return { source: {}, textUnit: {} };
}

export function normalizeRagValidationIssues(errors: unknown): RagManagementValidationIssue[] {
  if (!Array.isArray(errors)) return [];
  return errors
    .filter(isRecord)
    .map((entry) => ({
      code: readString(entry.code) || undefined,
      field: readString(entry.field) || undefined,
      message: readString(entry.message) || undefined,
      path: readString(entry.path) || undefined,
    }));
}

export function validationIssueMessage(issue: RagManagementValidationIssue) {
  return issue.message ?? issue.code ?? "Request validation failed.";
}

export function validationIssueField(issue: RagManagementValidationIssue) {
  return issue.field ?? issue.path ?? "";
}

export function mapRagCorpusSummaryToRecord(payload: RagCorpusSummaryApi): CorpusRecord {
  const capabilities = normalizeCorpusCapabilities(payload.capabilities);
  return {
    code: payload.corpusId,
    description: readString(payload.description),
    filters: [],
    fulltextHealth: "Healthy",
    ingestProfile: readString(payload.ingestProfileId),
    lastIndexed: "API",
    name: readString(payload.displayName) || payload.corpusId,
    retrievalProfile: readString(payload.retrievalProfileId),
    scope: "management",
    sources: 0,
    status: normalizeCorpusStatus(readString(payload.status)),
    storageBinding: readString(payload.storageSummary) || "API managed",
    strategies: [
      capabilities.fulltext ? "FULLTEXT" : "",
      capabilities.vector ? "VECTOR" : "",
      capabilities.tree ? "TREE_FULLTEXT" : "",
      capabilities.tree ? "TREE_VECTOR" : "",
      capabilities.graph ? "GRAPH" : "",
    ].filter(Boolean),
    textUnits: 0,
    vectorCoverage: capabilities.vector ? 100 : 0,
    warnings: 0,
  };
}

export function mapRagCorpusDetailToRecord(payload: RagCorpusDetailApi): CorpusRecord {
  const capabilities = normalizeCorpusCapabilities(payload.capabilities);
  return {
    ...mapRagCorpusSummaryToRecord(payload),
    config: payload.storageBinding
      ? {
          capabilities,
          corpusId: payload.corpusId,
          description: readString(payload.description),
          displayName: readString(payload.displayName) || payload.corpusId,
          fieldMapping: payload.fieldMapping ?? fallbackCorpusFieldMapping(),
          filterSchema: payload.filterSchema ?? fallbackCorpusFilterSchema(),
          status: normalizeCorpusStatusCode(readString(payload.status)),
          storageBinding: payload.storageBinding,
        }
      : undefined,
    filters: Object.keys(payload.filterSchema?.fields ?? {}),
    storageBinding: readString(payload.storageSummary)
      || (payload.storageBinding ? formatRagStorageBinding(payload.storageBinding) : "API managed"),
    warnings: payload.runtimeProjection?.issues?.length ?? 0,
  };
}

function buildProfileCorpusIds(primaryCorpusId: string, corpusIds: string[], extraCorpusIds: string[]) {
  return Array.from(new Set([primaryCorpusId, ...corpusIds, ...extraCorpusIds].filter(Boolean)));
}

export function mapRagProfileDetailToRecord(payload: RagProfileDetailApi): ProfileRecord {
  const profileType = readString(payload.profileType).toUpperCase() === "INGEST" ? "Ingest" : "Retrieval";
  const primaryCorpusId = readString(payload.primaryCorpusId);
  const corpusIds = readStringArray(payload.corpusIds);

  if (profileType === "Ingest") {
    const pipelineStages = Array.isArray(payload.pipelineStages) ? payload.pipelineStages : [];
    const pipeline = readProfilePipeline(payload.pipeline) ?? pipelineFromLegacyIngestStages(pipelineStages);
    const enabledStageIds = pipeline.nodes
      .filter((node) => node.enabled)
      .map((node) => ingestStageTypeToStageId(node.stageType))
      .filter((stageId): stageId is RagIngestStageId => Boolean(stageId));
    const stages = Object.fromEntries(
      pipeline.nodes
        .map((node) => [ingestStageTypeToStageId(node.stageType), node] as const)
        .filter(([stageId]) => Boolean(stageId))
        .map(([stageId, node]) => [stageId, {
          ...(isRecord(node.config) ? { config: node.config } : {}),
          strategyId: readString(node.strategyId),
        }]),
    );
    const record: IngestProfileRecord = {
      config: {
        dag: {
          edges: enabledStageIds.slice(0, -1).map((stageId, index) => ({ from: stageId, to: enabledStageIds[index + 1] })),
          nodes: enabledStageIds,
        },
        embeddingModelCode: readString(payload.embeddingModelCode),
        embeddingVersion: readString(payload.embeddingVersion),
        graphWritePolicy: readString(payload.graphWritePolicy),
        requiredSourceVersion: readString(payload.requiredSourceVersion) || "v1",
        stages,
      },
      corpus: primaryCorpusId,
      corpusIds: buildProfileCorpusIds(primaryCorpusId, corpusIds, []),
      description: readString(payload.description),
      id: payload.profileId,
      pipeline,
      primaryCorpusId,
      routes: enabledStageIds,
      status: defaultProfileStatus(readString(payload.status)),
      strategy: pipeline.nodes.find((node) => node.enabled && node.strategyId)?.strategyId ?? summarizeIngestStrategy(pipelineStages),
      type: "Ingest",
    };
    return record;
  }

  const pipeline = readProfilePipeline(payload.pipeline) ?? pipelineFromLegacyRetrievalRoutes(payload.routes ?? []);
  const routesConfig: RagRetrievalRouteConfig[] = pipeline.nodes
    .filter(isRetrievalRoutePipelineNode)
    .map((node, index) => {
      const config = isRecord(node.config) ? node.config : {};
      const strategyId = readString(node.strategyId);
      return {
        corpusId: readString(config.corpusId),
        enabled: node.enabled,
        routeId: node.nodeId || `route-${index + 1}`,
        routeType: retrievalRouteTypeFromPipelineNode(node) as RagRetrievalRouteConfig["routeType"],
        strategyId,
      };
    });
  const routeCorpusIds = routesConfig.map((route) => route.corpusId).filter(Boolean);
  const record: RetrievalProfileRecord = {
    config: {
      contextStrategies: readStringArray(payload.contextStrategies),
      dedupPolicy: readString(payload.dedupPolicy),
      filters: (payload.filters ?? []).map((filter) => ({
        field: readString(filter.field),
        operator: readString(filter.operator),
        value: readString(filter.value),
      })),
      fusionPolicy: readString(payload.fusionPolicy),
      graphExpansionPolicy: readString(payload.graphExpansionPolicy),
      rerankEnabled: Boolean(payload.rerankEnabled),
      rerankModelCode: readString(payload.rerankModelCode),
      routesConfig,
      topicPolicy: readString(payload.topicPolicy),
    },
    corpus: primaryCorpusId,
    corpusIds: buildProfileCorpusIds(primaryCorpusId, corpusIds, routeCorpusIds),
    description: readString(payload.description),
    id: payload.profileId,
    pipeline,
    primaryCorpusId,
    routes: summarizeRetrievalRoutes(routesConfig),
    status: defaultProfileStatus(readString(payload.status)),
    strategy: summarizeRetrievalStrategy(payload),
    type: "Retrieval",
  };
  return record;
}

function mapCatalogCorpusToRecord(payload: RagProfileCatalogCorpusApi): CorpusRecord {
  return {
    ...mapRagCorpusSummaryToRecord({
      corpusId: payload.corpusId,
      capabilities: payload.capabilities,
      displayName: payload.displayName,
      status: payload.status,
      storageSummary: payload.storageSummary,
    }),
    config: payload.filterSchema
      ? {
          capabilities: normalizeCorpusCapabilities(payload.capabilities),
          corpusId: payload.corpusId,
          description: "",
          displayName: readString(payload.displayName) || payload.corpusId,
          fieldMapping: fallbackCorpusFieldMapping(),
          filterSchema: payload.filterSchema,
          status: "enabled",
          storageBinding: {
            bindings: {},
            mode: "TARGETED",
            targets: [],
          },
        }
      : undefined,
    filters: Object.keys(payload.filterSchema?.fields ?? {}),
  };
}

export function normalizeRagProfileCatalog(payload: RagProfileCatalogApi): RagProfileCatalogNormalized {
  const corpora = Array.isArray(payload.corpora)
    ? payload.corpora.map(mapCatalogCorpusToRecord)
    : [];
  const corpusIds = corpora.map((corpus) => corpus.code).filter(Boolean);

  const stageStrategiesByCorpus: Record<string, Record<string, string[]>> = {};
  if (Array.isArray(payload.ingestStageStrategies)) {
    payload.ingestStageStrategies.filter(isRecord).forEach((row) => {
      const corpusId = readString(row.corpusId);
      const stageId = readString(row.stageId);
      if (!corpusId || !stageId) return;
      stageStrategiesByCorpus[corpusId] ??= {};
      stageStrategiesByCorpus[corpusId][stageId] = readStringArray(row.strategyIds);
    });
  } else {
    Object.assign(stageStrategiesByCorpus, distributeGlobalStrategies(
      readStrategyRecord(payload.ingestStageStrategies),
      corpusIds,
    ));
  }

  const routeTypesByCorpus: Record<string, string[]> = {};
  if (Array.isArray(payload.retrievalRouteTypes)) {
    const globalRouteTypes = payload.retrievalRouteTypes.filter((row): row is string => typeof row === "string");
    if (globalRouteTypes.length > 0) {
      const routeTypes = readStringArray(globalRouteTypes);
      corpusIds.forEach((corpusId) => {
        routeTypesByCorpus[corpusId] = routeTypes;
      });
    }
    payload.retrievalRouteTypes.filter(isRecord).forEach((row) => {
      const corpusId = readString(row.corpusId);
      if (!corpusId) return;
      routeTypesByCorpus[corpusId] = readStringArray(row.routeTypes);
    });
  }

  const routeStrategiesByCorpus: Record<string, Record<string, string[]>> = {};
  if (Array.isArray(payload.retrievalStrategies)) {
    payload.retrievalStrategies.filter(isRecord).forEach((row) => {
      const corpusId = readString(row.corpusId);
      const routeType = readString(row.routeType);
      if (!corpusId || !routeType) return;
      routeStrategiesByCorpus[corpusId] ??= {};
      routeStrategiesByCorpus[corpusId][routeType] = readStringArray(row.strategyIds);
    });
  } else {
    Object.assign(routeStrategiesByCorpus, distributeGlobalStrategies(
      readStrategyRecord(payload.retrievalStrategies),
      corpusIds,
    ));
  }

  return {
    contextStrategies: readStringArray(payload.contextStrategies),
    corpora,
    dedupPolicyTypes: readStringArray(payload.dedupPolicyTypes),
    failurePolicies: [],
    fusionPolicyTypes: readStringArray(payload.fusionPolicyTypes),
    graphExpansionPolicies: readStringArray(payload.graphExpansionPolicies),
    ingestStageDefinitions: normalizeIngestStageDefinitions(payload.ingestStageDefinitions),
    joinPolicies: [],
    pipelineStageDefinitions: [],
    routeStrategiesByCorpus,
    routeTypesByCorpus,
    stageStrategiesByCorpus,
    topicPolicyTypes: readStringArray(payload.topicPolicyTypes),
  };
}

function readPipelineStrategyDefinition(value: unknown) {
  if (!isRecord(value)) return undefined;
  const strategyId = readString(value.strategyId);
  if (!strategyId) return undefined;
  return {
    configSchema: isRecord(value.configSchema) ? value.configSchema : undefined,
    displayName: readString(value.displayName) || strategyId,
    requiredCapabilities: readStringArray(value.requiredCapabilities),
    requiredStorageSegments: readStringArray(value.requiredStorageSegments),
    strategyId,
  };
}

function readPipelineStageOrchestration(value: unknown) {
  const source = isRecord(value) ? value : {};
  return {
    actsAsBarrier: Boolean(source.actsAsBarrier),
    mustRunAfterStageTypes: readStringArray(source.mustRunAfterStageTypes),
    mustRunBeforeStageTypes: readStringArray(source.mustRunBeforeStageTypes),
    required: Boolean(source.required),
  };
}

function readPipelineStageDefinition(value: unknown): RagPipelineStageDefinition | undefined {
  if (!isRecord(value)) return undefined;
  const stageType = readString(value.stageType);
  if (!stageType) return undefined;
  const strategies = Array.isArray(value.strategies)
    ? value.strategies.map(readPipelineStrategyDefinition).filter((strategy): strategy is NonNullable<ReturnType<typeof readPipelineStrategyDefinition>> => Boolean(strategy))
    : [];
  const allowedStrategies = readStringArray(value.allowedStrategies);
  return {
    allowedStrategies: allowedStrategies.length > 0 ? allowedStrategies : strategies.map((strategy) => strategy.strategyId),
    defaultFailurePolicy: readString(value.defaultFailurePolicy) || undefined,
    defaultJoinPolicy: readString(value.defaultJoinPolicy) || undefined,
    displayName: readString(value.displayName) || stageType,
    inputArtifactKeys: readStringArray(value.inputArtifactKeys),
    orchestration: readPipelineStageOrchestration(value.orchestration),
    outputArtifactKeys: readStringArray(value.outputArtifactKeys),
    parallelSupport: Boolean(value.parallelSupport ?? value.parallel),
    profileTypes: readStringArray(value.profileTypes)
      .map((profileType) => profileType.toUpperCase())
      .filter((profileType): profileType is "INGEST" | "RETRIEVAL" => profileType === "INGEST" || profileType === "RETRIEVAL"),
    repeatable: Boolean(value.repeatable),
    stageType,
    strategies,
  };
}

export function normalizeRagPipelineCatalog(payload: RagPipelineCatalogApi): RagPipelineCatalog {
  const profileType = readString(payload.profileType).toUpperCase() === "INGEST" ? "INGEST" : "RETRIEVAL";
  return {
    failurePolicies: readStringArray(payload.failurePolicies),
    joinPolicies: readStringArray(payload.joinPolicies),
    profileType,
    stages: Array.isArray(payload.stages)
      ? payload.stages.map(readPipelineStageDefinition).filter((stage): stage is RagPipelineStageDefinition => Boolean(stage))
      : [],
    templates: Array.isArray(payload.templates)
      ? payload.templates.map(readPipelineTemplate).filter((template): template is RagPipelineTemplate => Boolean(template))
      : [],
  };
}

export function buildRagCorpusMutationPayload(
  form: RagCorpusFormState,
  metadataGroups: Array<Record<string, unknown>>,
): RagCorpusDetailApi {
  const config = buildRagCorpusConfigFromForm(form);
  return {
    capabilities: config.capabilities,
    corpusId: config.corpusId,
    description: config.description,
    displayName: config.displayName,
    fieldMapping: config.fieldMapping,
    filterSchema: config.filterSchema,
    metadataGroups,
    status: config.status,
    storageBinding: config.storageBinding,
  };
}

export function buildRagIngestProfileMutationPayload(
  form: RagIngestProfileFormState,
  pipelineCatalog?: RagPipelineCatalog | null,
): RagProfileDetailApi {
  const record = buildIngestProfileRecordFromForm(form, pipelineCatalog);
  return {
    corpusIds: record.corpusIds,
    description: record.description,
    embeddingModelCode: record.config.embeddingModelCode,
    embeddingVersion: record.config.embeddingVersion,
    graphWritePolicy: record.config.graphWritePolicy,
    pipeline: buildIngestPipelineFromForm(form, pipelineCatalog),
    primaryCorpusId: record.primaryCorpusId,
    profileId: record.id,
    profileType: "INGEST",
    requiredSourceVersion: record.config.requiredSourceVersion,
    status: profileStatusToApiCode(record.status),
  };
}

export function buildRagRetrievalProfileMutationPayload(
  form: RagRetrievalProfileFormState,
  hidden: RagRetrievalProfileHiddenFields,
): RagProfileDetailApi {
  const record = buildRetrievalProfileRecordFromForm(form);
  return {
    contextStrategies: record.config.contextStrategies,
    corpusIds: record.corpusIds,
    dedupPolicy: record.config.dedupPolicy,
    degradePolicy: hidden.degradePolicy,
    description: record.description,
    filters: record.config.filters,
    fusionPolicy: record.config.fusionPolicy,
    graphExpansionPolicy: record.config.graphExpansionPolicy,
    primaryCorpusId: record.primaryCorpusId,
    pipeline: buildRetrievalPipelineFromForm(form),
    profileId: record.id,
    profileType: "RETRIEVAL",
    requiredSourceVersion: hidden.requiredSourceVersion,
    rerankEnabled: record.config.rerankEnabled,
    rerankModelCode: record.config.rerankModelCode,
    status: profileStatusToApiCode(record.status),
    topicPolicy: record.config.topicPolicy,
  };
}

export function fetchRagCorpora(signal?: AbortSignal) {
  return requestApi<RagCorpusSummaryApi[]>(ragManagementApiEndpoints.listCorpora, { signal });
}

export function fetchRagCorpusDetail(corpusId: string, signal?: AbortSignal) {
  return requestApi<RagCorpusDetailApi>(ragManagementApiEndpoints.getCorpusDetail, {
    pathParams: { corpusId },
    signal,
  });
}

export function fetchRagCorpusCatalog(signal?: AbortSignal) {
  return requestApi<RagCorpusCatalogApi>(ragManagementApiEndpoints.getCorpusCatalog, { signal });
}

export function verifyRagCorpus(payload: RagCorpusDetailApi) {
  return requestApi<RagCorpusVerifyResultApi>(ragManagementApiEndpoints.verifyCorpus, {
    body: payload,
  });
}

export function createRagCorpus(payload: RagCorpusDetailApi) {
  return requestApi<RagCorpusDetailApi>(ragManagementApiEndpoints.createCorpus, {
    body: payload,
  });
}

export function updateRagCorpus(corpusId: string, payload: RagCorpusDetailApi) {
  return requestApi<RagCorpusDetailApi>(ragManagementApiEndpoints.updateCorpus, {
    body: payload,
    pathParams: { corpusId },
  });
}

export function deleteRagCorpus(corpusId: string) {
  return requestApi<RagCorpusDeleteResultApi>(ragManagementApiEndpoints.deleteCorpus, {
    pathParams: { corpusId },
  });
}

export function fetchRagProfiles(signal?: AbortSignal) {
  return requestApi<RagProfileDetailApi[]>(ragManagementApiEndpoints.listProfiles, { signal });
}

export function fetchRagProfileDetail(profileId: string, signal?: AbortSignal) {
  return requestApi<RagProfileDetailApi>(ragManagementApiEndpoints.getProfileDetail, {
    pathParams: { profileId },
    signal,
  });
}

export function fetchRagProfileCatalog(profileType?: "INGEST" | "RETRIEVAL", signal?: AbortSignal) {
  return requestApi<RagProfileCatalogApi>(ragManagementApiEndpoints.getProfileCatalog, {
    queryParams: profileType ? { profileType } : undefined,
    signal,
  });
}

export function fetchRagPipelineCatalog(profileType: "INGEST" | "RETRIEVAL", signal?: AbortSignal) {
  return requestApi<RagPipelineCatalogApi>(ragManagementApiEndpoints.getPipelineCatalog, {
    queryParams: { profileType },
    signal,
  });
}

export function verifyRagProfile(payload: RagProfileDetailApi) {
  return requestApi<RagProfileVerifyResultApi>(ragManagementApiEndpoints.verifyProfile, {
    body: payload,
  });
}

export function validateRagFilters(payload: RagFilterValidateInputApi) {
  return requestApi<RagFilterValidateResultApi>(ragManagementApiEndpoints.validateFilters, {
    body: payload,
  });
}

export function createRagProfile(payload: RagProfileDetailApi) {
  return requestApi<RagProfileDetailApi>(ragManagementApiEndpoints.createProfile, {
    body: payload,
  });
}

export function updateRagProfile(profileId: string, payload: RagProfileDetailApi) {
  return requestApi<RagProfileDetailApi>(ragManagementApiEndpoints.updateProfile, {
    body: payload,
    pathParams: { profileId },
  });
}

export function deleteRagProfile(profileId: string) {
  return requestApi<RagProfileDeleteResultApi>(ragManagementApiEndpoints.deleteProfile, {
    pathParams: { profileId },
  });
}

export function runRagIngest(payload: RagIngestRequestApi) {
  return requestApi<RagIngestResponseApi>(ragManagementApiEndpoints.runIngest, {
    body: payload,
  });
}

export function runRagSearch(payload: RagSearchRequestApi) {
  return requestApi<RagSearchResponseApi>(ragManagementApiEndpoints.runSearch, {
    body: payload,
  });
}
