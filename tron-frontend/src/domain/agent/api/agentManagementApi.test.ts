import { beforeEach, expect, it, vi } from "vitest";
import { requestApi } from "@/shared/api/httpClient";
import { fetchWorkspaceInternalAgents, listTools } from "./agentManagementApi";

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
