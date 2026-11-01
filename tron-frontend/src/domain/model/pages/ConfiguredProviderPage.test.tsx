// @vitest-environment jsdom
import type { InputHTMLAttributes, ReactNode, SelectHTMLAttributes } from "react";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, useLocation } from "react-router";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ConfiguredProvider, ConfiguredProviderCreate } from "@/domain/model/api/configuredProviderApi";

const fixture = vi.hoisted(() => ({
  fetch: vi.fn(), create: vi.fn(), update: vi.fn(), remove: vi.fn(), presets: vi.fn(), presetModels: vi.fn(),
  personAi: vi.fn(), providers: vi.fn(), catalog: vi.fn(), toast: { success: vi.fn(), error: vi.fn() },
  t: (key: string, options?: Record<string, unknown>) => `${key}${options?.number ? `:${options.number}` : options?.name ? `:${options.name}` : ""}`,
}));
vi.mock("@/domain/model/api/configuredProviderApi", () => ({
  fetchConfiguredProvider: fixture.fetch, createConfiguredProvider: fixture.create,
  updateConfiguredProvider: fixture.update, deleteConfiguredProvider: fixture.remove,
  fetchProviderPresets: fixture.presets, fetchProviderPresetModels: fixture.presetModels,
}));
vi.mock("@/domain/model/api", () => ({
  fetchModelProviderConnection: fixture.personAi, fetchModelProviderConnections: fixture.providers, createModelProviderAuthorization: vi.fn(),
  fetchProviderCatalog: fixture.catalog,
  disconnectModelProvider: vi.fn(), fetchProviderAccountBalance: vi.fn(), fetchModelProviderAuthorization: vi.fn(),
  pollModelProviderAuthorization: vi.fn(), syncModelProvider: vi.fn(),
}));
vi.mock("react-i18next", () => ({ useTranslation: () => ({ t: fixture.t }) }));
vi.mock("@/components/ui/toast", () => ({ useToast: () => fixture.toast }));
vi.mock("@/components/forms", () => ({
  Field: ({ children, label }: { children: ReactNode; label: ReactNode }) => <div><span>{label}</span>{children}</div>,
  TextInput: (props: InputHTMLAttributes<HTMLInputElement>) => <input {...props} />,
  SelectInput: ({ onValueChange, value, ...props }: SelectHTMLAttributes<HTMLSelectElement> & { onValueChange: (value: string) => void }) => <select {...props} value={value ?? ""} onChange={(event) => onValueChange(event.currentTarget.value)} />,
  ToggleSwitch: ({ label, ariaLabel, checked, disabled, onCheckedChange }: { label: string; ariaLabel: string; checked: boolean; disabled: boolean; onCheckedChange: (value: boolean) => void }) => <label><input type="checkbox" aria-label={ariaLabel} checked={checked} disabled={disabled} onChange={(event) => onCheckedChange(event.currentTarget.checked)} />{label}</label>,
}));
vi.mock("@/components/ui/select", () => ({
  SelectItem: ({ value, children, disabled }: { value: string; children: ReactNode; disabled?: boolean }) => <option value={value} disabled={disabled}>{children}</option>,
}));
vi.mock("@/shared", () => ({
  PageHeader: ({ title, actions }: { title: string; actions: ReactNode }) => <header><h1>{title}</h1>{actions}</header>,
  SurfaceCard: ({ children }: { children: ReactNode }) => <div>{children}</div>,
  useLocaleFormatters: () => ({ formatNumber: String, formatDateTime: String }),
}));
vi.mock("@/components/info-tooltip", () => ({ InfoTooltip: () => null }));

import { ConfiguredProviderPage } from "./ConfiguredProviderPage";
import { ModelProviderDetailsPage } from "./ModelProviderDetailsPage";

function savedProvider(patch: Partial<ConfiguredProvider> = {}): ConfiguredProvider {
  return {
    providerId: "custom-one", displayName: "My provider", protocol: "OpenAiChatCompletions",
    baseUrl: "https://provider.test/v1", hasApiKey: true, apiKeySummary: "sk-123***sdf-hsd", requestHeaders: { "x-client": "Tron" }, enabled: true,
    models: [{ modelType: "Text", modelCode: "model-stable", modelId: "remote-model", name: "My model", contextWindowTokens: 128000,
      maxOutputTokens: 4096, reasoning: false, supportsDeveloperRole: false, inputCapabilities: ["Text"], enabled: true }],
    createdAt: 1, updatedAt: 2, ...patch,
  };
}

function fill(label: string, value: string) {
  fireEvent.change(screen.getByLabelText(`configuredProviders.${label}`), { target: { value } });
}

function openProviderOptions() {
  fireEvent.keyDown(screen.getByRole("combobox", { name: "configuredProviders.catalog.provider" }), { key: "ArrowDown" });
}

async function chooseProvider(name: string) {
  openProviderOptions();
  fireEvent.click(await screen.findByRole("option", { name }));
}

function save() { fireEvent.click(screen.getAllByRole("button", { name: "configuredProviders.actions.save" })[0]!); }
function editModel(number: number) {
  const button = screen.getByRole("button", { name: `configuredProviders.models.editNumber:${number}` });
  if (button.getAttribute("aria-expanded") !== "true") fireEvent.click(button);
}
function CurrentPath() { return <output data-testid="current-path">{useLocation().pathname}</output>; }

beforeEach(() => { vi.clearAllMocks(); fixture.catalog.mockReset().mockResolvedValue([]); fixture.presets.mockReset().mockResolvedValue([]); fixture.presetModels.mockReset().mockResolvedValue([]); fixture.providers.mockReset().mockResolvedValue([{ providerCode: "custom-one", source: "Custom", displayName: "My provider" }, { providerCode: "PersonAi", source: "PersonAi", displayName: "Person-AI" }]); });
afterEach(cleanup);

describe("ConfiguredProviderPage", () => {
  it.each(["create", "update"])("returns to the provider list after successful %s", async mode => {
    fixture.fetch.mockResolvedValue(savedProvider());
    fixture.create.mockResolvedValue(savedProvider());
    fixture.update.mockResolvedValue(savedProvider());
    render(<MemoryRouter initialEntries={[mode === "create" ? "/settings/model-providers/new" : "/settings/model-providers/detail"]}>
      <CurrentPath /><ConfiguredProviderPage providerId={mode === "update" ? "custom-one" : undefined} />
    </MemoryRouter>);
    if (mode === "create") {
      await chooseProvider("configuredProviders.catalog.customProvider");
      fill("fields.displayName", "New provider");
      fill("fields.baseUrl", "https://provider.test/v1");
      fill("models.idNumber:1", "model");
      fill("models.nameNumber:1", "Model");
    } else await screen.findByDisplayValue("My provider");
    save();
    await waitFor(() => expect(screen.getByTestId("current-path").textContent).toBe("/settings/model-providers"));
  });

  it("keeps saved models compact and preserves edits when collapsed", async () => {
    fixture.fetch.mockResolvedValue(savedProvider());
    fixture.update.mockResolvedValue(savedProvider());
    render(<MemoryRouter><ConfiguredProviderPage providerId="custom-one" /></MemoryRouter>);
    await screen.findByText("remote-model");
    expect(screen.queryByLabelText("configuredProviders.models.idNumber:1")).toBeNull();
    editModel(1);
    fill("models.contextNumber:1", "256000");
    fireEvent.click(screen.getByRole("button", { name: "configuredProviders.models.editNumber:1" }));
    expect(screen.queryByLabelText("configuredProviders.models.contextNumber:1")).toBeNull();
    editModel(1);
    expect((screen.getByLabelText("configuredProviders.models.contextNumber:1") as HTMLInputElement).value).toBe("256000");
    fireEvent.click(screen.getByRole("button", { name: "configuredProviders.models.editNumber:1" }));
    save();
    await waitFor(() => expect(fixture.update).toHaveBeenCalledTimes(1));
    expect(fixture.update.mock.calls[0][1].models[0].contextWindowTokens).toBe(256000);
  });
  it("unifies multi-selection, removal and manual media models after connection settings", async () => {
    fixture.presets.mockResolvedValue([{ id: "sample", name: "Sample", protocol: "OpenAiChatCompletions", baseUrl: "https://provider.test/v1", supported: true }]);
    fixture.presetModels.mockResolvedValue(["a", "b"].map(id => ({ modelId: id, name: `Model ${id}`, contextWindowTokens: null, reasoning: false, inputCapabilities: ["text"], toolUse: true })));
    fixture.create.mockImplementation(async (body: ConfiguredProviderCreate) => savedProvider({ ...body, hasApiKey: false }));
    render(<MemoryRouter><ConfiguredProviderPage /></MemoryRouter>);
    await chooseProvider("Sample");
    const picker = await screen.findByRole("combobox", { name: "configuredProviders.catalog.selectModels" });
    await waitFor(() => expect((picker as HTMLInputElement).disabled).toBe(false));
    expect(screen.queryByText("configuredProviders.catalog.modelCount")).toBeNull();
    expect(screen.getByText("configuredProviders.connection").compareDocumentPosition(picker) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    const toggle = async (name: string) => {
      fireEvent.keyDown(picker, { key: "ArrowDown" });
      fireEvent.click(await screen.findByRole("option", { name }));
      fireEvent.keyDown(picker, { key: "Escape" });
    };
    fireEvent.change(picker, { target: { value: "Model a" } });
    fireEvent.keyDown(picker, { key: "ArrowDown" });
    fireEvent.click(await screen.findByRole("option", { name: "Model a a" }));
    expect(screen.queryByLabelText("configuredProviders.models.nameNumber:1")).toBeNull();
    expect(screen.getByText("a")).toBeTruthy();
    fireEvent.keyDown(picker, { key: "Escape" });
    expect(screen.queryByLabelText("configuredProviders.models.nameNumber:1")).toBeNull();
    fireEvent.change(picker, { target: { value: "" } });
    editModel(1);
    fill("models.nameNumber:1", "Edited a");
    await toggle("Model b b");
    expect((screen.getByLabelText("configuredProviders.models.nameNumber:1") as HTMLInputElement).value).toBe("Edited a");
    fireEvent.click(screen.getByRole("button", { name: "configuredProviders.models.add" }));
    fill("models.idNumber:3", "qwen-asr");
    fill("models.nameNumber:3", "Speech recognition");
    fireEvent.change(screen.getByLabelText("configuredProviders.models.typeNumber:3"), { target: { value: "SpeechRecognition" } });
    await toggle("Model a a");
    expect(screen.getByText("b")).toBeTruthy();
    expect(screen.queryByLabelText("configuredProviders.models.idNumber:1")).toBeNull();
    expect((screen.getByLabelText("configuredProviders.models.idNumber:2") as HTMLInputElement).value).toBe("qwen-asr");
    save();
    await waitFor(() => expect(fixture.create).toHaveBeenCalledTimes(1));
    expect(fixture.create.mock.calls[0][0].models).toMatchObject([
      { modelId: "b", modelType: "Text" },
      { modelId: "qwen-asr", modelType: "SpeechRecognition", inputCapabilities: ["Audio"], contextWindowTokens: null, maxOutputTokens: null },
    ]);
  });
  it("creates independent provider and model identities without discovering remote models", async () => {
    fixture.create.mockImplementation(async (body: ConfiguredProviderCreate) => savedProvider({ ...body, hasApiKey: true }));
    render(<MemoryRouter><ConfiguredProviderPage /></MemoryRouter>);
    expect(screen.queryByLabelText("configuredProviders.fields.providerId")).toBeNull();
    await chooseProvider("configuredProviders.catalog.customProvider");
    fill("fields.displayName", "My gateway");
    fill("fields.baseUrl", "https://gateway.test/custom/v1");
    fill("fields.apiKey", "test-key");
    fill("models.idNumber:1", "same-upstream-id");
    fill("models.nameNumber:1", "My model");
    fill("models.contextNumber:1", "128000");
    save();
    await waitFor(() => expect(fixture.create).toHaveBeenCalledTimes(1));
    expect(fixture.create.mock.calls[0][0]).toMatchObject({
      displayName: "My gateway", baseUrl: "https://gateway.test/custom/v1", apiKey: "test-key",
      models: [{ modelType: "Text", modelCode: null, modelId: "same-upstream-id", contextWindowTokens: 128000, maxOutputTokens: null }],
    });
    expect(fixture.create.mock.calls[0][0]).not.toHaveProperty("providerId");
    expect(fixture.presets).toHaveBeenCalledTimes(1);
    expect(fixture.presetModels).not.toHaveBeenCalled();
  });

  it("routes custom detail away from PersonAI and preserves a saved key on update", async () => {
    fixture.fetch.mockResolvedValue(savedProvider());
    fixture.update.mockResolvedValue(savedProvider({ displayName: "Renamed" }));
    render(<MemoryRouter initialEntries={["/settings/model-providers/detail?providerCode=custom-one"]}><ModelProviderDetailsPage /></MemoryRouter>);
    await screen.findByDisplayValue("My provider");
    expect(fixture.fetch).toHaveBeenCalledWith("custom-one", expect.any(AbortSignal));
    expect(fixture.personAi).not.toHaveBeenCalled();
    expect(screen.queryByLabelText("configuredProviders.fields.providerId")).toBeNull();
    const keyInput = screen.getByLabelText("configuredProviders.fields.apiKey") as HTMLInputElement;
    expect(keyInput.value).toBe("sk-123***sdf-hsd");
    fireEvent.focus(keyInput);
    expect(keyInput.value).toBe("");
    fill("fields.displayName", "Renamed");
    save();
    await waitFor(() => expect(fixture.update).toHaveBeenCalledTimes(1));
    expect(fixture.update.mock.calls[0][0]).toBe("custom-one");
    expect(fixture.update.mock.calls[0][1]).toMatchObject({ apiKey: null, displayName: "Renamed", models: [{ modelType: "Text", modelCode: "model-stable" }] });
    expect(fixture.update.mock.calls[0][1]).not.toHaveProperty("providerId");
  });

  it("keeps plugin providers out of the API-key create flow", async () => {
    render(<MemoryRouter><ConfiguredProviderPage /></MemoryRouter>);
    openProviderOptions();
    await screen.findByRole("option", { name: "configuredProviders.catalog.newFromExisting:My provider" });
    expect(screen.queryByRole("option", { name: "Person-AI" })).toBeNull();
    expect(fixture.catalog).not.toHaveBeenCalled();
    expect(fixture.fetch).not.toHaveBeenCalled();
  });

  it("filters by connection source rather than special-casing the Person-AI name", async () => {
    fixture.providers.mockResolvedValue([
      { providerCode: "another-plugin", source: "Plugin", displayName: "Another Plugin" },
      { providerCode: "renamed-personai", source: "PersonAi", displayName: "My authorized account" },
    ]);
    fixture.presets.mockResolvedValue([{ id: "sample", name: "Sample API", supported: true, protocol: "OpenAiChatCompletions", baseUrl: "https://api.example/v1" }]);
    render(<MemoryRouter><ConfiguredProviderPage /></MemoryRouter>);
    openProviderOptions();
    await screen.findByRole("option", { name: "Sample API" });
    expect(screen.queryByRole("option", { name: "Another Plugin" })).toBeNull();
    expect(screen.queryByRole("option", { name: "My authorized account" })).toBeNull();
    await chooseProvider("configuredProviders.catalog.customProvider");
    expect(screen.getByLabelText("configuredProviders.fields.apiKey")).toBeTruthy();
  });

  it("adds a second key from an existing connection while preserving its own name and model identities", async () => {
    fixture.providers.mockResolvedValue([{ providerCode: "custom-one", source: "Custom", displayName: "My provider" }]);
    fixture.fetch.mockResolvedValue(savedProvider());
    fixture.create.mockImplementation(async (body: ConfiguredProviderCreate) => savedProvider({ ...body, hasApiKey: true }));
    render(<MemoryRouter><ConfiguredProviderPage /></MemoryRouter>);
    await chooseProvider("configuredProviders.catalog.newFromExisting:My provider");
    await screen.findByDisplayValue("My provider (2)");
    expect((screen.getByLabelText("configuredProviders.fields.apiKey") as HTMLInputElement).value).toBe("");
    fill("fields.displayName", "My work account");
    fill("fields.apiKey", "second-account-key");
    save();
    await waitFor(() => expect(fixture.create).toHaveBeenCalledTimes(1));
    expect(fixture.create.mock.calls[0][0]).toMatchObject({
      displayName: "My work account", apiKey: "second-account-key", baseUrl: "https://provider.test/v1",
      models: [{ modelId: "remote-model", modelCode: null }],
    });
    expect(fixture.create.mock.calls[0][0]).not.toHaveProperty("providerId");
    expect(fixture.update).not.toHaveBeenCalled();
  });

  it("ignores an old connection copy after choosing a different provider", async () => {
    fixture.providers.mockResolvedValue([{ providerCode: "custom-one", source: "Custom", displayName: "My provider" }]);
    let resolve!: (value: ConfiguredProvider) => void;
    fixture.fetch.mockReturnValue(new Promise<ConfiguredProvider>(done => { resolve = done; }));
    render(<MemoryRouter><ConfiguredProviderPage /></MemoryRouter>);
    await chooseProvider("configuredProviders.catalog.newFromExisting:My provider");
    expect(screen.queryByLabelText("configuredProviders.fields.providerId")).toBeNull();
    await chooseProvider("configuredProviders.catalog.customProvider");
    fill("fields.displayName", "My new selection");
    await act(async () => { resolve(savedProvider()); });
    expect((screen.getByLabelText("configuredProviders.fields.displayName") as HTMLInputElement).value).toBe("My new selection");
    expect((fixture.fetch.mock.calls.at(-1)?.[1] as AbortSignal).aborted).toBe(true);
  });

  it("retries loading an existing configuration without enabling save on failure", async () => {
    fixture.providers.mockResolvedValue([{ providerCode: "custom-one", source: "Custom", displayName: "My provider" }]);
    fixture.fetch.mockRejectedValueOnce(new Error("Connection unavailable")).mockResolvedValue(savedProvider());
    render(<MemoryRouter><ConfiguredProviderPage /></MemoryRouter>);
    await chooseProvider("configuredProviders.catalog.newFromExisting:My provider");
    await screen.findByText("Connection unavailable");
    expect(screen.queryByLabelText("configuredProviders.fields.apiKey")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "configuredProviders.actions.retry" }));
    await screen.findByDisplayValue("My provider (2)");
    expect((screen.getByLabelText("configuredProviders.fields.apiKey") as HTMLInputElement).value).toBe("");
  });

  it("uses the shared detail route for a plugin provider without a fixed provider name", async () => {
    fixture.providers.mockResolvedValue([{ providerCode: "example-plugin", source: "Plugin", displayName: "Example Plugin" }]);
    fixture.personAi.mockResolvedValue({ providerCode: "example-plugin", displayName: "Example Plugin", status: "Disconnected", catalogModelCount: 0 });
    render(<MemoryRouter initialEntries={["/settings/model-providers/detail?providerCode=example-plugin"]}><ModelProviderDetailsPage /></MemoryRouter>);
    await screen.findByText("Example Plugin");
    expect(fixture.personAi).toHaveBeenCalledWith("example-plugin");
    expect(screen.getByRole("heading", { name: "configuredProviders.editTitle" })).toBeTruthy();
  });

  it("retains Person-AI's managed authorization detail instead of the API-key form", async () => {
    fixture.personAi.mockResolvedValue({ providerCode: "PersonAi", displayName: "Person-AI", status: "Disconnected", catalogModelCount: 16 });
    render(<MemoryRouter initialEntries={["/settings/model-providers/detail?providerCode=PersonAi"]}><ModelProviderDetailsPage /></MemoryRouter>);
    await screen.findByText("Person-AI");
    expect(fixture.personAi).toHaveBeenCalledWith("PersonAi");
    expect(fixture.fetch).not.toHaveBeenCalled();
    expect(screen.queryByLabelText("configuredProviders.fields.apiKey")).toBeNull();
  });

  it("allows manual configuration when the catalog fails to load", async () => {
    fixture.presets.mockRejectedValue(new Error("Catalog unavailable"));
    render(<MemoryRouter><ConfiguredProviderPage /></MemoryRouter>);
    await screen.findByText("Catalog unavailable");
    await chooseProvider("configuredProviders.catalog.customProvider");
    expect(screen.getByLabelText("configuredProviders.fields.displayName")).toBeTruthy();
    expect(screen.queryByLabelText("configuredProviders.fields.providerId")).toBeNull();
    expect(fixture.presetModels).not.toHaveBeenCalled();
  });

  it("blocks duplicate model IDs and output limits that exceed context", async () => {
    fixture.fetch.mockResolvedValue(savedProvider());
    render(<MemoryRouter><ConfiguredProviderPage providerId="custom-one" /></MemoryRouter>);
    await screen.findByDisplayValue("My provider");
    editModel(1);
    fill("models.outputNumber:1", "128001");
    save();
    expect(await screen.findByRole("alert")).toHaveProperty("textContent", "configuredProviders.validation.outputLimit");
    fill("models.outputNumber:1", "4096");
    fireEvent.click(screen.getByRole("button", { name: "configuredProviders.models.add" }));
    fill("models.idNumber:2", "remote-model");
    fill("models.nameNumber:2", "Another model");
    save();
    expect(await screen.findByRole("alert")).toHaveProperty("textContent", "configuredProviders.validation.duplicateModel");
    expect(fixture.update).not.toHaveBeenCalled();
  });

  it("adds only explicitly selected catalog models and leaves unknown output limits blank", async () => {
    fixture.presets.mockResolvedValue([
      { id: "sample", name: "Sample", protocol: "AnthropicMessages", baseUrl: null, envKey: "SAMPLE_KEY", guessed: true, needsBaseUrl: true, supported: true, unsupportedReason: null, modelCount: 2 },
      { id: "unsupported", name: "Unsupported", protocol: null, baseUrl: null, envKey: null, guessed: false, needsBaseUrl: false, supported: false, unsupportedReason: "proprietary-sdk", modelCount: 1 },
    ]);
    fixture.presetModels.mockResolvedValue([
      { modelId: "claude-a", name: "Claude A", contextWindowTokens: 200000, reasoning: true, inputCapabilities: ["text", "image", "video"], toolUse: true },
      { modelId: "claude-b", name: "Claude B", contextWindowTokens: null, reasoning: false, inputCapabilities: ["text"], toolUse: true },
    ]);
    render(<MemoryRouter><ConfiguredProviderPage /></MemoryRouter>);
    openProviderOptions();
    await screen.findByRole("option", { name: "Sample" });
    expect(screen.getByRole("option", { name: /Unsupported/ }).getAttribute("aria-disabled")).toBe("true");
    fireEvent.click(screen.getByRole("option", { name: "Sample" }));
    const modelPicker = await screen.findByRole("combobox", { name: "configuredProviders.catalog.selectModels" });
    await waitFor(() => expect((modelPicker as HTMLInputElement).disabled).toBe(false));
    expect(screen.queryByLabelText("configuredProviders.models.idNumber:1")).toBeNull();
    expect(screen.getByText("configuredProviders.catalog.guessed")).toBeTruthy();
    fireEvent.keyDown(modelPicker, { key: "ArrowDown" });
    fireEvent.click(await screen.findByRole("option", { name: /Claude A/ }));
    fireEvent.keyDown(modelPicker, { key: "Escape" });
    expect(screen.getByText("claude-a")).toBeTruthy();
    expect(screen.queryByLabelText("configuredProviders.models.idNumber:1")).toBeNull();
    editModel(1);
    expect((screen.getByLabelText("configuredProviders.models.idNumber:1") as HTMLInputElement).value).toBe("claude-a");
    expect((screen.getByLabelText("configuredProviders.models.outputNumber:1") as HTMLInputElement).value).toBe("");
    expect(screen.queryByLabelText("configuredProviders.models.idNumber:2")).toBeNull();
    save();
    expect(await screen.findByRole("alert")).toHaveProperty("textContent", "configuredProviders.validation.baseUrl");
    expect(fixture.create).not.toHaveBeenCalled();
    await chooseProvider("configuredProviders.catalog.customProvider");
    expect(screen.queryByLabelText("configuredProviders.fields.providerId")).toBeNull();
    expect((screen.getByLabelText("configuredProviders.models.idNumber:1") as HTMLInputElement).value).toBe("");
  });

  it("keeps the draft editable after a failed save and deletes only after confirmation", async () => {
    fixture.fetch.mockResolvedValue(savedProvider());
    fixture.update.mockRejectedValue(new Error("Connection validation failed"));
    fixture.remove.mockResolvedValue({});
    render(<MemoryRouter><ConfiguredProviderPage providerId="custom-one" /></MemoryRouter>);
    await screen.findByDisplayValue("My provider");
    fill("fields.displayName", "Changed draft");
    save();
    await screen.findByText("Connection validation failed");
    expect((screen.getByLabelText("configuredProviders.fields.displayName") as HTMLInputElement).value).toBe("Changed draft");
    fireEvent.click(screen.getByRole("button", { name: "configuredProviders.actions.delete" }));
    expect(fixture.remove).not.toHaveBeenCalled();
    const buttons = screen.getAllByRole("button", { name: "configuredProviders.actions.delete" });
    fireEvent.click(buttons[buttons.length - 1]!);
    await waitFor(() => expect(fixture.remove).toHaveBeenCalledWith("custom-one"));
  });
});
