import { useEffect } from "react";
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

  useEffect(() => {
    void initialize();
  }, [initialize]);
}
