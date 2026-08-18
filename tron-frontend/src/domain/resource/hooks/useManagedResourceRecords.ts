import { useMemo } from "react";

import { pageResources } from "@/domain/resource/api";
import { listWorkspaceResourceVersions } from "@/domain/resource/mock";
import type {
  ResourceScopeCode,
  ResourceSourceCode,
  ResourceVersionApiRecord,
} from "@/domain/resource/api";

import { useResourceApiQuery } from "./useResourceApiQuery";

export interface ManagedResourceRecord {
  attributes: Record<string, unknown>;
  definitionCode: string;
  definitionName: string;
  description: string;
  name: string;
  resourceCode: string;
  resourceVersion: string;
  scopeKind: ResourceScopeCode;
  sourceKind: ResourceSourceCode;
  updatedAt: number;
}

interface ManagedResourceQuery {
  definitionCodes: readonly string[];
  search: string;
  workspaceCode: string | null;
  workspaceName: string;
}

function toManagedRecord(record: ResourceVersionApiRecord): ManagedResourceRecord {
  return {
    attributes: record.attributes ?? {},
    definitionCode: record.definitionCode,
    definitionName: record.definitionName,
    description: record.description,
    name: record.name,
    resourceCode: record.resourceCode,
    resourceVersion: record.resourceVersion,
    scopeKind: record.scopeKind,
    sourceKind: record.sourceKind,
    updatedAt: record.updatedAt,
  };
}

function mockAttributes(resourceCode: string, definitionCode: string): Record<string, unknown> {
  if (definitionCode === "MYSQL_CONNECTION") {
    return { connectionMode: "SINGLE", database: "tron", endpoints: [{ host: "mysql.internal", port: 3306 }] };
  }
  if (definitionCode === "POSTGRESQL_CONNECTION") {
    return { database: "analytics", endpoints: [{ host: "postgres.internal", port: 5432 }] };
  }
  if (definitionCode === "REDIS_CONNECTION") {
    return { database: 0, endpoints: [{ host: "redis.internal", port: 6379 }], topology: "STANDALONE" };
  }
  if (definitionCode === "KAFKA_CONNECTION") {
    return { brokers: [{ host: "kafka.internal", port: 9092 }], securityProtocol: "SASL_SSL" };
  }
  if (definitionCode === "SERVER_CONNECTION") {
    return { host: resourceCode.includes("build") ? "10.20.0.18" : "10.20.0.12", port: 22, protocol: "Ssh" };
  }
  if (definitionCode === "SERVER_ACCOUNT") {
    return {
      authenticationType: resourceCode.includes("deploy") ? "PrivateKey" : "Password",
      serverCode: resourceCode.includes("build") ? "build-server" : "production-server",
      username: resourceCode.includes("deploy") ? "deploy" : "ops",
    };
  }
  return {};
}

function mockRecords(query: ManagedResourceQuery): ManagedResourceRecord[] {
  const search = query.search.trim().toLowerCase();
  return listWorkspaceResourceVersions(query.workspaceCode ?? "", query.workspaceName)
    .filter((record) => record.lifecycleStatus === "ACTIVE")
    .filter((record) => query.definitionCodes.includes(record.definitionCode))
    .filter((record) => !search || [record.name, record.resourceCode, record.description]
      .some((value) => value.toLowerCase().includes(search)))
    .map((record) => ({
      attributes: mockAttributes(record.resourceCode, record.definitionCode),
      definitionCode: record.definitionCode,
      definitionName: record.definitionName,
      description: record.description,
      name: record.name,
      resourceCode: record.resourceCode,
      resourceVersion: record.resourceVersion,
      scopeKind: record.scopeKind,
      sourceKind: record.sourceKind,
      updatedAt: Date.parse(record.updatedAt),
    }));
}

export function useManagedResourceRecords(query: ManagedResourceQuery) {
  const definitionKey = query.definitionCodes.join("|");
  const result = useResourceApiQuery<ManagedResourceRecord[]>({
    api: async () => {
      const pages = await Promise.all(query.definitionCodes.map((definitionCode) => pageResources({
        current: 1,
        definitionCode,
        definitionStatus: null,
        lifecycleStatus: "ACTIVE",
        resourceCode: null,
        scopeKind: "WORKSPACE",
        search: query.search.trim() || null,
        size: 200,
        sourceKind: null,
        validationStatus: null,
        workspaceCode: query.workspaceCode,
      })));
      return pages
        .flatMap((page) => page.records.map(toManagedRecord))
        .sort((left, right) => right.updatedAt - left.updatedAt);
    },
    deps: [definitionKey, query.search, query.workspaceCode],
    enabled: Boolean(query.workspaceCode),
    initialData: () => [],
    mock: () => mockRecords(query),
  });

  return useMemo(() => result, [result]);
}
