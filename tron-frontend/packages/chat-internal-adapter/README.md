# @tron/chat-internal-adapter

`@tron/chat-internal-adapter` 提供了用于 Tron 内部系统的聊天请求适配器。它实现了 Tron 宿主 `ChatAdapter` 接口（通过 `@tron/chat-agent-ui-adapter` 转为公共 UI 契约），负责与 Tron 的内部 API（`/api/v1/chats/...` 等）进行数据交互，包括会话管理、消息收发、流式响应（SSE）及 AG-UI 事件流。

## 安装

如果你的项目尚未引入该包，请确保将其添加到依赖中。通常在 monorepo 内可以直接引用。

## 使用说明

你需要通过 `createInternalChatAdapter` 方法创建一个适配器实例，并将其传递给聊天 UI 组件（通过 `@tron/chat-agent-ui-adapter` 接入公共 `ChatRuntimeWindow`）。

### 基本示例

```typescript
import { createInternalChatAdapter } from "@tron/chat-internal-adapter";
import { ChatRuntimeWindow } from "@personai/agent-ui-chat-react";
import { createAgentUiAdapter, toUiInput } from "@tron/chat-agent-ui-adapter";

// 1. 创建适配器实例
const hostAdapter = createInternalChatAdapter({
  workspaceCode: "WS-XXXXX",
  baseUrl: "https://api.your-domain.com", // 可选，默认使用当前域名
  getAccessToken: () => {
    // 返回当前的身份验证 Token
    return localStorage.getItem("access_token"); 
  }
});

// 2. 将适配器传递给组件
const adapter = createAgentUiAdapter(hostAdapter, { workspaceCode: "WS-XXXXX" });

export default function App() {
  return (
    <ChatRuntimeWindow
      adapter={adapter}
      config={{ floating: false }}
      createSendInput={(_session, content) => toUiInput({ content, workspaceCode: "WS-XXXXX" })}
    />
  );
}
```

## API 参考

### `createInternalChatAdapter(options: InternalChatAdapterOptions): ChatAdapter`

创建绑定工作空间的适配器实例。切换工作空间时创建新实例，所有聊天、SSE、回放和附件请求都携带该空间的 `X-Workspace-Code` 请求头。

#### `InternalChatAdapterOptions` 配置参数

| 参数名称 | 类型 | 必填 | 描述 |
| :--- | :--- | :---: | :--- |
| `workspaceCode` | `string` | 是 | 此适配器所属的工作空间，不能从全局选中状态动态读取。 |
| `baseUrl` | `string` | 否 | 请求的基础 URL。如果不传，则默认使用当前页面的域名。 |
| `credentials` | `RequestCredentials` | 否 | fetch 请求的 credentials 选项，默认为 `"include"`。 |
| `getAccessToken` | `() => string \| null \| undefined \| Promise<string \| null \| undefined>` | 否 | 获取 Bearer Token 的回调函数，支持异步返回。该 Token 会被放入请求头的 `Authorization` 字段中。 |

## 支持的能力 (Capabilities)
该适配器默认声明支持以下聊天能力：
- **`supportsAbort`**: `true` (支持中断生成)
- **`supportsAgui`**: `true` (支持获取 AG-UI 事件)
- **`supportsProtocolReplay`**: `true` (支持协议回放)
- **`supportsStreaming`**: `true` (支持流式响应)

该适配器同时支持会话私有附件的上传与下载。
