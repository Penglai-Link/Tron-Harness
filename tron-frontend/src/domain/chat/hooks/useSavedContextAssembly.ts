import { useQuery } from "@tanstack/react-query";
import { useAuthStore } from "@/store/authStore";
import { requestApi } from "@/shared/api/httpClient";
import type { ContextAssemblySnapshot } from "../model/contextAssemblyReport";

/** Full reports are persisted before their live event is emitted. No aggregate fallback. */
export function useSavedContextAssembly(sessionCode: string, live: ContextAssemblySnapshot | null, dudeCode?: string, refreshInterval?: number) {
  const user = useAuthStore((state) => state.session?.user.userCode ?? "");
  const query = useQuery({
    queryKey: ["chat-context-assembly", user, sessionCode, dudeCode],
    enabled: Boolean(user && sessionCode && !live),
    queryFn: ({ signal }) => requestApi<{snapshot: ContextAssemblySnapshot | null}>("GET /api/v1/chats/sessions/get-context-assembly", {queryParams:{sessionCode}, headers: dudeCode ? { "X-Dude-Code": dudeCode } : undefined, signal}),
    gcTime: 0,
    staleTime: 0,
    refetchOnMount: "always",
    refetchInterval: refreshInterval,
    retry: false,
  });
  return {snapshot: live ?? query.data?.snapshot ?? null, loading: !live && query.isPending && query.isFetching, error: !live ? query.error : null, retry: query.refetch};
}
