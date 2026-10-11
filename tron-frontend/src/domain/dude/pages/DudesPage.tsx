import { useEffect, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Link, useSearchParams } from "react-router";
import { Button, buttonVariants } from "@/components/ui/button";
import { SelectInput } from "@/components/forms";
import { SelectItem } from "@/components/ui/select";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { useTabVisibility } from "@/components/editable-tabs";
import { AgentAvatar } from "@/domain/agent/components/AgentAvatarPicker";
import { AgentChatWindow } from "@/domain/chat/components/AgentChatWindow";
import { useChatHistoryLabels, useChatWorkRailLabels } from "@/domain/chat";
import { createInternalChatAdapter } from "@tron/chat-internal-adapter";
import type { ChatSession, SendChatMessageInput } from "@tron/chat-core";
import { getApiAccessToken } from "@/shared/api/httpClient";
import { applyCurrentLocaleHeader } from "@/i18n";
import { useAuthSession } from "@/domain/auth";
import { useDudes } from "../hooks/useDudes";
import {
  dudeQueryKeys,
  listDudeEvents,
  listDudeSessions,
  resolveDudeSession,
  type DudeSession,
} from "../api/dudeApi";
import { DudeThreads } from "../components/DudeThreads";
import { DudeTasks } from "../components/DudeTasks";
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
  const workRailLabels = useChatWorkRailLabels();
  const historyLabels = useChatHistoryLabels();
  const adapter = useMemo(
    () =>
      createInternalChatAdapter({
        workspaceCode: session.workspaceCode,
        getAccessToken: getApiAccessToken,
        getHeaders: () => {
          const headers = applyCurrentLocaleHeader(new Headers());
          if (session.ownerDudeCode)
            headers.set("X-Dude-Code", session.ownerDudeCode);
          return headers;
        },
      }),
    [session.workspaceCode, session.ownerDudeCode]
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
      inputPlaceholder: t("dude.messagePlaceholder"),
    }),
    [session.agentCode, readOnly, title, workRailLabels, historyLabels, t]
  );
  const createSendInput = (
    current: ChatSession,
    content: string,
    attachments: NonNullable<SendChatMessageInput["attachments"]>
  ): SendChatMessageInput => ({
    content,
    workspaceCode: current.workspaceCode,
    agentCode: current.agentCode,
    attachments,
    clientCapabilities: { userInputRequests: true },
    modelSelection: { kind: "Auto" },
  });
  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="flex shrink-0 justify-end">
        <Link
          className={buttonVariants({ size: "sm", variant: "ghost" })}
          to={`/settings/observability/traces?sessionCode=${encodeURIComponent(
            session.sessionCode
          )}`}
        >
          {t("dude.viewTraces")}
        </Link>
      </div>
      <div className="flex min-h-0 flex-1 flex-col">
        <AgentChatWindow
          showHeader={false}
          adapter={adapter}
          config={config}
          initialSession={session}
          currentUserDisplayName={auth.session?.user.displayName ?? ""}
          createSendInput={createSendInput}
        />
      </div>
    </div>
  );
}
export function DudesPage() {
  const { t, i18n } = useTranslation("pages");
  const [search, setSearch] = useSearchParams();
  const visible = useTabVisibility();
  const dudes = useDudes();
  const code =
    search.get("dude") ??
    dudes.data?.find((dude) => dude.isMain)?.dudeCode ??
    dudes.data?.[0]?.dudeCode ??
    "";
  const dude = dudes.data?.find((item) => item.dudeCode === code);
  const tab = ["conversation", "work", "tasks", "history"].includes(
    search.get("tab") ?? ""
  )
    ? search.get("tab")!
    : "conversation";
  const [selectedHistory, setSelectedHistory] = useState<DudeSession | null>(
    null
  );
  const sessions = useQuery({
    queryKey: dudeQueryKeys.sessions(code),
    queryFn: ({ signal }) => listDudeSessions(code, signal),
    enabled: visible && Boolean(code),
    refetchInterval: visible ? 15000 : false,
  });
  const interaction = useQuery({
    queryKey: ["dude-interaction", code],
    queryFn: () => resolveDudeSession(code),
    enabled: visible && Boolean(code) && tab === "conversation",
    staleTime: Infinity,
  });
  const events = useQuery({
    queryKey: ["dude-events", code],
    queryFn: ({ signal }) => listDudeEvents(code, signal),
    enabled: visible && Boolean(code) && tab === "work",
    refetchInterval: visible && tab === "work" ? 5000 : false,
  });
  function selectTab(next: string) {
    setSearch((current) => {
      current.set("dude", code);
      current.set("tab", next);
      current.delete("session");
      current.delete("thread");
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
      current.set("dude", code);
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
    <div className="flex h-full min-h-0 min-w-0 flex-col lg:flex-row">
      <aside className="shrink-0 border-b border-[var(--color-outline-variant)] p-3 lg:w-56 lg:overflow-y-auto lg:border-r lg:border-b-0 lg:p-4">
        <div className="flex items-center justify-between">
          <span className="text-sm font-medium">{t("dude.title")}</span>
          <Button
            size="sm"
            variant="ghost"
            nativeButton={false}
            render={<Link to="/settings/agents?tab=dudes" />}
          >
            {t("dude.manage")}
          </Button>
        </div>
        {dudes.isPending ? <p role="status">{t("dude.loading")}</p> : null}
        {dudes.error ? (
          <div role="alert">
            {dudes.error.message}
            <Button size="sm" onClick={() => void dudes.refetch()}>
              {t("dude.retry")}
            </Button>
          </div>
        ) : null}
        <div className="hidden space-y-1 lg:block">
          {dudes.data?.map((item) => (
            <button
              type="button"
              key={item.dudeCode}
              aria-pressed={code === item.dudeCode}
              onClick={() => {
                setSelectedHistory(null);
                setSearch({ dude: item.dudeCode, tab: "conversation" });
              }}
              className={`flex w-full items-center gap-3 rounded-md px-2 py-3 text-left hover:bg-[var(--color-surface-container)] ${
                code === item.dudeCode
                  ? "bg-[var(--color-surface-container)]"
                  : ""
              }`}
            >
              <AgentAvatar
                avatarId={item.avatarId}
                className="size-9 shrink-0 rounded-md"
              />
              <span className="min-w-0">
                <span className="block truncate text-sm font-medium">
                  {item.name}
                </span>
                <span className="block text-xs text-[var(--color-on-surface-variant)]">
                  {t(item.enabled ? "dude.running" : "dude.paused")}
                  {item.isMain ? ` · ${t("dude.main")}` : ""}
                </span>
              </span>
            </button>
          ))}
        </div>
        {dudes.data?.length ? (
          <div className="mt-2 lg:hidden">
            <SelectInput
              aria-label={t("dude.selectDude")}
              value={code}
              onValueChange={(value) => {
                if (value) setSearch({ dude: value, tab: "conversation" });
              }}
            >
              {dudes.data.map((item) => (
                <SelectItem key={item.dudeCode} value={item.dudeCode}>
                  {item.name}
                </SelectItem>
              ))}
            </SelectInput>
          </div>
        ) : null}
        {dudes.data?.length === 0 ? <p>{t("dude.empty")}</p> : null}
      </aside>
      {dude ? (
        <section
          aria-label={dude.name}
          className="flex min-h-0 min-w-0 flex-1 flex-col px-3 pt-3 lg:px-5 lg:pt-4"
        >
          <div className="mb-2 flex shrink-0 flex-wrap items-center gap-2">
            <h1 className="text-base font-medium">{dude.name}</h1>
            <Badge variant="outline">
              {t(
                dude.kind === "Memory"
                  ? "dude.memory"
                  : dude.isMain
                  ? "dude.main"
                  : "dude.custom"
              )}
            </Badge>
          </div>
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
                  {t(`dude.tabs.${value}`)}
                </TabsTrigger>
              ))}
            </TabsList>
            <TabsContent
              className="min-h-0 overflow-hidden pb-3"
              value="conversation"
            >
              {interaction.isPending ? (
                <p role="status">{t("dude.loading")}</p>
              ) : interaction.error ? (
                <div role="alert">
                  {interaction.error.message}
                  <Button size="sm" onClick={() => void interaction.refetch()}>
                    {t("dude.retry")}
                  </Button>
                </div>
              ) : interaction.data ? (
                <SessionChat
                  key={interaction.data.sessionCode}
                  session={interaction.data}
                  readOnly={false}
                  title={dude.name}
                />
              ) : null}
            </TabsContent>
            <TabsContent className="min-h-0 overflow-y-auto pb-4" value="work">
              <DudeThreads
                key={code}
                dudeCode={code}
                active={visible && tab === "work"}
                requestedThread={search.get("thread")}
              />
              <h2 className="mt-6 border-t border-[var(--color-outline-variant)] pt-4 font-medium">
                {t("dude.activity")}
              </h2>
              <div className="divide-y divide-[var(--color-outline-variant)]">
                {events.isPending ? (
                  <p role="status">{t("dude.loading")}</p>
                ) : null}
                {events.error ? (
                  <div role="alert">
                    {events.error.message}
                    <Button size="sm" onClick={() => void events.refetch()}>
                      {t("dude.retry")}
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
                        {t(`dude.sources.${event.source}`, {
                          defaultValue: event.source,
                        })}
                      </span>
                      <p className="mt-1 text-xs text-[var(--color-on-surface-variant)]">
                        {date(event.createdAt)}
                      </p>
                      {event.result ? (
                        <details className="mt-2 text-sm">
                          <summary className="cursor-pointer text-[var(--color-on-surface-variant)]">
                            {t("dude.viewResult")}
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
                      {t(`dude.eventStatuses.${event.status}`, {
                        defaultValue: event.status,
                      })}
                    </Badge>
                    {event.sessionCode ? (
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={() => openSession(event.sessionCode!)}
                      >
                        {t("dude.viewWork")}
                      </Button>
                    ) : null}
                  </article>
                ))}
              </div>
              {events.data?.length === 0 ? (
                <p className="py-4 text-sm text-[var(--color-on-surface-variant)]">
                  {t("dude.noWork")}
                </p>
              ) : null}
            </TabsContent>
            <TabsContent className="min-h-0 overflow-y-auto pb-4" value="tasks">
              <DudeTasks
                key={code}
                dudeCode={code}
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
                  <div className="mb-2 flex shrink-0 items-center justify-between gap-3">
                    <span>{selectedHistory.title}</span>
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
                      {t("dude.backToHistory")}
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
                    <p role="status">{t("dude.loading")}</p>
                  ) : null}
                  {sessions.error ? (
                    <div role="alert">
                      {sessions.error.message}
                      <Button size="sm" onClick={() => void sessions.refetch()}>
                        {t("dude.retry")}
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
                          {t(`dude.purposes.${session.purpose}`, {
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
                        {t("dude.open")}
                      </Button>
                    </div>
                  ))}
                </div>
              )}
            </TabsContent>
          </Tabs>
        </section>
      ) : null}
    </div>
  );
}
