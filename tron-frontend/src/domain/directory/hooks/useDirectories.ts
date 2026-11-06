import { useCallback, useEffect, useState } from "react";
import { listDirectories, type DirectoryRecord } from "../api/directoryApi";

export function useDirectories(workspaceCode: string) {
  const [items, setItems] = useState<DirectoryRecord[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);
  const [revision, setRevision] = useState(0);
  const refresh = useCallback(() => setRevision((value) => value + 1), []);

  useEffect(() => {
    window.addEventListener("tron:directories-changed", refresh);
    return () => window.removeEventListener("tron:directories-changed", refresh);
  }, [refresh]);

  useEffect(() => {
    const controller = new AbortController();
    setError(null);
    if (!workspaceCode) {
      setLoading(false);
      return () => controller.abort();
    }
    setLoading(true);
    listDirectories(workspaceCode, controller.signal).then((result) => {
      if (!controller.signal.aborted) setItems(result);
    }).catch((reason: unknown) => {
      if (!controller.signal.aborted) setError(reason instanceof Error ? reason : new Error(String(reason)));
    }).finally(() => {
      if (!controller.signal.aborted) setLoading(false);
    });
    return () => controller.abort();
  }, [workspaceCode, revision]);

  return { items: items.filter((item) => item.workspaceCode === workspaceCode), loading, error, refresh };
}
