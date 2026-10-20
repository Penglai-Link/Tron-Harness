import { useEffect, useState } from "react";

/** Refresh the saved context and AG-UI audit while the inspector can show changes. */
export function useChatContextRefreshInterval(sessionCode: string, enabled: boolean, intervalMs = 2_000) {
  const [refreshKey, setRefreshKey] = useState(0);

  useEffect(() => {
    if (!sessionCode || !enabled) return;
    const timer = window.setInterval(() => {
      if (document.visibilityState === "visible") {
        setRefreshKey((current) => current + 1);
      }
    }, intervalMs);
    return () => window.clearInterval(timer);
  }, [sessionCode, enabled, intervalMs]);

  return refreshKey;
}
