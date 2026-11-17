import { ArrowLeft, ChevronRight } from "lucide-react";
import { ShinyText } from "@/shared/motion";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { historyMessages, type ChatWorkRailLabels, type ChatHistoryLabels, type SessionHistory } from "@personai/agent-ui-chat-core";
import { ChatPanel } from "@personai/agent-ui-chat-react";
import { useChatHistoryLabels } from "../hooks/useChatWorkRailLabels";
import { Button } from "@/components/ui/button";
import { MarkdownContent } from "@/components/markdown-content";
import { useTabVisibility } from "@/components/editable-tabs/TabVisibilityContext";
import { useChatSubagentDetail, useChatSubagents } from "../hooks/useChatSubagents";
import type { ChatSubagent, ChatSubagentDetail } from "../model/ChatSubagent";
import { SubagentAvatar } from "./SubagentAvatar";
import { useAguiWorkReplayPreferences } from "@/domain/settings/model/aguiWorkReplayPreferences";

export function ChatSubagentsPanel({ sessionCode, workspaceCode, ownerAssistantCode, data, workRailLabels, historyLabels }: {
  sessionCode: string;
  workspaceCode?: string | null;
  ownerAssistantCode?: string | null;
  data?: ReturnType<typeof useChatSubagents>;
  workRailLabels?: ChatWorkRailLabels;
  historyLabels?: ChatHistoryLabels;
}) {
  const { t } = useTranslation("pages");
  const visible = useTabVisibility();
  const queried = useChatSubagents(sessionCode, workspaceCode, visible && !data, ownerAssistantCode);
  const { items, loading, error } = data ?? queried;
  const [selected, setSelected] = useState("");
  const item = items.find((item) => item.runId === selected);
  if (item) return <SubagentSession key={`${sessionCode}:${selected}`} parent={sessionCode} workspace={workspaceCode} ownerAssistantCode={ownerAssistantCode} item={item} workRailLabels={workRailLabels} historyLabels={historyLabels} onBack={() => setSelected("")} />;
  return <section className="tron-scrollbar h-full min-h-0 overflow-y-auto p-4" aria-label={t("chat.subagents.title")}>
    {error ? <p role="alert" className="py-3 text-sm text-[var(--color-error)]">{t("chat.subagents.loadFailed")}</p> : null}
    {loading ? <p role="status" className="py-3 text-sm text-[var(--color-on-surface-variant)]"><ShinyText>{t("chat.subagents.loading")}</ShinyText></p> : null}
    {!loading && !error && !items.length ? <p className="py-3 text-sm text-[var(--color-on-surface-variant)]">{t("chat.subagents.empty")}</p> : null}
    <ul className="divide-y divide-[var(--color-outline-variant)]/40">
      {items.map((item) => <li key={item.runId}>
        <Button variant="ghost" className="h-auto w-full justify-start gap-3 rounded-md px-2 py-3 text-left" onClick={() => setSelected(item.runId)}>
          <SubagentAvatar avatarId={item.avatarId} name={item.agentName} status={item.status} />
          <span className="min-w-0 flex-1">
            <span className="block truncate font-medium">{item.agentName}</span>
            <span className="mt-1 line-clamp-2 whitespace-normal text-xs font-normal text-[var(--color-on-surface-variant)]">{item.task}</span>
          </span>
          <ChevronRight aria-hidden className="size-4 shrink-0" />
        </Button>
      </li>)}
    </ul>
  </section>;
}

function SubagentSession({ parent, workspace, ownerAssistantCode, item, onBack, workRailLabels, historyLabels }: {
  parent: string; workspace?: string | null; ownerAssistantCode?: string | null; item: ChatSubagent; onBack: () => void;
  workRailLabels?: ChatWorkRailLabels; historyLabels?: ChatHistoryLabels;
}) {
  const { t } = useTranslation("pages");
  const visible = useTabVisibility();
  const { detail, error } = useChatSubagentDetail(parent, workspace, item.runId, visible, ownerAssistantCode);
  return <section className="flex h-full min-h-0 min-w-0 flex-col overflow-hidden" aria-label={t("chat.subagents.session")}>
    <header className="flex shrink-0 items-center gap-2 border-b border-[var(--color-outline-variant)]/40 px-3 py-2">
      <Button size="icon" variant="ghost" aria-label={t("chat.subagents.back")} onClick={onBack}><ArrowLeft aria-hidden className="size-4" /></Button>
      <SubagentAvatar avatarId={item.avatarId} name={item.agentName} status={detail?.status ?? item.status} />
      <span className="min-w-0 flex-1 truncate text-sm font-medium">{item.agentName}</span>
    </header>
    <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
      <section className="tron-scrollbar max-h-40 shrink-0 overflow-y-auto px-4 py-3">
        <h3 className="mb-1 text-xs font-medium text-[var(--color-on-surface-variant)]">{t("chat.subagents.task")}</h3>
        <MarkdownContent content={item.task} />
      </section>
      {error ? <p role="alert" className="text-sm text-[var(--color-error)]">{t("chat.subagents.loadFailed")}</p> : null}
      {!detail && !error ? <p role="status" className="text-sm">{t("chat.subagents.loading")}</p> : null}
      {detail ? <SubagentTranscript detail={detail} item={item} workRailLabels={workRailLabels} historyLabels={historyLabels} /> : null}
    </div>
  </section>;
}

const ignoreInput = () => {};

function SubagentTranscript({ detail, item, workRailLabels, historyLabels }: {
  detail: ChatSubagentDetail; item: ChatSubagent;
  workRailLabels?: ChatWorkRailLabels; historyLabels?: ChatHistoryLabels;
}) {
  const { i18n, t } = useTranslation("pages");
  const fallbackHistoryLabels = useChatHistoryLabels();
  const { preferences: workPreferences } = useAguiWorkReplayPreferences();
  const messages = useMemo(() => {
    const run = {
      id: detail.runId, sessionId: detail.sessionId, parentRunId: null, rootRunId: detail.runId,
      status: detail.status, startedAtMs: item.startedAtMs, finishedAtMs: item.finishedAtMs, error: null,
    };
    // Native run envelopes carry transport bookkeeping, not user-authored
    // structured content. Keep genuine structured messages in the shared view.
    const records = { ...detail.records, messages: detail.records.messages.filter(message => message.role !== "User").map(message => {
      const value = message.structured as { runCode?: unknown; observations?: unknown } | null;
      return value?.runCode === detail.runId && Array.isArray(value.observations)
        ? { ...message, structured: null } : message;
    }) };
    const history: SessionHistory = {
      sessionId: detail.sessionId, version: detail.runId, nextCursor: null,
      runs: [run], records, pendingInteractions: [],
      activeRuns: detail.status === "Running" || detail.status === "Waiting" ? [run] : [],
    };
    return historyMessages(history).map(message => message.role === "ASSISTANT" ? { ...message, actor: item.agentName } : message);
  }, [detail, item]);
  const session = useMemo(() => ({
    id: detail.sessionId, sessionId: detail.sessionId, title: item.agentName, status: "ACTIVE" as const,
    createdAt: new Date(item.startedAtMs).toISOString(), updatedAt: new Date(item.finishedAtMs ?? item.startedAtMs).toISOString(),
  }), [detail.sessionId, item]);
  return <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden" data-subagent-session={detail.sessionId}>
    <ChatPanel session={session} messages={messages} protocolEvents={detail.events}
      locale={i18n.language} workRailLabels={workRailLabels} historyLabels={historyLabels ?? fallbackHistoryLabels}
      showReasoningCards={workPreferences.showReasoningCards}
      showHeader={false} showHistoryRail={false} showComposer={false} showStop={false} a2uiDisabled
      draft="" onDraftChange={ignoreInput} onSend={ignoreInput} />
    {detail.status === "Waiting" ? <p role="status" className="shrink-0 px-4 py-2 text-sm text-[var(--color-on-surface-variant)]">{t("chat.subagents.waiting")}</p> : null}
  </div>;
}
