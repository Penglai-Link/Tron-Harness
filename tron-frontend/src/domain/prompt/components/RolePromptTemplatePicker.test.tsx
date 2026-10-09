// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { i18n } from "@/i18n";
import { createPlatformPromptMock, getPlatformPromptMock, listPlatformPromptsMock, resetPromptMockState, setPlatformPromptEnabledMock, updateWorkspacePromptMock } from "@/domain/prompt/mock";
import { agentRolePromptSeeds } from "@/domain/prompt/mock/agentRolePromptSeeds";
import type { PromptRecord } from "@/domain/prompt/model";
import { localizedPromptContent } from "@/domain/prompt/model/promptLocalization";
import { RolePromptTemplatePicker } from "./RolePromptTemplatePicker";
import { CreateInternalAgentDialog } from "@/domain/agent/components/CreateInternalAgentDialog";

const fixture = vi.hoisted(() => ({
  source: "mock", platformPage: vi.fn(), workspacePage: vi.fn(),
  platformDetail: vi.fn(), workspaceDetail: vi.fn(), create: vi.fn(),
}));
vi.mock("@/shared/api/serviceConfig", () => ({ getServiceDataSource: () => fixture.source }));
vi.mock("@/domain/prompt/api", () => ({
  fetchPlatformPromptPage: fixture.platformPage, fetchWorkspacePromptPage: fixture.workspacePage,
  fetchPlatformPromptDetail: fixture.platformDetail, fetchWorkspacePromptDetail: fixture.workspaceDetail,
}));
vi.mock("@/domain/agent/api", () => ({ createWorkspaceInternalAgent: fixture.create }));
vi.mock("@/components/ui/toast", () => ({ useToast: () => ({ error: vi.fn(), success: vi.fn() }) }));

beforeEach(async () => {
  vi.clearAllMocks();
  fixture.source = "mock";
  resetPromptMockState();
  await i18n.changeLanguage("en-US");
});
afterEach(() => { cleanup(); });

async function selectExplorer() {
  fireEvent.click(await screen.findByRole("button", { name: /^Explorer Agent/ }));
  await waitFor(() => expect((screen.getByRole("button", { name: "Apply" }) as HTMLButtonElement).disabled).toBe(false));
}

describe("role prompt templates", () => {
  it("lists only enabled agent roles, searches Chinese names, and copies the previewed localized body", async () => {
    await i18n.changeLanguage("zh-CN");
    setPlatformPromptEnabledMock({ promptCode: "ROLE_TESTER", enabled: false, expectedVersion: 1 });
    const apply = vi.fn();
    render(<RolePromptTemplatePicker scopeKind="WORKSPACE" workspaceCode="TRON_CORE" value="" onApply={apply} />);
    fireEvent.click(screen.getByRole("button", { name: "选择模板" }));
    await screen.findByRole("button", { name: /^主智能体/ });
    expect(screen.queryByRole("button", { name: /^测试人员/ })).toBeNull();
    expect(screen.queryByRole("button", { name: /^Workspace Router/ })).toBeNull();
    fireEvent.change(screen.getByRole("searchbox", { name: "搜索角色模板" }), { target: { value: "探索智能体" } });
    await waitFor(() => expect(screen.queryByRole("button", { name: /^主智能体/ })).toBeNull());
    fireEvent.click(await screen.findByRole("button", { name: /^探索智能体/ }));
    const body = agentRolePromptSeeds.find((seed) => seed.promptCode === "ROLE_EXPLORER")!.translations["zh-CN"].promptBody;
    await waitFor(() => expect(screen.getByLabelText("模板预览").textContent).toContain(body));
    fireEvent.click(screen.getByRole("button", { name: "应用" }));
    expect(apply).toHaveBeenCalledExactlyOnceWith(body);
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  });

  it("keeps existing text after cancel and replaces all of it only after confirmation", async () => {
    const apply = vi.fn();
    render(<RolePromptTemplatePicker scopeKind="PLATFORM" value="Existing instructions" onApply={apply} />);
    fireEvent.click(screen.getByRole("button", { name: "Choose template" }));
    await selectExplorer();
    fireEvent.click(screen.getByRole("button", { name: "Apply" }));
    const confirm = await screen.findByRole("dialog", { name: "Replace role prompt?" });
    expect(apply).not.toHaveBeenCalled();
    fireEvent.click(within(confirm).getByRole("button", { name: "Cancel" }));
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Replace role prompt?" })).toBeNull());
    expect(screen.getByRole("dialog", { name: "Choose a role template" })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Apply" }));
    const reopened = await screen.findByRole("dialog", { name: "Replace role prompt?" });
    fireEvent.click(within(reopened).getByRole("button", { name: "Replace and apply" }));
    expect(apply).toHaveBeenCalledExactlyOnceWith(getPlatformPromptMock({ promptCode: "ROLE_EXPLORER" }).promptBody);
  });

  it("searches beyond the first page and uses effective workspace content", async () => {
    for (let index = 0; index < 20; index++) createPlatformPromptMock({ displayName: `Extra ${index}`, category: "AGENT_ROLE", promptBody: "Extra role", description: "", enabled: true, variables: [] });
    updateWorkspacePromptMock({ workspaceCode: "TRON_CORE", promptCode: "ROLE_EXPLORER", expectedVersion: 1, displayName: "Workspace Explorer", promptBody: "Local exploration instructions", description: "Local context", enabled: true, variables: [], translations: [{ locale: "en-US", displayName: "Workspace Explorer", promptBody: "Local exploration instructions", description: "Local context", status: "PUBLISHED" }] });
    const apply = vi.fn();
    render(<RolePromptTemplatePicker scopeKind="WORKSPACE" workspaceCode="TRON_CORE" value=" " onApply={apply} />);
    fireEvent.click(screen.getByRole("button", { name: "Choose template" }));
    await screen.findByRole("button", { name: /^Extra 19/ });
    expect(screen.queryByRole("button", { name: /^Extra 0/ })).toBeNull();
    fireEvent.change(screen.getByRole("searchbox", { name: "Search role templates" }), { target: { value: "Extra 0" } });
    await screen.findByRole("button", { name: /^Extra 0/ });
    fireEvent.change(screen.getByRole("searchbox", { name: "Search role templates" }), { target: { value: "Workspace Explorer" } });
    fireEvent.click(await screen.findByRole("button", { name: /^Workspace Explorer/ }));
    await waitFor(() => expect((screen.getByRole("button", { name: "Apply" }) as HTMLButtonElement).disabled).toBe(false));
    fireEvent.click(screen.getByRole("button", { name: "Apply" }));
    expect(apply).toHaveBeenCalledExactlyOnceWith("Local exploration instructions");
    expect(getPlatformPromptMock({ promptCode: "ROLE_EXPLORER" }).promptBody).not.toBe("Local exploration instructions");
  });

  it("keeps the creation draft editable when closing a nested picker with Escape", async () => {
    render(<CreateInternalAgentDialog workspaceCode="TRON_CORE" onCreated={vi.fn(async () => {})} />);
    fireEvent.click(screen.getByRole("button", { name: "New" }));
    const creation = await screen.findByRole("dialog", { name: "New internal agent" });
    fireEvent.change(within(creation).getByRole("textbox", { name: "Name" }), { target: { value: "My Explorer" } });
    fireEvent.click(within(creation).getByRole("button", { name: "Choose template" }));
    const picker = await screen.findByRole("dialog", { name: "Choose a role template" });
    fireEvent.keyDown(picker, { key: "Escape" });
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Choose a role template" })).toBeNull());
    expect((screen.getByRole("textbox", { name: "Name" }) as HTMLInputElement).value).toBe("My Explorer");
    fireEvent.click(screen.getByRole("button", { name: "Choose template" }));
    await selectExplorer();
    fireEvent.click(screen.getByRole("button", { name: "Apply" }));
    const role = screen.getByRole("textbox", { name: "Instructions" });
    expect((role as HTMLTextAreaElement).value).toBe(getPlatformPromptMock({ promptCode: "ROLE_EXPLORER" }).promptBody);
    fireEvent.change(role, { target: { value: "My edited exploration role" } });
    expect((role as HTMLTextAreaElement).value).toBe("My edited exploration role");
    expect(screen.getByRole("button", { name: "Choose template" })).toBeTruthy();
  });

  it.each(["PLATFORM", "WORKSPACE"] as const)("routes %s requests correctly and does not apply an unavailable detail", async (scopeKind) => {
    fixture.source = "api";
    const record = getPlatformPromptMock({ promptCode: "ROLE_EXPLORER" });
    const page = { current: 1, size: 15, total: 1, records: [record] };
    fixture.platformPage.mockResolvedValue(page);
    fixture.workspacePage.mockResolvedValue(page);
    fixture.platformDetail.mockResolvedValue({ ...record, enabled: false });
    fixture.workspaceDetail.mockRejectedValue(new Error("Template was deleted"));
    const apply = vi.fn();
    render(<RolePromptTemplatePicker scopeKind={scopeKind} workspaceCode={scopeKind === "WORKSPACE" ? "TRON_CORE" : undefined} value="" onApply={apply} />);
    fireEvent.click(screen.getByRole("button", { name: "Choose template" }));
    fireEvent.click(await screen.findByRole("button", { name: /^Explorer Agent/ }));
    const pageMock = scopeKind === "PLATFORM" ? fixture.platformPage : fixture.workspacePage;
    const detailMock = scopeKind === "PLATFORM" ? fixture.platformDetail : fixture.workspaceDetail;
    await waitFor(() => expect(detailMock).toHaveBeenCalled());
    expect(pageMock).toHaveBeenCalledWith(expect.objectContaining({ category: "AGENT_ROLE", enabled: true }), expect.any(AbortSignal));
    if (scopeKind === "WORKSPACE") expect(detailMock).toHaveBeenCalledWith({ workspaceCode: "TRON_CORE", promptCode: "ROLE_EXPLORER" }, expect.any(AbortSignal));
    else expect(fixture.workspacePage).not.toHaveBeenCalled();
    await waitFor(() => expect(screen.getByText(scopeKind === "PLATFORM" ? "This template is no longer available." : "Template was deleted")).toBeTruthy());
    expect((screen.getByRole("button", { name: "Apply" }) as HTMLButtonElement).disabled).toBe(true);
    expect(apply).not.toHaveBeenCalled();
  });

  it.each(["TRON_CORE", "$platform"])("copies a template into the %s creation draft and submits later edits", async (workspaceCode) => {
    fixture.source = "api";
    const record = getPlatformPromptMock({ promptCode: "ROLE_EXPLORER" });
    const page = { current: 1, size: 15, total: 1, records: [record] };
    fixture.platformPage.mockResolvedValue(page);
    fixture.workspacePage.mockResolvedValue(page);
    fixture.platformDetail.mockResolvedValue(record);
    fixture.workspaceDetail.mockResolvedValue(record);
    fixture.create.mockResolvedValue({});
    render(<CreateInternalAgentDialog workspaceCode={workspaceCode} onCreated={vi.fn(async () => {})} />);
    fireEvent.click(screen.getByRole("button", { name: "New" }));
    fireEvent.change(await screen.findByRole("textbox", { name: "Name" }), { target: { value: "My role" } });
    fireEvent.change(screen.getByRole("textbox", { name: "Code" }), { target: { value: "my-role" } });
    fireEvent.click(screen.getByRole("button", { name: "Choose template" }));
    await selectExplorer();
    fireEvent.click(screen.getByRole("button", { name: "Apply" }));
    const role = screen.getByRole("textbox", { name: "Instructions" });
    expect((role as HTMLTextAreaElement).value).toBe(record.promptBody);
    fireEvent.change(role, { target: { value: "Edited template instructions" } });
    fireEvent.click(screen.getByRole("button", { name: "New" }));
    await waitFor(() => expect(fixture.create).toHaveBeenCalledWith(workspaceCode, { agentCode: "my-role", name: "My role", loopCode: "builtin.react-loop", rolePrompt: "Edited template instructions" }));
    if (workspaceCode === "$platform") {
      expect(fixture.platformPage).toHaveBeenCalled();
      expect(fixture.workspacePage).not.toHaveBeenCalled();
    } else {
      expect(fixture.workspacePage).toHaveBeenCalled();
      expect(fixture.platformPage).not.toHaveBeenCalled();
    }
  });

  it("ignores a late detail response after the picker is closed and reopened", async () => {
    fixture.source = "api";
    const record = getPlatformPromptMock({ promptCode: "ROLE_EXPLORER" });
    fixture.platformPage.mockResolvedValue({ current: 1, size: 15, total: 1, records: [record] });
    let resolve!: (record: PromptRecord) => void;
    fixture.platformDetail.mockImplementation(() => new Promise<PromptRecord>((done) => { resolve = done; }));
    render(<RolePromptTemplatePicker scopeKind="PLATFORM" value="" onApply={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: "Choose template" }));
    fireEvent.click(await screen.findByRole("button", { name: /^Explorer Agent/ }));
    await waitFor(() => expect(fixture.platformDetail).toHaveBeenCalled());
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    fireEvent.click(screen.getByRole("button", { name: "Choose template" }));
    await act(async () => { resolve(record); });
    expect((screen.getByRole("button", { name: "Apply" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("falls back to base content when the locale only has an unpublished translation", () => {
    const record = listPlatformPromptsMock({ current: 1, size: 15, category: "AGENT_ROLE" }).records[0];
    const translated = { ...record, translations: record.translations?.map((translation) => ({ ...translation, status: "DRAFT" as const })) };
    expect(localizedPromptContent(translated, "zh-CN").promptBody).toBe(record.promptBody);
  });
});
