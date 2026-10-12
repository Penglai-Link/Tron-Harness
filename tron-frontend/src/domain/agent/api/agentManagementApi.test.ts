import { beforeEach, expect, it, vi } from "vitest";
import { requestApi } from "@/shared/api/httpClient";
import { fetchWorkspaceInternalAgents, listTools, fetchPlatformSystemAgents, fetchWorkspaceSystemAgents, createPlatformSystemAgent, updatePlatformSystemAgent } from "./agentManagementApi";

vi.mock("@/shared/api/httpClient", () => ({ requestApi: vi.fn() }));
beforeEach(() => vi.mocked(requestApi).mockReset());

it("preserves the new read-only mode returned by the backend", async () => {
  vi.mocked(requestApi).mockResolvedValue([{ agentCode: "explore-agent", toolAssignmentMode: "AllReadOnlyTools", selectedToolCodes: [], selectedServerCodes: [] }]);
  expect((await fetchWorkspaceInternalAgents("TRON_CORE"))[0].toolAssignmentMode).toBe("all_read_only_tools");
});

it("never normalizes an unknown or omitted operation kind into a query", async () => {
  vi.mocked(requestApi).mockResolvedValue(["QUERY", "COMMAND", "Unknown", undefined].map((operationKind, index) => ({toolCode: `fixture.${index}`, operationKind})));
  expect((await listTools()).map((tool) => tool.operationKind)).toEqual(["query", "command", "unknown", "unknown"]);
});

it("preserves logical system Agent identity and pinned model in platform and workspace lists", async () => {
  vi.mocked(requestApi).mockResolvedValue([{ agentCode: "codex-fast", baseAgentCode: "codex", modelId: "fast-model", models: [{ modelId: "fast-model", displayName: "Fast" }], status: "ENABLED", source: "SYSTEM" }]);
  for (const records of [await fetchPlatformSystemAgents(), await fetchWorkspaceSystemAgents("SPACE")]) {
    expect(records[0]).toMatchObject({ agentCode: "codex-fast", baseAgentCode: "codex", modelId: "fast-model" });
  }
});
it("sends creation and edit to the system Agent configuration endpoints", async () => {
  vi.mocked(requestApi).mockResolvedValue({ agentCode: "codex-fast" });
  const payload = { agentCode: "codex-fast", name: "Fast", baseAgentCode: "codex", modelId: "fast-model" };
  await createPlatformSystemAgent(payload);
  expect(requestApi).toHaveBeenLastCalledWith("POST /api/v1/agents/registry/system", { body: payload });
  await updatePlatformSystemAgent("codex-fast", { name: "Deep", modelId: "deep-model" });
  expect(requestApi).toHaveBeenLastCalledWith("PUT /api/v1/agents/registry/system/{agentCode}", { pathParams: { agentCode: "codex-fast" }, body: { name: "Deep", modelId: "deep-model" } });
});
