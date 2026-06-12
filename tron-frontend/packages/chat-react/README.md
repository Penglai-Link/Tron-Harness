# TronChatRuntimeWindow 组件文档

`TronChatRuntimeWindow` 是一个用于渲染聊天运行时窗口的 React 组件。它支持悬浮窗或内嵌面板模式，并提供完整的会话状态管理、消息收发和 Agui 事件处理等功能。

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
| `footerAddon`            | `ReactNode`                                 |  否  | 底部附加组件，通常用于在输入框下方或周围渲染额外的自定义元素。                                                     |
| `headerContent`          | `ReactNode`                                 |  否  | 头部内容扩展，可用于在面板顶部添加自定义按钮或信息（仅在非悬浮窗模式时显示）。                                     |
| `initialSession`         | `ChatSession \| null`                       |  否  | 初始会话状态，用于在组件初次渲染时指定默认的活动会话。                                                             |
| `onError`                | `(message: string, title?: string) => void` |  否  | 发生错误时的回调函数，默认使用 `console.error` 打印，可用于对接全局提示组件（如 Toast）。                          |
| `onInfo`                 | `(message: string, title?: string) => void` |  否  | 提示信息的回调函数，可用于对接全局提示组件。                                                                       |
| `onSessionChange`        | `(session: ChatSession \| null) => void`    |  否  | 当前活动会话发生改变时的回调函数。                                                                                 |
| `sidebarContent`         | `ReactNode`                                 |  否  | 侧边栏内容，用于在聊天界面旁提供额外的上下文或功能面板。                                                           |
| `windowState`            | `ChatFloatingState`                         |  否  | 受控的悬浮窗状态（如打开、关闭、最小化等）。                                                                       |
| `onWindowStateChange`    | `(state: ChatFloatingState) => void`        |  否  | 悬浮窗状态发生改变时的回调函数。                                                                                   |

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
  loadMessages: (sessionCode: string) => Promise<ChatMessage[]>;
  loadAguiEvents: (sessionCode: string) => Promise<ChatAguiEvent[]>;
  loadProtocolEvents?: (sessionCode: string) => Promise<ChatAguiStreamEvent[]>;
  streamMessage: (
    session: ChatSession,
    input: SendChatMessageInput,
    handlers?: ChatStreamHandlers,
    signal?: AbortSignal,
  ) => Promise<ChatSendResult>;
}
```

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

```tsx
import { TronChatRuntimeWindow } from "@tron/chat-react";

export default function App() {
  return (
    <TronChatRuntimeWindow
      config={{
        workspaceCode: "my-workspace",
        title: "智能助手",
      }}
      currentUserDisplayName="张三"
      floatingTitle="智能助手"
      onSessionChange={(session) => console.log("会话切换:", session)}
    />
  );
}
```
