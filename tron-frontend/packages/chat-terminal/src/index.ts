import "@xterm/xterm/css/xterm.css";
import {
  isInteractiveShellCommand,
  Terminal,
  type CommandExecutionContext,
  type CommandExplain,
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
  createSocketIoTerminalTransport,
  type AITransport,
  type SocketIoTerminalTransportOptions,
  type TerminalSize,
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

export interface TerminalConnectionOptions
  extends SocketIoTerminalTransportOptions {
  transport?: TerminalTransport;
  url?: string;
}

export interface TerminalAIPrompts {
  chat?: (content: string) => string;
  explain?: (payload: { command: string; output: string }) => string;
  generate?: (content: string) => string;
}

export interface TerminalAIOptions {
  prompts?: TerminalAIPrompts;
  transport: AITransport;
}

export interface ChatTerminalOptions {
  ai?: TerminalAIOptions;
  connection?: TerminalConnectionOptions;
  el: HTMLElement;
  locale?: ChatTerminalLocale;
  messages?: ChatTerminalMessageOverrides;
  termOptions?: ITerminalInitOnlyOptions &
    ITerminalOptions & {
      explainTitle?: string;
      agentTips?: string;
      agentTipsGapCells?: number;
      themeTokens?: TerminalThemeTokenOverrides;
    };
  runCommand?: (
    command: string,
    onOutput: (output: string) => void,
    context: CommandExecutionContext,
  ) => Promise<CommandExecutionResult | void>;
  cancelCommand?: () => void;
  isInteractiveCommand?: (command: string) => boolean;
  resizeTerminal?: (size: TerminalSize) => Promise<void> | void;
  writeCommandInput?: (data: string) => Promise<void> | void;
  commandChat?: (command: string) => Promise<void>;
  commandExplain?: CommandExplain;
  commandGenerate?: (content: string) => Promise<string>;
  onError?: (error: Error) => void;
}

export type IOptions = ChatTerminalOptions;

export interface CommandExecutionResult {
  currentDirectory?: string;
}

const defaultOptions: Partial<ChatTerminalOptions> = {};

export function initTerminal(options: ChatTerminalOptions) {
  const normalizeOptions = { ...defaultOptions, ...options };
  const { termOptions } = normalizeOptions;
  const terminalTransport = createTerminalTransport(
    normalizeOptions.connection,
  );
  const aiTransport = normalizeOptions.ai?.transport;
  const i18n = createChatTerminalI18n({
    locale: normalizeOptions.locale,
    messages: {
      ...(termOptions?.agentTips ? { agentTips: termOptions.agentTips } : {}),
      ...(termOptions?.explainTitle
        ? { explainTitle: termOptions.explainTitle }
        : {}),
      ...normalizeOptions.messages,
    },
  });
  let currentDirectory = "/";
  let disposed = false;

  const runCommand = normalizeOptions.runCommand
    ? normalizeOptions.runCommand
    : terminalTransport
      ? (
          command: string,
          onOutput: (output: string) => void,
          context: CommandExecutionContext,
        ) => terminalTransport.execute(command, onOutput, context)
      : undefined;

  if (!runCommand) {
    throw new Error("终端必须配置 connection 或 runCommand");
  }

  const commandChat =
    normalizeOptions.commandChat ??
    (aiTransport
      ? async (command: string) => {
          let requestId = "default";
          try {
            const result = await aiTransport.stream(
              normalizeOptions.ai?.prompts?.chat?.(command) ?? command,
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
      : undefined);
  const commandExplain =
    normalizeOptions.commandExplain ??
    (aiTransport
      ? (
          payload: { command: string; output: string },
          { signal }: { signal: AbortSignal },
        ) =>
          aiTransport.request(
            normalizeOptions.ai?.prompts?.explain?.(payload) ??
              `Command:\n${payload.command}\n\nOutput:\n${payload.output}`,
            { signal },
          )
      : undefined);
  const commandGenerate =
    normalizeOptions.commandGenerate ??
    (aiTransport
      ? (content: string) =>
          aiTransport.request(
            normalizeOptions.ai?.prompts?.generate?.(content) ?? content,
          )
      : undefined);

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
      const result = await runCommand?.(input, onOutput, context);
      if (result?.currentDirectory) {
        currentDirectory = result.currentDirectory;
      }
    } catch (error) {
      onOutput(`${error}\r\n`);
    }
  }

  const termInstance: Terminal = new Terminal(normalizeOptions.el, {
    cursorBlink: true,
    ...termOptions,
    localEcho: {
      enabled: true,
      prompt: () => {
        return formatePath(currentDirectory);
      },
      isInteractiveCommand:
        terminalTransport || normalizeOptions.writeCommandInput
          ? (normalizeOptions.isInteractiveCommand ?? isInteractiveShellCommand)
          : undefined,
      onCancelCommand:
        normalizeOptions.cancelCommand ?? (() => terminalTransport?.cancel()),
      onCommandInput:
        normalizeOptions.writeCommandInput ??
        ((data) => terminalTransport?.write(data)),
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
          gapCells: termOptions?.agentTipsGapCells || 1,
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
  const resizeTerminal =
    normalizeOptions.resizeTerminal ??
    (terminalTransport
      ? (size: TerminalSize) => terminalTransport.resize(size)
      : undefined);
  const resizeDisposable = resizeTerminal
    ? termInstance.terminal.onResize((size) => {
        void resizeTerminal(size);
      })
    : undefined;
  const sessionDisposable = terminalTransport?.onSessionChange((session) => {
    setCurrentDirectory(session.currentDirectory);
  });
  termInstance.terminal.write(formatePath(currentDirectory));
  void terminalTransport?.connect().catch((error) => {
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
  }

  return {
    dispose() {
      disposed = true;
      sessionDisposable?.dispose();
      resizeDisposable?.dispose();
      terminalTransport?.dispose();
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

function createTerminalTransport(connection?: TerminalConnectionOptions) {
  if (!connection) {
    return undefined;
  }
  if (connection.transport) {
    return connection.transport;
  }
  if (!connection.url) {
    throw new Error("connection 必须提供 url 或 transport");
  }

  return createSocketIoTerminalTransport(connection.url, {
    commandTimeoutMs: connection.commandTimeoutMs,
    socketOptions: connection.socketOptions,
  });
}

function toError(error: unknown) {
  return error instanceof Error ? error : new Error(String(error));
}
