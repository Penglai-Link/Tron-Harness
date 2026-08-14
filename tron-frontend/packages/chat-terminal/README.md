# @tron/chat-terminal SDK 说明

`@tron/chat-terminal` 基于 xterm.js 提供 AI 终端界面。SDK 只负责终端渲染、输入分流、AI 消息展示、命令卡片和解释弹窗；命令执行和大模型调用由业务方通过回调接入。

## 1. 安装

```bash
npm install @tron/chat-terminal
```

```ts
import { initTerminal } from "@tron/chat-terminal";
import "@tron/chat-terminal/styles.css";
```

挂载容器必须有明确宽高。SDK 会监听容器尺寸并自动调整终端行列数。

## 2. 初始化

```ts
// 初始化socket链接
import { io } from "socket.io-client";

const socket = io(url, {
  autoConnect: false,
  reconnection: true,
  timeout: 10_000,
  transports: ["websocket"],
});

let socketOutput = null;
let messageId = +new Date()

// 接受消息
socket.on("command:message", (response) =>{
  socketOutput(response),
}

// 初始化终端
const terminal = initTerminal({
  el: document.querySelector("#terminal") as HTMLElement,
  locale: "zh-CN", // 终端语言
  messages:{
    explainTitle: "命令解释",
  }, // 多语言文案替换,可不传
  termOptions: {
    // 主题色
    themeTokens: {
      primary: "var(--color-on-surface, #111827)",
      secondary: "var(--color-surface, #ffffff)",
      cardBackground: "var(--color-elevated-surface, #ffffff)",
      border: "var(--color-card-border, #d1d5db)",
    },
  },

  runCommand(command, onOutput) {
    // 调用socket，command是当前输入的命令，onOutput是将socket返回的内容写入到终点中
    socket.emit("command:execute", { command, messageId });
    messageId = +new Date();
    socketOutput = onOutput;
  },

  cancelCommand() {
    // 当按下ctrl+c的时候，调用socket终止运行的命令
    socket.emit("command:cancel", { messageId });
  },

  // 输入内容后进行大模型对话
  async commandChat(content) {
    let requestId = "";
    const result = await fetch("see接口地址");
    // 这里处理循环逻辑
    for(...){
        const message = frame.data;
        requestId = message.messageId;
        // 将数据添加到chat中
        terminal.appendChatChunk(message.delta, { requestId });
    }

    // 执行完成后关更新chat状态
    await terminal.completeChat(requestId || "");
  },

  // 解释代码
  commandExplain({ command, output }, { signal }) {
    // 返回大模型内容的字符串
    return fetch("解释代码的接口");
  },
  // 代码生成
  commandGenerate(content) {
    // 返回大模型内容的字符串
    return fetch("代码生成的接口");
  },
});
```
## 3. 参数说明

| 参数              | 类型                                                                                                            | 必填 | 说明                                              |
| ----------------- | --------------------------------------------------------------------------------------------------------------- | :--: | ------------------------------------------------- |
| `el`              | `HTMLElement`                                                                                                   |  是  | 终端挂载元素                                      |
| `locale`          | `"zh-CN" \| "en-US"`                                                                                            |  否  | SDK 内置固定文案语言，默认 `zh-CN`                |
| `messages`        | `Partial<ChatTerminalMessages>`                                                                                 |  否  | 覆盖 SDK 内置固定文案                             |
| `termOptions`     | `ITerminalInitOnlyOptions & ITerminalOptions & { explainTitle?; agentTips?; agentTipsGapCells?; themeTokens? }` |  否  | xterm.js 配置和 SDK 扩展配置                      |
| `runCommand`      | `(command, onOutput) => Promise<{ currentDirectory?: string } \| void>`                                         |  是  | 执行 Shell 命令；收到输出时调用 `onOutput(chunk)` |
| `cancelCommand`   | `() => void`                                                                                                    |  否  | 用户在命令执行中按 `Ctrl+C` 时触发                |
| `commandChat`     | `(content) => Promise<void>`                                                                                    |  是  | 处理自然语言输入，并把模型增量写回终端            |
| `commandExplain`  | `({ command, output }, { signal }) => Promise<string>`                                                          |  是  | 返回命令输出解释；应响应 `AbortSignal`            |
| `commandGenerate` | `(content) => Promise<string>`                                                                                  |  是  | 根据命令卡片中的当前命令返回新的单行命令          |

`runCommand` 收到的 `command` 已经去除首尾空白。`onOutput` 支持多次调用，SDK 会按顺序写入终端。命令完成后可返回 `currentDirectory`，SDK 会用它更新提示符。

### 3.1 termOptions 扩展字段

| 参数                | 类型                          | 默认值       | 说明                                                                 |
| ------------------- | ----------------------------- | ------------ | -------------------------------------------------------------------- |
| `agentTipsGapCells` | `number`                      | `1`          | 提示与光标之间的单元格数                                             |
| `themeTokens`       | `TerminalThemeTokenOverrides` | `{}`         | 终端语义色和 xterm ANSI 主题覆盖                                     |

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

### 3.2 多语言

SDK 内置 `zh-CN` 和 `en-US` 两套固定文案。初始化时可指定语言，也可覆盖部分文案：

```ts
const terminal = initTerminal({
  el,
  locale: "zh-CN",
  messages: {
    agentTips: "输入命令或者问题",
    explainTitle: "命令解释",
  },
  // 其他必填回调略
});
```

运行时调用 `setLocale` 切换语言：

```ts
terminal.setLocale("en-US");

terminal.setLocale("zh-CN", {
  explainLoading: "正在分析命令...",
});
```

语言切换只刷新 SDK 固定 UI 文案，例如空闲提示、解释弹窗标题和 loading 文案、命令卡片按钮和状态、SDK 兜底错误文案。SDK 不会翻译终端历史、命令输出、模型回复和业务接口返回内容。

可覆盖的文案 key：

```ts
type ChatTerminalMessageKey =
  | "agentTips"
  | "explainTitle"
  | "explainLoading"
  | "explainEmpty"
  | "explainFailed"
  | "explainInterrupted"
  | "commandConfirm"
  | "commandExecute"
  | "commandEdit"
  | "commandSave"
  | "commandReject"
  | "commandGenerate"
  | "commandGenerating"
  | "statusExecuted"
  | "statusRejected"
  | "modifyFailed"
  | "emptyGenerateCommand"
  | "modelReturnedEmptyCommand"
  | "generateFailed"
  | "executeFailed"
  | "rejectFailed";
```

### 3.3 主题色
当项目中主题色发生变化后，需调用refreshTheme刷新终端
```ts
terminal.refreshTheme()
```

## 4. 返回对象

`initTerminal` 返回以下方法：

| 方法                  | 输入                                                                  | 输出            | 说明                                    |
| --------------------- | --------------------------------------------------------------------- | --------------- | --------------------------------------- |
| `dispose`             | 无                                                                    | `void`          | 销毁终端、监听器和 DOM                  |
| `setCurrentDirectory` | `nextDirectory: string`                                               | `void`          | 更新提示符中的当前目录                  |
| `appendChatChunk`     | `response: string, ids?`                                              | `void`          | 追加模型文本增量                        |
| `completeChat`        | `requestId?: string`                                                  | `Promise<void>` | 固定模型响应布局                        |
| `setThemeTokens`      | `tokens: TerminalThemeTokenOverrides`                                 | `void`          | 增量覆盖主题 token                      |
| `resetTheme`          | 无                                                                    | `void`          | 清空主题 token 覆盖                     |
| `refreshTheme`        | 无                                                                    | `void`          | 重新同步 CSS 变量到 xterm 主题          |
| `setLocale`           | `locale: ChatTerminalLocale, messages?: ChatTerminalMessageOverrides` | `void`          | 切换 SDK 固定文案语言，并可增量覆盖文案 |
| `setMessages`         | `messages: ChatTerminalMessageOverrides`                              | `void`          | 增量覆盖 SDK 固定文案                   |

`appendChatChunk` 的第二个参数：

```ts
{
  requestId?: string;
  conversationId?: string;
}
```

同一段模型响应必须使用稳定的 `requestId`。并行会话需要同时传稳定的 `conversationId`。如果没有传 `ids`，SDK 使用 `default/default`；此时可以直接调用 `completeChat()`。如果传了 `requestId`，结束时优先调用 `completeChat(requestId)`；如只想完成当前活动会话，可调用 `completeChat("")`。

## 5. 后端入参和出参

响应参数说明
```ts
type TerminalMessageType =
  | "TEXT_MESSAGE_START" // 会话开始
  | "TEXT_MESSAGE_CONTENT" // 生成内容
  | "TEXT_MESSAGE_END" // 会话结束
  | "TEXT_MESSAGE_ERROR" // 会话错误
  | "TEXT_MESSAGE_SUGGESTION"; // 会话建议

interface TerminalMessage {
  type: TerminalMessageType;
  messageId: string; // 当前会话的唯一ID
  delta: string; // 生成的内容
  timestamp: number; // 时间戳
  currentDirectory?: string; // 当前目录，Scoket才会存在
}
```

### 5.1 Scoket

- `terminal:session`客户端和服务端链接建立成功，服务端返回的事件
  ```json
  // 成功时返回数据
  {
    currentDirectory: "当前执行的目录地址";
  }
  ```
- `command:message`服务端执行命令时返回的事件
  ```json
  // 响应开始标识
  {
    type: "TEXT_MESSAGE_START";
    messageId: "123";
    delta: "";
    timestamp: "1786679164005";
  }
  // 响应内容，这里可能会有多个
  {
    type: "TEXT_MESSAGE_CONTENT";
    messageId: "123";
    delta: "这个是执行输出的内容";
    timestamp: "1786679164005";
  }
  // 响应结束标识
  {
    type: "TEXT_MESSAGE_END";
    messageId: "123";
    delta: "";
    timestamp: "1786679164005";
    currentDirectory: "当前路径" // 返回当前执行的路径，用于终端更新
  }
  ```

- `command:execute`客户端发送指令给服务端
  ```json
  {
    command: "ls",
    messageId: "123", // 每次执行的时候生成的唯一id,后端需要缓存，用于终止当前执行的命令
  }
  ```
- `command:cancel`客户端发送终止命令给服务端
  ```json
  {
    messageId: "123", // 当前执行命令的唯一id
  }
  ```

字段说明：

| 字段               | 类型                  | 必填 | 说明                                             |
| ------------------ | --------------------- | :--: | ------------------------------------------------ |
| `type`             | `TerminalMessageType` |  是  | 消息类型                                         |
| `messageId`        | `string`              |  是  | 与入参一致；同一命令流保持不变                   |
| `delta`            | `string`              |  是  | 增量输出、完整建议或错误信息；无内容时传空字符串 |
| `timestamp`        | `number`              |  是  | 毫秒时间戳                                       |
| `currentDirectory` | `string`              |  否  | 命令结束后的目录，仅结束消息需要                 |

正常命令输出顺序：

```text
TEXT_MESSAGE_START
TEXT_MESSAGE_CONTENT  // 0 到多次
TEXT_MESSAGE_END
```

失败时返回 `TEXT_MESSAGE_ERROR`，`delta` 放可展示错误信息。

### 5.2 流式对话
大模型对话分为流式和非流式接口，这里只规定出参，入参可根据实际情况进行调整

流式对话出参按 SSE 返回，每个 `data:` 帧是：

```ts
{
  type:
    | "TEXT_MESSAGE_START"
    | "TEXT_MESSAGE_CONTENT"
    | "TEXT_MESSAGE_END"
    | "TEXT_MESSAGE_ERROR";
  messageId: string;
  delta: string;
  timestamp: number;
}
```

```text
data: {"type":"TEXT_MESSAGE_START","messageId":"chat-1","delta":"","timestamp":1785398400000}

data: {"type":"TEXT_MESSAGE_CONTENT","messageId":"chat-1","delta":"可以使用 ","timestamp":1785398400050}

data: {"type":"TEXT_MESSAGE_CONTENT","messageId":"chat-1","delta":"ls 命令。","timestamp":1785398400100}

data: {"type":"TEXT_MESSAGE_END","messageId":"chat-1","delta":"","timestamp":1785398400150}
```

前端接入层读取 `messageId` 和 `delta`：

```ts
// 将流式数据回显到终端
terminal.appendChatChunk(frame.data.delta, {
  requestId: frame.data.messageId,
});
```
### 5.3 非流式对话
```ts
{
  type:
    | "TEXT_MESSAGE_START"
    | "TEXT_MESSAGE_CONTENT"
    | "TEXT_MESSAGE_END"
    | "TEXT_MESSAGE_ERROR";
  messageId: string;
  delta: string;
  timestamp: number;
}
```
`commandExplain` 使用 `delta` 作为完整解释文本。
`commandGenerate` 使用 `delta` 作为新命令，后端必须只返回一条单行 Shell 命令，不能返回 Markdown 代码围栏、XML 标签或解释文字。

## 6. AI 命令块

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