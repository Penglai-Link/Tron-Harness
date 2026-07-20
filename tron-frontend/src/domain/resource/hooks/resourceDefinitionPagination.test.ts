import { beforeEach, describe, expect, it, vi } from "vitest";

import { pageResourceDefinitions } from "@/domain/resource/api";
import type { ResourceDefinitionVersionApiRecord } from "@/domain/resource/api";

import { pageAllResourceDefinitions } from "./resourceDefinitionPagination";

vi.mock("@/domain/resource/api", () => ({
  pageResourceDefinitions: vi.fn(),
}));

function definitionRecord(index: number): ResourceDefinitionVersionApiRecord {
  return {
    allowInheritance: true,
    attributesSchema: null,
    binarySchema: null,
    effectiveBinaryPolicy: null,
    binaryStorageProfile: null,
    builtIn: false,
    createdAt: index,
    definitionCode: `DEFINITION_${index}`,
    definitionStatus: "ENABLED",
    definitionVersion: "v1-A1a",
    description: "",
    draftRevision: null,
    lifecycleStatus: "ACTIVE",
    name: `Definition ${index}`,
    publishedAt: index,
    scopeCode: "TRON_CORE",
    scopeKind: "WORKSPACE",
    sourceKind: "LOCAL",
    updatedAt: index,
    validationStatus: "VALID",
  };
}

describe("pageAllResourceDefinitions", () => {
  beforeEach(() => {
    vi.mocked(pageResourceDefinitions).mockReset();
  });

  it("aggregates all management pages when matching definitions exceed the API page cap", async () => {
    const records = Array.from({ length: 101 }, (_, index) => definitionRecord(index + 1));
    vi.mocked(pageResourceDefinitions).mockImplementation(async (request) => ({
      current: request.current,
      records: records.slice((request.current - 1) * 50, request.current * 50),
      size: 50,
      total: records.length,
    }));

    await expect(pageAllResourceDefinitions({
      definitionCode: null,
      definitionStatus: "ENABLED",
      lifecycleStatus: "ACTIVE",
      scopeKind: "WORKSPACE",
      search: null,
      sourceKind: null,
      validationStatus: null,
      workspaceCode: "TRON_CORE",
    })).resolves.toEqual(records);

    expect(pageResourceDefinitions).toHaveBeenCalledTimes(3);
    expect(vi.mocked(pageResourceDefinitions).mock.calls.map(([request]) => ({
      current: request.current,
      size: request.size,
    }))).toEqual([
      { current: 1, size: 50 },
      { current: 2, size: 50 },
      { current: 3, size: 50 },
    ]);
  });
});
