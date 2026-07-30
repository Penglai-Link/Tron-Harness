import "@xterm/xterm/css/xterm.css";
import { Terminal, type CommandExplain } from "./terminal";
import { formatePath, isShellCommand } from "./terminal/utils/common";
import { ITerminalInitOnlyOptions, ITerminalOptions } from "@xterm/xterm";

export type {
  CommandExplain,
  CommandExplainContext,
  CommandExplainPayload,
} from "./terminal";

export interface IOptions {
  el: HTMLElement;
  termOptions?: ITerminalInitOnlyOptions &
    ITerminalOptions & {
      explainTitle?: string;
      agentTips?: string;
      agentTipsGapCells?: number;
    };
  runCommand: (
    command: string,
    onOutput: (output: string) => void,
  ) => Promise<void>;
  commandChat: (command: string) => Promise<void>;
  commandExplain: CommandExplain;
  commandGenerate: (content: string) => Promise<string>;
}

const defaultOptions: Partial<IOptions> = {};

export function initTerminal(options: IOptions) {
  const normalizeOptions = { ...defaultOptions, ...options };
  const { termOptions } = normalizeOptions;
  let currentDirectory = "/";

  async function runCommandByIo(
    command: string,
    onOutput: (output: string) => void = (output) => {
      termInstance.terminal.write(output);
    },
  ) {
    const input = command.trim();
    if (!input) {
      return;
    }
    try {
      await normalizeOptions.runCommand(input, onOutput);
    } catch (error) {
      onOutput(`${error}\r\n`);
    }
  }

  const termInstance = new Terminal(normalizeOptions.el, {
    ...termOptions,
    localEcho: {
      enabled: true,
      prompt: () => {
        return formatePath(currentDirectory);
      },
      // 执行命令前判断，系统命令直接执行，非系统命令调用大模型
      onBeforeCommand: async (command) => {
        // 系统命令
        if (isShellCommand(command)) {
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
      explainTitle: termOptions?.explainTitle || "",
    },
    agentPrompt: {
      text:
        termOptions?.agentTips ||
        "Enter natural language or commands, AI will intelligently respond. Give it a try by saying hello",
      gapCells: termOptions?.agentTipsGapCells || 1,
    },
    modelSession: {
      // 点击直接命令
      onExecuteCommand: ({ command }) => {
        runCommandByIo(command);
      },
      // 编辑命令成功回调
      onModifyCommand: async ({ command }) => command.trim(),
      // AI生成命令
      onGenerateCommand: ({ command }) => {
        return normalizeOptions.commandGenerate(command);
      },
    },
  });
  termInstance.terminal.write(formatePath(currentDirectory));

  return {
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
  };
}
