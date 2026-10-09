import { useEffect, useRef } from "react";
import { chatApiEndpoints } from "@/domain/chat/api";
import type { ChatSession, ChatSessionGroup } from "@/domain/chat/model";
import { requestApi } from "@/shared/api/httpClient";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

const defaultTitles = new Set(["new chat", "main agent", "新会话", "主智能体"]);
const pollIntervalMs = 1_500;
const maxPolls = 40;

export function useChatGeneratedTitle(
  session: ChatSession | undefined,
  onUpdated: (session: ChatSession) => void,
) {
  const onUpdatedRef = useRef(onUpdated);
  onUpdatedRef.current = onUpdated;
  const sessionCode = session?.sessionCode;
  const workspaceCode = session?.workspaceCode;
  const lastRunCode = session?.lastRunCode;
  const title = session?.title;
  const eligible = session?.sourceType?.toUpperCase() === "MANUAL"
    && session.status === "ACTIVE"
    && !session.taskCode
    && Boolean(lastRunCode)
    && defaultTitles.has(title?.trim().toLowerCase() ?? "");

  useEffect(() => {
    if (!eligible || !sessionCode || !workspaceCode || getServiceDataSource("chat") !== "api") return;
    const controller = new AbortController();
    let timer: ReturnType<typeof setTimeout>;
    let attempts = 0;
    const poll = async () => {
      attempts += 1;
      try {
        const groups = await requestApi<ChatSessionGroup[]>(chatApiEndpoints.listChatSessionGroups, {
          queryParams: { workspaceCode },
          signal: controller.signal,
        });
        const updated = groups.flatMap((group) => group.sessions)
          .find((candidate) => candidate.sessionCode === sessionCode);
        if (!controller.signal.aborted && updated && !defaultTitles.has(updated.title.trim().toLowerCase())) {
          onUpdatedRef.current(updated);
          return;
        }
      } catch {
        // Title refresh is independent of chat delivery; retry transient reads.
      }
      if (!controller.signal.aborted && attempts < maxPolls) timer = setTimeout(poll, pollIntervalMs);
    };
    timer = setTimeout(poll, pollIntervalMs);
    return () => {
      controller.abort();
      clearTimeout(timer);
    };
  }, [eligible, lastRunCode, sessionCode, title, workspaceCode]);
}
