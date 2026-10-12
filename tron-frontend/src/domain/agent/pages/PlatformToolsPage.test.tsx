// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { I18nextProvider } from "react-i18next";
import { createInstance } from "i18next";
import { pages as en } from "@/i18n/locales/en-US/pages";
import { pages as zh } from "@/i18n/locales/zh-CN/pages";
import { PlatformToolsPage } from "./PlatformToolsPage";
const fixture = vi.hoisted(() => ({ load: vi.fn(), save: vi.fn(), error: vi.fn(), success: vi.fn() }));
vi.mock("@/shared", async () => ({
  PageHeader: (await import("@/shared/layout/PageHeader")).PageHeader,
  useTablePagination: (await import("@/shared/hooks/useTablePagination")).useTablePagination,
}));
vi.mock("@/shared/api/serviceConfig", () => ({ getServiceDataSource: () => "api" }));
vi.mock("@/components/ui/toast", () => ({ useToast: () => ({ error: fixture.error, success: fixture.success }) }));
vi.mock("../api/toolPermissionsApi", () => ({ fetchToolPermissions: fixture.load, updateToolPermissions: fixture.save }));
vi.mock("@/domain/agent/api", () => ({
  fetchToolDomains: async () => [{ domainCode: "local", name: "Local", description: "Local tools", toolCount: 2, sampleTools: [], hasMutationTools: true }],
  listTools: async () => [
    { toolCode: "bash.exec", toolName: "bash_exec", domainCode: "local", name: "Shell", description: "Execute", operationKind: "command" },
    { toolCode: "fs.read", toolName: "fs_read", domainCode: "local", name: "Read", description: "Read file", operationKind: "query" },
  ],
}));
afterEach(cleanup);
beforeEach(() => {
  vi.clearAllMocks();
  fixture.load.mockResolvedValue([{ toolCode: "bash.exec", permission: "ask", confirmLevel: "high" }]);
  fixture.save.mockImplementation(async (payload) => payload.permissions);
});
async function mount(locale = "en-US") {
  const i18n = createInstance();
  await i18n.init({ lng: locale, resources: { "en-US": { pages: en }, "zh-CN": { pages: zh } }, interpolation: { escapeValue: false } });
  const view = render(<I18nextProvider i18n={i18n}><PlatformToolsPage /></I18nextProvider>);
  const expand = await screen.findByRole("button", { name: "local: Local" });
  fireEvent.click(expand);
  await screen.findByRole("group", { name: /Shell/ });
  return view;
}
describe("platform tool permissions", () => {
  it("loads saved Ask threshold, defaults unset tools to Run, and saves a full snapshot", async () => {
    await mount();
    const shell = screen.getByRole("group", { name: "Execution permission for Shell" });
    expect(within(shell).getByRole("button", { name: "Ask" }).getAttribute("aria-pressed")).toBe("true");
    expect(screen.getByRole("combobox", { name: "Confirmation threshold for Shell" }).textContent).toContain("High");
    const read = screen.getByRole("group", { name: "Execution permission for Read" });
    expect(within(read).getByRole("button", { name: "Run" }).getAttribute("aria-pressed")).toBe("true");
    fireEvent.click(within(shell).getByRole("button", { name: "No access" }));
    expect(screen.queryByRole("combobox", { name: "Confirmation threshold for Shell" })).toBeNull();
    expect((screen.getByRole("button", { name: "Refresh" }) as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(fixture.save).toHaveBeenCalledWith({ permissions: [
      { toolCode: "bash.exec", permission: "no", confirmLevel: "high" },
      { toolCode: "fs.read", permission: "run", confirmLevel: "low" },
    ] }));
    await waitFor(() => expect(fixture.success).toHaveBeenCalled());
    expect((screen.getByRole("button", { name: "Save" }) as HTMLButtonElement).disabled).toBe(true);
  });
  it("keeps the draft on failure and allows discarding it", async () => {
    fixture.save.mockRejectedValueOnce(new Error("storage unavailable"));
    await mount();
    fireEvent.click(within(screen.getByRole("group", { name: /Shell/ })).getByRole("button", { name: "Run" }));
    const unload = new Event("beforeunload", { cancelable: true });
    window.dispatchEvent(unload);
    expect(unload.defaultPrevented).toBe(true);
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(fixture.error).toHaveBeenCalledWith("storage unavailable"));
    expect(screen.getByText("Unsaved changes")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Discard" }));
    expect(screen.getByRole("combobox", { name: "Confirmation threshold for Shell" }).textContent).toContain("High");
    expect(screen.queryByText("Unsaved changes")).toBeNull();
  });
  it("reloads persisted values after remount", async () => {
    const view = await mount();
    fireEvent.click(within(screen.getByRole("group", { name: /Shell/ })).getByRole("button", { name: "No access" }));
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(fixture.success).toHaveBeenCalled());
    fixture.load.mockResolvedValue(fixture.save.mock.calls[0][0].permissions);
    view.unmount();
    await mount();
    expect(within(screen.getByRole("group", { name: /Shell/ })).getByRole("button", { name: "No access" }).getAttribute("aria-pressed")).toBe("true");
  });
  it("renders Chinese options and supports keyboard-focusable domain expansion", async () => {
    await mount("zh-CN");
    expect(screen.getAllByRole("button", { name: "直接执行" })).toHaveLength(2);
    expect(screen.getAllByRole("button", { name: "询问" })).toHaveLength(2);
    expect(screen.getAllByRole("button", { name: "无权限" })).toHaveLength(2);
    expect(screen.getByRole("button", { name: "local: Local" }).getAttribute("aria-expanded")).toBe("true");
  });
  it("disables editing after an initial load failure and allows retry", async () => {
    fixture.load.mockRejectedValueOnce(new Error("load failed"));
    const i18n = createInstance();
    await i18n.init({ lng: "en-US", resources: { "en-US": { pages: en } } });
    render(<I18nextProvider i18n={i18n}><PlatformToolsPage /></I18nextProvider>);
    await waitFor(() => expect(fixture.error).toHaveBeenCalledWith("load failed"));
    expect((screen.getByRole("button", { name: "Save" }) as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(screen.getByRole("button", { name: "Refresh" }));
    await screen.findByRole("button", { name: "local: Local" });
  });
});
