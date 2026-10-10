import { beforeEach, expect, it, vi } from "vitest";
import { loadAgentWorkspaceGrants, persistAgentWorkspaceGrants } from "./agentWorkspaceGrants";
const fixture = vi.hoisted(() => ({ source: "mock", fetch: vi.fn(), save: vi.fn() }));
vi.mock("@/shared/api/serviceConfig", () => ({ getServiceDataSource: () => fixture.source }));
vi.mock("../api/agentWorkspaceGrants", () => ({ fetchAgentWorkspaceGrants: fixture.fetch, saveAgentWorkspaceGrants: fixture.save }));
beforeEach(() => { vi.clearAllMocks(); });

it("keeps mock assignments in memory without backend writes, including revocation", async () => {
  fixture.source = "mock";
  await persistAgentWorkspaceGrants("mock-reviewer", ["A", "A", "B"]);
  const loaded = await loadAgentWorkspaceGrants("mock-reviewer");
  expect(loaded.workspaceCodes).toEqual(["A", "B"]);
  loaded.workspaceCodes.push("C");
  expect((await loadAgentWorkspaceGrants("mock-reviewer")).workspaceCodes).toEqual(["A", "B"]);
  await persistAgentWorkspaceGrants("mock-reviewer", []);
  expect((await loadAgentWorkspaceGrants("mock-reviewer")).workspaceCodes).toEqual([]);
  expect(fixture.fetch).not.toHaveBeenCalled();
  expect(fixture.save).not.toHaveBeenCalled();
});

it("uses the existing registry endpoints in API mode", async () => {
  fixture.source = "api";
  const signal = new AbortController().signal;
  await loadAgentWorkspaceGrants("reviewer", signal);
  await persistAgentWorkspaceGrants("reviewer", ["A"]);
  expect(fixture.fetch).toHaveBeenCalledWith("reviewer", signal);
  expect(fixture.save).toHaveBeenCalledWith("reviewer", ["A"]);
});
