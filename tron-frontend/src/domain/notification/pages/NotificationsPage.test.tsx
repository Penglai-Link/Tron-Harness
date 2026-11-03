// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter, useLocation } from "react-router";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { i18n } from "@/i18n";
import { TooltipProvider } from "@/components/ui/tooltip";
import { NotificationsPage } from "./NotificationsPage";
import { deleteNotification, listNotifications, notificationQueryKey, updateNotification, type PlatformNotification } from "../api/notificationApi";

vi.mock("@/components/editable-tabs", () => ({ useTabVisibility: () => true }));
vi.mock("../api/notificationApi", async (importOriginal) => ({ ...await importOriginal<typeof import("../api/notificationApi")>(),
  listNotifications: vi.fn(), updateNotification: vi.fn(), deleteNotification: vi.fn(), deleteArchivedNotifications: vi.fn(),
}));
const fullBody = "这是一份需要完整保留的通知正文。".repeat(40) + "正文末尾也要保留。";
const items: PlatformNotification[] = [
  { id: "one", dudeCode: "main-dude", title: "需要你确认的事项", body: fullBody, category: "work", target: { changeKind: "NeedsDecision", requestedAction: "请确认最终结果", evidence: ["sessionCode=original", "taskCode=original"] }, readAt: null, resolvedAt: null, archivedAt: null, createdAt: 1793736000000 },
  { id: "two", dudeCode: "main-dude", title: "已经完成的事项", body: "已完成的简短正文", category: "work", target: { changeKind: "Completed", requestedAction: "过时的待办要求", resolution: "已向用户确认并处理完成" }, readAt: 1, resolvedAt: 2, archivedAt: null, createdAt: 1793735000000 },
  { id: "three", dudeCode: null, title: "旧的归档通知", body: "归档的正文", category: "system", target: {}, readAt: 1, resolvedAt: null, archivedAt: 2, createdAt: 1793734000000 },
];
const clients: QueryClient[] = [];
function Location() { const location = useLocation(); return <output data-testid="location">{location.pathname}{location.search}</output>; }
function setup(path = "/notifications") {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false, staleTime: Infinity } } });
  clients.push(client);
  client.setQueryData(notificationQueryKey, items);
  return { client, ...render(<QueryClientProvider client={client}><TooltipProvider><MemoryRouter initialEntries={[path]}><NotificationsPage /><Location /></MemoryRouter></TooltipProvider></QueryClientProvider>) };
}
beforeEach(async () => {
  await i18n.changeLanguage("zh-CN");
  vi.mocked(listNotifications).mockResolvedValue(items);
  vi.mocked(updateNotification).mockResolvedValue({});
  vi.mocked(deleteNotification).mockResolvedValue({});
});
afterEach(() => { cleanup(); clients.splice(0).forEach((client) => client.clear()); vi.clearAllMocks(); });

it.each(["zh-CN", "en-US"])("shows compact summaries and restores the full content in the %s reader", async (locale) => {
  await i18n.changeLanguage(locale);
  const { client } = setup();
  expect(screen.queryByText(fullBody)).toBeNull();
  expect(screen.queryByText("请确认最终结果")).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "需要你确认的事项" }));
  const dialog = await screen.findByRole("dialog", { name: "需要你确认的事项" });
  expect(within(dialog).getByText(fullBody).textContent).toBe(fullBody);
  expect(within(dialog).getByText("请确认最终结果")).toBeTruthy();
  expect((within(dialog).getByText(/sessionCode=original/).closest("details") as HTMLDetailsElement).open).toBe(false);
  expect(updateNotification).toHaveBeenCalledWith("one", { read: true, archived: false });
  const summary = within(dialog).getByText(locale === "zh-CN" ? /变化依据/ : /Evidence/).closest("summary")!;
  fireEvent.click(summary);
  expect((summary.parentElement as HTMLDetailsElement).open).toBe(true);
  await act(async () => client.setQueryData(notificationQueryKey, items.map((item) => ({ ...item }))));
  expect((summary.parentElement as HTMLDetailsElement).open).toBe(true);
  fireEvent.click(within(dialog).getByRole("button", { name: locale === "zh-CN" ? "查看关联内容" : "Open related content" }));
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  expect(screen.getByTestId("location").textContent).toBe("/dudes?dude=main-dude");
});

it("keeps row actions separate from opening the reader and provides archived restore", async () => {
  setup();
  fireEvent.click(screen.getByRole("button", { name: "归档" }));
  await waitFor(() => expect(updateNotification).toHaveBeenCalledWith("one", { read: false, archived: true }));
  expect(screen.queryByRole("dialog")).toBeNull();
  fireEvent.click(screen.getByRole("tab", { name: /已归档/ }));
  const restore = await screen.findByRole("button", { name: "恢复" });
  await waitFor(() => expect((restore as HTMLButtonElement).disabled).toBe(false));
  fireEvent.click(restore);
  await waitFor(() => expect(updateNotification).toHaveBeenCalledWith("three", { read: true, archived: false }));
  expect(screen.queryByRole("dialog")).toBeNull();
});

it("shows resolution without a stale requested action and preserves the reader on polling", async () => {
  const { client } = setup("/notifications?notification=two");
  const dialog = await screen.findByRole("dialog", { name: "已经完成的事项" });
  expect(within(dialog).getByText("已向用户确认并处理完成")).toBeTruthy();
  expect(within(dialog).queryByText("过时的待办要求")).toBeNull();
  expect(updateNotification).not.toHaveBeenCalled();
  await act(async () => client.setQueryData(notificationQueryKey, items.map((item) => ({ ...item }))));
  expect(within(dialog).getByText("已完成的简短正文")).toBeTruthy();
  fireEvent.click(within(dialog).getAllByRole("button", { name: "关闭" })[0]);
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  expect(screen.getByTestId("location").textContent).toBe("/notifications");
});

it("keeps destructive deletion behind confirmation", async () => {
  setup();
  fireEvent.click(screen.getByRole("tab", { name: /已归档/ }));
  fireEvent.click(await screen.findByRole("button", { name: "删除" }));
  const dialog = screen.getByRole("dialog", { name: "删除这条通知？" });
  expect(deleteNotification).not.toHaveBeenCalled();
  fireEvent.click(within(dialog).getByRole("button", { name: "删除" }));
  await waitFor(() => expect(deleteNotification).toHaveBeenCalledWith("three"));
});

it("handles missing deep links with a dismissible message", async () => {
  setup("/notifications?notification=gone");
  const dialog = await screen.findByRole("dialog", { name: "通知" });
  expect(within(dialog).getByText("这条通知已删除或无法查看。")).toBeTruthy();
  fireEvent.click(within(dialog).getAllByRole("button", { name: "关闭" })[0]);
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
});
