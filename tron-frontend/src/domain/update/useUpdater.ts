import { useCallback, useEffect, useMemo, useState } from "react";

import {
  DesktopBridge,
  type UpdateStatus,
} from "@/shared/desktop/DesktopBridge";
import { getApiAccessToken } from "@/shared/api/httpClient";

export function useUpdater() {
  const bridge = useMemo(() => new DesktopBridge(), []);
  const [status, setStatus] = useState<UpdateStatus>({ type: "idle" });
  const isDesktop = bridge.isElectron();

  useEffect(() => {
    if (!isDesktop) return undefined;

    let active = true;
    const stopListening = bridge.onUpdateStatus((nextStatus) => {
      if (active) setStatus(nextStatus);
    });

    void bridge.getUpdateStatus()
      .then((nextStatus) => {
        if (active) setStatus(nextStatus);
      })
      .catch(() => undefined);

    return () => {
      active = false;
      stopListening();
    };
  }, [bridge, isDesktop]);

  const getToken = useCallback(() => getApiAccessToken() ?? "", []);

  const check = useCallback(async () => {
    const nextStatus = await bridge.checkUpdate(getToken());
    setStatus(nextStatus);
    return nextStatus;
  }, [bridge, getToken]);

  const startDownload = useCallback(async () => {
    const started = await bridge.startDownloadUpdate(getToken());
    return started;
  }, [bridge, getToken]);

  const apply = useCallback(async () => {
    const applied = await bridge.applyUpdate();
    return applied;
  }, [bridge]);

  return {
    apply,
    bridge,
    check,
    isDesktop,
    startDownload,
    status,
  };
}

export type { UpdateStatus };
