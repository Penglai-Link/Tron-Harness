// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { Link, MemoryRouter, useLocation } from "react-router";
import { afterEach, expect, it, vi } from "vitest";
import { PlatformWorkspaceScope, usePlatformDataHref, usePlatformDataNavigate } from "./PlatformWorkspaceScope";
import { useWorkspaceSpaces } from "@/shared/hooks/useWorkspaceSpaces";
import { localizeWorkspaceOptions } from "../constants/workspacePresentation";
import type { WorkspaceOption } from "@/shared/types/WorkspaceSpace";

vi.mock("@/domain/admin/api", () => ({ fetchWorkspaceOptions: vi.fn(async () => [
  { workspaceCode: "alpha", name: "Alpha", label: "Alpha" },
  { workspaceCode: "beta", name: "Beta", label: "Beta" },
]) }));
vi.mock("@/store/authStore", () => ({ useAuthStore: (select: (state: unknown) => unknown) => select({ session: { user: { userCode: "owner" } } }) }));
vi.mock("react-i18next", () => ({ useTranslation: () => ({ t: (key: string) => key }) }));

function Probe() {
  const spaces = useWorkspaceSpaces();
  const href = usePlatformDataHref();
  const navigate = usePlatformDataNavigate();
  const location = useLocation();
  return <>
    <output>scope:{spaces.selectedPrimarySpace?.workspaceCode}</output>
    <output>{location.pathname}{location.search}</output>
    <Link to={href("/memory")}>Memory home</Link>
    <button onClick={() => navigate("/rag?section=retrieval")}>RAG workbench</button>
  </>;
}
function mount(path: string, renderWithoutWorkspace = false, allowAll = false) {
  return render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
    <MemoryRouter initialEntries={[path]}><PlatformWorkspaceScope allowAll={allowAll} renderWithoutWorkspace={renderWithoutWorkspace}><Probe /></PlatformWorkspaceScope></MemoryRouter>
  </QueryClientProvider>);
}
afterEach(cleanup);
it("localizes the seeded workspace and its paths while preserving custom names and cached records", () => {
  const defaults: WorkspaceOption = { workspaceCode: "default", name: "默认工作空间", label: "默认工作空间", description: "PersonAI 档案的默认工作空间", parentWorkspaceCode: null, depth: 0, status: "Active" };
  const child = { ...defaults, workspaceCode: "child", name: "研发", label: "默认工作空间 / 研发", parentWorkspaceCode: "default", depth: 1 };
  const custom = { ...defaults, workspaceCode: "custom", description: null };
  const renamed = { ...defaults, workspaceCode: "renamed", name: "我的空间", label: "我的空间" };
  const options = [defaults, child, custom, renamed];
  const english = localizeWorkspaceOptions(options, "Default workspace");
  expect(english.map(({ name, label }) => ({ name, label }))).toEqual([
    { name: "Default workspace", label: "Default workspace" },
    { name: "研发", label: "Default workspace / 研发" },
    { name: "默认工作空间", label: "默认工作空间" },
    { name: "我的空间", label: "我的空间" },
  ]);
  expect(defaults.name).toBe("默认工作空间");
  expect(child.label).toBe("默认工作空间 / 研发");
  expect(localizeWorkspaceOptions(options, "默认工作空间")).toEqual(options);
});
it("preserves the explicit space across both breadcrumbs and workbench navigation", async () => {
  mount("/settings/memory/item?workspaceCode=beta");
  await screen.findByText("scope:beta");
  fireEvent.click(screen.getByText("Memory home"));
  expect(screen.getByText("/settings/memory?workspaceCode=beta")).toBeTruthy();
  fireEvent.click(screen.getByText("RAG workbench"));
  expect(screen.getByText("/settings/rag?section=retrieval&workspaceCode=beta")).toBeTruthy();
});
it("carries the initially selected space into navigation before it has a URL parameter", async () => {
  mount("/settings/memory");
  await screen.findByText("scope:alpha");
  fireEvent.click(screen.getByText("RAG workbench"));
  expect(screen.getByText("/settings/rag?section=retrieval&workspaceCode=alpha")).toBeTruthy();
});
it("does not silently fall back to another space for a stale explicit code", async () => {
  mount("/settings/memory?workspaceCode=deleted");
  await screen.findByRole("status");
  expect(screen.queryByText("scope:alpha")).toBeNull();
  expect(screen.queryByText("Memory home")).toBeNull();
});
it("renders the empty page with an empty scope rather than another space", async () => {
  mount("/settings/memory?workspaceCode=deleted", true);
  await screen.findByText("scope:");
  expect(screen.queryByText("scope:alpha")).toBeNull();
  expect(screen.getByText("Memory home")).toBeTruthy();
  expect(screen.queryByText("platformData.selectWorkspace", { selector: "p" })).toBeNull();
});

it("defaults observability to all workspaces independently of the available workspace list", async () => {
  mount("/settings/observability/dashboard", false, true);
  await screen.findByText("scope:");
  expect(screen.queryByText("scope:alpha")).toBeNull();
  expect(screen.getByText("observability:allWorkspaces")).toBeTruthy();
});
it("applies an explicit observability workspace without falling back to the first workspace", async () => {
  mount("/settings/observability/traces?workspaceCode=beta", false, true);
  await screen.findByText("scope:beta");
  expect(screen.queryByText("scope:alpha")).toBeNull();
});
