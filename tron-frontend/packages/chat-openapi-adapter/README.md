# @tron/chat-openapi-adapter

`@tron/chat-openapi-adapter` 提供了用于对接 Tron Open API 开放平台的聊天请求适配器。它实现了 Tron 宿主 `ChatAdapter` 接口（通过 `@tron/chat-agent-ui-adapter` 转为公共 UI 契约），负责与 `/open-api/v1/...` 相关的开放接口进行安全交互。该适配器内置了基于 AppKey 和数字签名的鉴权机制。

## 安装

确保你的项目中已经引入了该包。

## 使用说明

通过 `createOpenApiChatAdapter` 方法创建一个适配器实例，并将其传递给聊天 UI 组件。

### 基本示例

```typescript
import { createOpenApiChatAdapter } from "@tron/chat-openapi-adapter";
import { ChatRuntimeWindow } from "@eliteforge/agent-ui-chat-react";
import { createAgentUiAdapter, toUiInput } from "@tron/chat-agent-ui-adapter";

// 1. 创建 Open API 适配器实例
const hostAdapter = createOpenApiChatAdapter({
  baseUrl: "https://openapi.your-domain.com",
  openAppCode: "YOUR_OPEN_APP_CODE",
  appKey: "YOUR_APP_KEY",
  userCode: "USER_001", // 当前使用聊天的用户标识
});

// 2. 将适配器传递给 UI 组件
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

### `createOpenApiChatAdapter(options?: OpenApiAdapterOptions): ChatAdapter`

创建一个用于 Open API 环境的适配器实例。

#### `OpenApiAdapterOptions` 配置参数

该配置对象继承了部分 `ChatClientConfig` 的参数，并增加了鉴权相关项：

| 参数名称 | 类型 | 必填 | 描述 |
| :--- | :--- | :---: | :--- |
| `baseUrl` | `string` | 否 | 请求的基础 URL。如果不传，则使用当前域名。 |
| `openAppCode` | `string` | 是* | 开放平台的应用 Code，用于签名。 |
| `appKey` | `string` | 是* | 开放平台分配的 AppKey，用于签名与接口验权。 |
| `userCode` | `string` | 否 | 当前用户标识（优先使用 `userCode`，其次使用 `userId`）。 |
| `userId` | `string` | 否 | 当前用户标识的备选字段。 |
| `credentials` | `RequestCredentials` | 否 | fetch 请求的 credentials 选项，默认为 `"omit"`。 |
| `nonceFactory` | `() => string` | 否 | 自定义生成签名所需随机数 (nonce) 的函数。如果未提供，底层会自动生成。 |
| `timestampFactory` | `() => number` | 否 | 自定义生成签名所需时间戳的函数。如果未提供，底层会使用当前时间。 |

*(注：尽管类型定义上某些字段是可选的，但在调用 OpenAPI 时签名强依赖 `openAppCode` 和 `appKey`，实际使用中请务必传入)*

## 支持的能力 (Capabilities)
该适配器默认声明支持以下聊天能力：
- **`supportsAbort`**: `true` (支持中断生成)
- **`supportsAgui`**: `true` (支持获取 AG-UI 事件)
- **`supportsProtocolReplay`**: `true` (支持协议回放)
- **`supportsStreaming`**: `true` (支持流式响应)

该适配器同时支持会话私有附件的上传与下载，附件请求使用与其他 Open API 请求相同的 HMAC 签名机制。
