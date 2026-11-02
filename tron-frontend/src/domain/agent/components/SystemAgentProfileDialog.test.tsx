// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeAll, expect, it, vi } from "vitest";
import { SystemAgentProfileDialog } from "./SystemAgentProfileDialog";
import type { PlatformSystemAgentRecord } from "../model";
import { pages } from "@/i18n/locales/en-US/pages";

vi.mock("react-i18next", () => ({ useTranslation: () => ({ t: (key: string) => {
  if (key.startsWith("pages:settingsAgents.profile.")) {
    return pages.settingsAgents.profile[key.slice("pages:settingsAgents.profile.".length) as keyof typeof pages.settingsAgents.profile];
  }
  if (key === "agentActivation.active") return pages.agentActivation.active;
  if (key === "agentActivation.disabled") return pages.agentActivation.disabled;
  return key;
} }) }));
const agents: PlatformSystemAgentRecord[] = [{ agentCode: "codex", name: "Codex", description: "CLI", source: "System Scan", status: "Enabled", enabled: true, available: true, lastScannedAt: null, models: [{ modelId: "fast-model", displayName: "Fast" }, { modelId: "deep-model", displayName: "Deep" }] }];
const toastError = vi.hoisted(() => vi.fn());
vi.mock("@/components/ui/toast", () => ({ useToast: () => ({ error: toastError }) }));
beforeAll(() => { window.PointerEvent ??= MouseEvent as typeof PointerEvent; });
afterEach(() => { cleanup(); toastError.mockClear(); });

function fill(code: string, model: string) {
  fireEvent.change(screen.getByRole("textbox", { name: "Name" }), { target: { value: "Codex Fast" } });
  fireEvent.change(screen.getByRole("textbox", { name: "Code" }), { target: { value: code } });
  fireEvent.change(screen.getByRole("combobox", { name: "Model" }), { target: { value: model } });
}
it("validates a distinct identity and saves an explicit model with its base Agent", async () => {
  const onSave = vi.fn().mockResolvedValue(undefined), onClose = vi.fn();
  render(<SystemAgentProfileDialog agents={agents} onSave={onSave} onClose={onClose} />);
  fill("bad code", "fast-model");
  expect(screen.getByRole<HTMLButtonElement>("button", { name: "Save" }).disabled).toBe(true);
  fireEvent.change(screen.getByRole("textbox", { name: "Code" }), { target: { value: "codex-fast" } });
  fireEvent.click(screen.getByRole("button", { name: "Save" }));
  await waitFor(() => expect(onSave).toHaveBeenCalledExactlyOnceWith({ agentCode: "codex-fast", name: "Codex Fast", description: "", baseAgentCode: "codex", modelId: "fast-model", reasoningEffort: null, fastMode: false, avatarId: "codex", enabled: true }));
  await waitFor(() => expect(onClose).toHaveBeenCalledOnce());
});
it("keeps the configured model draft after a failed save and permits retry", async () => {
  const onSave = vi.fn().mockRejectedValueOnce(new Error("Network unavailable")).mockResolvedValue(undefined), onClose = vi.fn();
  render(<SystemAgentProfileDialog agents={agents} onSave={onSave} onClose={onClose} />);
  fill("codex-deep", "manual-model");
  fireEvent.click(screen.getByRole("button", { name: "Save" }));
  await waitFor(() => expect(toastError).toHaveBeenCalledWith("Network unavailable"));
  expect(onClose).not.toHaveBeenCalled();
  expect(screen.getByRole<HTMLInputElement>("combobox", { name: "Model" }).value).toBe("manual-model");
  fireEvent.click(screen.getByRole("button", { name: "Save" }));
  await waitFor(() => expect(onClose).toHaveBeenCalledOnce());
  expect(onSave).toHaveBeenCalledTimes(2);
});
it("edits the logical Agent without changing its identity or base and cancels without writes", () => {
  const record = { ...agents[0], agentCode: "codex-fast", baseAgentCode: "codex", modelId: "fast-model" };
  const onSave = vi.fn(), onClose = vi.fn();
  render(<SystemAgentProfileDialog agents={agents} record={record} onSave={onSave} onClose={onClose} />);
  expect(screen.queryByRole("textbox", { name: "Code" })).toBeNull();
  expect(screen.getByRole<HTMLInputElement>("combobox", { name: "Model" }).value).toBe("fast-model");
  fireEvent.change(screen.getByRole("combobox", { name: "Model" }), { target: { value: "deep-model" } });
  fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
  expect(onSave).not.toHaveBeenCalled();
  expect(onClose).toHaveBeenCalledOnce();
});
it("requires a qualified model ID for OpenCode", () => {
  render(<SystemAgentProfileDialog agents={[{ ...agents[0], agentCode: "opencode", name: "OpenCode" }]} onSave={vi.fn()} onClose={vi.fn()} />);
  fill("opencode-fast", "fast-model");
  expect(screen.getByRole<HTMLButtonElement>("button", { name: "Save" }).disabled).toBe(true);
  expect(screen.getByText("OpenCode requires a provider/model ID.")).toBeTruthy();
  fireEvent.change(screen.getByRole("combobox", { name: "Model" }), { target: { value: "provider/fast-model" } });
  expect(screen.getByRole<HTMLButtonElement>("button", { name: "Save" }).disabled).toBe(false);
});

it("saves the disabled state from the edit header", async () => {
  const onSave = vi.fn().mockResolvedValue(undefined);
  render(<SystemAgentProfileDialog agents={agents} record={{ ...agents[0], agentCode: "codex-fast", baseAgentCode: "codex", modelId: "fast-model" }} onSave={onSave} onClose={vi.fn()} />);
  const activation = screen.getByRole("switch", { name: /^Active\b/ });
  const fastMode = screen.getByRole("switch", { name: "Fast mode" });
  expect(activation.getAttribute("aria-checked")).toBe("true");
  expect(fastMode.getAttribute("aria-checked")).toBe("false");
  fireEvent.click(activation);
  expect(activation.getAttribute("aria-checked")).toBe("false");
  expect(fastMode.getAttribute("aria-checked")).toBe("false");
  fireEvent.click(screen.getByRole("button", { name: "Save" }));
  await waitFor(() => expect(onSave).toHaveBeenCalledExactlyOnceWith({
    agentCode: "codex-fast", name: "Codex", description: "CLI", baseAgentCode: "codex",
    modelId: "fast-model", reasoningEffort: null, fastMode: false, avatarId: "codex", enabled: false,
  }));
});

it("requires a qualified dsh model and saves the selected native route", async () => {
  const onSave=vi.fn().mockResolvedValue(undefined);
  render(<SystemAgentProfileDialog agents={[{...agents[0],agentCode:"dsh",name:"DeepSeek Harness",models:[]}]} onSave={onSave} onClose={vi.fn()} />);
  fill("dsh-custom", "model");
  expect(screen.getByRole<HTMLButtonElement>("button",{name:"Save"}).disabled).toBe(true);
  fireEvent.change(screen.getByRole("combobox",{name:"Model"}),{target:{value:"deepseek-account/model/submodel"}});
  fireEvent.click(screen.getByRole("button",{name:"Save"}));
  await waitFor(()=>expect(onSave).toHaveBeenCalledWith(expect.objectContaining({baseAgentCode:"dsh",modelId:"deepseek-account/model/submodel",reasoningEffort:null,fastMode:false})));
});

for (const avatarId of ["codex", "deepseek", "opencode"]) {
  it(`previews, saves and restores the ${avatarId} system avatar`, async () => {
    const onSave = vi.fn().mockResolvedValue(undefined);
    const view = render(<SystemAgentProfileDialog agents={agents} onSave={onSave} onClose={vi.fn()} />);
    fill("custom-avatar", "fast-model");
    const label = `workspaceAgents.internal.avatar.${avatarId}`;
    fireEvent.click(screen.getByRole("button", { name: label }));
    expect(screen.getByRole("img", { name: label }).getAttribute("src")).toBe(`/agent-brands/${avatarId}-pixel.png`);
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(onSave).toHaveBeenCalledWith(expect.objectContaining({ avatarId })));
    view.unmount();
    render(<SystemAgentProfileDialog agents={agents} record={{ ...agents[0], agentCode: "custom-avatar", baseAgentCode: "codex", modelId: "fast-model", avatarId }} onSave={onSave} onClose={vi.fn()} />);
    expect(screen.getByRole("button", { name: label }).getAttribute("aria-pressed")).toBe("true");
    expect(screen.getByRole("img", { name: label }).getAttribute("src")).toBe(`/agent-brands/${avatarId}-pixel.png`);
  });
}
