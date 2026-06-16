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
    // Return URL remains the source of truth when browser storage is unavailable.
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
      <h2 className="text-base font-semibold">Validation Summary</h2>
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

function strategyOptionLabel(stageDefinition: RagPipelineStageDefinition | undefined, strategyId: string) {
  const strategy = strategyDefinitionForNode(stageDefinition, strategyId);
  if (!strategy?.displayName || strategy.displayName === strategyId) return strategyId;
  return `${strategy.displayName} · ${strategyId}`;
}

function stageDisplayName(pipelineCatalog: RagPipelineCatalog | null, stageType: string) {
  return findPipelineStageDefinition(pipelineCatalog, stageType)?.displayName || stageType;
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

  return {
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
}

function PipelineEdgesTable({ edges }: { edges: Array<{ from: string; to: string }> }) {
  return (
    <div className="overflow-x-auto rounded-xl border border-[var(--color-outline-variant)]">
      <table className="min-w-full divide-y divide-[var(--color-outline-variant)] text-left text-sm">
        <thead className="bg-[var(--color-surface-container-low)] text-[11px] uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
          <tr>
            <th className="px-4 py-3 font-semibold">From</th>
            <th className="px-4 py-3 font-semibold">To</th>
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
              <td className="px-4 py-3 text-[var(--color-on-surface-variant)]" colSpan={2}>Enable at least two connected nodes to create a DAG edge.</td>
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
      <h2 className="text-base font-semibold">Pipeline Edges</h2>
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
          <h2 className="text-base font-semibold">Pipeline Template</h2>
          <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">
            Pick a backend-declared ingest pattern first, then adjust stages only when needed.
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
                <p className="font-medium">{template.displayName}</p>
                <span className={`rounded-full border px-2 py-0.5 font-mono text-[11px] ${
                  selected
                    ? "border-[var(--color-primary)] bg-[var(--color-primary-container)] text-[var(--color-on-primary-container)]"
                    : "border-[var(--color-outline-variant)]"
                }`}>
                    {selected ? "Selected" : `${template.pipeline.nodes.length} nodes`}
                  </span>
              </div>
              {template.description ? (
                <p className="mt-2 text-sm text-[var(--color-on-surface-variant)]">{template.description}</p>
              ) : null}
              <div className="mt-3 flex flex-wrap gap-2">
                {capabilities.length > 0 ? capabilities.map((capability) => (
                  <span className={`rounded-full px-2 py-0.5 font-mono text-[11px] ${
                    selected
                      ? "bg-[var(--color-primary-container)] text-[var(--color-on-primary-container)]"
                      : "bg-[var(--color-surface)]"
                  }`} key={capability}>
                    {capability}
                  </span>
                )) : (
                  <span className="rounded-full bg-[var(--color-surface)] px-2 py-0.5 text-[11px]">basic</span>
                )}
              </div>
              {disabled ? (
                <p className="mt-3 text-[12px] text-[var(--color-error)]">Select a graph-backed corpus to use this template.</p>
              ) : null}
            </button>
          );
        }) : (
          <div className="rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4 text-sm text-[var(--color-on-surface-variant)] lg:col-span-3">
            No backend templates are available yet. Use advanced node controls below.
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
          <h2 className="text-base font-semibold">Pipeline Flow</h2>
          <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">
            The saved pipeline remains a DAG; branches in the same column can run in parallel.
          </p>
        </div>
        <span className="rounded-full border border-[var(--color-outline-variant)] px-3 py-1 text-sm">{nodeCount} enabled nodes</span>
      </div>
      {levels.length > 0 ? (
        <div className="mt-5 overflow-x-auto">
          <div className="flex min-w-max items-stretch gap-3">
            {levels.map((level, levelIndex) => (
              <div className="flex items-center gap-3" key={`level-${levelIndex}`}>
                <div className="grid min-w-52 gap-3">
                  {level.length > 1 ? (
                    <span className="w-fit rounded-full bg-[var(--color-surface-container-high)] px-2 py-0.5 text-[11px] uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
                      parallel
                    </span>
                  ) : null}
                  {level.map((node) => {
                    const stageDefinition = findPipelineStageDefinition(pipelineCatalog, node.stageType);
                    return (
                      <div className="rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-3" key={node.nodeId}>
                        <p className="font-medium">{stageDefinition?.displayName || node.stageType}</p>
                        <p className="mt-1 font-mono text-[11px] text-[var(--color-on-surface-variant)]">{node.strategyId || "No strategy"}</p>
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
          Apply a template or enable nodes to preview the DAG.
        </p>
      )}
      <details className="mt-4 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4">
        <summary className="cursor-pointer text-sm font-medium">Edge details</summary>
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
      <h2 className="text-base font-semibold">Verify Result</h2>
      <div className="mt-4 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4 text-sm">
        {result ? (
          <>
            <p className="font-medium">{result.valid === false || issues.length > 0 ? "Backend rejected the profile." : "Backend verification passed."}</p>
            {issues.length > 0 ? (
              <div className="mt-3 grid gap-2 text-[var(--color-error)]">
                {issues.map((issue, index) => (
                  <p key={`${validationIssueField(issue)}-${index}`}>{validationIssueMessage(issue)}</p>
                ))}
              </div>
            ) : null}
          </>
        ) : (
          <p className="text-[var(--color-on-surface-variant)]">Save Profile runs backend verification before mutation.</p>
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
        <summary className="cursor-pointer text-base font-semibold">Developer Preview</summary>
        <p className="mt-2 text-sm text-[var(--color-on-surface-variant)]">
          This is the exact management payload sent to the existing profile save and verify APIs.
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
      label: "Route Search",
      value: enabledRoutes.length > 0 ? `${enabledRoutes.length} enabled route${enabledRoutes.length === 1 ? "" : "s"}` : "No route selected",
    },
    { label: "Fuse Results", value: form.fusionPolicy || "SUM_SCORE" },
    { label: "Dedup Results", value: form.dedupPolicy || "TEXT_UNIT" },
    { label: "Build Context", value: contextStrategy },
  ];

  return (
    <SurfaceCard>
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="text-base font-semibold">Retrieval Flow</h2>
          <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">
            The saved profile runs route search in parallel, then applies result processing and context building.
          </p>
        </div>
        <span className="rounded-full border border-[var(--color-outline-variant)] px-3 py-1 text-sm">
          {enabledRoutes.length} route{enabledRoutes.length === 1 ? "" : "s"}
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
        <Field label="Corpus" error={error?.corpusId}>
          <SelectInput
            aria-label={`Route ${index + 1} corpus`}
            className="w-full"
            value={row.corpusId}
            onChange={(event) => onUpdate({ corpusId: event.target.value, strategyId: "" })}
          >
            <option value="">Select corpus</option>
            {corpusOptions.map((corpus) => (
              <option key={corpus.code} value={corpus.code}>{corpus.code}</option>
            ))}
          </SelectInput>
        </Field>
        <Field label="Retriever Strategy" error={error?.strategyId}>
          <SelectInput
            aria-label={`Route ${index + 1} retriever strategy`}
            className="w-full font-mono"
            value={row.strategyId}
            onChange={(event) => onUpdate({ strategyId: event.target.value })}
          >
            <option value="">{strategyOptions.length > 0 ? "Select route strategy" : "No supported strategies"}</option>
            {strategyOptions.map((strategyId) => <option key={strategyId} value={strategyId}>{strategyId}</option>)}
          </SelectInput>
        </Field>
        {canRemove ? (
          <div className="flex items-center justify-end lg:pb-1">
            <Button aria-label={`Remove route ${index + 1}`} size="icon" type="button" variant="ghost" onClick={onRemove}>
              <Trash2 size={16} aria-hidden="true" />
            </Button>
          </div>
        ) : null}
      </div>
      {row.corpusId && strategyOptions.length === 0 ? (
        <p className="text-[12px] text-[var(--color-on-surface-variant)]">
          No backend-supported retrieval strategies exist for this corpus yet.
        </p>
      ) : null}
      <ToggleSwitch checked={row.enabled} label={`Enable route ${index + 1}`} onCheckedChange={(checked) => onUpdate({ enabled: checked })} />
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
        <Field label="Field" error={error?.field}>
          <SelectInput
            aria-label={`Filter ${index + 1} field`}
            className="w-full"
            value={row.field}
            onChange={(event) => onUpdate({ field: event.target.value })}
          >
            <option value="">Select mapped field</option>
            {fieldOptions.map((field) => <option key={field} value={field}>{field}</option>)}
          </SelectInput>
        </Field>
        <Field label="Operator" error={error?.operator}>
          <SelectInput
            aria-label={`Filter ${index + 1} operator`}
            className="w-full"
            value={row.operator}
            onChange={(event) => onUpdate({ operator: event.target.value })}
          >
            {operatorOptions.map((operator) => <option key={operator} value={operator}>{operator}</option>)}
          </SelectInput>
        </Field>
        <Field label="Value" error={error?.value}>
          <TextInput
            aria-label={`Filter ${index + 1} value`}
            className="w-full font-mono"
            value={row.value}
            onChange={(event) => onUpdate({ value: event.target.value })}
          />
        </Field>
        {canRemove ? (
          <div className="flex items-center justify-end lg:pb-1">
            <Button aria-label={`Remove filter ${index + 1}`} size="icon" type="button" variant="ghost" onClick={onRemove}>
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
  const title = mode === "create" ? "Create Ingest Profile" : "Edit Ingest Profile";
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
        toast.error(error instanceof Error ? error.message : "Profile catalog could not be loaded");
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

  function updateStageStrategy(stageId: string, strategyId: string) {
    setForm((currentForm) => ({
      ...currentForm,
      pipelineStages: currentForm.pipelineStages.map((stage) => (stage.id === stageId ? { ...stage, strategyId } : stage)),
    }));
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
      toast.success(mode === "create" ? "Ingest profile created." : "Profile updated.");
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
        toast.error(validationIssueMessage(issues[0] ?? { message: "Resolve profile form errors before saving." }));
        return;
      }

      const saved = mode === "create"
        ? await createRagProfile(payload)
        : await updateRagProfile(profileId ?? form.id.trim(), payload);
      const savedId = typeof saved.profileId === "string" && saved.profileId.trim()
        ? saved.profileId.trim()
        : form.id.trim();
      toast.success(`${savedId} ${mode === "create" ? "created" : "saved"}`);
      navigate(returnPath);
    } catch (error) {
      const errorData = typeof error === "object" && error !== null && "data" in error
        ? (error as { data?: unknown }).data
        : undefined;
      const issues = isRecord(errorData) ? normalizeRagValidationIssues(errorData.errors) : [];
      if (issues.length > 0) {
        setErrors(mapIngestApiErrors(issues, form));
      }
      toast.error(error instanceof Error ? error.message : "Ingest profile could not be saved");
    } finally {
      setSubmitting(false);
    }
  }

  if (mode === "edit" && loading) {
    return (
      <>
        <PageHeader
          title="Loading Ingest Profile"
          actions={(
            <Button variant="secondary" onClick={() => navigate(returnPath)}>
              <X size={16} aria-hidden="true" />
              Cancel
            </Button>
          )}
        />
        <SurfaceCard>
          <h2 className="text-base font-semibold">Loading Profile</h2>
          <p className="mt-3 text-sm text-[var(--color-on-surface-variant)]">Loading ingest profile management data...</p>
        </SurfaceCard>
      </>
    );
  }

  if (mode === "edit" && !existingProfile) {
    return (
      <>
        <PageHeader
          title="Edit Ingest Profile"
          actions={(
            <Button variant="secondary" onClick={() => navigate(returnPath)}>
              <X size={16} aria-hidden="true" />
              Cancel
            </Button>
          )}
        />
        <SurfaceCard>
          <h2 className="text-base font-semibold">Profile Not Found</h2>
          <p className="mt-3 text-sm text-[var(--color-on-surface-variant)]">
            {isApiMode
              ? "The requested ingest profile could not be loaded from the RAG management API."
              : "The requested ingest profile does not exist in the mock registry."}
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
              Cancel
            </Button>
            <Button aria-busy={submitting} disabled={loading || submitting} type="submit" form="rag-ingest-profile-form">
              <Save size={16} aria-hidden="true" />
              {submitting ? "Saving..." : "Save Profile"}
            </Button>
          </>
        )}
      />

      <form className="grid gap-5" id="rag-ingest-profile-form" onSubmit={handleSubmit}>
        <SurfaceCard>
          <h2 className="text-base font-semibold">Identity</h2>
          <div className="mt-4 grid gap-4 lg:grid-cols-2">
            <Field label="Profile ID" error={errors.id}>
              <TextInput
                aria-label="Profile ID"
                className="font-mono"
                disabled={mode === "edit"}
                value={form.id}
                onChange={(event) => updateForm("id", event.target.value)}
              />
            </Field>
            <Field label="Status" error={errors.status}>
              <SelectInput aria-label="Status" value={form.status} onChange={(event) => updateForm("status", event.target.value as RagProfileStatus)}>
                {listProfileStatuses().map((status) => <option key={status} value={status}>{status}</option>)}
              </SelectInput>
            </Field>
            <Field label="Primary Corpus" error={errors.primaryCorpusId}>
              <SelectInput aria-label="Primary Corpus" value={form.primaryCorpusId} onChange={(event) => updateForm("primaryCorpusId", event.target.value)}>
                <option value="">Select corpus</option>
                {corpusOptions.map((corpus) => <option key={corpus.code} value={corpus.code}>{corpus.code}</option>)}
              </SelectInput>
            </Field>
            <Field label="Description">
              <TextArea aria-label="Description" className="min-h-28" value={form.description} onChange={(event) => updateForm("description", event.target.value)} />
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
            <summary className="cursor-pointer text-base font-semibold">Advanced Node Controls</summary>
            <p className="mt-2 text-sm text-[var(--color-on-surface-variant)]">
              Use these controls when the selected template needs a custom stage combination.
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
                        <span className="font-medium">{stageDefinition?.displayName || stageType}</span>
                        {required ? (
                          <span className="rounded-full bg-[var(--color-primary-container)] px-2 py-0.5 text-[11px] text-[var(--color-on-primary-container)]">Required</span>
                        ) : null}
                        {stageDefinition?.orchestration?.actsAsBarrier ? (
                          <span className="rounded-full bg-[var(--color-surface)] px-2 py-0.5 text-[11px] text-[var(--color-on-surface-variant)]">Control barrier</span>
                        ) : null}
                      </div>
                      <p className="mt-1 font-mono text-[12px] text-[var(--color-on-surface-variant)]">{ingestStageNodeId(stage.id)} · {stageType}</p>
                      {(stageDefinition?.orchestration?.mustRunBeforeStageTypes.length ?? 0) > 0 ? (
                        <p className="mt-1 text-[12px] text-[var(--color-on-surface-variant)]">
                          Before {stageDefinition?.orchestration?.mustRunBeforeStageTypes.join(", ")}
                        </p>
                      ) : null}
                    </div>
                    <ToggleSwitch checked={stage.enabled} disabled={required} label={`Enable ${stageType} node`} onCheckedChange={(checked) => updateStage(stage.id, checked)} />
                  </div>
                );
              })}
              {errors.pipeline ? <p className="text-[12px] text-[var(--color-error)]">{errors.pipeline}</p> : null}
            </div>
          </details>
        </SurfaceCard>

        <SurfaceCard>
          <h2 className="text-base font-semibold">Input Contract</h2>
          <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">
            These fields define what the workbench and ingest API should accept for this profile.
          </p>
          <div className="mt-4 grid gap-4 lg:grid-cols-2">
            <Field label="Supported Source Types" error={errors.supportedSourceTypesText}>
              <TextInput aria-label="Supported Source Types" className="font-mono" value={form.supportedSourceTypesText} onChange={(event) => updateForm("supportedSourceTypesText", event.target.value)} />
            </Field>
            <Field label="Required Source Version">
              <TextInput aria-label="Required Source Version" className="font-mono" value={form.requiredSourceVersion} onChange={(event) => updateForm("requiredSourceVersion", event.target.value)} />
            </Field>
          </div>
        </SurfaceCard>

        <SurfaceCard>
          <h2 className="text-base font-semibold">Stage Strategies</h2>
          <div className="mt-4 grid gap-4">
            {enabledStages.length === 0 ? (
              <p className="text-sm text-[var(--color-on-surface-variant)]">Enable one or more stages above to configure stage strategies.</p>
            ) : null}
            {enabledStages.map((stage) => {
              const stageType = ingestStageIdToStageType(stage.id);
              const stageDefinition = findPipelineStageDefinition(pipelineCatalog, stageType);
              const configSchema = strategySchemaForNode(stageDefinition, stage.strategyId);
              return (
              <div className="rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4" key={stage.id}>
                <div className="flex flex-wrap items-start justify-between gap-3">
                  <div>
                    <p className="font-medium">{stageDefinition?.displayName || stageType}</p>
                    <p className="mt-1 font-mono text-[12px] text-[var(--color-on-surface-variant)]">{stageType}</p>
                  </div>
                  <span className="rounded-full border border-[var(--color-outline-variant)] px-2.5 py-1 font-mono text-[11px] text-[var(--color-on-surface-variant)]">{ingestStageNodeId(stage.id)}</span>
                </div>
                <div className="mt-3 grid gap-4 lg:grid-cols-2">
                  <Field label="Strategy" error={errors.stageRows?.[stage.id]?.strategyId}>
                    <SelectInput
                      aria-label={`${stageType} strategy ID`}
                      value={stage.strategyId}
                      onChange={(event) => updateStageStrategy(stage.id, event.target.value)}
                    >
                      <option value="">{(allowedStageStrategies[stage.id]?.length ?? 0) > 0 ? "Select stage strategy" : "No supported strategies"}</option>
                      {(allowedStageStrategies[stage.id] ?? []).map((strategyId) => (
                        <option key={strategyId} value={strategyId}>{strategyOptionLabel(stageDefinition, strategyId)}</option>
                      ))}
                    </SelectInput>
                  </Field>

                  {stage.id === "resolveSource" ? (
                    <>
                      <Field label="Allowed URI Schemes">
                        <TextInput
                          aria-label="Resolve Source allowed URI schemes"
                          className="font-mono"
                          value={form.resolveSourceAllowedUriSchemesText}
                          onChange={(event) => updateForm("resolveSourceAllowedUriSchemesText", event.target.value)}
                        />
                      </Field>
                      <Field label="Max Bytes" error={errors.resolveSourceMaxBytes}>
                        <TextInput
                          aria-label="Resolve Source max bytes"
                          className="font-mono"
                          value={form.resolveSourceMaxBytes}
                          onChange={(event) => updateForm("resolveSourceMaxBytes", event.target.value)}
                        />
                      </Field>
                    </>
                  ) : null}

                  {stage.id === "parseDocument" ? (
                    <>
                      <Field label="Document Format" error={errors.parseDocumentFormat}>
                        <TextInput
                          aria-label="Parse Document format"
                          className="font-mono"
                          value={form.parseDocumentFormat}
                          onChange={(event) => updateForm("parseDocumentFormat", event.target.value)}
                        />
                      </Field>
                      <Field label="Unsupported Format Policy">
                        <TextInput
                          aria-label="Parse Document unsupported format policy"
                          className="font-mono"
                          value={form.parseDocumentUnsupportedFormatPolicy}
                          onChange={(event) => updateForm("parseDocumentUnsupportedFormatPolicy", event.target.value)}
                        />
                      </Field>
                    </>
                  ) : null}

                  {stage.id === "splitText" ? (
                    <>
                      <Field label="Splitter Type" error={errors.splitTextType}>
                        <TextInput
                          aria-label="Split Text splitter type"
                          className="font-mono"
                          value={form.splitTextType}
                          onChange={(event) => updateForm("splitTextType", event.target.value)}
                        />
                      </Field>
                      <Field label="Max Tokens" error={errors.splitTextMaxTokens}>
                        <TextInput
                          aria-label="Split Text max tokens"
                          className="font-mono"
                          value={form.splitTextMaxTokens}
                          onChange={(event) => updateForm("splitTextMaxTokens", event.target.value)}
                        />
                      </Field>
                      <Field label="Overlap Tokens" error={errors.splitTextOverlapTokens}>
                        <TextInput
                          aria-label="Split Text overlap tokens"
                          className="font-mono"
                          value={form.splitTextOverlapTokens}
                          onChange={(event) => updateForm("splitTextOverlapTokens", event.target.value)}
                        />
                      </Field>
                      <Field label="Heading Depth" error={errors.splitTextHeadingDepth}>
                        <TextInput
                          aria-label="Split Text heading depth"
                          className="font-mono"
                          value={form.splitTextHeadingDepth}
                          onChange={(event) => updateForm("splitTextHeadingDepth", event.target.value)}
                        />
                      </Field>
                    </>
                  ) : null}

                  {stage.id === "vectorIndex" ? (
                    <>
                      <Field label="Embedding Model Code">
                        <TextInput
                          aria-label="Embedding Model Code"
                          className="font-mono"
                          value={form.embeddingModelCode}
                          onChange={(event) => updateForm("embeddingModelCode", event.target.value)}
                        />
                      </Field>
                      <Field label="Embedding Version">
                        <TextInput
                          aria-label="Embedding Version"
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
                    No supported strategies exist for this stage and corpus yet.
                  </p>
                ) : null}
                <details className="mt-4 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface)] p-3">
                  <summary className="cursor-pointer text-sm font-medium">Developer contract</summary>
                  <div className="mt-3 grid gap-3">
                    <div className="grid gap-2 text-[12px] text-[var(--color-on-surface-variant)] lg:grid-cols-2">
                      <p><span className="font-semibold">Input:</span> {stageDefinition?.inputArtifactKeys.join(", ") || "n/a"}</p>
                      <p><span className="font-semibold">Output:</span> {stageDefinition?.outputArtifactKeys.join(", ") || "n/a"}</p>
                    </div>
                    <JsonPreview value={configSchema} />
                  </div>
                </details>
              </div>
            );
            })}
            {graphBackedCorpus ? (
              <Field label="Graph Write Policy">
                <TextInput aria-label="Graph Write Policy" className="font-mono" value={form.graphWritePolicy} onChange={(event) => updateForm("graphWritePolicy", event.target.value)} />
              </Field>
            ) : null}
          </div>
        </SurfaceCard>

        <SurfaceCard>
          <h2 className="text-base font-semibold">Pipeline Summary</h2>
          <div className="mt-4">
            <SummaryList rows={[
              { label: "Primary Corpus", value: form.primaryCorpusId || "Not set" },
              { label: "Enabled Stages", value: String(form.pipelineStages.filter((stage) => stage.enabled).length) },
              {
                label: "Payload Strategy",
                value: form.pipelineStages.find((stage) => stage.id === "preparePayload")?.strategyId || "Not set",
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
  const title = mode === "create" ? "Create Retrieval Profile" : "Edit Retrieval Profile";
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
        toast.error(error instanceof Error ? error.message : "Profile catalog could not be loaded");
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
      toast.success(mode === "create" ? "Retrieval profile created." : "Profile updated.");
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
        toast.error(validationIssueMessage(issues[0] ?? { message: "Resolve profile form errors before saving." }));
        return;
      }

      const saved = mode === "create"
        ? await createRagProfile(payload)
        : await updateRagProfile(profileId ?? form.id.trim(), payload);
      const savedId = typeof saved.profileId === "string" && saved.profileId.trim()
        ? saved.profileId.trim()
        : form.id.trim();
      toast.success(`${savedId} ${mode === "create" ? "created" : "saved"}`);
      navigate(returnPath);
    } catch (error) {
      const errorData = typeof error === "object" && error !== null && "data" in error
        ? (error as { data?: unknown }).data
        : undefined;
      const issues = isRecord(errorData) ? normalizeRagValidationIssues(errorData.errors) : [];
      if (issues.length > 0) {
        setErrors(mapRetrievalApiErrors(issues, form));
      }
      toast.error(error instanceof Error ? error.message : "Retrieval profile could not be saved");
    } finally {
      setSubmitting(false);
    }
  }

  if (mode === "edit" && loading) {
    return (
      <>
        <PageHeader
          title="Loading Retrieval Profile"
          actions={(
            <Button variant="secondary" onClick={() => navigate(returnPath)}>
              <X size={16} aria-hidden="true" />
              Cancel
            </Button>
          )}
        />
        <SurfaceCard>
          <h2 className="text-base font-semibold">Loading Profile</h2>
          <p className="mt-3 text-sm text-[var(--color-on-surface-variant)]">Loading retrieval profile management data...</p>
        </SurfaceCard>
      </>
    );
  }

  if (mode === "edit" && !existingProfile) {
    return (
      <>
        <PageHeader
          title="Edit Retrieval Profile"
          actions={(
            <Button variant="secondary" onClick={() => navigate(returnPath)}>
              <X size={16} aria-hidden="true" />
              Cancel
            </Button>
          )}
        />
        <SurfaceCard>
          <h2 className="text-base font-semibold">Profile Not Found</h2>
          <p className="mt-3 text-sm text-[var(--color-on-surface-variant)]">
            {isApiMode
              ? "The requested retrieval profile could not be loaded from the RAG management API."
              : "The requested retrieval profile does not exist in the mock registry."}
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
              Cancel
            </Button>
            <Button aria-busy={submitting} disabled={loading || submitting} type="submit" form="rag-retrieval-profile-form">
              <Save size={16} aria-hidden="true" />
              {submitting ? "Saving..." : "Save Profile"}
            </Button>
          </>
        )}
      />

      <form className="grid gap-5" id="rag-retrieval-profile-form" onSubmit={handleSubmit}>
        <SurfaceCard>
          <h2 className="text-base font-semibold">Identity</h2>
          <div className="mt-4 grid gap-4 lg:grid-cols-2">
            <Field label="Profile ID" error={errors.id}>
              <TextInput
                aria-label="Profile ID"
                className="font-mono"
                disabled={mode === "edit"}
                value={form.id}
                onChange={(event) => updateForm("id", event.target.value)}
              />
            </Field>
            <Field label="Status" error={errors.status}>
              <SelectInput aria-label="Status" value={form.status} onChange={(event) => updateForm("status", event.target.value as RagProfileStatus)}>
                {listProfileStatuses().map((status) => <option key={status} value={status}>{status}</option>)}
              </SelectInput>
            </Field>
            <Field label="Primary Corpus" error={errors.primaryCorpusId}>
              <SelectInput aria-label="Primary Corpus" value={form.primaryCorpusId} onChange={(event) => updateForm("primaryCorpusId", event.target.value)}>
                <option value="">Select corpus</option>
                {corpusOptions.map((corpus) => <option key={corpus.code} value={corpus.code}>{corpus.code}</option>)}
              </SelectInput>
            </Field>
            <Field label="Description">
              <TextArea aria-label="Description" className="min-h-28" value={form.description} onChange={(event) => updateForm("description", event.target.value)} />
            </Field>
          </div>
        </SurfaceCard>

        <SurfaceCard>
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div>
              <h2 className="text-base font-semibold">Retrieval Routes</h2>
              <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">
                Each route selects a corpus and retrieval strategy. The strategy decides whether it is fulltext, vector, graph, or structured recall.
              </p>
            </div>
            <Button type="button" variant="secondary" onClick={addRoute}>
              <Plus size={16} aria-hidden="true" />
              Add Route
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
          <h2 className="text-base font-semibold">Result Policies</h2>
          <div className="mt-4 grid gap-4 lg:grid-cols-2">
            <Field label="Fusion Policy">
              <SelectInput aria-label="Fusion Policy" className="font-mono" value={form.fusionPolicy} onChange={(event) => updateForm("fusionPolicy", event.target.value)}>
                {fusionPolicyOptions.map((policy) => <option key={policy} value={policy}>{policy}</option>)}
              </SelectInput>
            </Field>
            <Field label="Dedup Policy">
              <SelectInput aria-label="Dedup Policy" className="font-mono" value={form.dedupPolicy} onChange={(event) => updateForm("dedupPolicy", event.target.value)}>
                {dedupPolicyOptions.map((policy) => <option key={policy} value={policy}>{policy}</option>)}
              </SelectInput>
            </Field>
            <Field label="Topic Policy">
              <SelectInput aria-label="Topic Policy" className="font-mono" value={form.topicPolicy} onChange={(event) => updateForm("topicPolicy", event.target.value)}>
                {topicPolicyOptions.map((policy) => <option key={policy} value={policy}>{policy}</option>)}
              </SelectInput>
            </Field>
            <Field label="Rerank Model Code">
              <TextInput aria-label="Rerank Model Code" className="font-mono" value={form.rerankModelCode} onChange={(event) => updateForm("rerankModelCode", event.target.value)} />
            </Field>
          </div>
          <div className="mt-4">
            <ToggleSwitch checked={form.rerankEnabled} label="Rerank Enabled" onCheckedChange={(checked) => updateForm("rerankEnabled", checked)} />
          </div>
        </SurfaceCard>

        <SurfaceCard>
          <h2 className="text-base font-semibold">Context Strategies</h2>
          <div className="mt-4 grid gap-3">
            {contextStrategyOptions.map((strategyId) => (
              <div className="flex items-center justify-between gap-4 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] px-4 py-3" key={strategyId}>
                <span className="font-mono text-sm">{strategyId}</span>
                <ToggleSwitch
                  checked={form.contextStrategies.includes(strategyId)}
                  label={`Enable context strategy ${strategyId}`}
                  onCheckedChange={(checked) => toggleContextStrategy(strategyId, checked)}
                />
              </div>
            ))}
            {errors.contextStrategies ? <p className="text-[12px] text-[var(--color-error)]">{errors.contextStrategies}</p> : null}
          </div>
        </SurfaceCard>

        <SurfaceCard>
          <h2 className="text-base font-semibold">Graph Expansion</h2>
          <div className="mt-4">
            <Field label="Graph Expansion Policy" error={errors.graphExpansionPolicy}>
              <TextInput
                aria-label="Graph Expansion Policy"
                className="font-mono"
                placeholder={enabledGraphRouteExists ? "" : "Select an enabled graph strategy to use this policy"}
                value={form.graphExpansionPolicy}
                onChange={(event) => updateForm("graphExpansionPolicy", event.target.value)}
              />
            </Field>
          </div>
        </SurfaceCard>

        <SurfaceCard>
          <div className="flex flex-wrap items-start justify-between gap-3">
            <h2 className="text-base font-semibold">Profile Filters</h2>
            <Button type="button" variant="secondary" onClick={addFilter}>
              <Plus size={16} aria-hidden="true" />
              Add Filter
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
