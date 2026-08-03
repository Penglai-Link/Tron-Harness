# @tron/chat-terminal SDK 说明

`@tron/chat-terminal` 基于 xterm.js 提供 AI 终端界面。SDK 只负责终端渲染、输入分流、AI 消息展示、命令卡片和解释弹窗；命令执行和大模型调用由业务方通过回调接入。

公共 API 以 `src/index.ts` 为准。`src/terminal/` 导出的底层扩展类型主要服务内部实现，业务接入优先使用 `initTerminal`。

## 1. 安装

```bash
npm install @tron/chat-terminal
```

```ts
import { initTerminal } from "@tron/chat-terminal";
import "@tron/chat-terminal/styles.css";
```

挂载容器必须有明确宽高。SDK 会监听容器尺寸并自动调整终端行列数。

## 2. 最小接入

```ts
const terminal = initTerminal({
  el: document.querySelector("#terminal") as HTMLElement,

  termOptions: {
    themeTokens: {
      primary: "var(--color-on-surface, #111827)",
      secondary: "var(--color-surface, #ffffff)",
      cardBackground: "var(--color-elevated-surface, #ffffff)",
      border: "var(--color-card-border, #d1d5db)",
    },
  },

  runCommand(command, onOutput) {
    return terminalClient.execute(command, onOutput);
  },

  cancelCommand() {
    terminalClient.cancelCurrent();
  },

  async commandChat(content) {
    let requestId = "";

    await streamChat(content, (frame) => {
      const message = frame.data;
      requestId = message.messageId;
      terminal.appendChatChunk(message.delta, { requestId });
    });

    await terminal.completeChat(requestId || "");
  },

  commandExplain({ command, output }, { signal }) {
    return explainCommand({ command, output, signal });
  },

  commandGenerate(content) {
    return generateCommand(content);
  },
});
```

## 3. 输入分流

用户按回车后，SDK 会先判断输入是否为 Shell 命令：

- 内置命令表中的输入直接走 `runCommand`，如 `ls`、`pwd`、`cat`、`find`；
- 未命中内置命令表的普通文本走 `commandChat`，如 `git status` 当前会走 AI 对话；
- 以 `./`、`../`、`~/`、`/` 开头的输入直接走 `runCommand`；
- 形如 `FOO=bar` 的环境变量赋值直接走 `runCommand`；
- 如需让未命中的命令直接执行，请在后续版本扩展命令识别规则。

`commandChat` 必须自己处理流式请求、错误展示和结束收尾。业务方按第 6 章约定的事件和响应体接入后端；收到模型增量时调用 `appendChatChunk`，响应结束时调用 `completeChat`。

## 4. 初始化参数

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | :--: | --- |
| `el` | `HTMLElement` | 是 | 终端挂载元素 |
| `termOptions` | `ITerminalInitOnlyOptions & ITerminalOptions & { explainTitle?; agentTips?; agentTipsGapCells?; themeTokens? }` | 否 | xterm.js 配置和 SDK 扩展配置 |
| `runCommand` | `(command, onOutput) => Promise<{ currentDirectory?: string } \| void>` | 是 | 执行 Shell 命令；收到输出时调用 `onOutput(chunk)` |
| `cancelCommand` | `() => void` | 否 | 用户在命令执行中按 `Ctrl+C` 时触发 |
| `commandChat` | `(content) => Promise<void>` | 是 | 处理自然语言输入，并把模型增量写回终端 |
| `commandExplain` | `({ command, output }, { signal }) => Promise<string>` | 是 | 返回命令输出解释；应响应 `AbortSignal` |
| `commandGenerate` | `(content) => Promise<string>` | 是 | 根据命令卡片中的当前命令返回新的单行命令 |

`runCommand` 收到的 `command` 已经去除首尾空白。`onOutput` 支持多次调用，SDK 会按顺序写入终端。命令完成后可返回 `currentDirectory`，SDK 会用它更新提示符。

### 4.1 termOptions 扩展字段

| 参数 | 类型 | 默认值 | 说明 |
| --- | --- | --- | --- |
| `explainTitle` | `string` | `""` | 命令解释弹窗标题 |
| `agentTips` | `string` | 内置英文提示 | 空闲输入提示 |
| `agentTipsGapCells` | `number` | `1` | 提示与光标之间的单元格数 |
| `themeTokens` | `TerminalThemeTokenOverrides` | `{}` | 终端语义色和 xterm ANSI 主题覆盖 |

主题 token：

```ts
interface TerminalThemeTokenOverrides {
  primary?: string;
  secondary?: string;
  cardBackground?: string;
  border?: string;
  ansi?: Partial<ITheme>;
}
```

## 5. 返回对象

`initTerminal` 返回以下方法：

| 方法 | 输入 | 输出 | 说明 |
| --- | --- | --- | --- |
| `dispose` | 无 | `void` | 销毁终端、监听器和 DOM |
| `setCurrentDirectory` | `nextDirectory: string` | `void` | 更新提示符中的当前目录 |
| `appendChatChunk` | `response: string, ids?` | `void` | 追加模型文本增量 |
| `completeChat` | `requestId?: string` | `Promise<void>` | 固定模型响应布局 |
| `setThemeTokens` | `tokens: TerminalThemeTokenOverrides` | `void` | 增量覆盖主题 token |
| `resetTheme` | 无 | `void` | 清空主题 token 覆盖 |
| `refreshTheme` | 无 | `void` | 重新同步 CSS 变量到 xterm 主题 |

`appendChatChunk` 的第二个参数：

```ts
{
  requestId?: string;
  conversationId?: string;
}
```

同一段模型响应必须使用稳定的 `requestId`。并行会话需要同时传稳定的 `conversationId`。如果没有传 `ids`，SDK 使用 `default/default`；此时可以直接调用 `completeChat()`。如果传了 `requestId`，结束时优先调用 `completeChat(requestId)`；如只想完成当前活动会话，可调用 `completeChat("")`。

## 6. 后端入参和出参

后端协议只约定入参、出参、事件名和项目统一响应体，不在本文展开后端实现策略。当前 Tron 终端页使用以下约定：

- 命令执行使用 Socket.IO 事件；
- 大模型调用使用 `/api/llm/v1/chat/completions`；
- `/api/**` HTTP JSON 响应和 SSE 的每个 `data:` 帧都使用项目统一响应体；
- Socket.IO 事件不是 HTTP 响应体，事件载荷按本章约定传递。

项目统一响应体：

```ts
interface ApiResponse<T> {
  code: string;
  message: string;
  data: T;
}
```

### 6.1 命令执行

事件约定：

| 方向 | 事件 | 载荷 |
| --- | --- | --- |
| 客户端 -> 服务端 | `command:execute` | `CommandExecuteParam` |
| 客户端 -> 服务端 | `command:cancel` | `CommandCancelParam` |
| 服务端 -> 客户端 | `terminal:session` | `ApiResponse<TerminalSessionVO>` |
| 服务端 -> 客户端 | `command:message` | `TerminalMessage` |

会话状态出参：

```ts
interface TerminalSessionVO {
  currentDirectory?: string;
}
```

成功示例：

```json
{
  "code": "0",
  "message": "",
  "data": {
    "currentDirectory": "/workspace/tron"
  }
}
```

失败示例：

```json
{
  "code": "-1",
  "message": "终端会话无法启动",
  "data": {}
}
```

执行命令入参：

```ts
interface CommandExecuteParam {
  messageId: string;
  command: string;
}
```

取消命令入参：

```ts
interface CommandCancelParam {
  messageId: string;
}
```

命令消息出参。服务端通过 `command:message` 返回，载荷直接使用 `TerminalMessage`：

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

字段说明：

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | :--: | --- |
| `type` | `TerminalMessageType` | 是 | 消息类型 |
| `messageId` | `string` | 是 | 与入参一致；同一命令流保持不变 |
| `delta` | `string` | 是 | 增量输出、完整建议或错误信息；无内容时传空字符串 |
| `timestamp` | `number` | 是 | 毫秒时间戳 |
| `currentDirectory` | `string` | 否 | 命令结束后的目录，仅结束消息需要 |

正常命令输出顺序：

```text
TEXT_MESSAGE_START
TEXT_MESSAGE_CONTENT  // 0 到多次
TEXT_MESSAGE_END
```

失败时返回 `TEXT_MESSAGE_ERROR`，`delta` 放可展示错误信息。

### 6.2 大模型对话

接口约定：

| 使用场景 | 请求方法 | 请求 URL | 响应类型 |
| --- | --- | --- | --- |
| `commandChat` | `POST` | `/api/llm/v1/chat/completions` | `text/event-stream` |
| `commandExplain` | `POST` | `/api/llm/v1/chat/completions` | `application/json` |
| `commandGenerate` | `POST` | `/api/llm/v1/chat/completions` | `application/json` |

流式对话入参 `ChatCompletionsStreamParam`：

```json
{
  "messageId": "chat-1",
  "messages": [
    {
      "role": "user",
      "content": "列出当前目录下的文件"
    }
  ],
  "stream": true
}
```

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | :--: | --- |
| `messageId` | `string` | 是 | 客户端生成的请求唯一标识；同一响应流保持不变 |
| `messages` | `{ role: string; content: string }[]` | 是 | 发送给大模型的消息 |
| `stream` | `true` | 是 | 固定传 `true` |

流式对话出参按 SSE 返回，每个 `data:` 帧是 `ApiResponse<ChatCompletionsStreamVO>`：

```ts
interface ChatCompletionsStreamVO {
  type: "TEXT_MESSAGE_START" | "TEXT_MESSAGE_CONTENT" | "TEXT_MESSAGE_END" | "TEXT_MESSAGE_ERROR";
  messageId: string;
  delta: string;
  timestamp: number;
}
```

```text
data: {"code":"0","message":"","data":{"type":"TEXT_MESSAGE_START","messageId":"chat-1","delta":"","timestamp":1785398400000}}

data: {"code":"0","message":"","data":{"type":"TEXT_MESSAGE_CONTENT","messageId":"chat-1","delta":"可以使用 ","timestamp":1785398400050}}

data: {"code":"0","message":"","data":{"type":"TEXT_MESSAGE_CONTENT","messageId":"chat-1","delta":"ls 命令。","timestamp":1785398400100}}

data: {"code":"0","message":"","data":{"type":"TEXT_MESSAGE_END","messageId":"chat-1","delta":"","timestamp":1785398400150}}
```

前端接入层读取 `data.messageId` 和 `data.delta`：

```ts
terminal.appendChatChunk(frame.data.delta, {
  requestId: frame.data.messageId,
});
```

### 6.3 解释和重新生成

非流式对话入参 `ChatCompletionsParam`：

```json
{
  "messageId": "generate-1",
  "messages": [
    {
      "role": "user",
      "content": "根据当前命令生成一条新的单行 Shell 命令：find"
    }
  ],
  "stream": false
}
```

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | :--: | --- |
| `messageId` | `string` | 是 | 客户端生成的请求唯一标识 |
| `messages` | `{ role: string; content: string }[]` | 是 | 发送给大模型的消息 |
| `stream` | `false` | 是 | 固定传 `false` |

非流式对话出参为 `ApiResponse<ChatCompletionsVO>`：

```ts
interface ChatCompletionsVO {
  type: "TEXT_MESSAGE_SUGGESTION" | "TEXT_MESSAGE_ERROR";
  messageId: string;
  delta: string;
  timestamp: number;
}
```

```json
{
  "code": "0",
  "message": "",
  "data": {
    "type": "TEXT_MESSAGE_SUGGESTION",
    "messageId": "generate-1",
    "delta": "find . -maxdepth 2 -type f",
    "timestamp": 1785398400000
  }
}
```

失败响应：

```json
{
  "code": "-1",
  "message": "系统异常",
  "data": {}
}
```

`commandExplain` 使用 `data.delta` 作为完整解释文本。`commandGenerate` 使用 `data.delta` 作为新命令，后端必须只返回一条单行 Shell 命令，不能返回 Markdown 代码围栏、XML 标签或解释文字。

## 7. AI 命令块

AI 回复里的可执行命令需要使用命令块。SDK 会把命令块渲染为可编辑、可重新生成、可执行、可拒绝的命令卡片。

推荐格式：

```xml
<execute_command>
<command>git status --short</command>
</execute_command>
```

兼容格式：

````markdown
```execute_command
git status --short
```
````

每个命令块只放一条命令。普通说明使用 Markdown。用户点击执行后，SDK 会调用 `runCommand`，不会自动执行模型返回的命令。

## 8. 导出类型

包入口导出以下常用类型：

```ts
export type {
  CommandExplain,
  CommandExplainContext,
  CommandExplainPayload,
  TerminalThemeTokenOverrides,
} from "@tron/chat-terminal";
```
