// @vitest-environment jsdom
import { act, cleanup, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import type { ChatSession } from "@/domain/chat/model";
import { sortPinnedSessions, usePinnedSessionCodes } from "./usePinnedSessionCodes";

afterEach(() => {
  cleanup();
  localStorage.clear();
});

describe("pinned chat sessions", () => {
  it("moves pinned sessions ahead of recent sessions while retaining order within each set", () => {
    const sessions = ["a", "b", "c", "d"].map((sessionCode) => ({ sessionCode })) as ChatSession[];
    expect(sortPinnedSessions(sessions, ["c", "a"]).map((session) => session.sessionCode)).toEqual(["a", "c", "b", "d"]);
    expect(sessions.map((session) => session.sessionCode)).toEqual(["a", "b", "c", "d"]);
  });

  it("persists pin and unpin choices locally", () => {
    const { result } = renderHook(usePinnedSessionCodes);
    act(() => result.current.togglePinnedSession("session-1"));
    expect(result.current.pinnedSessionCodes).toEqual(["session-1"]);
    expect(JSON.parse(localStorage.getItem("tron.chat.pinned-session-codes") ?? "[]")).toEqual(["session-1"]);
    act(() => result.current.togglePinnedSession("session-1"));
    expect(result.current.pinnedSessionCodes).toEqual([]);
  });
});
