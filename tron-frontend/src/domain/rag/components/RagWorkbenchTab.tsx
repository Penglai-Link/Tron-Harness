import {
  useEffect,
  useRef,
  useState,
} from "react";
import {
  ChevronDown,
  Search,
  Upload,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import {
  Field,
  MultiSelectDropdown,
  SelectInput,
  SurfaceCard,
  TextArea,
  TextInput,
  ToggleSwitch,
} from "@/shared";
import { Button } from "@/shared/ui/Button";
import { i18n } from "@/i18n";

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
  type RagSearchHitApi,
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
import { SelectItem } from "@/components/ui/select";

type WorkbenchSourcePayloadForm = {
  documentFormat: string;
  documentUri: string;
  language: string;
  metadataSourcePath: string;
  sourceCode: string;
  sourceVersion: string;
  text: string;
};

type WorkbenchBusinessAttributeRow = {
  field: string;
  formId: string;
  value: string;
};

type WorkbenchRelatedNodeRow = {
  code: string;
  formId: string;
  kind: string;
  label: string;
  propertiesJson: string;
  ref: string;
  text: string;
  title: string;
};

type WorkbenchExistingGraphNodeRow = {
  code: string;
  formId: string;
  ref: string;
};

type WorkbenchRelatedEdgeRow = {
  formId: string;
  from: string;
  propertiesJson: string;
  relationship: string;
  to: string;
};

type WorkbenchFilterRow = {
  field: string;
  formId: string;
  operator: string;
  value: string;
};

type WorkbenchFieldOption = {
  field: string;
  type: RagFilterFieldType;
};

type WorkbenchPanel = "ingest" | "retrieval";
type WorkbenchScopeType = "AGENT" | "SPACE" | "USER";

const workbenchPanels: Array<{ label: string; summary: string; value: WorkbenchPanel }> = [
  {
    label: "Ingest request",
    summary: "Write a source payload into the selected corpus.",
    value: "ingest",
  },
  {
    label: "Retrieval test",
    summary: "Run a retrieval profile with query fields and filters.",
    value: "retrieval",
  },
];
const workbenchScopeOptions: Array<{ label: string; value: WorkbenchScopeType }> = [
  { label: "Workspace", value: "SPACE" },
  { label: "User", value: "USER" },
  { label: "Agent", value: "AGENT" },
];
const relatedGraphAnchorRefs = ["$target", "$ragNode", "$source", "$skill"] as const;
const reservedRelatedGraphNodeProperties = new Set(["relatedKind", "relatedRef", "title", "text"]);
const reservedRelatedGraphEdgeProperties = new Set(["edgeCode", "sourceNodeCode", "targetNodeCode", "relationship"]);

const workbenchFieldHints = {
  businessField: "Business field written to payload.domainFields, sourced from the current corpus field mapping.",
  businessValue: "Value for the business field. It is converted with the corpus filter schema type.",
  contextStrategy: "Strategy used to assemble the context pack from retrieval results.",
  corpus: "Target corpus for this ingest request.",
  documentFormat: "Document parse format, such as PDF, DOCX, or MARKDOWN. It must match the ingest parse strategy.",
  documentUri: "Document source URI. Project documents usually start with project-doc://.",
  filterField: "Additional filter field for this retrieval request, sourced from the profile-bound corpus filter schema.",
  filterOperator: "Filter comparison operator, such as eq for equals or in for list membership.",
  filterValue: "Filter query value. Use comma-separated values for the in operator.",
  ingestProfile: "Profile used for this ingest request. It controls source contract, parsing, splitting, and indexing.",
  language: "Language tag written to metadata.language. Defaults to zh-CN.",
  limit: "Maximum hits returned by this retrieval request.",
  metadataSourcePath: "Path written to metadata.sourcePath so the document can be traced in the project.",
  query: "Natural-language query for this retrieval test.",
  relatedExistingNodeCode: "Node code that already exists in the graph store. It is referenced only and is not updated by this ingest.",
  relatedGraph: "Optional graph nodes and relationships attached to this ingest payload.",
  relatedGraphEdgeEndpoint: "Reference used by this edge. It can be an anchor, a new node ref, or an existing node ref.",
  relatedGraphEdgeProperties: "Optional JSON object written to the edge properties. Values must be string, number, or boolean.",
  relatedGraphNodeCode: "Graph node code for a new node written by this request.",
  relatedGraphNodeKind: "Business kind saved as relatedKind and checked against graphWritePolicy when configured.",
  relatedGraphNodeLabel: "Graph label used when creating or updating the new node.",
  relatedGraphNodeProperties: "Optional JSON object written to the new node properties. Values must be string, number, or boolean.",
  relatedGraphNodeRef: "Local reference name used by edges in this relatedGraph payload.",
  relatedGraphRelationship: "Relationship code for this edge, checked against graphWritePolicy when configured.",
  retrievalProfile: "Profile used for this retrieval request. It controls routes, fusion, deduplication, and context strategy.",
  returnField: "Extra field requested in retrieval results, sourced from corpus field mapping.",
  scopeType: "Target request scope. The scope code is filled automatically from the selected type.",
  sourceCode: "Unique source code for this ingest request. The same code and version support idempotent updates.",
  sourceText: "Body text for direct text ingest. Document reference ingest uses document URI and format instead.",
  sourceVersion: "Source version, usually v1. It cannot be edited when the ingest profile requires a fixed version.",
  tokenBudget: "Maximum token budget for the context pack returned to the model.",
} satisfies Record<string, string>;

const workbenchFilterOperators = ["eq", "in", "gt", "gte", "lt", "lte"] as const;
const hiddenSourceTypeFields = new Set(["sourceType", "derivedSourceType"]);
const workbenchReservedReturnFields = new Set([
  "corpusId",
  ...hiddenSourceTypeFields,
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
  "status",
  "textUnitStatus",
]);

function isDocumentPayloadProfile(profile?: RagIngestProfileRecord) {
  return profile?.config.stages.preparePayload?.strategyId === "RAG_DOCUMENT_REF_CONTRACT_PAYLOAD_V1";
}

function defaultDocumentFormat(profile?: RagIngestProfileRecord) {
  const parseDocument = profile?.config.stages.parseDocument;
  const config = parseDocument?.config;
  if (Array.isArray(config?.supportedFormats)) {
    const supportedFormat = config.supportedFormats.find((format): format is string => (
      typeof format === "string" && Boolean(format.trim())
    ));
    if (supportedFormat) return supportedFormat.trim();
  }
  if (typeof config?.format === "string" && config.format.trim()) {
    return config.format.trim();
  }
  if (parseDocument?.strategyId === "RAG_PARSE_MINERU_DOCUMENT") return "PDF";
  return "MARKDOWN";
}

function defaultDocumentExtension(documentFormat: string) {
  const normalized = documentFormat.trim().toUpperCase();
  if (normalized === "DOCX") return "docx";
  if (normalized === "PDF") return "pdf";
  return "md";
}

function buildDefaultPayloadForm(
  corpusId: string,
  profile?: RagIngestProfileRecord,
  workspaceCode = "TRON_CORE",
): WorkbenchSourcePayloadForm {
  const sourceVersion = profile?.config.requiredSourceVersion || "v1";
  const sourceCode = `${corpusId || "rag"}-workbench-source`;
  const documentFormat = defaultDocumentFormat(profile);
  const normalizedWorkspaceCode = workspaceCode.trim() || "TRON_CORE";
  return {
    documentFormat,
    documentUri: `project-doc://${normalizedWorkspaceCode}/docs/${sourceCode}.${defaultDocumentExtension(documentFormat)}`,
    language: "zh-CN",
    metadataSourcePath: "",
    sourceCode,
    sourceVersion,
    text: i18n.t("pages:rag.workbench.defaults.sampleText", {
      corpusId: corpusId || "RAG",
      defaultValue: "{{corpusId}} workbench ingest sample.",
    }),
  };
}

function buildPayloadFromForm(
  form: WorkbenchSourcePayloadForm,
  documentPayload: boolean,
  domainFields: Record<string, unknown>,
  relatedGraph?: Record<string, unknown>,
): Record<string, unknown> {
  const metadata: Record<string, string> = {};
  if (form.language.trim()) metadata.language = form.language.trim();
  if (form.metadataSourcePath.trim()) metadata.sourcePath = form.metadataSourcePath.trim();

  const payload: Record<string, unknown> = {
    metadata,
    source: {
      code: form.sourceCode.trim(),
      version: form.sourceVersion.trim(),
    },
  };
  if (Object.keys(domainFields).length > 0) {
    payload.domainFields = domainFields;
  }
  if (relatedGraph) {
    payload.relatedGraph = relatedGraph;
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
  return error instanceof Error ? error.message : i18n.t("pages:rag.workbench.errors.requestFailed", {
    defaultValue: "RAG workbench request failed.",
  });
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
  return Number.isFinite(score) ? score!.toFixed(3) : i18n.t("pages:rag.common.none", { defaultValue: "None" });
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

function sanitizeGraphCodePart(value: string) {
  return value
    .trim()
    .toUpperCase()
    .replace(/[^A-Z0-9_-]+/g, "_")
    .replace(/^_+|_+$/g, "");
}

function createWorkbenchRelatedNodeRow(index: number, sourceCode: string): WorkbenchRelatedNodeRow {
  const codePrefix = sanitizeGraphCodePart(sourceCode) || "RELATED";
  return {
    code: `${codePrefix}_NODE_${index + 1}`,
    formId: `related-node-${index + 1}`,
    kind: "TOPIC",
    label: "RagTopic",
    propertiesJson: "",
    ref: `new.node${index + 1}`,
    text: "",
    title: "",
  };
}

function createWorkbenchExistingGraphNodeRow(index: number): WorkbenchExistingGraphNodeRow {
  return {
    code: "",
    formId: `existing-graph-node-${index + 1}`,
    ref: `existing.node${index + 1}`,
  };
}

function createWorkbenchRelatedEdgeRow(index: number, from = "$target", to = ""): WorkbenchRelatedEdgeRow {
  return {
    formId: `related-edge-${index + 1}`,
    from,
    propertiesJson: "",
    relationship: "RELATES_TO",
    to,
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function isPrimitiveGraphPropertyValue(value: unknown): value is string | number | boolean {
  return (
    typeof value === "string"
    || typeof value === "boolean"
    || (typeof value === "number" && Number.isFinite(value))
  );
}

function parseRelatedGraphProperties(
  value: string,
  label: string,
  reservedNames: Set<string>,
): Record<string, string | number | boolean> | undefined {
  const trimmed = value.trim();
  if (!trimmed) return undefined;
  let parsed: unknown;
  try {
    parsed = JSON.parse(trimmed);
  } catch {
    throw new Error(i18n.t("pages:rag.workbench.errors.relatedGraphPropertiesJson", {
      defaultValue: "{{label}} properties must be a JSON object.",
      label,
    }));
  }
  if (!isRecord(parsed)) {
    throw new Error(i18n.t("pages:rag.workbench.errors.relatedGraphPropertiesObject", {
      defaultValue: "{{label}} properties must be a JSON object.",
      label,
    }));
  }
  return Object.fromEntries(
    Object.entries(parsed).map(([key, propertyValue]) => {
      const name = key.trim();
      if (!name || reservedNames.has(name) || !isPrimitiveGraphPropertyValue(propertyValue)) {
        throw new Error(i18n.t("pages:rag.workbench.errors.relatedGraphPropertyValue", {
          defaultValue: "{{label}} properties can only use non-reserved string, number, or boolean values.",
          label,
        }));
      }
      return [name, propertyValue];
    }),
  );
}

function buildRelatedGraphFromRows(
  nodes: WorkbenchRelatedNodeRow[],
  existingNodes: WorkbenchExistingGraphNodeRow[],
  edges: WorkbenchRelatedEdgeRow[],
): Record<string, unknown> | undefined {
  const relatedNodes = nodes
    .map((row, index) => {
      const hasValue = [
        row.code,
        row.kind,
        row.label,
        row.propertiesJson,
        row.ref,
        row.text,
        row.title,
      ].some((value) => value.trim());
      if (!hasValue) return undefined;
      if (!row.ref.trim() || !row.kind.trim() || !row.label.trim() || !row.code.trim()) {
        throw new Error(i18n.t("pages:rag.workbench.errors.relatedGraphNodeRequired", {
          defaultValue: "Related graph node {{index}} must include ref, kind, label, and code.",
          index: index + 1,
        }));
      }
      const properties = parseRelatedGraphProperties(
        row.propertiesJson,
        i18n.t("pages:rag.workbench.relatedGraph.nodeLabel", {
          defaultValue: "Node {{index}}",
          index: index + 1,
        }),
        reservedRelatedGraphNodeProperties,
      );
      return {
        code: row.code.trim(),
        kind: row.kind.trim(),
        label: row.label.trim(),
        ...(properties ? { properties } : {}),
        ref: row.ref.trim(),
        ...(row.text.trim() ? { text: row.text.trim() } : {}),
        ...(row.title.trim() ? { title: row.title.trim() } : {}),
      };
    })
    .filter(Boolean) as Record<string, unknown>[];

  const relatedExistingNodes = existingNodes
    .map((row, index) => {
      const hasValue = [row.code, row.ref].some((value) => value.trim());
      if (!hasValue) return undefined;
      if (!row.ref.trim() || !row.code.trim()) {
        throw new Error(i18n.t("pages:rag.workbench.errors.relatedGraphExistingNodeRequired", {
          defaultValue: "Existing graph node {{index}} must include ref and code.",
          index: index + 1,
        }));
      }
      return {
        code: row.code.trim(),
        ref: row.ref.trim(),
      };
    })
    .filter(Boolean) as Record<string, unknown>[];

  const refs = new Set<string>(relatedGraphAnchorRefs);
  [...relatedNodes, ...relatedExistingNodes].forEach((row) => {
    const ref = typeof row.ref === "string" ? row.ref : "";
    if (!ref || refs.has(ref)) {
      throw new Error(i18n.t("pages:rag.workbench.errors.relatedGraphDuplicateRef", {
        defaultValue: "Related graph refs must be unique and cannot reuse anchor names.",
      }));
    }
    refs.add(ref);
  });

  const relatedEdges = edges
    .map((row, index) => {
      const hasValue = [row.from, row.propertiesJson, row.relationship, row.to].some((value) => value.trim());
      if (!hasValue) return undefined;
      if (!row.from.trim() || !row.to.trim() || !row.relationship.trim()) {
        throw new Error(i18n.t("pages:rag.workbench.errors.relatedGraphEdgeRequired", {
          defaultValue: "Related graph edge {{index}} must include from, to, and relationship.",
          index: index + 1,
        }));
      }
      if (!refs.has(row.from.trim()) || !refs.has(row.to.trim())) {
        throw new Error(i18n.t("pages:rag.workbench.errors.relatedGraphEdgeRef", {
          defaultValue: "Related graph edge {{index}} must use refs declared by anchors, new nodes, or existing nodes.",
          index: index + 1,
        }));
      }
      const properties = parseRelatedGraphProperties(
        row.propertiesJson,
        i18n.t("pages:rag.workbench.relatedGraph.edgeLabel", {
          defaultValue: "Edge {{index}}",
          index: index + 1,
        }),
        reservedRelatedGraphEdgeProperties,
      );
      return {
        from: row.from.trim(),
        ...(properties ? { properties } : {}),
        relationship: row.relationship.trim(),
        to: row.to.trim(),
      };
    })
    .filter(Boolean) as Record<string, unknown>[];

  if (relatedNodes.length === 0 && relatedExistingNodes.length === 0 && relatedEdges.length === 0) {
    return undefined;
  }
  return {
    edges: relatedEdges,
    existingNodes: relatedExistingNodes,
    nodes: relatedNodes,
  };
}

function buildRelatedGraphPreview(
  nodes: WorkbenchRelatedNodeRow[],
  existingNodes: WorkbenchExistingGraphNodeRow[],
  edges: WorkbenchRelatedEdgeRow[],
) {
  const parsePropertiesForPreview = (value: string) => {
    const trimmed = value.trim();
    if (!trimmed) return undefined;
    try {
      return JSON.parse(trimmed) as unknown;
    } catch {
      return trimmed;
    }
  };
  const relatedNodes = nodes
    .filter((row) => [row.code, row.kind, row.label, row.ref, row.text, row.title, row.propertiesJson].some((value) => value.trim()))
    .map((row) => ({
      code: row.code.trim(),
      kind: row.kind.trim(),
      label: row.label.trim(),
      ...(row.propertiesJson.trim() ? { properties: parsePropertiesForPreview(row.propertiesJson) } : {}),
      ref: row.ref.trim(),
      ...(row.text.trim() ? { text: row.text.trim() } : {}),
      ...(row.title.trim() ? { title: row.title.trim() } : {}),
    }));
  const relatedExistingNodes = existingNodes
    .filter((row) => [row.code, row.ref].some((value) => value.trim()))
    .map((row) => ({
      code: row.code.trim(),
      ref: row.ref.trim(),
    }));
  const relatedEdges = edges
    .filter((row) => [row.from, row.propertiesJson, row.relationship, row.to].some((value) => value.trim()))
    .map((row) => ({
      from: row.from.trim(),
      ...(row.propertiesJson.trim() ? { properties: parsePropertiesForPreview(row.propertiesJson) } : {}),
      relationship: row.relationship.trim(),
      to: row.to.trim(),
    }));
  if (relatedNodes.length === 0 && relatedExistingNodes.length === 0 && relatedEdges.length === 0) {
    return undefined;
  }
  return {
    edges: relatedEdges,
    existingNodes: relatedExistingNodes,
    nodes: relatedNodes,
  };
}

function parseFilterValue(value: string, valueType: "boolean" | "integer" | "string", field: string): unknown {
  const trimmed = value.trim();
  if (valueType === "integer") {
    const parsed = Number.parseInt(trimmed, 10);
    if (!Number.isFinite(parsed)) {
      throw new Error(i18n.t("pages:rag.workbench.errors.integerField", {
        defaultValue: "Filter field {{field}} must be an integer.",
        field,
      }));
    }
    return parsed;
  }
  if (valueType === "boolean") {
    const normalized = trimmed.toLowerCase();
    if (["true", "1", "yes"].includes(normalized)) return true;
    if (["false", "0", "no"].includes(normalized)) return false;
    throw new Error(i18n.t("pages:rag.workbench.errors.booleanField", {
      defaultValue: "Filter field {{field}} must be true or false.",
      field,
    }));
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

function corpusSupportsRelatedGraph(corpus?: RagCorpusRecord) {
  if (!corpus) return false;
  return Boolean(
    corpus.config?.capabilities.graph
    || corpus.strategies.includes("GRAPH")
    || corpus.config?.storageBinding.targets.some((target) => target.backend === "GRAPH"),
  );
}

function collectRelatedGraphRefOptions(
  nodes: WorkbenchRelatedNodeRow[],
  existingNodes: WorkbenchExistingGraphNodeRow[],
) {
  const refs = new Set<string>(relatedGraphAnchorRefs);
  nodes.forEach((row) => {
    if (row.ref.trim()) refs.add(row.ref.trim());
  });
  existingNodes.forEach((row) => {
    if (row.ref.trim()) refs.add(row.ref.trim());
  });
  return [...refs];
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
      throw new Error(i18n.t("pages:rag.workbench.errors.businessAttributeRequired", {
        defaultValue: "Business attribute {{index}} must include both field and value.",
        index: index + 1,
      }));
    }
    const valueType = fieldSchema[field]?.type ?? "string";
    domainFields[field] = parseFilterValue(rawValue, valueType, field);
  });
  return domainFields;
}

function formatFieldValue(value: unknown) {
  if (value === null || value === undefined) return "";
  if (typeof value === "string") return value;
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  return JSON.stringify(value);
}

function treeHitMetadataRows(hit: RagSearchHitApi) {
  const fields = isRecord(hit.fields) ? hit.fields : {};
  const metadata = isRecord(hit.metadata) ? hit.metadata : {};
  const treeValue = (field: "matchedBy" | "treeNodeCode" | "treeNodeTitle" | "treeScore") => (
    fields[field] ?? hit[field] ?? metadata[field]
  );
  const treeScore = treeValue("treeScore");
  const rows = [
    { label: "matchedBy", value: treeValue("matchedBy") },
    { label: "treeNodeCode", value: treeValue("treeNodeCode") },
    { label: "treeNodeTitle", value: treeValue("treeNodeTitle") },
    {
      label: "treeScore",
      value: typeof treeScore === "number" ? formatScore(treeScore) : treeScore,
    },
  ];
  return rows.filter((row) => row.value !== undefined && row.value !== null && String(row.value).trim());
}

function formatArtifactKeys(keys: string[] | undefined) {
  return keys && keys.length > 0 ? keys.join(", ") : i18n.t("pages:rag.common.none", { defaultValue: "None" });
}

function StageResultsList({
  results,
  title,
}: {
  results?: RagIngestResponseApi["stageResults"];
  title: string;
}) {
  const { t } = useTranslation(["pages"]);
  const pageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`pages:rag.workbench.${key}`, { defaultValue, ...options })
  );

  if (!results || results.length === 0) return null;
  const warningCount = results.reduce((count, result) => count + (result.warnings?.length ?? 0), 0);
  const totalDurationMs = results.reduce((duration, result) => duration + (result.durationMs ?? 0), 0);

  return (
    <details className="group mt-5 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] text-sm">
      <summary className="flex cursor-pointer list-none flex-wrap items-center justify-between gap-3 p-4 marker:hidden [&::-webkit-details-marker]:hidden">
        <div className="flex min-w-0 items-center gap-[12px]">
          <ChevronDown
            className="shrink-0 transition-transform group-open:rotate-180"
            size={16}
            aria-hidden="true"
          />
          <h2 className="truncate text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">{title}</h2>
        </div>
        <div className="flex flex-wrap items-center justify-end gap-[12px]">
          <span className="rounded-full border border-[var(--color-outline-variant)] px-2.5 py-1 text-[12px] text-[var(--color-on-surface-variant)]">
            {pageT("stageResults.stageDuration", "{{count}} stages · {{duration}}ms", { count: results.length, duration: totalDurationMs })}
          </span>
          {warningCount > 0 ? (
            <span className="rounded-full border border-[var(--color-warning)]/30 px-2.5 py-1 text-[12px] text-[var(--color-warning)]">
              {pageT("stageResults.warningCount", "{{count}} warnings", { count: warningCount })}
            </span>
          ) : null}
          <span className="text-[12px] text-[var(--color-on-surface-variant)]">{pageT("stageResults.expand", "Expand")}</span>
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
              <div className="mt-3 flex flex-wrap items-center gap-[12px]">
                <span className="rounded-full border border-[var(--color-outline-variant)] px-2.5 py-1 font-mono text-[12px] text-[var(--color-on-surface-variant)]">
                  {formatRagStatusLabel(result.status, t)} · {result.durationMs ?? 0}ms
                </span>
              </div>
              <dl className="mt-3 grid gap-[12px] rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-3 sm:grid-cols-2">
                <div>
                  <dt className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">{pageT("stageResults.inputArtifacts", "Input artifacts")}</dt>
                  <dd className="mt-1 break-all font-mono text-[12px]">{formatArtifactKeys(result.inputArtifactKeys)}</dd>
                </div>
                <div>
                  <dt className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">{pageT("stageResults.outputArtifacts", "Output artifacts")}</dt>
                  <dd className="mt-1 break-all font-mono text-[12px]">{formatArtifactKeys(result.outputArtifactKeys)}</dd>
                </div>
              </dl>
              {warnings.length > 0 ? (
                <p className="mt-3 text-[12px] text-[var(--color-warning)]">
                  {pageT("stageResults.warnings", "Warnings: {{warnings}}", { warnings: warnings.join(", ") })}
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
  hidePanelSwitcher = false,
  initialCorpusId,
  initialIngestProfileId,
  initialPanel = "ingest",
  initialRetrievalProfileId,
  lockIngestContext = false,
  lockRetrievalContext = false,
  profiles,
  traceEnabled,
  userCode,
  workspaceCode,
  onNotice,
  onTraceEnabledChange,
}: {
  corpora: RagCorpusRecord[];
  hidePanelSwitcher?: boolean;
  initialCorpusId?: string;
  initialIngestProfileId?: string;
  initialPanel?: WorkbenchPanel;
  initialRetrievalProfileId?: string;
  lockIngestContext?: boolean;
  lockRetrievalContext?: boolean;
  profiles: RagProfileRecord[];
  traceEnabled: boolean;
  userCode: string;
  workspaceCode: string;
  onNotice: (notice: string) => void;
  onTraceEnabledChange: (checked: boolean) => void;
}) {
  const { t } = useTranslation(["pages"]);
  const pageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`pages:rag.workbench.${key}`, { defaultValue, ...options })
  );
  const hintT = (key: keyof typeof workbenchFieldHints) => (
    pageT(`hints.${key}`, workbenchFieldHints[key])
  );
  const visibleCorpora = corpora.filter((corpus) => corpus.status !== "Reserved" && corpus.status !== "Disabled");
  const ingestProfiles = profiles.filter((profile): profile is RagIngestProfileRecord => profile.type === "Ingest");
  const retrievalProfiles = profiles.filter((profile): profile is RagRetrievalProfileRecord => (
    profile.type === "Retrieval" && profile.status !== "Draft"
  ));
  const initialVisibleCorpusId = initialCorpusId && visibleCorpora.some((corpus) => corpus.code === initialCorpusId)
    ? initialCorpusId
    : visibleCorpora[0]?.code ?? "";
  const [corpusId, setCorpusId] = useState(initialCorpusId ?? initialVisibleCorpusId);
  const initialIngestId = initialIngestProfileId && ingestProfiles.some((profile) => profile.id === initialIngestProfileId)
    ? initialIngestProfileId
    : ingestProfiles[0]?.id ?? "";
  const [ingestProfileId, setIngestProfileId] = useState(initialIngestProfileId ?? initialIngestId);
  const [targetScopeType, setTargetScopeType] = useState<WorkbenchScopeType>("SPACE");
  const [sourcePayloadEdited, setSourcePayloadEdited] = useState(false);
  const [sourcePayloadForm, setSourcePayloadForm] = useState<WorkbenchSourcePayloadForm>(() => (
    buildDefaultPayloadForm(initialCorpusId ?? visibleCorpora[0]?.code ?? "", ingestProfiles[0], workspaceCode)
  ));
  const [businessAttributeRows, setBusinessAttributeRows] = useState<WorkbenchBusinessAttributeRow[]>([]);
  const [relatedNodeRows, setRelatedNodeRows] = useState<WorkbenchRelatedNodeRow[]>([]);
  const [relatedExistingNodeRows, setRelatedExistingNodeRows] = useState<WorkbenchExistingGraphNodeRow[]>([]);
  const [relatedEdgeRows, setRelatedEdgeRows] = useState<WorkbenchRelatedEdgeRow[]>([]);
  const initialRetrievalId = initialRetrievalProfileId && retrievalProfiles.some((profile) => profile.id === initialRetrievalProfileId)
    ? initialRetrievalProfileId
    : retrievalProfiles[0]?.id ?? "";
  const [retrievalProfileId, setRetrievalProfileId] = useState(initialRetrievalProfileId ?? initialRetrievalId);
  const [query, setQuery] = useState(() => pageT("defaults.query", "How should the RAG context pack respect tokenBudget?"));
  const [contextStrategy, setContextStrategy] = useState(retrievalProfiles[0]?.config.contextStrategies[0] ?? "DOC_QA_CONTEXT_PACK");
  const [selectedReturnFields, setSelectedReturnFields] = useState<string[]>([]);
  const [filterRows, setFilterRows] = useState<WorkbenchFilterRow[]>([]);
  const [limit, setLimit] = useState("8");
  const [tokenBudget, setTokenBudget] = useState("4096");
  const [ingestLoading, setIngestLoading] = useState(false);
  const [searchLoading, setSearchLoading] = useState(false);
  const [ingestResult, setIngestResult] = useState<RagIngestResponseApi | null>(null);
  const [searchResult, setSearchResult] = useState<RagSearchResponseApi | null>(null);
  const [ingestFeedback, setIngestFeedback] = useState("");
  const [searchFeedback, setSearchFeedback] = useState("");
  const [activeWorkbenchPanel, setActiveWorkbenchPanel] = useState<WorkbenchPanel>(initialPanel);
  const appliedInitialCorpusIdRef = useRef<string | undefined>(undefined);
  const appliedInitialIngestProfileIdRef = useRef<string | undefined>(undefined);
  const appliedInitialRetrievalProfileIdRef = useRef<string | undefined>(undefined);
  const { loading: ingestProfileDetailLoading, profile: ingestProfileDetail } = useRagProfileDetail(ingestProfileId);
  const { loading: retrievalProfileDetailLoading, profile: retrievalProfileDetail } = useRagProfileDetail(retrievalProfileId);
  const selectedIngestCorpus = corpora.find((corpus) => corpus.code === corpusId);
  const selectedIngestProfile = ingestProfileDetail?.type === "Ingest"
    ? ingestProfileDetail
    : ingestProfiles.find((profile) => profile.id === ingestProfileId);
  const selectedRetrievalProfile = retrievalProfileDetail?.type === "Retrieval"
    ? retrievalProfileDetail
    : retrievalProfiles.find((profile) => profile.id === retrievalProfileId);
  const relatedGraphSupported = corpusSupportsRelatedGraph(selectedIngestCorpus);
  const relatedGraphRefOptions = collectRelatedGraphRefOptions(relatedNodeRows, relatedExistingNodeRows);
  const relatedGraphPreview = relatedGraphSupported
    ? buildRelatedGraphPreview(relatedNodeRows, relatedExistingNodeRows, relatedEdgeRows)
    : undefined;
  const usesDocumentPayload = isDocumentPayloadProfile(selectedIngestProfile);
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
    .filter((option) => !hiddenSourceTypeFields.has(option.field))
    .filter((option, index, options) => options.findIndex((item) => item.field === option.field) === index)
    .sort((left, right) => left.field.localeCompare(right.field));
  const filterFieldSchema = Object.fromEntries(filterFieldOptions.map((option) => [option.field, option]));
  const businessFieldOptions = collectWorkbenchFieldOptions(
    selectedIngestCorpus ? [selectedIngestCorpus] : [],
    workbenchFixedPayloadFields,
  );
  const businessFieldSchema = Object.fromEntries(businessFieldOptions.map((option) => [option.field, option]));
  const returnFieldOptions = collectWorkbenchFieldOptions(selectedRetrievalCorpora, workbenchReservedReturnFields);
  const returnFieldSelectOptions = returnFieldOptions.map((option) => ({
    description: option.type,
    label: option.field,
    value: option.field,
  }));
  const targetScope = buildTargetScope(targetScopeType, userCode, workspaceCode);

  useEffect(() => {
    if (initialCorpusId && visibleCorpora.some((corpus) => corpus.code === initialCorpusId)) {
      if (appliedInitialCorpusIdRef.current !== initialCorpusId) {
        appliedInitialCorpusIdRef.current = initialCorpusId;
        if (corpusId !== initialCorpusId) {
          setCorpusId(initialCorpusId);
          resetIngestPayloadRows();
          setSourcePayloadEdited(false);
        }
      }
      return;
    }
    if (!corpusId && initialVisibleCorpusId) {
      setCorpusId(initialVisibleCorpusId);
    }
  }, [corpusId, initialCorpusId, initialVisibleCorpusId, visibleCorpora]);

  useEffect(() => {
    if (initialIngestProfileId && ingestProfiles.some((profile) => profile.id === initialIngestProfileId)) {
      if (appliedInitialIngestProfileIdRef.current !== initialIngestProfileId) {
        appliedInitialIngestProfileIdRef.current = initialIngestProfileId;
        if (ingestProfileId !== initialIngestProfileId) {
          setIngestProfileId(initialIngestProfileId);
          resetIngestPayloadRows();
          setSourcePayloadEdited(false);
        }
      }
      return;
    }
    if (!ingestProfileId && initialIngestId) {
      setIngestProfileId(initialIngestId);
    }
  }, [ingestProfileId, initialIngestId, initialIngestProfileId, ingestProfiles]);

  useEffect(() => {
    if (initialRetrievalProfileId && retrievalProfiles.some((profile) => profile.id === initialRetrievalProfileId)) {
      if (appliedInitialRetrievalProfileIdRef.current !== initialRetrievalProfileId) {
        appliedInitialRetrievalProfileIdRef.current = initialRetrievalProfileId;
        if (retrievalProfileId !== initialRetrievalProfileId) {
          setRetrievalProfileId(initialRetrievalProfileId);
          setContextStrategy("RESULT_LIST");
          setFilterRows([]);
          setSelectedReturnFields([]);
        }
      }
      return;
    }
    if (!retrievalProfileId && initialRetrievalId) {
      setRetrievalProfileId(initialRetrievalId);
    }
  }, [initialRetrievalId, initialRetrievalProfileId, retrievalProfileId, retrievalProfiles]);

  useEffect(() => {
    setActiveWorkbenchPanel(initialPanel);
  }, [initialPanel]);

  useEffect(() => {
    if (!sourcePayloadEdited) {
      setSourcePayloadForm(buildDefaultPayloadForm(corpusId, selectedIngestProfile, workspaceCode));
    }
  }, [corpusId, selectedIngestProfile, sourcePayloadEdited, workspaceCode]);

  useEffect(() => {
    if (!contextStrategyOptions.includes(contextStrategy)) {
      setContextStrategy(contextStrategyOptions[0] ?? "DOC_QA_CONTEXT_PACK");
    }
  }, [contextStrategy, contextStrategyOptions]);

  function toggleReturnField(field: string) {
    setSelectedReturnFields((current) => (
      current.includes(field)
        ? current.filter((currentField) => currentField !== field)
        : [...current, field]
    ));
  }

  function updateSourcePayloadForm(patch: Partial<WorkbenchSourcePayloadForm>) {
    setSourcePayloadForm((current) => ({ ...current, ...patch }));
    setSourcePayloadEdited(true);
  }

  function resetIngestPayloadRows() {
    setBusinessAttributeRows([]);
    setRelatedNodeRows([]);
    setRelatedExistingNodeRows([]);
    setRelatedEdgeRows([]);
  }

  function updateRelatedNodeRow(formId: string, patch: Partial<WorkbenchRelatedNodeRow>) {
    setRelatedNodeRows((current) => current.map((row) => (
      row.formId === formId ? { ...row, ...patch } : row
    )));
  }

  function updateRelatedExistingNodeRow(formId: string, patch: Partial<WorkbenchExistingGraphNodeRow>) {
    setRelatedExistingNodeRows((current) => current.map((row) => (
      row.formId === formId ? { ...row, ...patch } : row
    )));
  }

  function updateRelatedEdgeRow(formId: string, patch: Partial<WorkbenchRelatedEdgeRow>) {
    setRelatedEdgeRows((current) => current.map((row) => (
      row.formId === formId ? { ...row, ...patch } : row
    )));
  }

  function buildSearchFilters() {
    const filters: Record<string, unknown> = {};
    filterRows.forEach((row, index) => {
      const field = row.field.trim();
      const operator = row.operator.trim();
      const rawValue = row.value.trim();
      if (!field && !operator && !rawValue) return;
      if (!field || !operator || !rawValue) {
        throw new Error(pageT("errors.filterRequired", "Filter {{index}} must include field, operator, and value.", { index: index + 1 }));
      }

      const schema = filterFieldSchema[field];
      const valueType = schema?.type ?? "string";
      const value = operator === "in"
        ? parseCommaSeparatedValues(rawValue).map((item) => parseFilterValue(item, valueType, field))
        : parseFilterValue(rawValue, valueType, field);
      if (operator === "in" && Array.isArray(value) && value.length === 0) {
        throw new Error(pageT("errors.filterValueRequired", "Filter {{index}} needs at least one value.", { index: index + 1 }));
      }
      appendFilter(filters, field, { op: operator, value });
    });
    return filters;
  }

  async function handleIngestSubmit() {
    setIngestFeedback("");
    let domainFields: Record<string, unknown>;
    let relatedGraph: Record<string, unknown> | undefined;
    try {
      domainFields = buildDomainFields(businessAttributeRows, businessFieldSchema);
      relatedGraph = relatedGraphSupported
        ? buildRelatedGraphFromRows(relatedNodeRows, relatedExistingNodeRows, relatedEdgeRows)
        : undefined;
    } catch (error) {
      const message = readErrorMessage(error);
      setIngestFeedback(message);
      onNotice(message);
      return;
    }
    const payload = buildPayloadFromForm(sourcePayloadForm, usesDocumentPayload, domainFields, relatedGraph);
    if (!sourcePayloadForm.sourceCode.trim() || !sourcePayloadForm.sourceVersion.trim()) {
      const message = pageT("errors.sourceIdentityRequired", "Source code and version are required.");
      setIngestFeedback(message);
      onNotice(message);
      return;
    }
    if (usesDocumentPayload && (!sourcePayloadForm.documentUri.trim() || !sourcePayloadForm.documentFormat.trim())) {
      const message = pageT("errors.documentRequired", "Document URI and format are required.");
      setIngestFeedback(message);
      onNotice(message);
      return;
    }
    if (!usesDocumentPayload && !sourcePayloadForm.text.trim()) {
      const message = pageT("errors.sourceTextRequired", "Source text is required.");
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
      const message = pageT("messages.ingestCompleted", "{{code}} ingest completed.", { code: result.sourceCode || corpusId });
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
      const message = pageT("messages.searchCompleted", "Retrieval returned {{count}} hits.", { count: hitCount });
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
      {hidePanelSwitcher ? null : (
        <div className="rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-1">
          <div className="grid gap-1 sm:grid-cols-2" role="tablist" aria-label={pageT("tabs.ariaLabel", "Workbench mode")}>
            {workbenchPanels.map((panel) => {
              const selected = activeWorkbenchPanel === panel.value;
              return (
                <button
                  aria-controls={`rag-workbench-${panel.value}-panel`}
                  aria-selected={selected}
                  className={`rounded-xl px-4 py-3 text-left transition ${selected
                    ? "bg-[var(--color-elevated-surface)] shadow-sm"
                    : "text-[var(--color-on-surface-variant)] hover:bg-[var(--color-surface)]"
                    }`}
                  id={`rag-workbench-${panel.value}-tab`}
                  key={panel.value}
                  role="tab"
                  type="button"
                  onClick={() => setActiveWorkbenchPanel(panel.value)}
                >
                  <span className="block font-medium">{pageT(`tabs.${panel.value}.label`, panel.label)}</span>
                  <span className="mt-1 block text-[13px] text-[var(--color-on-surface-variant)]">{pageT(`tabs.${panel.value}.summary`, panel.summary)}</span>
                </button>
              );
            })}
          </div>
        </div>
      )}

      {activeWorkbenchPanel === "ingest" ? (
        <SurfaceCard>
          <div
            aria-label={hidePanelSwitcher ? pageT("ingest.title", "Ingest request") : undefined}
            aria-labelledby={hidePanelSwitcher ? undefined : "rag-workbench-ingest-tab"}
            id="rag-workbench-ingest-panel"
            role="tabpanel"
          >
            <SectionTitle
              icon={<Upload size={17} aria-hidden="true" />}
              title={pageT("ingest.title", "Ingest request")}
            />
            <div className="mt-4 grid gap-3">
              <Field label={pageT("fields.corpus", "Corpus")} hint={hintT("corpus")}>
                {lockIngestContext ? (
                  <TextInput
                    aria-label={pageT("fields.corpus", "Corpus")}
                    className="font-mono"
                    readOnly
                    value={corpusId}
                  />
                ) : (
                  <SelectInput
                    aria-label={pageT("fields.corpus", "Corpus")}
                    value={corpusId}
                    onValueChange={(event) => {
                      setCorpusId(event!);
                      resetIngestPayloadRows();
                      setSourcePayloadEdited(false);
                    }}
                  >
                    {visibleCorpora.map((corpus) => (
                      <SelectItem key={corpus.code} value={corpus.code}>{corpus.code}</SelectItem>
                    ))}
                  </SelectInput>
                )}
              </Field>
              <Field label={pageT("fields.ingestProfile", "Ingest profile")} hint={hintT("ingestProfile")}>
                {lockIngestContext ? (
                  <TextInput
                    aria-label={pageT("fields.ingestProfile", "Ingest profile")}
                    className="font-mono"
                    readOnly
                    value={ingestProfileId}
                  />
                ) : (
                  <SelectInput
                    aria-label={pageT("fields.ingestProfile", "Ingest profile")}
                    value={ingestProfileId}
                    onValueChange={(event) => {
                      setIngestProfileId(event!);
                      resetIngestPayloadRows();
                      setSourcePayloadEdited(false);
                    }}
                  >
                    {ingestProfiles.map((profile) => (
                      <SelectItem key={profile.id} value={profile.id}>{profile.id}</SelectItem>
                    ))}
                  </SelectInput>
                )}
              </Field>
              <Field label={pageT("fields.scopeType", "Scope type")} hint={hintT("scopeType")}>
                <SelectInput
                  aria-label={pageT("fields.scopeType", "Scope type")}
                  value={targetScopeType}
                  onValueChange={(event) => setTargetScopeType(event!)}
                >
                  {workbenchScopeOptions.map((option) => (
                    <SelectItem key={option.value} value={option.value}>{pageT(`scope.${option.value}`, option.label)}</SelectItem>
                  ))}
                </SelectInput>
                <p className="mt-1 text-[12px] text-[var(--color-on-surface-variant)]">
                  {pageT("scope.resolvedCode", "Scope code is resolved automatically:")} <span className="font-mono">{targetScope.scopeCode}</span>
                </p>
              </Field>
              <div className="grid gap-3 sm:grid-cols-2">
                <Field label={pageT("fields.sourceCode", "Source code")} hint={hintT("sourceCode")}>
                  <TextInput aria-label={pageT("fields.sourceCode", "Source code")} value={sourcePayloadForm.sourceCode} onChange={(event) => updateSourcePayloadForm({ sourceCode: event.target.value })} />
                </Field>
                <Field label={pageT("fields.sourceVersion", "Source version")} hint={hintT("sourceVersion")}>
                  <TextInput
                    aria-label={pageT("fields.sourceVersion", "Source version")}
                    disabled={Boolean(requiredSourceVersion)}
                    value={sourcePayloadForm.sourceVersion}
                    onChange={(event) => updateSourcePayloadForm({ sourceVersion: event.target.value })}
                  />
                </Field>
                <Field label={pageT("fields.language", "Language")} hint={hintT("language")}>
                  <TextInput aria-label={pageT("fields.language", "Language")} value={sourcePayloadForm.language} onChange={(event) => updateSourcePayloadForm({ language: event.target.value })} />
                </Field>
              </div>
              <Field label={pageT("fields.metadataSourcePath", "Metadata source path")} hint={hintT("metadataSourcePath")}>
                <TextInput aria-label={pageT("fields.metadataSourcePath", "Metadata source path")} value={sourcePayloadForm.metadataSourcePath} onChange={(event) => updateSourcePayloadForm({ metadataSourcePath: event.target.value })} />
              </Field>
              <div className="rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-3">
                <div className="flex flex-wrap items-center justify-between gap-3">
                  <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">{pageT("businessAttributes.title", "Business attributes")}</p>
                  <Button
                    disabled={businessFieldOptions.length === 0}
                    size="sm"
                    type="button"
                    variant="secondary"
                    onClick={() => setBusinessAttributeRows((current) => [...current, createWorkbenchBusinessAttributeRow(current.length)])}
                  >
                    {pageT("businessAttributes.add", "Add business attribute")}
                  </Button>
                </div>
                {businessAttributeRows.length > 0 ? (
                  <div className="mt-3 grid gap-3">
                    {businessAttributeRows.map((row, index) => {
                      const selectedFieldsInOtherRows = new Set(
                        businessAttributeRows
                          .filter((item) => item.formId !== row.formId)
                          .map((item) => item.field)
                          .filter(Boolean),
                      );
                      const availableBusinessFieldOptions = businessFieldOptions.filter((option) => (
                        option.field === row.field || !selectedFieldsInOtherRows.has(option.field)
                      ));
                      return (
                        <div className="grid gap-3 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] p-3 xl:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_auto]" key={row.formId}>
                          <Field label={pageT("businessAttributes.fieldLabel", "Field")} hint={hintT("businessField")}>
                            <SelectInput
                              aria-label={pageT("businessAttributes.fieldAria", "Business attribute {{index}} field", { index: index + 1 })}
                              placeholder={pageT("fields.selectField", "Select field")}
                              value={row.field}
                              onValueChange={(event) => setBusinessAttributeRows((current) => current.map((item) => (
                                item.formId === row.formId ? { ...item, field: event! } : item
                              )))}
                            >
                              {availableBusinessFieldOptions.map((option) => (
                                <SelectItem key={option.field} value={option.field}>{option.field}</SelectItem>
                              ))}
                            </SelectInput>
                          </Field>
                          <Field label={pageT("businessAttributes.valueLabel", "Value")} hint={hintT("businessValue")}>
                            <TextInput
                              aria-label={pageT("businessAttributes.valueAria", "Business attribute {{index}} value", { index: index + 1 })}
                              value={row.value}
                              onChange={(event) => setBusinessAttributeRows((current) => current.map((item) => (
                                item.formId === row.formId ? { ...item, value: event.target.value } : item
                              )))}
                            />
                          </Field>
                          <Button
                            aria-label={pageT("businessAttributes.removeAria", "Remove business attribute {{index}}", { index: index + 1 })}
                            className="self-end"
                            size="sm"
                            type="button"
                            variant="ghost"
                            onClick={() => setBusinessAttributeRows((current) => current.filter((item) => item.formId !== row.formId))}
                          >
                            {pageT("actions.remove", "Remove")}
                          </Button>
                        </div>
                      );
                    })}
                  </div>
                ) : null}
              </div>
              {relatedGraphSupported ? (
                <div className="rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-3">
                  <div className="flex flex-wrap items-center justify-between gap-3">
                    <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">{pageT("relatedGraph.title", "Related graph")}</p>
                    <div className="flex flex-wrap items-center gap-[12px]">
                      <Button
                        size="sm"
                        type="button"
                        variant="secondary"
                        onClick={() => setRelatedNodeRows((current) => [
                          ...current,
                          createWorkbenchRelatedNodeRow(current.length, sourcePayloadForm.sourceCode),
                        ])}
                      >
                        {pageT("relatedGraph.addNode", "Add node")}
                      </Button>
                      <Button
                        size="sm"
                        type="button"
                        variant="secondary"
                        onClick={() => setRelatedExistingNodeRows((current) => [
                          ...current,
                          createWorkbenchExistingGraphNodeRow(current.length),
                        ])}
                      >
                        {pageT("relatedGraph.addExistingNode", "Add existing node")}
                      </Button>
                      <Button
                        size="sm"
                        type="button"
                        variant="secondary"
                        onClick={() => setRelatedEdgeRows((current) => {
                          const from = relatedNodeRows.find((row) => row.ref.trim())?.ref.trim() || "$target";
                          const to = relatedExistingNodeRows.find((row) => row.ref.trim())?.ref.trim()
                            || relatedNodeRows.find((row) => row.ref.trim() && row.ref.trim() !== from)?.ref.trim()
                            || "$target";
                          return [...current, createWorkbenchRelatedEdgeRow(current.length, from, to)];
                        })}
                      >
                        {pageT("relatedGraph.addEdge", "Add edge")}
                      </Button>
                    </div>
                  </div>
                  {relatedNodeRows.length > 0 ? (
                    <div className="mt-3 grid gap-3">
                      <p className="text-[12px] font-medium text-[var(--color-on-surface-variant)]">{pageT("relatedGraph.nodesTitle", "New nodes")}</p>
                      {relatedNodeRows.map((row, index) => (
                        <div className="grid gap-3 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] p-3" key={row.formId}>
                          <div className="grid gap-3 lg:grid-cols-[minmax(0,1fr)_160px_minmax(0,1fr)_minmax(0,1fr)_auto]">
                            <Field label={pageT("relatedGraph.nodeRef", "Ref")} hint={hintT("relatedGraphNodeRef")}>
                              <TextInput
                                aria-label={pageT("relatedGraph.nodeRefAria", "Related graph node {{index}} ref", { index: index + 1 })}
                                className="font-mono"
                                value={row.ref}
                                onChange={(event) => updateRelatedNodeRow(row.formId, { ref: event.target.value })}
                              />
                            </Field>
                            <Field label={pageT("relatedGraph.nodeKind", "Kind")} hint={hintT("relatedGraphNodeKind")}>
                              <TextInput
                                aria-label={pageT("relatedGraph.nodeKindAria", "Related graph node {{index}} kind", { index: index + 1 })}
                                className="font-mono"
                                value={row.kind}
                                onChange={(event) => updateRelatedNodeRow(row.formId, { kind: event.target.value })}
                              />
                            </Field>
                            <Field label={pageT("relatedGraph.nodeLabelField", "Label")} hint={hintT("relatedGraphNodeLabel")}>
                              <TextInput
                                aria-label={pageT("relatedGraph.nodeLabelAria", "Related graph node {{index}} label", { index: index + 1 })}
                                className="font-mono"
                                value={row.label}
                                onChange={(event) => updateRelatedNodeRow(row.formId, { label: event.target.value })}
                              />
                            </Field>
                            <Field label={pageT("relatedGraph.nodeCode", "Code")} hint={hintT("relatedGraphNodeCode")}>
                              <TextInput
                                aria-label={pageT("relatedGraph.nodeCodeAria", "Related graph node {{index}} code", { index: index + 1 })}
                                className="font-mono"
                                value={row.code}
                                onChange={(event) => updateRelatedNodeRow(row.formId, { code: event.target.value })}
                              />
                            </Field>
                            <Button
                              aria-label={pageT("relatedGraph.removeNodeAria", "Remove related graph node {{index}}", { index: index + 1 })}
                              className="self-end"
                              size="sm"
                              type="button"
                              variant="ghost"
                              onClick={() => setRelatedNodeRows((current) => current.filter((item) => item.formId !== row.formId))}
                            >
                              {pageT("actions.remove", "Remove")}
                            </Button>
                          </div>
                          <div className="grid gap-3 lg:grid-cols-3">
                            <Field label={pageT("relatedGraph.nodeTitle", "Title")}>
                              <TextInput
                                aria-label={pageT("relatedGraph.nodeTitleAria", "Related graph node {{index}} title", { index: index + 1 })}
                                value={row.title}
                                onChange={(event) => updateRelatedNodeRow(row.formId, { title: event.target.value })}
                              />
                            </Field>
                            <Field label={pageT("relatedGraph.nodeText", "Text")}>
                              <TextInput
                                aria-label={pageT("relatedGraph.nodeTextAria", "Related graph node {{index}} text", { index: index + 1 })}
                                value={row.text}
                                onChange={(event) => updateRelatedNodeRow(row.formId, { text: event.target.value })}
                              />
                            </Field>
                            <Field label={pageT("relatedGraph.nodeProperties", "Properties JSON")} hint={hintT("relatedGraphNodeProperties")}>
                              <TextInput
                                aria-label={pageT("relatedGraph.nodePropertiesAria", "Related graph node {{index}} properties JSON", { index: index + 1 })}
                                className="font-mono"
                                placeholder='{"priority":3}'
                                value={row.propertiesJson}
                                onChange={(event) => updateRelatedNodeRow(row.formId, { propertiesJson: event.target.value })}
                              />
                            </Field>
                          </div>
                        </div>
                      ))}
                    </div>
                  ) : null}
                  {relatedExistingNodeRows.length > 0 ? (
                    <div className="mt-3 grid gap-3">
                      <p className="text-[12px] font-medium text-[var(--color-on-surface-variant)]">{pageT("relatedGraph.existingNodesTitle", "Existing node refs")}</p>
                      {relatedExistingNodeRows.map((row, index) => (
                        <div className="grid gap-3 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] p-3 lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_auto]" key={row.formId}>
                          <Field label={pageT("relatedGraph.existingNodeRef", "Ref")} hint={hintT("relatedGraphNodeRef")}>
                            <TextInput
                              aria-label={pageT("relatedGraph.existingNodeRefAria", "Existing graph node {{index}} ref", { index: index + 1 })}
                              className="font-mono"
                              value={row.ref}
                              onChange={(event) => updateRelatedExistingNodeRow(row.formId, { ref: event.target.value })}
                            />
                          </Field>
                          <Field label={pageT("relatedGraph.existingNodeCode", "Code")} hint={hintT("relatedExistingNodeCode")}>
                            <TextInput
                              aria-label={pageT("relatedGraph.existingNodeCodeAria", "Existing graph node {{index}} code", { index: index + 1 })}
                              className="font-mono"
                              value={row.code}
                              onChange={(event) => updateRelatedExistingNodeRow(row.formId, { code: event.target.value })}
                            />
                          </Field>
                          <Button
                            aria-label={pageT("relatedGraph.removeExistingNodeAria", "Remove existing graph node {{index}}", { index: index + 1 })}
                            className="self-end"
                            size="sm"
                            type="button"
                            variant="ghost"
                            onClick={() => setRelatedExistingNodeRows((current) => current.filter((item) => item.formId !== row.formId))}
                          >
                            {pageT("actions.remove", "Remove")}
                          </Button>
                        </div>
                      ))}
                    </div>
                  ) : null}
                  {relatedEdgeRows.length > 0 ? (
                    <div className="mt-3 grid gap-3">
                      <p className="text-[12px] font-medium text-[var(--color-on-surface-variant)]">{pageT("relatedGraph.edgesTitle", "Edges")}</p>
                      {relatedEdgeRows.map((row, index) => (
                        <div className="grid gap-3 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] p-3 xl:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_minmax(0,1fr)_minmax(0,1fr)_auto]" key={row.formId}>
                          <Field label={pageT("relatedGraph.edgeFrom", "From")} hint={hintT("relatedGraphEdgeEndpoint")}>
                            <SelectInput
                              aria-label={pageT("relatedGraph.edgeFromAria", "Related graph edge {{index}} source", { index: index + 1 })}
                              className="font-mono"
                              placeholder={pageT("fields.selectField", "Select field")}
                              value={row.from}
                              onValueChange={(event) => updateRelatedEdgeRow(row.formId, { from: event! })}
                            >
                              {relatedGraphRefOptions.map((ref) => (
                                <SelectItem key={ref} value={ref}>{ref}</SelectItem>
                              ))}
                            </SelectInput>
                          </Field>
                          <Field label={pageT("relatedGraph.edgeTo", "To")} hint={hintT("relatedGraphEdgeEndpoint")}>
                            <SelectInput
                              aria-label={pageT("relatedGraph.edgeToAria", "Related graph edge {{index}} target", { index: index + 1 })}
                              className="font-mono"
                              placeholder={pageT("fields.selectField", "Select field")}
                              value={row.to}
                              onValueChange={(event) => updateRelatedEdgeRow(row.formId, { to: event! })}
                            >
                              {relatedGraphRefOptions.map((ref) => (
                                <SelectItem key={ref} value={ref}>{ref}</SelectItem>
                              ))}
                            </SelectInput>
                          </Field>
                          <Field label={pageT("relatedGraph.relationship", "Relationship")} hint={hintT("relatedGraphRelationship")}>
                            <TextInput
                              aria-label={pageT("relatedGraph.relationshipAria", "Related graph edge {{index}} relationship", { index: index + 1 })}
                              className="font-mono"
                              value={row.relationship}
                              onChange={(event) => updateRelatedEdgeRow(row.formId, { relationship: event.target.value })}
                            />
                          </Field>
                          <Field label={pageT("relatedGraph.edgeProperties", "Properties JSON")} hint={hintT("relatedGraphEdgeProperties")}>
                            <TextInput
                              aria-label={pageT("relatedGraph.edgePropertiesAria", "Related graph edge {{index}} properties JSON", { index: index + 1 })}
                              className="font-mono"
                              placeholder='{"weight":0.8}'
                              value={row.propertiesJson}
                              onChange={(event) => updateRelatedEdgeRow(row.formId, { propertiesJson: event.target.value })}
                            />
                          </Field>
                          <Button
                            aria-label={pageT("relatedGraph.removeEdgeAria", "Remove related graph edge {{index}}", { index: index + 1 })}
                            className="self-end"
                            size="sm"
                            type="button"
                            variant="ghost"
                            onClick={() => setRelatedEdgeRows((current) => current.filter((item) => item.formId !== row.formId))}
                          >
                            {pageT("actions.remove", "Remove")}
                          </Button>
                        </div>
                      ))}
                    </div>
                  ) : null}
                  {relatedGraphPreview ? (
                    <details className="group mt-3 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] text-sm">
                      <summary className="flex cursor-pointer list-none items-center gap-[12px] p-3 marker:hidden [&::-webkit-details-marker]:hidden">
                        <ChevronDown
                          className="shrink-0 transition-transform group-open:rotate-180"
                          size={16}
                          aria-hidden="true"
                        />
                        <span className="text-[12px] font-medium text-[var(--color-on-surface-variant)]">{pageT("relatedGraph.preview", "relatedGraph preview")}</span>
                      </summary>
                      <pre className="max-h-72 overflow-auto border-t border-[var(--color-outline-variant)] p-3 text-[12px] leading-5">
                        {JSON.stringify(relatedGraphPreview, null, 2)}
                      </pre>
                    </details>
                  ) : null}
                </div>
              ) : null}
              {usesDocumentPayload ? (
                <div className="grid gap-3 sm:grid-cols-2">
                  <Field label={pageT("fields.documentUri", "Document URI")} hint={hintT("documentUri")}>
                    <TextInput aria-label={pageT("fields.documentUri", "Document URI")} value={sourcePayloadForm.documentUri} onChange={(event) => updateSourcePayloadForm({ documentUri: event.target.value })} />
                  </Field>
                  <Field label={pageT("fields.documentFormat", "Document format")} hint={hintT("documentFormat")}>
                    <TextInput aria-label={pageT("fields.documentFormat", "Document format")} value={sourcePayloadForm.documentFormat} onChange={(event) => updateSourcePayloadForm({ documentFormat: event.target.value })} />
                  </Field>
                </div>
              ) : (
                <Field label={pageT("fields.sourceText", "Source text")} hint={hintT("sourceText")}>
                  <TextArea aria-label={pageT("fields.sourceText", "Source text")} className="min-h-28" value={sourcePayloadForm.text} onChange={(event) => updateSourcePayloadForm({ text: event.target.value })} />
                </Field>
              )}
              <Button disabled={ingestLoading || ingestProfileDetailLoading || !corpusId || !ingestProfileId} onClick={handleIngestSubmit}>
                {ingestLoading ? pageT("actions.submittingIngest", "Submitting ingest...") : pageT("actions.submitIngest", "Submit ingest")}
              </Button>
            </div>
            {ingestResult ? (
              <div className="mt-5 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4">
                <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">{pageT("ingest.resultTitle", "Ingest result")}</p>
                <div className="mt-3 grid gap-[12px] text-sm">
                  <InspectorRow label={pageT("result.status", "Status")} value={formatRagStatusLabel(ingestResult.status ?? "unknown", t)} />
                  <InspectorRow label={pageT("result.sourceCode", "Source code")} value={ingestResult.sourceCode ?? sourcePayloadForm.sourceCode} />
                  <InspectorRow label={pageT("result.textUnits", "Text units")} value={String(ingestResult.textUnitCount ?? 0)} />
                  <InspectorRow label={pageT("result.fulltextIndex", "Fulltext index")} value={formatRagStatusLabel(ingestResult.fulltextStatus ?? "not started", t)} />
                  <InspectorRow label={pageT("result.vectorIndex", "Vector index")} value={formatRagStatusLabel(ingestResult.embeddingStatus ?? "not started", t)} />
                </div>
              </div>
            ) : null}
            <StageResultsList results={ingestResult?.stageResults} title={pageT("stageResults.ingestTitle", "Ingest stage results")} />
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
            aria-label={hidePanelSwitcher ? pageT("retrieval.title", "Retrieval test") : undefined}
            aria-labelledby={hidePanelSwitcher ? undefined : "rag-workbench-retrieval-tab"}
            id="rag-workbench-retrieval-panel"
            role="tabpanel"
          >
            <SectionTitle
              icon={<Search size={17} aria-hidden="true" />}
              title={pageT("retrieval.title", "Retrieval test")}
            />
            <div className="mt-4 grid gap-3">
              <Field label={pageT("fields.retrievalProfile", "Retrieval profile")} hint={hintT("retrievalProfile")}>
                {lockRetrievalContext ? (
                  <TextInput
                    aria-label={pageT("fields.retrievalProfile", "Retrieval profile")}
                    className="font-mono"
                    readOnly
                    value={retrievalProfileId}
                  />
                ) : (
                  <SelectInput
                    aria-label={pageT("fields.retrievalProfile", "Retrieval profile")}
                    value={retrievalProfileId}
                    onValueChange={(event) => {
                      setRetrievalProfileId(event!);
                      setContextStrategy("RESULT_LIST");
                      setFilterRows([]);
                      setSelectedReturnFields([]);
                    }}
                  >
                    {retrievalProfiles.map((profile) => (
                      <SelectItem key={profile.id} value={profile.id}>{profile.id}</SelectItem>
                    ))}
                  </SelectInput>
                )}
              </Field>
              <Field label={pageT("fields.query", "Query")} hint={hintT("query")}>
                <TextArea aria-label={pageT("fields.query", "Query")} className="min-h-24" value={query} onChange={(event) => setQuery(event.target.value)} />
              </Field>
              <Field label={pageT("fields.contextStrategy", "Context strategy")} hint={hintT("contextStrategy")}>
                <SelectInput aria-label={pageT("fields.contextStrategy", "Context strategy")} value={contextStrategy} onValueChange={(event) => setContextStrategy(event!)}>
                  {contextStrategyOptions.map((strategy) => (
                    <SelectItem key={strategy} value={strategy}>{strategy}</SelectItem>
                  ))}
                </SelectInput>
              </Field>
              <Field label={pageT("returnFields.title", "Return fields")}>
                <MultiSelectDropdown
                  ariaLabel={pageT("returnFields.title", "Return fields")}
                  disabled={returnFieldOptions.length === 0}
                  emptyMessage={pageT("returnFields.empty", "No return fields available.")}
                  options={returnFieldSelectOptions}
                  placeholder={pageT("returnFields.select", "Select return fields")}
                  searchEmptyMessage={pageT("returnFields.searchEmpty", "No matching return fields.")}
                  searchPlaceholder={pageT("returnFields.searchPlaceholder", "Search return fields...")}
                  selectedValues={selectedReturnFields}
                  onToggle={toggleReturnField}
                />
              </Field>
              <div className="rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-3">
                <div className="flex flex-wrap items-center justify-between gap-3">
                  <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">{pageT("filters.title", "Filters")}</p>
                  <Button
                    size="sm"
                    type="button"
                    variant="secondary"
                    onClick={() => setFilterRows((current) => [...current, createWorkbenchFilterRow(current.length)])}
                  >
                    {pageT("filters.add", "Add filter")}
                  </Button>
                </div>
                {filterRows.length > 0 ? (
                  <div className="mt-3 grid gap-3">
                    {filterRows.map((row, index) => {
                      const selectedField = filterFieldSchema[row.field];
                      const operators = selectedField?.operators?.length ? selectedField.operators : [...workbenchFilterOperators];
                      const selectedFieldsInOtherRows = new Set(
                        filterRows
                          .filter((item) => item.formId !== row.formId)
                          .map((item) => item.field)
                          .filter(Boolean),
                      );
                      const availableFilterFieldOptions = filterFieldOptions.filter((option) => (
                        option.field === row.field || !selectedFieldsInOtherRows.has(option.field)
                      ));
                      return (
                        <div className="grid gap-3 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] p-3 lg:grid-cols-[minmax(220px,1fr)_160px_minmax(220px,1fr)_auto]" key={row.formId}>
                          <Field label={pageT("filters.fieldLabel", "Field")} hint={hintT("filterField")}>
                            <SelectInput
                              aria-label={pageT("filters.fieldAria", "Filter {{index}} field", { index: index + 1 })}
                              placeholder={pageT("fields.selectField", "Select field")}
                              value={row.field}
                              onValueChange={(event) => setFilterRows((current) => current.map((item) => (
                                item.formId === row.formId ? { ...item, field: event! } : item
                              )))}
                            >
                              {availableFilterFieldOptions.map((option) => (
                                <SelectItem key={option.field} value={option.field}>{option.field}</SelectItem>
                              ))}
                            </SelectInput>
                          </Field>
                          <Field label={pageT("filters.operatorLabel", "Condition")} hint={hintT("filterOperator")}>
                            <SelectInput
                              aria-label={pageT("filters.operatorAria", "Filter {{index}} condition", { index: index + 1 })}
                              value={row.operator}
                              onValueChange={(event) => setFilterRows((current) => current.map((item) => (
                                item.formId === row.formId ? { ...item, operator: event! } : item
                              )))}
                            >
                              {operators.map((operator) => (
                                <SelectItem key={operator} value={operator}>{operator}</SelectItem>
                              ))}
                            </SelectInput>
                          </Field>
                          <Field label={pageT("filters.valueLabel", "Value")} hint={hintT("filterValue")}>
                            <TextInput
                              aria-label={pageT("filters.valueAria", "Filter {{index}} value", { index: index + 1 })}
                              value={row.value}
                              onChange={(event) => setFilterRows((current) => current.map((item) => (
                                item.formId === row.formId ? { ...item, value: event.target.value } : item
                              )))}
                            />
                          </Field>
                          <Button
                            aria-label={pageT("filters.removeAria", "Remove filter {{index}}", { index: index + 1 })}
                            className="self-end"
                            size="sm"
                            type="button"
                            variant="ghost"
                            onClick={() => setFilterRows((current) => current.filter((item) => item.formId !== row.formId))}
                          >
                            {pageT("actions.remove", "Remove")}
                          </Button>
                        </div>
                      );
                    })}
                  </div>
                ) : null}
              </div>
              <div className="grid gap-3 sm:grid-cols-2">
                <Field label={pageT("fields.limit", "Limit")} hint={hintT("limit")}>
                  <TextInput aria-label={pageT("fields.limit", "Limit")} type="number" value={limit} onChange={(event) => setLimit(event.target.value)} />
                </Field>
                <Field label={pageT("fields.tokenBudget", "Token budget")} hint={hintT("tokenBudget")}>
                  <TextInput aria-label={pageT("fields.tokenBudget", "Token budget")} type="number" value={tokenBudget} onChange={(event) => setTokenBudget(event.target.value)} />
                </Field>
              </div>
              <ToggleSwitch checked={traceEnabled} label={pageT("fields.traceEnabled", "Enable tracing")} onCheckedChange={onTraceEnabledChange} />
              <Button disabled={searchLoading || retrievalProfileDetailLoading || !retrievalProfileId || !query.trim()} onClick={handleSearchSubmit}>
                {searchLoading ? pageT("actions.searching", "Searching...") : pageT("actions.runSearch", "Run retrieval")}
              </Button>
            </div>
            <div className="mt-5 grid gap-3">
              <PreviewLine title={pageT("preview.firstHit", "First hit")} value={searchResult?.hits?.[0]?.title ?? searchResult?.hits?.[0]?.sourceCode ?? pageT("preview.waiting", "Waiting")} />
              <PreviewLine title={pageT("preview.contextPack", "Context pack")} value={String(searchResult?.contextPack?.tokenCount ?? 0)} />
              <PreviewLine title={pageT("preview.traceCode", "Trace code")} value={searchResult?.traceCode ?? (traceEnabled ? pageT("preview.enabled", "Enabled") : pageT("preview.disabled", "Disabled"))} />
            </div>
            <StageResultsList results={searchResult?.stageResults} title={pageT("stageResults.retrievalTitle", "Retrieval stage results")} />
            {searchHits.length > 0 ? (
              <div className="mt-5 space-y-3">
                <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">{pageT("results.title", "Retrieval results")}</p>
                {searchHits.map((hit, index) => {
                  const returnedFields = selectedReturnFields
                    .map((field) => [field, hit[field]] as const)
                    .filter(([, value]) => value !== undefined && value !== null);
                  const treeMetadataRows = treeHitMetadataRows(hit);
                  return (
                    <article
                      className="rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4 text-sm"
                      key={`${hit.corpusId ?? "hit"}-${hit.sourceCode ?? index}-${hit.sourceVersion ?? "v"}`}
                    >
                      <div className="flex flex-wrap items-start justify-between gap-3">
                        <div className="min-w-0">
                          <p className="font-medium text-[var(--color-on-surface)]">
                            {hit.title ?? hit.sourceCode ?? pageT("results.hitFallback", "Hit {{index}}", { index: index + 1 })}
                          </p>
                          <p className="mt-1 break-all font-mono text-[12px] text-[var(--color-on-surface-variant)]">
                            {[hit.corpusId, hit.sourceCode, hit.sourceVersion].filter(Boolean).join(" · ")}
                          </p>
                        </div>
                        <span className="rounded-full border border-[var(--color-outline-variant)] px-2.5 py-1 font-mono text-[12px] text-[var(--color-on-surface-variant)]">
                          {pageT("results.score", "Score {{score}}", { score: formatScore(hit.score) })}
                        </span>
                      </div>
                      {returnedFields.length > 0 ? (
                        <dl className="mt-3 grid gap-[12px] rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] p-3 sm:grid-cols-2">
                          {returnedFields.map(([field, value]) => (
                            <div key={field}>
                              <dt className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">{field}</dt>
                              <dd className="mt-1 break-all text-[var(--color-on-surface)]">{formatFieldValue(value)}</dd>
                            </div>
                          ))}
                        </dl>
                      ) : null}
                      {treeMetadataRows.length > 0 ? (
                        <dl className="mt-3 grid gap-[12px] rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] p-3 sm:grid-cols-2">
                          {treeMetadataRows.map((row) => (
                            <div key={row.label}>
                              <dt className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">{row.label}</dt>
                              <dd className="mt-1 break-all text-[var(--color-on-surface)]">{formatFieldValue(row.value)}</dd>
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
                <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">{pageT("contextItems.title", "Context pack items")}</p>
                {contextPackItems.map((item, index) => (
                  <article
                    className="rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4 text-sm"
                    key={`${item.sourceCode ?? "context"}-${index}`}
                  >
                    <div className="flex flex-wrap items-center justify-between gap-3">
                      <p className="break-all font-mono text-[12px] text-[var(--color-on-surface-variant)]">
                        {item.sourceCode ?? pageT("contextItems.itemFallback", "Context item {{index}}", { index: index + 1 })}
                      </p>
                      <span className="font-mono text-[12px] text-[var(--color-on-surface-variant)]">
                        {pageT("contextItems.tokens", "{{count}} tokens", { count: item.tokens ?? 0 })}
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
