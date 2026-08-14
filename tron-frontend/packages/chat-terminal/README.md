# @tron/chat-terminal

`@tron/chat-terminal` 基于 xterm.js 提供可嵌入的 AI 终端。SDK 负责终端渲染、命令输入分流、交互式 PTY 输入、终端尺寸同步、AI 消息展示、命令卡片和输出解释。

命令执行默认通过 Socket.IO transport 接入，AI 请求默认通过 SSE transport 接入。也可以使用自定义 transport 或逐项提供回调。

## 安装

```bash
npm install @tron/chat-terminal
```

```ts
import {
  createSseAITransport,
  initTerminal,
} from "@tron/chat-terminal";
import "@tron/chat-terminal/styles.css";
```

挂载元素必须有明确的宽高。SDK 会根据容器尺寸调整 xterm.js 的行列数，并在使用 Socket.IO transport 时同步 PTY 尺寸。

## 快速接入

下面的配置使用内置 transport。业务代码不需要直接创建 Socket.IO 客户端，也不需要手动处理命令消息或 SSE 帧。

```ts
const terminalServerUrl = "http://127.0.0.1:8787";
const terminalElement = document.querySelector("#terminal");

if (!(terminalElement instanceof HTMLElement)) {
  throw new Error("终端容器不存在");
}

const aiTransport = createSseAITransport({
  url: terminalServerUrl + "/api/llm/v1/chat/completions",
  headers: () => ({
    Authorization: "Bearer <token>",
  }),
});

const terminal = initTerminal({
  el: terminalElement,
  connection: {
    url: terminalServerUrl,
  },
  ai: {
    transport: aiTransport,
    prompts: {
      chat: (content) => content,
      explain: ({ command, output }) =>
        "解释命令：" + command + "\n\n命令输出：\n" + output,
      generate: (content) => content,
    },
  },
  locale: "zh-CN",
});

// 页面销毁时释放终端、Socket.IO 和 AI 请求。
terminal.dispose();
```

`connection.url` 会创建 `SocketIoTerminalTransport`。`ai.transport` 会创建 SSE 请求。默认请求体为：

```json
{
  "messages": [{ "content": "...", "role": "user" }],
  "stream": true
}
```

可以通过 `SseAITransportOptions.buildBody` 修改请求体，通过 `headers` 或异步 headers 工厂提供认证信息。

## 配置

`initTerminal` 接收 `ChatTerminalOptions`：

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | :---: | --- |
| `el` | `HTMLElement` | 是 | 终端挂载元素。 |
| `connection` | `TerminalConnectionOptions` | 与 `runCommand` 二选一 | 使用内置或自定义终端 transport。 |
| `ai` | `TerminalAIOptions` | 否 | 使用内置 AI transport 和提示词。 |
| `runCommand` | `(command, onOutput, context) => Promise<Result \| void>` | 与 `connection` 二选一 | 自定义命令执行逻辑。 |
| `cancelCommand` | `() => void` | 否 | 自定义取消当前命令。未配置时由终端 transport 处理。 |
| `isInteractiveCommand` | `(command) => boolean` | 否 | 覆盖交互式命令判断。 |
| `writeCommandInput` | `(data) => void \| Promise<void>` | 否 | 自定义 PTY stdin 写入。 |
| `resizeTerminal` | `(size) => void \| Promise<void>` | 否 | 自定义终端尺寸同步。 |
| `commandChat` | `(content) => Promise<void>` | 否 | 自定义自然语言处理。 |
| `commandExplain` | `({ command, output }, context) => Promise<string>` | 否 | 自定义命令输出解释。 |
| `commandGenerate` | `(content) => Promise<string>` | 否 | 自定义命令生成。 |
| `locale` | `"zh-CN" \| "en-US"` | 否 | SDK 固定文案语言，默认 `zh-CN`。 |
| `messages` | `ChatTerminalMessageOverrides` | 否 | 覆盖 SDK 固定文案。 |
| `termOptions` | xterm.js 配置和 SDK 扩展字段 | 否 | 终端主题、光标、字体等配置。 |
| `onError` | `(error: Error) => void` | 否 | 连接失败等 transport 错误回调。 |

未配置 `connection` 时，必须提供 `runCommand`。`ai` 和三个 AI 回调可以分别配置；未配置的能力不会启用对应 UI。

### 命令执行上下文

```ts
interface CommandExecutionContext {
  interactive: boolean;
}

interface CommandExecutionResult {
  currentDirectory?: string;
}
```

`interactive` 为 `true` 时，命令需要持续接收原始键盘输入。`runCommand` 应保持 Promise 直到进程退出，并通过 `onOutput` 传递输出。

### 终端 transport

```ts
interface TerminalConnectionOptions {
  url?: string;
  transport?: TerminalTransport;
  commandTimeoutMs?: number;
  socketOptions?: Record<string, unknown>;
}

interface TerminalTransport {
  connect(): Promise<{ currentDirectory: string }>;
  execute(
    command: string,
    onOutput: (output: string) => void,
    options?: { interactive?: boolean; timeoutMs?: number },
  ): Promise<{ currentDirectory?: string }>;
  cancel(): void;
  write(data: string): void;
  resize(size: { cols: number; rows: number }): void;
  onSessionChange(listener: (session: { currentDirectory: string }) => void): {
    dispose(): void;
  };
  dispose(): void;
}
```

`url` 和 `transport` 二选一。`url` 使用 Socket.IO；`transport` 适用于已有 WebSocket、PTY 或测试实现。

### 自定义 transport 示例

使用自定义 `transport` 时，不要同时配置 `connection.url`。`initTerminal` 会自动调用 `transport.connect()`，销毁终端时会调用 `transport.dispose()`。

如果自定义 transport 内部使用 Socket.IO 或 WebSocket，应在 transport 内部创建和管理 socket。不能把 socket 客户端直接传给 `connection.transport`，传入对象必须实现完整的 `TerminalTransport` 接口。

如果 `connection.transport` 需要对接 Socket.IO，可以直接使用 SDK 导出的
`createSocketIoTerminalTransport`。它会创建并管理 Socket.IO 客户端，同时实现
`TerminalTransport` 接口：

```ts
import {
  createSocketIoTerminalTransport,
  initTerminal,
} from "@tron/chat-terminal";

const terminalTransport = createSocketIoTerminalTransport(
  "http://127.0.0.1:8787",
  {
    commandTimeoutMs: 60_000,
    socketOptions: {
      auth: { token: "<token>" },
      transports: ["websocket"],
    },
  },
);

const terminal = initTerminal({
  el: terminalElement,
  connection: {
    transport: terminalTransport,
  },
});
```

此时不要再配置 `connection.url`。如果需要完全自定义 Socket.IO 事件或协议，创建一个内部持有 Socket.IO 客户端的适配器，并让它实现完整的 `TerminalTransport` 接口；不要把 Socket.IO 客户端实例直接传给 `connection.transport`。

配合内置 SSE AI transport 使用：

```ts
import {
  createSseAITransport,
  createSocketIoTerminalTransport,
  initTerminal,
} from "@tron/chat-terminal";

const terminal = initTerminal({
  el: terminalElement,
  connection: {
    transport: createSocketIoTerminalTransport(terminalServerUrl),
  },
  ai: {
    transport: createSseAITransport({
      url: "/api/llm/v1/chat/completions",
      headers: { Authorization: "Bearer <token>" },
      buildBody(content, stream) {
        return {
          messages: [{ content, role: "user" }],
          model: "your-model",
          stream,
        };
      },
    }),
  },
});
```

`createSseAITransport` 使用 `fetch` 发起 POST 请求，并解析服务端返回的 SSE 帧。服务端需要在流式响应中返回 OpenAI 兼容的 `data: {...}` 数据，并以 `data: [DONE]` 结束。

如果业务接口不是 SSE，也可以直接实现 `AITransport`。例如，普通 JSON 接口可以把完整响应作为一个 chunk：

```ts
import type { AITransport } from "@tron/chat-terminal";

const fetchAITransport: AITransport = {
  async stream(content, { onChunk, signal }) {
    const requestId = crypto.randomUUID();
    const response = await fetch("/api/llm", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      signal,
      body: JSON.stringify({ content }),
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
      signal,
      body: JSON.stringify({ content }),
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

initTerminal({
  el: terminalElement,
  connection: {
    transport: createSocketIoTerminalTransport(terminalServerUrl),
  },
  ai: { transport: fetchAITransport },
});
```

上面的 fetch AI 示例是非流式实现，`stream` 只回调一次。需要逐字输出时，应在 `response.body` 上读取流，或者使用 `createSseAITransport`。

### AI transport

```ts
interface AITransport {
  stream(
    content: string,
    options: {
      onChunk: (chunk: { delta: string; requestId: string }) => void;
      signal?: AbortSignal;
    },
  ): Promise<{ requestId: string }>;
  request(content: string, options?: { signal?: AbortSignal }): Promise<string>;
  dispose?(): void;
}
```

可直接使用 `createSseAITransport`：

```ts
const aiTransport = createSseAITransport({
  url: "/api/llm/v1/chat/completions",
  headers: { Authorization: "Bearer <token>" },
  buildBody(content, stream) {
    return {
      messages: [{ content, role: "user" }],
      model: "your-model",
      stream,
    };
  },
});
```

## Vim 和交互式命令

使用内置 Socket.IO transport 时，以下命令默认进入交互模式：

`vim`、`vi`、`nvim`、`nano`、`emacs`、`top`、`htop`、`less`、`more`、`man`、`watch`、`tmux`、`screen`。

交互模式会：

1. 在命令请求中携带 `interactive: true`。
2. 将 `i`、`Esc`、方向键、功能键和 `Ctrl+C` 等输入通过 `command:input` 原样写入 PTY。
3. 禁用交互命令的命令超时。
4. 通过 `terminal:resize` 同步 PTY 尺寸。

可以直接执行：

```text
vim workspace.json
```

点击 `i` 后编辑文件，使用 `Esc`、`:wq` 保存并退出。

需要支持其他交互式程序时，覆盖判断函数：

```ts
const terminal = initTerminal({
  el,
  connection: { url: terminalServerUrl },
  isInteractiveCommand(command) {
    return /^(?:vim|nvim|fzf)(?:\s|$)/.test(command.trim());
  },
});
```

使用自定义 `runCommand` 时，同时提供 `writeCommandInput` 和 `resizeTerminal`，或者实现完整的 `TerminalTransport`。

## 返回对象

`initTerminal` 返回：

| 方法 | 说明 |
| --- | --- |
| `dispose()` | 销毁终端、事件监听器和 transport。 |
| `setCurrentDirectory(path)` | 更新终端提示符目录。 |
| `appendChatChunk(response, ids?)` | 追加 AI 流式响应。 |
| `completeChat(requestId?)` | 完成当前 AI 会话。 |
| `setThemeTokens(tokens)` | 增量覆盖主题 token。 |
| `resetTheme()` | 清空主题覆盖。 |
| `refreshTheme()` | 重新同步 CSS 变量和 xterm 主题。 |
| `setLocale(locale, messages?)` | 切换语言并覆盖文案。 |
| `setMessages(messages)` | 增量覆盖文案。 |

### 主题 token

```ts
interface TerminalThemeTokenOverrides {
  primary?: string;
  secondary?: string;
  cardBackground?: string;
  border?: string;
  ansi?: Partial<ITheme>;
}
```

### 文案和语言

SDK 内置 `zh-CN` 和 `en-US`。可覆盖 `agentTips`、`explainTitle`、命令卡片按钮、状态和错误文案：

```ts
terminal.setLocale("en-US", {
  agentTips: "Enter a command or ask a question",
});
```

SDK 只翻译固定 UI 文案，不翻译命令、终端输出、模型回复或业务接口内容。

## Socket.IO 协议

### 会话

服务端发送 `terminal:session`：

```json
{
  "code": "0",
  "message": "",
  "data": {
    "currentDirectory": "/workspace"
  }
}
```

失败时 `code` 非 `"0"`，`message` 为可展示错误，`data` 为空对象。

### 命令执行

客户端发送 `command:execute`：

```json
{
  "command": "vim workspace.json",
  "messageId": "command-1",
  "interactive": true
}
```

普通命令可以省略 `interactive`。交互命令执行期间，客户端发送 `command:input`：

```json
{
  "messageId": "command-1",
  "data": "i"
}
```

取消命令发送 `command:cancel`：

```json
{
  "messageId": "command-1"
}
```

终端尺寸变化发送 `terminal:resize`：

```json
{
  "cols": 120,
  "rows": 40
}
```

### 命令消息

服务端通过 `command:message` 返回：

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

正常顺序为：

```text
TEXT_MESSAGE_START
TEXT_MESSAGE_CONTENT  // 0 到多次
TEXT_MESSAGE_END
```

命令错误使用 `TEXT_MESSAGE_ERROR`，错误文本放在 `delta`。AI SSE transport 复用同一消息格式；流式内容通过 `TEXT_MESSAGE_CONTENT` 传递，非流式建议使用 `TEXT_MESSAGE_SUGGESTION`。

## AI 命令块

AI 回复中的命令块会渲染为可编辑、可重新生成、可执行和可拒绝的命令卡片。推荐使用：

```xml
<execute_command>
<command>git status --short</command>
</execute_command>
```

也兼容：

````markdown
```execute_command
git status --short
```
````

每个命令块只放一条命令。用户点击执行后，SDK 才会调用命令执行 transport，不会自动执行模型返回的命令。
