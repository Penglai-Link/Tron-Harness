// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import { afterEach, describe, expect, it, vi } from "vitest";
import { i18n } from "@/i18n";
import type { Dude } from "../api/dudeApi";
import { DudeSwitcher } from "./DudeSwitcher";

vi.mock("@/domain/agent/components/AgentAvatarPicker", () => ({ AgentAvatar: () => null }));
afterEach(cleanup);

const dude = (dudeCode: string, name: string, overrides: Partial<Dude> = {}): Dude => ({
  dudeCode, name, rolePrompt: "", avatarId: "", candidateModelCodes: [],
  enabled: true, heartbeatMinutes: 30, memoryAutoApprove: false, kind: "Main", isMain: true,
  createdAt: 0, updatedAt: 0, ...overrides,
});

describe.each(["zh-CN", "en-US"])("Dude switcher in %s", locale => {
  it("reveals other Dudes on demand, keeps paused Dudes accessible, and exposes management", async () => {
    await i18n.changeLanguage(locale);
    const change = vi.fn();
    render(<MemoryRouter><DudeSwitcher value="main" onValueChange={change} dudes={[
      dude("main", "Main Dude"), dude("memory", "Memory Dude", { kind: "Memory", isMain: false, enabled: false }),
    ]} /></MemoryRouter>);
    expect(screen.queryByRole("menuitemradio")).toBeNull();
    expect(screen.queryByText("Memory Dude")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: /Main Dude/ }));
    expect((screen.getByRole("menuitemradio", { name: /Main Dude/, hidden: true })).getAttribute("aria-checked")).toBe("true");
    const memory = screen.getByRole("menuitemradio", { name: /Memory Dude/ });
    expect(memory.textContent).toContain(locale === "zh-CN" ? "已暂停" : "Paused");
    expect(screen.getByRole("menuitem", { name: locale === "zh-CN" ? "管理" : "Manage" }).getAttribute("href")).toBe("/settings/agents?tab=dudes");
    fireEvent.click(memory);
    expect(change).toHaveBeenCalledExactlyOnceWith("memory");
  });

  it("keeps management reachable when no Dude has loaded", async () => {
    await i18n.changeLanguage(locale);
    render(<MemoryRouter><DudeSwitcher value="" onValueChange={vi.fn()} dudes={[]} /></MemoryRouter>);
    fireEvent.click(screen.getByRole("button", { name: locale === "zh-CN" ? "切换 Dude" : "Switch Dude" }));
    expect(screen.getByRole("menuitem", { name: locale === "zh-CN" ? "管理" : "Manage", hidden: true })).toBeTruthy();
    expect(screen.queryByRole("menuitemradio")).toBeNull();
  });
});
