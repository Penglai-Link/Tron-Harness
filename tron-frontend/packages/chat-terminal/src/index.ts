import "@xterm/xterm/css/xterm.css";
import {
  isInteractiveShellCommand,
  Terminal,
  type CommandExecutionContext,
  type CommandExplainContext,
} from "./terminal";
import { formatePath, isShellCommand } from "./terminal/utils/common";
import { ITerminalInitOnlyOptions, ITerminalOptions } from "@xterm/xterm";
import type { TerminalThemeTokenOverrides } from "./terminal/types/themes";
import {
  createChatTerminalI18n,
  type ChatTerminalLocale,
  type ChatTerminalMessageOverrides,
} from "./terminal/i18n";
import {
  type AITransport,
  type TerminalTransport,
} from "./transport";

export type {
  ChatTerminalLocale,
  ChatTerminalMessageKey,
  ChatTerminalMessageOverrides,
  ChatTerminalMessages,
  CommandExecutionContext,
  CommandExplain,
  CommandExplainContext,
  CommandExplainPayload,
} from "./terminal";
export type { TerminalThemeTokenOverrides } from "./terminal";
export * from "./transport";

export interface TerminalPrompts {
  chat?: (content: string) => string;
  explain?: (payload: { command: string; output: string }) => string;
  generate?: (content: string) => string;
}

export interface ChatTerminalOptions {
  el: HTMLElement;
  transport: {
    ai?: AITransport;
    terminal: TerminalTransport;
  };
  prompts?: TerminalPrompts;
  locale?: ChatTerminalLocale;
  messages?: ChatTerminalMessageOverrides;
  terminalOptions?: ITerminalInitOnlyOptions &
    ITerminalOptions & {
      explainTitle?: string;
      agentTips?: string;
      agentTipsGapCells?: number;
      themeTokens?: TerminalThemeTokenOverrides;
    };
  isInteractiveCommand?: (command: string) => boolean;
  onError?: (error: Error) => void;
}

export type IOptions = ChatTerminalOptions;

const defaultOptions: Partial<ChatTerminalOptions> = {};

export function initTerminal(options: ChatTerminalOptions) {
  const normalizeOptions = { ...defaultOptions, ...options };
  const { terminalOptions } = normalizeOptions;
  const terminalTransport = normalizeOptions.transport.terminal;
  const aiTransport = normalizeOptions.transport.ai;
  const i18n = createChatTerminalI18n({
    locale: normalizeOptions.locale,
    messages: {
      ...(terminalOptions?.agentTips
        ? { agentTips: terminalOptions.agentTips }
        : {}),
      ...(terminalOptions?.explainTitle
        ? { explainTitle: terminalOptions.explainTitle }
        : {}),
      ...normalizeOptions.messages,
    },
  });
  let currentDirectory = "/";
  let disposed = false;

  const commandChat = aiTransport
    ? async (command: string) => {
        let requestId = "default";
        try {
          const result = await aiTransport.stream(
            normalizeOptions.prompts?.chat?.(command) ?? command,
            {
              onChunk: ({ delta, requestId: chunkRequestId }) => {
                requestId = chunkRequestId;
                if (disposed) return;
                termInstance.modelSessionAddon?.appendMessageChunk({
                  conversationId: "default",
                  requestId,
                  response: delta,
                });
              },
            },
          );
          requestId = result.requestId;
        } finally {
          if (!disposed) {
            await (
              termInstance.modelSessionAddon?.complete(requestId) ??
              Promise.resolve()
            );
          }
        }
      }
      : undefined;
  const commandExplain = aiTransport
    ? async (
        payload: { command: string; output: string },
        { onChunk, signal }: CommandExplainContext,
      ) => {
        let content = "";
        await aiTransport.stream(
          normalizeOptions.prompts?.explain?.(payload) ??
            `Command:\n${payload.command}\n\nOutput:\n${payload.output}`,
          {
            onChunk: ({ delta }) => {
              content += delta;
              onChunk?.(delta);
            },
            signal,
          },
        );
        return content;
      }
      : undefined;
  const commandGenerate = aiTransport
    ? (content: string) =>
        aiTransport.request(
          normalizeOptions.prompts?.generate?.(content) ?? content,
        )
      : undefined;

  async function runCommandByIo(
    command: string,
    onOutput: (output: string) => void = (output) => {
      termInstance.terminal.write(output);
    },
    context: CommandExecutionContext = { interactive: false },
  ) {
    const input = command.trim();
    if (!input) {
      return;
    }
    try {
      const result = await terminalTransport.execute(input, onOutput, context);
      if (result?.currentDirectory) {
        currentDirectory = result.currentDirectory;
      }
    } catch (error) {
      onOutput(`${error}\r\n`);
    }
  }

  const termInstance: Terminal = new Terminal(normalizeOptions.el, {
    cursorBlink: true,
    ...terminalOptions,
    localEcho: {
      enabled: true,
      prompt: () => {
        return formatePath(currentDirectory);
      },
      isInteractiveCommand:
        normalizeOptions.isInteractiveCommand ?? isInteractiveShellCommand,
      onCancelCommand: () => terminalTransport.cancel(),
      onCommandInput: (data) => terminalTransport.write(data),
      // 执行命令前判断，系统命令直接执行，非系统命令调用大模型
      onBeforeCommand: async (command) => {
        if (!command.trim()) {
          return false;
        }
        // 系统命令
        if (isShellCommand(command.toLocaleLowerCase())) {
          return false;
        }
        if (!commandChat) {
          return false;
        }
        await commandChat(command);
        // 调用大模型
        return true;
      },
      onCommand: runCommandByIo,
      // 结果解释
      onExplain: commandExplain,
      i18n,
    },
    agentPrompt: commandChat
      ? {
          i18n,
          textKey: "agentTips",
          gapCells: terminalOptions?.agentTipsGapCells || 1,
        }
      : undefined,
    modelSession:
      commandChat || commandGenerate
        ? {
            i18n,
            // 点击直接命令
            onExecuteCommand: ({ command }) => {
              return termInstance.executeCommand(command);
            },
            // 编辑命令成功回调
            onModifyCommand: async ({ command }) => command.trim(),
            // AI生成命令
            onGenerateCommand: ({ command }) => {
              if (!commandGenerate) {
                return Promise.reject(new Error("未配置 AI 命令生成能力"));
              }
              return commandGenerate(command);
            },
          }
        : undefined,
  });
  const resizeDisposable = termInstance.terminal.onResize((size) => {
    terminalTransport.resize(size);
  });
  const sessionDisposable = terminalTransport.onSessionChange((session) => {
    setCurrentDirectory(session.currentDirectory);
  });
  termInstance.terminal.write(formatePath(currentDirectory));
  termInstance.markPromptRendered();
  void terminalTransport.connect().catch((error) => {
    const normalizedError = toError(error);
    if (normalizeOptions.onError) {
      normalizeOptions.onError(normalizedError);
    } else {
      console.warn(normalizedError);
    }
  });

  function setCurrentDirectory(nextDirectory: string) {
    currentDirectory = nextDirectory;
    termInstance.terminal.write(`\r\x1b[2K${formatePath(currentDirectory)}`);
    termInstance.markPromptRendered();
  }

  return {
    dispose() {
      disposed = true;
      sessionDisposable?.dispose();
      resizeDisposable?.dispose();
      terminalTransport.dispose();
      aiTransport?.dispose?.();
      termInstance.dispose();
    },
    setCurrentDirectory(nextDirectory: string) {
      setCurrentDirectory(nextDirectory);
    },
    // 向模型会话追加一段流式增量，渲染到终端
    appendChatChunk(
      response: string,
      ids?: { requestId?: string; conversationId?: string },
    ) {
      termInstance.modelSessionAddon?.appendMessageChunk({
        requestId: ids?.requestId ?? "default",
        conversationId: ids?.conversationId ?? "default",
        response,
      });
    },
    // 结束模型会话，收尾渲染
    completeChat(requestId = "default") {
      return (
        termInstance.modelSessionAddon?.complete(requestId) ?? Promise.resolve()
      );
    },
    setThemeTokens(tokens: TerminalThemeTokenOverrides) {
      termInstance.setThemeTokens(tokens);
    },
    resetTheme() {
      termInstance.resetTheme();
    },
    refreshTheme() {
      termInstance.refreshTheme();
    },
    setLocale(
      locale: ChatTerminalLocale,
      messages?: ChatTerminalMessageOverrides,
    ) {
      i18n.setLocale(locale);
      if (messages) {
        i18n.setMessages(messages);
      }
      termInstance.refreshLocale();
    },
    setMessages(messages: ChatTerminalMessageOverrides) {
      i18n.setMessages(messages);
      termInstance.refreshLocale();
    },
  };
}

function toError(error: unknown) {
  return error instanceof Error ? error : new Error(String(error));
}
