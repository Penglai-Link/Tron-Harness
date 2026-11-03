// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { AutomaticRouter } from "../api/automaticRouterApi";

const fixture = vi.hoisted(() => ({ list: vi.fn(), save: vi.fn(), toast: { success: vi.fn(), error: vi.fn() }, t: (key: string) => key }));
vi.mock("react-i18next", () => ({ useTranslation: () => ({ t: fixture.t, i18n: { language: "en-US" } }) }));
vi.mock("../api/automaticRouterApi", () => ({ fetchAutomaticRouters: fixture.list, updateAutomaticRouter: fixture.save }));
vi.mock("@/components/ui/toast", () => ({ useToast: () => fixture.toast }));
vi.mock("@/components/info-tooltip", () => ({ InfoTooltip: () => null }));
vi.mock("@/shared", () => ({ PageHeader: ({ actions }: { actions: React.ReactNode }) => <header>{actions}</header> }));
import { AutomaticRoutersPage } from "./AutomaticRoutersPage";

beforeEach(() => {
  vi.clearAllMocks();
  const routes: AutomaticRouter[] = ["Chat", "TextGeneration", "Embedding", "Rerank"].map(kind => ({ kind: kind as AutomaticRouter["kind"], sources: { mode: "AllProviders", providerIds: [], modelCodes: [] }, candidates: [
    { providerId: "account-a", providerName: "Work key", modelCode: "model-a", modelName: "Shared model", bindingId: "binding", inputModalities: ["Text", "Image"], dimension: 1024 },
    { providerId: "account-b", providerName: "Personal key", modelCode: "model-b", modelName: "Shared model", bindingId: "binding", inputModalities: ["Text", "Image"], dimension: 1024 },
  ] }));
  fixture.list.mockResolvedValue(routes);
  fixture.save.mockResolvedValue({});
});
afterEach(cleanup);

it("shows four unified routes and saves an explicit empty selection without broadening it", async () => {
  render(<AutomaticRoutersPage />);
  const section = await screen.findByRole("region", { name: "automaticRouters.kinds.Rerank" });
  expect(screen.getAllByRole("region")).toHaveLength(4);
  const trigger = within(section).getByRole("combobox");
  fireEvent.keyDown(trigger, { key: "ArrowDown" });
  const option = await screen.findByRole("option", { name: "automaticRouters.modes.SelectedModels" });
  fireEvent.pointerDown(option, { pointerType: "mouse" });
  fireEvent.click(option);
  fireEvent.click(within(section).getByRole("button", { name: "automaticRouters.save" }));
  await waitFor(() => expect(fixture.save).toHaveBeenCalledWith("Rerank", { mode: "SelectedModels", providerIds: [], modelCodes: [] }));
  expect(fixture.toast.success).toHaveBeenCalledWith("automaticRouters.saved");
});

it("keeps a failed save editable and reports its cause", async () => {
  fixture.save.mockRejectedValueOnce(new Error("Storage unavailable"));
  render(<AutomaticRoutersPage />);
  const section = await screen.findByRole("region", { name: "automaticRouters.kinds.Embedding" });
  fireEvent.keyDown(within(section).getByRole("combobox"), { key: "ArrowDown" });
  const option = await screen.findByRole("option", { name: "automaticRouters.modes.SelectedProviders" });
  fireEvent.pointerDown(option, { pointerType: "mouse" });
  fireEvent.click(option);
  fireEvent.click(within(section).getByRole("button", { name: "automaticRouters.save" }));
  await waitFor(() => expect(fixture.toast.error).toHaveBeenCalledWith("Storage unavailable"));
  expect(within(section).getByRole("button", { name: "automaticRouters.save" }).hasAttribute("disabled")).toBe(false);
});
