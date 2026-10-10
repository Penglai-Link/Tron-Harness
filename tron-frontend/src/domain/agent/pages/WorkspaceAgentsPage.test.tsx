// @vitest-environment jsdom
import type { ReactNode, SelectHTMLAttributes } from "react";
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { WorkspaceInternalAgentMcpPolicyPayload, WorkspaceInternalAgentPolicyPayload, WorkspaceInternalAgentRecord } from "@/domain/agent/model";

const fixture = vi.hoisted(() => ({
  record: {} as WorkspaceInternalAgentRecord,
  additionalRecords: [] as WorkspaceInternalAgentRecord[],
  groups: [] as string[],
  createGroup: vi.fn(),
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
  knowledge: vi.fn(),
  workspaceGrants: vi.fn(),
}));

vi.mock("../hooks/useInternalAgentGroups", () => ({ useInternalAgentGroups: () => ({ groups: fixture.groups, isPending: false, error: null, create: fixture.createGroup }) }));

vi.mock("@tanstack/react-query", () => ({ useQueryClient: () => ({ invalidateQueries: vi.fn() }) }));
vi.mock("@/domain/knowledge-base/api/knowledgeBaseApi", () => ({ setAgentKnowledgeLibraries: fixture.knowledge }));
vi.mock("../hooks/agentWorkspaceGrants", () => ({ persistAgentWorkspaceGrants: fixture.workspaceGrants }));
vi.mock("../components/AgentWorkspaceField", () => ({ AgentWorkspaceField: ({ disabled, onSelectionChange }: { disabled: boolean; onSelectionChange: (codes: string[]) => void }) => <><button type="button" disabled={disabled} onClick={() => onSelectionChange(["TRON_CORE"])}>Choose workspace</button><button type="button" disabled={disabled} onClick={() => onSelectionChange([])}>Clear workspaces</button></> }));

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
vi.mock("@/domain/agent/hooks/useLoopDesignList", () => ({ useLoopDesignList: () => ({ designs: [{loopCode:"builtin.react-loop",name:"ReAct",kind:"React"},{loopCode:"review-flow",name:"Review",kind:"Workflow"}], loading:false,error:null }) }));
vi.mock("@/domain/mcp", () => ({ fetchWorkspaceMcpServers: vi.fn(async () => []), fetchPlatformMcpServers: vi.fn(async () => []), listPlatformMcpServersMock: vi.fn(() => []), listWorkspaceMcpServersMock: vi.fn(() => []) }));
vi.mock("@/shared/api/serviceConfig", () => ({ getServiceDataSource: () => "api" }));
vi.mock("react-i18next", async () => {
  const actual = await vi.importActual<typeof import("react-i18next")>("react-i18next");
  return {
    ...actual,
    useTranslation: () => ({
      t: (key: string, options?: { defaultValue?: string; [key: string]: unknown }) => {
        const common: Record<string, string> = { "common:save": "Save", "common:cancel": "Cancel", "common:close": "Close", "common:saving": "Saving..." };
        return (common[key] ?? options?.defaultValue ?? key).replace(/{{(\w+)}}/g, (_, name) => String(options?.[name] ?? name));
      },
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
vi.mock("@/components/forms", async () => ({
  // Exercise the actual Base UI inputs for focus/caret regressions.
  ...(await vi.importActual<typeof import("@/components/forms")>("@/components/forms")),
  Field: ({ label, children, actions }: { label: ReactNode; children: ReactNode; actions?: ReactNode }) => <div><label>{label}{children}</label>{actions}</div>,
  SelectInput: ({ onValueChange, ...props }: SelectHTMLAttributes<HTMLSelectElement> & { onValueChange: (value: string) => void }) => <select {...props} onChange={(event) => onValueChange(event.currentTarget.value)} />,
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

vi.mock("../components/InternalAgentKnowledgeField", () => ({ InternalAgentKnowledgeField: ({ agentCode, onSelectionChange }: {agentCode:string; onSelectionChange?: (ids: string[]) => void}) => <><output aria-label={`Knowledge libraries for ${agentCode}`}>platform</output><button onClick={() => onSelectionChange?.(["kb-shared"])}>Choose knowledge</button></> }));

import { WorkspaceAgentsPage } from "./WorkspaceAgentsPage";

beforeEach(() => {
  vi.clearAllMocks();
  fixture.calls = [];
  fixture.additionalRecords = [];
  fixture.groups = [];
  fixture.createGroup.mockImplementation(async (name: string) => { fixture.groups.push(name.trim()); return name.trim(); });
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


async function openEditor(scope: "workspace" | "platform" = "workspace") {
  render(<WorkspaceAgentsPage category="internal" scope={scope} />);
  fireEvent.click(await screen.findByRole("button", { name: "Edit Main Agent" }));
  return screen.findByRole("dialog", { name: "Edit Main Agent" });
}

function save() { fireEvent.click(screen.getByRole("button", { name: "Save" })); }

describe("internal agent cards and manual editing", () => {
  it("shows compact cards and opens only the chosen agent", async () => {
    fixture.additionalRecords = [{ ...fixture.record, agentCode: "reviewer", name: "Reviewer" }];
    render(<WorkspaceAgentsPage category="internal" />);
    const card = await screen.findByRole("button", { name: "Edit Reviewer" });
    expect(card.textContent).toContain("reviewer");
    expect(screen.queryByRole("textbox")).toBeNull();
    fireEvent.click(card);
    const dialog = await screen.findByRole("dialog", { name: "Edit Reviewer" });
    expect(within(dialog).getByRole<HTMLInputElement>("textbox", { name: "Name" }).value).toBe("Reviewer");
    expect(screen.getAllByRole("textbox", { name: "Role prompt" })).toHaveLength(1);
  });

  it.each(["workspace", "platform"] as const)("stages definition, avatar and loop changes until Save in %s", async (scope) => {
    const dialog = await openEditor(scope);
    fireEvent.change(within(dialog).getByRole("textbox", { name: "Name" }), { target: { value: "Reviewer" } });
    fireEvent.change(within(dialog).getByRole("textbox", { name: "Role prompt" }), { target: { value: "Check evidence" } });
    fireEvent.click(within(dialog).getByRole("button", { name: "workspaceAgents.internal.avatar.fox" }));
    fireEvent.change(within(dialog).getByRole("combobox", { name: "Agent Loop" }), { target: { value: "review-flow" } });
    await act(async () => { await new Promise((resolve) => setTimeout(resolve, 550)); });
    expect(fixture.patchDefinition).not.toHaveBeenCalled();
    save();
    await waitFor(() => expect(fixture.patchDefinition).toHaveBeenCalledExactlyOnceWith(scope === "platform" ? "$platform" : "TRON_CORE", "main-agent", { name: "Reviewer", rolePrompt: "Check evidence", avatarId: "fox", loopCode: "review-flow" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(await screen.findByRole("button", { name: "Edit Reviewer" })).toBeTruthy();
    expect(fixture.patch).not.toHaveBeenCalled();
  });

  it("cancels drafts without writes and reopens saved values", async () => {
    const dialog = await openEditor();
    fireEvent.change(within(dialog).getByRole("textbox", { name: "Name" }), { target: { value: "Discard me" } });
    fireEvent.click(within(dialog).getByRole("button", { name: "Cancel" }));
    fireEvent.click(screen.getByRole("button", { name: "Edit Main Agent" }));
    expect(screen.getByRole<HTMLInputElement>("textbox", { name: "Name" }).value).toBe("Main Agent");
    expect(fixture.patchDefinition).not.toHaveBeenCalled();
    expect((screen.getByRole("button", { name: "Save" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("keeps the modal and its draft when the backdrop is pressed", async () => {
    const dialog = await openEditor();
    fireEvent.change(within(dialog).getByRole("textbox", { name: "Name" }), { target: { value: "Still editing" } });
    const backdrop = document.querySelector('[data-slot="dialog-overlay"]')!;
    fireEvent.pointerDown(backdrop); fireEvent.pointerUp(backdrop); fireEvent.click(backdrop);
    expect(screen.getByRole("dialog")).toBe(dialog);
    expect(within(dialog).getByRole<HTMLInputElement>("textbox", { name: "Name" }).value).toBe("Still editing");
    expect(fixture.patchDefinition).not.toHaveBeenCalled();
  });

  it("persists confirmed tool selection before switching policy on manual Save", async () => {
    await openEditor();
    fireEvent.change(screen.getByRole("combobox", { name: /tool assignment mode/i }), { target: { value: "selected_tools" } });
    fireEvent.click(screen.getByRole("button", { name: "Confirm" }));
    expect(fixture.assign).not.toHaveBeenCalled();
    expect(fixture.patch).not.toHaveBeenCalled();
    save();
    await waitFor(() => expect(fixture.assign).toHaveBeenCalledWith("TRON_CORE", "main-agent", []));
    await waitFor(() => expect(fixture.calls).toEqual(["assign", "policy"]));
  });

  it("cancels a nested selection without changing the policy", async () => {
    await openEditor();
    fireEvent.change(screen.getByRole("combobox", { name: /tool assignment mode/i }), { target: { value: "selected_tools" } });
    const nested = screen.getAllByRole("dialog").at(-1)!;
    fireEvent.click(within(nested).getByRole("button", { name: "Cancel" }));
    expect(screen.getByRole<HTMLSelectElement>("combobox", { name: /tool assignment mode/i }).value).toBe("all_platform_tools");
    expect(fixture.patch).not.toHaveBeenCalled();
  });

  it("stages MCP selection until the editor is saved", async () => {
    await openEditor();
    fireEvent.change(screen.getByRole("combobox", { name: /MCP access mode/i }), { target: { value: "selected_servers" } });
    fireEvent.click(screen.getByRole("button", { name: "Confirm" }));
    expect(fixture.patchMcp).not.toHaveBeenCalled();
    save();
    await waitFor(() => expect(fixture.patchMcp).toHaveBeenCalledWith("TRON_CORE", "main-agent", { mcpAccessMode: "selected_servers" }));
  });

  it("defers knowledge authorization until Save even without other changes", async () => {
    await openEditor();
    fireEvent.click(screen.getByRole("button", { name: "Choose knowledge" }));
    expect(fixture.knowledge).not.toHaveBeenCalled();
    save();
    await waitFor(() => expect(fixture.knowledge).toHaveBeenCalledWith("main-agent", ["kb-shared"]));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(fixture.patchDefinition).not.toHaveBeenCalled();
  });

  it("keeps failed saves open with the draft and supports retry", async () => {
    fixture.patchDefinition.mockRejectedValueOnce(new Error("Storage unavailable"));
    await openEditor();
    fireEvent.change(screen.getByRole("textbox", { name: "Name" }), { target: { value: "Retry me" } });
    save();
    await waitFor(() => expect(fixture.error).toHaveBeenCalledWith("Storage unavailable"));
    expect(screen.getByRole<HTMLInputElement>("textbox", { name: "Name" }).value).toBe("Retry me");
    save();
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(fixture.patchDefinition).toHaveBeenCalledTimes(2);
  });

  it("stages restoring inheritance until Save without recreating overrides", async () => {
    fixture.record = { ...fixture.record, source: "workspace_override", rolePrompt: "Workspace role" };
    await openEditor();
    const api = await import("@/domain/agent/api");
    vi.mocked(api.fetchWorkspaceInternalAgents).mockResolvedValueOnce([{ ...fixture.record, source: "platform_default", rolePrompt: "Platform role" }]);
    fireEvent.click(screen.getByRole("button", { name: "Restore inheritance" }));
    await waitFor(() => expect(screen.getByRole<HTMLTextAreaElement>("textbox", { name: "Role prompt" }).value).toBe("Platform role"));
    expect(fixture.reset).not.toHaveBeenCalled();
    save();
    await waitFor(() => expect(fixture.reset).toHaveBeenCalledWith("TRON_CORE", "main-agent"));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(fixture.patchDefinition).not.toHaveBeenCalled();
  });

  it("blocks duplicate submission and closing while saving", async () => {
    let finish!: () => void;
    fixture.patchDefinition.mockImplementationOnce(async () => { await new Promise<void>((resolve) => { finish = resolve; }); fixture.record.name = "Slow save"; });
    await openEditor();
    fireEvent.change(screen.getByRole("textbox", { name: "Name" }), { target: { value: "Slow save" } });
    save();
    expect(screen.getByRole<HTMLInputElement>("textbox", { name: "Name" }).disabled).toBe(true);
    expect(screen.getByRole<HTMLButtonElement>("button", { name: "Cancel" }).disabled).toBe(true);
    expect(fixture.patchDefinition).toHaveBeenCalledTimes(1);
    await act(async () => finish());
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  });

  it("creates a platform agent with the chosen built-in avatar", async () => {
    render(<WorkspaceAgentsPage category="internal" scope="platform" />);
    await screen.findByRole("button", { name: "Edit Main Agent" });
    fireEvent.click(screen.getByRole("button", { name: "New" }));
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getAllByRole("button", { pressed: false })).toHaveLength(11);
    fireEvent.change(within(dialog).getByRole("textbox", { name: "Name" }), { target: { value: "Reviewer" } });
    fireEvent.change(within(dialog).getByRole("textbox", { name: "Code" }), { target: { value: "reviewer" } });
    fireEvent.click(within(dialog).getByRole("button", { name: "workspaceAgents.internal.avatar.owl" }));
    fireEvent.click(within(dialog).getByRole("button", { name: "Choose workspace" }));
    fireEvent.click(within(dialog).getByRole("button", { name: "New" }));
    await waitFor(() => expect(fixture.create).toHaveBeenCalledWith("$platform", { agentCode: "reviewer", name: "Reviewer", rolePrompt: "", loopCode: "builtin.react-loop", avatarId: "owl", groupName: "" }));
    expect(await screen.findByRole("button", { name: "Edit Reviewer" })).toBeTruthy();
    expect(screen.queryByRole("textbox")).toBeNull();
    expect(fixture.workspaceGrants).toHaveBeenCalledWith("reviewer", ["TRON_CORE"]);
  });

  it("saves workspace-only changes including clearing every grant", async () => {
    fixture.record.agentCode = "reviewer";
    await openEditor("platform");
    fireEvent.click(screen.getByRole("button", { name: "Clear workspaces" }));
    expect(fixture.workspaceGrants).not.toHaveBeenCalled();
    save();
    await waitFor(() => expect(fixture.workspaceGrants).toHaveBeenCalledWith("reviewer", []));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(fixture.patchDefinition).not.toHaveBeenCalled();
  });

  it("drops workspace drafts on Cancel and keeps workspace-scope editing isolated", async () => {
    fixture.record.agentCode = "reviewer";
    await openEditor("platform");
    fireEvent.click(screen.getByRole("button", { name: "Choose workspace" }));
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    fireEvent.click(screen.getByRole("button", { name: "Edit Main Agent" }));
    expect(screen.getByRole<HTMLButtonElement>("button", { name: "Save" }).disabled).toBe(true);
    expect(fixture.workspaceGrants).not.toHaveBeenCalled();
    cleanup();
    await openEditor("workspace");
    expect(screen.queryByRole("button", { name: "Choose workspace" })).toBeNull();
  });

  it("keeps a failed assignment draft for retry", async () => {
    fixture.record.agentCode = "reviewer";
    fixture.workspaceGrants.mockRejectedValueOnce(new Error("Assignment failed"));
    await openEditor("platform");
    fireEvent.click(screen.getByRole("button", { name: "Choose workspace" }));
    save();
    await waitFor(() => expect(fixture.error).toHaveBeenCalledWith("Assignment failed"));
    expect(screen.getByRole("dialog")).toBeTruthy();
    save();
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(fixture.workspaceGrants).toHaveBeenCalledTimes(2);
  });

  it("retries a created agent's failed assignment without creating twice", async () => {
    fixture.workspaceGrants.mockRejectedValueOnce(new Error("Assignment failed"));
    render(<WorkspaceAgentsPage category="internal" scope="platform" />);
    await screen.findByRole("button", { name: "Edit Main Agent" });
    fireEvent.click(screen.getByRole("button", { name: "New" }));
    const dialog = await screen.findByRole("dialog");
    fireEvent.change(within(dialog).getByRole("textbox", { name: "Name" }), { target: { value: "Reviewer" } });
    fireEvent.change(within(dialog).getByRole("textbox", { name: "Code" }), { target: { value: "reviewer" } });
    fireEvent.click(within(dialog).getByRole("button", { name: "New" }));
    await waitFor(() => expect(fixture.error).toHaveBeenCalledWith("Assignment failed"));
    fireEvent.click(within(dialog).getByRole("button", { name: "pages:platformData.save" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(fixture.create).toHaveBeenCalledTimes(1);
    expect(fixture.workspaceGrants).toHaveBeenCalledTimes(2);
  });
});

describe("internal agent groups and built-in read-only view", () => {
  it("shows collapsible groups with counts and filters by group or search", async () => {
    fixture.record.groupName = "Engineering";
    fixture.additionalRecords = [
      { ...fixture.record, agentCode: "finance", name: "Finance reviewer", groupName: "Finance" },
      { ...fixture.record, agentCode: "ungrouped", name: "Unsorted", groupName: null },
    ];
    render(<WorkspaceAgentsPage category="internal" scope="platform" />);
    await screen.findByRole("button", { name: "Edit Finance reviewer" });
    const engineering = screen.getByText("Engineering", { selector: "summary span" }).closest("details")!;
    expect(within(engineering).getByText("1", { selector: "summary span" })).toBeTruthy();
    expect(engineering.hasAttribute("open")).toBe(true);
    fireEvent.click(engineering.querySelector("summary")!);
    expect(engineering.hasAttribute("open")).toBe(false);
    fireEvent.change(screen.getByRole("combobox", { name: "Group" }), { target: { value: "group:Finance" } });
    expect(screen.queryByRole("button", { name: "Edit Main Agent" })).toBeNull();
    expect(screen.getByRole("button", { name: "Edit Finance reviewer" })).toBeTruthy();
    fireEvent.change(screen.getByRole("combobox", { name: "Group" }), { target: { value: "all" } });
    fireEvent.change(screen.getByRole("searchbox", { name: "Search agents or groups" }), { target: { value: "Engineering" } });
    expect(screen.getByRole("button", { name: "Edit Main Agent" })).toBeTruthy();
    await waitFor(() => expect(screen.queryByRole("button", { name: "Edit Finance reviewer" })).toBeNull());
    fireEvent.change(screen.getByRole("searchbox"), { target: { value: "" } });
    fireEvent.change(screen.getByRole("combobox", { name: "Group" }), { target: { value: "ungrouped" } });
    await waitFor(() => expect(screen.getByRole("button", { name: "Edit Unsorted" })).toBeTruthy());
  });
  it("creates a group within the creation dialog and sends the chosen group", async () => {
    render(<WorkspaceAgentsPage category="internal" scope="platform" />);
    await screen.findByRole("button", { name: "Edit Main Agent" });
    fireEvent.click(screen.getByRole("button", { name: "New" }));
    const dialog = await screen.findByRole("dialog");
    fireEvent.change(within(dialog).getByRole("textbox", { name: "Code" }), { target: { value: "grouped" } });
    fireEvent.change(within(dialog).getByRole("textbox", { name: "Name" }), { target: { value: "Grouped" } });
    fireEvent.click(within(dialog).getByRole("button", { name: "New group" }));
    fireEvent.change(within(dialog).getByRole("textbox", { name: "Group name" }), { target: { value: "Engineering" } });
    fireEvent.click(within(dialog).getByRole("button", { name: "Add" }));
    await waitFor(() => expect((within(dialog).getByRole("combobox", { name: "Group" }) as HTMLSelectElement).value).toBe("group:Engineering"));
    fireEvent.click(within(dialog).getByRole("button", { name: "New" }));
    await waitFor(() => expect(fixture.create).toHaveBeenCalledWith("$platform", expect.objectContaining({ groupName: "Engineering" })));
  });
  it("saves group changes and can clear to ungrouped", async () => {
    fixture.groups = ["Engineering", "Finance"];
    fixture.record.groupName = "Engineering";
    render(<WorkspaceAgentsPage category="internal" scope="platform" />);
    fireEvent.click(await screen.findByRole("button", { name: "Edit Main Agent" }));
    let dialog = await screen.findByRole("dialog");
    fireEvent.change(within(dialog).getByRole("combobox", { name: "Group" }), { target: { value: "group:Finance" } });
    fireEvent.click(within(dialog).getByRole("button", { name: "Save" }));
    await waitFor(() => expect(fixture.patchDefinition).toHaveBeenCalledWith("$platform", "main-agent", { groupName: "Finance" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    fireEvent.click(screen.getByRole("button", { name: "Edit Main Agent" }));
    dialog = await screen.findByRole("dialog");
    fireEvent.change(within(dialog).getByRole("combobox", { name: "Group" }), { target: { value: "ungrouped" } });
    fireEvent.click(within(dialog).getByRole("button", { name: "Save" }));
    await waitFor(() => expect(fixture.patchDefinition).toHaveBeenLastCalledWith("$platform", "main-agent", { groupName: "" }));
  });
  it("shows an accessible shield and a read-only built-in view", async () => {
    fixture.record.builtIn = true;
    render(<WorkspaceAgentsPage category="internal" scope="platform" />);
    const shield = await screen.findByRole("button", { name: "Built-in agent" });
    expect(shield.getAttribute("aria-label")).toBe("Built-in agent");
    fireEvent.click(screen.getByRole("button", { name: "View Main Agent · Built-in agent" }));
    const dialog = await screen.findByRole("dialog");
    expect((within(dialog).getByRole("textbox", { name: "Name" }) as HTMLInputElement).disabled).toBe(true);
    expect((within(dialog).getByRole("combobox", { name: "Group" }) as HTMLSelectElement).disabled).toBe(true);
    expect(within(dialog).queryByRole("button", { name: "Save" })).toBeNull();
    fireEvent.click(within(dialog).getAllByRole("button", { name: "Close" }).at(-1)!);
    expect(fixture.patchDefinition).not.toHaveBeenCalled();
    expect(fixture.patch).not.toHaveBeenCalled();
  });
  it("keeps built-in definitions read-only while saving independent platform workspace grants", async () => {
    fixture.record = { ...fixture.record, agentCode: "builtin-reviewer", builtIn: true };
    render(<WorkspaceAgentsPage category="internal" scope="platform" />);
    fireEvent.click(await screen.findByRole("button", { name: "View Main Agent · Built-in agent" }));
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByRole<HTMLInputElement>("textbox", { name: "Name" }).disabled).toBe(true);
    expect(within(dialog).getByRole<HTMLSelectElement>("combobox", { name: "Group" }).disabled).toBe(true);
    fireEvent.click(within(dialog).getByRole("button", { name: "Choose workspace" }));
    save();
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(fixture.workspaceGrants).toHaveBeenCalledWith("builtin-reviewer", ["TRON_CORE"]);
    expect(fixture.patchDefinition).not.toHaveBeenCalled();
    expect(fixture.patch).not.toHaveBeenCalled();
  });

  it("retries group plus authorization after a partial save without marking it complete", async () => {
    fixture.record.agentCode = "reviewer";
    fixture.groups = ["Engineering"];
    fixture.workspaceGrants.mockRejectedValueOnce(new Error("Assignment unavailable"));
    const dialog = await openEditor("platform");
    fireEvent.change(within(dialog).getByRole("combobox", { name: "Group" }), { target: { value: "group:Engineering" } });
    fireEvent.click(within(dialog).getByRole("button", { name: "Choose workspace" }));
    save();
    await waitFor(() => expect(fixture.error).toHaveBeenCalledWith("Assignment unavailable"));
    expect(screen.getByRole("dialog")).toBe(dialog);
    expect(within(dialog).getByRole<HTMLSelectElement>("combobox", { name: "Group" }).value).toBe("group:Engineering");
    expect(within(dialog).getByText(/Save failed.*Assignment unavailable/)).toBeTruthy();
    save();
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(fixture.patchDefinition).toHaveBeenCalledWith("$platform", "reviewer", { groupName: "Engineering" });
    expect(fixture.workspaceGrants).toHaveBeenCalledTimes(2);
  });

  it("discards a newly named group and workspace selection on Cancel without writes", async () => {
    fixture.record.agentCode = "reviewer";
    const dialog = await openEditor("platform");
    fireEvent.click(within(dialog).getByRole("button", { name: "New group" }));
    fireEvent.change(within(dialog).getByRole("textbox", { name: "Group name" }), { target: { value: "Draft group" } });
    fireEvent.click(within(dialog).getByRole("button", { name: "Add" }));
    fireEvent.click(within(dialog).getByRole("button", { name: "Choose workspace" }));
    fireEvent.click(within(dialog).getByRole("button", { name: "Cancel" }));
    expect(fixture.createGroup).not.toHaveBeenCalled();
    expect(fixture.patchDefinition).not.toHaveBeenCalled();
    expect(fixture.workspaceGrants).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Edit Main Agent" }));
    expect(screen.getByRole<HTMLSelectElement>("combobox", { name: "Group" }).value).toBe("ungrouped");
  });

  it("retains workspace draft when the final authoritative refresh fails", async () => {
    fixture.record.agentCode = "reviewer";
    const dialog = await openEditor("platform");
    const api = await import("@/domain/agent/api");
    vi.mocked(api.fetchWorkspaceInternalAgents).mockRejectedValueOnce(new Error("Refresh failed"));
    fireEvent.click(within(dialog).getByRole("button", { name: "Choose workspace" }));
    save();
    await waitFor(() => expect(fixture.error).toHaveBeenCalledWith("Refresh failed"));
    expect(screen.getByRole("dialog")).toBe(dialog);
    save();
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(fixture.workspaceGrants).toHaveBeenCalledTimes(2);
  });

});
