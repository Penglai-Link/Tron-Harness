import { useEffect, useState } from "react";
import { chatApiEndpoints } from "@/domain/chat/api";
import type { SessionApplications } from "@/domain/chat/model/SessionApplications";
import { useServiceQueryState } from "@/shared/api/serviceConfig";

const empty = (): SessionApplications => ({ projectCode: null, primaryAppCode: null, workingDirectory: null, applications: [] });

export function useSessionApplications(sessionCode: string, active: boolean, runKey: string) {
  const [focusRevision, setFocusRevision] = useState(0);
  useEffect(() => {
    if (!active) return;
    const refresh = () => setFocusRevision((value) => value + 1);
    window.addEventListener("focus", refresh);
    return () => window.removeEventListener("focus", refresh);
  }, [active]);
  const query = useServiceQueryState<SessionApplications>({
    domain: "chat",
    enabled: active && Boolean(sessionCode),
    endpoint: chatApiEndpoints.getSessionApplications,
    params: { sessionCode },
    deps: [sessionCode],
    refreshKey: `${runKey}:${focusRevision}`,
    apiInitialData: empty,
    mockData: empty,
    suppressErrorToast: true,
  });
  useEffect(() => {
    if (!active || !sessionCode || query.loading) return;
    const timer = window.setTimeout(query.retry, 3000);
    return () => window.clearTimeout(timer);
  }, [active, sessionCode, query.loading, query.data, query.retry]);
  return query;
}
