import { useState } from "react";
import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { historyMessages, type HistoryCursor, type SessionHistory } from "@eliteforge/agent-ui-chat-core";
import { ChatPanel } from "@eliteforge/agent-ui-chat-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { useChatHistoryLabels, useChatWorkRailLabels } from "@/domain/chat";
import { useAuthSession } from "@/domain/auth";
import { getDudeThreadHistory, listDudeThreads, type DudeThread } from "../api/dudeApi";
function uniqueById<T extends { id: string }>(values: T[]): T[] { return [...new Map(values.map(value => [value.id, value])).values()]; }
function mergePages(pages: SessionHistory[]): SessionHistory | null {
  if (!pages.length) return null;
  return { ...pages[0], runs: uniqueById(pages.flatMap(page => page.runs)), records: {
    messages: uniqueById(pages.flatMap(page => page.records.messages)).sort((a, b) => a.createdAtMs - b.createdAtMs),
    steps: uniqueById(pages.flatMap(page => page.records.steps)),
    toolInvocations: uniqueById(pages.flatMap(page => page.records.toolInvocations)),
    interactions: uniqueById(pages.flatMap(page => page.records.interactions)),
  } };
}
export function DudeThreads({ dudeCode, active, requestedThread }: { dudeCode: string; active: boolean; requestedThread?: string | null }) {
  const { t } = useTranslation("pages"); const [selected, setSelected] = useState<string | null>(requestedThread ?? null);
  const query = useQuery({ queryKey: ["dude-threads", dudeCode], queryFn: ({ signal }) => listDudeThreads(dudeCode, signal), enabled: active, refetchInterval: active ? 5000 : false });
  const thread = query.data?.find(item => item.threadCode === selected);
  return <section className="space-y-3" aria-label={t("dude.threads")}><h2 className="font-medium">{t("dude.threads")}</h2>
    {query.isPending ? <p role="status">{t("dude.loading")}</p> : null}{query.error ? <div role="alert">{query.error.message}<Button size="sm" variant="outline" onClick={() => void query.refetch()}>{t("dude.retry")}</Button></div> : null}
    {thread ? <><div className="flex items-center justify-between gap-3"><span className="truncate text-sm">{thread.title ?? thread.threadCode}</span><Button size="sm" variant="outline" onClick={() => setSelected(null)}>{t("dude.backToThreads")}</Button></div><DudeThreadHistory key={thread.threadCode} dudeCode={dudeCode} thread={thread} active={active} /></> : <div className="divide-y divide-[var(--color-outline-variant)]">{query.data?.map(item => <div key={item.threadCode} className="flex items-center gap-3 py-3"><span className="min-w-0 flex-1 truncate text-sm">{item.title ?? item.threadCode}</span><Badge variant="outline">{t(`dude.threadStatuses.${item.status}`, { defaultValue: item.status })}</Badge><Button size="sm" variant="outline" onClick={() => setSelected(item.threadCode)}>{t("dude.open")}</Button></div>)}{query.data?.length === 0 ? <p className="py-2 text-sm text-[var(--color-on-surface-variant)]">{t("dude.noThreads")}</p> : null}</div>}
  </section>;
}
function DudeThreadHistory({ dudeCode, thread, active }: { dudeCode: string; thread: DudeThread; active: boolean }) {
  const { t } = useTranslation("pages"); const auth = useAuthSession(); const historyLabels = useChatHistoryLabels(); const workRailLabels = useChatWorkRailLabels();
  const query = useInfiniteQuery({ queryKey: ["dude-thread-history", dudeCode, thread.threadCode], initialPageParam: null as HistoryCursor | null,
    queryFn: ({ signal, pageParam }) => getDudeThreadHistory(dudeCode, thread.threadCode, { limit: 100, cursor: pageParam }, signal), getNextPageParam: page => page.nextCursor ?? undefined,
    enabled: active, refetchInterval: active && (thread.status === "Running" || thread.status === "AwaitingInput") ? 1500 : false });
  const history = mergePages(query.data?.pages ?? []); const messages = history ? historyMessages(history) : [];
  return <div className="space-y-3">{query.error ? <div role="alert">{query.error.message}<Button size="sm" variant="outline" onClick={() => void query.refetch()}>{t("dude.retry")}</Button></div> : null}
    <div className="h-[min(65dvh,800px)] min-h-80"><ChatPanel disabledInput draft="" onDraftChange={() => undefined} onSend={() => undefined} currentUserDisplayName={auth.session?.user.displayName ?? ""} messages={messages} historyLabels={historyLabels} workRailLabels={workRailLabels} historyLoading={query.isPending} historyLoadingOlder={query.isFetchingNextPage} onLoadOlderHistory={query.hasNextPage ? () => void query.fetchNextPage() : undefined} showStop={false} showHeader={false} title={thread.title ?? thread.threadCode}
      session={{ id: thread.threadCode, sessionId: history?.sessionId ?? thread.threadCode, title: thread.title ?? thread.threadCode, status: "ACTIVE", createdAt: history?.runs[0] ? new Date(history.runs[0].startedAtMs).toISOString() : "", updatedAt: "" }} />
    </div><div className="flex justify-end"><Button size="sm" variant="ghost" onClick={() => void query.refetch()}>{t("dude.refresh")}</Button></div>
  </div>;
}
