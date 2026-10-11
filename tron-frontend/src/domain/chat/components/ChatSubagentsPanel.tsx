import { ShinyText } from "@/shared/motion";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { historyText } from "@eliteforge/agent-ui-chat-core";
import { Button } from "@/components/ui/button";
import { MarkdownContent } from "@/components/markdown-content";
import { useTabVisibility } from "@/components/editable-tabs/TabVisibilityContext";
import { useChatSubagentDetail, useChatSubagents } from "../hooks/useChatSubagents";
import type { ChatSubagent, ChatSubagentDetail } from "../model/ChatSubagent";
import { SubagentAvatar } from "./SubagentAvatar";

export function ChatSubagentsPanel({ sessionCode, workspaceCode }: { sessionCode: string; workspaceCode: string }) {
  const { t } = useTranslation("pages");
  const visible = useTabVisibility();
  const { items, loading, error } = useChatSubagents(sessionCode, workspaceCode, visible);
  const [selected, setSelected] = useState("");
  const item = items.find((item) => item.runId === selected);
  if (item) return <SubagentSession key={`${sessionCode}:${selected}`} parent={sessionCode} workspace={workspaceCode} item={item} onBack={() => setSelected("")} />;
  return <section className="h-full overflow-y-auto p-4" aria-label={t("chat.subagents.title")}>
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

        </Button>
      </li>)}
    </ul>
  </section>;
}

function SubagentSession({ parent, workspace, item, onBack }: { parent: string; workspace: string; item: ChatSubagent; onBack: () => void }) {
  const { t } = useTranslation("pages");
  const visible = useTabVisibility();
  const { detail, error } = useChatSubagentDetail(parent, workspace, item.runId, visible);
  return <section className="flex h-full min-h-0 flex-col" aria-label={t("chat.subagents.session")}>
    <header className="flex items-center gap-2 border-b border-[var(--color-outline-variant)]/40 px-3 py-2">
      <Button size="sm" variant="ghost" aria-label={t("chat.subagents.back")} onClick={onBack}>{t("chat.subagents.back")}</Button>
      <SubagentAvatar avatarId={item.avatarId} name={item.agentName} status={detail?.status ?? item.status} />
      <span className="min-w-0 flex-1 truncate text-sm font-medium">{item.agentName}</span>
    </header>
    <div className="flex-1 space-y-5 overflow-y-auto p-4">
      <section><h3 className="mb-1 text-xs font-medium text-[var(--color-on-surface-variant)]">{t("chat.subagents.task")}</h3><MarkdownContent content={item.task} /></section>
      {error ? <p role="alert" className="text-sm text-[var(--color-error)]">{t("chat.subagents.loadFailed")}</p> : null}
      {!detail && !error ? <p role="status" className="text-sm">{t("chat.subagents.loading")}</p> : null}
      {detail ? <SubagentTranscript detail={detail} /> : null}
    </div>
  </section>;
}

function SubagentTranscript({ detail }: { detail: ChatSubagentDetail }) {
  const { t } = useTranslation("pages");
  const liveText = useMemo(() => {
    // This stream belongs to the selected temporary session only.
    const texts = new Map<string, string>();
    for (const event of detail.events) {
      if (event.type === "TEXT_MESSAGE_CONTENT" && typeof event.messageId === "string" && typeof event.delta === "string") {
        texts.set(event.messageId, (texts.get(event.messageId) ?? "") + event.delta);
      }
    }
    return [...texts.values()].at(-1) ?? "";
  }, [detail.events]);
  const messages = detail.records.messages.filter((message) => message.role === "Assistant");
  const committedText = messages.map((message) => historyText(message.content));
  const showLive = detail.status === "Running" && liveText && !committedText.includes(liveText);
  return <div className="space-y-5" data-subagent-session={detail.sessionId}>
    {messages.map((message) => {
      const reasoning = message.content.filter((block) => block.type === "Reasoning").map((block) => "text" in block ? block.text : "").join("");
      const tools = detail.records.toolInvocations.filter((tool) => tool.messageId === message.id);
      return <section key={message.id} className="min-w-0 space-y-2">
        {reasoning ? <details className="text-sm text-[var(--color-on-surface-variant)]"><summary className="cursor-pointer py-1">{t("chat.subagents.reasoning")}</summary><MarkdownContent content={reasoning} /></details> : null}
        {historyText(message.content) ? <MarkdownContent content={historyText(message.content)} /> : null}
        {tools.map((tool) => <details key={tool.id} className="text-sm">
          <summary className="cursor-pointer py-1"><span>{tool.toolName}</span><span className="ml-2 text-xs text-[var(--color-on-surface-variant)]">{t(`chat.subagents.statuses.${tool.status}`)}</span></summary>
          {historyText(tool.content) ? <MarkdownContent content={historyText(tool.content)} /> : null}
        </details>)}
      </section>;
    })}
    {showLive ? <MarkdownContent content={liveText} /> : null}
    {detail.status === "Waiting" ? <p role="status" className="text-sm text-[var(--color-on-surface-variant)]">{t("chat.subagents.waiting")}</p> : null}
  </div>;
}
