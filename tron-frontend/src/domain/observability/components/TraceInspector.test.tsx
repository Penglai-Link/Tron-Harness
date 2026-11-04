// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import { afterEach, expect, it, vi } from "vitest";
import { TraceInspector } from "./TraceInspector";
import type { TraceDetail } from "../model/Trace";
vi.mock("../api/observabilityApi", () => ({ queryLogs: vi.fn() }));
vi.mock("react-i18next", () => ({ useTranslation: () => ({ t: (key: string) => key }) }));
vi.mock("@/shared/hooks/useLocaleFormatters", () => ({ useLocaleFormatters: () => ({ currentLocale: "en-US", formatNumber: String, formatDateTime: String }) }));
vi.mock("@/components/ui/toast", () => ({ useToast: () => ({ success: vi.fn(), error: vi.fn() }) }));
vi.mock("./SpanVisualization", () => ({ SpanVisualization: ({ mode, selected, onSelect }: { mode: string; selected: string; onSelect: (id: string) => void }) => <div><span>{mode}:{selected}</span><button onClick={() => onSelect("child")}>select child</button></div> }));
afterEach(cleanup);
it("switches detail views and shows input, cached, output, and total tokens", () => {
  const detail: TraceDetail = { trace: { workspaceCode: "WS", traceId: "trace", sessionCode: "session", sessionName: "Session", runCode: "run", agentCode: "main", endedAt: 1350, spanCount: 2, errorCount: 1, modelCount: 0, toolCount: 1, startedAt: 100, durationMs: 1250, status: "Completed", inputTokens: 12000, cachedInputTokens: 5000, outputTokens: 3000 }, spans: [
    { spanId: "root", parentSpanId: null, name: "agent.run", startedAt: 100, durationMs: 1250, status: "Completed", attributes: {}, events: [], links: [] },
    { spanId: "child", parentSpanId: "root", name: "agent.tool", startedAt: 200, durationMs: 300, status: "Error", attributes: { tool: "search" }, events: [], links: [] },
  ] };
  render(<MemoryRouter><TraceInspector detail={detail} /></MemoryRouter>);
  expect(screen.getByRole("table")).toBeTruthy();
  fireEvent.click(screen.getByRole("tab", { name: "waterfall" }));
  fireEvent.click(screen.getByRole("button", { name: "select child" }));
  expect(screen.getByText("waterfall:child")).toBeTruthy();
  fireEvent.click(screen.getByRole("tab", { name: "topology" }));
  expect(screen.getByText("topology:child")).toBeTruthy();
  expect(screen.getByText("search")).toBeTruthy();
  fireEvent.click(screen.getByRole("tab", { name: "tableView" }));
  expect(screen.getByRole("table")).toBeTruthy();
  expect(screen.getByText("search")).toBeTruthy();
  expect(screen.getByRole("button", { name: "openSession" }).getAttribute("href")).toBe("/chats?sessionCode=session&workspaceCode=WS");
  expect(screen.getByText("cachedInputTokens").nextElementSibling?.textContent).toBe("5K");
  expect(screen.getByText("traceTotalTokens").nextElementSibling?.textContent).toBe("15K");
});

it("opens Dude execution traces in the Dude history view", () => {
  const detail: TraceDetail = { trace: { workspaceCode: "WS", traceId: "trace", sessionCode: "dude-session", sessionName: "Session", runCode: "run", agentCode: "main-dude", endedAt: 1350, spanCount: 1, errorCount: 0, modelCount: 0, toolCount: 0, startedAt: 100, durationMs: 1250, status: "Completed", inputTokens: 0, cachedInputTokens: 0, outputTokens: 0 }, spans: [
    { spanId: "root", parentSpanId: null, name: "tron.dude.execute", startedAt: 100, durationMs: 1250, status: "Completed", attributes: {}, events: [], links: [] },
  ] };
  render(<MemoryRouter><TraceInspector detail={detail} /></MemoryRouter>);
  expect(screen.getByRole("button", { name: "openSession" }).getAttribute("href")).toBe("/dudes?dude=main-dude&tab=history&session=dude-session");
});

it.each([
  ["", { "tron.trace.page": "true", "url.path": "/api/v1/profiles/profile/avatar" }],
  ["session", { "tron.trace.page": "true", "url.path": "/chats" }],
  ["   ", {}],
])("does not offer session navigation for a page trigger or missing session (%s)", (sessionCode, attributes) => {
  const detail: TraceDetail = { trace: { workspaceCode: "WS", traceId: "trace", sessionCode, sessionName: "", runCode: "", agentCode: "", endedAt: 200, spanCount: 1, errorCount: 0, modelCount: 0, toolCount: 0, startedAt: 100, durationMs: 100, status: "Completed", inputTokens: 0, cachedInputTokens: 0, outputTokens: 0 }, spans: [
    { spanId: "root", parentSpanId: null, name: "GET avatar", startedAt: 100, durationMs: 100, status: "Completed", attributes, events: [], links: [] },
  ] };
  render(<MemoryRouter><TraceInspector detail={detail} /></MemoryRouter>);
  expect(screen.queryByRole("button", { name: "openSession" })).toBeNull();
  expect(screen.getByRole("button", { name: "copyTraceId" })).toBeTruthy();
});
