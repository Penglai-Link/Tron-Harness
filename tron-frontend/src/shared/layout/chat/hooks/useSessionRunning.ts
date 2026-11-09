import { useEffect } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { getChatActiveRun } from "@/domain/chat/hooks/useChatMessageListBySession";

/** Observe visible sidebar sessions independently of the currently open chat. */
export function useSessionRunning(sessionCode: string) {
  const client = useQueryClient();
  const query = useQuery({
    queryKey: ["sidebar-session-running", sessionCode],
    queryFn: async ({ signal }) => Boolean((await getChatActiveRun(sessionCode, signal)).activeRun),
    enabled: Boolean(sessionCode),
    refetchInterval: 3000,
    staleTime: 0,
    retry: false,
  });

  useEffect(() => {
    const update = (event: Event) => {
      const detail = (event as CustomEvent<{ sessionCode?: string; active?: boolean }>).detail;
      if (detail?.sessionCode !== sessionCode || typeof detail.active !== "boolean") return;
      // Cancel an older poll so it cannot overwrite the newer runtime signal.
      void client.cancelQueries({ queryKey: ["sidebar-session-running", sessionCode] });
      client.setQueryData(["sidebar-session-running", sessionCode], detail.active);
    };
    window.addEventListener("tron:chat-session-run-state", update);
    return () => window.removeEventListener("tron:chat-session-run-state", update);
  }, [client, sessionCode]);

  return query.data ?? false;
}
