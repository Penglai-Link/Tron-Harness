import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
// @vitest-environment jsdom
import { cleanup, render as rtlRender, screen, fireEvent, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import type { ReactNode } from "react";
import { MemoryRouter } from "react-router";
import { observability } from "@/i18n/locales/zh-CN/observability";
import { DashboardPage } from "./DashboardPage";
const resetApi = vi.hoisted(() => vi.fn().mockResolvedValue({ resetAt: 123 }));
vi.mock("../api/observabilityApi", () => ({ resetObservability: resetApi }));
vi.mock("@/components/ui/toast", () => ({ useToast: () => ({ error: vi.fn() }) }));
function render(ui: ReactNode) { const client = new QueryClient(); return rtlRender(ui, { wrapper: ({ children }) => <QueryClientProvider client={client}>{children}</QueryClientProvider> }); }
const state = vi.hoisted(() => ({
  data: {
    sampledAt: 60000,
    traceCount: 3,
    failedTraceCount: 1,
    runningTraceCount: 1,
    inputTokens: 1000,
    outputTokens: 500,
    averageDurationMs: 1250,
    p95DurationMs: 2000,
    recentRequests: [{ startedAt: 0, requests: 3, errors: 1 }],
    cpuPercent: 3.6,
    memoryBytes: 104857600,
    diskUsage: { bytes: 1610612736 as number | null, sampledAt: 60000, paths: ["/tron-data"], measurement: "Allocated" as const },
    resourceSampledAt: 60000,
    metricPointCount: 8,
    processes: [
      {
        pid: 10,
        parentPid: null,
        name: "tron-backend",
        memoryBytes: 104857600,
        startedAt: 1,
      },
      {
        pid: 20,
        parentPid: 10,
        name: "plugin-backend",
        memoryBytes: 10485760,
        startedAt: 1,
      },
    ],
  },
  isError: false,
}));
vi.mock("react-i18next", async (importOriginal) => ({
  ...(await importOriginal<typeof import("react-i18next")>()),
  useTranslation: () => ({
    t: (key: string, values?: Record<string, unknown>) =>
      (
        (observability[key as keyof typeof observability] as string) ?? key
      ).replace(/{{(.*?)}}/g, (_, k) => String(values?.[k] ?? "")),
  }),
}));
vi.mock("@/shared", () => ({
  PageHeader: ({ actions }: { actions: ReactNode }) => (
    <header>{actions}</header>
  ),
  SurfaceCard: ({ children }: { children: ReactNode }) => <div>{children}</div>,
  useWorkspaceSpaces: () => ({ selectedPrimarySpace: { workspaceCode: "WS" } }),
  useLocaleFormatters: () => ({
    currentLocale: "en-US",
    formatNumber: (n: number, options?: Intl.NumberFormatOptions) =>
      new Intl.NumberFormat("en-US", options).format(n),
    formatDateTime: String,
  }),
}));
vi.mock("../hooks/useObservability", () => ({
  useObservabilityDashboard: () => ({
    ...state,
    isPending: false,
    refetch: vi.fn(),
  }),
}));
vi.mock("../components/CollectorConfigButton", () => ({
  CollectorConfigButton: () => null,
}));
vi.mock("../components/ResourceTrendCards", () => ({
  ResourceTrendCards: () => null,
  PluginCountCard: () => null,
}));
const electronState = vi.hoisted(() => ({ available: false, isError: false, data: undefined as import("@/shared/desktop/DesktopBridge").DesktopProcessMetrics | undefined }));
vi.mock("../hooks/useElectronMetrics", () => ({ useElectronMetrics: () => electronState }));
afterEach(() => { cleanup(); electronState.available = false; electronState.isError = false; electronState.data = undefined; });
it("shows scoped request error rate, readable latency, resources and actual child process relationships", () => {
  render(
    <MemoryRouter>
      <DashboardPage />
    </MemoryRouter>
  );
  expect(screen.getByText("33.3%")).toBeTruthy();
  expect(screen.getByText("累计错误率")).toBeTruthy();
  expect(screen.getByText("最近 15 分钟：33.3%")).toBeTruthy();
  expect(screen.getByText("1.25 s")).toBeTruthy();
  expect(screen.getAllByText((_, element) => element?.textContent === "100 MiB" && !Array.from(element.children).some((child) => child.textContent === "100 MiB"))).toHaveLength(1);
  expect(screen.getByText("plugin-backend")).toBeTruthy();
  expect(screen.getByText("磁盘占用")).toBeTruthy();
  expect(screen.getByText((_, element) => element?.textContent === "1.5 GiB" && element.tagName === "DD")).toBeTruthy();
  expect(
    screen.getByRole("link", { name: "查看链路" }).getAttribute("href")
  ).toBe("/settings/observability/traces");
  expect(screen.queryByRole("link", { name: "查看指标" })).toBeNull();
  expect(screen.getByRole("link", { name: "指标数据点" }).getAttribute("href")).toBe("/settings/observability/metrics");
  const samplingHeader = screen.getByText("采样时间").closest("p")!.parentElement!;
  expect(samplingHeader.querySelector("h2")?.textContent).toBe("资源使用");
  expect(samplingHeader.querySelector("time")?.dateTime).toBe(new Date(state.data.resourceSampledAt).toISOString());
});
it.each([[3, 1], [0, 0]])("shows the minute, request total and error total when hovering a trend column (%i requests)", async (requests, errors) => {
  const previous = state.data;
  state.data = { ...previous, recentRequests: [{ startedAt: 0, requests, errors }] };
  render(<MemoryRouter><DashboardPage /></MemoryRouter>);
  const column = screen.getByLabelText(`0 · ${requests} 次请求`);
  fireEvent.mouseEnter(column);
  fireEvent.mouseMove(column);
  const tooltip = await screen.findByRole("tooltip");
  expect(tooltip.textContent).toContain("0");
  expect(tooltip.textContent).toContain(`${requests} 次请求`);
  expect(tooltip.textContent).toContain(`${errors} 次失败`);
  state.data = previous;
});
it("keeps unavailable disk usage distinct from an empty data directory", () => {
  const previous = state.data;
  state.data = { ...previous, diskUsage: { ...previous.diskUsage, bytes: null } };
  const view = render(<MemoryRouter><DashboardPage /></MemoryRouter>);
  const diskValue = screen.getByText("磁盘占用").closest("dt")!.nextElementSibling;
  expect(diskValue?.textContent).toBe("—");
  state.data = { ...previous, diskUsage: { ...previous.diskUsage, bytes: 0 } };
  view.rerender(<MemoryRouter><DashboardPage /></MemoryRouter>);
  expect(diskValue?.textContent).toBe("0 B");
  state.data = previous;
});
it("does not invent a zero-percent error rate when no requests were collected", () => {
  const previous = state.data;
  state.data = { ...previous, traceCount: 0, failedTraceCount: 0 };
  render(
    <MemoryRouter>
      <DashboardPage />
    </MemoryRouter>
  );
  expect(screen.queryByText("0%")).toBeNull();
  expect(screen.getByText("累计错误率").closest("dt")?.nextElementSibling?.textContent).toBe("—");
  state.data = previous;
});
it.each([[4, "0%"], [0, "—"]])("keeps the recent error rate independent from retained failures with %i recent traces", (requests, rate) => {
  const previous = state.data;
  state.data = { ...previous, recentRequests: [{ startedAt: 0, requests, errors: 0 }] };
  render(<MemoryRouter><DashboardPage /></MemoryRouter>);
  expect(screen.getByText("33.3%")).toBeTruthy();
  expect(screen.getByText(`最近 15 分钟：${rate}`)).toBeTruthy();
  state.data = previous;
});

it("requires confirmation and resets globally only after explicit confirmation", async () => {
  resetApi.mockClear();
  render(<MemoryRouter><DashboardPage /></MemoryRouter>);
  fireEvent.click(screen.getByRole("button", { name: "重置统计" }));
  expect(screen.getByRole("dialog").textContent).toContain("所有工作区");
  expect(resetApi).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: "取消" }));
  expect(resetApi).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: "重置统计" }));
  fireEvent.click(screen.getByRole("button", { name: "我已知晓" }));
  await waitFor(() => expect(resetApi).toHaveBeenCalledTimes(1));
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
});

it("sums all Electron processes with Rust and shows the same processes in topology", async () => {
  electronState.available = true;
  electronState.data = {
    sampledAt: 60000, memoryBytes: 300 * 1048576,
    processes: [
      { pid: 101, parentPid: null, name: "Electron · Browser", memoryBytes: 100 * 1048576, startedAt: 1 },
      { pid: 102, parentPid: null, name: "Electron · GPU", memoryBytes: 200 * 1048576, startedAt: 1 },
    ],
  };
  render(<MemoryRouter><DashboardPage /></MemoryRouter>);
  const row = screen.getByLabelText("总内存");
  expect(row.textContent).toContain("400 MiB");
  expect(screen.getByText("Electron · GPU")).toBeTruthy();
  expect(screen.getByText("4 个进程")).toBeTruthy();
  fireEvent.mouseEnter(row); fireEvent.mouseMove(row);
  const tooltip = await screen.findByRole("tooltip");
  expect(tooltip.textContent).toContain("Electron（所有进程）: 300 MiB");
  expect(tooltip.textContent).toContain("Rust 后端: 100 MiB");
});
it("does not present Rust alone as a total in browser mode or reuse failed Electron samples", () => {
  const view = render(<MemoryRouter><DashboardPage /></MemoryRouter>);
  expect(screen.getByLabelText("总内存").textContent).toContain("—");
  expect(screen.getByText(observability.electronBrowserUnavailable)).toBeTruthy();
  view.unmount();
  electronState.available = true; electronState.isError = true;
  electronState.data = { sampledAt: 1, memoryBytes: 999, processes: [] };
  render(<MemoryRouter><DashboardPage /></MemoryRouter>);
  expect(screen.getByLabelText("总内存").textContent).toContain("—");
  expect(screen.getByText(observability.electronMetricsFailed)).toBeTruthy();
});
