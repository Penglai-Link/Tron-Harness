import { useEffect } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { getChatActiveRun, listChatUserInputRequests } from "@/domain/chat/hooks/useChatMessageListBySession";

/** Observe visible sidebar sessions independently of the currently open chat. */
export function useSessionActivity(sessionCode: string) {
  const client = useQueryClient();
  const query = useQuery({
    queryKey: ["sidebar-session-running", sessionCode],
    queryFn: async ({ signal }) => Boolean((await getChatActiveRun(sessionCode, signal)).activeRun),
    enabled: Boolean(sessionCode),
    refetchInterval: 3000,
    staleTime: 0,
    retry: false,
  });
  const inputQuery = useQuery({
    queryKey: ["sidebar-session-awaiting-input", sessionCode],
    queryFn: async ({ signal }) => (await listChatUserInputRequests(sessionCode, signal))
      .some((request) => request.status === "Pending"),
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
      void client.invalidateQueries({ queryKey: ["sidebar-session-awaiting-input", sessionCode] });
    };
    window.addEventListener("tron:chat-session-run-state", update);
    return () => window.removeEventListener("tron:chat-session-run-state", update);
  }, [client, sessionCode]);

  return { running: query.data ?? false, awaitingInput: inputQuery.data ?? false };
}
