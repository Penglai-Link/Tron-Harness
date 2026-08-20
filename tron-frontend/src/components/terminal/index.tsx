import { createInternalChatAdapter, createInternalTerminalAdapter } from "@tron/chat-internal-adapter";
import { type AITransport, createTerminalTransport, initTerminal } from "@tron/chat-terminal";
import { useCallback, useEffect, useRef } from "react";

import { useLocale } from "@/i18n";
import { buildApiUrl, getApiBaseUrl } from "@/shared/api/httpClient";
import { useTheme, useWorkspaceSpaces } from "@/shared";

import { explainResultMessage, generateAiChatMessage, generateCommandMessage } from "./prompt";

import "@tron/chat-terminal/styles.css";
import "@xterm/xterm/css/xterm.css";

const CONFIGURED_TERMINAL_WORKSPACE_CODE = import.meta.env.VITE_TRON_TERMINAL_WORKSPACE_CODE?.trim() || null;
const TERMINAL_APP_CODE = import.meta.env.VITE_TRON_TERMINAL_APP_CODE?.trim() || null;
const TERMINAL_MODEL_CODE = import.meta.env.VITE_TRON_TERMINAL_MODEL_CODE?.trim() || null;
const TERMINAL_API_BASE_URL = getApiBaseUrl();

type TerminalChatContext = Awaited<ReturnType<typeof createTerminalChatContext>>;
type TerminalChatContextCache = {
  context: Promise<TerminalChatContext>;
  workspaceCode: string;
};

const Terminal = () => {
  const { theme } = useTheme();
  const { currentLocale } = useLocale();
  const { selectedPrimarySpaceId, selectedSecondarySpaceId } = useWorkspaceSpaces();
  const terminalRef = useRef<HTMLDivElement>(null);
  const terminalInstanceRef = useRef<ReturnType<typeof initTerminal> | null>(null);
  const terminalChatContextRef = useRef<TerminalChatContextCache | null>(null);
  const currentLocaleRef = useRef(currentLocale);
  const workspaceCode = CONFIGURED_TERMINAL_WORKSPACE_CODE
    || selectedSecondarySpaceId
    || selectedPrimarySpaceId;

  const terminalChatContext = useCallback((targetWorkspaceCode: string) => {
    const cached = terminalChatContextRef.current;
    if (cached?.workspaceCode === targetWorkspaceCode) return cached.context;

    const context = createTerminalChatContext(targetWorkspaceCode);
    const nextCache = { context, workspaceCode: targetWorkspaceCode };
    terminalChatContextRef.current = nextCache;
    void context.catch(() => {
      if (terminalChatContextRef.current === nextCache) {
        terminalChatContextRef.current = null;
      }
    });
    return context;
  }, []);

  useEffect(() => {
    const container = terminalRef.current;
    if (!container || !workspaceCode) return;

    let disposed = false;
    let terminalInstance: ReturnType<typeof initTerminal> | null = null;
    let releaseSession: (() => void) | undefined;
    let activeChatAbortController: AbortController | null = null;

    void (async () => {
      const { chatAdapter, session, terminalAdapter } = await terminalChatContext(workspaceCode);
      let sessionReleased = false;
      releaseSession = () => {
        if (sessionReleased) return;
        sessionReleased = true;
        terminalChatContextRef.current = null;
        activeChatAbortController?.abort();
        void (async () => {
          try {
            const activeRun = await chatAdapter.getActiveRun?.(session.sessionCode);
            if (activeRun?.activeRun) {
              await chatAdapter.stopRun?.(session, {
                expectedRunCode: activeRun.activeRun.runCode,
              });
            }
          } catch (error) {
            console.warn("Terminal chat run cleanup failed", error);
          } finally {
            await chatAdapter.deleteSession(session.sessionCode).catch((error: unknown) => {
              console.warn("Terminal chat session cleanup failed", error);
            });
          }
        })();
      };
      if (disposed) {
        releaseSession();
        return;
      }
      const socketConnection = await terminalAdapter.createSocketTicket({
        appCode: TERMINAL_APP_CODE,
        workspaceCode,
      });
      if (disposed) return;

      const terminalTransport = createTerminalTransport({
        socketOptions: {
          auth: { ticket: socketConnection.ticket },
          path: socketConnection.socketPath,
          reconnection: false,
        },
        url: socketConnection.socketUrl,
      });
      const aiTransport: AITransport = {
        async stream(content, { onChunk, signal }) {
          const controller = new AbortController();
          const abort = () => controller.abort(signal?.reason);
          if (signal?.aborted) {
            abort();
          } else {
            signal?.addEventListener("abort", abort, { once: true });
          }
          activeChatAbortController = controller;
          const requestId = crypto.randomUUID();
          try {
            await chatAdapter.streamMessage(session, {
              content,
              workspaceCode,
            }, {
              onDelta(delta) {
                onChunk({ delta, requestId });
              },
            }, controller.signal);
            return { requestId };
          } finally {
            signal?.removeEventListener("abort", abort);
            if (activeChatAbortController === controller) {
              activeChatAbortController = null;
            }
          }
        },

        dispose() {
          activeChatAbortController?.abort();
        },

        async request(content, { signal } = {}) {
          const result = await terminalAdapter.generateCommand({
            modelCode: TERMINAL_MODEL_CODE,
            prompt: content,
            workspaceCode,
          }, signal);
          return result.command;
        },
      };

      terminalInstance = initTerminal({
        el: container,
        prompts: {
          chat: generateAiChatMessage,
          explain: ({ command, output }) => explainResultMessage(command, output),
          generate: generateCommandMessage,
        },
        terminalOptions: {
          themeTokens: {
            border: "var(--color-card-border, gray)",
            cardBackground: "var(--color-elevated-surface, white)",
            primary: "var(--color-on-surface, black)",
            secondary: "var(--color-header-backdrop, white)",
          },
        },
        transport: {
          ai: aiTransport,
          terminal: terminalTransport,
        },
      });
      if (disposed) {
        terminalInstance.dispose();
        terminalInstance = null;
        return;
      }
      terminalInstanceRef.current = terminalInstance;
      terminalInstance.refreshTheme();
      terminalInstance.setLocale(currentLocaleRef.current);
    })().catch((error: unknown) => {
      releaseSession?.();
      if (!disposed) console.error("Terminal initialization failed", error);
    });

    return () => {
      disposed = true;
      terminalInstance?.dispose();
      if (terminalInstanceRef.current === terminalInstance) {
        terminalInstanceRef.current = null;
      }
      releaseSession?.();
    };
  }, [terminalChatContext, workspaceCode]);

  useEffect(() => {
    terminalInstanceRef.current?.refreshTheme();
  }, [theme]);

  useEffect(() => {
    currentLocaleRef.current = currentLocale;
    terminalInstanceRef.current?.setLocale(currentLocale);
  }, [currentLocale]);

  if (!workspaceCode) {
    return (
      <div className="flex h-full w-full items-center justify-center px-6 text-sm text-[var(--color-on-surface-variant)]">
        Select a workspace before opening the terminal.
      </div>
    );
  }

  return <div ref={terminalRef} className="relative h-full w-full py-2.5 pl-2.5" />;
};

export default Terminal;

async function createTerminalChatContext(workspaceCode: string) {
  const { accessToken } = await fetch(buildApiUrl("/api/v1/auth/refresh"), {
    credentials: "include",
    method: "POST",
  })
    .then((response) => response.json())
    .then((response) => response.data);
  const adapterOptions = {
    baseUrl: TERMINAL_API_BASE_URL,
    getAccessToken: () => accessToken,
  };
  const chatAdapter = createInternalChatAdapter(adapterOptions);
  const terminalAdapter = createInternalTerminalAdapter(adapterOptions);
  const session = await chatAdapter.createSession({
    sourceType: "TERMINAL",
    workspaceCode,
  });
  return { chatAdapter, session, terminalAdapter };
}
