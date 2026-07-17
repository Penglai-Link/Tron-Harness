import type {
  ResourceBinaryMapping,
  ResourceVersionDetail,
  ResourceVersionRecord,
} from "@/domain/resource/model/ResourceModel";
import {
  createWorkspaceResourceDefinitionDraft,
  getWorkspaceResourceDefinitionVersionDetail,
  listPlatformResourceDefinitionVersionDetails,
  listWorkspaceResourceDefinitionVersionDetails,
} from "./resourceDefinitionData";
import { createJsonSchemaDraft7FormValue } from "@/shared/ui/json-schema-draft7";

const workspaceResourceVersions: ResourceVersionRecord[] = [
  {
    definitionCode: "PRODUCT_IMAGES",
    definitionName: "Product Images",
    definitionVersion: "v2-P4d",
    description: "Product gallery assets used by the storefront.",
    draftRevision: null,
    lifecycleStatus: "ACTIVE",
    name: "Storefront Product Gallery",
    ownerWorkspaceCode: "TRON_CORE",
    ownerWorkspaceName: "Tron Core",
    resourceCode: "storefront-product-gallery",
    resourceVersion: "v4-M7q",
    scopeKind: "WORKSPACE",
    sourceKind: "LOCAL",
    updatedAt: "2026-07-17T03:18:00Z",
    validationIssueCount: 0,
    validationStatus: "VALID",
  },
  {
    definitionCode: "MYSQL_CONNECTION",
    definitionName: "MySQL Connection",
    definitionVersion: "v4-a7K",
    description: "Primary application database connection.",
    draftRevision: null,
    lifecycleStatus: "ACTIVE",
    name: "Production MySQL",
    ownerWorkspaceCode: null,
    ownerWorkspaceName: null,
    resourceCode: "production-mysql",
    resourceVersion: "v3-A8n",
    scopeKind: "PLATFORM",
    sourceKind: "PLATFORM",
    updatedAt: "2026-07-16T09:34:00Z",
    validationIssueCount: 0,
    validationStatus: "VALID",
  },
  {
    definitionCode: "FILE",
    definitionName: "File",
    definitionVersion: "v1-F8s",
    description: "Shared customer import templates inherited from Platform.",
    draftRevision: null,
    lifecycleStatus: "ACTIVE",
    name: "Customer Import Templates",
    ownerWorkspaceCode: "PLATFORM",
    ownerWorkspaceName: "Platform",
    resourceCode: "customer-import-templates",
    resourceVersion: "v2-P9x",
    scopeKind: "WORKSPACE",
    sourceKind: "INHERITED",
    updatedAt: "2026-07-16T07:12:00Z",
    validationIssueCount: 0,
    validationStatus: "VALID",
  },
  {
    definitionCode: "PDF",
    definitionName: "PDF Document",
    definitionVersion: "v2-L6q",
    description: "Workspace-specific policy documents overriding the upstream set.",
    draftRevision: null,
    lifecycleStatus: "ACTIVE",
    name: "Knowledge Policy PDFs",
    ownerWorkspaceCode: "TRON_CORE",
    ownerWorkspaceName: "Tron Core",
    resourceCode: "knowledge-policy-pdfs",
    resourceVersion: "v5-R2c",
    scopeKind: "WORKSPACE",
    sourceKind: "OVERRIDE",
    updatedAt: "2026-07-15T11:28:00Z",
    validationIssueCount: 0,
    validationStatus: "VALID",
  },
  {
    definitionCode: "POSTGRESQL_CONNECTION",
    definitionName: "PostgreSQL Connection",
    definitionVersion: "v3-N8x",
    description: "Analytics warehouse inherited from the Platform workspace.",
    draftRevision: null,
    lifecycleStatus: "ACTIVE",
    name: "Analytics Warehouse",
    ownerWorkspaceCode: "PLATFORM",
    ownerWorkspaceName: "Platform",
    resourceCode: "analytics-warehouse",
    resourceVersion: "v2-N6v",
    scopeKind: "WORKSPACE",
    sourceKind: "INHERITED",
    updatedAt: "2026-07-14T15:06:00Z",
    validationIssueCount: 0,
    validationStatus: "VALID",
  },
  {
    definitionCode: "IMAGE",
    definitionName: "Image Collection",
    definitionVersion: "v2-M3p",
    description: "Platform brand marks and product identity images.",
    draftRevision: null,
    lifecycleStatus: "ACTIVE",
    name: "Brand Logo Library",
    ownerWorkspaceCode: null,
    ownerWorkspaceName: null,
    resourceCode: "brand-logo-library",
    resourceVersion: "v2-K4p",
    scopeKind: "PLATFORM",
    sourceKind: "PLATFORM",
    updatedAt: "2026-07-13T08:46:00Z",
    validationIssueCount: 0,
    validationStatus: "VALID",
  },
  {
    definitionCode: "REDIS_CONNECTION",
    definitionName: "Redis Connection",
    definitionVersion: "v2-D9n",
    description: "Session cache maintained by this Workspace.",
    draftRevision: null,
    lifecycleStatus: "ACTIVE",
    name: "Session Cache",
    ownerWorkspaceCode: "TRON_CORE",
    ownerWorkspaceName: "Tron Core",
    resourceCode: "session-cache",
    resourceVersion: "v1-D3s",
    scopeKind: "WORKSPACE",
    sourceKind: "LOCAL",
    updatedAt: "2026-07-12T04:22:00Z",
    validationIssueCount: 0,
    validationStatus: "VALID",
  },
  {
    definitionCode: "PRODUCT_IMAGES",
    definitionName: "Product Images",
    definitionVersion: "v2-P4d",
    description: "Draft refresh of storefront product assets.",
    draftRevision: 6,
    lifecycleStatus: "DRAFT",
    name: "Storefront Product Gallery",
    ownerWorkspaceCode: "TRON_CORE",
    ownerWorkspaceName: "Tron Core",
    resourceCode: "storefront-product-gallery",
    resourceVersion: "v5-X7b",
    scopeKind: "WORKSPACE",
    sourceKind: "LOCAL",
    updatedAt: "2026-07-17T05:06:00Z",
    validationIssueCount: 0,
    validationStatus: "VALID",
  },
  {
    definitionCode: "FILE",
    definitionName: "File",
    definitionVersion: "v1-F8s",
    description: "New archive for raw customer uploads.",
    draftRevision: 2,
    lifecycleStatus: "DRAFT",
    name: "Customer Upload Archive",
    ownerWorkspaceCode: "TRON_CORE",
    ownerWorkspaceName: "Tron Core",
    resourceCode: "customer-upload-archive",
    resourceVersion: "v1-Q2m",
    scopeKind: "WORKSPACE",
    sourceKind: "LOCAL",
    updatedAt: "2026-07-17T04:41:00Z",
    validationIssueCount: 2,
    validationStatus: "INVALID",
  },
  {
    definitionCode: "MYSQL_CONNECTION",
    definitionName: "MySQL Connection",
    definitionVersion: "v4-a7K",
    description: "Read-only reporting database under preparation.",
    draftRevision: 1,
    lifecycleStatus: "DRAFT",
    name: "Reporting MySQL",
    ownerWorkspaceCode: "TRON_CORE",
    ownerWorkspaceName: "Tron Core",
    resourceCode: "reporting-mysql",
    resourceVersion: "v1-H8k",
    scopeKind: "WORKSPACE",
    sourceKind: "LOCAL",
    updatedAt: "2026-07-16T12:32:00Z",
    validationIssueCount: 0,
    validationStatus: "VALID",
  },
  {
    definitionCode: "PDF",
    definitionName: "PDF Document",
    definitionVersion: "v2-L6q",
    description: "Draft update with incomplete policy metadata.",
    draftRevision: 3,
    lifecycleStatus: "DRAFT",
    name: "Knowledge Policy PDFs",
    ownerWorkspaceCode: "TRON_CORE",
    ownerWorkspaceName: "Tron Core",
    resourceCode: "knowledge-policy-pdfs",
    resourceVersion: "v6-C4t",
    scopeKind: "WORKSPACE",
    sourceKind: "OVERRIDE",
    updatedAt: "2026-07-16T10:18:00Z",
    validationIssueCount: 1,
    validationStatus: "INVALID",
  },
  {
    definitionCode: "PRODUCT_IMAGES",
    definitionName: "Product Images",
    definitionVersion: "v2-P4d",
    description: "Previous storefront product gallery snapshot.",
    draftRevision: null,
    lifecycleStatus: "HISTORICAL",
    name: "Storefront Product Gallery",
    ownerWorkspaceCode: "TRON_CORE",
    ownerWorkspaceName: "Tron Core",
    resourceCode: "storefront-product-gallery",
    resourceVersion: "v3-V5d",
    scopeKind: "WORKSPACE",
    sourceKind: "LOCAL",
    updatedAt: "2026-06-23T07:26:00Z",
    validationIssueCount: 0,
    validationStatus: "VALID",
  },
  {
    definitionCode: "IMAGE",
    definitionName: "Image Collection",
    definitionVersion: "v1-J8r",
    description: "Original platform brand logo collection.",
    draftRevision: null,
    lifecycleStatus: "HISTORICAL",
    name: "Brand Logo Library",
    ownerWorkspaceCode: null,
    ownerWorkspaceName: null,
    resourceCode: "brand-logo-library",
    resourceVersion: "v1-B8q",
    scopeKind: "PLATFORM",
    sourceKind: "PLATFORM",
    updatedAt: "2026-06-18T03:44:00Z",
    validationIssueCount: 0,
    validationStatus: "VALID",
  },
  {
    definitionCode: "PDF",
    definitionName: "PDF Document",
    definitionVersion: "v2-L6q",
    description: "Previous Workspace policy document snapshot.",
    draftRevision: null,
    lifecycleStatus: "HISTORICAL",
    name: "Knowledge Policy PDFs",
    ownerWorkspaceCode: "TRON_CORE",
    ownerWorkspaceName: "Tron Core",
    resourceCode: "knowledge-policy-pdfs",
    resourceVersion: "v4-L6p",
    scopeKind: "WORKSPACE",
    sourceKind: "OVERRIDE",
    updatedAt: "2026-06-11T09:08:00Z",
    validationIssueCount: 0,
    validationStatus: "VALID",
  },
  {
    definitionCode: "MYSQL_CONNECTION",
    definitionName: "MySQL Connection",
    definitionVersion: "v3-Q8m",
    description: "Previous production database connection snapshot.",
    draftRevision: null,
    lifecycleStatus: "HISTORICAL",
    name: "Production MySQL",
    ownerWorkspaceCode: null,
    ownerWorkspaceName: null,
    resourceCode: "production-mysql",
    resourceVersion: "v2-G7r",
    scopeKind: "PLATFORM",
    sourceKind: "PLATFORM",
    updatedAt: "2026-05-28T02:16:00Z",
    validationIssueCount: 0,
    validationStatus: "VALID",
  },
];

const platformResourceVersions: ResourceVersionRecord[] = [
  ...workspaceResourceVersions.filter((record) => record.scopeKind === "PLATFORM"),
  {
    definitionCode: "FILE",
    definitionName: "File",
    definitionVersion: "v1-F8s",
    description: "Shared customer import templates published by Platform.",
    draftRevision: null,
    lifecycleStatus: "ACTIVE",
    name: "Customer Import Templates",
    ownerWorkspaceCode: null,
    ownerWorkspaceName: null,
    resourceCode: "customer-import-templates",
    resourceVersion: "v2-P9x",
    scopeKind: "PLATFORM",
    sourceKind: "PLATFORM",
    updatedAt: "2026-07-16T07:12:00Z",
    validationIssueCount: 0,
    validationStatus: "VALID",
  },
  {
    definitionCode: "POSTGRESQL_CONNECTION",
    definitionName: "PostgreSQL Connection",
    definitionVersion: "v3-N8x",
    description: "Platform analytics warehouse connection inherited by workspaces.",
    draftRevision: null,
    lifecycleStatus: "ACTIVE",
    name: "Analytics Warehouse",
    ownerWorkspaceCode: null,
    ownerWorkspaceName: null,
    resourceCode: "analytics-warehouse",
    resourceVersion: "v2-N6v",
    scopeKind: "PLATFORM",
    sourceKind: "PLATFORM",
    updatedAt: "2026-07-14T15:06:00Z",
    validationIssueCount: 0,
    validationStatus: "VALID",
  },
  {
    definitionCode: "FILE",
    definitionName: "File",
    definitionVersion: "v1-F8s",
    description: "Platform support bundle under preparation.",
    draftRevision: 2,
    lifecycleStatus: "DRAFT",
    name: "Support Bundle",
    ownerWorkspaceCode: null,
    ownerWorkspaceName: null,
    resourceCode: "support-bundle",
    resourceVersion: "v1-T4m",
    scopeKind: "PLATFORM",
    sourceKind: "PLATFORM",
    updatedAt: "2026-07-17T05:38:00Z",
    validationIssueCount: 1,
    validationStatus: "INVALID",
  },
];

export function listWorkspaceResourceVersions(workspaceCode: string, workspaceName: string) {
  if (!workspaceCode) return [];

  return workspaceResourceVersions.map((record) => {
    if (record.ownerWorkspaceCode !== "TRON_CORE") return { ...record };
    return {
      ...record,
      ownerWorkspaceCode: workspaceCode,
      ownerWorkspaceName: workspaceName || record.ownerWorkspaceName,
    };
  });
}

export function listPlatformResourceVersions() {
  return platformResourceVersions.map(clone);
}

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

function attributesForRecord(record: ResourceVersionRecord) {
  if (record.definitionCode === "PRODUCT_IMAGES") {
    return {
      altText: record.lifecycleStatus === "DRAFT" ? "Storefront product gallery" : "Product gallery image",
      imageType: "GALLERY",
      locale: "zh-CN",
      productCode: "TRON-STOREFRONT",
      tags: record.lifecycleStatus === "HISTORICAL" ? ["storefront"] : ["storefront", "product"],
    };
  }
  if (record.definitionCode === "MYSQL_CONNECTION") {
    return {
      connectionMode: "REPLICATION",
      credentials: {
        password: "DATABASE_PASSWORD",
        username: "DATABASE_USERNAME",
      },
      database: record.resourceCode === "reporting-mysql" ? "reporting" : "tron_production",
      endpoints: [
        { host: "mysql-primary.internal", port: 3306, role: "PRIMARY" },
        { host: "mysql-replica.internal", port: 3306, role: "REPLICA" },
      ],
      properties: {
        connectTimeout: 5000,
        rewriteBatchedStatements: true,
        useUnicode: "true",
      },
      tls: {
        caCertificate: "MYSQL_CA_CERTIFICATE",
        clientCertificate: "MYSQL_CLIENT_CERTIFICATE",
        clientKey: "MYSQL_CLIENT_KEY",
        mode: "VERIFY_IDENTITY",
      },
    };
  }
  if (record.definitionCode === "PDF") {
    return {
      description: "Workspace policy document maintained by the knowledge team.",
      fileExtension: "pdf",
      fileName: record.lifecycleStatus === "DRAFT" ? "" : "workspace-policy.pdf",
      tags: ["policy", "knowledge"],
      title: "Workspace Knowledge Policy",
    };
  }
  if (record.definitionCode === "FILE") {
    const incompleteDraft = record.resourceCode === "customer-upload-archive" || record.resourceCode === "support-bundle";
    return {
      description: incompleteDraft ? "" : "Template used by customer import workflows.",
      fileExtension: record.resourceCode === "support-bundle" ? "zip" : "xlsx",
      fileName: incompleteDraft ? "" : "customer-import-template.xlsx",
      tags: incompleteDraft ? [] : ["customer", "import"],
    };
  }
  return {};
}

function binariesForRecord(record: ResourceVersionRecord): ResourceBinaryMapping[] {
  if (record.definitionCode === "PRODUCT_IMAGES") {
    const names = record.lifecycleStatus === "HISTORICAL"
      ? ["hero-front.png", "hero-side.png"]
      : record.lifecycleStatus === "DRAFT"
        ? ["hero-front.png", "hero-side.png", "hero-detail.png", "hero-mobile.png"]
        : ["hero-front.png", "hero-side.png", "hero-detail.png"];
    return names.map((binaryName) => ({ binaryName }));
  }
  if (record.definitionCode === "PDF") return [{ binaryName: "workspace-policy.pdf" }];
  if (record.definitionCode === "FILE") {
    const incompleteDraft = record.resourceCode === "customer-upload-archive" || record.resourceCode === "support-bundle";
    return incompleteDraft ? [] : [{ binaryName: "customer-import-template.xlsx" }];
  }
  if (record.definitionCode === "IMAGE") {
    return ["tron-mark.svg", "tron-wordmark.svg", "tron-avatar.png"].map((binaryName) => ({ binaryName }));
  }
  return [];
}

function definitionForRecord(
  record: ResourceVersionRecord,
  workspaceCode: string,
  workspaceName: string,
) {
  return getWorkspaceResourceDefinitionVersionDetail(
    workspaceCode,
    workspaceName,
    record.definitionCode,
    record.definitionVersion,
  ) ?? listWorkspaceResourceDefinitionVersionDetails(workspaceCode, workspaceName)
    .find((definition) => definition.definitionCode === record.definitionCode && definition.lifecycleStatus === "ACTIVE")
    ?? createWorkspaceResourceDefinitionDraft(workspaceCode, workspaceName);
}

function detailForRecord(
  record: ResourceVersionRecord,
  workspaceCode: string,
  workspaceName: string,
): ResourceVersionDetail {
  const definition = definitionForRecord(record, workspaceCode, workspaceName);
  return detailWithDefinition(record, definition);
}

function detailWithDefinition(
  record: ResourceVersionRecord,
  definition: ReturnType<typeof definitionForRecord>,
): ResourceVersionDetail {
  const createdAt = new Date(Date.parse(record.updatedAt) - 14 * 24 * 60 * 60 * 1000).toISOString();
  const fixtureAttributes = attributesForRecord(record);
  return {
    ...clone(record),
    allowInheritance: true,
    attributes: Object.keys(fixtureAttributes).length
      ? fixtureAttributes
      : createJsonSchemaDraft7FormValue(definition.attributesSchema),
    attributesSchema: clone(definition.attributesSchema),
    binaries: binariesForRecord(record),
    binarySchema: clone(definition.binarySchema),
    createdAt,
    publishedAt: record.lifecycleStatus === "DRAFT" ? null : record.updatedAt,
  };
}

function detailForPlatformRecord(record: ResourceVersionRecord) {
  const definitions = listPlatformResourceDefinitionVersionDetails();
  const definition = definitions.find((item) => (
    item.definitionCode === record.definitionCode && item.definitionVersion === record.definitionVersion
  )) ?? definitions.find((item) => (
    item.definitionCode === record.definitionCode && item.lifecycleStatus === "ACTIVE"
  )) ?? createWorkspaceResourceDefinitionDraft("PLATFORM", "Platform");
  return detailWithDefinition(record, definition);
}

export function listWorkspaceResourceVersionDetails(workspaceCode: string, workspaceName: string) {
  return listWorkspaceResourceVersions(workspaceCode, workspaceName)
    .map((record) => detailForRecord(record, workspaceCode, workspaceName));
}

export function listPlatformResourceVersionDetails() {
  return listPlatformResourceVersions().map(detailForPlatformRecord);
}

export function getWorkspaceResourceVersionDetail(
  workspaceCode: string,
  workspaceName: string,
  resourceCode: string,
  resourceVersion: string,
) {
  return listWorkspaceResourceVersionDetails(workspaceCode, workspaceName)
    .find((record) => record.resourceCode === resourceCode && record.resourceVersion === resourceVersion) ?? null;
}

export function createWorkspaceResourceDraft(workspaceCode: string, workspaceName: string): ResourceVersionDetail {
  const now = "2026-07-17T06:00:00Z";
  const emptyDefinition = createWorkspaceResourceDefinitionDraft(workspaceCode, workspaceName);
  return {
    allowInheritance: true,
    attributes: createJsonSchemaDraft7FormValue(emptyDefinition.attributesSchema),
    attributesSchema: clone(emptyDefinition.attributesSchema),
    binaries: [],
    binarySchema: clone(emptyDefinition.binarySchema),
    createdAt: now,
    definitionCode: "",
    definitionName: "",
    definitionVersion: "",
    description: "",
    draftRevision: 1,
    lifecycleStatus: "DRAFT",
    name: "",
    ownerWorkspaceCode: workspaceCode,
    ownerWorkspaceName: workspaceName,
    publishedAt: null,
    resourceCode: "",
    resourceVersion: "v1-B9q",
    scopeKind: "WORKSPACE",
    sourceKind: "LOCAL",
    updatedAt: now,
    validationIssueCount: 2,
    validationStatus: "INVALID",
  };
}
