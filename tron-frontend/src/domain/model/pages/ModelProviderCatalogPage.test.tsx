// @vitest-environment jsdom
import type { ReactNode, SelectHTMLAttributes } from "react";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ProviderCatalogModel } from "@/domain/model/api";

const fixture = vi.hoisted(() => ({
  fetch: vi.fn(),
  sync: vi.fn(),
  toast: { error: vi.fn() },
  t: (key: string, options?: { count?: number }) => options?.count === undefined ? key : `${key}:${options.count}`,
  formatNumber: (value: number) => String(value),
  formatDateTime: () => "date",
}));

vi.mock("@/domain/model/api", () => ({ fetchProviderCatalog: fixture.fetch, syncModelProvider: fixture.sync }));
vi.mock("react-i18next", () => ({ useTranslation: () => ({ t: fixture.t }) }));
vi.mock("@/components/ui/toast", () => ({ useToast: () => fixture.toast }));
vi.mock("@/components/info-tooltip", () => ({ InfoTooltip: () => null }));
vi.mock("@/shared", () => ({
  PageHeader: ({ title, actions }: { title: string; actions: ReactNode }) => <header><h1>{title}</h1>{actions}</header>,
  useLocaleFormatters: () => ({ formatNumber: fixture.formatNumber, formatDateTime: fixture.formatDateTime }),
}));
vi.mock("@/components/forms", () => ({
  SelectInput: ({ onValueChange, ...props }: SelectHTMLAttributes<HTMLSelectElement> & { onValueChange: (value: string) => void }) => (
    <select {...props} onChange={(event) => onValueChange(event.currentTarget.value)} />
  ),
}));
vi.mock("@/components/ui/select", () => ({
  SelectItem: ({ value, children }: { value: string; children: ReactNode }) => <option value={value}>{children}</option>,
}));

import { ModelProviderCatalogPage } from "./ModelProviderCatalogPage";

function model(code: string, vendor: string, quality: "High" | "Middle" | "Low" = "High"): ProviderCatalogModel {
  return {
    providerCode: "PersonAi", catalogModelType: "Chat", modelId: null,
    modelCode: code, modelName: code, upstreamVendorCode: vendor, upstreamVendorName: vendor,
    qualityType: quality, protocols: ["CHAT_COMPLETIONS"], contextWindowTokens: 100000,
    inputCoefficient: 0.01, cacheHitCoefficient: 0.001, outputCoefficient: 0.1,
    status: "Available", firstSeenAt: 1, lastSeenAt: 1, updatedAt: 1,
  };
}

beforeEach(() => vi.resetAllMocks());
afterEach(cleanup);

describe("ModelProviderCatalogPage", () => {
  it("keeps six high-quality vendor entries distinct when filtering and refreshing", async () => {
    const high = [
      model("deepseek", "alibaba"), model("deepseek", "qianfan"),
      model("glm", "qianfan"), model("glm", "glm-plan"),
      model("qwen", "qwen-platform"), model("qwen", "alibaba"),
    ];
    const records = [...high, model("deepseek", "alibaba", "Middle"), model("qwen-low", "alibaba", "Low")];
    fixture.fetch.mockResolvedValue(records);
    fixture.sync.mockResolvedValue({ status: "Succeeded" });
    const errors = vi.spyOn(console, "error").mockImplementation(() => {});
    try {
      render(<MemoryRouter><ModelProviderCatalogPage /></MemoryRouter>);
      await screen.findByText("modelProviders.pagination.catalog:8");
      expect(fixture.sync).not.toHaveBeenCalled();
      expect(screen.getAllByRole("row")).toHaveLength(9);
      fireEvent.change(screen.getByLabelText("modelProviders.filters.quality"), { target: { value: "High" } });
      await screen.findByText("modelProviders.pagination.catalog:6");
      expect(screen.getAllByRole("row")).toHaveLength(7);
      fireEvent.change(screen.getByLabelText("modelProviders.filters.quality"), { target: { value: "Middle" } });
      await screen.findByText("modelProviders.pagination.catalog:1");
      expect(screen.getAllByRole("row")).toHaveLength(2);
      fixture.fetch.mockResolvedValue(high);
      fireEvent.click(screen.getByRole("button", { name: "modelProviders.actions.refresh" }));
      await waitFor(() => expect(fixture.sync).toHaveBeenCalledWith("PersonAi"));
      await screen.findByText("modelProviders.empty.catalogFiltered");
      fireEvent.change(screen.getByLabelText("modelProviders.filters.quality"), { target: { value: "High" } });
      await screen.findByText("modelProviders.pagination.catalog:6");
      expect(screen.getAllByRole("row")).toHaveLength(7);
      expect(fixture.sync.mock.invocationCallOrder[0]).toBeLessThan(fixture.fetch.mock.invocationCallOrder[1]);
      expect(errors).not.toHaveBeenCalled();
    } finally {
      errors.mockRestore();
    }
  });

  it("preserves the last catalogue and reports a remote sync failure", async () => {
    fixture.fetch.mockResolvedValue([model("deepseek", "alibaba")]);
    fixture.sync.mockRejectedValue(new Error("Person-AI unavailable"));
    render(<MemoryRouter><ModelProviderCatalogPage /></MemoryRouter>);
    await screen.findByText("modelProviders.pagination.catalog:1");
    fireEvent.click(screen.getByRole("button", { name: "modelProviders.actions.refresh" }));
    await waitFor(() => expect(fixture.toast.error).toHaveBeenCalledWith("Person-AI unavailable"));
    expect(fixture.fetch).toHaveBeenCalledTimes(1);
    expect(screen.getAllByRole("row")).toHaveLength(2);
    expect(screen.getByText("alibaba")).toBeTruthy();
  });
});
