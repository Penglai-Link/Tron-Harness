import type { ResourceRecord } from "@/domain/resource/model/ResourceModel";

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

const workspaceResources: ResourceRecord[] = [
  {
    allowInheritance: false,
    attributes: null,
    createdAt: "2026-07-10T02:10:00Z",
    description: "Product gallery assets used by the storefront.",
    file: { byteSize: 482_304, fileExtension: "png", fileName: "hero-front.png" },
    kind: "IMAGE",
    name: "Storefront Product Gallery",
    resourceCode: "storefront-product-gallery",
    sourceKind: "LOCAL",
    updatedAt: "2026-07-17T03:18:00Z",
    workspaceCode: "TRON_CORE",
  },
  {
    allowInheritance: true,
    attributes: { connectionMode: "SINGLE", database: "tron", endpoints: [{ host: "mysql.internal", port: 3306 }] },
    createdAt: "2026-07-08T01:22:00Z",
    description: "Primary application database connection.",
    file: null,
    kind: "MYSQL_CONNECTION",
    name: "Production MySQL",
    resourceCode: "production-mysql",
    sourceKind: "LOCAL",
    updatedAt: "2026-07-16T09:34:00Z",
    workspaceCode: "TRON_CORE",
  },
  {
    allowInheritance: true,
    attributes: null,
    createdAt: "2026-07-05T06:40:00Z",
    description: "Shared customer import templates inherited from the parent workspace.",
    file: { byteSize: 63_488, fileExtension: "xlsx", fileName: "customer-import-template.xlsx" },
    kind: "FILE",
    name: "Customer Import Templates",
    resourceCode: "customer-import-templates",
    sourceKind: "INHERITED",
    updatedAt: "2026-07-16T07:12:00Z",
    workspaceCode: "TRON_CORE",
  },
  {
    allowInheritance: false,
    attributes: { fileName: "workspace-policy.pdf", pageCount: 18, sizeBytes: 184_320 },
    createdAt: "2026-07-04T08:02:00Z",
    description: "Workspace-specific policy documents overriding the upstream set.",
    file: { byteSize: 184_320, fileExtension: "pdf", fileName: "workspace-policy.pdf" },
    kind: "PDF",
    name: "Knowledge Policy PDFs",
    resourceCode: "knowledge-policy-pdfs",
    sourceKind: "OVERRIDE",
    updatedAt: "2026-07-15T11:28:00Z",
    workspaceCode: "TRON_CORE",
  },
  {
    allowInheritance: true,
    attributes: { database: "analytics", endpoints: [{ host: "postgres.internal", port: 5432 }] },
    createdAt: "2026-07-03T04:30:00Z",
    description: "Analytics warehouse inherited from the parent workspace.",
    file: null,
    kind: "POSTGRESQL_CONNECTION",
    name: "Analytics Warehouse",
    resourceCode: "analytics-warehouse",
    sourceKind: "INHERITED",
    updatedAt: "2026-07-14T15:06:00Z",
    workspaceCode: "TRON_CORE",
  },
  {
    allowInheritance: false,
    attributes: { database: 0, endpoints: [{ host: "redis.internal", port: 6379 }], topology: "STANDALONE" },
    createdAt: "2026-07-02T03:18:00Z",
    description: "Session cache maintained by this workspace.",
    file: null,
    kind: "REDIS_CONNECTION",
    name: "Session Cache",
    resourceCode: "session-cache",
    sourceKind: "LOCAL",
    updatedAt: "2026-07-12T04:22:00Z",
    workspaceCode: "TRON_CORE",
  },
  {
    allowInheritance: false,
    attributes: { brokers: [{ host: "kafka.internal", port: 9092 }], securityProtocol: "SASL_SSL" },
    createdAt: "2026-07-02T01:04:00Z",
    description: "Event stream connection for workspace automations.",
    file: null,
    kind: "KAFKA_CONNECTION",
    name: "Workspace Event Stream",
    resourceCode: "workspace-kafka",
    sourceKind: "LOCAL",
    updatedAt: "2026-07-12T02:10:00Z",
    workspaceCode: "TRON_CORE",
  },
  {
    allowInheritance: false,
    attributes: { host: "10.20.0.12", port: 22, protocol: "Ssh" },
    createdAt: "2026-07-01T08:44:00Z",
    description: "Production application server managed by this workspace.",
    file: null,
    kind: "SERVER_CONNECTION",
    name: "Production Server",
    resourceCode: "production-server",
    sourceKind: "LOCAL",
    updatedAt: "2026-07-11T09:16:00Z",
    workspaceCode: "TRON_CORE",
  },
  {
    allowInheritance: false,
    attributes: { host: "10.20.0.18", port: 22, protocol: "Ssh" },
    createdAt: "2026-06-30T07:52:00Z",
    description: "Build worker server managed by this workspace.",
    file: null,
    kind: "SERVER_CONNECTION",
    name: "Build Server",
    resourceCode: "build-server",
    sourceKind: "LOCAL",
    updatedAt: "2026-07-10T08:05:00Z",
    workspaceCode: "TRON_CORE",
  },
  {
    allowInheritance: false,
    attributes: { authenticationType: "Password", serverCode: "production-server", username: "ops" },
    createdAt: "2026-07-01T08:42:00Z",
    description: "Operations password account.",
    file: null,
    kind: "SERVER_ACCOUNT",
    name: "Operations Account",
    resourceCode: "production-ops-account",
    sourceKind: "LOCAL",
    updatedAt: "2026-07-11T09:15:00Z",
    workspaceCode: "TRON_CORE",
  },
  {
    allowInheritance: false,
    attributes: { authenticationType: "PrivateKey", serverCode: "build-server", username: "deploy" },
    createdAt: "2026-06-30T07:50:00Z",
    description: "Deployment private-key account.",
    file: null,
    kind: "SERVER_ACCOUNT",
    name: "Deployment Account",
    resourceCode: "build-deploy-account",
    sourceKind: "LOCAL",
    updatedAt: "2026-07-10T08:04:00Z",
    workspaceCode: "TRON_CORE",
  },
];

export function listWorkspaceResources(workspaceCode: string, workspaceName: string) {
  if (!workspaceCode) return [];
  void workspaceName;
  const records = new Map<string, ResourceRecord>();
  for (const record of workspaceResources) {
    const key = mockResourceKey(workspaceCode, record.resourceCode);
    if (!mockDeletedResourceKeys.has(key)) {
      records.set(record.resourceCode, { ...clone(record), workspaceCode });
    }
  }
  for (const record of mockResourceOverrides) {
    if (record.workspaceCode === workspaceCode) {
      records.set(record.resourceCode, clone(record));
    }
  }
  return [...records.values()];
}

export function getWorkspaceResourceDetail(
  workspaceCode: string,
  workspaceName: string,
  resourceCode: string,
) {
  return listWorkspaceResources(workspaceCode, workspaceName)
    .find((record) => record.resourceCode === resourceCode) ?? null;
}

export interface MockQuickResourceCreateInput {
  allowInheritance: boolean;
  description: string;
  file: File;
  kind: "FILE" | "IMAGE" | "PDF";
  name: string;
  pageCount: number | null;
  resourceCode: string;
  workspaceCode: string;
}

export interface MockManagedResourceCreateInput {
  allowInheritance: boolean;
  attributes: Record<string, unknown> | null;
  description: string;
  file: ResourceRecord["file"];
  kind: ResourceRecord["kind"];
  name: string;
  resourceCode: string;
  workspaceCode: string;
}

const mockResourceOverrides: ResourceRecord[] = [];
const mockDeletedResourceKeys = new Set<string>();

function mockResourceKey(workspaceCode: string, resourceCode: string) {
  return `${workspaceCode}:${resourceCode}`;
}

function upsertMockResource(record: ResourceRecord) {
  const index = mockResourceOverrides.findIndex((candidate) => (
    candidate.workspaceCode === record.workspaceCode
    && candidate.resourceCode === record.resourceCode
  ));
  if (index >= 0) mockResourceOverrides.splice(index, 1, clone(record));
  else mockResourceOverrides.unshift(clone(record));
  mockDeletedResourceKeys.delete(mockResourceKey(record.workspaceCode, record.resourceCode));
}

export function createMockQuickResource(input: MockQuickResourceCreateInput): ResourceRecord {
  const now = new Date().toISOString();
  const attributes = input.kind === "PDF"
    ? { fileName: input.file.name, pageCount: input.pageCount ?? 0, sizeBytes: input.file.size }
    : null;
  const record: ResourceRecord = {
    allowInheritance: input.allowInheritance,
    attributes,
    createdAt: now,
    description: input.description,
    file: {
      byteSize: input.file.size,
      fileExtension: input.file.name.includes(".") ? input.file.name.slice(input.file.name.lastIndexOf(".") + 1).toLowerCase() : "",
      fileName: input.file.name,
    },
    kind: input.kind,
    name: input.name,
    resourceCode: input.resourceCode,
    sourceKind: "LOCAL",
    updatedAt: now,
    workspaceCode: input.workspaceCode,
  };
  upsertMockResource(record);
  return clone(record);
}

export function createMockManagedResource(input: MockManagedResourceCreateInput): ResourceRecord {
  const now = new Date().toISOString();
  const record: ResourceRecord = {
    ...input,
    createdAt: now,
    sourceKind: "LOCAL",
    updatedAt: now,
  };
  upsertMockResource(record);
  return clone(record);
}

export function listMockCreatedResources(workspaceCode: string) {
  return mockResourceOverrides
    .filter((record) => record.workspaceCode === workspaceCode)
    .map(clone);
}

export function updateMockResource(
  workspaceCode: string,
  resourceCode: string,
  changes: Pick<ResourceRecord, "allowInheritance" | "attributes" | "description" | "name">,
) {
  const current = getWorkspaceResourceDetail(workspaceCode, "", resourceCode);
  if (!current || current.sourceKind !== "LOCAL") return null;
  const updated: ResourceRecord = {
    ...current,
    ...changes,
    updatedAt: new Date().toISOString(),
  };
  upsertMockResource(updated);
  return clone(updated);
}

export function replaceMockResourceFile(
  workspaceCode: string,
  resourceCode: string,
  file: File,
) {
  const current = getWorkspaceResourceDetail(workspaceCode, "", resourceCode);
  if (!current || current.sourceKind !== "LOCAL" || !current.file) return null;
  const updated: ResourceRecord = {
    ...current,
    file: {
      byteSize: file.size,
      fileExtension: file.name.includes(".") ? file.name.slice(file.name.lastIndexOf(".") + 1).toLowerCase() : "",
      fileName: file.name,
    },
    updatedAt: new Date().toISOString(),
  };
  upsertMockResource(updated);
  return clone(updated);
}

export function deleteMockResource(workspaceCode: string, resourceCode: string) {
  const current = getWorkspaceResourceDetail(workspaceCode, "", resourceCode);
  if (!current || current.sourceKind !== "LOCAL") return false;
  mockDeletedResourceKeys.add(mockResourceKey(workspaceCode, resourceCode));
  const index = mockResourceOverrides.findIndex((candidate) => (
    candidate.workspaceCode === workspaceCode && candidate.resourceCode === resourceCode
  ));
  if (index >= 0) mockResourceOverrides.splice(index, 1);
  return true;
}
