// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { DudeEditorDialog } from "./DudeEditorDialog";
const { create, update } = vi.hoisted(() => ({ create: vi.fn(), update: vi.fn() }));
vi.mock("../api/dudeApi", () => ({ createDude: create, updateDude: update }));
vi.mock("@/domain/agent/components/AgentAvatarPicker", () => ({ AgentAvatarPicker: () => null }));
vi.mock("@/domain/agent/components/AgentLoopField", () => ({ AgentLoopField: ({ onChange }: { onChange: (value: string) => void }) => <button type="button" onClick={() => onChange("main")}>Choose loop</button> }));
vi.mock("@/domain/agent/components/AgentCandidateModelsField", () => ({ AgentCandidateModelsField: () => null }));
vi.mock("@/domain/agent/components/AgentRolePromptField", () => ({ AgentRolePromptField: () => null }));
vi.mock("@/domain/agent/components/InternalAgentGroupField", () => ({ InternalAgentGroupField: () => <span>Agent group</span> }));
vi.mock("./DudeResourceFields", () => ({ DudeResourceFields: () => null }));
vi.mock("react-i18next", async importOriginal => ({ ...await importOriginal<typeof import("react-i18next")>(), useTranslation: () => ({ t: (key: string) => key }) }));
afterEach(() => { cleanup(); vi.clearAllMocks(); });
it("shows the points warning and retries refresh without creating another Dude", async () => {
  create.mockResolvedValue({ dudeCode: "buddy" }); update.mockResolvedValue({ dudeCode: "buddy" });
  const onSaved = vi.fn().mockRejectedValueOnce(new Error("Refresh failed")).mockResolvedValueOnce(undefined);
  const close = vi.fn(); render(<DudeEditorDialog onClose={close} onSaved={onSaved} />);
  expect(screen.getByText("dude.costWarning")).toBeTruthy();
  expect(screen.queryByText("Agent group")).toBeNull();
  fireEvent.change(screen.getByLabelText("dude.code"), { target: { value: "buddy" } });
  fireEvent.change(screen.getByLabelText("dude.name"), { target: { value: "Buddy" } });
  fireEvent.click(screen.getByText("Choose loop"));
  fireEvent.click(screen.getByRole("button", { name: "dude.save" }));
  await screen.findByText("Refresh failed");
  fireEvent.click(screen.getByRole("button", { name: "dude.save" }));
  await waitFor(() => expect(close).toHaveBeenCalledOnce());
  expect(create).toHaveBeenCalledOnce(); expect(update).toHaveBeenCalledWith("buddy", expect.objectContaining({ name: "Buddy" }));
  expect(create.mock.calls[0][0]).not.toHaveProperty("groupName");
  expect(update.mock.calls[0][1]).not.toHaveProperty("groupName");
  expect(screen.queryByLabelText("pointsBudget")).toBeNull();
});

it("names each background switch once and keeps help outside its accessible name", () => {
  render(<DudeEditorDialog dude={{dudeCode:"memory",name:"Memory",rolePrompt:"",loopCode:"main",avatarId:"robot",candidateModelCodes:[],enabled:true,heartbeatMinutes:30,memoryAutoApprove:false,kind:"Memory",isMain:false,createdAt:0,updatedAt:0}} onClose={vi.fn()} onSaved={vi.fn()} />);
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
