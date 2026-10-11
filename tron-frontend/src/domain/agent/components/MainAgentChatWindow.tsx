import { defaultMainAgentWindowState, MAIN_AGENT_WINDOW_STORAGE_KEY } from "@/domain/agent/model/mainAgentWindowState";
import {
  readFloatingState,
  writeFloatingState,
  type ChatFloatingState,
  type ChatSession,
  type SendChatMessageInput,
} from "@tron/chat-core";
import { createInternalChatAdapter } from "@tron/chat-internal-adapter";
import { AgentChatWindow } from "@/domain/chat/components/AgentChatWindow";
import "@eliteforge/agent-ui-chat-react/styles.css"
import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { useTranslation } from "react-i18next";

import { MarkdownLink } from "@/components/markdown-link";
import { useChatWorkRailLabels, useChatHistoryLabels } from "@/domain/chat";
import { getApiAccessToken } from "@/shared/api/httpClient";
import { DesktopBridge } from "@/shared/desktop/DesktopBridge";
import { OPEN_CHATS_PAGE_VIEW_EVENT } from "@/shared/lib/shellEvents";
import { useToast } from "@/components/ui/toast";
import { useTheme } from "@/shared";
import { applyCurrentLocaleHeader } from "@/i18n";
import { useChatFeedback } from "@/domain/chat/hooks/useChatFeedback";
import { resolveMainDudeSession } from "@/domain/dude/api/dudeApi";
import { Button } from "@/components/ui/button";

interface MainAgentChatWindowProps {
  currentUserDisplayName?: string;
  floatingSubtitle?: string;
  floatingTitle?: string;
  footerAddon?: ReactNode;
  pageContext?: SendChatMessageInput["pageContext"];
  workspaceCode?: string;
  windowState?: ChatFloatingState;
  onWindowStateChange?: (state: ChatFloatingState) => void;
}

// Resolution is server-owned and platform-wide, including across browser tabs.
let pendingMainDudeSession: Promise<ChatSession> | null = null;
function resolvePlatformSession() {
  if (!pendingMainDudeSession) {
    pendingMainDudeSession = resolveMainDudeSession().finally(() => { pendingMainDudeSession = null; });
  }
  return pendingMainDudeSession;
}

export function MainAgentChatWindow({
  currentUserDisplayName,
  floatingSubtitle,
  floatingTitle,
  footerAddon,
  pageContext,
  workspaceCode: _workspaceCode,
  windowState,
  onWindowStateChange,
}: MainAgentChatWindowProps) {
  const toast = useToast();
  const feedback = useChatFeedback();
  const { t } = useTranslation("pages");
  const { agentReplyEffect, agentReplySpeed } = useTheme();
  const workRailLabels = useChatWorkRailLabels();
  const historyLabels = useChatHistoryLabels();
  const desktopBridge = useMemo(() => new DesktopBridge(), []);
  const openMarkdownLinkInChatsSidebar = useCallback((url: string) => {
    window.dispatchEvent(new CustomEvent(OPEN_CHATS_PAGE_VIEW_EVENT, { detail: url }));
  }, []);
  const openMarkdownLink = useCallback((url: string) => {
    if (!desktopBridge.isDesktop()) {
      window.open(url, "_blank", "noopener,noreferrer");
      return;
    }

    // 桌面端：走 IPC 让主进程打开系统浏览器
    // fire-and-forget：主进程立即返回，不等浏览器打开，
    // 避免渲染进程后续处理抢回焦点导致浏览器窗口层级变低
    void desktopBridge.openTargetPage({ mode: "browser", url }).catch(() => {
      toast.error(t("chat.markdownLink.openExternalFailed"));
    });
  }, [desktopBridge, t, toast]);
  const platformSessionRef = useRef<ChatSession | null>(null);
  const adapter = useMemo(() => createInternalChatAdapter({
    workspaceCode: null,
    getAccessToken: getApiAccessToken,
    getHeaders: () => {
      const headers = applyCurrentLocaleHeader(new Headers());
      const owner = platformSessionRef.current?.ownerDudeCode;
      if (owner) headers.set("X-Dude-Code", owner);
      return headers;
    },
  }), []);
  const controlledWindowState = Boolean(windowState && onWindowStateChange);
  const [localWindowState, setLocalWindowState] = useState<ChatFloatingState>(
    () =>
      readFloatingState(MAIN_AGENT_WINDOW_STORAGE_KEY, defaultMainAgentWindowState),
  );
  const resolvedWindowState = controlledWindowState
    ? (windowState as ChatFloatingState)
    : localWindowState;
  const [persistedSession, setPersistedSession] = useState<ChatSession | null>(null);
  platformSessionRef.current = persistedSession;
  const [resolutionError, setResolutionError] = useState<string | null>(null);
  const [retry, setRetry] = useState(0);
  useEffect(() => {
    if (!controlledWindowState) {
      writeFloatingState(MAIN_AGENT_WINDOW_STORAGE_KEY, localWindowState);
    }
  }, [controlledWindowState, localWindowState]);

  const handleWindowStateChange = useCallback(
    (nextState: ChatFloatingState) => {
      if (controlledWindowState) {
        onWindowStateChange?.(nextState);
        return;
      }
      setLocalWindowState(nextState);
    },
    [controlledWindowState, onWindowStateChange],
  );

  const handleSessionChange = useCallback((nextSession: ChatSession | null) => {
    // Chat completion returns a generic Session; Dude ownership is resolved by
    // the platform endpoint and must survive updates to that same conversation.
    setPersistedSession(current => current && nextSession?.sessionCode === current.sessionCode
      ? { ...nextSession, ownerDudeCode: current.ownerDudeCode, purpose: current.purpose }
      : nextSession);
  }, []);

  const windowConfig = useMemo(() => ({
    agentCode: persistedSession?.agentCode ?? undefined,
    floating: true,
    showNewSession: false,
    inputPlaceholder: t("dude.messagePlaceholder"),
    storageKey: MAIN_AGENT_WINDOW_STORAGE_KEY,
    title: t("dude.mainConversation"),
    workRailLabels,
    historyLabels,
    workspaceCode: undefined,
  }), [workRailLabels, historyLabels, persistedSession?.agentCode, t]);
  const createSendInput = useCallback((
    session: ChatSession,
    content: string,
    attachments: NonNullable<SendChatMessageInput["attachments"]>,
  ): SendChatMessageInput => ({
    agentCode: session.agentCode,
    appCode: session.appCode,
    attachments: attachments.length ? attachments : undefined,
    clientCapabilities: { userInputRequests: true },
    content,
    modelSelection: { kind: "Auto" },
    pageContext,
    taskCode: session.taskCode,
    workspaceCode: null,
  }), [pageContext]);

  const handleError = useCallback((message: string, title?: string) => {
    feedback.error(message, title);
  }, [feedback.error]);

  const handleInfo = useCallback((message: string, title?: string) => {
    feedback.info(message, title);
  }, [feedback.info]);

  useEffect(() => {
    if (!resolvedWindowState.open) return;
    let active = true;
    setResolutionError(null);
    void resolvePlatformSession().then(session => {
      if (active) setPersistedSession(session);
    }).catch((reason: unknown) => {
      if (active) setResolutionError(reason instanceof Error ? reason.message : String(reason));
    });
    return () => { active = false; };
  }, [resolvedWindowState.open, retry]);

  if (!persistedSession) return resolvedWindowState.open ? <div role="dialog" aria-label={t("dude.mainConversation")} className="fixed bottom-4 right-4 z-50 max-w-sm rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] p-4 shadow-lg">
    {resolutionError ? <><p role="alert">{resolutionError}</p><Button className="mt-3" onClick={() => setRetry(value => value + 1)}>{t("dude.retry")}</Button></> : <p role="status">{t("dude.loading")}</p>}
    <Button className="ml-2" variant="ghost" onClick={() => handleWindowStateChange({ ...resolvedWindowState, open: false })}>{t("dude.close")}</Button>
  </div> : null;

  return (
    <>
      <AgentChatWindow
        agentReplyEffect={agentReplyEffect}
        agentReplySpeed={agentReplySpeed}
        adapter={adapter}
        config={windowConfig}
        createSendInput={createSendInput}
        currentUserDisplayName={currentUserDisplayName}
        floatingSubtitle={floatingSubtitle ?? t("dude.platform")}
        floatingTitle={floatingTitle ?? t("dude.mainConversation")}
        footerAddon={footerAddon}
        initialSession={persistedSession}
        onError={handleError}
        onInfo={handleInfo}
        onSessionChange={handleSessionChange}
        windowState={resolvedWindowState}
        onWindowStateChange={handleWindowStateChange}
        renderMarkdownLink={({ href, children }) => (
          <MarkdownLink
            href={href}
            onOpenInternal={openMarkdownLinkInChatsSidebar}
            onOpenExternal={openMarkdownLink}
          >
            {children}
          </MarkdownLink>
        )}
      />
    </>
  );
}
