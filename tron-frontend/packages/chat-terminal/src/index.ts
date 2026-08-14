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

export interface IOptions {
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
  runCommand: (
    command: string,
    onOutput: (output: string) => void,
    context: CommandExecutionContext,
  ) => Promise<CommandExecutionResult | void>;
  cancelCommand?: () => void;
  isInteractiveCommand?: (command: string) => boolean;
  resizeTerminal?: (size: TerminalSize) => Promise<void> | void;
  writeCommandInput?: (data: string) => Promise<void> | void;
  commandChat: (command: string) => Promise<void>;
  commandExplain: CommandExplain;
  commandGenerate: (content: string) => Promise<string>;
}

export interface CommandExecutionResult {
  currentDirectory?: string;
}

export interface TerminalSize {
  cols: number;
  rows: number;
}

const defaultOptions: Partial<IOptions> = {};

export function initTerminal(options: IOptions) {
  const normalizeOptions = { ...defaultOptions, ...options };
  const { termOptions } = normalizeOptions;
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
      const result = await normalizeOptions.runCommand(input, onOutput, context);
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
      isInteractiveCommand: normalizeOptions.writeCommandInput
        ? (normalizeOptions.isInteractiveCommand ?? isInteractiveShellCommand)
        : undefined,
      onCancelCommand: normalizeOptions.cancelCommand,
      onCommandInput: normalizeOptions.writeCommandInput,
      // 执行命令前判断，系统命令直接执行，非系统命令调用大模型
      onBeforeCommand: async (command) => {
        if (!command.trim()) {
          return false;
        }
        // 系统命令
        if (isShellCommand(command.toLocaleLowerCase())) {
          return false;
        }
        await normalizeOptions.commandChat(command);
        // 调用大模型
        return true;
      },
      onCommand: runCommandByIo,
      // 结果解释
      onExplain: (payload, context) =>
        normalizeOptions.commandExplain(payload, context),
      i18n,
    },
    agentPrompt: {
      i18n,
      textKey: "agentTips",
      gapCells: termOptions?.agentTipsGapCells || 1,
    },
    modelSession: {
      i18n,
      // 点击直接命令
      onExecuteCommand: ({ command }) => {
        return termInstance.executeCommand(command);
      },
      // 编辑命令成功回调
      onModifyCommand: async ({ command }) => command.trim(),
      // AI生成命令
      onGenerateCommand: ({ command }) => {
        return normalizeOptions.commandGenerate(command);
      },
    },
  });
  const resizeDisposable = normalizeOptions.resizeTerminal
    ? termInstance.terminal.onResize((size) => {
        void normalizeOptions.resizeTerminal?.(size);
      })
    : undefined;
  termInstance.terminal.write(formatePath(currentDirectory));

  return {
    dispose() {
      resizeDisposable?.dispose();
      termInstance.dispose();
    },
    setCurrentDirectory(nextDirectory: string) {
      currentDirectory = nextDirectory;
      termInstance.terminal.write(`\r\x1b[2K${formatePath(currentDirectory)}`);
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
