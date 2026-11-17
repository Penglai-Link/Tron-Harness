import { lazy, Suspense, useEffect, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Link, useSearchParams } from "react-router";
import { Button, buttonVariants } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { useTabVisibility, type EditableTab } from "@/components/editable-tabs";
import { useRightPanelTabs, useTabPanelConfig, useTabPanelScopeId } from "@/shared/layout/content/TabPanelRegistry";
import { useTabStore } from "@/store/tabStore";
import { AgentChatWindow } from "@/domain/chat/components/AgentChatWindow";
import { ChatSubagentsPanel } from "@/domain/chat/components/ChatSubagentsPanel";
import { useChatSubagents } from "@/domain/chat/hooks/useChatSubagents";
import { useChatHistoryLabels, useChatWorkRailLabels } from "@/domain/chat";
import { createInternalChatAdapter } from "@tron/chat-internal-adapter";
import type { ChatSession, SendChatMessageInput } from "@tron/chat-core";
import { getApiAccessToken } from "@/shared/api/httpClient";
import { applyCurrentLocaleHeader } from "@/i18n";
import { useAuthSession } from "@/domain/auth";
import { useAssistantPageContext } from "@/shared/layout/assistantPageContext";
import { useAssistants } from "../hooks/useAssistants";
import {
  assistantQueryKeys,
  listAssistantEvents,
  listAssistantSessions,
  resolveAssistantSession,
  type AssistantSession,
} from "../api/assistantApi";
import { AssistantThreads } from "../components/AssistantThreads";
import { AssistantTasks } from "../components/AssistantTasks";
import { AssistantSwitcher } from "../components/AssistantSwitcher";
import "./AssistantsPage.css";

const ChatContextInspector = lazy(() => import("@/domain/chat/components/ChatContextInspector")
  .then(module => ({ default: module.ChatContextInspector })));
function SessionChat({
  session,
  readOnly,
  title,
}: {
  session: ChatSession;
  readOnly: boolean;
  title: string;
}) {
  const { t } = useTranslation("pages");
  const auth = useAuthSession();
  const getPageContext = useAssistantPageContext();
  const workRailLabels = useChatWorkRailLabels();
  const historyLabels = useChatHistoryLabels();
  const adapter = useMemo(
    () =>
      createInternalChatAdapter({
        workspaceCode: session.workspaceCode,
        getAccessToken: getApiAccessToken,
        getHeaders: () => {
          const headers = applyCurrentLocaleHeader(new Headers());
          if (session.ownerAssistantCode)
            headers.set("X-Assistant-Code", session.ownerAssistantCode);
          return headers;
        },
      }),
    [session.workspaceCode, session.ownerAssistantCode]
  );
  const config = useMemo(
    () => ({
      agentCode: session.agentCode ?? undefined,
      disabledInput: readOnly,
      showSessionList: false,
      showNewSession: false,
      floating: false,
      title,
      workRailLabels,
      historyLabels,
      inputPlaceholder: t("assistant.messagePlaceholder"),
    }),
    [session.agentCode, readOnly, title, workRailLabels, historyLabels, t]
  );
  const createSendInput = (
    current: ChatSession,
    content: string,
    attachments: NonNullable<SendChatMessageInput["attachments"]>
  ): SendChatMessageInput => {
    const pageContext = getPageContext();
    return {
      content,
      workspaceCode: pageContext?.workspaceCode ?? null,
      pageContext,
      agentCode: current.agentCode,
      attachments,
      clientCapabilities: { userInputRequests: true },
      modelSelection: { kind: "Auto" },
    };
  };
  return (
    <div className="assistant-session-chat flex h-full min-h-0 min-w-0 flex-col">
      <AgentChatWindow
        showHeader={false}
        adapter={adapter}
        config={config}
        initialSession={session}
        currentUserDisplayName={auth.session?.user.displayName || auth.session?.user.username || ""}
        createSendInput={createSendInput}
      />
    </div>
  );
}
export function AssistantsPage() {
  const { t, i18n } = useTranslation("pages");
  const [search, setSearch] = useSearchParams();
  const visible = useTabVisibility();
  const scopeTabId = useTabPanelScopeId();
  const rightPanelOpen = useTabStore((state) => state.tabs.find((item) => item.id === scopeTabId)?.rightPanelOpen ?? false);
  const workRailLabels = useChatWorkRailLabels();
  const historyLabels = useChatHistoryLabels();
  const assistants = useAssistants();
  const code =
    search.get("assistant") ??
    assistants.data?.find((assistant) => assistant.isMain)?.assistantCode ??
    assistants.data?.[0]?.assistantCode ??
    "";
  const assistant = assistants.data?.find((item) => item.assistantCode === code);
  const tab = ["conversation", "work", "tasks", "history"].includes(
    search.get("tab") ?? ""
  )
    ? search.get("tab")!
    : "conversation";
  const [selectedHistory, setSelectedHistory] = useState<AssistantSession | null>(
    null
  );
  const sessions = useQuery({
    queryKey: assistantQueryKeys.sessions(code),
    queryFn: ({ signal }) => listAssistantSessions(code, signal),
    enabled: visible && Boolean(code),
    refetchInterval: visible ? 15000 : false,
  });
  const interaction = useQuery({
    queryKey: ["assistant-interaction", code],
    queryFn: () => resolveAssistantSession(code),
    enabled: visible && Boolean(code) && tab === "conversation",
    staleTime: Infinity,
  });
  const events = useQuery({
    queryKey: ["assistant-events", code],
    queryFn: ({ signal }) => listAssistantEvents(code, signal),
    enabled: visible && Boolean(code) && tab === "work",
    refetchInterval: visible && tab === "work" ? 5000 : false,
  });
  const candidateSession = tab === "conversation" ? interaction.data
    : tab === "history" ? selectedHistory : null;
  const inspectedSession = candidateSession?.ownerAssistantCode === code ? candidateSession : null;
  useRightPanelTabs(inspectedSession?.sessionCode);
  // Discover child runs before their tab exists, using the same PersonAI identity
  // as the parent transcript. The panel reuses this query instead of polling twice.
  const subagents = useChatSubagents(
    inspectedSession?.sessionCode ?? "",
    inspectedSession?.workspaceCode,
    visible && rightPanelOpen,
    inspectedSession?.ownerAssistantCode,
  );
  const rightPanelTabs = useMemo<EditableTab[]>(() => {
    if (!inspectedSession) return [];
    const tabs: EditableTab[] = [{
      key: "context",
      label: t("chat.shortTermMemory.shortTermMemoryTab"),
      closable: false,
      children: <Suspense fallback={null}><ChatContextInspector
        key={`${inspectedSession.ownerAssistantCode}:${inspectedSession.sessionCode}`}
        session={inspectedSession}
      /></Suspense>,
    }];
    if (subagents.items.length > 0 || subagents.error) {
      tabs.push({
        key: "subagents", label: t("chat.subagents.title"), closable: false,
        children: <ChatSubagentsPanel
          key={`${inspectedSession.ownerAssistantCode}:${inspectedSession.sessionCode}`}
          sessionCode={inspectedSession.sessionCode}
          workspaceCode={inspectedSession.workspaceCode}
          ownerAssistantCode={inspectedSession.ownerAssistantCode}
          data={subagents}
          workRailLabels={workRailLabels}
          historyLabels={historyLabels}
        />,
      });
    }
    return tabs;
  }, [inspectedSession, subagents, workRailLabels, historyLabels, t]);
  useTabPanelConfig({ rightPanelTabs, rightPanelBrowserOnly: false });
  function selectTab(next: string) {
    setSearch((current) => {
      current.set("assistant", code);
      current.set("tab", next);
      current.delete("session");
      current.delete("task");
      return current;
    });
  }
  function openSession(sessionCode: string) {
    const session = sessions.data?.find(
      (item) => item.sessionCode === sessionCode
    );
    if (session) setSelectedHistory(session);
    setSearch((current) => {
      current.set("session", sessionCode);
      current.set("assistant", code);
      current.set("tab", "history");
      return current;
    });
  }
  useEffect(() => {
    const requested = search.get("session");
    const found = requested
      ? sessions.data?.find((item) => item.sessionCode === requested)
      : undefined;
    setSelectedHistory(found ?? null);
  }, [search, sessions.data, code]);
  const date = (value: string | number) =>
    new Intl.DateTimeFormat(i18n.language, {
      dateStyle: "medium",
      timeStyle: "short",
    }).format(new Date(value));
  return (
    <section
      aria-label={assistant?.name ?? t("assistant.title")}
      data-theme-anchor="card"
      className="flex h-full min-h-0 min-w-0 flex-col overflow-hidden rounded-[var(--tron-card-radius,8px)] border border-[var(--tron-card-border,var(--color-card-border))] bg-[var(--tron-card-bg,var(--color-elevated-surface))] text-[var(--tron-card-fg,var(--color-on-surface))] p-3 shadow-[var(--tron-card-shadow,0_18px_50px_rgba(0,0,0,0.04))] lg:p-4"
    >
      <header className="mb-2 flex shrink-0 flex-wrap items-center gap-2">
        <AssistantSwitcher assistants={assistants.data ?? []} value={code} onValueChange={(next) => {
          if (next === code) return;
          setSelectedHistory(null);
          setSearch({ assistant: next, tab: "conversation" });
        }} />
      </header>
      {assistants.isPending ? <p role="status">{t("assistant.loading")}</p> : null}
      {assistants.error ? <div role="alert">
        {assistants.error.message}
        <Button size="sm" onClick={() => void assistants.refetch()}>{t("assistant.retry")}</Button>
      </div> : null}
      {assistants.data?.length === 0 ? <p>{t("assistant.empty")}</p> : null}
      {assistant ? (
          <Tabs
            className="min-h-0 flex-1"
            value={tab}
            onValueChange={(value) => {
              if (typeof value === "string") selectTab(value);
            }}
          >
            <TabsList
              variant="line"
              className="max-w-full shrink-0 self-stretch justify-start border-b border-[var(--color-outline-variant)]"
            >
              {["conversation", "work", "tasks", "history"].map((value) => (
                <TabsTrigger
                  className="min-w-0 flex-1 px-1 text-xs sm:flex-none sm:px-3 sm:text-sm"
                  key={value}
                  value={value}
                >
                  {t(`assistant.tabs.${value}`)}
                </TabsTrigger>
              ))}
            </TabsList>
            <TabsContent
              className="min-h-0 overflow-hidden pb-3"
              value="conversation"
            >
              {interaction.isPending ? (
                <p role="status">{t("assistant.loading")}</p>
              ) : interaction.error ? (
                <div role="alert">
                  {interaction.error.message}
                  <Button size="sm" onClick={() => void interaction.refetch()}>
                    {t("assistant.retry")}
                  </Button>
                </div>
              ) : interaction.data ? (
                <SessionChat
                  key={interaction.data.sessionCode}
                  session={interaction.data}
                  readOnly={false}
                  title={assistant.name}
                />
              ) : null}
            </TabsContent>
            <TabsContent className="min-h-0 overflow-y-auto pb-4" value="work">
              <AssistantThreads
                key={code}
                assistantCode={code}
                active={visible && tab === "work"}
                requestedThread={search.get("session")}
              />
              <h2 className="mt-6 border-t border-[var(--color-outline-variant)] pt-4 font-medium">
                {t("assistant.activity")}
              </h2>
              <div className="divide-y divide-[var(--color-outline-variant)]">
                {events.isPending ? (
                  <p role="status">{t("assistant.loading")}</p>
                ) : null}
                {events.error ? (
                  <div role="alert">
                    {events.error.message}
                    <Button size="sm" onClick={() => void events.refetch()}>
                      {t("assistant.retry")}
                    </Button>
                  </div>
                ) : null}
                {events.data?.map((event) => (
                  <article
                    key={event.eventId}
                    className="flex flex-wrap items-center gap-3 py-3"
                  >
                    <div className="min-w-0 flex-1">
                      <span className="font-medium">
                        {t(`assistant.sources.${event.source}`, {
                          defaultValue: event.source,
                        })}
                      </span>
                      <p className="mt-1 text-xs text-[var(--color-on-surface-variant)]">
                        {date(event.createdAt)}
                      </p>
                      {event.result ? (
                        <details className="mt-2 text-sm">
                          <summary className="cursor-pointer text-[var(--color-on-surface-variant)]">
                            {t("assistant.viewResult")}
                          </summary>
                          <p className="mt-2 whitespace-pre-wrap break-words">
                            {event.result}
                          </p>
                        </details>
                      ) : null}
                    </div>
                    <Badge
                      variant={event.status === "Failed" ? "error" : "outline"}
                    >
                      {t(`assistant.eventStatuses.${event.status}`, {
                        defaultValue: event.status,
                      })}
                    </Badge>
                    {event.sessionCode ? (
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={() => openSession(event.sessionCode!)}
                      >
                        {t("assistant.viewWork")}
                      </Button>
                    ) : null}
                  </article>
                ))}
              </div>
              {events.data?.length === 0 ? (
                <p className="py-4 text-sm text-[var(--color-on-surface-variant)]">
                  {t("assistant.noWork")}
                </p>
              ) : null}
            </TabsContent>
            <TabsContent className="min-h-0 overflow-y-auto pb-4" value="tasks">
              <AssistantTasks
                key={code}
                assistantCode={code}
                active={visible && tab === "tasks"}
                onOpenSession={openSession}
                highlightedTask={search.get("task")}
              />
            </TabsContent>
            <TabsContent
              className="min-h-0 overflow-y-auto pb-4"
              value="history"
            >
              {selectedHistory ? (
                <div className="flex h-full min-h-0 flex-col">
                  <div className="mb-2 flex shrink-0 items-center justify-end gap-2">
                    <Button
                      size="sm"
                      variant="outline"
                      onClick={() => {
                        setSelectedHistory(null);
                        setSearch((current) => {
                          current.delete("session");
                          return current;
                        });
                      }}
                    >
                      {t("assistant.backToHistory")}
                    </Button>
                    <Button
                      size="sm"
                      variant="outline"
                      nativeButton={false}
                      render={<Link to={`/settings/observability/traces?category=Agent&sessionCode=${encodeURIComponent(selectedHistory.sessionCode)}`} />}
                    >
                      {t("assistant.viewTraces")}
                    </Button>
                  </div>
                  <SessionChat
                    key={selectedHistory.sessionCode}
                    session={selectedHistory}
                    readOnly={selectedHistory.purpose !== "Interactive"}
                    title={selectedHistory.title}
                  />
                </div>
              ) : (
                <div className="divide-y divide-[var(--color-outline-variant)]">
                  {sessions.isPending ? (
                    <p role="status">{t("assistant.loading")}</p>
                  ) : null}
                  {sessions.error ? (
                    <div role="alert">
                      {sessions.error.message}
                      <Button size="sm" onClick={() => void sessions.refetch()}>
                        {t("assistant.retry")}
                      </Button>
                    </div>
                  ) : null}
                  {sessions.data?.map((session) => (
                    <div
                      key={session.sessionCode}
                      className="flex items-center gap-3 py-3"
                    >
                      <div className="min-w-0 flex-1">
                        <span className="block truncate font-medium">
                          {session.title}
                        </span>
                        <span className="text-xs text-[var(--color-on-surface-variant)]">
                          {t(`assistant.purposes.${session.purpose}`, {
                            defaultValue: session.purpose,
                          })}{" "}
                          · {date(session.updatedAt)}
                        </span>
                      </div>
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={() => openSession(session.sessionCode)}
                      >
                        {t("assistant.open")}
                      </Button>
                    </div>
                  ))}
                </div>
              )}
            </TabsContent>
          </Tabs>
      ) : null}
    </section>
  );
}
