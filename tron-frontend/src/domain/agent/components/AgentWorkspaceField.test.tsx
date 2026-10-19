// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { AgentWorkspaceField } from "./AgentWorkspaceField";

const fixture = vi.hoisted(() => ({ fetch: vi.fn(), refetch: vi.fn() }));
vi.mock("../hooks/agentWorkspaceGrants", () => ({ loadAgentWorkspaceGrants: fixture.fetch }));
vi.mock("@/store/authStore", () => ({ useAuthStore: (select: (state: unknown) => unknown) => select({ session: { user: { userCode: "user-1" } } }) }));
vi.mock("@/domain/settings/components/PlatformWorkspaceScope", () => ({ usePlatformWorkspaces: () => ({ data: [{ workspaceCode: "A", label: "Space A", status: "Active" }, { workspaceCode: "B", label: "Space B", status: "Active" }], isPending: false, error: null, refetch: fixture.refetch }) }));
vi.mock("react-i18next", () => ({ useTranslation: () => ({ t: (key: string, options?: { count?: number }) => options?.count !== undefined ? `${key} (${options.count})` : key }) }));

function renderField(code = "reviewer") {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const onSelectionChange = vi.fn();
  const result = render(<QueryClientProvider client={client}><AgentWorkspaceField code={code} name="Reviewer" disabled={false} onSelectionChange={onSelectionChange} /></QueryClientProvider>);
  return { ...result, onSelectionChange };
}

beforeEach(() => { vi.clearAllMocks(); fixture.fetch.mockResolvedValue({ workspaceCodes: ["A"] }); });
afterEach(cleanup);

it("shows existing workspace names and confirms a local selection", async () => {
  const { onSelectionChange } = renderField();
  await screen.findByText("Space A");
  fireEvent.click(screen.getByRole("button", { name: "platformData.chooseWorkspaces (1)" }));
  fireEvent.click(screen.getByRole("checkbox", { name: "Space B" }));
  expect(onSelectionChange).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: "platformData.confirm" }));
  await waitFor(() => expect(onSelectionChange).toHaveBeenCalledWith(["A", "B"]));
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
});

it("blocks selection after a failed load and lets the user retry", async () => {
  fixture.fetch.mockRejectedValueOnce(new Error("Grants unavailable"));
  renderField();
  await screen.findByRole("alert");
  expect(screen.getByRole<HTMLButtonElement>("button", { name: "platformData.chooseWorkspaces (0)" }).disabled).toBe(true);
  fireEvent.click(screen.getByRole("button", { name: "platformData.retry" }));
  await screen.findByText("Space A");
  expect(fixture.fetch).toHaveBeenCalledTimes(2);
  expect(screen.getByRole<HTMLButtonElement>("button", { name: "platformData.chooseWorkspaces (1)" }).disabled).toBe(false);
});

it("keeps the main agent available everywhere without an assignment request", () => {
  renderField("main-agent");
  expect(screen.getByText("platformData.builtinAgent")).toBeTruthy();
  expect(screen.queryByRole("button")).toBeNull();
  expect(fixture.fetch).not.toHaveBeenCalled();
});

it("previews the default active-workspace grants before an Agent exists", () => {
  renderField("");
  expect(screen.getByRole("button", { name: "platformData.chooseWorkspaces (2)" })).toBeTruthy();
  expect(screen.getByText("Space A · Space B")).toBeTruthy();
  expect(fixture.fetch).not.toHaveBeenCalled();
});
