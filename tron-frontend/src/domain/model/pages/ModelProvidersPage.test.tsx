// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { MemoryRouter, useLocation } from "react-router";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { ModelProviderConnection } from "@/domain/model/api";
import type { ConfiguredProviderUpdate } from "@/domain/model/api/configuredProviderApi";

const fixture = vi.hoisted(() => ({
  rows: [] as ModelProviderConnection[],
  list: vi.fn(), fetch: vi.fn(), update: vi.fn(), remove: vi.fn(),
  authorize: vi.fn(), sync: vi.fn(), disconnect: vi.fn(),
  toast: { success: vi.fn(), error: vi.fn() },
  t: (key: string, options?: { name?: string }) => `${key}${options?.name ? `:${options.name}` : ""}`,
  format: (value: unknown) => String(value),
}));
vi.mock("react-i18next", () => ({ useTranslation: () => ({ t: fixture.t }) }));
vi.mock("@/domain/model/api", () => ({
  fetchModelProviderConnections: fixture.list,
  createModelProviderAuthorization: fixture.authorize,
  syncModelProvider: fixture.sync, disconnectModelProvider: fixture.disconnect,
}));
vi.mock("@/domain/model/api/configuredProviderApi", () => ({
  fetchProviderPresets: vi.fn().mockResolvedValue([]),
  fetchConfiguredProvider: fixture.fetch, updateConfiguredProvider: fixture.update, deleteConfiguredProvider: fixture.remove,
}));
vi.mock("@/components/ui/toast", () => ({ useToast: () => fixture.toast }));
vi.mock("@/components/editable-tabs", () => ({ useTabVisibility: () => true }));
vi.mock("@/components/info-tooltip", () => ({ InfoTooltip: () => null }));
vi.mock("@/shared", () => ({ PageHeader: ({ actions }: { actions: React.ReactNode }) => <header>{actions}</header>, useLocaleFormatters: () => ({ formatDateTime: fixture.format }) }));
vi.mock("@/components/data-table", () => ({
  DataTable: ({ columns, rows, onRowClick }: { columns: { id: string; render: (row: ModelProviderConnection) => React.ReactNode }[]; rows: ModelProviderConnection[]; onRowClick: (row: ModelProviderConnection) => void }) =>
    <table><tbody>{rows.map(row => <tr key={row.providerCode} onClick={() => onRowClick(row)}>{columns.map(column => <td key={column.id}>{column.render(row)}</td>)}</tr>)}</tbody></table>,
}));

import { ModelProvidersPage } from "./ModelProvidersPage";

function Location() { const value = useLocation(); return <output data-testid="location">{value.pathname}{value.search}</output>; }
function mount() { render(<MemoryRouter initialEntries={["/settings/model-providers"]}><Location /><ModelProvidersPage /></MemoryRouter>); }
async function menu(name: string) {
  const trigger = await screen.findByRole("button", { name: `configuredProviders.actionsFor:${name}` });
  trigger.focus();
  fireEvent.keyDown(trigger, { key: "ArrowDown" });
  return screen.findByRole("menu");
}
function connection(providerCode: string, displayName: string, source: "Custom" | "PersonAi"): ModelProviderConnection {
  return { providerCode, displayName, source, status: "Connected", serverBaseUrl: "https://provider.test/v1", catalogModelCount: 1,
    personAiUserId: null, personAiUsername: null, authorizedAt: null, authorizationExpiresAt: null, lastSyncAt: null, lastError: null, updatedAt: 1 };
}
const models = [{ modelType: "Text", modelCode: "model-stable", modelId: "remote-model", name: "Model", contextWindowTokens: 8192,
  maxOutputTokens: 1024, reasoning: false, supportsDeveloperRole: false, inputCapabilities: ["Text"], enabled: true }];

beforeEach(() => {
  vi.clearAllMocks();
  fixture.rows = [connection("custom-one", "Work account", "Custom"), connection("PersonAi", "Person-AI", "PersonAi")];
  fixture.list.mockImplementation(async () => [...fixture.rows]);
  fixture.fetch.mockImplementation(async providerId => ({ providerId, displayName: "Work account", protocol: "OpenAiChatCompletions", baseUrl: "https://provider.test/v1",
    requestHeaders: { "x-client": "Tron" }, models, enabled: true, hasApiKey: true, createdAt: 1, updatedAt: 1 }));
  fixture.update.mockReset().mockImplementation(async (code: string, body: ConfiguredProviderUpdate) => {
    fixture.rows = fixture.rows.map(row => row.providerCode === code ? { ...row, status: body.enabled ? "Connected" : "Disconnected" } : row);
    return body;
  });
  fixture.remove.mockReset().mockImplementation(async code => { fixture.rows = fixture.rows.filter(row => row.providerCode !== code); return {}; });
  fixture.disconnect.mockReset().mockImplementation(async code => { fixture.rows = fixture.rows.map(row => row.providerCode === code ? { ...row, status: "Disconnected" } : row); });
  fixture.sync.mockReset().mockResolvedValue({ modelCount: 1 });
  fixture.authorize.mockReset().mockResolvedValue({ sessionId: "session-one", loginUrl: "https://provider.test/authorize" });
});
afterEach(() => { cleanup(); vi.restoreAllMocks(); });

it("refreshes rejected authorization on return and offers reauthorization instead of sync", async () => {
  mount();
  await screen.findByText("modelProviders.status.Connected");
  fixture.rows = fixture.rows.map(row => row.providerCode === "PersonAi" ? {
    ...row, status: "AuthRequired", lastError: "PersonAI encryption key changed; authorize PersonAI again",
  } : row);
  fireEvent(window, new Event("focus"));
  await screen.findByText("modelProviders.status.AuthRequired");
  expect(screen.queryByText("modelProviders.status.Connected")).toBeNull();
  const actions = await menu("Person-AI");
  expect(within(actions).getByRole("menuitem", { name: "modelProviders.actions.reauthorize" })).toBeTruthy();
  const sync = within(actions).getByRole("menuitem", { name: "modelProviders.actions.syncNow" });
  expect(sync.getAttribute("aria-disabled")).toBe("true");
});

it("uses a three-dot action menu without a Manage item or unintended row navigation", async () => {
  mount();
  const actions = await menu("Work account");
  expect(within(actions).getByRole("menuitem", { name: "configuredProviders.actions.disable" })).toBeTruthy();
  expect(within(actions).getByRole("menuitem", { name: "configuredProviders.actions.delete" })).toBeTruthy();
  expect(screen.queryByText("modelProviders.actions.manage")).toBeNull();
  expect(screen.getByTestId("location").textContent).toBe("/settings/model-providers");
});

it("disables and enables a custom connection while retaining its saved key and models", async () => {
  mount();
  fireEvent.click(within(await menu("Work account")).getByRole("menuitem", { name: "configuredProviders.actions.disable" }));
  await waitFor(() => expect(fixture.update).toHaveBeenCalledTimes(1));
  expect(fixture.update).toHaveBeenLastCalledWith("custom-one", { displayName: "Work account", protocol: "OpenAiChatCompletions", baseUrl: "https://provider.test/v1", requestHeaders: { "x-client": "Tron" }, models, enabled: false, apiKey: null });
  await screen.findByText("configuredProviders.disabled");
  fireEvent.click(within(await menu("Work account")).getByRole("menuitem", { name: "configuredProviders.actions.enable" }));
  await waitFor(() => expect(fixture.update).toHaveBeenCalledTimes(2));
  expect(fixture.update.mock.calls[1][1]).toMatchObject({ enabled: true, apiKey: null, models });
  expect(screen.getByTestId("location").textContent).toBe("/settings/model-providers");
});

it("deletes a custom connection only after confirmation and refreshes the list", async () => {
  mount();
  fireEvent.click(within(await menu("Work account")).getByRole("menuitem", { name: "configuredProviders.actions.delete" }));
  expect(fixture.remove).not.toHaveBeenCalled();
  fireEvent.click(within(screen.getByRole("dialog")).getByRole("button", { name: "configuredProviders.actions.cancel" }));
  expect(fixture.remove).not.toHaveBeenCalled();
  fireEvent.click(within(await menu("Work account")).getByRole("menuitem", { name: "configuredProviders.actions.delete" }));
  fireEvent.click(within(screen.getByRole("dialog")).getByRole("button", { name: "configuredProviders.actions.delete" }));
  await waitFor(() => expect(fixture.remove).toHaveBeenCalledExactlyOnceWith("custom-one"));
  await waitFor(() => expect(screen.queryByText("Work account")).toBeNull());
  expect(fixture.disconnect).not.toHaveBeenCalled();
});

it("keeps the original state and reports a failed toggle", async () => {
  fixture.update.mockRejectedValueOnce(new Error("Update unavailable"));
  mount();
  fireEvent.click(within(await menu("Work account")).getByRole("menuitem", { name: "configuredProviders.actions.disable" }));
  await waitFor(() => expect(fixture.toast.error).toHaveBeenCalledWith("Update unavailable"));
  expect(within(await menu("Work account")).getByRole("menuitem", { name: "configuredProviders.actions.disable" })).toBeTruthy();
  expect(fixture.toast.success).not.toHaveBeenCalled();
});

it("offers plugin sync and disconnect instead of pretending they are custom enable/delete operations", async () => {
  mount();
  const actions = await menu("Person-AI");
  expect(within(actions).queryByRole("menuitem", { name: "configuredProviders.actions.delete" })).toBeNull();
  expect(within(actions).queryByRole("menuitem", { name: "configuredProviders.actions.disable" })).toBeNull();
  fireEvent.click(within(actions).getByRole("menuitem", { name: "modelProviders.actions.disconnect" }));
  expect(fixture.disconnect).not.toHaveBeenCalled();
  fireEvent.click(within(screen.getByRole("dialog")).getByRole("button", { name: "modelProviders.actions.disconnect" }));
  await waitFor(() => expect(fixture.disconnect).toHaveBeenCalledExactlyOnceWith("PersonAi"));
  expect(fixture.remove).not.toHaveBeenCalled();
  expect(fixture.update).not.toHaveBeenCalled();
  await screen.findAllByText("Person-AI");
});

it("submits a plugin catalog sync from the menu", async () => {
  mount();
  fireEvent.click(within(await menu("Person-AI")).getByRole("menuitem", { name: "modelProviders.actions.syncNow" }));
  await waitFor(() => expect(fixture.sync).toHaveBeenCalledExactlyOnceWith("PersonAi"));
  await waitFor(() => expect(fixture.toast.success).toHaveBeenCalledWith("configuredProviders.messages.syncStarted"));
  expect(screen.getByTestId("location").textContent).toBe("/settings/model-providers");
});

it("starts native authorization and opens the corresponding authorization session", async () => {
  fixture.rows[1]!.status = "Disconnected";
  const open = vi.spyOn(window, "open").mockReturnValue(null);
  mount();
  fireEvent.click(within(await menu("Person-AI")).getByRole("menuitem", { name: "modelProviders.actions.authorize" }));
  await waitFor(() => expect(fixture.authorize).toHaveBeenCalledExactlyOnceWith("PersonAi"));
  expect(open).toHaveBeenCalledWith("https://provider.test/authorize", "_blank", "noopener,noreferrer");
  await waitFor(() => expect(screen.getByTestId("location").textContent).toBe("/settings/model-providers/detail?providerCode=PersonAi&authorizationSessionId=session-one"));
});
