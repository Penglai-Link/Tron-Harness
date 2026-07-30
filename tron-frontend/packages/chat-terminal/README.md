# @tron/chat-terminal 对接文档

`@tron/chat-terminal` 基于 xterm.js 提供 AI 终端界面。组件本身不固定后端实现，业务方通过回调接入以下能力：

- 执行 Shell 命令并增量输出结果；
- 将自然语言发送给大模型；
- 解释命令及其输出；
- 根据现有命令重新生成命令。

本文档以 `src/index.ts` 导出的公共 API 为准。`src/test/` 仅用于测试，其中的客户端、提示词和辅助函数不属于包的公共 API。

| 目录            | 定位                                                   |
| --------------- | ------------------------------------------------------ |
| `src/index.ts`  | 包入口；定义业务方可以使用的初始化参数、回调和返回方法 |
| `src/terminal/` | 终端内部实现及底层扩展类型                             |
| `src/test/`     | 测试文件；验证 Socket.IO、HTTP 和 SSE 接入流程         |

## 1. 对接范围

| 能力         | 前端回调          | 传输方式        | 后端职责                             |
| ------------ | ----------------- | --------------- | ------------------------------------ |
| 执行命令     | `runCommand`      | Socket.IO       | 启动终端进程并流式返回 stdout/stderr |
| AI 对话      | `commandChat`     | SSE 对话接口    | 流式返回模型文本                     |
| 解释命令输出 | `commandExplain`  | 非 SSE 对话接口 | 返回完整解释文本                     |
| 重新生成命令 | `commandGenerate` | 非 SSE 对话接口 | 返回一条可执行命令                   |

SSE 和非 SSE 请求使用两个独立接口。模型由后端配置，前端不能指定或覆盖模型。命令执行必须由受控的终端服务完成，不要将 Shell 权限暴露给浏览器。

## 2. 安装与初始化

```bash
npm install @tron/chat-terminal
```

```ts
import { initTerminal } from "@tron/chat-terminal";
import "@tron/chat-terminal/styles.css";

const terminal = initTerminal({
  el: document.querySelector("#terminal") as HTMLElement,

  async runCommand(command, onOutput) {
    // 对接命令执行服务；每收到一段输出就调用一次 onOutput。
    await terminalClient.execute(command, onOutput);
  },

  async commandChat(content) {
    // 对接 SSE。每收到一段模型增量就追加到终端。
    const requestId = crypto.randomUUID();
    try {
      await streamChat(content, (delta) => {
        terminal.appendChatChunk(delta, { requestId });
      });
    } finally {
      await terminal.completeChat(requestId);
    }
  },

  commandExplain({ command, output }, { signal }) {
    return explainCommand({ command, output, signal });
  },

  commandGenerate(content) {
    return generateCommand(content);
  },
});
```

挂载元素必须有明确的宽度和高度。组件会监听容器尺寸并自动调整终端行列数。

### 2.1 初始化参数

| 参数              | 类型                                                                                              | 必填 | 说明                                                    |
| ----------------- | ------------------------------------------------------------------------------------------------- | :--: | ------------------------------------------------------- |
| `el`              | `HTMLElement`                                                                                     |  是  | 终端挂载元素                                            |
| `termOptions`     | `ITerminalInitOnlyOptions & ITerminalOptions & { explainTitle?; agentTips?; agentTipsGapCells? }` |  否  | xterm.js 配置和组件文案；扩展字段由入口文件内联定义     |
| `runCommand`      | `(command, onOutput) => Promise<void>`                                                            |  是  | 执行 Shell 命令；通过 `onOutput` 写入增量输出           |
| `commandChat`     | `(content) => Promise<void>`                                                                      |  是  | 处理自然语言输入，并调用 `appendChatChunk` 写入模型增量 |
| `commandExplain`  | `({ command, output }, { signal }) => Promise<string>`                                            |  是  | 返回命令输出的完整解释；必须响应取消信号                |
| `commandGenerate` | `(content) => Promise<string>`                                                                    |  是  | 根据当前命令返回新的单行命令                            |

`termOptions` 的组件扩展字段：

| 参数                | 类型     | 默认值       | 说明                     |
| ------------------- | -------- | ------------ | ------------------------ |
| `explainTitle`      | `string` | `""`         | 命令解释弹窗标题         |
| `agentTips`         | `string` | 内置英文提示 | 终端空闲时的输入提示     |
| `agentTipsGapCells` | `number` | `1`          | 提示与光标之间的单元格数 |

### 2.2 返回对象

| 方法                  | 输入                       | 输出            | 说明                                                             |
| --------------------- | -------------------------- | --------------- | ---------------------------------------------------------------- |
| `setCurrentDirectory` | `nextDirectory: string`    | `void`          | 更新提示符中的当前目录，仅改变前端显示                           |
| `appendChatChunk`     | `response: string`, `ids?` | `void`          | 追加一段模型响应；`ids` 包含可选的 `requestId`、`conversationId` |
| `completeChat`        | `requestId?: string`       | `Promise<void>` | 完成指定模型响应并固定布局                                       |

同一段流式响应必须始终使用相同的 `requestId`。并行会话还必须传入稳定的 `conversationId`。

## 3. 通用消息结构

Socket.IO 和大模型接口复用 `TerminalMessage`：

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
}

interface ApiResponse<T> {
  code: string;
  message: string;
  data: T;
}
```

`TerminalMessage` 是前端接入层使用的统一消息结构，不是后端跨接口复用的 VO。后端必须分别定义 `ChatCompletionsStreamVO` 和 `ChatCompletionsVO`，前端再将两种响应映射为 `TerminalMessage`。HTTP JSON 响应和 SSE 的每个 `data:` 帧必须使用 `ApiResponse`。Socket.IO 事件不是 HTTP 响应体，事件载荷直接使用 `TerminalMessage`。

| 字段        | 类型                  | 必填 | 说明                                                                     |
| ----------- | --------------------- | :--: | ------------------------------------------------------------------------ |
| `type`      | `TerminalMessageType` |  是  | 消息类型，必须传枚举 CODE，不能传枚举名称映射值（§2.1.2）                |
| `messageId` | `string`              |  是  | 一次请求或命令的唯一标识；同一消息流保持不变                             |
| `delta`     | `string`              |  是  | 增量文本、完整建议或错误信息；无内容时传空字符串，不能省略字段（§2.1.4） |
| `timestamp` | `number`              |  是  | 服务器生成的毫秒时间戳（§2.1.1）                                         |

消息类型语义：

| `type`                    | `delta` 内容             | 终止消息 |
| ------------------------- | ------------------------ | :------: |
| `TEXT_MESSAGE_START`      | 空字符串                 |    否    |
| `TEXT_MESSAGE_CONTENT`    | 一段增量文本或终端输出   |    否    |
| `TEXT_MESSAGE_END`        | 空字符串                 |    是    |
| `TEXT_MESSAGE_ERROR`      | 可展示的错误信息         |    是    |
| `TEXT_MESSAGE_SUGGESTION` | 完整的解释文本或建议命令 |    是    |

## 4. 命令执行接口

### 4.1 建立连接

- 协议：Socket.IO，传输层仅使用 WebSocket；
- namespace：`/`；
- 客户端连接地址：终端服务 HTTP 基础地址；Socket.IO 自动升级到 WebSocket；
- 生产环境必须完成身份认证、来源校验和命令权限校验；
- 每个客户端断开连接后，后端必须终止该客户端仍在运行的进程。

### 4.2 执行命令

客户端发送事件：`command:execute`

```json
{
  "messageId": "4db80731-faf1-44c0-b047-dcf15f24ca7d",
  "command": "pwd"
}
```

| 参数        | 类型     | 必填 | 约束                                                       |
| ----------- | -------- | :--: | ---------------------------------------------------------- |
| `messageId` | `string` |  是  | 客户端生成；同一连接内不能与正在执行的命令重复             |
| `command`   | `string` |  是  | 去除首尾空白后不能为空；长度上限和允许范围需由业务后端确定 |

服务端通过 `command:message` 事件返回消息。正常顺序如下：

```text
TEXT_MESSAGE_START
  -> TEXT_MESSAGE_CONTENT（0 到多次）
  -> TEXT_MESSAGE_END
```

示例：

```json
{
  "type": "TEXT_MESSAGE_CONTENT",
  "messageId": "4db80731-faf1-44c0-b047-dcf15f24ca7d",
  "delta": "/workspace/tron\r\n",
  "timestamp": 1785398400000
}
```

执行失败时返回 `TEXT_MESSAGE_ERROR`。当前前端收到该消息后会拒绝 `runCommand` 的 Promise，并将 `delta` 作为错误信息显示。

### 4.3 取消命令

客户端发送事件：`command:cancel`

```json
{
  "messageId": "4db80731-faf1-44c0-b047-dcf15f24ca7d"
}
```

| 参数        | 类型     | 必填 | 约束                             |
| ----------- | -------- | :--: | -------------------------------- |
| `messageId` | `string` |  是  | 必须对应当前连接中正在执行的命令 |

进程退出后，服务端发送 `TEXT_MESSAGE_END`。找不到命令时发送 `TEXT_MESSAGE_ERROR`。前端接入层必须设置超时，超时后发送取消事件并结束本地等待。

## 5. 大模型接口

### 5.1 接口总览

| 接口        | 请求方法 | 请求 URL                              | 响应类型            | 使用场景                            |
| ----------- | -------- | ------------------------------------- | ------------------- | ----------------------------------- |
| SSE 对话    | `POST`   | `/api/llm/v1/chat-completions-stream` | `text/event-stream` | `commandChat`                       |
| 非 SSE 对话 | `POST`   | `/api/llm/v1/chat-completions`        | `application/json`  | `commandExplain`、`commandGenerate` |

两个接口使用独立 Param 和 VO 类，禁止跨接口复用（§2.4、§2.5）。URL 使用 kebab-case 且不使用路径参数（§2.2、§2.3）。请求体上限由后端、网关和模型服务共同确定。

### 5.2 SSE 对话接口

基本信息：

| 项目     | 值                                    |
| -------- | ------------------------------------- |
| 请求方法 | `POST`                                |
| 请求 URL | `/api/llm/v1/chat-completions-stream` |
| 响应类型 | `text/event-stream; charset=utf-8`    |

请求体 `ChatCompletionsStreamParam`：

```json
{
  "messageId": "chat-cd91d411-b8e7-4fb3-930c-f8967ef3f23e",
  "message": "列出当前目录下的文件"
}
```

| 参数        | 类型     | 必填 | 说明                                           |
| ----------- | -------- | :--: | ---------------------------------------------- |
| `message`   | `string` |  是  | 发送给大模型的完整消息；去除首尾空白后不能为空 |
| `messageId` | `string` |  是  | 客户端生成的请求唯一标识；同一响应流保持不变   |

响应按 SSE 帧返回 `ApiResponse<ChatCompletionsStreamVO>`。`ChatCompletionsStreamVO` 包含 `type`、`messageId`、`delta`、`timestamp`：

```text
data: {"code":"0","message":"","data":{"type":"TEXT_MESSAGE_START","messageId":"chat-cd91d411-b8e7-4fb3-930c-f8967ef3f23e","delta":"","timestamp":1785398400000}}

data: {"code":"0","message":"","data":{"type":"TEXT_MESSAGE_CONTENT","messageId":"chat-cd91d411-b8e7-4fb3-930c-f8967ef3f23e","delta":"可以使用 ","timestamp":1785398400050}}

data: {"code":"0","message":"","data":{"type":"TEXT_MESSAGE_CONTENT","messageId":"chat-cd91d411-b8e7-4fb3-930c-f8967ef3f23e","delta":"ls 命令。","timestamp":1785398400100}}

data: {"code":"0","message":"","data":{"type":"TEXT_MESSAGE_END","messageId":"chat-cd91d411-b8e7-4fb3-930c-f8967ef3f23e","delta":"","timestamp":1785398400150}}

```

正常顺序为 `START -> CONTENT（0 到多次）-> END`。前端接入层提取每帧的 `data.delta`，再调用 `appendChatChunk`。收到 `END` 后调用 `completeChat(messageId)`。

发生错误时返回 `TEXT_MESSAGE_ERROR` 帧并关闭响应。该帧的 `code` 必须为业务错误码或 `-1`，`message` 和 `data.delta` 必须包含可展示的错误信息。浏览器中止请求或断开连接后，后端必须同时中止模型请求。

### 5.3 非 SSE 对话接口

基本信息：

| 项目     | 值                             |
| -------- | ------------------------------ |
| 请求方法 | `POST`                         |
| 请求 URL | `/api/llm/v1/chat-completions` |
| 响应类型 | `application/json`             |

请求体 `ChatCompletionsParam`：

```json
{
  "messageId": "generate-91eb29e9-3c9c-4dc0-a57e-6ff41ae8970a",
  "message": "根据当前命令生成一条新的单行 Shell 命令：find"
}
```

| 参数        | 类型     | 必填 | 说明                                           |
| ----------- | -------- | :--: | ---------------------------------------------- |
| `message`   | `string` |  是  | 发送给大模型的完整消息；去除首尾空白后不能为空 |
| `messageId` | `string` |  是  | 客户端生成的请求唯一标识                       |

成功响应：

```json
{
  "code": "0",
  "message": "",
  "data": {
    "type": "TEXT_MESSAGE_SUGGESTION",
    "messageId": "generate-91eb29e9-3c9c-4dc0-a57e-6ff41ae8970a",
    "delta": "find . -maxdepth 2 -type f",
    "timestamp": 1785398400000
  }
}
```

响应类型为 `ApiResponse<ChatCompletionsVO>`。`ChatCompletionsVO` 包含 `type`、`messageId`、`delta`、`timestamp`，其中 `messageId` 必须与请求一致。

`commandExplain` 调用该接口时，前端接入层将 `command` 和 `output` 组装到 `message`，`data.delta` 返回完整解释文本。`commandGenerate` 调用该接口时，前端接入层将当前命令组装到 `message`，`data.delta` 必须只包含一条单行 Shell 命令，不能包含 Markdown 代码围栏、XML 标签或解释文字。

### 5.4 错误处理

只有业务处理成功时才返回 HTTP 200（§2.5）。流建立前发生错误时，服务端必须返回对应的非 200 HTTP 状态码和统一错误响应体。流建立后发生错误时，服务端通过 `TEXT_MESSAGE_ERROR` 帧结束响应。

## 6. 项目统一响应体

`/api/**` HTTP 接口必须使用 `code`、`message`、`data` 三个字段（§2.5），不能增加同级字段：

```ts
interface ApiResponse<T> {
  code: string;
  message: string;
  data: T;
}
```

非流式成功响应：

```json
{
  "code": "0",
  "message": "",
  "data": {
    "type": "TEXT_MESSAGE_SUGGESTION",
    "messageId": "generate-91eb29e9-3c9c-4dc0-a57e-6ff41ae8970a",
    "delta": "find . -maxdepth 2 -type f",
    "timestamp": 1785398400000
  }
}
```

非流式失败响应：

```json
{
  "code": "-1",
  "message": "系统异常",
  "data": {}
}
```

`0` 表示成功，`-1` 仅表示系统异常或未知异常。业务错误必须使用项目分配的 9 位错误码。错误码尚未在 `chat-terminal` 中定义，需后端服务负责人补充。

SSE 每个 `data:` 帧的 JSON 内容必须是 `ApiResponse<ChatCompletionsStreamVO>`。前端接入层先提取并映射 `data`，再调用入口返回的 `appendChatChunk`。Socket.IO 事件载荷继续使用 `TerminalMessage`。

## 7. AI 命令块协议

AI 对话中的可执行命令必须使用以下格式：

```xml
<execute_command>
<command>git status --short</command>
</execute_command>
```

组件也兼容以下围栏格式：

````markdown
```execute_command
git status --short
```
````

每个命令块只能包含一条命令。普通说明使用 Markdown。后端或提示词必须阻止模型生成明显具有破坏性、不可逆或越权风险的命令；用户仍需主动点击执行。
