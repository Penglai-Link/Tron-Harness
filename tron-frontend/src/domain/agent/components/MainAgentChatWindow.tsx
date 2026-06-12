import {
  defaultFloatingState,
  createMockChatAdapter,
  normalizeChatSession,
  readFloatingState,
  writeFloatingState,
  type ChatFloatingState,
  type ChatSession,
} from "@tron/chat-core";
import { createInternalChatAdapter } from "@tron/chat-internal-adapter";
import { TronChatRuntimeWindow } from "@tron/chat-react";
import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";

import { getApiAccessToken } from "@/shared/api/httpClient";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { useToast } from "@/shared/ui/Toast";

interface MainAgentChatWindowProps {
  currentUserDisplayName?: string;
  floatingSubtitle?: string;
  floatingTitle?: string;
  footerAddon?: ReactNode;
  workspaceCode?: string;
  windowState?: ChatFloatingState;
  onWindowStateChange?: (state: ChatFloatingState) => void;
}

const MAIN_AGENT_WINDOW_STORAGE_KEY = "tron.main-agent.window";
const MAIN_AGENT_SESSION_STORAGE_PREFIX = "tron.main-agent.session";
const pendingMainAgentSessions = new Map<string, Promise<ChatSession>>();

function mainAgentSessionStorageKey(workspaceCode: string) {
  const normalizedWorkspaceCode = workspaceCode.trim();
  return `${MAIN_AGENT_SESSION_STORAGE_PREFIX}:${normalizedWorkspaceCode || "__default__"}`;
}

function readPersistedMainAgentSession(storageKey: string) {
  if (typeof window === "undefined" || !storageKey) return null;
  try {
    const raw = window.localStorage.getItem(storageKey);
    if (!raw) return null;
    return normalizeChatSession(JSON.parse(raw) as ChatSession);
  } catch {
    return null;
  }
}

function writePersistedMainAgentSession(
  storageKey: string,
  session: ChatSession | null,
) {
  if (typeof window === "undefined" || !storageKey) return;
  if (!session) {
    window.localStorage.removeItem(storageKey);
    return;
  }
  window.localStorage.setItem(
    storageKey,
    JSON.stringify(normalizeChatSession(session)),
  );
}

export function MainAgentChatWindow({
  currentUserDisplayName,
  floatingSubtitle = "Workspace Coordinator",
  floatingTitle = "Main Agent",
  footerAddon,
  workspaceCode = "",
  windowState,
  onWindowStateChange,
}: MainAgentChatWindowProps) {
  const toast = useToast();
  const adapter = useMemo(
    () =>
      getServiceDataSource("chat") === "mock"
        ? createMockChatAdapter()
        : createInternalChatAdapter({
            getAccessToken: getApiAccessToken,
          }),
    [],
  );
  const controlledWindowState = Boolean(windowState && onWindowStateChange);
  const [localWindowState, setLocalWindowState] = useState<ChatFloatingState>(
    () =>
      readFloatingState(MAIN_AGENT_WINDOW_STORAGE_KEY, defaultFloatingState),
  );
  const resolvedWindowState = controlledWindowState
    ? (windowState as ChatFloatingState)
    : localWindowState;
  const sessionStorageKey = useMemo(
    () => mainAgentSessionStorageKey(workspaceCode),
    [workspaceCode],
  );
  const [persistedSession, setPersistedSession] = useState<ChatSession | null>(
    () => readPersistedMainAgentSession(sessionStorageKey),
  );

  useEffect(() => {
    if (!controlledWindowState) {
      writeFloatingState(MAIN_AGENT_WINDOW_STORAGE_KEY, localWindowState);
    }
  }, [controlledWindowState, localWindowState]);

  useEffect(() => {
    setPersistedSession(readPersistedMainAgentSession(sessionStorageKey));
  }, [sessionStorageKey]);

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

  const handleSessionChange = useCallback(
    (nextSession: ChatSession | null) => {
      setPersistedSession(nextSession);
      writePersistedMainAgentSession(sessionStorageKey, nextSession);
    },
    [sessionStorageKey],
  );

  const windowConfig = useMemo(() => ({
    agentCode: "main-agent",
    floating: true,
    inputPlaceholder: "Message Main Agent...",
    storageKey: MAIN_AGENT_WINDOW_STORAGE_KEY,
    title: "Main Agent",
    workspaceCode,
  }), [workspaceCode]);

  const handleError = useCallback((message: string, title?: string) => {
    toast.error(message, { title: title ?? "Main Agent failed" });
  }, [toast]);

  const handleInfo = useCallback((message: string, title?: string) => {
    toast.info(message, { title });
  }, [toast]);

  useEffect(() => {
    if (
      !resolvedWindowState.open ||
      persistedSession ||
      !workspaceCode.trim()
    ) {
      return;
    }

    let active = true;
    const pendingSession =
      pendingMainAgentSessions.get(sessionStorageKey) ??
      adapter.createSession({
        agentCode: "main-agent",
        title: floatingTitle,
        workspaceCode: workspaceCode.trim(),
      });

    pendingMainAgentSessions.set(sessionStorageKey, pendingSession);

    void pendingSession
      .then((createdSession) => {
        if (!active) return;
        setPersistedSession(createdSession);
        writePersistedMainAgentSession(sessionStorageKey, createdSession);
      })
      .catch((error) => {
        if (!active) return;
        const message =
          error instanceof Error
            ? error.message
            : "Main Agent session could not be created";
        toast.error(message, { title: "Main Agent failed" });
      })
      .finally(() => {
        if (
          pendingMainAgentSessions.get(sessionStorageKey) === pendingSession
        ) {
          pendingMainAgentSessions.delete(sessionStorageKey);
        }
      });

    return () => {
      active = false;
    };
  }, [
    adapter,
    floatingTitle,
    persistedSession,
    resolvedWindowState.open,
    sessionStorageKey,
    toast,
    workspaceCode,
  ]);

  return (
    <>
      <TronChatRuntimeWindow
        adapter={adapter}
        config={windowConfig}
        currentUserDisplayName={currentUserDisplayName}
        floatingSubtitle={floatingSubtitle}
        floatingTitle={floatingTitle}
        footerAddon={footerAddon}
        initialSession={persistedSession}
        onError={handleError}
        onInfo={handleInfo}
        onSessionChange={handleSessionChange}
        windowState={resolvedWindowState}
        onWindowStateChange={handleWindowStateChange}
      />
    </>
  );
}
