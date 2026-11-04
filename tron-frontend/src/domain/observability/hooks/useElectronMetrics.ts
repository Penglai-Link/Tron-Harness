import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { DesktopBridge } from "@/shared/desktop/DesktopBridge";

export function useElectronMetrics() {
  const bridge = useMemo(() => new DesktopBridge(), []);
  const available = bridge.isElectron();
  const query = useQuery({
    queryKey: ["observability-electron-processes"],
    queryFn: () => bridge.getProcessMetrics(),
    enabled: available,
    refetchInterval: 5000,
    staleTime: 5000,
    retry: false,
  });
  return { ...query, available };
}
