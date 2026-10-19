// @vitest-environment jsdom
import { useState } from "react";
import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
const fixture = vi.hoisted(() => ({ list: vi.fn(), catalog: vi.fn(), save: vi.fn() }));
vi.mock("@/domain/skill/api", () => ({
  fetchAgentSkills: fixture.list,
  fetchRegistrySkills: fixture.catalog,
  CAPABILITY_RESOURCES_CHANGED: "resource-change",
}));
vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: { count?: number }) =>
      key.endsWith("choose")
        ? `Choose skills (${options?.count})`
        : key.split(".").at(-1),
  }),
}));
import { InternalAgentSkillField } from "./InternalAgentSkillField";
const active = {
  skillName: "active",
  sourceRef: "pack",
  description: "Allowed",
  available: true,
  assigned: true,
};
const disabled = {
  skillName: "disabled",
  sourceRef: "pack",
  available: false,
  assigned: true,
  disabledReason: "Capability pack pack is disabled",
};
beforeEach(() => {
  vi.clearAllMocks();
  vi.stubGlobal("PointerEvent", MouseEvent);
  fixture.catalog.mockResolvedValue([active, disabled]);
  fixture.list.mockResolvedValue([
    active,
    disabled,
    { ...active, skillName: "optional", assigned: false },
  ]);
});
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});
function setup(initialNames?: string[], agentCode = "main-agent") {
  const change = vi.fn();
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  function Editor() {
    const [names, setNames] = useState(initialNames);
    return (
      <InternalAgentSkillField
        agentCode={agentCode}
        disabled={false}
        selectedNames={names}
        onSelectionChange={(next) => {
          change(next);
          setNames(next);
        }}
      />
    );
  }
  render(
    <QueryClientProvider client={client}>
      <Editor />
    </QueryClientProvider>
  );
  return { change, client };
}
async function open() {
  fireEvent.click(
    await screen.findByRole("button", { name: "Choose skills (1)" })
  );
  return screen.findByRole("dialog");
}
describe("Agent skill draft", () => {
  it("preserves confirmed outer draft additions and removals when another resource changes", async () => {
    const { change } = setup(["optional"]);
    await screen.findByRole("button", { name: "Choose skills (1)" });
    fixture.list.mockResolvedValue([
      active,
      { ...active, skillName: "optional", assigned: false },
    ]);
    window.dispatchEvent(new Event("resource-change"));
    await waitFor(() => expect(fixture.list).toHaveBeenCalledTimes(2));
    const dialog = await open();
    expect(
      within(dialog).getByRole<HTMLInputElement>("checkbox", { name: "active" })
        .checked
    ).toBe(false);
    expect(
      within(dialog).getByRole<HTMLInputElement>("checkbox", {
        name: "optional",
      }).checked
    ).toBe(true);
    expect(change).not.toHaveBeenCalled();
  });
  it("cancel discards selection without saving", async () => {
    const { change } = setup();
    const dialog = await open();
    fireEvent.click(within(dialog).getByRole("checkbox", { name: "active" }));
    fireEvent.click(within(dialog).getByRole("button", { name: "cancel" }));
    expect(change).not.toHaveBeenCalled();
    expect(
      screen.getByRole("button", { name: "Choose skills (1)" })
    ).toBeTruthy();
  });
  it("all skips disabled resources and confirmation only updates the editor draft", async () => {
    const { change } = setup();
    const dialog = await open();
    const blocked = within(dialog).getByRole("checkbox", {
      name: "disabled",
    }) as HTMLInputElement;
    expect(blocked.disabled).toBe(true);
    expect(blocked.checked).toBe(false);
    fireEvent.click(within(dialog).getByRole("button", { name: "selectAll" }));
    fireEvent.click(within(dialog).getByRole("button", { name: "confirm" }));
    expect(change).toHaveBeenCalledWith(["active", "optional"]);
    expect(fixture.save).not.toHaveBeenCalled();
  });
  it("keeps selection, search and focus on an ordinary successful query refresh", async () => {
    const { change, client } = setup();
    const dialog = await open();
    const optional = within(dialog).getByRole<HTMLInputElement>("checkbox", {
      name: "optional",
    });
    fireEvent.click(optional);
    fireEvent.click(within(dialog).getByRole("checkbox", { name: "active" }));
    const search = within(dialog).getByRole<HTMLInputElement>("searchbox", {
      name: "search",
    });
    fireEvent.change(search, { target: { value: "optional" } });
    search.focus();
    await act(async () => {
      await client.invalidateQueries({ queryKey: ["agent-skills"] });
    });
    expect(screen.getByRole("dialog")).toBe(dialog);
    expect(document.activeElement).toBe(search);
    expect(search.value).toBe("optional");
    expect(optional.checked).toBe(true);
    fireEvent.click(within(dialog).getByRole("button", { name: "confirm" }));
    expect(change).toHaveBeenCalledWith(["optional"]);
  });
  it("prunes revoked choices without discarding unaffected local selections", async () => {
    const { change } = setup();
    const dialog = await open();
    fireEvent.click(within(dialog).getByRole("checkbox", { name: "optional" }));
    fixture.list.mockResolvedValue([
      { ...active, available: false, assigned: false },
      { ...active, skillName: "optional", assigned: false },
    ]);
    window.dispatchEvent(new Event("resource-change"));
    await waitFor(() =>
      expect(
        screen.getByRole<HTMLInputElement>("checkbox", { name: "active" })
          .disabled
      ).toBe(true)
    );
    expect(
      screen.getByRole<HTMLInputElement>("checkbox", { name: "optional" })
        .checked
    ).toBe(true);
    fireEvent.click(within(dialog).getByRole("button", { name: "confirm" }));
    expect(change).toHaveBeenCalledWith(["optional"]);
  });
  it("applies re-enabled default grants without resetting unrelated pending choices", async () => {
    setup();
    const dialog = await open();
    fireEvent.click(within(dialog).getByRole("checkbox", { name: "optional" }));
    fixture.list.mockResolvedValue([
      { ...active, available: false, assigned: false },
      { ...active, skillName: "optional", assigned: false },
    ]);
    window.dispatchEvent(new Event("resource-change"));
    await waitFor(() =>
      expect(
        within(dialog).getByRole<HTMLInputElement>("checkbox", {
          name: "active",
        }).disabled
      ).toBe(true)
    );
    fixture.list.mockResolvedValue([
      active,
      { ...active, skillName: "optional", assigned: false },
    ]);
    window.dispatchEvent(new Event("resource-change"));
    await waitFor(() =>
      expect(
        within(dialog).getByRole<HTMLInputElement>("checkbox", {
          name: "active",
        }).checked
      ).toBe(true)
    );
    expect(
      within(dialog).getByRole<HTMLInputElement>("checkbox", {
        name: "optional",
      }).checked
    ).toBe(true);
  });
  it("refresh invalidates an open draft and removed resources disappear", async () => {
    const { change } = setup();
    await open();
    fixture.list.mockResolvedValue([
      {
        ...active,
        available: false,
        assigned: false,
        disabledReason: "Capability pack pack is disabled",
      },
    ]);
    window.dispatchEvent(new Event("resource-change"));
    await waitFor(() =>
      expect(
        (screen.getByRole("checkbox", { name: "active" }) as HTMLInputElement)
          .disabled
      ).toBe(true)
    );
    expect(screen.queryByRole("checkbox", { name: "optional" })).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "confirm" }));
    expect(change).toHaveBeenCalledWith([]);
  });
});

it("uses the skill catalog for a creation draft without inheriting another Agent's assignments", async () => {
  setup(undefined, "");
  const button = await screen.findByRole<HTMLButtonElement>("button", { name: "Choose skills (0)" });
  await waitFor(() => expect(button.disabled).toBe(false));
  fireEvent.click(button);
  const dialog = await screen.findByRole("dialog");
  expect((within(dialog).getByRole("checkbox", { name: "active" }) as HTMLInputElement).checked).toBe(false);
  expect((within(dialog).getByRole("checkbox", { name: "disabled" }) as HTMLInputElement).disabled).toBe(true);
  expect(fixture.catalog).toHaveBeenCalled();
  expect(fixture.list).not.toHaveBeenCalled();
});

it("previews automatic capability-pack skill grants in the creation draft", async () => {
  fixture.catalog.mockResolvedValue([{ ...active, providerPackId: "tron.harness", assigned: false }]);
  setup(undefined, "");
  const button = await screen.findByRole<HTMLButtonElement>("button", { name: "Choose skills (1)" });
  await waitFor(() => expect(button.disabled).toBe(false));
  fireEvent.click(button);
  expect((within(await screen.findByRole("dialog")).getByRole("checkbox", { name: "active" }) as HTMLInputElement).checked).toBe(true);
  expect(fixture.list).not.toHaveBeenCalled();
});
