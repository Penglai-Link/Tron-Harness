// @vitest-environment jsdom
import { act, cleanup, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ChatSession } from "@/domain/chat/model";

const api = vi.hoisted(() => ({ request: vi.fn() }));
vi.mock("@/shared/api/httpClient", () => ({ requestApi: api.request }));
vi.mock("@/shared/api/serviceConfig", () => ({ getServiceDataSource: () => "api" }));
import { useChatGeneratedTitle } from "./useChatGeneratedTitle";

const session: ChatSession = {
  id: 1, sessionCode: "session", workspaceCode: "workspace", sourceType: "MANUAL",
  title: "New chat", status: "ACTIVE", lastRunCode: "run", createdAt: "", updatedAt: "",
};
beforeEach(() => { vi.useFakeTimers(); api.request.mockReset(); });
afterEach(() => { cleanup(); vi.useRealTimers(); });

describe("generated session title", () => {
  it("refreshes a title generated after the chat response, then stops polling", async () => {
    const updated = { ...session, title: "文档会诊" };
    api.request.mockResolvedValueOnce([{ sessions: [session] }])
      .mockResolvedValueOnce([{ sessions: [updated] }]);
    const changed = vi.fn();
    const { rerender } = renderHook(({ current }) => useChatGeneratedTitle(current, changed), { initialProps: { current: session } });
    await act(() => vi.advanceTimersByTimeAsync(1_500));
    expect(changed).not.toHaveBeenCalled();
    await act(() => vi.advanceTimersByTimeAsync(1_500));
    expect(changed).toHaveBeenCalledWith(updated);
    rerender({ current: updated });
    await act(() => vi.advanceTimersByTimeAsync(60_000));
    expect(api.request).toHaveBeenCalledTimes(2);
  });

  it("does not overwrite a renamed title with an older in-flight response", async () => {
    let resolve!: (value: unknown) => void;
    api.request.mockImplementation(() => new Promise((done) => { resolve = done; }));
    const changed = vi.fn();
    const { rerender } = renderHook(({ current }) => useChatGeneratedTitle(current, changed), { initialProps: { current: session } });
    await act(() => vi.advanceTimersByTimeAsync(1_500));
    rerender({ current: { ...session, title: "我的标题" } });
    await act(async () => { resolve([{ sessions: [{ ...session, title: "旧生成标题" }] }]); });
    expect(changed).not.toHaveBeenCalled();
  });

  it("does not poll unsent, task, or already named chats", async () => {
    const variants = [{ ...session, lastRunCode: null }, { ...session, taskCode: "task" }, { ...session, title: "现有标题" }];
    for (const current of variants) {
      const hook = renderHook(() => useChatGeneratedTitle(current, vi.fn()));
      await act(() => vi.advanceTimersByTimeAsync(3_000));
      hook.unmount();
    }
    expect(api.request).not.toHaveBeenCalled();
  });

  it("retries transient failures with a bounded polling lifetime", async () => {
    api.request.mockRejectedValue(new Error("temporary read failure"));
    const hook = renderHook(() => useChatGeneratedTitle(session, vi.fn()));
    await act(() => vi.advanceTimersByTimeAsync(120_000));
    expect(api.request).toHaveBeenCalledTimes(40);
    hook.unmount();
  });
});
