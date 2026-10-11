// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import type { PlatformToolRecord, ToolCatalogDomainRecord } from "../model";
import type { WorkspaceMcpServerRecord } from "@/domain/mcp/model";
vi.mock("@/domain/skill/api", () => ({
  fetchAgentSkills: vi.fn(),
  CAPABILITY_RESOURCES_CHANGED: "resource-change",
}));
vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: { defaultValue?: string }) =>
      options?.defaultValue || key,
  }),
}));
import { InternalAgentToolSelector } from "./InternalAgentToolSelector";
import { InternalAgentMcpServerSelector } from "./InternalAgentMcpServerSelector";
afterEach(cleanup);
const reason = "Capability pack fixture is disabled";
it("tool selection discards stale checks and all skips disabled pack tools", async () => {
  const all = vi.fn();
  render(
    <InternalAgentToolSelector
      selectedToolCodes={["blocked"]}
      onToggleTool={vi.fn()}
      onSetToolsChecked={all}
      toolDomains={[
        {
          domainCode: "fixture",
          name: "Fixture",
          sampleTools: [],
        } as unknown as ToolCatalogDomainRecord,
      ]}
      tools={
        [
          {
            toolCode: "active",
            domainCode: "fixture",
            name: "Active",
            toolName: "active",
            description: "Active tool",
            operationKind: "query",
            assignable: true,
          },
          {
            toolCode: "blocked",
            domainCode: "fixture",
            name: "Blocked",
            toolName: "blocked",
            description: "Blocked tool",
            operationKind: "query",
            assignable: false,
            disabledReason: reason,
          },
        ] as PlatformToolRecord[]
      }
    />
  );
  const checks = await screen.findAllByRole("checkbox");
  const blocked = checks.find(
    (c) => (c as HTMLInputElement).disabled
  ) as HTMLInputElement;
  expect(blocked.checked).toBe(false);
  expect(screen.getByRole("button", { name: reason }).tabIndex).toBe(0);
  fireEvent.click(screen.getByRole("button", { name: "Select all" }));
  expect(all).toHaveBeenCalledWith(["active"], true);
});
it("MCP selection discards stale checks and visible selection skips disabled servers", () => {
  const all = vi.fn();
  render(
    <InternalAgentMcpServerSelector
      selectedServerCodes={["blocked"]}
      onToggleServer={vi.fn()}
      onSetServersChecked={all}
      servers={
        [
          {
            serverCode: "active",
            name: "Active",
            effectiveEnabled: true,
            assignable: true,
          },
          {
            serverCode: "blocked",
            name: "Blocked",
            effectiveEnabled: false,
            assignable: false,
            disabledReason: reason,
          },
        ] as WorkspaceMcpServerRecord[]
      }
    />
  );
  const blocked = screen
    .getAllByRole("checkbox")
    .find((c) => (c as HTMLInputElement).disabled) as HTMLInputElement;
  expect(blocked.checked).toBe(false);
  expect(screen.getByRole("button", { name: reason }).tabIndex).toBe(0);
  fireEvent.click(screen.getByRole("button", { name: "Select visible" }));
  expect(all).toHaveBeenCalledWith(["active"], true);
});
