// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, expect, it, vi } from "vitest";
import { AgentWorkspaceAssignment } from "./AgentWorkspaceAssignment";
const { request, changed } = vi.hoisted(() => ({ request: vi.fn(), changed: vi.fn() }));
vi.mock("@/shared/api/httpClient", () => ({ requestApi: request }));
vi.mock("../model", () => ({ notifyInternalAgentRegistryChanged: changed }));
vi.mock("@/domain/admin/api", () => ({ fetchWorkspaceOptions: vi.fn(async () => [
  { workspaceCode: "alpha", name: "Alpha" }, { workspaceCode: "beta", name: "Beta" },
]) }));
vi.mock("@/store/authStore", () => ({ useAuthStore: (select: (state: unknown) => unknown) => select({ session: { user: { userCode: "owner" } } }) }));
vi.mock("react-i18next", () => ({ useTranslation: () => ({ t: (key: string) => key }) }));
function mount(code = "expert") {
  request.mockImplementation(async (endpoint: string) => endpoint.startsWith("GET") ? { workspaceCodes: ["alpha"] } : {});
  return render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
    <AgentWorkspaceAssignment code={code} name="Expert" />
  </QueryClientProvider>);
}
afterEach(() => { cleanup(); vi.clearAllMocks(); });
it("loads saved assignments and submits only the selected spaces", async () => {
  mount();
  expect(request).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: "platformData.assign" }));
  const alpha = await screen.findByLabelText("Alpha") as HTMLInputElement;
  expect(alpha.checked).toBe(true);
  fireEvent.click(alpha);
  fireEvent.click(screen.getByLabelText("Beta"));
  fireEvent.click(screen.getByRole("button", { name: "platformData.save" }));
  await waitFor(() => expect(changed).toHaveBeenCalledOnce());
  expect(request).toHaveBeenCalledWith("PUT /api/v1/agents/registry/{code}/workspace-grants", {
    pathParams: { code: "expert" }, body: { workspaceCodes: ["beta"] },
  });
  expect(screen.queryByRole("dialog")).toBeNull();
});
it("keeps the built-in Main Agent available without an editable assignment", () => {
  mount("main-agent");
  expect(screen.getByText("platformData.builtinAgent")).toBeTruthy();
  expect(screen.queryByRole("button")).toBeNull();
  expect(request).not.toHaveBeenCalled();
});
