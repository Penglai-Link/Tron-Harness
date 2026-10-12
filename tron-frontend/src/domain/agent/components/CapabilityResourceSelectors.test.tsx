// @vitest-environment jsdom
import { useState } from "react";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  within,
} from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import type { PlatformToolRecord, ToolCatalogDomainRecord } from "../model";
import type { WorkspaceMcpServerRecord } from "@/domain/mcp/model";
vi.mock("@/domain/skill/api", () => ({
  fetchAgentSkills: vi.fn(),
  CAPABILITY_RESOURCES_CHANGED: "resource-change",
}));
vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: { defaultValue?: string; count?: number }) =>
      (options?.defaultValue || key).replace(
        "{{count}}",
        String(options?.count ?? "")
      ),
  }),
}));
import { InternalAgentToolSelector } from "./InternalAgentToolSelector";
import { InternalAgentMcpServerSelector } from "./InternalAgentMcpServerSelector";
afterEach(cleanup);
const reason = "Capability pack fixture is disabled";
it("tool selection discards stale checks and disables blocked pack tools", async () => {
  const toggle = vi.fn();
  render(
    <InternalAgentToolSelector
      selectedToolCodes={["blocked"]}
      onToggleTool={toggle}
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
  expect(screen.queryByRole("checkbox")).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: /^Fixture/ }));
  const checks = await screen.findAllByRole("checkbox");
  const blocked = checks.find(
    (c) => (c as HTMLInputElement).disabled
  ) as HTMLInputElement;
  expect(blocked.checked).toBe(false);
  expect(screen.getByRole("button", { name: reason }).tabIndex).toBe(0);
  fireEvent.click(screen.getByRole("checkbox", { name: /^Active/ }));
  expect(toggle).toHaveBeenCalledWith("active", true);
  expect(screen.queryByRole("button", { name: "Select all" })).toBeNull();
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

const domains = [
  {
    domainCode: "files",
    name: "Files",
    description: "File access",
    sampleTools: [],
    toolCount: 2,
    hasMutationTools: true,
  },
  {
    domainCode: "search",
    name: "Search",
    description: "Workspace search",
    sampleTools: [],
    toolCount: 1,
    hasMutationTools: false,
  },
] as ToolCatalogDomainRecord[];
const tools = [
  {
    toolCode: "files.read",
    toolName: "files_read",
    domainCode: "files",
    name: "Read file",
    description: "Read contents",
    operationKind: "query",
    assignable: true,
  },
  {
    toolCode: "files.write",
    toolName: "files_write",
    domainCode: "files",
    name: "Write file",
    description: "Write contents",
    operationKind: "command",
    assignable: true,
  },
  {
    toolCode: "search.code",
    toolName: "search_code",
    domainCode: "search",
    name: "Find code",
    description: "Find code matches",
    operationKind: "query",
    assignable: true,
  },
] as PlatformToolRecord[];

function ToolSelectionFixture() {
  const [selected, setSelected] = useState(["files.read"]);
  return (
    <InternalAgentToolSelector
      tools={tools}
      toolDomains={domains}
      selectedToolCodes={selected}
      onToggleTool={(code, checked) =>
        setSelected((current) =>
          checked ? [...current, code] : current.filter((item) => item !== code)
        )
      }
    />
  );
}

it("starts with every domain collapsed and preserves selected tools across independent expansions", () => {
  const view = render(<ToolSelectionFixture />);
  const files = screen.getByRole("button", { name: /^Files/ });
  const search = screen.getByRole("button", { name: /^Search/ });
  expect(files.getAttribute("aria-expanded")).toBe("false");
  expect(search.getAttribute("aria-expanded")).toBe("false");
  expect(files.textContent).toContain("1 selected");
  expect(screen.queryByRole("checkbox")).toBeNull();

  fireEvent.click(files);
  expect(
    (screen.getByRole("checkbox", { name: /^Read file/ }) as HTMLInputElement)
      .checked
  ).toBe(true);
  fireEvent.click(screen.getByRole("checkbox", { name: /^Write file/ }));
  fireEvent.click(search);
  expect(screen.getAllByRole("checkbox")).toHaveLength(3);
  fireEvent.click(files);
  expect(screen.getAllByRole("checkbox")).toHaveLength(1);
  expect(files.textContent).toContain("2 selected");
  fireEvent.click(files);
  expect(
    (screen.getByRole("checkbox", { name: /^Write file/ }) as HTMLInputElement)
      .checked
  ).toBe(true);
  const filesPanel = within(screen.getByRole("region", { name: /^Files/ }));
  expect(filesPanel.queryByText("File access")).toBeNull();
  expect(filesPanel.queryByRole("button", { name: "Clear" })).toBeNull();
  expect(filesPanel.getByText("files_read")).toBeTruthy();
  expect(filesPanel.queryByText("files.read")).toBeNull();
  fireEvent.click(filesPanel.getByRole("checkbox", { name: /^Read file/ }));
  fireEvent.click(filesPanel.getByRole("checkbox", { name: /^Write file/ }));
  expect(files.textContent).toContain("0 selected");

  view.unmount();
  render(<ToolSelectionFixture />);
  expect(screen.queryByRole("checkbox")).toBeNull();
});

it("searches callable tool names inside collapsed domains and selects by tool code", async () => {
  const toggle = vi.fn();
  render(
    <InternalAgentToolSelector
      tools={tools}
      toolDomains={domains}
      selectedToolCodes={[]}
      onToggleTool={toggle}
    />
  );
  fireEvent.change(screen.getByRole("searchbox"), {
    target: { value: "files_read" },
  });
  const files = await screen.findByRole("button", {
    name: "Files 1 tools 0 selected",
  });
  expect(screen.queryByRole("button", { name: /^Search/ })).toBeNull();
  expect(screen.queryByRole("checkbox")).toBeNull();
  fireEvent.click(files);
  expect(screen.getAllByRole("checkbox")).toHaveLength(1);
  fireEvent.click(screen.getByRole("checkbox", { name: /^Read file/ }));
  expect(toggle).toHaveBeenCalledWith("files.read", true);
  fireEvent.change(screen.getByRole("searchbox"), {
    target: { value: "No matching tool" },
  });
  expect(
    await screen.findByText("No tools match the search query.")
  ).toBeTruthy();
});

it("disables tool changes while saving without preventing domain inspection", () => {
  render(
    <InternalAgentToolSelector
      busy
      tools={tools}
      toolDomains={domains}
      selectedToolCodes={["files.read"]}
      onToggleTool={vi.fn()}
    />
  );
  fireEvent.click(screen.getByRole("button", { name: /^Files/ }));
  expect(
    screen
      .getAllByRole("checkbox")
      .every((input) => (input as HTMLInputElement).disabled)
  ).toBe(true);
  expect(screen.queryByRole("button", { name: "Select all" })).toBeNull();
  expect(screen.queryByRole("button", { name: "Clear" })).toBeNull();
});
