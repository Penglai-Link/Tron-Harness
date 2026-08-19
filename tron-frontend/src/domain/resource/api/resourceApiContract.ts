export type ResourceSourceCode = "LOCAL" | "INHERITED" | "OVERRIDE";
export type ResourceKindCode =
  | "FILE"
  | "IMAGE"
  | "PDF"
  | "MYSQL_CONNECTION"
  | "POSTGRESQL_CONNECTION"
  | "REDIS_CONNECTION"
  | "KAFKA_CONNECTION"
  | "SERVER_CONNECTION"
  | "SERVER_ACCOUNT";

export interface ResourceApiPage<T> {
  current: number;
  size: number;
  total: number;
  records: T[];
}

export interface ResourceApiFileRecord {
  fileName: string;
  fileExtension: string;
  byteSize: number;
}

export interface ResourceApiRecord {
  workspaceCode: string;
  sourceKind: ResourceSourceCode;
  resourceCode: string;
  kind: ResourceKindCode;
  name: string;
  description: string;
  allowInheritance: boolean;
  file: ResourceApiFileRecord | null;
  attributes: Record<string, unknown> | null;
  createdAt: number;
  updatedAt: number;
}

export interface ResourcePageApiRequest {
  workspaceCode: string;
  kind: ResourceKindCode | null;
  search: string | null;
  current: number;
  size: number;
}

export interface ResourceGetApiRequest {
  workspaceCode: string;
  resourceCode: string;
}

export interface ResourceCreateApiRequest {
  workspaceCode: string;
  resourceCode: string;
  kind: "FILE" | "IMAGE" | "PDF";
  name: string;
  description: string;
  allowInheritance: boolean;
  file: File;
}

export interface ResourceUpdateApiRequest {
  workspaceCode: string;
  resourceCode: string;
  name?: string | null;
  description?: string | null;
  allowInheritance?: boolean | null;
}

export interface ResourceCommitApiResult {
  resourceCode: string;
  kind: ResourceKindCode;
}

export interface ResourceDeleteApiRequest {
  workspaceCode: string;
  resourceCode: string;
}

export interface ResourceDeleteApiResult {
  resourceCode: string;
  deleted: boolean;
}

export interface ResourceBinaryUploadApiRequest {
  workspaceCode: string;
  resourceCode: string;
  file: File;
}

export interface ResourceBinaryUploadApiResult {
  action: "UPLOADED";
  resourceCode: string;
  fileName: string;
  byteSize: number;
  targetPath: string | null;
}

export interface ResourceBinaryDownloadApiRequest {
  workspaceCode: string;
  resourceCode: string;
}
