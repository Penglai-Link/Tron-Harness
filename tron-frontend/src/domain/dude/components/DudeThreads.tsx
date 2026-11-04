import { Link } from "react-router";
import { useEffect, useState } from "react";
import { useInfiniteQuery, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { historyMessages, historyUserInputRequests, nativeUserInputBatchAnswer, pendingApprovalRequests, type ChatUserInputAnswer, type ChatUserInputLabels, type HistoryCursor, type SessionHistory } from "@eliteforge/agent-ui-chat-core";
import { ChatPanel } from "@eliteforge/agent-ui-chat-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { useChatHistoryLabels, useChatWorkRailLabels } from "@/domain/chat";
import { useAuthSession } from "@/domain/auth";
import { answerDudeThreadInteraction, getDudeThreadHistory, listDudeThreads, sendDudeThreadMessage, type DudeThread } from "../api/dudeApi";
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
  useEffect(() => { setSelected(requestedThread ?? null); }, [requestedThread]);
  const thread = query.data?.find(item => item.sessionCode === selected);
  return <section className="space-y-3" aria-label={t("dude.threads")}><h2 className="font-medium">{t("dude.threads")}</h2>
    {query.isPending ? <p role="status">{t("dude.loading")}</p> : null}{query.error ? <div role="alert">{query.error.message}<Button size="sm" variant="outline" onClick={() => void query.refetch()}>{t("dude.retry")}</Button></div> : null}
    {thread ? <><div className="flex items-center justify-between gap-3"><span className="truncate text-sm">{thread.title ?? thread.sessionCode}</span><Button size="sm" variant="outline" onClick={() => setSelected(null)}>{t("dude.backToThreads")}</Button></div><DudeThreadHistory key={thread.sessionCode} dudeCode={dudeCode} thread={thread} active={active} /></> : <div className="divide-y divide-[var(--color-outline-variant)]">{query.data?.map(item => <div key={item.sessionCode} className="flex items-center gap-3 py-3"><span className="min-w-0 flex-1 truncate text-sm">{item.title ?? item.sessionCode}</span><Badge variant="outline">{t(`dude.threadStatuses.${item.status}`, { defaultValue: item.status })}</Badge><Button size="sm" variant="outline" onClick={() => setSelected(item.sessionCode)}>{t("dude.open")}</Button></div>)}{query.data?.length === 0 ? <p className="py-2 text-sm text-[var(--color-on-surface-variant)]">{t("dude.noThreads")}</p> : null}</div>}
  </section>;
}
function DudeThreadHistory({ dudeCode, thread, active }: { dudeCode: string; thread: DudeThread; active: boolean }) {
  const { t } = useTranslation("pages"); const client = useQueryClient(); const [draft, setDraft] = useState(""); const [busy, setBusy] = useState(false); const [actionError, setActionError] = useState(""); const auth = useAuthSession(); const historyLabels = useChatHistoryLabels(); const workRailLabels = useChatWorkRailLabels();
  const query = useInfiniteQuery({ queryKey: ["dude-thread-history", dudeCode, thread.sessionCode], initialPageParam: null as HistoryCursor | null,
    queryFn: ({ signal, pageParam }) => getDudeThreadHistory(dudeCode, thread.sessionCode, { limit: 100, cursor: pageParam }, signal), getNextPageParam: page => page.nextCursor ?? undefined,
    enabled: active, refetchInterval: active && (busy || thread.status === "Running" || thread.status === "AwaitingInput") ? 1500 : false });
  const history = mergePages(query.data?.pages ?? []); const messages = history ? historyMessages(history) : [];
  const userInputLabels: ChatUserInputLabels = {
    ariaLabel: t("chat.userInput.ariaLabel"), progress: (current, total) => t("chat.userInput.progress", { current, total }),
    back: t("chat.userInput.back"), next: t("chat.userInput.next"), continue: t("chat.userInput.continue"),
    approvalTitle: t("chat.userInput.approvalTitle"),
    approvalTool: t("chat.userInput.approvalTool"),
    approvalExecution: t("chat.userInput.approvalExecution"),
    approvalReason: t("chat.userInput.approvalReason"),
    approvalDirectory: t("chat.userInput.approvalDirectory"),
    approvalParameters: t("chat.userInput.approvalParameters"),
    approvalMissingDetails: t("chat.userInput.approvalMissingDetails"),
    approvalScope: t("chat.userInput.approvalScope"),
    approvalBatch: t("chat.userInput.approvalBatch"),
    approvalBatchHint: t("chat.userInput.approvalBatchHint"),
    approvalWaiting: t("chat.userInput.approvalWaiting"),
    approvalRetry: t("chat.userInput.approvalRetry"),
    approvalChange: t("chat.userInput.approvalChange"),
    approved: t("chat.userInput.approved"),
    rejected: t("chat.userInput.rejected"),
    approve: t("chat.userInput.approve"), reject: t("chat.userInput.reject"), continuing: t("chat.userInput.continuing"),
    otherOptionLabel: t("chat.userInput.otherOptionLabel"), otherOptionDescription: t("chat.userInput.otherOptionDescription"),
    skipOptionLabel: t("chat.userInput.skipOptionLabel"), skipOptionDescription: t("chat.userInput.skipOptionDescription"),
    otherAnswerPlaceholder: t("chat.userInput.otherAnswerPlaceholder"), otherAnswerAriaLabel: index => t("chat.userInput.otherAnswerAriaLabel", { index }),
  };
  const requests = history ? historyUserInputRequests(history, userInputLabels) : [];
  async function perform(action: () => Promise<unknown>) {
    if (busy) return;
    setBusy(true); setActionError("");
    try { await action(); }
    catch (error) { setActionError(error instanceof Error ? error.message : String(error)); }
    finally { setBusy(false); await Promise.all([query.refetch(), client.invalidateQueries({ queryKey: ["dude-threads", dudeCode] })]); }
  }
  function send() {
    const content = draft.trim(); if (!content) return;
    void perform(async () => { await sendDudeThreadMessage(dudeCode, thread.sessionCode, content); setDraft(""); });
  }
  function answer(answers: ChatUserInputAnswer[]) {
    const request = requests[0]; if (!request) return;
    void perform(async () => {
      const entries = nativeUserInputBatchAnswer(pendingApprovalRequests(request, requests), answers).resume;
      if (!entries?.length) throw new Error(t("chat.userInput.ariaLabel"));
      await answerDudeThreadInteraction(dudeCode, thread.sessionCode, entries.map(entry => ({interactionId:entry.interruptId, value:entry.payload})));
    });
  }
  return <div className="space-y-3">{query.error ? <div role="alert">{query.error.message}<Button size="sm" variant="outline" onClick={() => void query.refetch()}>{t("dude.retry")}</Button></div> : null}
    {actionError ? <p role="alert">{actionError}</p> : null}
    <div className="flex h-[min(65dvh,800px)] min-h-80 flex-col"><ChatPanel disabledInput={query.isPending || busy || thread.status === "Running" || requests.length > 0} draft={draft} onDraftChange={setDraft} onSend={send} sending={busy} sendDisabled={!draft.trim()} userInputRequest={requests[0] ?? null} userInputRequests={requests} userInputLabels={userInputLabels} userInputAnswerPending={busy} onAnswerUserInput={answer} currentUserDisplayName={auth.session?.user.displayName || auth.session?.user.username || ""} messages={messages} historyLabels={historyLabels} workRailLabels={workRailLabels} historyLoading={query.isPending} historyLoadingOlder={query.isFetchingNextPage} onLoadOlderHistory={query.hasNextPage ? () => void query.fetchNextPage() : undefined} showStop={false} showHeader={false} title={thread.title ?? thread.sessionCode}
      session={{ id: thread.sessionCode, sessionId: history?.sessionId ?? thread.sessionCode, title: thread.title ?? thread.sessionCode, status: "ACTIVE", createdAt: history?.runs[0] ? new Date(history.runs[0].startedAtMs).toISOString() : "", updatedAt: "" }} />
    </div><div className="flex justify-end gap-2"><Button size="sm" variant="ghost" nativeButton={false} render={<Link to={`/settings/observability/traces?category=Agent&sessionCode=${encodeURIComponent(thread.sessionCode)}`} />}>{t("dude.viewTraces")}</Button><Button size="sm" variant="ghost" onClick={() => void query.refetch()}>{t("dude.refresh")}</Button></div>
  </div>;
}
