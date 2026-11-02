// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { DudeEditorDialog } from "./DudeEditorDialog";
const { create, update, soul, saveSoul, toastError } = vi.hoisted(() => ({
  create: vi.fn(), update: vi.fn(), toastError: vi.fn(),
  soul: vi.fn().mockResolvedValue({ name: "Memory", markdown: "", state: "Uninitialized" }),
  saveSoul: vi.fn().mockResolvedValue({ name: "Buddy", markdown: "", state: "Uninitialized" }),
}));
vi.mock("@/components/ui/toast", () => ({ useToast: () => ({ error: toastError }) }));
vi.mock("./SoulFields", () => ({ SoulFields: ({ name, onName }: { name: string; onName: (name: string) => void }) => <input aria-label="dude.name" value={name} onChange={e => onName(e.target.value)} /> }));
vi.mock("../api/dudeApi", () => ({ createDude: create, updateDude: update, getDudeSoul: soul, saveDudeSoul: saveSoul, getDudeResources: vi.fn() }));
vi.mock("@/domain/agent/components/AgentAvatarPicker", () => ({ AgentAvatarPicker: ({ onChange }: { onChange: (value: string) => void }) => <button type="button" onClick={() => onChange("neo")}>Neo</button> }));
vi.mock("@/domain/agent/components/AgentCandidateModelsField", () => ({ AgentCandidateModelsField: () => null }));
vi.mock("@/domain/agent/components/AgentRolePromptField", () => ({ AgentRolePromptField: ({ value, onChange, required }: { value: string; onChange: (value: string) => void; required?: boolean }) => <textarea aria-label="Role prompt" required={required} value={value} onChange={event => onChange(event.target.value)} /> }));
vi.mock("@/domain/agent/components/InternalAgentGroupField", () => ({ InternalAgentGroupField: () => <span>Agent group</span> }));
vi.mock("@/domain/agent/components/AgentResourcesEditor", () => ({ AgentResourcesEditor: () => null }));
vi.mock("react-i18next", async importOriginal => ({ ...await importOriginal<typeof import("react-i18next")>(), useTranslation: () => ({ t: (key: string) => key }) }));
afterEach(() => { cleanup(); vi.clearAllMocks(); });
it("shows the points warning and retries refresh without creating another Dude", async () => {
  create.mockResolvedValue({ dudeCode: "buddy" }); update.mockResolvedValue({ dudeCode: "buddy" });
  const onSaved = vi.fn().mockRejectedValueOnce(new Error("Refresh failed")).mockResolvedValueOnce(undefined);
  const close = vi.fn(); render(<DudeEditorDialog onClose={close} onSaved={onSaved} />);
  expect(screen.getByText("dude.costWarning")).toBeTruthy();
  expect(screen.queryByText("Agent group")).toBeNull();
  expect(screen.queryByLabelText("dude.kind")).toBeNull();
  expect((screen.getByLabelText("Role prompt") as HTMLTextAreaElement).required).toBe(true);
  fireEvent.change(screen.getByLabelText("dude.code"), { target: { value: "buddy" } });
  fireEvent.change(screen.getByLabelText("dude.name"), { target: { value: "Buddy" } });
  fireEvent.change(screen.getByLabelText("Role prompt"), { target: { value: "A helpful buddy" } });
  fireEvent.click(screen.getByRole("button", { name: "dude.save" }));
  await waitFor(() => expect(toastError).toHaveBeenCalledWith("Refresh failed"));
  fireEvent.click(screen.getByRole("button", { name: "dude.save" }));
  await waitFor(() => expect(close).toHaveBeenCalledOnce());
  expect(create).toHaveBeenCalledOnce(); expect(update).toHaveBeenCalledWith("buddy", expect.objectContaining({ avatarId: "robot" }));
  expect(create.mock.calls[0][0]).not.toHaveProperty("groupName");
  expect(update.mock.calls[0][1]).not.toHaveProperty("groupName");
  expect(screen.queryByLabelText("pointsBudget")).toBeNull();
});

it("names each background switch once and keeps help outside its accessible name", () => {
  render(<DudeEditorDialog dude={{dudeCode:"memory",name:"Memory",rolePrompt:"",avatarId:"robot",candidateModelCodes:[],enabled:true,heartbeatMinutes:30,memoryAutoApprove:false,kind:"Memory",isMain:false,createdAt:0,updatedAt:0}} onClose={vi.fn()} onSaved={vi.fn()} />);
  fireEvent.click(screen.getByRole("tab", { name: "dude.backgroundSection" }));
  const background = screen.getByRole("switch", {name:"dude.backgroundEnabled"});
  const approval = screen.getByRole("switch", {name:"dude.autoApprove"});
  expect(background.getAttribute("aria-checked")).toBe("true");
  expect(approval.getAttribute("aria-describedby")).toBeTruthy();
});

 it("retains profile edits when switching background settings", () => {
  render(<DudeEditorDialog onClose={vi.fn()} onSaved={vi.fn()} />);
  fireEvent.change(screen.getByLabelText("dude.name"), { target: { value: "My Buddy" } });
  fireEvent.click(screen.getByRole("tab", { name: "dude.backgroundSection" }));
  fireEvent.change(screen.getByLabelText("dude.heartbeatMinutes"), { target: { value: "5" } });
  fireEvent.click(screen.getByRole("tab", { name: "dude.profileSection" }));
  expect((screen.getByLabelText("dude.name") as HTMLInputElement).value).toBe("My Buddy");
  fireEvent.click(screen.getByRole("tab", { name: "dude.backgroundSection" }));
  expect((screen.getByLabelText("dude.heartbeatMinutes") as HTMLInputElement).value).toBe("5");
});

const existing = { dudeCode: "main-dude", name: "Memory", rolePrompt: "Help", avatarId: "robot", candidateModelCodes: [], enabled: true, heartbeatMinutes: 30, memoryAutoApprove: false, kind: "Main" as const, isMain: true, createdAt: 0, updatedAt: 0 };
it("saves an avatar without rewriting or confirming an unchanged soul", async () => {
  update.mockResolvedValue(existing);
  const close = vi.fn();
  render(<DudeEditorDialog dude={existing} onClose={close} onSaved={vi.fn().mockResolvedValue(undefined)} />);
  await waitFor(() => expect(soul).toHaveBeenCalled());
  fireEvent.click(screen.getByText("Neo"));
  fireEvent.click(screen.getByRole("button", { name: "dude.save" }));
  await waitFor(() => expect(close).toHaveBeenCalledOnce());
  expect(update).toHaveBeenCalledWith("main-dude", expect.objectContaining({ avatarId: "neo" }));
  expect(saveSoul).not.toHaveBeenCalled();
});
it("reports save failures as a toast and keeps the avatar draft open for retry", async () => {
  update.mockRejectedValueOnce(new Error("Save failed"));
  const close = vi.fn();
  render(<DudeEditorDialog dude={existing} onClose={close} onSaved={vi.fn()} />);
  await waitFor(() => expect(soul).toHaveBeenCalled());
  fireEvent.click(screen.getByText("Neo"));
  fireEvent.click(screen.getByRole("button", { name: "dude.save" }));
  await waitFor(() => expect(toastError).toHaveBeenCalledWith("Save failed"));
  expect(screen.queryByText("Save failed")).toBeNull();
  expect(close).not.toHaveBeenCalled();
  expect(saveSoul).not.toHaveBeenCalled();
});

it("exposes the same tabs and activation control for creation and editing", async () => {
  const props = { onClose: vi.fn(), onSaved: vi.fn() };
  const { rerender } = render(<DudeEditorDialog {...props} />);
  const creationTabs = screen.getAllByRole("tab").map((tab) => tab.textContent);
  expect(creationTabs).toEqual(["dude.profileSection", "dude.soulSection", "dude.backgroundSection", "dude.resourcesSection", "dude.advancedSection"]);
  expect(screen.getByRole("switch", { name: "dude.pause" })).toBeTruthy();
  rerender(<DudeEditorDialog {...props} dude={existing} />);
  await waitFor(() => expect(soul).toHaveBeenCalled());
  expect(screen.getAllByRole("tab").map((tab) => tab.textContent)).toEqual(creationTabs);
});
