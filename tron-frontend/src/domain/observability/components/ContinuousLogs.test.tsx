// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { ContinuousLogs } from "./ContinuousLogs";

const state = vi.hoisted(() => ({ hook: vi.fn(), older: vi.fn().mockResolvedValue(undefined) }));
vi.mock("react-i18next", () => ({ useTranslation: () => ({ t: (key: string) => key }) }));
vi.mock("@/shared", () => ({ useLocaleFormatters: () => ({ formatDateTime: String, formatNumber: String }) }));
vi.mock("./LogMessage", () => ({ LogMessage: ({ message }: { message: string }) => <p>{message}</p> }));
vi.mock("./TraceIdentifier", () => ({ TraceIdentifier: ({ value, onClick }: { value: string; onClick: () => void }) => <button onClick={onClick}>{value}</button> }));
vi.mock("../hooks/useObservability", () => ({ useContinuousLogs: (filters: unknown, live: boolean) => {
  state.hook(filters, live);
  const row = (id: string, message: string) => ({ logId: id, message, timestamp: 1, level: "Info", target: "app", traceId: "a".repeat(32), spanId: null, attributes: { detail: "full field" } });
  return { data: { pages: [{ records: [row("3", "newest"), row("2", "middle")] }, { records: [row("2", "middle"), row("1", "oldest")] }] }, hasNextPage: true, isFetching: false, isFetchingNextPage: false, isError: false, isPending: false, fetchNextPage: state.older, refetch: vi.fn() };
} }));
afterEach(() => { cleanup(); vi.clearAllMocks(); });
const filters = { date: "2026-10-11", timeZone: "Asia/Shanghai", level: "" as const, keyword: "", traceId: "", spanId: "" };

it("shows full records in chronological order, deduplicates boundaries, and filters IDs", () => {
  const onFilter = vi.fn();
  const { container } = render(<ContinuousLogs filters={filters} live isToday onFilter={onFilter} />);
  expect([...container.querySelectorAll("[data-log-id]")].map(row => row.getAttribute("data-log-id"))).toEqual(["1", "2", "3"]);
  expect(screen.getAllByText("middle")).toHaveLength(1);
  expect(screen.getAllByText("detail=full field")).toHaveLength(3);
  fireEvent.click(screen.getAllByRole("button", { name: "a".repeat(32) })[0]);
  expect(onFilter).toHaveBeenCalledWith("traceId", "a".repeat(32));
});

it("pauses automatic updates while loading history, resumes following, and never polls history dates", () => {
  const { rerender } = render(<ContinuousLogs filters={filters} live isToday onFilter={vi.fn()} />);
  expect(state.hook).toHaveBeenLastCalledWith(filters, true);
  expect(screen.queryByRole("button", { name: "loadEarlierLogs" })).toBeNull();
  const viewport = screen.getByRole("region", { name: "continuousLogs" });
  Object.defineProperties(viewport, { scrollHeight: { configurable: true, value: 1000 }, clientHeight: { configurable: true, value: 300 }, scrollTop: { configurable: true, writable: true, value: 20 } });
  fireEvent.scroll(viewport);
  fireEvent.scroll(viewport);
  expect(state.older).toHaveBeenCalledOnce();
  expect(state.hook).toHaveBeenLastCalledWith(filters, false);
  fireEvent.click(screen.getByRole("button", { name: "followLatestLogs" }));
  expect(state.hook).toHaveBeenLastCalledWith(filters, true);
  rerender(<ContinuousLogs filters={filters} live isToday={false} onFilter={vi.fn()} />);
  expect(state.hook).toHaveBeenLastCalledWith(filters, false);
});
