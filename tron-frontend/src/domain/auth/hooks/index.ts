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
  const handleForegroundResume = useEffectEvent(() => {
    void ensureSessionFresh();
  });

  useEffect(() => {
    void initialize();
  }, [initialize]);

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
