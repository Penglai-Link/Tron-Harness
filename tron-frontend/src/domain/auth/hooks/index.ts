import {
  useEffect,
  useEffectEvent,
} from "react";
import { useShallow } from "zustand/react/shallow";

import { useAuthStore } from "@/store/authStore";

export function useAuthSession() {
  return useAuthStore(useShallow((state) => ({
    session: state.session,
    status: state.status,
  })));
}

export function useBootstrapAuthSession() {
  const initialize = useAuthStore((state) => state.initialize);
  const ensureSessionFresh = useAuthStore((state) => state.ensureSessionFresh);
  const userCode = useAuthStore((state) => state.session?.user.userCode);
  const refreshAccountSnapshot = useAuthStore((state) => state.refreshAccountSnapshot);
  const handleForegroundResume = useEffectEvent(() => {
    void ensureSessionFresh();
  });

  useEffect(() => {
    void initialize();
  }, [initialize]);

  useEffect(() => {
    if (!userCode) return;
    // Reads the local persisted snapshot, never the upstream provider. Keep it
    // independent of the user-menu's mounting and open/closed state.
    const refresh = () => { if (document.visibilityState === "visible") void refreshAccountSnapshot(); };
    refresh();
    const timer = window.setInterval(refresh, 15_000);
    return () => window.clearInterval(timer);
  }, [userCode, refreshAccountSnapshot]);

  useEffect(() => {
    function handleVisibilityChange() {
      if (document.visibilityState !== "visible") {
        return;
      }

      handleForegroundResume();
    }

    window.addEventListener("focus", handleForegroundResume);
    window.addEventListener("online", handleForegroundResume);
    document.addEventListener("visibilitychange", handleVisibilityChange);

    return () => {
      window.removeEventListener("focus", handleForegroundResume);
      window.removeEventListener("online", handleForegroundResume);
      document.removeEventListener("visibilitychange", handleVisibilityChange);
    };
  }, [handleForegroundResume]);
}
