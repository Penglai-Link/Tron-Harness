// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { SystemAgentProfileDialog } from "./SystemAgentProfileDialog";
import type { PlatformSystemAgentRecord } from "../model";
import { pages } from "@/i18n/locales/en-US/pages";

vi.mock("react-i18next", () => ({ useTranslation: () => ({ t: (key: string) => key.startsWith("pages:settingsAgents.profile.") ? pages.settingsAgents.profile[key.slice("pages:settingsAgents.profile.".length) as keyof typeof pages.settingsAgents.profile] : "Close" }) }));
const agents: PlatformSystemAgentRecord[] = [{ agentCode: "codex", name: "Codex", description: "CLI", source: "System Scan", status: "Enabled", enabled: true, available: true, lastScannedAt: null, models: [{ modelId: "fast-model", displayName: "Fast" }, { modelId: "deep-model", displayName: "Deep" }] }];
afterEach(cleanup);

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
  await waitFor(() => expect(onSave).toHaveBeenCalledExactlyOnceWith({ agentCode: "codex-fast", name: "Codex Fast", baseAgentCode: "codex", modelId: "fast-model" }));
  await waitFor(() => expect(onClose).toHaveBeenCalledOnce());
});
it("keeps the configured model draft after a failed save and permits retry", async () => {
  const onSave = vi.fn().mockRejectedValueOnce(new Error("Network unavailable")).mockResolvedValue(undefined), onClose = vi.fn();
  render(<SystemAgentProfileDialog agents={agents} onSave={onSave} onClose={onClose} />);
  fill("codex-deep", "manual-model");
  fireEvent.click(screen.getByRole("button", { name: "Save" }));
  expect(await screen.findByRole("alert")).toHaveProperty("textContent", "Network unavailable");
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
