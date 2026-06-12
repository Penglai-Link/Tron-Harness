export interface RagMockTableSchema {
  columns: string[];
  label: string;
  tableName: string;
}

export interface RagMockGraphSchema {
  label: string;
  namespace: string;
  nodeLabel: string;
  properties: string[];
}

function snakeToCamel(value: string) {
  return value.replace(/_([a-z0-9])/g, (_, token: string) => token.toUpperCase());
}

function camelToSnake(value: string) {
  return value.replace(/([A-Z])/g, "_$1").toLowerCase();
}

export const ragMockTableSchemas: Record<string, RagMockTableSchema> = {
  rag_diagnostics: {
    columns: [
      "record_type",
      "record_code",
      "corpus_id",
      "profile_id",
      "source_code",
      "source_version",
      "stage_id",
      "severity",
      "message",
      "metadata",
      "created_at",
      "expires_at",
    ],
    label: "RAG Diagnostics",
    tableName: "rag_diagnostics",
  },
  rag_memory_text_units: {
    columns: [
      "text_unit_code",
      "source_code",
      "source_version",
      "scope_type",
      "scope_code",
      "unit_type",
      "text",
      "heading_path",
      "ordinal",
      "token_count",
      "content_hash",
      "status",
      "metadata",
      "created_at",
      "updated_at",
      "source_type",
      "source_status",
      "title",
      "source_content_hash",
      "source_metadata",
      "source_created_at",
      "source_updated_at",
      "memory_type",
      "memory_status",
      "review_status",
      "version",
      "superseded_by",
      "conflict_group",
      "duplicate_group",
      "injectable",
      "importance_score",
      "confidence_score",
      "quality_score",
      "pin_level",
      "expires_at",
      "derived_source_type",
      "derived_source_code",
    ],
    label: "Memory Text Units",
    tableName: "rag_memory_text_units",
  },
  rag_project_doc_sources: {
    columns: [
      "source_code",
      "source_version",
      "source_type",
      "scope_type",
      "scope_code",
      "status",
      "title",
      "content_hash",
      "document_code",
      "document_type",
      "path",
      "repo_code",
      "module_code",
      "module_name",
      "body_hash",
      "metadata",
      "created_at",
      "updated_at",
    ],
    label: "Project Document Sources",
    tableName: "rag_project_doc_sources",
  },
  rag_project_doc_text_units: {
    columns: [
      "text_unit_code",
      "source_code",
      "source_version",
      "scope_type",
      "scope_code",
      "unit_type",
      "text",
      "heading_path",
      "ordinal",
      "token_count",
      "content_hash",
      "status",
      "metadata",
      "document_code",
      "created_at",
      "updated_at",
    ],
    label: "Project Document Text Units",
    tableName: "rag_project_doc_text_units",
  },
  rag_skill_text_units: {
    columns: [
      "text_unit_code",
      "source_code",
      "source_version",
      "scope_type",
      "scope_code",
      "source_type",
      "source_status",
      "title",
      "source_content_hash",
      "source_metadata",
      "source_created_at",
      "source_updated_at",
      "skill_name",
      "skill_description",
      "unit_type",
      "text",
      "heading_path",
      "ordinal",
      "token_count",
      "content_hash",
      "status",
      "metadata",
      "created_at",
      "updated_at",
    ],
    label: "Skill Text Units",
    tableName: "rag_skill_text_units",
  },
};

export const ragMockGraphSchemas: Record<string, RagMockGraphSchema> = {
  "rag_doc:RagDocUnit": {
    label: "Project Document Graph Node",
    namespace: "rag_doc",
    nodeLabel: "RagDocUnit",
    properties: [
      "sourceCode",
      "sourceVersion",
      "sourceType",
      "scopeType",
      "scopeCode",
      "sourceStatus",
      "textUnitStatus",
      "documentCode",
      "documentType",
      "path",
      "repoCode",
      "moduleCode",
      "moduleName",
      "text",
      "title",
      "vectorPayload",
      "sourceMetadataJson",
    ],
  },
  "rag_skill:RagSkill": {
    label: "Skill Graph Node",
    namespace: "rag_skill",
    nodeLabel: "RagSkill",
    properties: [
      "sourceCode",
      "sourceVersion",
      "sourceType",
      "scopeType",
      "scopeCode",
      "sourceStatus",
      "textUnitStatus",
      "title",
      "text",
      "skillName",
      "skillDescription",
      "sourceMetadataJson",
    ],
  },
};

export function getMockTableColumns(tableName?: string) {
  if (!tableName) return [];
  return ragMockTableSchemas[tableName]?.columns ?? [];
}

export function hasMockTableColumn(tableName: string | undefined, column: string) {
  const trimmedColumn = column.trim();
  if (!trimmedColumn) return false;
  const columns = getMockTableColumns(tableName);
  if (!columns.length) return true;
  return columns.includes(trimmedColumn);
}

export function isKnownMockTable(tableName?: string) {
  return Boolean(tableName && ragMockTableSchemas[tableName]);
}

export function getMockGraphProperties(namespace?: string, nodeLabel?: string) {
  if (!namespace || !nodeLabel) return [];
  return ragMockGraphSchemas[`${namespace}:${nodeLabel}`]?.properties ?? [];
}

export function hasMockGraphProperty(namespace: string | undefined, nodeLabel: string | undefined, property: string) {
  const trimmedProperty = property.trim();
  if (!trimmedProperty) return false;
  const properties = getMockGraphProperties(namespace, nodeLabel);
  if (!properties.length) return true;
  const runtimeAliases: Record<string, string[]> = {
    metadata: ["sourceMetadataJson"],
    source_metadata: ["sourceMetadataJson"],
    sourceMetadata: ["sourceMetadataJson"],
    status: ["sourceStatus"],
  };
  return [
    trimmedProperty,
    snakeToCamel(trimmedProperty),
    camelToSnake(trimmedProperty),
    ...(runtimeAliases[trimmedProperty] ?? []),
  ].some((candidate) => properties.includes(candidate));
}

export function isKnownMockGraph(namespace?: string, nodeLabel?: string) {
  return Boolean(namespace && nodeLabel && ragMockGraphSchemas[`${namespace}:${nodeLabel}`]);
}
