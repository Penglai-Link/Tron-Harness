// @vitest-environment jsdom
import type { InputHTMLAttributes, ReactNode, SelectHTMLAttributes } from "react";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ConfiguredProvider, ConfiguredProviderCreate } from "@/domain/model/api/configuredProviderApi";

const fixture = vi.hoisted(() => ({
  fetch: vi.fn(), create: vi.fn(), update: vi.fn(), remove: vi.fn(), presets: vi.fn(), presetModels: vi.fn(),
  personAi: vi.fn(), toast: { success: vi.fn(), error: vi.fn() },
  t: (key: string, options?: Record<string, unknown>) => `${key}${options?.number ? `:${options.number}` : options?.name ? `:${options.name}` : ""}`,
}));
vi.mock("@/domain/model/api/configuredProviderApi", () => ({
  fetchConfiguredProvider: fixture.fetch, createConfiguredProvider: fixture.create,
  updateConfiguredProvider: fixture.update, deleteConfiguredProvider: fixture.remove,
  fetchProviderPresets: fixture.presets, fetchProviderPresetModels: fixture.presetModels,
}));
vi.mock("@/domain/model/api", () => ({
  fetchModelProviderConnection: fixture.personAi, createModelProviderAuthorization: vi.fn(),
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
    baseUrl: "https://provider.test/v1", hasApiKey: true, requestHeaders: { "x-client": "Tron" }, enabled: true,
    models: [{ modelCode: "model-stable", modelId: "remote-model", name: "My model", contextWindowTokens: 128000,
      maxOutputTokens: 4096, reasoning: false, supportsDeveloperRole: false, inputCapabilities: ["Text"], enabled: true }],
    createdAt: 1, updatedAt: 2, ...patch,
  };
}

function fill(label: string, value: string) {
  fireEvent.change(screen.getByLabelText(`configuredProviders.${label}`), { target: { value } });
}

function save() { fireEvent.click(screen.getAllByRole("button", { name: "configuredProviders.actions.save" })[0]!); }

beforeEach(() => { vi.clearAllMocks(); });
afterEach(cleanup);

describe("ConfiguredProviderPage", () => {
  it("creates independent provider and model identities without discovering remote models", async () => {
    fixture.create.mockImplementation(async (body: ConfiguredProviderCreate) => savedProvider({ ...body, hasApiKey: true }));
    render(<MemoryRouter><ConfiguredProviderPage /></MemoryRouter>);
    fill("fields.providerId", "my-openai");
    fill("fields.displayName", "My gateway");
    fill("fields.baseUrl", "https://gateway.test/custom/v1");
    fill("fields.apiKey", "test-key");
    fill("models.idNumber:1", "same-upstream-id");
    fill("models.nameNumber:1", "My model");
    fill("models.contextNumber:1", "128000");
    save();
    await waitFor(() => expect(fixture.create).toHaveBeenCalledTimes(1));
    expect(fixture.create.mock.calls[0][0]).toMatchObject({
      providerId: "my-openai", displayName: "My gateway", baseUrl: "https://gateway.test/custom/v1", apiKey: "test-key",
      models: [{ modelCode: null, modelId: "same-upstream-id", contextWindowTokens: 128000, maxOutputTokens: null }],
    });
    expect(fixture.presets).not.toHaveBeenCalled();
    expect(fixture.presetModels).not.toHaveBeenCalled();
  });

  it("routes custom detail away from PersonAI and preserves a saved key on update", async () => {
    fixture.fetch.mockResolvedValue(savedProvider());
    fixture.update.mockResolvedValue(savedProvider({ displayName: "Renamed" }));
    render(<MemoryRouter initialEntries={["/settings/model-providers/detail?providerCode=custom-one"]}><ModelProviderDetailsPage /></MemoryRouter>);
    await screen.findByDisplayValue("My provider");
    expect(fixture.fetch).toHaveBeenCalledWith("custom-one", expect.any(AbortSignal));
    expect(fixture.personAi).not.toHaveBeenCalled();
    expect((screen.getByLabelText("configuredProviders.fields.providerId") as HTMLInputElement).disabled).toBe(true);
    expect((screen.getByLabelText("configuredProviders.fields.apiKey") as HTMLInputElement).value).toBe("");
    fill("fields.displayName", "Renamed");
    save();
    await waitFor(() => expect(fixture.update).toHaveBeenCalledTimes(1));
    expect(fixture.update.mock.calls[0][0]).toBe("custom-one");
    expect(fixture.update.mock.calls[0][1]).toMatchObject({ apiKey: null, displayName: "Renamed", models: [{ modelCode: "model-stable" }] });
    expect(fixture.update.mock.calls[0][1]).not.toHaveProperty("providerId");
  });

  it("blocks duplicate model IDs and output limits that exceed context", async () => {
    fixture.fetch.mockResolvedValue(savedProvider());
    render(<MemoryRouter><ConfiguredProviderPage providerId="custom-one" /></MemoryRouter>);
    await screen.findByDisplayValue("My provider");
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
    render(<MemoryRouter><ConfiguredProviderPage fromCatalog /></MemoryRouter>);
    await screen.findByRole("option", { name: "Sample" });
    expect((screen.getByRole("option", { name: /Unsupported/ }) as HTMLOptionElement).disabled).toBe(true);
    fill("catalog.provider", "sample");
    await screen.findByRole("button", { name: "configuredProviders.catalog.addModel:Claude A" });
    expect(screen.queryByLabelText("configuredProviders.models.idNumber:1")).toBeNull();
    expect(screen.getByText("configuredProviders.catalog.guessed")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "configuredProviders.catalog.addModel:Claude A" }));
    expect((screen.getByLabelText("configuredProviders.models.idNumber:1") as HTMLInputElement).value).toBe("claude-a");
    expect((screen.getByLabelText("configuredProviders.models.outputNumber:1") as HTMLInputElement).value).toBe("");
    expect(screen.queryByLabelText("configuredProviders.models.idNumber:2")).toBeNull();
    save();
    expect(await screen.findByRole("alert")).toHaveProperty("textContent", "configuredProviders.validation.baseUrl");
    expect(fixture.create).not.toHaveBeenCalled();
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
