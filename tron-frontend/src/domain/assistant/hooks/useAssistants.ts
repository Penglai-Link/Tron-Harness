import { useTabVisibility } from "@/components/editable-tabs";
import { useQuery } from "@tanstack/react-query";
import { assistantQueryKeys, listAssistants } from "../api/assistantApi";
export function useAssistants() {
  const active = useTabVisibility();
  return useQuery({ enabled: active, queryKey: assistantQueryKeys.all, queryFn: ({ signal }) => listAssistants(signal), refetchInterval: active ? 15000 : false });
}
