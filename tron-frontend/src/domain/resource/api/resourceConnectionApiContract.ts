import type {
  ResourceActivationApiResult,
  ResourceScopeCode,
} from "./resourceApiContract";

export type DataSourceDefinitionCode =
  | "MYSQL_CONNECTION"
  | "POSTGRESQL_CONNECTION"
  | "REDIS_CONNECTION"
  | "KAFKA_CONNECTION";

export type ServerAuthenticationType = "Password" | "PrivateKey";

export interface DataSourceCreateApiRequest {
  scopeKind: ResourceScopeCode;
  workspaceCode: string | null;
  resourceCode: string;
  definitionCode: DataSourceDefinitionCode;
  name: string;
  description: string | null;
  attributes: Record<string, unknown>;
  allowInheritance: boolean;
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
  scopeKind: ResourceScopeCode;
  workspaceCode: string | null;
  serverCode: string;
  name: string;
  description: string | null;
  host: string;
  port: number;
  allowInheritance: boolean;
  initialAccount: ServerAccountCreateInputApiRequest;
}

export interface ServerAccountCreateApiRequest {
  scopeKind: ResourceScopeCode;
  workspaceCode: string | null;
  serverCode: string;
  account: ServerAccountCreateInputApiRequest;
}

export interface ServerCreateApiResult {
  server: ResourceActivationApiResult;
  initialAccount: ResourceActivationApiResult;
  connectivityVerified: boolean;
}
