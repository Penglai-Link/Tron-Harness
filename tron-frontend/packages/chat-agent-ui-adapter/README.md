# @tron/chat-agent-ui-adapter

把 Tron 的会话、消息、附件、模型选择、用户提问与 AG-UI 契约转换为 Agent UI 公共契约。
适用于 `@tron/chat-internal-adapter` 和 `@tron/chat-openapi-adapter`；认证与 API 请求仍由原传输 adapter 负责。

```tsx
import { ChatRuntimeWindow } from "@personai/agent-ui-chat-react";
import { createAgentUiAdapter, toUiInput } from "@tron/chat-agent-ui-adapter";
import { createOpenApiChatAdapter } from "@tron/chat-openapi-adapter";

// 按部署环境提供合法的 OpenAPI 参数。
const host = createOpenApiChatAdapter(openApiOptions);
const adapter = createAgentUiAdapter(host, { workspaceCode: "WS-1" });

<ChatRuntimeWindow
  adapter={adapter}
  config={{ floating: false }}
  createSendInput={(_session, content) => toUiInput({ content, workspaceCode: "WS-1" })}
/>;
```

Tron 页面通过 `AgentChatWindow` 保留完整产品输入与会话范围。公共 UI 的 `metadata.tron` 是宿主快照，
往返时不得删除，否则操作会拒绝执行。该快照不是服务器授权依据。
模型 Code、质量路由、系统 Agent 会话、pageContext、附件引用、expectedRunCode、AbortSignal 以及恢复回答都由转换层保留。

终端使用 `toUiEvent` 和 `fromUiResume` 处理同一套提问/恢复字段，PTY 生命周期由 Tron 后端管理。

压缩详情：历史接口按步骤 ID 返回 `compactionDetails`，适配器映射为
`HistoryStep.compaction.blocks`；实时流透传 `agent.memory.compaction.details`。
记忆块名称与结果由宿主的 notices 文案回调国际化，历史摘要生成单独标注。

Agent UI 依赖统一固定为公共 npm 的 `1.0.0-preview.5`，包含压缩详情展示能力。
