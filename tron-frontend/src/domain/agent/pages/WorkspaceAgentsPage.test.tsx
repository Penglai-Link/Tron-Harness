// @vitest-environment jsdom
import type { ReactNode, SelectHTMLAttributes, InputHTMLAttributes, TextareaHTMLAttributes } from "react";
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
  patchDefinition: vi.fn(),
  create: vi.fn(),
  reset: vi.fn(),
}));

vi.mock("@/domain/agent/api", () => ({
  fetchWorkspaceInternalAgents: vi.fn(async () => [fixture.record, ...fixture.additionalRecords].map((record) => ({ ...record, selectedToolCodes: [...record.selectedToolCodes] }))),
  fetchToolDomains: vi.fn(async () => []),
  listTools: vi.fn(async () => []),
  fetchWorkspaceSystemAgents: vi.fn(async () => []),
  patchWorkspaceInternalAgentPolicy: fixture.patch,
  patchWorkspaceInternalAgentDefinition: fixture.patchDefinition,
  createWorkspaceInternalAgent: fixture.create,
  resetWorkspaceInternalAgentPolicy: fixture.reset,
  updateWorkspaceInternalAgentToolAssignments: fixture.assign,
  patchWorkspaceInternalAgentMcpPolicy: fixture.patchMcp,
  updateWorkspaceInternalAgentMcpServerAssignments: fixture.assignServers,
  disableWorkspaceSystemAgent: vi.fn(),
  enableWorkspaceSystemAgent: vi.fn(),
}));
vi.mock("@/domain/mcp", () => ({ fetchWorkspaceMcpServers: vi.fn(async () => []), fetchPlatformMcpServers: vi.fn(async () => []), listPlatformMcpServersMock: vi.fn(() => []), listWorkspaceMcpServersMock: vi.fn(() => []) }));
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
  PageHeader: ({ title, actions }: { title: string; actions?: ReactNode }) => <header><h1>{title}</h1>{actions}</header>,
  SurfaceCard: ({ children }: { children: ReactNode }) => <div>{children}</div>,
}));
vi.mock("@/components/ui/toast", () => ({ useToast: () => ({ error: fixture.error, success: vi.fn() }) }));
vi.mock("@/components/forms", () => ({
  Field: ({ label, children, actions }: { label: ReactNode; children: ReactNode; actions?: ReactNode }) => <div><label>{label}{children}</label>{actions}</div>,
  SelectInput: ({ onValueChange, ...props }: SelectHTMLAttributes<HTMLSelectElement> & { onValueChange: (value: string) => void }) => <select {...props} onChange={(event) => onValueChange(event.currentTarget.value)} />,
  TextInput: (props: InputHTMLAttributes<HTMLInputElement>) => <input {...props} />,
  TextArea: (props: TextareaHTMLAttributes<HTMLTextAreaElement>) => <textarea {...props} />,
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

vi.mock("../components/InternalAgentKnowledgeField", () => ({ InternalAgentKnowledgeField: ({ agentCode }: {agentCode:string}) => <output aria-label={`Knowledge libraries for ${agentCode}`}>platform</output> }));

import { WorkspaceAgentsPage } from "./WorkspaceAgentsPage";

beforeEach(() => {
  vi.clearAllMocks();
  fixture.calls = [];
  fixture.additionalRecords = [];
  fixture.storedTools = [];
  fixture.record = {
    agentCode: "main-agent", name: "Main Agent", description: "Fixture", rolePrompt: "", platformDefined: true,
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
  fixture.patchDefinition.mockImplementation(async (_workspace: string, _agent: string, payload: { name: string; rolePrompt: string }) => {
    fixture.record = { ...fixture.record, ...payload };
    return { ...fixture.record };
  });
  fixture.reset.mockImplementation(async () => {
    fixture.record = { ...fixture.record, source: "platform_default", rolePrompt: "Platform role" };
    return { ...fixture.record };
  });
  fixture.create.mockImplementation(async (_workspace: string, payload: { agentCode: string; name: string; rolePrompt: string }) => {
    const record = { ...fixture.record, ...payload, agentCode: payload.agentCode };
    fixture.additionalRecords.push(record);
    return record;
  });
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

async function chooseSelectedTools(confirm = true) {
  render(<WorkspaceAgentsPage category="internal" />);
  const select = await screen.findByRole("combobox", { name: /tool assignment mode/i });
  await waitFor(() => expect((select as HTMLSelectElement).disabled).toBe(false));
  fireEvent.change(select, { target: { value: "selected_tools" } });
  if (confirm) fireEvent.click(screen.getByRole("button", { name: "Confirm" }));
}

describe("internal agent tool selection autosave", () => {
  it.each(["workspace", "platform"] as const)("offers role templates in %s editing without permanent help paragraphs", async (scope) => {
    render(<WorkspaceAgentsPage category="internal" scope={scope} />);
    await screen.findByRole("textbox", { name: "Role prompt" });
    expect(screen.getByRole("button", { name: "Role library" })).toBeTruthy();
    expect(screen.queryByText(/Sets this agent's role and instructions/)).toBeNull();
    expect(screen.queryByText(/Inherit platform agents or create workspace agents/)).toBeNull();
  });

  it("creates a platform definition before workspace authorization", async () => {
    render(<WorkspaceAgentsPage category="internal" scope="platform" />);
    await screen.findByRole("textbox", { name: "Role prompt" });
    fireEvent.click(screen.getByRole("button", { name: "New" }));
    const dialog = await screen.findByRole("dialog");
    const { within } = await import("@testing-library/react");
    fireEvent.change(within(dialog).getByRole("textbox", { name: "Name" }), { target: { value: "Reviewer" } });
    expect((within(dialog).getByRole("button", { name: "New" }) as HTMLButtonElement).disabled).toBe(true);
    fireEvent.change(within(dialog).getByRole("textbox", { name: "Code" }), { target: { value: "Invalid Code" } });
    expect(within(dialog).getByRole("textbox", { name: "Code" }).getAttribute("aria-invalid")).toBe("true");
    expect((within(dialog).getByRole("button", { name: "New" }) as HTMLButtonElement).disabled).toBe(true);
    fireEvent.change(within(dialog).getByRole("textbox", { name: "Code" }), { target: { value: "reviewer" } });
    fireEvent.change(within(dialog).getByRole("textbox", { name: "Role prompt" }), { target: { value: "Review changes and explain risks." } });
    fireEvent.click(within(dialog).getByRole("button", { name: "New" }));
    await waitFor(() => expect(fixture.create).toHaveBeenCalledWith("$platform", { agentCode: "reviewer", name: "Reviewer", rolePrompt: "Review changes and explain risks." }));
    await screen.findByRole("heading", { name: "Reviewer" });
    expect(screen.getAllByRole("textbox", { name: "Role prompt" })).toHaveLength(2);
    expect(screen.getByLabelText("Knowledge libraries for main-agent").textContent).toBe("platform");
    expect(screen.getByLabelText("Knowledge libraries for reviewer").textContent).toBe("platform");
  });

  it("offers no agent creation inside a workspace", async () => {
    render(<WorkspaceAgentsPage category="internal" />);
    await screen.findByRole("textbox", { name: "Role prompt" });
    expect(screen.queryByRole("button", { name: "New" })).toBeNull();
  });

  it("autosaves name and role together without changing tool policy", async () => {
    render(<WorkspaceAgentsPage category="internal" />);
    const name = await screen.findByRole("textbox", { name: "Name" });
    await waitFor(() => expect((name as HTMLInputElement).disabled).toBe(false));
    fireEvent.change(name, { target: { value: "Workspace assistant" } });
    fireEvent.change(screen.getByRole("textbox", { name: "Role prompt" }), { target: { value: "Answer using the workspace knowledge base." } });
    await waitFor(() => expect(fixture.patchDefinition).toHaveBeenCalledExactlyOnceWith("TRON_CORE", "main-agent", { name: "Workspace assistant", rolePrompt: "Answer using the workspace knowledge base." }));
    expect(fixture.patch).not.toHaveBeenCalled();
    expect(fixture.record.name).toBe("Workspace assistant");
    expect(fixture.record.rolePrompt).toBe("Answer using the workspace knowledge base.");
  });

  it("overrides only the role field when the inherited name is unchanged", async () => {
    render(<WorkspaceAgentsPage category="internal" />);
    const role = await screen.findByRole("textbox", { name: "Role prompt" });
    await waitFor(() => expect((role as HTMLTextAreaElement).disabled).toBe(false));
    fireEvent.change(role, { target: { value: "Workspace role" } });
    await waitFor(() => expect(fixture.patchDefinition).toHaveBeenCalledExactlyOnceWith("TRON_CORE", "main-agent", { rolePrompt: "Workspace role" }));
  });

  it("restores a platform definition and updates its visible role", async () => {
    fixture.record.source = "workspace_override";
    fixture.record.rolePrompt = "Workspace role";
    render(<WorkspaceAgentsPage category="internal" />);
    const restore = await screen.findByRole("button", { name: "Restore inheritance" });
    await waitFor(() => expect((restore as HTMLButtonElement).disabled).toBe(false));
    fireEvent.click(restore);
    await waitFor(() => expect(fixture.reset).toHaveBeenCalledExactlyOnceWith("TRON_CORE", "main-agent"));
    await waitFor(() => expect((screen.getByRole("textbox", { name: "Role prompt" }) as HTMLTextAreaElement).value).toBe("Platform role"));
    expect(screen.queryByRole("button", { name: "Restore inheritance" })).toBeNull();
  });

  it("saves platform fields in global scope and offers no workspace reset", async () => {
    render(<WorkspaceAgentsPage category="internal" scope="platform" />);
    const role = await screen.findByRole("textbox", { name: "Role prompt" });
    await waitFor(() => expect((role as HTMLTextAreaElement).disabled).toBe(false));
    fireEvent.change(role, { target: { value: "Global role" } });
    await waitFor(() => expect(fixture.patchDefinition).toHaveBeenCalledExactlyOnceWith("$platform", "main-agent", { rolePrompt: "Global role" }));
    expect(screen.queryByRole("button", { name: "Restore inheritance" })).toBeNull();
  });

  it("persists the visible all-tools selection when only switching to selected mode", async () => {
    await chooseSelectedTools();
    await waitFor(() => expect(fixture.patch).toHaveBeenCalled());
    expect(fixture.assign).toHaveBeenCalledWith("TRON_CORE", "main-agent", ["bash.exec", "fs.read_text"]);
    expect(fixture.calls).toEqual(["assign", "policy"]);
    expect(fixture.record.selectedToolCodes).toEqual(["bash.exec", "fs.read_text"]);
    expect(screen.queryByLabelText("Visible selected tools")).toBeNull();
  });

  it("persists an explicitly cleared selection as zero authorized tools", async () => {
    await chooseSelectedTools(false);
    fireEvent.click(screen.getByRole("button", { name: "Clear selected tools" }));
    fireEvent.click(screen.getByRole("button", { name: "Confirm" }));
    await waitFor(() => expect(fixture.patch).toHaveBeenCalled());
    expect(fixture.assign).toHaveBeenCalledWith("TRON_CORE", "main-agent", []);
    expect(fixture.calls).toEqual(["assign", "policy"]);
    expect(fixture.record.selectedToolCodes).toEqual([]);
  });

  it("keeps consecutive edits local until confirmation, then saves after the debounce", async () => {
    fixture.record.toolAssignmentMode = "selected_tools";
    fixture.record.selectedToolCodes = [];
    render(<WorkspaceAgentsPage category="internal" />);
    const select = await screen.findByRole("combobox", { name: /tool assignment mode/i });
    await waitFor(() => expect((select as HTMLSelectElement).disabled).toBe(false));
    fireEvent.click(screen.getByRole("button", { name: /Choose tools/ }));
    vi.useFakeTimers();
    fireEvent.click(screen.getByRole("button", { name: "Select tools consecutively" }));
    expect(screen.getByLabelText("Visible selected tools").textContent).toBe("bash.exec,fs.read_text");
    await act(async () => { await vi.advanceTimersByTimeAsync(600); });
    expect(fixture.assign).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Remove read tool" }));
    fireEvent.click(screen.getByRole("button", { name: "Confirm" }));
    await act(async () => { await vi.advanceTimersByTimeAsync(449); });
    expect(fixture.assign).not.toHaveBeenCalled();
    await act(async () => { await vi.advanceTimersByTimeAsync(1); });
    expect(fixture.assign).toHaveBeenCalledExactlyOnceWith("TRON_CORE", "main-agent", ["bash.exec"]);
    expect(fixture.patch).not.toHaveBeenCalled();
    expect(fixture.record.selectedToolCodes).toEqual(["bash.exec"]);
  });

  it.each(["workspace", "platform"] as const)("cancels a new specified-tool selection in %s scope without saving", async (scope) => {
    render(<WorkspaceAgentsPage category="internal" scope={scope} />);
    const select = await screen.findByRole("combobox", { name: /tool assignment mode/i });
    await waitFor(() => expect((select as HTMLSelectElement).disabled).toBe(false));
    fireEvent.change(select, { target: { value: "selected_tools" } });
    fireEvent.click(screen.getByRole("button", { name: "Clear selected tools" }));
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(screen.queryByRole("dialog")).toBeNull();
    expect((select as HTMLSelectElement).value).toBe("all_platform_tools");
    expect(fixture.assign).not.toHaveBeenCalled();
    expect(fixture.patch).not.toHaveBeenCalled();
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

    fireEvent.click(screen.getAllByRole("button", { name: /Choose tools/ })[0]);
    fireEvent.click(screen.getByRole("button", { name: "Select tools consecutively" }));
    fireEvent.click(screen.getByRole("button", { name: "Confirm" }));
    await act(async () => { await vi.advanceTimersByTimeAsync(450); });
    expect(fixture.assign).toHaveBeenCalledExactlyOnceWith("TRON_CORE", "main-agent", ["bash.exec", "fs.read_text"]);
    fireEvent.click(screen.getAllByRole("button", { name: /Choose tools/ })[1]);
    fireEvent.click(screen.getByRole("button", { name: "Select tools consecutively" }));
    fireEvent.click(screen.getByRole("button", { name: "Confirm" }));
    await act(async () => { completeFirstSave(); await firstSave; });

    expect(fixture.record.selectedToolCodes).toEqual(["bash.exec"]);
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
    fireEvent.click(screen.getByRole("button", { name: "Confirm" }));
  }

  it("dismisses MCP selection without changing the saved mode or assignments", async () => {
    render(<WorkspaceAgentsPage category="internal" />);
    const select = await screen.findByRole("combobox", { name: /mcp access mode/i });
    await waitFor(() => expect((select as HTMLSelectElement).disabled).toBe(false));
    fireEvent.change(select, { target: { value: "selected_servers" } });
    fireEvent.click(screen.getByRole("button", { name: "Choose new MCP server" }));
    fireEvent.click(screen.getByRole("button", { name: "Close" }));
    expect(screen.queryByRole("dialog")).toBeNull();
    expect((select as HTMLSelectElement).value).toBe("disabled");
    expect(fixture.assignServers).not.toHaveBeenCalled();
    expect(fixture.patchMcp).not.toHaveBeenCalled();
  });

  it("persists the new server assignments before enabling selected-server mode", async () => {
    await chooseNewServer();
    await waitFor(() => expect(fixture.patchMcp).toHaveBeenCalled());
    expect(fixture.assignServers).toHaveBeenCalledExactlyOnceWith("TRON_CORE", "main-agent", ["new-server"]);
    expect(fixture.patchMcp).toHaveBeenCalledExactlyOnceWith("TRON_CORE", "main-agent", { mcpAccessMode: "selected_servers" });
    expect(fixture.calls).toEqual(["servers", "mcp-policy"]);
    expect(fixture.record.mcpAccessMode).toBe("selected_servers");
    expect(fixture.record.selectedServerCodes).toEqual(["new-server"]);
    expect(screen.queryByLabelText("Visible selected servers")).toBeNull();
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
