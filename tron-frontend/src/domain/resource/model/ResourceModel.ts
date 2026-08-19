export type ResourceKind =
  | "FILE"
  | "IMAGE"
  | "PDF"
  | "MYSQL_CONNECTION"
  | "POSTGRESQL_CONNECTION"
  | "REDIS_CONNECTION"
  | "KAFKA_CONNECTION"
  | "SERVER_CONNECTION"
  | "SERVER_ACCOUNT";

export type ResourceSourceKind = "LOCAL" | "INHERITED" | "OVERRIDE";

export interface ResourceFileRecord {
  fileName: string;
  fileExtension: string;
  byteSize: number;
}

export interface ResourceRecord {
  workspaceCode: string;
  resourceCode: string;
  kind: ResourceKind;
  name: string;
  description: string;
  sourceKind: ResourceSourceKind;
  allowInheritance: boolean;
  file: ResourceFileRecord | null;
  attributes: Record<string, unknown> | null;
  createdAt: string;
  updatedAt: string;
}
