// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { MemoryRouter } from "react-router";
import { TraceExplorer } from "./TraceExplorer";
const state = vi.hoisted(() => ({ query: vi.fn(), error: null as Error | null, toast: vi.fn() }));
vi.mock("@/domain/settings/components/PlatformWorkspaceScope", () => ({ PlatformWorkspaceFilter: () => null }));
vi.mock("react-i18next", () => ({ useTranslation: () => ({ t: (key: string) => key }) }));
vi.mock("@/shared", () => ({ PageHeader: () => null, useLocaleFormatters: () => ({ currentLocale: "en-US", formatNumber: String, formatDateTime: String }) }));
vi.mock("@/components/ui/toast", () => ({ useToast: () => ({ error: state.toast }) }));
vi.mock("./CollectorConfigButton", () => ({ CollectorConfigButton: () => null }));
vi.mock("@/components/table-pagination", () => ({ TablePagination: ({ onPageChange }: { onPageChange: (value: number) => void }) => <button onClick={() => onPageChange(2)}>Next</button> }));
vi.mock("../hooks/useObservability", () => ({ useTraceList: (query: unknown) => { state.query(query); return { data: { records: [], total: 40 }, error: state.error, isSuccess: !state.error, isPending: false, isError: !!state.error, isFetching: false, refetch: vi.fn() }; } }));
afterEach(() => { cleanup(); state.error = null; vi.clearAllMocks(); });
it("combines sort columns in priority order and allows changing and removing each column", () => {
  render(<MemoryRouter initialEntries={["/?sessionCode=session"]}><TraceExplorer workspaceCode="workspace" /></MemoryRouter>);
  fireEvent.click(screen.getByRole("button", { name: "Next" }));
  fireEvent.click(screen.getByRole("button", { name: "duration" }));
  expect(state.query).toHaveBeenLastCalledWith(expect.objectContaining({ current: 1, workspaceCode: "workspace", sessionCode: "session", sorts: [{ field: "StartedAt", direction: "Desc" }, { field: "Duration", direction: "Desc" }] }));
  fireEvent.click(screen.getByRole("button", { name: "duration ↓" }));
  expect(screen.getByRole("columnheader", { name: "duration ↑" }).getAttribute("aria-sort")).toBe("ascending");
  fireEvent.click(screen.getByRole("button", { name: "Spans" }));
  fireEvent.click(screen.getByRole("button", { name: "duration ↑" }));
  expect(state.query).toHaveBeenLastCalledWith(expect.objectContaining({ sorts: [{ field: "StartedAt", direction: "Desc" }, { field: "Spans", direction: "Desc" }] }));
  expect(screen.getByRole("button", { name: "Spans ↓" })).toBeTruthy();
});

it("does not repeat the same refresh error until a successful response", () => {
  state.error = new Error("invalid query");
  const view = () => <MemoryRouter><TraceExplorer workspaceCode="workspace" /></MemoryRouter>;
  const rendered = render(view());
  expect(state.toast).toHaveBeenCalledTimes(1);
  state.error = new Error("invalid query");
  rendered.rerender(view());
  expect(state.toast).toHaveBeenCalledTimes(1);
  state.error = null; rendered.rerender(view());
  state.error = new Error("invalid query"); rendered.rerender(view());
  expect(state.toast).toHaveBeenCalledTimes(2);
});

it("restores, submits and clears the session ID filter while resetting pagination", () => {
  render(<MemoryRouter initialEntries={["/?category=Agent&sessionCode=original"]}><TraceExplorer workspaceCode="workspace" /></MemoryRouter>);
  const input = screen.getByRole<HTMLInputElement>("textbox", { name: "sessionId" });
  expect(input.value).toBe("original");
  fireEvent.click(screen.getByRole("button", { name: "Next" }));
  fireEvent.change(input, { target: { value: "  session-New  " } });
  fireEvent.click(screen.getByRole("button", { name: "search" }));
  expect(state.query).toHaveBeenLastCalledWith(expect.objectContaining({ sessionCode: "session-New", current: 1, category: "Agent" }));
  fireEvent.change(input, { target: { value: "" } });
  fireEvent.click(screen.getByRole("button", { name: "search" }));
  expect(state.query).toHaveBeenLastCalledWith(expect.objectContaining({ sessionCode: "", current: 1 }));
});
