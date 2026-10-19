import { useQuery } from "@tanstack/react-query";
import { useAuthStore } from "@/store/authStore";
import { requestApi } from "@/shared/api/httpClient";
import type { ContextAssemblySnapshot } from "../model/contextAssemblyReport";

/** Full reports are persisted before their live event is emitted. No aggregate fallback. */
export function useSavedContextAssembly(sessionCode: string, live: ContextAssemblySnapshot | null) {
  const user = useAuthStore((state) => state.session?.user.userCode ?? "");
  const query = useQuery({
    queryKey: ["chat-context-assembly", user, sessionCode],
    enabled: Boolean(user && sessionCode && !live),
    queryFn: ({ signal }) => requestApi<{snapshot: ContextAssemblySnapshot | null}>("GET /api/v1/chats/sessions/get-context-assembly", {queryParams:{sessionCode}, signal}),
    gcTime: 0,
    staleTime: 0,
    refetchOnMount: "always",
    retry: false,
  });
  return {snapshot: live ?? query.data?.snapshot ?? null, loading: !live && query.isFetching, error: !live ? query.error : null, retry: query.refetch};
}
