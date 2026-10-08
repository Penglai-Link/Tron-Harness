import { useCallback, useEffect, useState } from "react";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { listLoopDesigns } from "../api/loopApi";
import { listLoopDesignsMock } from "../mock/loopData";
import { LOOP_DESIGNS_CHANGED, type LoopDesign } from "../model/LoopModel";
export function useLoopDesignList() {
  const [designs, setDesigns] = useState<LoopDesign[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const source = getServiceDataSource("agent");
  const refresh = useCallback(
    async (signal?: AbortSignal) => {
      setLoading(true);
      try {
        const records =
          source === "api"
            ? await listLoopDesigns(signal)
            : listLoopDesignsMock();
        if (!signal?.aborted) {
          setDesigns(records);
          setError(null);
        }
      } catch (e) {
        if (!signal?.aborted)
          setError(e instanceof Error ? e.message : String(e));
      } finally {
        if (!signal?.aborted) setLoading(false);
      }
    },
    [source],
  );
  useEffect(() => {
    const controller = new AbortController();
    void refresh(controller.signal);
    const changed = () => {
      void refresh(controller.signal);
    };
    window.addEventListener(LOOP_DESIGNS_CHANGED, changed);
    return () => {
      controller.abort();
      window.removeEventListener(LOOP_DESIGNS_CHANGED, changed);
    };
  }, [refresh]);
  return { designs, loading, error, refresh };
}
