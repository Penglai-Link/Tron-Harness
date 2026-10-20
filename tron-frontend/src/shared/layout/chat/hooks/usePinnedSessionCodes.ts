import { useCallback, useEffect, useState } from "react";
import type { ChatSession } from "@/domain/chat/model";

const storageKey = "tron.chat.pinned-session-codes";

export function sortPinnedSessions(sessions: ChatSession[], pinnedSessionCodes: string[]) {
  const pinned = new Set(pinnedSessionCodes);
  return [...sessions].sort((left, right) =>
    Number(pinned.has(right.sessionCode)) - Number(pinned.has(left.sessionCode))
  );
}

function readPinnedSessionCodes(): string[] {
  try {
    const value = window.localStorage.getItem(storageKey);
    const parsed: unknown = value ? JSON.parse(value) : [];
    return Array.isArray(parsed)
      ? parsed.filter((code): code is string => typeof code === "string")
      : [];
  } catch {
    return [];
  }
}

export function usePinnedSessionCodes() {
  const [pinnedSessionCodes, setPinnedSessionCodes] = useState(readPinnedSessionCodes);

  useEffect(() => {
    window.localStorage.setItem(storageKey, JSON.stringify(pinnedSessionCodes));
  }, [pinnedSessionCodes]);

  const togglePinnedSession = useCallback((sessionCode: string) => {
    setPinnedSessionCodes((current) => current.includes(sessionCode)
      ? current.filter((code) => code !== sessionCode)
      : [sessionCode, ...current]);
  }, []);

  const removePinnedSession = useCallback((sessionCode: string) => {
    setPinnedSessionCodes((current) => current.filter((code) => code !== sessionCode));
  }, []);

  return { pinnedSessionCodes, togglePinnedSession, removePinnedSession };
}
