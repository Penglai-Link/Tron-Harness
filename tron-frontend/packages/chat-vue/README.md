# TronChatRuntimeWindow 组件文档

`TronChatRuntimeWindow` 是一个用于渲染聊天运行时窗口的 Vue 组件。它支持悬浮窗或内嵌面板模式，并提供完整的会话状态管理、消息收发和 Agui 事件处理等功能。

运行中输入遵循 Codex 风格的两种意图：`steer` 将消息注入当前 `runCode`，`queue` 将消息加入当前会话的本地 FIFO 队列。运行中唯一动作按钮仍为原 Stop，不渲染 Queue 或 Steer 按钮：Enter 默认 queue，`Command/Ctrl+Enter` 才触发 steer；输入框会同步提示这组快捷键。Runtime 会通过 `getActiveRun` 恢复刷新前已经存在的 run：`RUNNING` 且 runtime 支持 steer 时允许 queue/steer，`FINISHING` 时只允许 queue。临时查询失败会保留最后一次成功状态并继续轮询；查询成功返回 null 后，支持 protocol replay 的 adapter 还会等待对应 `RUN_FINISHED` 或 `RUN_ERROR`。排队消息显示在输入框上方，可编辑或撤回；确认自然终态后只取出一条 queued message。显式 Stop 将当前已提交项视为已消费，并暂停剩余 FIFO，不会盲目重发已持久化的 USER 消息。

Stop 是独立的后端控制操作。Runtime 先调用 `stopRun`，收到 ACK 后才断开本地 SSE observer。切换会话、新建会话或卸载组件只会断开 observer，不会停止后端 run。为兼容旧的自定义 adapter，只有本组件仍持有本地流且 adapter 声明 `supportsAbort` 时，缺少 `stopRun` 才会退回本地 abort；恢复出的远端 run 不会把 observer abort 伪装成 Stop。

steer 被服务端明确以“run 已结束、run 不匹配或 runtime 不支持”等 pre-admission 状态拒绝时，Runtime 才会回退到 queue。Abort、断网或其他没有 ACK 的不确定投递不会自动重试；否则服务端已持久化但 ACK 丢失时会产生重复 USER。可靠重试需要由 adapter 提供幂等键。

## 属性参数 (Props)

该组件接受 `TronChatRuntimeWindowProps` 对象作为参数，以下是各属性的详细说明：

| 参数名称                 | 类型                                        | 必填 | 功能描述                                                                                                           |
| :----------------------- | :------------------------------------------ | :--: | :----------------------------------------------------------------------------------------------------------------- |
| `adapter`                | `ChatAdapter`                               |  否  | 聊天适配器，用于处理底层聊天数据交互（如加载消息、发送消息等）。如果未提供，默认会创建一个 Mock 适配器。           |
| `config`                 | `ChatClientConfig`                          |  是  | 客户端配置，包含工作空间代码 (workspaceCode)、应用代码 (appCode)、代理代码 (agentCode)、输入占位符等必要配置信息。 |
| `currentUserDisplayName` | `string`                                    |  否  | 当前用户的显示名称，默认为 "User"。                                                                                |
| `debugOptions`           | `ChatDebugOptions`                          |  否  | 调试选项，用于配置组件内部的日志和事件类型调试展示。                                                               |
| `floatingTitle`          | `string`                                    |  否  | 当开启悬浮窗模式时，显示的主标题，默认为 "Main Agent"。                                                            |
| `floatingSubtitle`       | `string`                                    |  否  | 当开启悬浮窗模式时，显示的副标题，默认为 "Workspace assistant"。                                                   |
| `initialSession`         | `ChatSession \| null`                       |  否  | 初始会话状态，用于在组件初次渲染时指定默认的活动会话。                                                             |
| `onError`                | `(message: string, title?: string) => void` |  否  | 发生错误时的回调函数，默认使用 `console.error` 打印，可用于对接全局提示组件（如 Toast）。                          |
| `onInfo`                 | `(message: string, title?: string) => void` |  否  | 提示信息的回调函数，可用于对接全局提示组件。                                                                       |
| `onSessionChange`        | `(session: ChatSession \| null) => void`    |  否  | 当前活动会话发生改变时的回调函数。                                                                                 |
| `windowState`            | `ChatFloatingState`                         |  否  | 受控的悬浮窗状态（如打开、关闭、最小化等）。                                                                       |
| `onWindowStateChange`    | `(state: ChatFloatingState) => void`        |  否  | 悬浮窗状态发生改变时的回调函数。                                                                                   |

自定义区域通过 Vue slots 提供：`#header`、`#sidebar` 和 `#footerAddon`。这些不是 `ReactNode` props。

## 核心对象类型定义

为了更好地理解组件的各个参数，以下列出了常用对象的类型结构与功能说明：

### ChatClientConfig (客户端配置)

核心配置对象，用于控制界面的基础行为和绑定后端参数。
| 属性名称 | 类型 | 描述 |
| :--- | :--- | :--- |
| `workspaceCode` | `string` | 工作空间标识代码（通常为**必填**项）。 |
| `appCode` | `string` | 关联的应用代码。 |
| `agentCode` | `string` | 关联的代理代码。 |
| `taskCode` | `string` | 关联的任务代码。 |
| `title` | `string` | 组件内显示的标题名。 |
| `inputPlaceholder` | `string` | 输入框的占位提示文本。 |
| `disabledInput` | `boolean` | 是否禁用输入功能。 |
| `floating` | `boolean` | 是否为悬浮窗模式，设为 `false` 则表现为内嵌面板模式。 |
| `showAgui` | `boolean` | 是否展示 AG-UI 执行过程和事件流日志。 |

### ChatAdapter (聊天适配器)

`ChatAdapter` 是用于连接 UI 组件和后端聊天的核心接口。你需要实现这些方法来处理底层的 API 请求：

```typescript
export interface ChatAdapter {
  capabilities: ChatAdapterCapabilities; // 声明该适配器支持的能力 (例如是否支持流式返回、中断、Agui 等)
  createSession: (input: CreateChatSessionInput) => Promise<ChatSession>;
  getActiveRun?: (
    sessionCode: string,
    signal?: AbortSignal,
  ) => Promise<ChatActiveRunResult>;
  loadMessages: (sessionCode: string) => Promise<ChatMessage[]>;
  loadAguiEvents: (sessionCode: string) => Promise<ChatAguiEvent[]>;
  loadProtocolEvents?: (sessionCode: string) => Promise<ChatAguiStreamEvent[]>;
  steerMessage?: (
    session: ChatSession,
    input: SendChatMessageInput,
    context: { expectedRunCode: string },
    signal?: AbortSignal,
  ) => Promise<ChatSteerResult>;
  stopRun?: (
    session: ChatSession,
    context: { expectedRunCode: string },
    signal?: AbortSignal,
  ) => Promise<ChatStopResult>;
  streamMessage: (
    session: ChatSession,
    input: SendChatMessageInput,
    handlers?: ChatStreamHandlers,
    signal?: AbortSignal,
  ) => Promise<ChatSendResult>;
}
```

启用 steer 时，`capabilities.supportsSteer` 必须为 `true`，并实现 `steerMessage`。内置 Internal/OpenAPI adapter 会向各自的 `/chats/sessions/steer-message` POST `sessionCode`、`expectedRunCode` 和消息内容；该请求返回包含服务端 `messageCode` 的 ACK，当前 SSE 输出流保持不变。`RUN_STARTED.supportsSteer` 会进一步声明当前 runtime 是否支持 steer。

恢复和停止远端 run 时，适配器需分别声明 `capabilities.supportsActiveRun`、`capabilities.supportsStop`，并实现 `getActiveRun`、`stopRun`。低层 SSE 的 abort 仅表示 observer detach，不等同于 Stop。

直接使用低层 `TronChatPanel` 时，调用方负责通过 `runningSubmitModes`、`queuedMessages`、`onSend(mode)`、`onEditQueuedMessage` 和 `onRemoveQueuedMessage` 控制运行中意图与队列；运行中 Enter 只会触发 queue，steer 只接受 `Command/Ctrl+Enter`，不会渲染独立按钮。省略 `runningSubmitModes` 时，运行中按 Enter 不会提交或停止，Stop 只能通过独立按钮显式触发。

普通业务接入应优先使用 `TronChatRuntimeWindow`；只有调用方已经自行管理会话、active run、SSE observer、队列和 Stop 生命周期时，才直接使用 `TronChatPanel`。

### ChatSession (聊天会话)

组件内维护和回调的当前会话状态信息对象：

```typescript
export interface ChatSession {
  id: number;
  sessionCode: string; // 会话的唯一编码
  workspaceCode: string;
  userCode?: string | null;
  appCode?: string | null;
  taskCode?: string | null;
  agentCode?: string | null;
  title: string; // 会话标题
  status: "ACTIVE" | "ARCHIVED";
  createdAt: string;
  updatedAt: string;
}
```

### ChatFloatingState (悬浮窗状态)

当悬浮窗的打开、关闭、折叠等状态需要由外部受控时，需通过此对象传递：

```typescript
export interface ChatFloatingState {
  open: boolean; // 悬浮窗是否展开
  collapsed: boolean; // 悬浮窗是否处于最小化/折叠状态
  height: number; // 窗口高度
  width: number; // 窗口宽度
  x: number; // X轴坐标位置
  y: number; // Y轴坐标位置
}
```

### ChatDebugOptions (调试选项)

用于本地开发时观测内部事件流行为：

```typescript
export interface ChatDebugOptions {
  debugEventTypes?: string[]; // 允许打印到面板上的特定类型 AG-UI 事件
  recentTurnLimit?: number;
}
```

## 使用示例

```vue
<script setup lang="ts">
import { TronChatRuntimeWindow } from "@tron/chat-vue";
import { createOpenApiChatAdapter } from "@tron/chat-openapi-adapter";

const adapter = createOpenApiChatAdapter({
  baseUrl: "http://localhost:8080",
  openAppCode: "",
  appKey: "",
  userCode: "",
  getAccessToken: () => {
    return "";
  },
});
</script>
<template>
  <TronChatRuntimeWindow
    :adapter="adapter"
    :config="{
      workspaceCode: 'my-workspace',
      title: '智能助手',
    }"
    :currentUserDisplayName="张三"
    :floatingTitle="智能助手"
    :onSessionChange="(session) => console.log('会话切换:', session)"
  />
</template>
```
