// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import type { ReactNode } from "react";
import { observability } from "@/i18n/locales/zh-CN/observability";
import { LogsPage } from "./LogsPage";
import { useTimeZoneStore } from "@/shared/lib/userTimeZone";
const state = vi.hoisted(() => ({ toastError: vi.fn(), hook: vi.fn(), refetch: vi.fn(), queryState: {} as Record<string, unknown> }));
vi.mock("@/components/ui/toast", () => ({ useToast: () => ({ error: state.toastError }) }));
vi.mock("react-i18next", async (original) => ({
  ...(await original<typeof import("react-i18next")>()),
  useTranslation: () => ({
    t: (key: string) => {
      const value = observability[key as keyof typeof observability];
      return typeof value === "string" ? value : key;
    },
  }),
}));
vi.mock("@/shared", () => ({
  PageHeader: ({ actions }: { actions: ReactNode }) => (
    <header>{actions}</header>
  ),
  SurfaceCard: ({ children, className }: { children: ReactNode; className?: string }) => <section className={className}>{children}</section>,
  useLocaleFormatters: () => ({ formatDateTime: String, formatNumber: String }),
}));
vi.mock("../components/LogContextDialog", () => ({
  LogContextDialog: ({ anchor }: { anchor: { message: string } }) => <div role="dialog" aria-label="日志上下文">{anchor.message}</div>,
}));
vi.mock("../components/ContinuousLogs", () => ({
  ContinuousLogs: () => <section aria-label="连续日志内容">Continuous log stream</section>,
}));
vi.mock("../components/LogMessage", () => ({
  LogMessage: ({ message }: { message: string }) => <span>{message}</span>,
}));
vi.mock("../components/TraceIdentifier", () => ({
  TraceIdentifier: ({
    value,
    onClick,
  }: {
    value: string;
    onClick: () => void;
  }) => <button onClick={onClick}>{value}</button>,
}));
vi.mock("@/components/info-tooltip/InfoTooltip", () => ({
  InfoTooltip: () => null,
}));
vi.mock("@/components/table-pagination", () => ({
  TablePagination: ({
    onPageChange,
  }: {
    onPageChange: (p: number) => void;
  }) => <button onClick={() => onPageChange(2)}>Next page</button>,
}));
vi.mock("../hooks/useObservability", () => ({
  useApplicationLogs: (query: unknown, live: boolean) => {
    state.hook(query, live);
    return {
      data: {
        total: 30,
        records: [
          {
            logId: "1",
            timestamp: 1,
            level: "Info",
            target: "tron",
            message: "Agent run finished",
            traceId: "a".repeat(32),
            spanId: "b".repeat(16),
            attributes: { outcome: "completed" },
          },
        ],
      },
      isPending: false,
      isError: false,
      refetch: state.refetch,
      ...state.queryState,
    };
  },
}));
afterEach(() => {
  cleanup();
  vi.clearAllMocks();
  state.queryState = {};
});
it("combines trace and span filters and prevents invalid ID requests", () => {
  render(<LogsPage />);
  fireEvent.click(screen.getByRole("tab", { name: "日志块" }));
  fireEvent.change(screen.getByRole("textbox", { name: "Trace ID" }), {
    target: { value: "A".repeat(32) },
  });
  fireEvent.change(screen.getByRole("textbox", { name: "Span ID" }), {
    target: { value: "B".repeat(16) },
  });
  fireEvent.click(screen.getByRole("button", { name: "搜索" }));
  expect(state.hook).toHaveBeenLastCalledWith(
    expect.objectContaining({
      traceId: "a".repeat(32),
      spanId: "b".repeat(16),
      current: 1,
    }),
    true
  );
  fireEvent.change(screen.getByRole("textbox", { name: "Span ID" }), {
    target: { value: "bad" },
  });
  fireEvent.click(screen.getByRole("button", { name: "搜索" }));
  expect(state.toastError).toHaveBeenCalledWith(expect.stringContaining("16 位"));
  expect(screen.queryByRole("alert")).toBeNull();
  expect(state.hook).toHaveBeenLastCalledWith(
    expect.objectContaining({ spanId: "b".repeat(16) }),
    true
  );
});
it("clicking an ID filters logs, reset clears filters, and details reveal fields", () => {
  render(<LogsPage />);
  fireEvent.click(screen.getByRole("tab", { name: "日志块" }));
  fireEvent.click(screen.getByRole("button", { name: "a".repeat(32) }));
  expect(state.hook).toHaveBeenLastCalledWith(
    expect.objectContaining({ traceId: "a".repeat(32) }),
    true
  );
  fireEvent.click(screen.getByRole("button", { name: "日志详情" }));
  expect(screen.getByText("completed")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "重置" }));
  expect(state.hook).toHaveBeenLastCalledWith(
    expect.objectContaining({ traceId: "", spanId: "" }),
    true
  );
  expect(screen.queryByRole("button", { name: "自动刷新" })).toBeNull();
});
it("queries the selected local date and applies a changed user time zone", () => {
  const { rerender } = render(<LogsPage />);
  fireEvent.click(screen.getByRole("tab", { name: "日志块" }));
  fireEvent.change(screen.getByLabelText("日志日期"), {
    target: { value: "2026-01-02" },
  });
  expect(state.hook).toHaveBeenLastCalledWith(
    expect.objectContaining({ date: "2026-01-02", timeZone: "Asia/Shanghai" }),
    false
  );
  expect(screen.queryByRole("button", { name: "自动刷新" })).toBeNull();
  useTimeZoneStore.getState().setTimeZone("America/New_York");
  rerender(<LogsPage />);
  expect(state.hook).toHaveBeenLastCalledWith(
    expect.objectContaining({
      date: "2026-01-02",
      timeZone: "America/New_York",
    }),
    false
  );
  useTimeZoneStore.setState({ timeZone: "Asia/Shanghai" });
});

it("defaults to continuous logs first and switches to log blocks with text-only search", () => {
  render(<LogsPage />);
  expect(screen.getAllByRole("tab").map(tab => tab.textContent)).toEqual(["连续日志", "日志块"]);
  expect(screen.getByRole("tab", { name: "连续日志" }).getAttribute("aria-selected")).toBe("true");
  expect(screen.getByRole("button", { name: "搜索" }).querySelector("svg")).toBeNull();
  fireEvent.click(screen.getByRole("tab", { name: "连续日志" }));
  expect(screen.getByRole("region", { name: "连续日志内容" })).toBeTruthy();
  expect(state.hook).toHaveBeenLastCalledWith(expect.anything(), false);
  fireEvent.click(screen.getByRole("tab", { name: "日志块" }));
  expect(screen.queryByRole("region", { name: "连续日志内容" })).toBeNull();
});

it("keeps the table and filters mounted during background refresh", () => {
  const { rerender } = render(<LogsPage />);
  fireEvent.click(screen.getByRole("tab", { name: "日志块" }));
  const table = screen.getByRole("table");
  const keyword = screen.getByRole("textbox", { name: "关键字" });
  fireEvent.change(keyword, { target: { value: "unsent filter" } });
  state.queryState = { isFetching: true };
  rerender(<LogsPage />);
  expect(screen.getByRole("table")).toBe(table);
  expect(screen.getByRole("textbox", { name: "关键字" })).toBe(keyword);
  expect((keyword as HTMLInputElement).value).toBe("unsent filter");
  expect(screen.getByText("Agent run finished")).toBeTruthy();
});
it("does not render inline errors or retry buttons when log loading fails", () => {
  state.queryState = { data: undefined, isError: true };
  render(<LogsPage />);
  fireEvent.click(screen.getByRole("tab", { name: "日志块" }));
  expect(screen.queryByRole("alert")).toBeNull();
  expect(screen.queryByRole("button", { name: "重试" })).toBeNull();
  expect(screen.getByRole("table")).toBeTruthy();
});

it("opens context for a clicked row without intercepting identifier actions", () => {
  render(<LogsPage />);
  fireEvent.click(screen.getByRole("tab", { name: "日志块" }));
  fireEvent.click(screen.getByRole("button", { name: "a".repeat(32) }));
  expect(screen.queryByRole("dialog")).toBeNull();
  fireEvent.click(screen.getByText("Agent run finished").closest("tr")!);
  expect(screen.getByRole("dialog", { name: "日志上下文" }).textContent).toBe("Agent run finished");
});
