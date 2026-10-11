import { useTabVisibility } from "@/components/editable-tabs";
import { useQuery } from "@tanstack/react-query";
import { dudeQueryKeys, listDudes } from "../api/dudeApi";
export function useDudes() {
  const active = useTabVisibility();
  return useQuery({ enabled: active, queryKey: dudeQueryKeys.all, queryFn: ({ signal }) => listDudes(signal), refetchInterval: active ? 15000 : false });
}
