// @vitest-environment jsdom
import { act, renderHook } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { useChatContextRefreshInterval } from "./useChatContextRefreshInterval";

afterEach(() => vi.useRealTimers());

it("refreshes a visible running session and stops after the run ends", () => {
  vi.useFakeTimers();
  const view = renderHook(
    ({ sessionCode, running }) => useChatContextRefreshInterval(sessionCode, running),
    { initialProps: { sessionCode: "session-a", running: true } },
  );

  act(() => vi.advanceTimersByTime(2_000));
  expect(view.result.current).toBe(1);

  view.rerender({ sessionCode: "session-a", running: false });
  act(() => vi.advanceTimersByTime(4_000));
  expect(view.result.current).toBe(1);

  view.rerender({ sessionCode: "session-b", running: true });
  act(() => vi.advanceTimersByTime(2_000));
  expect(view.result.current).toBe(2);
  view.unmount();
});
