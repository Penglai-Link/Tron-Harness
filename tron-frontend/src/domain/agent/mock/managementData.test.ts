import { beforeEach, expect, it, vi } from "vitest";

beforeEach(() => vi.resetModules());

it("lets platform settings edit built-in instructions and propagates them to workspaces", async () => {
  const { patchWorkspaceInternalAgentDefinitionMock, fetchWorkspaceInternalAgentMock } = await import("./managementData");
  const { PLATFORM_INTERNAL_AGENT_SCOPE } = await import("../model");
  const changed = patchWorkspaceInternalAgentDefinitionMock(PLATFORM_INTERNAL_AGENT_SCOPE, "main-agent", { rolePrompt: "Platform instructions" });
  expect(changed.rolePrompt).toBe("Platform instructions");
  expect(fetchWorkspaceInternalAgentMock("TRON_CORE", "main-agent").rolePrompt).toBe("Platform instructions");
});

it("keeps built-in definitions read-only within a workspace", async () => {
  const { patchWorkspaceInternalAgentDefinitionMock, fetchWorkspaceInternalAgentMock } = await import("./managementData");
  const before = fetchWorkspaceInternalAgentMock("TRON_CORE", "main-agent");
  expect(() => patchWorkspaceInternalAgentDefinitionMock("TRON_CORE", "main-agent", { rolePrompt: "Override" })).toThrow("read-only");
  expect(fetchWorkspaceInternalAgentMock("TRON_CORE", "main-agent").rolePrompt).toBe(before.rolePrompt);
});
