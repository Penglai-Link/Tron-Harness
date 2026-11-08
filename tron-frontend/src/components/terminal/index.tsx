import { createInternalChatAdapter, createInternalTerminalAdapter } from "@tron/chat-internal-adapter";
import { applyCurrentLocaleHeader } from "@/i18n";
import { type AITransport, createTerminalTransport, initTerminal } from "@personai/agent-ui-terminal";
import type { ChatAguiStreamEvent, ChatUserInputAnswer } from "@tron/chat-core";
import { fromUiResume, toUiEvent } from "@tron/chat-agent-ui-adapter";
import { useCallback, useEffect, useRef } from "react";

import { useLocale } from "@/i18n";
import { archiveChatSession } from "@/domain/chat/hooks";
import { getApiAccessToken, getApiBaseUrl } from "@/shared/api/httpClient";
import { useTheme, useWorkspaceSpaces } from "@/shared";

import { explainResultMessage, generateAiChatMessage, generateCommandMessage } from "./prompt";
import { createTerminalChatInput } from "./terminalChatInput";
import { getTerminalAnsiTheme } from "./terminalTheme";

import "@personai/agent-ui-terminal/styles.css";
import "@xterm/xterm/css/xterm.css";

const CONFIGURED_TERMINAL_WORKSPACE_CODE = import.meta.env.VITE_TRON_TERMINAL_WORKSPACE_CODE?.trim() || null;
const TERMINAL_APP_CODE = import.meta.env.VITE_TRON_TERMINAL_APP_CODE?.trim() || null;
const TERMINAL_MODEL_CODE = import.meta.env.VITE_TRON_TERMINAL_MODEL_CODE?.trim() || null;
const TERMINAL_API_BASE_URL = getApiBaseUrl();

type TerminalChatContext = Awaited<ReturnType<typeof createTerminalChatContext>>;
type TerminalChatContextCache = {
  context: Promise<TerminalChatContext>;
  releaseTimer: ReturnType<typeof setTimeout> | null;
  retainCount: number;
  workspaceCode: string;
};

type TerminalChatContextLease = {
  context: Promise<TerminalChatContext>;
  release: () => void;
};

interface TerminalProps {
  onExit?: () => void;
}

const Terminal = ({ onExit }: TerminalProps) => {
  const { theme, activeSkin } = useTheme();
  const { currentLocale } = useLocale();
  const { selectedPrimarySpaceId, selectedSecondarySpaceId } = useWorkspaceSpaces();
  const terminalRef = useRef<HTMLDivElement>(null);
  const terminalInstanceRef = useRef<ReturnType<typeof initTerminal> | null>(null);
  const terminalChatContextRef = useRef<TerminalChatContextCache | null>(null);
  const currentLocaleRef = useRef(currentLocale);
  const currentThemeRef = useRef(theme);
  const workspaceCode = CONFIGURED_TERMINAL_WORKSPACE_CODE
    || selectedSecondarySpaceId
    || selectedPrimarySpaceId;

  const acquireTerminalChatContext = useCallback((targetWorkspaceCode: string): TerminalChatContextLease => {
    const cached = terminalChatContextRef.current;
    const cache = cached?.workspaceCode === targetWorkspaceCode
      ? cached
      : (() => {
        const context = createTerminalChatContext(targetWorkspaceCode);
        const nextCache: TerminalChatContextCache = {
          context,
          releaseTimer: null,
          retainCount: 0,
          workspaceCode: targetWorkspaceCode,
        };
        terminalChatContextRef.current = nextCache;
        void context.catch(() => {
          if (terminalChatContextRef.current === nextCache) {
            terminalChatContextRef.current = null;
          }
        });
        return nextCache;
      })();
    if (cache.releaseTimer) {
      clearTimeout(cache.releaseTimer);
      cache.releaseTimer = null;
    }
    cache.retainCount += 1;

    let released = false;
    return {
      context: cache.context,
      release: () => {
        if (released) return;
        released = true;
        cache.retainCount -= 1;
        if (cache.retainCount !== 0) return;

        // StrictMode re-runs effects synchronously, so the next mount retains this cache first.
        cache.releaseTimer = setTimeout(() => {
          cache.releaseTimer = null;
          if (cache.retainCount !== 0) return;

          if (terminalChatContextRef.current === cache) {
            terminalChatContextRef.current = null;
          }
          void disposeTerminalChatSession(cache.context);
        }, 0);
      },
    };
  }, []);

  useEffect(() => {
    const container = terminalRef.current;
    if (!container || !workspaceCode) return;

    let disposed = false;
    let terminalInstance: ReturnType<typeof initTerminal> | null = null;
    let activeChatAbortController: AbortController | null = null;
    const activeChatRequestControllers = new Map<AbortSignal, AbortController>();
    const terminalChatLease = acquireTerminalChatContext(workspaceCode);

    void (async () => {
      const { chatAdapter, session, terminalAdapter } = await terminalChatLease.context;
      if (disposed) {
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
        async stream(content, { onChunk, onEvent, resume, signal }) {
          const controller = new AbortController();
          const abort = () => controller.abort(signal?.reason);
          if (signal?.aborted) {
            abort();
          } else {
            signal?.addEventListener("abort", abort, { once: true });
          }
          activeChatAbortController = controller;
          if (signal) {
            activeChatRequestControllers.set(signal, controller);
          }
          const requestId = crypto.randomUUID();
          try {
            const handlers = {
              onDelta(delta: string) {
                onChunk({ delta, requestId });
              },
              onAguiEvent(event: ChatAguiStreamEvent) {
                onEvent?.(toUiEvent(event));
              },
            };
            const hostResume = resume ? fromUiResume(resume) : undefined;
            const resumeEntry = hostResume?.[0];
            if (resumeEntry && chatAdapter.answerUserInputRequestStream) {
              const payload = resumeEntry.payload as { answers?: ChatUserInputAnswer[] };
              await chatAdapter.answerUserInputRequestStream(
                session,
                resumeEntry.interruptId,
                {
                  answers: Array.isArray(payload?.answers) ? payload.answers : [],
                  resume: hostResume,
                },
                handlers,
                controller.signal,
              );
            } else {
              await chatAdapter.streamMessage(
                session,
                createTerminalChatInput(content, workspaceCode),
                handlers,
                controller.signal,
              );
            }
            return { requestId };
          } finally {
            signal?.removeEventListener("abort", abort);
            if (signal) {
              activeChatRequestControllers.delete(signal);
            }
            if (activeChatAbortController === controller) {
              activeChatAbortController = null;
            }
          }
        },

        dispose() {
          activeChatAbortController?.abort();
        },

        async cancel(signal) {
          const controller = activeChatRequestControllers.get(signal);

          try {
            const activeRun = await chatAdapter.getActiveRun?.(session.sessionCode);
            if (activeRun?.activeRun?.runCode && chatAdapter.stopRun) {
              await chatAdapter.stopRun(session, {
                expectedRunCode: activeRun.activeRun.runCode,
              });
            }
          } finally {
            controller?.abort();
          }
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
        // Fires when the shell process itself exits, which covers `exit`,
        // Ctrl-D and a killed shell alike.
        onExit: () => onExit?.(),
        terminalOptions: {
          themeTokens: {
            ansi: getTerminalAnsiTheme(currentThemeRef.current),
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
      terminalChatLease.release();
      if (!disposed) console.error("Terminal initialization failed", error);
    });

    return () => {
      disposed = true;
      terminalInstance?.dispose();
      activeChatAbortController?.abort();
      if (terminalInstanceRef.current === terminalInstance) {
        terminalInstanceRef.current = null;
      }
      terminalChatLease.release();
    };
  }, [acquireTerminalChatContext, onExit, workspaceCode]);

  useEffect(() => {
    currentThemeRef.current = theme;
    terminalInstanceRef.current?.setThemeTokens({
      ansi: getTerminalAnsiTheme(theme),
    });
  }, [theme, activeSkin]);

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

  return <div ref={terminalRef} className="relative h-full w-full" />;
};

export default Terminal;

async function createTerminalChatContext(workspaceCode: string) {
  const adapterOptions = {
    workspaceCode,
    baseUrl: TERMINAL_API_BASE_URL,
    getAccessToken: getApiAccessToken,
    getHeaders: () => applyCurrentLocaleHeader(new Headers()),
  };
  const chatAdapter = createInternalChatAdapter(adapterOptions);
  const terminalAdapter = createInternalTerminalAdapter(adapterOptions);
  const session = await chatAdapter.createSession({
    sourceType: "TERMINAL",
    workspaceCode,
  });
  return { chatAdapter, session, terminalAdapter };
}

async function disposeTerminalChatSession(context: Promise<TerminalChatContext>) {
  let terminalChatContext: TerminalChatContext;
  try {
    terminalChatContext = await context;
  } catch (error) {
    console.warn("Terminal chat session cleanup failed", error);
    return;
  }

  const { chatAdapter, session } = terminalChatContext;
  try {
    const activeRun = await chatAdapter.getActiveRun?.(session.sessionCode);
    if (activeRun?.activeRun) {
      await chatAdapter.stopRun?.(session, {
        expectedRunCode: activeRun.activeRun.runCode,
      });
    }
  } catch (error) {
    console.warn("Terminal chat run cleanup failed", error);
    return;
  }
  try {
    await archiveChatSession(session.sessionCode);
    await chatAdapter.deleteSession(session.sessionCode);
  } catch (error) {
    console.warn("Terminal chat session cleanup failed", error);
  }
}
