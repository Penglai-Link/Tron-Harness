# @tron/chat-terminal

基于 xterm.js 的可嵌入 AI 终端。SDK 提供命令执行、交互式 PTY 输入、终端尺寸同步、AI 流式消息、命令卡片和输出解释。

默认使用 Socket.IO 传输终端命令，使用 SSE 传输 AI 消息。终端和 AI 传输都可以替换。

## 安装

```bash
npm install @tron/chat-terminal
```

## 快速接入

终端容器必须有明确的宽高。

```ts
import {
  createAITransport,
  createTerminalTransport,
  initTerminal,
} from "@tron/chat-terminal";
import "@tron/chat-terminal/styles.css";

const el = document.querySelector("#terminal");
if (!(el instanceof HTMLElement)) {
  throw new Error("终端容器不存在");
}

const serverUrl = "http://127.0.0.1:8787";
const terminal = initTerminal({
  el,
  transport: {
    terminal: createTerminalTransport({ url: serverUrl }),
    ai: createAITransport({
      url: `${serverUrl}/api/llm/v1/chat/completions`, // sse和非sse共用一个端点，后端需要通过stream参数进行区分
      headers: () => ({ Authorization: "Bearer <token>" }),
    }),
  },
  prompts: {
    chat: (content) => content, // 生成ai会话的提示词参数
    explain: ({ command, output }) =>
      `解释命令：${command}\n\n命令输出：\n${output}`, // 生成命令解释的提示词参数
    generate: (content) => content, // 生成命令的提示词参数
  },
  terminalOptions: {
    // 终端主题色
    themeTokens: {
        primary: "var(--color-on-surface, black)", // 主色
        secondary: "var(--color-header-backdrop, white)", // 次要色
        cardBackground: "var(--color-elevated-surface, white)", // 卡片背景色
        border: "var(--color-card-border, gray)",// 边框
    },
  },
  locale: "zh-CN", // 设置终端多语言环境
});

// 页面销毁时调用
terminal.dispose();
```

## Tron 后端接入

Tron 的终端和 AI 能力使用不同的后端通道。组件先通过适配器获取一次性 Socket.IO 票据，再连接终端命名空间；不要把登录 Token 或 OpenAPI HMAC 凭据放进 Socket.IO 握手。

| SDK 能力 | 后端通道 | 内部接口 | 开放接口 |
| --- | --- | --- | --- |
| 执行命令、交互输入、取消、尺寸同步 | Socket.IO `/terminals` | `POST /api/v1/terminals/socket-tickets` | `POST /open-api/v1/terminals/socket-tickets` |
| `commandGenerate` | REST，工作区文本模型 | `POST /api/v1/terminals/commands/generate` | `POST /open-api/v1/terminals/commands/generate` |
| `commandExplain`、终端聊天 | ChatService SSE | `POST /api/v1/chats/sessions/{sessionCode}/messages/stream` | `POST /open-api/v1/chats/sessions/{sessionCode}/messages/stream` |

内部前端应使用 `@tron/chat-internal-adapter`，而不是在终端组件中自行 `fetch` 终端接口：

```ts
import { createInternalChatAdapter, createInternalTerminalAdapter } from "@tron/chat-internal-adapter";
import { createTerminalTransport, initTerminal } from "@tron/chat-terminal";

const workspaceCode = "TRON_CORE";
const appCode: string | null = null; // null uses TRON_TERMINAL_WORKING_DIRECTORY
const accessToken = "<access-token>";
const adapterOptions = {
  getAccessToken: () => accessToken,
};
const chatAdapter = createInternalChatAdapter(adapterOptions);
const terminalAdapter = createInternalTerminalAdapter(adapterOptions);
const session = await chatAdapter.createSession({
  sourceType: "TERMINAL",
  workspaceCode,
});
const connection = await terminalAdapter.createSocketTicket({ appCode, workspaceCode });

const terminal = initTerminal({
  el,
  transport: {
    terminal: createTerminalTransport({
      url: connection.socketUrl,
      socketOptions: {
        auth: { ticket: connection.ticket },
        path: connection.socketPath,
        reconnection: false,
      },
    }),
    ai: {
      async stream(content, { onChunk, onEvent, signal }) {
        const requestId = crypto.randomUUID();
        await chatAdapter.streamMessage(session, { content, workspaceCode }, {
          onDelta: (delta) => onChunk({ delta, requestId }),
          onAguiEvent: onEvent,
        }, signal);
        return { requestId };
      },
      async request(content, { signal } = {}) {
        const result = await terminalAdapter.generateCommand({
          prompt: content,
          workspaceCode,
        }, signal);
        return result.command;
      },
    },
  },
});

// Close the terminal first, then remove its transient ChatService session.
terminal.dispose();
await chatAdapter.deleteSession(session.sessionCode);
```

`appCode` 为可选字段。提供时服务端以应用的 `localDirectory` 作为 cwd；为 `null` 或未提供时服务端以 `TRON_TERMINAL_WORKING_DIRECTORY` 作为 cwd，客户端不能自行提交目录。终端创建会话时必须传 `sourceType: "TERMINAL"`：服务端不会把它返回给 Chat Management，终端关闭时必须调用 `deleteSession` 清理会话、消息和关联的短期记忆。开放应用使用 `createOpenApiTerminalAdapter`。它会为两个 REST 调用签名；拿到的 `ticket` 仍仅用于一次 Socket.IO 连接。票据 60 秒失效且服务端消费后不可重用，因此连接断开后应重新签发票据并创建新的终端 transport。


### `initTerminal` 选项

| 选项 | 类型 | 说明 |
| --- | --- | --- |
| `el` | `HTMLElement` | 终端挂载元素。 |
| `transport.terminal` | `TerminalTransport` | 必填。命令执行传输。 |
| `transport.ai` | `AITransport` | 可选。未配置时不启用 AI 能力。 |
| `prompts` | `TerminalPrompts` | 配置聊天、解释和命令生成请求。 |
| `terminalOptions` | xterm.js 选项及 SDK 扩展 | 主题、字体、`agentTips` 等。 |
| `isCommandInputEnabled` | `(command: string) => boolean` | 控制已路由到 shell 的命令是否接收键盘输入；默认由内置检测器决定。 |
| `isInteractiveCommand` | `(command: string) => boolean` | 标记全屏终端程序，保留原始 ANSI 输出并隐藏终端辅助 UI。 |
| `locale` | `"zh-CN" \| "en-US"` | 默认 `"zh-CN"`。 |
| `messages` | `ChatTerminalMessageOverrides` | 覆盖内置文案。 |
| `onError` | `(error: Error) => void` | 连接失败等 transport 错误回调。 |

## 使用内置 transport

### 终端 Socket.IO

`createTerminalTransport` 创建并管理 Socket.IO 客户端：

```ts
const terminalTransport = createTerminalTransport({
  url: "https://terminal.example.com",
  commandTimeoutMs: 60_000,
  socketOptions: {
    auth: { token: accessToken },
    path: "/custom/socket.io",
    query: { tenantId: "tenant-1" },
    transports: ["websocket"],
  },
});
```

`socketOptions` 会传给 `socket.io-client`。如果只是需要认证、path、query 或连接参数，使用这个配置即可。

### AI SSE

使用`createAITransport` 进行创建的话，同一个后端接口需要区分流式和非流式请求(流式接口用于终端流式输出，非流式用于代码生成)，请求参数如下：

```json
{
  "messages": [{ "content": "...", "role": "user" }],
  "stream": true
}
```

可使用 `body` 替换请求体，`headers` 添加请求头：

```ts
const aiTransport = createAITransport({
  url: "/api/llm/v1/chat/completions",
  headers: { Authorization: "Bearer <token>" },
  body: ({ content, stream }) => ({
    messages: [{ content, role: "user" }],
    model: "your-model",
    stream,
  }),
});
```

## 自定义transport

### 自定义AITransport
```ts
import type { AITransport } from "@tron/chat-terminal";

// 实现AITransport接口
const aiTransport: AITransport = {
  async stream(content, { onChunk, signal }) {
    const requestId = crypto.randomUUID();
    const response = await fetch("/api/llm", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ content }),
      signal,
    });
    if (!response.ok) throw new Error("AI 请求失败");
    const body = (await response.json()) as TerminalMessage;
    if (body.type === "TEXT_MESSAGE_ERROR") {
      throw new Error(body.delta || "AI 请求失败");
    }
    onChunk({ delta: body.delta, requestId });
    return { requestId };
  },

  async request(content, { signal } = {}) {
    const response = await fetch("/api/llm", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ content }),
      signal,
    });
    if (!response.ok) throw new Error("AI 请求失败");
    const body = (await response.json()) as TerminalMessage;
    if (body.type === "TEXT_MESSAGE_ERROR") {
      throw new Error(body.delta || "AI 请求失败");
    }
    return body.delta;
  },
};
```

### 自定义 TerminalTransport

```ts
import type {
  TerminalCommandOptions,
  TerminalCommandResult,
  TerminalSession,
  TerminalSize,
  TerminalMessage,
  TerminalTransport,
  TransportDisposable,
} from "@tron/chat-terminal";

type Socket = {
  connect(): void;
  close(): void;
  send(event: string, payload: unknown): void;
  on(event: string, listener: (payload: any) => void): () => void;
};

// 由业务侧创建 WebSocket、Socket.IO 或其他 socket 实现。
declare function createSocket(): Socket;

type PendingCommand = {
  interactive: boolean;
  onOutput: (output: string) => void;
  resolve: (result: TerminalCommandResult) => void;
  reject: (error: Error) => void;
  timeout?: number;
};

// 实现TerminalTransport类型
export class CustomTerminalTransport implements TerminalTransport {
  private readonly pending = new Map<string, PendingCommand>();
  private readonly sessionListeners = new Set<
    (session: TerminalSession) => void
  >();
  private activeMessageId?: string;
  private session?: TerminalSession;
  private latestSize?: TerminalSize;
  private disposed = false;
  private connecting?: Promise<TerminalSession>;
  private resolveConnect?: (session: TerminalSession) => void;
  private rejectConnect?: (error: Error) => void;

  constructor(private readonly socket: Socket) {
    socket.on("terminal:session", (payload) => this.handleSession(payload));
    socket.on("command:message", (payload) => this.handleMessage(payload));
    socket.on("disconnect", () => this.handleDisconnect());
  }

  connect(): Promise<TerminalSession> {
    if (this.disposed) return Promise.reject(new Error("终端已销毁"));
    if (this.session) return Promise.resolve(this.session);
    if (this.connecting) return this.connecting;

    this.connecting = new Promise<TerminalSession>((resolve, reject) => {
      this.resolveConnect = resolve;
      this.rejectConnect = reject;
    });
    this.socket.connect();
    return this.connecting;
  }

  async execute(
    command: string,
    onOutput: (output: string) => void,
    options: TerminalCommandOptions = {},
  ): Promise<TerminalCommandResult> {
    await this.connect();
    const messageId = crypto.randomUUID();
    const interactive = options.interactive === true;

    return new Promise((resolve, reject) => {
      const timeoutMs = options.timeoutMs ?? 60_000;
      const timeout = !interactive && timeoutMs > 0
        ? window.setTimeout(() => {
            this.socket.send("command:cancel", { messageId });
            this.rejectCommand(messageId, new Error("命令执行超时"));
          }, timeoutMs)
        : undefined;

      this.pending.set(messageId, {
        interactive,
        onOutput,
        resolve,
        reject,
        timeout,
      });
      this.activeMessageId = messageId;
      this.socket.send("command:execute", {
        command,
        messageId,
        ...(interactive ? { interactive: true } : {}),
      });
    });
  }

  cancel() {
    const messageId = this.activeMessageId;
    if (messageId && this.pending.has(messageId)) {
      this.socket.send("command:cancel", { messageId });
    }
  }

  write(data: string) {
    const messageId = this.activeMessageId;
    const command = messageId ? this.pending.get(messageId) : undefined;
    if (data && messageId && command?.interactive) {
      this.socket.send("command:input", { data, messageId });
    }
  }

  resize(size: TerminalSize) {
    this.latestSize = size;
    if (this.session) this.socket.send("terminal:resize", size);
  }

  onSessionChange(listener: (session: TerminalSession) => void): TransportDisposable {
    this.sessionListeners.add(listener);
    if (this.session) listener(this.session);
    return { dispose: () => this.sessionListeners.delete(listener) };
  }

  dispose() {
    if (this.disposed) return;
    this.disposed = true;
    const error = new Error("终端连接已关闭");
    this.rejectConnect?.(error);
    for (const messageId of this.pending.keys()) {
      this.rejectCommand(messageId, error);
    }
    this.sessionListeners.clear();
    this.socket.close();
  }

  private handleSession(payload: TerminalMessage) {
    if (
      payload.type === "TEXT_MESSAGE_ERROR" &&
      payload.delta
    ) {
      const error = new Error(payload.delta);
      this.rejectConnect?.(error);
      this.connecting = undefined;
      this.rejectPending(error);
      return;
    }
    if (
      payload.type !== "TEXT_MESSAGE_CONTENT" ||
      payload.delta !== "" ||
      !payload.currentDirectory
    ) {
      return;
    }
    this.session = { currentDirectory: payload.currentDirectory };
    this.connecting = undefined;
    this.resolveConnect?.(this.session);
    for (const listener of this.sessionListeners) listener(this.session);
    if (this.latestSize) this.socket.send("terminal:resize", this.latestSize);
  }

  private handleDisconnect() {
    const error = new Error("终端连接已断开");
    this.session = undefined;
    this.connecting = undefined;
    this.rejectConnect?.(error);
    this.rejectPending(error);
  }

  private handleMessage(message: {
    type: string;
    messageId: string;
    delta: string;
    currentDirectory?: string;
  }) {
    const command = this.pending.get(message.messageId);
    if (!command) return;
    if (message.type === "TEXT_MESSAGE_CONTENT") command.onOutput(message.delta);
    if (message.type === "TEXT_MESSAGE_END") {
      this.resolveCommand(message.messageId, {
        currentDirectory: message.currentDirectory,
      });
    }
    if (message.type === "TEXT_MESSAGE_ERROR") {
      this.rejectCommand(message.messageId, new Error(message.delta || "命令执行失败"));
    }
  }

  private resolveCommand(messageId: string, result: TerminalCommandResult) {
    const command = this.pending.get(messageId);
    if (!command) return;
    if (command.timeout !== undefined) window.clearTimeout(command.timeout);
    this.pending.delete(messageId);
    if (this.activeMessageId === messageId) this.activeMessageId = undefined;
    command.resolve(result);
  }

  private rejectCommand(messageId: string, error: Error) {
    const command = this.pending.get(messageId);
    if (!command) return;
    if (command.timeout !== undefined) window.clearTimeout(command.timeout);
    this.pending.delete(messageId);
    if (this.activeMessageId === messageId) this.activeMessageId = undefined;
    command.reject(error);
  }

  private rejectPending(error: Error) {
    for (const messageId of this.pending.keys()) {
      this.rejectCommand(messageId, error);
    }
  }
}

initTerminal({
  el,
  transport: { terminal: new CustomTerminalTransport(createSocket()) },
});
```

自定义实现需要保证：

- `connect()` 在收到初始 session 后 resolve，并返回 `currentDirectory`。
- `execute()` 持有 `messageId`，在内容消息中调用 `onOutput`，在结束消息中 resolve。
- 交互命令的 `options.interactive` 为 `true` 时，不设置普通命令超时。
- `cancel()`、`write()` 和 `resize()` 映射到业务 socket 协议。
- `dispose()` 关闭 socket，并 reject 所有未完成命令。

## Socket.IO 协议

### 响应数据结构
成功响应：

```json
{
  "type": "TEXT_MESSAGE_CONTENT",
  "messageId": "session-1",
  "delta": "",
  "timestamp": 1710000000000,
  "currentDirectory": "/workspace" // 可选
}
```
失败响应：

```json
{
  "type": "TEXT_MESSAGE_ERROR",
  "messageId": "session-1",
  "delta": "终端会话创建失败",
  "timestamp": 1710000000000
}
```
```ts
type TerminalMessageType =
  | "TEXT_MESSAGE_START"
  | "TEXT_MESSAGE_CONTENT"
  | "TEXT_MESSAGE_END"
  | "TEXT_MESSAGE_ERROR"
  | "TEXT_MESSAGE_SUGGESTION";

interface TerminalMessage {
  type: TerminalMessageType;
  messageId: string;
  delta: string;
  timestamp: number;
  currentDirectory?: string;
}
```

字段约定：

- `messageId` 标识一次命令或 AI 请求。一次流式响应中的所有消息帧使用同一个值。
- `delta` 是终端输出、AI 文本或错误信息。`START` 和 `END` 通常为空字符串。
- `currentDirectory` 在会话成功消息和命令执行结束消息中返回，用于更新终端提示符目录。
- `TEXT_MESSAGE_SUGGESTION` 的 `delta` 保存 AI 生成的可执行命令。
- `TEXT_MESSAGE_ERROR` 的 `delta` 保存错误信息。

客户端事件：

| 事件 | payload |
| --- | --- |
| `command:execute` | `{ command, messageId, interactive? }` |
| `command:input` | `{ data, messageId }` |
| `command:cancel` | `{ messageId }` |
| `terminal:resize` | `{ cols, rows }` |

服务端通过 `command:message` 返回 `TerminalMessage`：

正常顺序为 `START`、零次或多次 `CONTENT`、`END`。错误使用 `TEXT_MESSAGE_ERROR`，错误文本放在 `delta`。

AI SSE 的每个 `data:` 帧也使用 `TerminalMessage` 格式，并以 `data: [DONE]` 结束。

## 返回实例

`initTerminal` 返回的实例支持：

| 方法 | 说明 |
| --- | --- |
| `dispose()` | 销毁终端、事件监听器和 transport。 |
| `setCurrentDirectory(path)` | 更新终端提示符目录。 |
| `appendChatChunk(response, ids?)` | 追加 AI 流式响应。 |
| `completeChat(requestId?)` | 完成 AI 会话。 |
| `setThemeTokens(tokens)` | 增量覆盖主题 token。 |
| `resetTheme()` | 清除主题覆盖。 |
| `refreshTheme()` | 重新同步主题。 |
| `setLocale(locale, messages?)` | 切换语言并覆盖文案。 |
| `setMessages(messages)` | 增量覆盖文案。 |

主题覆盖字段为 `primary`、`secondary`、`cardBackground`、`border` 和可选的 xterm `ansi` 调色板。

AI 回复中的命令块支持以下格式：

```xml
<execute_command>
<command>git status --short</command>
</execute_command>
```

用户点击执行后，SDK 才会调用命令 transport，不会自动执行模型返回的命令。

## 主题设置
```ts
// 主题发生变更的时候调用,更新终端主题色
terminal.refreshTheme();
```

## 多语言设置
```ts
// 多语言发生变更的时候调用,更新终端多语言
terminal.setLocale(currentLocale);
```
