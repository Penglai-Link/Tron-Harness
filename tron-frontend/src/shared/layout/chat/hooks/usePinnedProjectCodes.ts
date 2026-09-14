import { useCallback, useEffect, useState } from "react";

const storageKey = "tron.chat.pinned-project-codes";

function readPinnedProjectCodes(): string[] {
  try {
    const value = window.localStorage.getItem(storageKey);
    const parsed = value ? JSON.parse(value) : [];
    return Array.isArray(parsed)
      ? parsed.filter((code): code is string => typeof code === "string")
      : [];
  } catch {
    return [];
  }
}

export function usePinnedProjectCodes() {
  const [pinnedProjectCodes, setPinnedProjectCodes] = useState(readPinnedProjectCodes);

  useEffect(() => {
    window.localStorage.setItem(storageKey, JSON.stringify(pinnedProjectCodes));
  }, [pinnedProjectCodes]);

  const togglePinned = useCallback((projectCode: string) => {
    setPinnedProjectCodes((current) => (
      current.includes(projectCode)
        ? current.filter((code) => code !== projectCode)
        : [projectCode, ...current]
    ));
  }, []);

  return { pinnedProjectCodes, togglePinned };
}
