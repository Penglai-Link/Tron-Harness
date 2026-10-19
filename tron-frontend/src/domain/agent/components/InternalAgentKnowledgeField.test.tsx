// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const fixture = vi.hoisted(() => ({ list: vi.fn(), catalog: vi.fn(), save: vi.fn(), success: vi.fn(), error: vi.fn() }));
vi.mock("@/domain/knowledge-base/api/knowledgeBaseApi", () => ({ listKnowledgeLibraries: fixture.catalog, listAgentKnowledgeLibraries: fixture.list, setAgentKnowledgeLibraries: fixture.save }));
vi.mock("@/domain/knowledge-base/hooks/useKnowledgeLibraries", () => ({ useKnowledgeUser: () => "USER", useKnowledgeViewActive: () => ({ current: true }) }));
vi.mock("@/components/ui/toast", () => ({ useToast: () => ({ success: fixture.success, error: fixture.error }) }));
vi.mock("react-i18next", () => ({ useTranslation: () => ({ t: (key: string, options?: { count?: number }) => {
  const name = key.split(".").at(-1)!;
  return name === "choose" ? `Choose knowledge bases (${options?.count})` : name;
} }) }));
import { InternalAgentKnowledgeField } from "./InternalAgentKnowledgeField";

beforeEach(() => {
  vi.clearAllMocks();
  vi.stubGlobal("PointerEvent", MouseEvent);
  fixture.list.mockResolvedValue([
    { id: "a", name: "Engineering", description: "Build systems", documentCount: 2, granted: true },
    { id: "b", name: "Product", description: "User scenarios", documentCount: 1, granted: false },
  ]);
  fixture.save.mockResolvedValue({});
  fixture.catalog.mockResolvedValue([{ id: "a", name: "Engineering", description: "Build systems", documentCount: 2 }]);
});
afterEach(() => { cleanup(); vi.unstubAllGlobals(); });
function setup(onSelectionChange?: (ids: string[]) => void, agentCode = "main-agent") {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
  const invalidate = vi.spyOn(client, "invalidateQueries");
  render(<QueryClientProvider client={client}><InternalAgentKnowledgeField agentCode={agentCode} agentName="Main" disabled={false} onSelectionChange={onSelectionChange} /></QueryClientProvider>);
  return { client, invalidate };
}
async function open() {
  const button = await screen.findByRole("button", { name: "Choose knowledge bases (1)" });
  fireEvent.click(button);
  return screen.findByRole("dialog");
}
describe("internal agent knowledge assignments", () => {
  it("shows existing grants and discards changes on cancel", async () => {
    setup();
    const dialog = await open();
    expect(fixture.list).toHaveBeenCalledWith("main-agent", expect.any(AbortSignal));
    expect((within(dialog).getByRole("switch", { name: "Engineering" }) as HTMLInputElement).getAttribute("aria-checked")).toBe("true");
    fireEvent.click(within(dialog).getByRole("switch", { name: "Engineering" }));
    fireEvent.click(within(dialog).getByRole("button", { name: "cancel" }));
    expect(fixture.save).not.toHaveBeenCalled();
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(screen.getByText("Engineering")).toBeTruthy();
  });
  it("preserves hidden selections during search and refreshes both authorization views", async () => {
    const { invalidate } = setup();
    const dialog = await open();
    fireEvent.change(within(dialog).getByRole("searchbox", { name: "search" }), { target: { value: "User scenarios" } });
    await waitFor(() => expect(within(dialog).queryByRole("switch", { name: "Engineering" })).toBeNull());
    fireEvent.click(within(dialog).getByRole("switch", { name: "Product" }));
    fixture.list.mockResolvedValue([
      { id: "a", name: "Engineering", description: "Build systems", documentCount: 2, granted: true },
      { id: "b", name: "Product", description: "User scenarios", documentCount: 1, granted: true },
    ]);
    fireEvent.click(within(dialog).getByRole("button", { name: "confirm" }));
    await waitFor(() => expect(fixture.save).toHaveBeenCalledExactlyOnceWith("main-agent", ["a", "b"]));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(screen.getByRole("button", { name: "Choose knowledge bases (2)" })).toBeTruthy();
    expect(invalidate).toHaveBeenCalledWith({ queryKey: ["knowledge-base"] });
    expect(invalidate).toHaveBeenCalledWith({ queryKey: ["knowledge-bases"] });
    expect(invalidate).toHaveBeenCalledWith({ queryKey: ["knowledge-agent-libraries"] });
  });
  it("keeps the dialog and original grants on a failed save", async () => {
    setup(); fixture.save.mockRejectedValue(new Error("Storage unavailable"));
    const dialog = await open();
    fireEvent.click(within(dialog).getByRole("switch", { name: "Engineering" }));
    fireEvent.click(within(dialog).getByRole("button", { name: "confirm" }));
    await waitFor(() => expect(fixture.error).toHaveBeenCalledWith("Storage unavailable"));
    expect(fixture.save).toHaveBeenCalledWith("main-agent", []);
    expect(screen.getByRole("dialog")).toBeTruthy();
    fireEvent.click(within(dialog).getByRole("button", { name: "cancel" }));
    expect(screen.getByRole("button", { name: "Choose knowledge bases (1)" })).toBeTruthy();
  });
  it("returns selection to the editor without writing grants", async () => {
    const onSelectionChange = vi.fn();
    setup(onSelectionChange);
    const dialog = await open();
    fireEvent.click(within(dialog).getByRole("switch", { name: "Product" }));
    fireEvent.click(within(dialog).getByRole("button", { name: "confirm" }));
    expect(onSelectionChange).toHaveBeenCalledWith(["a", "b"]);
    expect(fixture.save).not.toHaveBeenCalled();
    expect(screen.queryByRole("dialog")).toBeNull();
  });
  it("refreshes grants when knowledge management invalidates the shared query", async () => {
    const { client } = setup();
    await screen.findByRole("button", { name: "Choose knowledge bases (1)" });
    fixture.list.mockResolvedValue([]);
    await client.invalidateQueries({ queryKey: ["knowledge-agent-libraries"] });
    await screen.findByRole("button", { name: "Choose knowledge bases (0)" });
    expect(screen.getByText("none")).toBeTruthy();
  });
});

it("stages a creation-time library selection without querying or writing an unknown Agent", async () => {
  const choose = vi.fn();
  setup(choose, "");
  const button = await screen.findByRole<HTMLButtonElement>("button", { name: "Choose knowledge bases (0)" });
  await waitFor(() => expect(button.disabled).toBe(false));
  fireEvent.click(button);
  const dialog = await screen.findByRole("dialog");
  fireEvent.click(within(dialog).getByRole("switch", { name: "Engineering" }));
  fireEvent.click(within(dialog).getByRole("button", { name: "confirm" }));
  expect(choose).toHaveBeenCalledWith(["a"]);
  expect(fixture.catalog).toHaveBeenCalledWith(expect.any(AbortSignal));
  expect(fixture.list).not.toHaveBeenCalled();
  expect(fixture.save).not.toHaveBeenCalled();
});
