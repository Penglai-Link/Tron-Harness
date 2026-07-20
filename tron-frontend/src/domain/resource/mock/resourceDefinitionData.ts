import type {
  ResourceDefinitionAttributesSchema,
  ResourceDefinitionBinarySchema,
  ResourceDefinitionVersionDetail,
  ResourceDefinitionVersionRecord,
} from "@/domain/resource/model";

const workspaceDefinitionVersions: ResourceDefinitionVersionRecord[] = [
  {
    definitionCode: "PRODUCT_IMAGES",
    definitionVersion: "v3-K7m",
    description: "Product gallery images and presentation metadata.",
    draftRevision: 4,
    enabled: true,
    lifecycleStatus: "DRAFT",
    name: "Product Images",
    ownerWorkspaceCode: "TRON_CORE",
    ownerWorkspaceName: "Tron Core",
    preset: false,
    scopeKind: "WORKSPACE",
    sourceKind: "LOCAL",
    updatedAt: "2026-07-17T02:42:00Z",
    validationIssueCount: 0,
    validationStatus: "VALID",
  },
  {
    definitionCode: "PRODUCT_IMAGES",
    definitionVersion: "v2-P4d",
    description: "Product gallery images and presentation metadata.",
    draftRevision: null,
    enabled: true,
    lifecycleStatus: "ACTIVE",
    name: "Product Images",
    ownerWorkspaceCode: "TRON_CORE",
    ownerWorkspaceName: "Tron Core",
    preset: false,
    scopeKind: "WORKSPACE",
    sourceKind: "LOCAL",
    updatedAt: "2026-07-14T08:20:00Z",
    validationIssueCount: 0,
    validationStatus: "VALID",
  },
  {
    definitionCode: "MYSQL_CONNECTION",
    definitionVersion: "v4-a7K",
    description: "Connection information for MySQL middleware.",
    draftRevision: null,
    enabled: true,
    lifecycleStatus: "ACTIVE",
    name: "MySQL Connection",
    ownerWorkspaceCode: null,
    ownerWorkspaceName: null,
    preset: true,
    scopeKind: "PLATFORM",
    sourceKind: "PLATFORM",
    updatedAt: "2026-07-16T06:05:00Z",
    validationIssueCount: 0,
    validationStatus: "VALID",
  },
  {
    definitionCode: "POSTGRESQL_CONNECTION",
    definitionVersion: "v3-N8x",
    description: "Reusable PostgreSQL endpoints, credentials, and TLS options.",
    draftRevision: null,
    enabled: true,
    lifecycleStatus: "ACTIVE",
    name: "PostgreSQL Connection",
    ownerWorkspaceCode: "PLATFORM",
    ownerWorkspaceName: "Platform",
    preset: true,
    scopeKind: "WORKSPACE",
    sourceKind: "INHERITED",
    updatedAt: "2026-07-15T11:36:00Z",
    validationIssueCount: 0,
    validationStatus: "VALID",
  },
  {
    definitionCode: "PDF",
    definitionVersion: "v3-R2c",
    description: "PDF documents with searchable business metadata.",
    draftRevision: 2,
    enabled: true,
    lifecycleStatus: "DRAFT",
    name: "PDF Document",
    ownerWorkspaceCode: "TRON_CORE",
    ownerWorkspaceName: "Tron Core",
    preset: false,
    scopeKind: "WORKSPACE",
    sourceKind: "LOCAL",
    updatedAt: "2026-07-17T01:18:00Z",
    validationIssueCount: 2,
    validationStatus: "INVALID",
  },
  {
    definitionCode: "PDF",
    definitionVersion: "v2-L6q",
    description: "PDF documents with searchable business metadata.",
    draftRevision: null,
    enabled: true,
    lifecycleStatus: "ACTIVE",
    name: "PDF Document",
    ownerWorkspaceCode: "TRON_CORE",
    ownerWorkspaceName: "Tron Core",
    preset: false,
    scopeKind: "WORKSPACE",
    sourceKind: "OVERRIDE",
    updatedAt: "2026-07-12T09:08:00Z",
    validationIssueCount: 0,
    validationStatus: "VALID",
  },
  {
    definitionCode: "FILE",
    definitionVersion: "v1-F8s",
    description: "General files with resource information.",
    draftRevision: null,
    enabled: true,
    lifecycleStatus: "ACTIVE",
    name: "File",
    ownerWorkspaceCode: null,
    ownerWorkspaceName: null,
    preset: true,
    scopeKind: "PLATFORM",
    sourceKind: "PLATFORM",
    updatedAt: "2026-07-10T04:25:00Z",
    validationIssueCount: 0,
    validationStatus: "VALID",
  },
  {
    definitionCode: "IMAGE",
    definitionVersion: "v2-M3p",
    description: "One or more images with caller-defined metadata.",
    draftRevision: null,
    enabled: true,
    lifecycleStatus: "ACTIVE",
    name: "Image Collection",
    ownerWorkspaceCode: null,
    ownerWorkspaceName: null,
    preset: true,
    scopeKind: "PLATFORM",
    sourceKind: "PLATFORM",
    updatedAt: "2026-07-09T07:48:00Z",
    validationIssueCount: 0,
    validationStatus: "VALID",
  },
  {
    definitionCode: "REDIS_CONNECTION",
    definitionVersion: "v2-D9n",
    description: "Redis deployment topology and connection options.",
    draftRevision: null,
    enabled: false,
    lifecycleStatus: "ACTIVE",
    name: "Redis Connection",
    ownerWorkspaceCode: "TRON_CORE",
    ownerWorkspaceName: "Tron Core",
    preset: false,
    scopeKind: "WORKSPACE",
    sourceKind: "LOCAL",
    updatedAt: "2026-07-08T13:02:00Z",
    validationIssueCount: 0,
    validationStatus: "VALID",
  },
  {
    definitionCode: "KAFKA_CONNECTION",
    definitionVersion: "v1-X5b",
    description: "Kafka brokers, security settings, and client properties.",
    draftRevision: 1,
    enabled: true,
    lifecycleStatus: "DRAFT",
    name: "Kafka Connection",
    ownerWorkspaceCode: "TRON_CORE",
    ownerWorkspaceName: "Tron Core",
    preset: false,
    scopeKind: "WORKSPACE",
    sourceKind: "LOCAL",
    updatedAt: "2026-07-07T05:54:00Z",
    validationIssueCount: 1,
    validationStatus: "INVALID",
  },
  {
    definitionCode: "MYSQL_CONNECTION",
    definitionVersion: "v3-Q8m",
    description: "Connection information for MySQL middleware.",
    draftRevision: null,
    enabled: true,
    lifecycleStatus: "HISTORICAL",
    name: "MySQL Connection",
    ownerWorkspaceCode: null,
    ownerWorkspaceName: null,
    preset: true,
    scopeKind: "PLATFORM",
    sourceKind: "PLATFORM",
    updatedAt: "2026-06-21T03:15:00Z",
    validationIssueCount: 0,
    validationStatus: "VALID",
  },
  {
    definitionCode: "PRODUCT_IMAGES",
    definitionVersion: "v1-a7K",
    description: "Product gallery images and presentation metadata.",
    draftRevision: null,
    enabled: true,
    lifecycleStatus: "HISTORICAL",
    name: "Product Images",
    ownerWorkspaceCode: "TRON_CORE",
    ownerWorkspaceName: "Tron Core",
    preset: false,
    scopeKind: "WORKSPACE",
    sourceKind: "LOCAL",
    updatedAt: "2026-06-11T10:42:00Z",
    validationIssueCount: 0,
    validationStatus: "VALID",
  },
];

const platformDefinitionVersions: ResourceDefinitionVersionRecord[] = [
  ...workspaceDefinitionVersions.filter((record) => record.scopeKind === "PLATFORM"),
  {
    definitionCode: "POSTGRESQL_CONNECTION",
    definitionVersion: "v3-N8x",
    description: "Reusable PostgreSQL endpoints, credentials, and TLS options.",
    draftRevision: null,
    enabled: true,
    lifecycleStatus: "ACTIVE",
    name: "PostgreSQL Connection",
    ownerWorkspaceCode: null,
    ownerWorkspaceName: null,
    preset: true,
    scopeKind: "PLATFORM",
    sourceKind: "PLATFORM",
    updatedAt: "2026-07-15T11:36:00Z",
    validationIssueCount: 0,
    validationStatus: "VALID",
  },
  {
    definitionCode: "KAFKA_CONNECTION",
    definitionVersion: "v2-Q3s",
    description: "Next platform Kafka connection contract under preparation.",
    draftRevision: 3,
    enabled: true,
    lifecycleStatus: "DRAFT",
    name: "Kafka Connection",
    ownerWorkspaceCode: null,
    ownerWorkspaceName: null,
    preset: true,
    scopeKind: "PLATFORM",
    sourceKind: "PLATFORM",
    updatedAt: "2026-07-17T05:20:00Z",
    validationIssueCount: 1,
    validationStatus: "INVALID",
  },
];

export function listWorkspaceResourceDefinitionVersions(workspaceCode: string, workspaceName: string) {
  if (!workspaceCode) return [];

  return workspaceDefinitionVersions.map((record) => {
    if (record.ownerWorkspaceCode !== "TRON_CORE") return { ...record };
    return {
      ...record,
      ownerWorkspaceCode: workspaceCode || record.ownerWorkspaceCode,
      ownerWorkspaceName: workspaceName || record.ownerWorkspaceName,
    };
  });
}

export function listPlatformResourceDefinitionVersions() {
  return platformDefinitionVersions.map(clone);
}

const draft7 = "http://json-schema.org/draft-07/schema#" as const;

const schemaByDefinitionCode: Record<string, ResourceDefinitionAttributesSchema> = {
  PRODUCT_IMAGES: {
    $schema: draft7,
    additionalProperties: false,
    properties: {
      productCode: {
        description: "Business identifier of the product represented by this image collection.",
        minLength: 1,
        title: "Product code",
        type: "string",
      },
      imageType: {
        description: "Presentation role assigned by the calling application.",
        enum: ["PRIMARY", "GALLERY", "THUMBNAIL"],
        title: "Image type",
        type: "string",
      },
      altText: {
        description: "Accessible alternative text maintained by the caller.",
        maxLength: 240,
        title: "Alternative text",
        type: "string",
      },
      locale: {
        default: "zh-CN",
        description: "Locale associated with the presentation metadata.",
        title: "Locale",
        type: "string",
      },
      tags: {
        description: "Caller-maintained image classification tags.",
        items: { type: "string" },
        title: "Tags",
        type: "array",
      },
    },
    required: ["productCode", "imageType"],
    title: "Product image metadata",
    type: "object",
  },
  MYSQL_CONNECTION: {
    $schema: draft7,
    additionalProperties: false,
    allOf: [
      {
        if: { properties: { connectionMode: { const: "SINGLE" } }, required: ["connectionMode"] },
        then: { properties: { endpoints: { maxItems: 1 } } },
      },
      {
        if: { properties: { connectionMode: { const: "REPLICATION" } }, required: ["connectionMode"] },
        then: {
          properties: {
            endpoints: {
              allOf: [
                { contains: { properties: { role: { const: "PRIMARY" } }, required: ["role"] } },
                { contains: { properties: { role: { const: "REPLICA" } }, required: ["role"] } },
              ],
              items: { required: ["role"] },
              minItems: 2,
            },
          },
        },
      },
    ],
    properties: {
      connectionMode: {
        enum: ["SINGLE", "FAILOVER", "LOAD_BALANCE", "REPLICATION"],
        title: "Connection mode",
        type: "string",
      },
      endpoints: {
        description: "One or more MySQL server endpoints.",
        items: {
          additionalProperties: false,
          properties: {
            host: { maxLength: 255, minLength: 1, title: "Host", type: "string" },
            port: { maximum: 65535, minimum: 1, title: "Port", type: "integer" },
            role: { enum: ["PRIMARY", "REPLICA"], title: "Role", type: "string" },
          },
          required: ["host", "port"],
          type: "object",
        },
        minItems: 1,
        title: "Endpoints",
        type: "array",
        uniqueItems: true,
      },
      database: {
        maxLength: 64,
        minLength: 1,
        title: "Database",
        type: "string",
      },
      credentials: {
        additionalProperties: false,
        properties: {
          username: { format: "tron-variable-ref", title: "Username variable", type: "string" },
          password: { format: "tron-variable-ref", title: "Password variable", type: "string" },
        },
        required: ["username", "password"],
        title: "Credentials",
        type: "object",
      },
      tls: {
        additionalProperties: false,
        properties: {
          mode: { enum: ["DISABLED", "PREFERRED", "REQUIRED", "VERIFY_CA", "VERIFY_IDENTITY"], title: "TLS mode", type: "string" },
          caCertificate: { format: "tron-variable-ref", title: "CA certificate variable", type: "string" },
          clientCertificate: { format: "tron-variable-ref", title: "Client certificate variable", type: "string" },
          clientKey: { format: "tron-variable-ref", title: "Client key variable", type: "string" },
        },
        required: ["mode"],
        title: "TLS",
        type: "object",
      },
      properties: {
        additionalProperties: { type: ["string", "number", "boolean"] },
        maxProperties: 128,
        propertyNames: { maxLength: 128, minLength: 1 },
        title: "Driver properties",
        type: "object",
      },
    },
    required: ["connectionMode", "endpoints", "database", "credentials", "tls"],
    title: "MySQL connection",
    type: "object",
  },
  PDF: {
    $schema: draft7,
    additionalProperties: false,
    properties: {
      fileName: { minLength: 1, title: "File name", type: "string" },
      sizeBytes: { minimum: 0, title: "File size (bytes)", type: "integer" },
      pageCount: { minimum: 1, title: "Page count", type: "integer" },
      title: { title: "Document title", type: "string" },
      author: { title: "Author", type: "string" },
      pdfVersion: { pattern: "^[0-9]+\\.[0-9]+$", title: "PDF version", type: "string" },
      encrypted: { title: "Encrypted", type: "boolean" },
    },
    required: ["fileName", "sizeBytes", "pageCount"],
    title: "PDF document metadata",
    type: "object",
  },
  FILE: {
    $schema: draft7,
    additionalProperties: false,
    properties: {
      fileName: { minLength: 1, title: "File name", type: "string" },
      fileExtension: { title: "File extension", type: "string" },
      sizeBytes: { minimum: 0, title: "File size (bytes)", type: "integer" },
    },
    required: ["fileName", "sizeBytes"],
    title: "File metadata",
    type: "object",
  },
  IMAGE: {
    $schema: draft7,
    additionalProperties: false,
    properties: {
      images: {
        items: {
          additionalProperties: false,
          properties: {
            binaryName: { maxLength: 128, minLength: 1, title: "Stored file name", type: "string" },
            fileName: { minLength: 1, title: "File name", type: "string" },
            fileExtension: { title: "File extension", type: "string" },
            imageFormat: { enum: ["JPEG", "PNG", "GIF", "WEBP", "SVG", "BMP", "AVIF"], title: "Image format", type: "string" },
            sizeBytes: { minimum: 0, title: "File size (bytes)", type: "integer" },
            width: { minimum: 1, title: "Width", type: "integer" },
            height: { minimum: 1, title: "Height", type: "integer" },
          },
          required: ["binaryName", "fileName", "imageFormat", "sizeBytes"],
          type: "object",
        },
        maxItems: 10,
        minItems: 1,
        title: "Images",
        type: "array",
      },
    },
    required: ["images"],
    title: "Image metadata",
    type: "object",
  },
};

const emptyAttributesSchema: ResourceDefinitionAttributesSchema = {
  $schema: draft7,
  additionalProperties: false,
  properties: {},
  required: [],
  title: "New resource attributes",
  type: "object",
};

const noBinaries: ResourceDefinitionBinarySchema = {
  maxCount: 0,
  maxNameLength: 255,
  maxSizeBytes: 0,
  minCount: 0,
  namePattern: "^[^/]+$",
  storageProfile: "@default",
};

const binarySchemaByDefinitionCode: Record<string, ResourceDefinitionBinarySchema> = {
  FILE: {
    maxCount: 1,
    maxNameLength: 255,
    maxSizeBytes: 104857600,
    minCount: 1,
    namePattern: "^[^/]+$",
    storageProfile: "@default",
  },
  IMAGE: {
    maxCount: 20,
    maxNameLength: 180,
    maxSizeBytes: 20971520,
    minCount: 1,
    namePattern: "^[a-zA-Z0-9._-]+$",
    storageProfile: "@default",
  },
  PDF: {
    maxCount: 1,
    maxNameLength: 180,
    maxSizeBytes: 52428800,
    minCount: 1,
    namePattern: "^[^/]+\\.pdf$",
    storageProfile: "@default",
  },
  PRODUCT_IMAGES: {
    maxCount: 12,
    maxNameLength: 160,
    maxSizeBytes: 15728640,
    minCount: 1,
    namePattern: "^[a-zA-Z0-9._-]+$",
    storageProfile: "@default",
  },
};

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

function effectiveBinaryPolicy(schema: ResourceDefinitionBinarySchema) {
  return {
    maxCount: schema.maxCount,
    maxNameLength: schema.maxNameLength,
    maxSizeBytes: schema.maxSizeBytes,
    minCount: schema.minCount,
    namePattern: schema.namePattern || null,
  };
}

function detailForRecord(record: ResourceDefinitionVersionRecord): ResourceDefinitionVersionDetail {
  const updatedAt = Date.parse(record.updatedAt);
  const createdAt = new Date(updatedAt - (record.lifecycleStatus === "DRAFT" ? 2 : 14) * 24 * 60 * 60 * 1000).toISOString();
  const binarySchema = clone(binarySchemaByDefinitionCode[record.definitionCode] ?? noBinaries);

  return {
    ...clone(record),
    allowInheritance: record.scopeKind === "PLATFORM" || record.sourceKind !== "INHERITED",
    attributesSchema: clone(schemaByDefinitionCode[record.definitionCode] ?? emptyAttributesSchema),
    binarySchema,
    createdAt,
    effectiveBinaryPolicy: effectiveBinaryPolicy(binarySchema),
    publishedAt: record.lifecycleStatus === "DRAFT" ? null : record.updatedAt,
  };
}

export function listWorkspaceResourceDefinitionVersionDetails(workspaceCode: string, workspaceName: string) {
  return listWorkspaceResourceDefinitionVersions(workspaceCode, workspaceName).map(detailForRecord);
}

export function listPlatformResourceDefinitionVersionDetails() {
  return listPlatformResourceDefinitionVersions().map(detailForRecord);
}

export function getWorkspaceResourceDefinitionVersionDetail(
  workspaceCode: string,
  workspaceName: string,
  definitionCode: string,
  definitionVersion: string,
) {
  return listWorkspaceResourceDefinitionVersionDetails(workspaceCode, workspaceName)
    .find((record) => record.definitionCode === definitionCode && record.definitionVersion === definitionVersion) ?? null;
}

export function createWorkspaceResourceDefinitionDraft(workspaceCode: string, workspaceName: string): ResourceDefinitionVersionDetail {
  const now = "2026-07-17T04:00:00Z";
  return {
    allowInheritance: true,
    attributesSchema: clone(emptyAttributesSchema),
    binarySchema: clone(noBinaries),
    createdAt: now,
    effectiveBinaryPolicy: effectiveBinaryPolicy(noBinaries),
    definitionCode: "",
    definitionVersion: "v1-A7q",
    description: "",
    draftRevision: 1,
    enabled: true,
    lifecycleStatus: "DRAFT",
    name: "",
    ownerWorkspaceCode: workspaceCode,
    ownerWorkspaceName: workspaceName,
    preset: false,
    publishedAt: null,
    scopeKind: "WORKSPACE",
    sourceKind: "LOCAL",
    updatedAt: now,
    validationIssueCount: 2,
    validationStatus: "INVALID",
  };
}
