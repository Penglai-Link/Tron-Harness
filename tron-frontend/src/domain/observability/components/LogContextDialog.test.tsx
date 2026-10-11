// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import type { ApplicationLog } from "../model/ApplicationLog";
import { LogContextDialog } from "./LogContextDialog";
const api = vi.hoisted(() => vi.fn());
vi.mock("../api/observabilityApi", () => ({ queryLogs: api }));
vi.mock("react-i18next", () => ({ useTranslation: () => ({ t: (key: string) => key }) }));
vi.mock("@/shared", () => ({ useLocaleFormatters: () => ({ formatDateTime: String }) }));
vi.mock("@/components/ui/toast", () => ({ useToast: () => ({ error: vi.fn() }) }));
vi.mock("./LogMessage", () => ({ LogMessage: ({ message }: { message: string }) => <p>{message}</p> }));
const row = (n: number): ApplicationLog => ({ logId: `file:${n}`, timestamp: n, level: "Info", target: "test", message: `line-${n}`, traceId: null, spanId: null, attributes: {} });
const scope = { date: "2026-10-11", timeZone: "UTC", workspaceCode: "alpha" };
afterEach(() => { cleanup(); vi.clearAllMocks(); });
it("centres the anchor between its physical neighbours and loads both directions without duplicates", async () => {
  api.mockResolvedValueOnce({ records: [row(4), row(3)], total: 4 })
    .mockResolvedValueOnce({ records: [row(7), row(6)], total: 4 })
    .mockResolvedValueOnce({ records: [row(2), row(1)], total: 2 })
    .mockResolvedValueOnce({ records: [row(9), row(8)], total: 2 });
  const { container } = render(<LogContextDialog anchor={row(5)} scope={scope} onClose={vi.fn()} />);
  await screen.findByText("line-7");
  const ids = () => [...container.ownerDocument.querySelectorAll("[data-log-id]")].map(el => el.getAttribute("data-log-id"));
  expect(ids()).toEqual(["file:3", "file:4", "file:5", "file:6", "file:7"]);
  expect(document.querySelector("[data-anchor=true]")?.textContent).toContain("line-5");
  expect(api.mock.calls[0][0]).toMatchObject({ ...scope, beforeLogId: "file:5", keyword: "", traceId: "", level: "" });
  expect(api.mock.calls[1][0]).toMatchObject({ afterLogId: "file:5" });
  const viewport = screen.getByRole("region", { name: "logContext" });
  let height = 1000;
  Object.defineProperties(viewport, { scrollHeight: { configurable: true, get: () => height }, clientHeight: { configurable: true, value: 300 }, scrollTop: { configurable: true, writable: true, value: 20 } });
  fireEvent.scroll(viewport);
  height = 1400;
  await screen.findByText("line-1");
  expect(viewport.scrollTop).toBe(420);
  expect(api.mock.calls[2][0]).toMatchObject({ beforeLogId: "file:3" });
  viewport.scrollTop = 1100;
  fireEvent.scroll(viewport);
  await screen.findByText("line-9");
  expect(api.mock.calls[3][0]).toMatchObject({ afterLogId: "file:7" });
  expect(ids()).toEqual(Array.from({ length: 9 }, (_, i) => `file:${i + 1}`));
  fireEvent.scroll(viewport);
  await waitFor(() => expect(api).toHaveBeenCalledTimes(4));
});
