// @vitest-environment jsdom
import type { ReactNode, SelectHTMLAttributes, InputHTMLAttributes } from "react";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { WorkspaceInternalAgentMcpPolicyPayload, WorkspaceInternalAgentPolicyPayload, WorkspaceInternalAgentRecord } from "@/domain/agent/model";

const fixture = vi.hoisted(() => ({
  record: {} as WorkspaceInternalAgentRecord,
  additionalRecords: [] as WorkspaceInternalAgentRecord[],
  storedTools: [] as string[],
  calls: [] as string[],
  assign: vi.fn(),
  patch: vi.fn(),
  error: vi.fn(),
  assignServers: vi.fn(),
  patchMcp: vi.fn(),
}));

vi.mock("@/domain/agent/api", () => ({
  fetchWorkspaceInternalAgents: vi.fn(async () => [fixture.record, ...fixture.additionalRecords].map((record) => ({ ...record, selectedToolCodes: [...record.selectedToolCodes] }))),
  fetchToolDomains: vi.fn(async () => []),
  listTools: vi.fn(async () => []),
  fetchWorkspaceSystemAgents: vi.fn(async () => []),
  patchWorkspaceInternalAgentPolicy: fixture.patch,
  updateWorkspaceInternalAgentToolAssignments: fixture.assign,
  patchWorkspaceInternalAgentMcpPolicy: fixture.patchMcp,
  updateWorkspaceInternalAgentMcpServerAssignments: fixture.assignServers,
  disableWorkspaceSystemAgent: vi.fn(),
  enableWorkspaceSystemAgent: vi.fn(),
}));
vi.mock("@/domain/mcp", () => ({ fetchWorkspaceMcpServers: vi.fn(async () => []), listWorkspaceMcpServersMock: vi.fn(() => []) }));
vi.mock("@/shared/api/serviceConfig", () => ({ getServiceDataSource: () => "api" }));
vi.mock("react-i18next", async () => {
  const actual = await vi.importActual<typeof import("react-i18next")>("react-i18next");
  return {
    ...actual,
    useTranslation: () => ({
      t: (_key: string, options?: { defaultValue?: string }) => options?.defaultValue ?? _key,
      i18n: { language: "en-US", resolvedLanguage: "en-US" },
    }),
  };
});
vi.mock("@/shared", () => ({
  useWorkspaceSpaces: () => ({ selectedSecondarySpace: { workspaceCode: "TRON_CORE", label: "Test workspace" } }),
  useTablePagination: () => ({}),
  PageHeader: ({ title }: { title: string }) => <h1>{title}</h1>,
  SurfaceCard: ({ children }: { children: ReactNode }) => <div>{children}</div>,
}));
vi.mock("@/components/ui/toast", () => ({ useToast: () => ({ error: fixture.error, success: vi.fn() }) }));
vi.mock("@/components/forms", () => ({
  Field: ({ label, children }: { label: ReactNode; children: ReactNode }) => <label>{label}{children}</label>,
  SelectInput: ({ onValueChange, ...props }: SelectHTMLAttributes<HTMLSelectElement> & { onValueChange: (value: string) => void }) => <select {...props} onChange={(event) => onValueChange(event.currentTarget.value)} />,
  TextInput: (props: InputHTMLAttributes<HTMLInputElement>) => <input {...props} />,
}));
vi.mock("@/components/ui/select", () => ({ SelectItem: ({ children, value }: { children: ReactNode; value: string }) => <option value={value}>{children}</option> }));
vi.mock("@/components/info-tooltip", () => ({ InfoTooltip: () => null }));
vi.mock("@/domain/agent/components", () => ({
  AgentPolicySwitch: () => null,
  AgentStatusBadge: () => null,
  InternalAgentMcpServerSelector: ({ selectedServerCodes, onSetServersChecked }: { selectedServerCodes: string[]; onSetServersChecked: (codes: string[], checked: boolean) => void }) => <div>
    <output aria-label="Visible selected servers">{selectedServerCodes.join(",")}</output>
    <button onClick={() => {
      onSetServersChecked(selectedServerCodes, false);
      onSetServersChecked(["new-server"], true);
    }}>Choose new MCP server</button>
  </div>,
  InternalAgentToolSelector: ({ selectedToolCodes, onSetToolsChecked }: { selectedToolCodes: string[]; onSetToolsChecked: (codes: string[], checked: boolean) => void }) => <div>
    <output aria-label="Visible selected tools">{selectedToolCodes.join(",")}</output>
    <button onClick={() => onSetToolsChecked(selectedToolCodes, false)}>Clear selected tools</button>
    <button onClick={() => {
      onSetToolsChecked(["bash.exec"], true);
      onSetToolsChecked(["fs.read_text"], true);
    }}>Select tools consecutively</button>
    <button onClick={() => onSetToolsChecked(["fs.read_text"], false)}>Remove read tool</button>
  </div>,
}));

import { WorkspaceAgentsPage } from "./WorkspaceAgentsPage";

beforeEach(() => {
  vi.clearAllMocks();
  fixture.calls = [];
  fixture.additionalRecords = [];
  fixture.storedTools = [];
  fixture.record = {
    agentCode: "main-agent", name: "Main Agent", description: "Fixture",
    mcpAccessMode: "disabled", toolAssignmentMode: "all_platform_tools", toolExposureMode: "direct_list",
    expandedDomainPersistenceMode: "sticky_expanded", maxSteps: 10, maxToolCalls: 10, maxParallelToolCalls: 1,
    selectedServerCodes: [], selectedToolCodes: ["bash.exec", "fs.read_text"], source: "platform_default",
  };
  fixture.assign.mockImplementation(async (_workspace: string, _agent: string, tools: string[]) => {
    fixture.calls.push("assign");
    fixture.storedTools = [...tools];
    if (fixture.record.toolAssignmentMode === "selected_tools") fixture.record.selectedToolCodes = [...tools];
    return { ...fixture.record };
  });
  fixture.assignServers.mockImplementation(async (_workspace: string, _agent: string, servers: string[]) => {
    fixture.calls.push("servers");
    fixture.record.selectedServerCodes = [...servers];
    return { ...fixture.record };
  });
  fixture.patchMcp.mockImplementation(async (_workspace: string, _agent: string, payload: WorkspaceInternalAgentMcpPolicyPayload) => {
    fixture.calls.push("mcp-policy");
    fixture.record = { ...fixture.record, ...payload };
    return { ...fixture.record };
  });
  fixture.patch.mockImplementation(async (_workspace: string, _agent: string, payload: WorkspaceInternalAgentPolicyPayload) => {
    fixture.calls.push("policy");
    fixture.record = { ...fixture.record, ...payload };
    if (fixture.record.toolAssignmentMode === "selected_tools") fixture.record.selectedToolCodes = [...fixture.storedTools];
    return { ...fixture.record };
  });
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

async function chooseSelectedTools() {
  render(<WorkspaceAgentsPage category="internal" />);
  const select = await screen.findByRole("combobox", { name: /tool assignment mode/i });
  await waitFor(() => expect((select as HTMLSelectElement).disabled).toBe(false));
  fireEvent.change(select, { target: { value: "selected_tools" } });
}

describe("internal agent tool selection autosave", () => {
  it("persists the visible all-tools selection when only switching to selected mode", async () => {
    await chooseSelectedTools();
    await waitFor(() => expect(fixture.patch).toHaveBeenCalled());
    expect(fixture.assign).toHaveBeenCalledWith("TRON_CORE", "main-agent", ["bash.exec", "fs.read_text"]);
    expect(fixture.calls).toEqual(["assign", "policy"]);
    expect(fixture.record.selectedToolCodes).toEqual(["bash.exec", "fs.read_text"]);
    await waitFor(() => expect(screen.getByLabelText("Visible selected tools").textContent).toBe("bash.exec,fs.read_text"));
  });

  it("persists an explicitly cleared selection as zero authorized tools", async () => {
    await chooseSelectedTools();
    fireEvent.click(screen.getByRole("button", { name: "Clear selected tools" }));
    await waitFor(() => expect(fixture.patch).toHaveBeenCalled());
    expect(fixture.assign).toHaveBeenCalledWith("TRON_CORE", "main-agent", []);
    expect(fixture.calls).toEqual(["assign", "policy"]);
    expect(fixture.record.selectedToolCodes).toEqual([]);
  });

  it("saves the latest consecutive edits only after the 450ms debounce", async () => {
    fixture.record.toolAssignmentMode = "selected_tools";
    fixture.record.selectedToolCodes = [];
    render(<WorkspaceAgentsPage category="internal" />);
    const select = await screen.findByRole("combobox", { name: /tool assignment mode/i });
    await waitFor(() => expect((select as HTMLSelectElement).disabled).toBe(false));
    vi.useFakeTimers();

    fireEvent.click(screen.getByRole("button", { name: "Select tools consecutively" }));
    expect(screen.getByLabelText("Visible selected tools").textContent).toBe("bash.exec,fs.read_text");
    await act(async () => { await vi.advanceTimersByTimeAsync(300); });
    fireEvent.click(screen.getByRole("button", { name: "Remove read tool" }));
    await act(async () => { await vi.advanceTimersByTimeAsync(449); });
    expect(fixture.assign).not.toHaveBeenCalled();
    await act(async () => { await vi.advanceTimersByTimeAsync(1); });

    expect(fixture.assign).toHaveBeenCalledExactlyOnceWith("TRON_CORE", "main-agent", ["bash.exec"]);
    expect(fixture.patch).not.toHaveBeenCalled();
    expect(fixture.record.selectedToolCodes).toEqual(["bash.exec"]);
    expect(screen.getByLabelText("Visible selected tools").textContent).toBe("bash.exec");
  });

  it("preserves another agent's pending draft when an earlier save refreshes", async () => {
    fixture.record.toolAssignmentMode = "selected_tools";
    fixture.record.selectedToolCodes = [];
    fixture.additionalRecords = [{ ...fixture.record, agentCode: "second-agent", name: "Second Agent" }];
    let completeFirstSave!: () => void;
    const firstSave = new Promise<void>((resolve) => { completeFirstSave = resolve; });
    fixture.assign.mockImplementation(async (_workspace: string, agentCode: string, tools: string[]) => {
      if (agentCode === "main-agent") {
        await firstSave;
        // Server canonicalization must remain authoritative for the saved row.
        fixture.record.selectedToolCodes = ["bash.exec"];
        return { ...fixture.record };
      }
      fixture.additionalRecords[0].selectedToolCodes = [...tools];
      return { ...fixture.additionalRecords[0] };
    });
    render(<WorkspaceAgentsPage category="internal" />);
    await waitFor(() => {
      const selects = screen.getAllByRole("combobox", { name: /tool assignment mode/i });
      expect(selects).toHaveLength(2);
      expect(selects.every((select) => !(select as HTMLSelectElement).disabled)).toBe(true);
    });
    vi.useFakeTimers();

    fireEvent.click(screen.getAllByRole("button", { name: "Select tools consecutively" })[0]);
    await act(async () => { await vi.advanceTimersByTimeAsync(450); });
    expect(fixture.assign).toHaveBeenCalledExactlyOnceWith("TRON_CORE", "main-agent", ["bash.exec", "fs.read_text"]);
    fireEvent.click(screen.getAllByRole("button", { name: "Select tools consecutively" })[1]);
    await act(async () => { completeFirstSave(); await firstSave; });

    const selections = screen.getAllByLabelText("Visible selected tools");
    expect(selections[0].textContent).toBe("bash.exec");
    expect(selections[1].textContent).toBe("bash.exec,fs.read_text");
    await act(async () => { await vi.advanceTimersByTimeAsync(449); });
    expect(fixture.assign).toHaveBeenCalledTimes(1);
    await act(async () => { await vi.advanceTimersByTimeAsync(1); });
    expect(fixture.assign).toHaveBeenNthCalledWith(2, "TRON_CORE", "second-agent", ["bash.exec", "fs.read_text"]);
    expect(fixture.additionalRecords[0].selectedToolCodes).toEqual(["bash.exec", "fs.read_text"]);
    expect(fixture.patch).not.toHaveBeenCalled();
  });

  it("keeps the previous policy mode when saving assignments fails", async () => {
    fixture.assign.mockRejectedValue(new Error("assignment storage unavailable"));
    await chooseSelectedTools();
    await waitFor(() => expect(fixture.error).toHaveBeenCalledWith("assignment storage unavailable"));
    expect(fixture.patch).not.toHaveBeenCalled();
    expect(fixture.record.toolAssignmentMode).toBe("all_platform_tools");
  });
});


describe("internal agent MCP selection autosave", () => {
  async function chooseNewServer() {
    fixture.record.selectedServerCodes = ["old-server"];
    render(<WorkspaceAgentsPage category="internal" />);
    const select = await screen.findByRole("combobox", { name: /mcp access mode/i });
    await waitFor(() => expect((select as HTMLSelectElement).disabled).toBe(false));
    fireEvent.change(select, { target: { value: "selected_servers" } });
    fireEvent.click(screen.getByRole("button", { name: "Choose new MCP server" }));
  }

  it("persists the new server assignments before enabling selected-server mode", async () => {
    await chooseNewServer();
    await waitFor(() => expect(fixture.patchMcp).toHaveBeenCalled());
    expect(fixture.assignServers).toHaveBeenCalledExactlyOnceWith("TRON_CORE", "main-agent", ["new-server"]);
    expect(fixture.patchMcp).toHaveBeenCalledExactlyOnceWith("TRON_CORE", "main-agent", { mcpAccessMode: "selected_servers" });
    expect(fixture.calls).toEqual(["servers", "mcp-policy"]);
    expect(fixture.record.mcpAccessMode).toBe("selected_servers");
    expect(fixture.record.selectedServerCodes).toEqual(["new-server"]);
    await waitFor(() => expect(screen.getByLabelText("Visible selected servers").textContent).toBe("new-server"));
  });

  it("keeps MCP disabled when saving the new server assignments fails", async () => {
    fixture.assignServers.mockRejectedValue(new Error("server assignment storage unavailable"));
    await chooseNewServer();
    await waitFor(() => expect(fixture.error).toHaveBeenCalledWith("server assignment storage unavailable"));
    expect(fixture.assignServers).toHaveBeenCalledExactlyOnceWith("TRON_CORE", "main-agent", ["new-server"]);
    expect(fixture.patchMcp).not.toHaveBeenCalled();
    expect(fixture.record.mcpAccessMode).toBe("disabled");
    expect(fixture.record.selectedServerCodes).toEqual(["old-server"]);
  });
});
