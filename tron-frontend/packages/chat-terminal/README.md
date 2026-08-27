# @tron/chat-terminal

基于 xterm.js 的可嵌入 AI 终端。SDK 提供真实 PTY 终端、命令 block、AI 流式消息、命令卡片和输出解释。

终端按**字节流**工作：按键原样送进 PTY，shell 的输出原样回传。PTY 是唯一权威——回显、行编辑、提示符渲染都由 shell 负责，SDK 不再本地回显、不再封帧命令。命令边界由 shell 通过 OSC 133 语义标记自己上报，因此 `vim`、`top`、`ssh` 等程序原生透传，block 与 AI 在 `ssh` 远端同样可用。

默认使用 Socket.IO 传输终端字节流，使用 SSE 传输 AI 消息。终端和 AI 传输都可以替换。

## 服务端要求

服务端必须为终端安装 shell integration，否则 SDK 退化为普通终端（无 block、无提示符态 AI 触发）：

- shell 为 `bash` 或 `zsh`。`sh` 没有 precmd/preexec 钩子，会以 `supportsBlocks: false` 降级。
- 不要关闭 shell 的回显或清空 `PS1`。
- hook 脚本需上报下表的标记，并且**不能覆盖用户自己的 rc**（bash 用 `--init-file`、zsh 用临时 `ZDOTDIR`，其中先 source 用户真实配置再装 hook）。
- Windows 暂不支持，需在 WSL 中运行。

| 标记 | 含义 |
| --- | --- |
| `OSC 133 ; A ST` | 提示符开始 |
| `OSC 133 ; B ST` | 提示符结束，开始接收命令输入 |
| `OSC 133 ; C ST` | 命令输出开始 |
| `OSC 133 ; D ; <exit> ST` | 命令结束及退出码 |
| `OSC 633 ; E ; <line> ST` | 即将执行的命令原文 |
| `OSC 7 ; file://<host><cwd> ST` | 工作目录变更 |

参考实现见 `server/shell-integration.sh` 与 `server/shellIntegration.js`；Tron 后端使用同一份脚本。

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
| `agentPrefix` | `string` | AI 显式触发前缀，默认 `"#"`。行首命中时该行走 AI 而不执行。 |
| `shouldRouteToAgent` | `(input: string) => boolean` | 覆盖启发式兜底判定。返回 `true` 时该行走 AI。 |
| `onExit` | `(exitCode: number \| null) => void` | shell 进程自身退出时触发，宿主可据此关闭标签页。 |
| `locale` | `"zh-CN" \| "en-US"` | 默认 `"zh-CN"`。 |
| `messages` | `ChatTerminalMessageOverrides` | 覆盖内置文案。 |
| `onError` | `(error: Error) => void` | 连接失败等 transport 错误回调。 |

## 使用内置 transport

### 终端 Socket.IO

`createTerminalTransport` 创建并管理 Socket.IO 客户端：

```ts
const terminalTransport = createTerminalTransport({
  url: "https://terminal.example.com",
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
  TerminalSession,
  TerminalSignalName,
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

// 实现 TerminalTransport：只搬运字节，不解析输出。
export class CustomTerminalTransport implements TerminalTransport {
  private readonly outputListeners = new Set<(data: string) => void>();
  private readonly exitListeners = new Set<(code: number | null) => void>();
  private readonly errorListeners = new Set<(error: Error) => void>();
  private readonly sessionListeners = new Set<
    (session: TerminalSession) => void
  >();
  private session?: TerminalSession;
  private latestSize?: TerminalSize;
  private disposed = false;
  private connecting?: Promise<TerminalSession>;
  private resolveConnect?: (session: TerminalSession) => void;
  private rejectConnect?: (error: Error) => void;

  constructor(private readonly socket: Socket) {
    socket.on("terminal:session", (payload) => this.handleSession(payload));
    socket.on("terminal:output", ({ data }) => {
      for (const listener of this.outputListeners) listener(data);
    });
    socket.on("terminal:exit", ({ exitCode }) => {
      for (const listener of this.exitListeners) listener(exitCode ?? null);
    });
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

  // 按键原样送进 PTY：不本地回显，也不封帧。
  input(data: string) {
    if (data && this.session) this.socket.send("terminal:input", { data });
  }

  // 写进 PTY 而不是发给子进程，这样当前前台任务（含 ssh 里的）才收得到。
  signal(name: TerminalSignalName) {
    if (this.session) this.socket.send("terminal:signal", { signal: name });
  }

  resize(size: TerminalSize) {
    this.latestSize = size;
    if (this.session) this.socket.send("terminal:resize", size);
  }

  onOutput(listener: (data: string) => void): TransportDisposable {
    this.outputListeners.add(listener);
    return { dispose: () => this.outputListeners.delete(listener) };
  }

  onSessionChange(listener: (session: TerminalSession) => void): TransportDisposable {
    this.sessionListeners.add(listener);
    if (this.session) listener(this.session);
    return { dispose: () => this.sessionListeners.delete(listener) };
  }

  onExit(listener: (code: number | null) => void): TransportDisposable {
    this.exitListeners.add(listener);
    return { dispose: () => this.exitListeners.delete(listener) };
  }

  onError(listener: (error: Error) => void): TransportDisposable {
    this.errorListeners.add(listener);
    return { dispose: () => this.errorListeners.delete(listener) };
  }

  dispose() {
    if (this.disposed) return;
    this.disposed = true;
    this.rejectConnect?.(new Error("终端连接已关闭"));
    this.outputListeners.clear();
    this.sessionListeners.clear();
    this.exitListeners.clear();
    this.errorListeners.clear();
    this.socket.close();
  }

  private handleSession(payload: {
    currentDirectory?: string;
    shell?: string;
    supportsBlocks?: boolean;
    subshellBootstrap?: string;
    error?: string;
  }) {
    if (payload.error) {
      this.fail(new Error(payload.error));
      return;
    }
    if (!payload.currentDirectory) return;

    this.session = {
      currentDirectory: payload.currentDirectory,
      shell: payload.shell ?? "sh",
      // 未装 hook 时必须为 false：SDK 退化为普通终端，而不是画出空 block。
      supportsBlocks: payload.supportsBlocks === true,
      subshellBootstrap: payload.subshellBootstrap ?? "",
    };
    this.connecting = undefined;
    this.resolveConnect?.(this.session);
    for (const listener of this.sessionListeners) listener(this.session);
    if (this.latestSize) this.socket.send("terminal:resize", this.latestSize);
  }

  private handleDisconnect() {
    this.session = undefined;
    this.fail(new Error("终端连接已断开"));
  }

  private fail(error: Error) {
    this.connecting = undefined;
    this.rejectConnect?.(error);
    for (const listener of this.errorListeners) listener(error);
  }

}

initTerminal({
  el,
  transport: { terminal: new CustomTerminalTransport(createSocket()) },
});
```

自定义实现需要保证：

- `connect()` 在收到 session 后 resolve，返回 `currentDirectory`、`shell`、`supportsBlocks` 和 `subshellBootstrap`。
- `onOutput` 把 PTY 字节**原样**交给 SDK：不要按行缓冲、不要剥离 ANSI/OSC，否则 block 与 cwd 会丢。
- `input()` 原样透传按键，不做回显、不做行编辑、不封帧。全屏程序（`vim`、`top`）依赖这一点。
- `signal()` 把 `\x03` 写进 PTY，而不是发信号给子进程；否则 `ssh` 内的前台任务收不到 Ctrl-C。
- `resize()` 在 session 建立后补发最后一次尺寸。
- `onExit` 在 shell 进程退出时上报退出码，宿主据此关闭标签页。
- `dispose()` 关闭 socket 并清空所有监听器。

不需要实现命令超时：字节流模型下没有「一条命令」的生命周期，长任务和 `ssh` 会话本来就该一直开着。

## Socket.IO 协议

### 客户端 → 服务端

| 事件 | payload | 说明 |
| --- | --- | --- |
| `terminal:input` | `{ data: string }` | 按键原样写入 PTY，服务端应限制单帧大小（参考实现 64 KiB）。 |
| `terminal:signal` | `{ signal: "SIGINT" \| "SIGKILL" }` | `SIGINT` 写入 `\x03`；`SIGKILL` 结束 shell 进程。 |
| `terminal:resize` | `{ cols: number, rows: number }` | 正整数，服务端应校验上限。 |

### 服务端 → 客户端

| 事件 | payload | 说明 |
| --- | --- | --- |
| `terminal:session` | `{ currentDirectory, shell, supportsBlocks, subshellBootstrap }` | 每次连接一条。 |
| `terminal:session` | `{ error: string }` | 会话创建失败，`error` 为展示文案。 |
| `terminal:output` | `{ data: string }` | PTY 字节流，原样透传。 |
| `terminal:exit` | `{ exitCode: number \| null }` | shell 退出；被信号杀死时为 `null`。 |

```ts
interface TerminalSession {
  currentDirectory: string;
  /** `bash`、`zsh` 或 `sh`。 */
  shell: string;
  /** 为 false 时无 block、无提示符态 AI 触发。 */
  supportsBlocks: boolean;
  /** 注入远端 shell 的一行 bootstrap，用于 ssh 之后继续可用。 */
  subshellBootstrap: string;
}
```

约定：

- 没有命令级事件。命令边界完全来自输出流里的 OSC 133 标记，服务端不需要知道「当前在跑哪条命令」。
- `terminal:output` 不做缓冲也不做改写。UTF-8 多字节字符会跨 PTY 读边界，服务端必须保留尾部残字节 carry，不能对每次读取单独做 lossy 解码，否则中文输出必然乱码。
- `currentDirectory` 只在建立会话时给一个初值。此后 cwd 由 shell 通过 `OSC 7` 自行上报，服务端不再参与。
- 每个 socket 一个 PTY。断开即回收 PTY 与生成的临时 rc 目录。

AI SSE 的每个 `data:` 帧使用 `TerminalMessage` 格式，并以 `data: [DONE]` 结束。

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
