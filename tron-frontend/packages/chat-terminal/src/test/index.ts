import { createFetchTransport } from "@tron/chat-core";
import { initTerminal } from "..";
import {
  parseTerminalMessage,
  TerminalSocketClient,
  type TerminalMessage,
} from "./socketClient";
import {
  explainResultMessage,
  generateAiChatMessage,
  generateCommandMessage,
} from "./prompt";
import "./style.css";

const MODEL = "Qwen3-32B";
const API_BASE_URL = `http://${window.location.hostname}:8787/`;
const API_TOKEN = "Bearer sk-42f0b5eb8a0043509c6c48270314c90d";

const transport = createFetchTransport({
  baseUrl: API_BASE_URL,
  credentials: "omit",
  getHeaders: () => ({
    Authorization: API_TOKEN,
    "Content-Type": "application/json",
  }),
  unwrapEnvelope: false,
});

const terminalClient = new TerminalSocketClient(API_BASE_URL);

window.addEventListener("beforeunload", () => terminalClient.destroy(), {
  once: true,
});

terminalClient.connect().catch((error) => {
  console.warn(error);
});

async function chat(content: string, scope: string, signal?: AbortSignal) {
  const messageId = createMessageId(scope);
  const payload = await transport.requestJson<TerminalMessage>({
    body: {
      messageId,
      messages: [{ content, role: "user" }],
      model: MODEL,
      stream: false,
    },
    method: "POST",
    path: "api/llm/v1/chat/completions",
    signal,
  });
  const message = parseTerminalMessage(payload);

  return message?.delta || "";
}

async function chatSSE(command: string) {
  const messageId = createMessageId("chat");
  try {
    await transport.streamSse<TerminalMessage>({
      path: "api/llm/v1/chat/completions",
      body: {
        messageId,
        messages: [{ content: generateAiChatMessage(command), role: "user" }],
        model: MODEL,
        stream: true,
      },
      handlers: {
        onFrame: (_eventName, payload) => {
          const message = parseTerminalMessage(payload);
          if (
            message?.messageId === messageId &&
            message.type === "TEXT_MESSAGE_CONTENT" &&
            message.delta
          ) {
            terminalView.appendChatChunk(message.delta, {
              requestId: messageId,
            });
          }
        },
      },
      onResult: (payload) => {
        const message = parseTerminalMessage(payload);
        if (
          message?.type === "TEXT_MESSAGE_END" ||
          message?.type === "TEXT_MESSAGE_ERROR"
        ) {
          return message;
        }
        return null;
      },
    });
  } finally {
    await terminalView.completeChat(messageId);
  }
}

function createMessageId(scope: string) {
  return `${scope}-${globalThis.crypto?.randomUUID?.() ?? Date.now()}`;
}

const dom = document.createElement("div");
dom.style.width = "100%";
dom.style.height = "100%";
dom.style.padding = "20px";
dom.style.boxSizing = "border-box";
dom.style.background = "#000";

const terminalView = initTerminal({
  el: dom,
  runCommand: (command, onOutput) => terminalClient.execute(command, onOutput),
  // ai对话
  commandChat: chatSSE,
  // 解释输出结果
  commandExplain: ({ command, output }, { signal }) =>
    chat(explainResultMessage(command, output), "explain", signal),
  // 根据命令重新生成命令
  commandGenerate: (content) =>
    chat(generateCommandMessage(content), "generate"),
});

document.body.appendChild(dom);
