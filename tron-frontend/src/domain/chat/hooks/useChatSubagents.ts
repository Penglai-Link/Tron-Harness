import { useEffect, useMemo, useState } from "react";
import { requestApi } from "@/shared/api/httpClient";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import type { ChatSubagent, ChatSubagentDetail } from "../model/ChatSubagent";

export function subagentEndpoint(session: string, runId?: string) {
  return `/api/v1/chats/sessions/${encodeURIComponent(session)}/subagents${runId ? `/${encodeURIComponent(runId)}` : ""}`;
}

/** The list polls summaries only. A transcript is fetched only while selected. */
export function useChatSubagents(session: string, workspace: string | null | undefined, active: boolean, ownerDudeCode?: string | null) {
  const [state, setState] = useState<{ session: string; items: ChatSubagent[]; error: boolean; loading: boolean }>({ session, items: [], error: false, loading: true });
  useEffect(() => {
    if (!session || !active || getServiceDataSource("chat") !== "api") return;
    const controller = new AbortController();
    let timer: ReturnType<typeof setTimeout>;
    async function refresh() {
      try {
        const items = await requestApi<ChatSubagent[]>(subagentEndpoint(session), {
          signal: controller.signal, queryParams: { workspaceCode: workspace },
          headers: ownerDudeCode ? { "X-Dude-Code": ownerDudeCode } : undefined,
        });
        if (!controller.signal.aborted) setState({ session, items, error: false, loading: false });
      } catch {
        if (!controller.signal.aborted) setState((previous) => ({ session, items: previous.session === session ? previous.items : [], error: true, loading: false }));
      } finally {
        if (!controller.signal.aborted) timer = setTimeout(refresh, 1500);
      }
    }
    void refresh();
    return () => { controller.abort(); clearTimeout(timer); };
  }, [session, workspace, active, ownerDudeCode]);
  const api = getServiceDataSource("chat") === "api";
  return useMemo(() => {
    if (!session || !api) return { session, items: [], error: false, loading: false };
    return state.session === session ? state : { session, items: [], error: false, loading: true };
  }, [api, session, state]);
}

export function useChatSubagentDetail(session: string, workspace: string | null | undefined, runId: string, active: boolean, ownerDudeCode?: string | null) {
  const [state, setState] = useState<{ key: string; detail: ChatSubagentDetail | null; error: boolean }>({ key: "", detail: null, error: false });
  const key = `${session}:${runId}`;
  useEffect(() => {
    if (!session || !runId || !active) return;
    const controller = new AbortController();
    let timer: ReturnType<typeof setTimeout>;
    async function refresh() {
      try {
        const detail = await requestApi<ChatSubagentDetail>(subagentEndpoint(session, runId), {
          signal: controller.signal, queryParams: { workspaceCode: workspace },
          headers: ownerDudeCode ? { "X-Dude-Code": ownerDudeCode } : undefined,
        });
        if (controller.signal.aborted) return;
        setState({ key, detail, error: false });
        if (detail.status === "Running" || detail.status === "Waiting") timer = setTimeout(refresh, 1000);
      } catch {
        if (!controller.signal.aborted) {
          setState((previous) => ({ key, detail: previous.key === key ? previous.detail : null, error: true }));
          timer = setTimeout(refresh, 3000);
        }
      }
    }
    void refresh();
    return () => { controller.abort(); clearTimeout(timer); };
  }, [session, workspace, runId, key, active, ownerDudeCode]);
  return state.key === key ? state : { key, detail: null, error: false };
}
