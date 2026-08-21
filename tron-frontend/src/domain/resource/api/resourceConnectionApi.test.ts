// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from "vitest";

import {
  createDataSourceConnection,
  createServerAccountConnection,
  createServerConnection,
  deleteConnection,
  pageConnections,
  updateDataSourceConnection,
} from "@/domain/resource/api";

function successResponse(data: unknown) {
  return new Response(JSON.stringify({ code: "0", data, message: "ok" }), {
    headers: { "Content-Type": "application/json" },
    status: 200,
  });
}

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("Resource connection API client", () => {
  it("pages connections through the dedicated paginated contract", async () => {
    const fetchMock = vi.fn().mockResolvedValueOnce(successResponse({
      current: 2,
      records: [],
      size: 50,
      total: 70,
    }));
    vi.stubGlobal("fetch", fetchMock);

    await pageConnections({
      current: 2,
      kinds: ["MysqlConnection"],
      search: "primary",
      size: 50,
      workspaceCode: "TRON_CORE",
    });

    const [url, request] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(new URL(url).pathname).toBe("/api/v1/resource-connections/page");
    expect(request.body).toBe(JSON.stringify({
      current: 2,
      kinds: ["MysqlConnection"],
      search: "primary",
      size: 50,
      workspaceCode: "TRON_CORE",
    }));
  });

  it("uses the dedicated endpoints and preserves connection commands", async () => {
    const commitResult = {
      kind: "MysqlConnection",
      resourceCode: "production-mysql",
      workspaceCode: "TRON_CORE",
    };
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(successResponse(commitResult))
      .mockResolvedValueOnce(successResponse({ ...commitResult, kind: "ServerConnection" }))
      .mockResolvedValueOnce(successResponse({ ...commitResult, kind: "ServerAccount" }))
      .mockResolvedValueOnce(successResponse({ ...commitResult, kind: "RedisConnection" }))
      .mockResolvedValueOnce(successResponse({ deleted: true, resourceCode: "production-mysql" }));
    vi.stubGlobal("fetch", fetchMock);

    const dataSource = {
      allowInheritance: false,
      attributes: {
        connectionMode: "SINGLE",
        credentials: { password: "mysql-password", username: "mysql-username" },
        database: "tron",
        endpoints: [{ host: "mysql.internal", port: 3306 }],
        tls: { mode: "DISABLED" },
      },
      description: null,
      kind: "MYSQL_CONNECTION" as const,
      name: "Production MySQL",
      resourceCode: "production-mysql",
      workspaceCode: "TRON_CORE",
    };
    const account = {
      accountCode: "deploy-account",
      allowInheritance: false,
      authenticationType: "PrivateKey" as const,
      description: null,
      name: "Deployment Account",
      privateKeyVariableKey: "deploy-private-key",
      username: "deploy",
    };
    const server = {
      allowInheritance: false,
      description: null,
      host: "10.20.0.12",
      name: "Production Server",
      port: 22,
      serverCode: "production-server",
      workspaceCode: "TRON_CORE",
    };

    await createDataSourceConnection(dataSource);
    await createServerConnection(server);
    await createServerAccountConnection({
      account,
      serverCode: "production-server",
      workspaceCode: "TRON_CORE",
    });
    await updateDataSourceConnection({
      description: "Primary database",
      resourceCode: "production-mysql",
      workspaceCode: "TRON_CORE",
    });
    await deleteConnection({
      resourceCode: "production-mysql",
      workspaceCode: "TRON_CORE",
    });

    const calls = fetchMock.mock.calls as [string, RequestInit][];
    expect(calls.map(([url]) => new URL(url).pathname)).toEqual([
      "/api/v1/resource-connections/data-sources",
      "/api/v1/resource-connections/servers",
      "/api/v1/resource-connections/server-accounts",
      "/api/v1/resource-connections/data-sources/update",
      "/api/v1/resource-connections/delete",
    ]);
    expect(calls.map(([, request]) => request.method)).toEqual(["POST", "POST", "POST", "POST", "POST"]);
    expect(calls[0][1].body).toBe(JSON.stringify({ ...dataSource, kind: "MysqlConnection" }));
    expect(calls[1][1].body).toBe(JSON.stringify(server));
    expect(calls[2][1].body).toBe(JSON.stringify({
      account,
      serverCode: "production-server",
      workspaceCode: "TRON_CORE",
    }));
    expect(calls[3][1].body).toBe(JSON.stringify({
      description: "Primary database",
      resourceCode: "production-mysql",
      workspaceCode: "TRON_CORE",
    }));
    expect(calls[4][1].body).toBe(JSON.stringify({
      resourceCode: "production-mysql",
      workspaceCode: "TRON_CORE",
    }));
  });
});
