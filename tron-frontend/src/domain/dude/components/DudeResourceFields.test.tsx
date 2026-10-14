// @vitest-environment jsdom
import { createRef, type ReactNode } from "react";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { DudeResourceFields, type DudeResourceFieldsHandle } from "./DudeResourceFields";

const api = vi.hoisted(() => ({ record: vi.fn(), domains: vi.fn(), tools: vi.fn(), policy: vi.fn(), mcp: vi.fn(), assignTools: vi.fn(), assignServers: vi.fn(), skills: vi.fn(), knowledge: vi.fn(), notify: vi.fn() }));
vi.mock("@/domain/agent/api", () => ({ fetchToolDomains: api.domains, listTools: api.tools, patchWorkspaceInternalAgentPolicy: api.policy, patchWorkspaceInternalAgentMcpPolicy: api.mcp, updateWorkspaceInternalAgentToolAssignments: api.assignTools, updateWorkspaceInternalAgentMcpServerAssignments: api.assignServers }));
vi.mock("../api/dudeApi", () => ({ getDudeResources: api.record }));
vi.mock("@/domain/agent/model", () => ({ PLATFORM_INTERNAL_AGENT_SCOPE: "$platform", notifyInternalAgentRegistryChanged: api.notify }));
vi.mock("@/domain/skill/api", () => ({ setAgentSkills: api.skills }));
vi.mock("@/domain/knowledge-base/api/knowledgeBaseApi", () => ({ setAgentKnowledgeLibraries: api.knowledge }));
vi.mock("@/domain/mcp/api/mcpApi", () => ({ fetchPlatformMcpServers: vi.fn().mockResolvedValue([]) }));
vi.mock("react-i18next", () => ({ useTranslation: () => ({ t: (key: string) => key }) }));
vi.mock("@/components/info-tooltip", () => ({ InfoTooltip: () => null }));
vi.mock("@/components/forms", () => ({
  Field: ({ label, actions, children }: { label: ReactNode; actions?: ReactNode; children: ReactNode }) => <div>{label}{actions}{children}</div>,
  SelectInput: ({ children, onValueChange, ...props }: { children: ReactNode; onValueChange: (value: string) => void }) => <select {...props} onChange={(event) => onValueChange(event.target.value)}>{children}</select>,
  TextInput: (props: object) => <input {...props} />,
}));
vi.mock("@/components/ui/select", () => ({ SelectItem: ({ value, children }: { value: string; children: ReactNode }) => <option value={value}>{children}</option> }));
vi.mock("@/domain/agent/components/InternalAgentSkillField", () => ({ InternalAgentSkillField: ({ onSelectionChange }: { onSelectionChange: (names: string[]) => void }) => <button onClick={() => onSelectionChange(["skill-a"])}>Choose skill</button> }));
vi.mock("@/domain/agent/components/InternalAgentKnowledgeField", () => ({ InternalAgentKnowledgeField: ({ onSelectionChange }: { onSelectionChange: (names: string[]) => void }) => <button onClick={() => onSelectionChange(["library-a"])}>Choose knowledge</button> }));
vi.mock("@/domain/agent/components/InternalAgentAssignmentDialog", () => ({ InternalAgentAssignmentDialog: ({ onConfirm, onClose }: { onConfirm: (codes: string[]) => void; onClose: () => void }) => <div><button onClick={() => onConfirm(["tool-a"])}>Confirm tools</button><button onClick={onClose}>Cancel tools</button></div> }));
const record = {
  agentCode: "dude-a", name: "Dude A", description: "", rolePrompt: "", platformDefined: true,
  toolAssignmentMode: "all_platform_tools", toolExposureMode: "direct_list", expandedDomainPersistenceMode: "sticky_expanded",
  maxSteps: 100, maxToolCalls: 100, maxParallelToolCalls: 4, mcpAccessMode: "disabled", selectedToolCodes: [], selectedServerCodes: [], source: "platform_default",
};
function mount(panel: "resources" | "advanced" = "resources") {
  const ref = createRef<DudeResourceFieldsHandle>();
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(<QueryClientProvider client={client}><DudeResourceFields dudeCode="dude-a" panel={panel} ref={ref} /></QueryClientProvider>);
  return ref;
}
beforeEach(() => {
  vi.clearAllMocks(); api.record.mockResolvedValue(record); api.domains.mockResolvedValue([]); api.tools.mockResolvedValue([]);
  api.assignTools.mockResolvedValue(record); api.assignServers.mockResolvedValue(record);
  api.policy.mockImplementation(async (_scope, _code, policy) => ({ ...record, ...policy }));
  api.mcp.mockImplementation(async (_scope, _code, policy) => ({ ...record, ...policy }));
  api.skills.mockResolvedValue([]); api.knowledge.mockResolvedValue({});
});
afterEach(cleanup);

describe("Dude resource drafts", () => {
  it("saves the dynamic read-only rule without writing a fixed tool list", async () => {
    const ref = mount(); await screen.findByText("Choose skill");
    fireEvent.change(screen.getAllByRole("combobox")[1], { target: { value: "all_read_only_tools" } });
    expect(api.policy).not.toHaveBeenCalled();
    expect(screen.queryByText("Confirm tools")).toBeNull();
    await act(() => ref.current!.save());
    expect(api.policy).toHaveBeenCalledWith("$platform", "dude-a", { toolAssignmentMode: "all_read_only_tools" });
    expect(api.assignTools).not.toHaveBeenCalled();
  });
  it("uses platform definition and makes no writes until the parent explicitly saves", async () => {
    const ref = mount();
    await screen.findByText("Choose skill");
    expect(api.record).toHaveBeenCalledWith("dude-a", expect.any(AbortSignal));
    fireEvent.click(screen.getByText("Choose skill")); fireEvent.click(screen.getByText("Choose knowledge"));
    expect(api.skills).not.toHaveBeenCalled(); expect(api.knowledge).not.toHaveBeenCalled();
    await act(() => ref.current!.save());
    expect(api.skills).toHaveBeenCalledWith("dude-a", ["skill-a"]);
    expect(api.knowledge).toHaveBeenCalledWith("dude-a", ["library-a"]);
  });
  it("cancelling a tool selector preserves the original mode and assignments", async () => {
    const ref = mount(); await screen.findByText("Choose skill");
    fireEvent.change(screen.getAllByRole("combobox")[1], { target: { value: "selected_tools" } });
    fireEvent.click(screen.getByText("Cancel tools"));
    await act(() => ref.current!.save());
    expect(api.policy).not.toHaveBeenCalled(); expect(api.assignTools).not.toHaveBeenCalled();
  });
  it("saves chosen tools before activating selected mode", async () => {
    const ref = mount(); await screen.findByText("Choose skill");
    fireEvent.change(screen.getAllByRole("combobox")[1], { target: { value: "selected_tools" } });
    fireEvent.click(screen.getByText("Confirm tools"));
    expect(api.assignTools).not.toHaveBeenCalled();
    await act(() => ref.current!.save());
    expect(api.assignTools).toHaveBeenCalledWith("$platform", "dude-a", ["tool-a"]);
    expect(api.policy).toHaveBeenCalledWith("$platform", "dude-a", { toolAssignmentMode: "selected_tools" });
    expect(api.assignTools.mock.invocationCallOrder[0]).toBeLessThan(api.policy.mock.invocationCallOrder[0]);
  });
  it("retains unsaved knowledge selection after a partial failure and retries only the pending grant", async () => {
    api.knowledge.mockRejectedValueOnce(new Error("Grant unavailable"));
    const ref = mount(); await screen.findByText("Choose skill");
    fireEvent.click(screen.getByText("Choose skill")); fireEvent.click(screen.getByText("Choose knowledge"));
    await act(async () => { await expect(ref.current!.save()).rejects.toThrow("Grant unavailable"); });
    expect(screen.queryByRole("alert")).toBeNull(); // The enclosing editor owns the error toast.
    await act(() => ref.current!.save());
    expect(api.skills).toHaveBeenCalledTimes(1); expect(api.knowledge).toHaveBeenCalledTimes(2);
  });
  it("saves advanced policy separately from capability and access assignments", async () => {
    const ref = mount("advanced");
    const maxSteps = await screen.findByLabelText("workspaceAgents.internal.labels.maxSteps");
    fireEvent.change(maxSteps, { target: { value: "12" } });
    await act(() => ref.current!.save());
    expect(api.policy).toHaveBeenCalledWith("$platform", "dude-a", { maxSteps: 12 });
    expect(api.assignTools).not.toHaveBeenCalled();
    expect(api.assignServers).not.toHaveBeenCalled();
    expect(api.skills).not.toHaveBeenCalled();
    expect(api.knowledge).not.toHaveBeenCalled();
  });
});

it("does not let an untouched resource panel block an avatar save while still loading", async () => {
  api.record.mockReturnValueOnce(new Promise(() => {}));
  const ref = mount();
  await act(() => ref.current!.save());
  expect(api.policy).not.toHaveBeenCalled();
  expect(api.mcp).not.toHaveBeenCalled();
});
