import { useCallback, useEffect, useRef, useState } from "react";
import { browseRagGraph, expandRagGraph, searchRagGraph } from "../api/ragGraphApi";
import { mergeRagGraphWindow } from "../model/ragGraphWindow";
import type { RagGraphBrowseRequest, RagGraphExpandRequest, RagGraphSearchRequest, RagGraphWindow } from "../model/RagGraphModel";

export function useRagGraph() {
  const [window, setWindow] = useState<RagGraphWindow | null>(null);
  const [error, setError] = useState<Error | null>(null);
  const [loading, setLoading] = useState(false);
  const controller = useRef<AbortController | null>(null);
  useEffect(() => () => controller.current?.abort(), []);
  const reset = useCallback(() => { controller.current?.abort(); controller.current = null; setWindow(null); setError(null); setLoading(false); }, []);
  const request = useCallback(async (body: RagGraphBrowseRequest | RagGraphSearchRequest | RagGraphExpandRequest, append: boolean) => {
    controller.current?.abort(); const active = new AbortController(); controller.current = active;
    setLoading(true); setError(null); if (!append) setWindow(null);
    try {
      const next = "query" in body ? await searchRagGraph(body, active.signal)
        : "nodeIds" in body || "cursor" in body ? await expandRagGraph(body, active.signal)
          : await browseRagGraph(body, active.signal);
      if (!active.signal.aborted) setWindow((old) => append ? mergeRagGraphWindow(old, next) : next);
    } catch (cause) {
      if (!active.signal.aborted) { setWindow(null); setError(cause instanceof Error ? cause : new Error(String(cause))); }
    } finally { if (!active.signal.aborted) setLoading(false); }
  }, []);
  return { window, error, loading, reset, request };
}
