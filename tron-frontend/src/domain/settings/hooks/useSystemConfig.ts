import { useCallback, useEffect, useState } from "react";

import { getSystemConfig, updateSystemConfig } from "@/domain/settings/api/systemConfigApi";
import type { SystemConfigChange, SystemConfigDocument } from "@/domain/settings/model/SystemConfig";

export function useSystemConfig() {
  const [document, setDocument] = useState<SystemConfigDocument | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [refreshKey, setRefreshKey] = useState(0);

  useEffect(() => {
    const controller = new AbortController();
    setLoading(true);
    setError(null);
    void getSystemConfig(controller.signal)
      .then(setDocument)
      .catch((reason) => {
        if (!controller.signal.aborted) setError(reason);
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });
    return () => controller.abort();
  }, [refreshKey]);

  const save = useCallback(async (changes: SystemConfigChange[], confirmMigration: boolean) => {
    if (!document) return null;
    setSaving(true);
    setError(null);
    try {
      const result = await updateSystemConfig(document.revision, changes, confirmMigration);
      setRefreshKey((current) => current + 1);
      return result;
    } catch (reason) {
      setError(reason);
      throw reason;
    } finally {
      setSaving(false);
    }
  }, [document]);

  return { document, error, loading, save, saving };
}
