// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter, useLocation } from "react-router";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { i18n } from "@/i18n";
import { NotificationLauncher } from "./NotificationLauncher";
import { listNotifications, notificationQueryKey, updateNotification, type PlatformNotification } from "../api/notificationApi";

vi.mock("../api/notificationApi", async (importOriginal) => ({
  ...await importOriginal<typeof import("../api/notificationApi")>(),
  listNotifications: vi.fn(), updateNotification: vi.fn(),
}));
const clients: QueryClient[] = [];
const unread: PlatformNotification[] = Array.from({ length: 7 }, (_, index) => ({
  id: String(index), dudeCode: "main-dude", title: `通知 ${index}`, body: "正文",
  category: "work", target: {}, readAt: null, resolvedAt: null, archivedAt: null, createdAt: index + 1,
}));
const items = [...unread,
  { ...unread[0], id: "read", readAt: 1 },
  { ...unread[0], id: "archived", archivedAt: 1 },
];
function Location() { return <output data-testid="location">{useLocation().pathname}</output>; }
async function setup(records = items) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false, staleTime: Infinity } } });
  clients.push(client);
  client.setQueryData(notificationQueryKey, records);
  render(<QueryClientProvider client={client}><MemoryRouter initialEntries={["/workspace"]}><NotificationLauncher /><Location /></MemoryRouter></QueryClientProvider>);
  fireEvent.click(screen.getByRole("button"));
  await screen.findByText("消息中心");
  return client;
}
beforeEach(async () => {
  await i18n.changeLanguage("zh-CN");
  vi.mocked(listNotifications).mockResolvedValue(items.map(item => ({ ...item, readAt: 1 })));
  vi.mocked(updateNotification).mockResolvedValue({});
});
afterEach(() => { cleanup(); clients.splice(0).forEach(client => client.clear()); vi.resetAllMocks(); });

it("places both actions in the header and marks all unread records, including those beyond the preview", async () => {
  let finish!: () => void;
  const request = new Promise<void>(resolve => { finish = resolve; });
  vi.mocked(updateNotification).mockImplementation(() => request);
  const client = await setup();
  const action = screen.getByRole("button", { name: "全部已读" });
  const header = action.closest("header")!;
  expect(within(header).getByRole("link", { name: "消息中心" })).toBeTruthy();
  expect(screen.getAllByText(/^通知 \d$/)).toHaveLength(5);
  fireEvent.click(action);
  await waitFor(() => expect(updateNotification).toHaveBeenCalledTimes(7));
  unread.forEach(item => expect(updateNotification).toHaveBeenCalledWith(item.id, { read: true, archived: false }));
  expect(action.hasAttribute("disabled")).toBe(true);
  fireEvent.click(action);
  expect(updateNotification).toHaveBeenCalledTimes(7);
  await act(async () => { finish(); });
  await screen.findByText("暂无未读通知");
  expect(client.getQueryData(notificationQueryKey)).toEqual(items.map(item => ({ ...item, readAt: 1 })));
});

it("shows a failed update and refreshes actual read states after partial success", async () => {
  vi.mocked(updateNotification).mockRejectedValueOnce(new Error("更新失败"));
  vi.mocked(listNotifications).mockResolvedValue(items.map(item => item.id === "0" ? item : { ...item, readAt: 1 }));
  await setup();
  fireEvent.click(screen.getByRole("button", { name: "全部已读" }));
  expect((await screen.findByRole("alert")).textContent).toContain("更新失败");
  await waitFor(() => expect(screen.getAllByText(/^通知 \d$/)).toHaveLength(1));
  await waitFor(() => expect(screen.getByRole("button", { name: "全部已读" }).hasAttribute("disabled")).toBe(false));
});

it("keeps the empty-state action disabled and navigates from the header", async () => {
  await setup([]);
  expect(screen.getByRole("button", { name: "全部已读" }).hasAttribute("disabled")).toBe(true);
  expect(screen.getByText("暂无未读通知")).toBeTruthy();
  fireEvent.click(screen.getByRole("link", { name: "消息中心" }));
  expect(screen.getByTestId("location").textContent).toBe("/notifications");
  await waitFor(() => expect(screen.queryByRole("link", { name: "消息中心" })).toBeNull());
  expect(updateNotification).not.toHaveBeenCalled();
});
