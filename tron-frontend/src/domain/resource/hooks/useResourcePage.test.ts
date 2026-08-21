// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from "vitest";

import { queryResourceConnectionPage } from "./useResourcePage";

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("resource connection page query", () => {
  it("keeps connection attributes from the dedicated page response", async () => {
    const fetchMock = vi.fn().mockResolvedValueOnce(new Response(JSON.stringify({
      code: "0",
      data: {
        current: 1,
        records: [{
          allowInheritance: false,
          attributes: {
            endpoints: [{ host: "mysql.internal", port: 3306 }],
          },
          createdAt: 1,
          description: "",
          file: null,
          kind: "MysqlConnection",
          name: "Production MySQL",
          resourceCode: "production-mysql",
          sourceKind: "Local",
          updatedAt: 1,
          workspaceCode: "TRON_CORE",
        }],
        size: 15,
        total: 1,
      },
      message: "ok",
    }), {
      headers: { "Content-Type": "application/json" },
      status: 200,
    }));
    vi.stubGlobal("fetch", fetchMock);

    const result = await queryResourceConnectionPage({
      context: "ALL",
      current: 1,
      kinds: ["MYSQL_CONNECTION"],
      search: "",
      size: 15,
      workspaceCode: "TRON_CORE",
      workspaceName: "Tron Core",
    });

    expect(result.records[0]?.attributes).toEqual({
      endpoints: [{ host: "mysql.internal", port: 3306 }],
    });
    const [url, request] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(new URL(url).pathname).toBe("/api/v1/resource-connections/page");
    expect(request.body).toBe(JSON.stringify({
      current: 1,
      kinds: ["MysqlConnection"],
      search: null,
      size: 15,
      workspaceCode: "TRON_CORE",
    }));
  });
});
