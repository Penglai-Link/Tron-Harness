// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { CapabilityPackDetailPage } from "./CapabilityPackDetailPage";
import { CapabilityPacksPage } from "./CapabilityPacksPage";
import { listCapabilityPacks, setCapabilityPackEnabled, type CapabilityPackRecord } from "../api/capabilityApi";

vi.mock("../api/capabilityApi", () => ({ listCapabilityPacks: vi.fn(), setCapabilityPackEnabled: vi.fn(), importCapabilityPack: vi.fn() }));
vi.mock("react-i18next", async (importOriginal) => ({ ...await importOriginal<typeof import("react-i18next")>(), useTranslation: () => ({ t: (key: string, options?: { defaultValue?: string }) => options?.defaultValue ?? key }) }));
const pack: CapabilityPackRecord = {
  id: "framework.a2ui", name: "A2UI Interfaces", version: "0.1.0", description: "Interactive interfaces",
  provider: "EliteForge", updatedAt: null, effective: true, kind: "capability",
  contents: [{ kind: "Tool", name: "a2ui_publish", description: "Publish a surface" }, { kind: "Skill", name: "a2ui-interface", description: "" }],
};
function mount(path = "/settings/capability-packs/framework.a2ui") {
  return render(<MemoryRouter initialEntries={[path]}><Routes>
    <Route path="/settings/capability-packs" element={<CapabilityPacksPage />} />
    <Route path="/settings/capability-packs/:packId" element={<CapabilityPackDetailPage />} />
  </Routes></MemoryRouter>);
}
beforeEach(() => { vi.mocked(listCapabilityPacks).mockResolvedValue([pack]); vi.mocked(setCapabilityPackEnabled).mockResolvedValue(pack); });
afterEach(() => { cleanup(); vi.clearAllMocks(); });
it("opens a pack from the list, shows its tools and skills, and returns", async () => {
  mount("/settings/capability-packs");
  fireEvent.click(await screen.findByRole("link", { name: pack.name }));
  await screen.findByText("a2ui_publish");
  expect(screen.getByText("Publish a surface")).toBeTruthy();
  fireEvent.click(screen.getByRole("tab", { name: /Skill/ }));
  expect(await screen.findByText("a2ui-interface")).toBeTruthy();
  fireEvent.click(screen.getByRole("link", { name: "shell:settings.items.capabilityPacks" }));
  expect(await screen.findByRole("link", { name: pack.name })).toBeTruthy();
});
it("keeps disabling separate from detail navigation", async () => {
  mount("/settings/capability-packs");
  fireEvent.click(await screen.findByRole("button", { name: "pages:capabilityManagement.disable" }));
  await waitFor(() => expect(setCapabilityPackEnabled).toHaveBeenCalledWith(pack.id, false));
  expect(screen.queryByText("a2ui_publish")).toBeNull();
});
it("handles a missing pack", async () => {
  mount("/settings/capability-packs/removed");
  expect(await screen.findByText("pages:capabilityManagement.notFound")).toBeTruthy();
});
it("shows a load error and recovers on refresh", async () => {
  vi.mocked(listCapabilityPacks).mockRejectedValueOnce(new Error("offline"));
  mount();
  expect(await screen.findByRole("alert")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "pages:capabilityManagement.refresh" }));
  expect(await screen.findByText("a2ui_publish")).toBeTruthy();
});
it("shows empty declared contents for a pack with no contributions", async () => {
  vi.mocked(listCapabilityPacks).mockResolvedValue([{ ...pack, contents: [] }]);
  mount();
  expect(await screen.findByText("pages:capabilityManagement.emptyContents")).toBeTruthy();
});

it("groups contributions into tabs and shows plugin descriptions and provided capabilities", async () => {
  vi.mocked(listCapabilityPacks).mockResolvedValue([{ ...pack, contents: [
    ...pack.contents,
    { kind: "Plugin", name: "agent.a2ui", description: "Interactive interface plugin", provides: ["A2uiService", "a2ui-interface"] },
    { kind: "Implementation", name: "A2uiService", description: "tool: Interactive surfaces" },
    { kind: "Resource", name: "references/examples.json", description: "Skill examples" },
  ] }]);
  mount();
  expect(await screen.findByText("a2ui_publish")).toBeTruthy();
  expect(screen.queryByText("agent.a2ui")).toBeNull();
  fireEvent.click(screen.getByRole("tab", { name: /Plugin/ }));
  expect(await screen.findByText("agent.a2ui")).toBeTruthy();
  expect(screen.getByText("Interactive interface plugin")).toBeTruthy();
  expect(screen.getByText("A2uiService")).toBeTruthy();
  expect(screen.getByText("tool: Interactive surfaces")).toBeTruthy();
  expect(screen.getByText("a2ui-interface")).toBeTruthy();
  expect(screen.queryByText("a2ui_publish")).toBeNull();
  fireEvent.click(screen.getByRole("tab", { name: /Resource/ }));
  expect(await screen.findByText("references/examples.json")).toBeTruthy();
});
