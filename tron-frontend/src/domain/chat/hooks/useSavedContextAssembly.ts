import { useQuery } from "@tanstack/react-query";
import { useAuthStore } from "@/store/authStore";
import { useTabVisibility } from "@/components/editable-tabs";
import { requestApi } from "@/shared/api/httpClient";
import type { ContextAssemblySnapshot } from "../model/contextAssemblyReport";

/** Full reports are persisted before their live event is emitted. No aggregate fallback. */
export function useSavedContextAssembly(sessionCode: string, live: ContextAssemblySnapshot | null, assistantCode?: string, refreshInterval = 2_000) {
  const user = useAuthStore((state) => state.session?.user.userCode ?? "");
  const visible = useTabVisibility();
  const query = useQuery({
    queryKey: ["chat-context-assembly", user, sessionCode, assistantCode],
    enabled: Boolean(user && sessionCode && visible),
    queryFn: ({ signal }) => requestApi<{snapshot: ContextAssemblySnapshot | null}>("GET /api/v1/chats/sessions/get-context-assembly", {queryParams:{sessionCode}, headers: assistantCode ? { "X-Assistant-Code": assistantCode } : undefined, signal}),
    gcTime: 0,
    staleTime: 0,
    refetchOnMount: "always",
    refetchInterval: refreshInterval,
    retry: false,
  });
  const saved = query.data?.snapshot ?? null;
  // Replay can lag behind storage. Compare session sequences so an old live
  // event cannot pin the panel, and an in-flight read cannot replace a newer event.
  const snapshot = live && (!saved || BigInt(live.snapshotSequence ?? "0") > BigInt(saved.snapshotSequence ?? "0"))
    ? live : saved;
  return {snapshot, loading: !snapshot && query.isPending && query.isFetching, error: !snapshot ? query.error : null, retry: query.refetch};
}
