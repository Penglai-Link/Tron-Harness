// @vitest-environment jsdom
import { act, cleanup, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, expect, it } from "vitest";
import { useAguiWorkReplayPreferences } from "./aguiWorkReplayPreferences";

const storageKey = "tron.aguiWorkReplayPreferences";

beforeEach(() => window.localStorage.removeItem(storageKey));
afterEach(() => {
  cleanup();
  window.localStorage.removeItem(storageKey);
});

it("defaults reasoning cards off and Enter to steer, then syncs and persists preferences", () => {
  const first = renderHook(useAguiWorkReplayPreferences);
  const second = renderHook(useAguiWorkReplayPreferences);
  expect(first.result.current.preferences).toEqual({ recentTurnLimit: 5, showReasoningCards: false, runningEnterSubmitMode: "steer" });

  act(() => first.result.current.setShowReasoningCards(true));
  expect(second.result.current.preferences.showReasoningCards).toBe(true);
  act(() => second.result.current.setRecentTurnLimit(8));
  act(() => second.result.current.setRunningEnterSubmitMode("queue"));
  expect(first.result.current.preferences).toEqual({ recentTurnLimit: 8, showReasoningCards: true, runningEnterSubmitMode: "queue" });

  first.unmount();
  second.unmount();
  const restored = renderHook(useAguiWorkReplayPreferences);
  expect(restored.result.current.preferences).toEqual({ recentTurnLimit: 8, showReasoningCards: true, runningEnterSubmitMode: "queue" });
});

it("keeps reasoning cards off for previously saved replay settings", () => {
  window.localStorage.setItem(storageKey, JSON.stringify({ recentTurnLimit: 7 }));
  const view = renderHook(useAguiWorkReplayPreferences);
  expect(view.result.current.preferences).toEqual({ recentTurnLimit: 7, showReasoningCards: false, runningEnterSubmitMode: "steer" });
});
