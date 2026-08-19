import type { ResourceKindCode } from "./resourceApiContract";

export type DataSourceKindCode =
  | "MYSQL_CONNECTION"
  | "POSTGRESQL_CONNECTION"
  | "REDIS_CONNECTION"
  | "KAFKA_CONNECTION";

export type ServerAuthenticationType = "Password" | "PrivateKey";

export interface ConnectionCommitApiResult {
  workspaceCode: string;
  resourceCode: string;
  kind: ResourceKindCode;
}

export interface ConnectionDeleteApiRequest {
  workspaceCode: string;
  resourceCode: string;
}

export interface ConnectionDeleteApiResult {
  resourceCode: string;
  deleted: boolean;
}

export interface DataSourceCreateApiRequest {
  workspaceCode: string;
  resourceCode: string;
  kind: DataSourceKindCode;
  name: string;
  description: string | null;
  attributes: Record<string, unknown>;
  allowInheritance: boolean;
}

export interface DataSourceUpdateApiRequest {
  workspaceCode: string;
  resourceCode: string;
  name?: string | null;
  description?: string | null;
  attributes?: Record<string, unknown> | null;
  allowInheritance?: boolean | null;
}

export interface ServerAccountCreateInputApiRequest {
  accountCode: string;
  name: string;
  description: string | null;
  username: string;
  authenticationType: ServerAuthenticationType;
  passwordVariableKey?: string;
  privateKeyVariableKey?: string;
  privateKeyPassphraseVariableKey?: string;
  allowInheritance: boolean;
}

export interface ServerCreateApiRequest {
  workspaceCode: string;
  serverCode: string;
  name: string;
  description: string | null;
  host: string;
  port: number;
  allowInheritance: boolean;
  initialAccount: ServerAccountCreateInputApiRequest;
}

export interface ServerAccountCreateApiRequest {
  workspaceCode: string;
  serverCode: string;
  account: ServerAccountCreateInputApiRequest;
}

export interface ServerCreateApiResult {
  server: ConnectionCommitApiResult;
  initialAccount: ConnectionCommitApiResult;
  connectivityVerified: boolean;
}
