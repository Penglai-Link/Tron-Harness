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
      url: `${serverUrl}/api/llm/v1/chat/completions`,
      headers: () => ({ Authorization: "Bearer <token>" }),
    }),
  },
  prompts: {
    chat: (content) => content,
    explain: ({ command, output }) =>
      `解释命令：${command}\n\n命令输出：\n${output}`,
    generate: (content) => content,
  },
  locale: "zh-CN",
});

// 页面销毁时调用 
terminal.dispose()

// 多语言发生变更的时候调用
terminal.setLocale(currentLocale)

// 主题发生变更的时候调用
terminal.refreshTheme()

```


### `initTerminal` 选项

| 选项 | 类型 | 说明 |
| --- | --- | --- |
| `el` | `HTMLElement` | 终端挂载元素。 |
| `transport.terminal` | `TerminalTransport` | 必填。命令执行传输。 |
| `transport.ai` | `AITransport` | 可选。未配置时不启用 AI 能力。 |
| `prompts` | `TerminalPrompts` | 配置聊天、解释和命令生成请求。 |
| `terminalOptions` | xterm.js 选项及 SDK 扩展 | 主题、字体、`agentTips` 等。 |
| `isInteractiveCommand` | `(command: string) => boolean` | 覆盖交互式命令判断(解析输入的内容是否是内置命令)。 |
| `locale` | `"zh-CN" \| "en-US"` | 默认 `"zh-CN"`。 |
| `messages` | `ChatTerminalMessageOverrides` | 覆盖内置文案。 |
| `onError` | `(error: Error) => void` | 连接失败等 transport 错误回调。 |

## 内置 transport

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

`createAITransport` 使用 `fetch` 发起 POST 请求并解析 SSE。默认请求体为：

```json
{
  "messages": [{ "content": "...", "role": "user" }],
  "stream": true
}
```

使用 `body` 可以替换请求体，`headers` 支持静态值或异步工厂：

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

非 SSE 接口可以直接实现 `AITransport`。`stream` 至少调用一次 `onChunk`，并返回本次请求的 `requestId`；`request` 返回完整文本。

```ts
import type { AITransport } from "@tron/chat-terminal";

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
    const body = (await response.json()) as {
      code: string;
      message: string;
      data: { content: string };
    };
    if (body.code !== "0") throw new Error(body.message || "AI 请求失败");
    onChunk({ delta: body.data.content, requestId });
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
    const body = (await response.json()) as {
      code: string;
      message: string;
      data: { content: string };
    };
    if (body.code !== "0") throw new Error(body.message || "AI 请求失败");
    return body.data.content;
  },
};
```

## 自定义 TerminalTransport

自定义 WebSocket、Socket.IO 协议或已有 PTY 时，在业务侧实现 `TerminalTransport`，再传给 `transport.terminal`。不要把裸 socket 实例直接传入。

```ts
import type {
  TerminalCommandOptions,
  TerminalCommandResult,
  TerminalSession,
  TerminalSize,
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

  private handleSession(payload: {
    code: string;
    message: string;
    data?: { currentDirectory?: string };
  }) {
    if (payload.code !== "0" || !payload.data?.currentDirectory) {
      this.rejectConnect?.(new Error(payload.message || "终端会话失败"));
      this.connecting = undefined;
      this.rejectPending(new Error(payload.message || "终端会话失败"));
      return;
    }
    this.session = { currentDirectory: payload.data.currentDirectory };
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

## 交互式命令

内置 transport 默认将以下命令视为交互式命令：

```text
vim vi nvim nano emacs top htop less more man watch tmux screen
```

也可以覆盖判断：

```ts
initTerminal({
  el,
  transport: { terminal: createTerminalTransport({ url: serverUrl }) },
  isInteractiveCommand(command) {
    return /^(?:vim|nvim|fzf)(?:\s|$)/.test(command.trim());
  },
});
```

交互命令通过 `command:input` 发送原始键盘输入，通过 `terminal:resize` 同步 PTY 尺寸。

## Socket.IO 协议

会话事件 `terminal:session` 使用统一响应体：

```json
{
  "code": "0",
  "message": "",
  "data": { "currentDirectory": "/workspace" }
}
```

客户端事件：

| 事件 | payload |
| --- | --- |
| `command:execute` | `{ command, messageId, interactive? }` |
| `command:input` | `{ data, messageId }` |
| `command:cancel` | `{ messageId }` |
| `terminal:resize` | `{ cols, rows }` |

服务端通过 `command:message` 返回：

```ts
interface TerminalMessage {
  type:
    | "TEXT_MESSAGE_START"
    | "TEXT_MESSAGE_CONTENT"
    | "TEXT_MESSAGE_END"
    | "TEXT_MESSAGE_ERROR"
    | "TEXT_MESSAGE_SUGGESTION";
  messageId: string;
  delta: string;
  timestamp: number;
  currentDirectory?: string;
}
```

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
