> 当前直接 ModelClient 重构与 UI 原生结果整改的验收，以任务 `direct-model-architecture-evidence/results.json`、`reopened-ui-fault-evidence/results.json` 及最终源码快照为准；其余归档是各自源码基线的历史证据。

# 模型请求统一到 Framework

Tron 生产模型请求由 Framework 官方 HTTP model plugin 执行；独立文本与 typed operation 直接复用 SDK `ModelClient` / `ModelOperations`。Tron 保留 DTO/配置、用户与 workspace 权限、PersonAI 登录与凭据刷新、业务路由和产品结果快照，不拥有模型 POST、私有 SSE parser 或隐式模型重发。

| 产品入口 | 受控入口 |
| --- | --- |
| ReAct 主聊天、工具后续调用 | Framework ModelInvoker → TronChatModel 薄映射 → SDK single-attempt transport |
| 自动 overflow 压缩 | Framework ContextReductionJob workflow → memory MemoryCompactor → invoke_auxiliary |
| 手动上下文压缩 | ModelClient.compact_memory → SDK ModelClient.workflow_observed → memory MemoryCompactor |
| goal judge / JSON repair | SDK ModelClient.chat_with_validator_observed + ModelResponseValidator，共享调用数与总 token 预算 |
| 标题、session summary、terminal/prompt/RAG 文本与独立 LLM generate | 产品 ModelClient.generate → SDK ModelClient.chat_observed |
| RAG 摄取、查询、重建 embedding / rerank | SDK ModelClient.embed_observed / rerank_observed；PersonAI chat fallback 也由 SDK 构造执行 |
| PersonAI 激活验证 chat / embedding | 同一个 ModelClient owner 的直接 chat / embedding probe |
| 自定义模型 verify-preview / verify-and-persist | SDK ModelProbeKind payload + 已注册 HttpProbeModel 的直接调用；preview 不写产品状态 |

共享 SDK ModelClient owner 绑定 FrameworkHost.close；with_runtime / with_options 视图共享 stop/drain 状态。独立调用不构建 Agent，不启动原生 run，不创建 session、历史或工具循环。ModelInvocationOptions.for_call 仅创建 ledger/deadline/cancellation/extensions 控制数据；for_run 复用父 ledger、取消与 deadline。主 run 停止、goal-judge 等待取消、业务 timeout/drop 和 host close 都释放受控模型工作；首次使用前 close 也拒绝后续直接调用。模型 timeout/不完整流不会盲目重试，空流不会再发非流请求。单次口 generate_once/generate_stream 只由主 Framework invoker 与自动压缩 auxiliary 适配器调用，源码 architecture test 防回退。

PersonAI 的官方加密 envelope、X-Product-Code 和 conversation/quality 字段保持。产品 factory 为每次发送绑定凭据 attempt fingerprint；401 恢复和终态 rejection 校验该身份。账号切换与迟到 refresh 通过同一 SQLite 事务核对原 user/server/encrypted credentials，拒绝过期结果，保护新账号凭据和状态。普通外部 HTTP 401 不改变 PersonAI 连接。

模型错误分类从 SDK 保留到公共执行控制，不先降成字符串再猜 transient。400、权限、quota、device unbound 不进入通用 retry；PersonAI 429 保持额度不足语义。当前模型调用允许真实图片和附件输入；SDK 保守预算包括原生 payload 与 continuation metadata，最终 max_tokens 服从 Core 夹取。

dispatch 次数是发送预算，发送前失败也可能消耗 slot；模型 HTTP 次数不超过 SDK dispatch 数。provider 未报告 usage 时产品仍为 unavailable，Core 保留已预留估算，避免后续 repair 免费重发。压缩的 `usage` 是本次 memory ledger 差额，用于保守预算计费；`provider_usage` 仅在所有 summary 响应完整报告且与该差额一致时提供。产品 checkpoint/audit 只映射 `provider_usage`，缺失或含失败尝试时保持 tokenUsage null / actual unavailable。

源码边界回归：`tron-backend/tests/model_architecture.rs` 全 src 扫描模型 POST/私有 parser/单次口调用。允许的其它网络请求是路由注册、登录/市场目录等产品服务、MCP/外部 agent bridge 与 MinerU 文档解析。分块短期记忆、历史 projection/CAS 和自动手动压缩归属参见 [memory 接入](memory-framework-migration.md)。UI 与外部 Agent 协议能力保持既有接入文档。

首轮模型请求统一验收保存在 `model-unification-evidence`，provider 修正轮保存在 `context-provider-correction-evidence`；两者均为各自冻结源码的历史证据。本轮原生模型绑定、生命周期和 goal 驱动整改使用 `sdk-integration-correction-evidence`，以本轮实际门禁与源码清单为准。

模型 HTTP client 通过 SDK `ModelHttpClient::from_builder` 保留宿主 proxy/connect/pool/timeout 配置并强制禁止自动 redirect；307/308 是终态错误，配置应使用实际模型 endpoint，不能隐式再次 POST。PersonAI 官方 gateway request 加密与 header 仍保留。

## 原生模型绑定与观察结果

主 run 使用公开 `ModelExecutionScope` 注册经产品 ACL 授权的真实 `ModelRef`、协议和 descriptor。产品只给候选排序；SDK `ModelSelectionStrategy`、`ModelRuntime` 与 `ModelRecoveryPolicy` 负责选择、重试及 unavailable 恢复。辅助摘要绑定独立真实模型，沿同一 SDK run 的调用、输入、输出、时间与取消约束执行。固定和 Auto 选择均不再通过占位 `tron/route` 修改宿主模型。动态授权刷新只接受本 scope 的相同配置；模型端点或能力配置发生变化时明确要求新建 scope，避免 fresh descriptor 与旧宿主配置混用。

Descriptor 的 `context_window_tokens` 表示输入与输出总窗口，`max_input_tokens` 表示输入硬限额；实际输入容量取两者约束的最小值。最终输出额度同时受请求、run 硬上限和模型输出上限夹取；未知模型窗口保持未知，不虚构容量。独立 chat 和 typed embedding/rerank 同样在发送前执行容量检查。

`ObservedModelResult` 在成功和错误时都保留 dispatch、尝试与 retry 数、真实选择、保守 ledger charge、完整 provider usage 或已知 subtotal。实际 usage 完整性依赖每次派发均报告，不要求实际费用等于保守估算；缺失段不按零伪造完整值，cached/reasoning 指标也仅在所有已报告段均提供该字段时聚合。现有兼容 DTO 调用仍可只取结果，显式 observed 口提供完整 SDK 观察。

模型客户端默认单次、repair、观察与关闭能力均不静默退化为另一个受控任务；不支持的实现明确返回 unsupported。goal judge 的直接模型调用沿父 SDK 控制上下文执行，共享树级硬限额，repair 不可通过替换 extensions 重置该计数。真实 SDK 历史通过 occurrence witness 保留 typed Media、Resource、structured 和 model context，产品 Data JSON 无法创建 tool 或 continuation 协议身份。

主 conversation 的 `tron.responses.v2` 仍是产品续传策略快照，但其 identity 已绑定 SDK 真实 scoped ModelRef/protocol；旧占位 v2 和 v1 无法匹配时完整 replay。独立 HTTP 模型调用的 counted continuation private witness 与 `wire.response.v1` opaque 响应桥不代表这个持久化格式。SDK `ModelInvoker` 的每步 tracker 取调用方与有效 run 上限的最小值；直接模型调用的 workflow/validator admission 还按剩余实际派发量前置拒绝，不产生额外 ModelDispatched 或树级 slot。

本轮删除新增的模型执行 Job façade，回归保留直接 text/embed/rerank、validator、重试、超时、取消、Drop、关闭、父 ledger 与 memory CAS 语义。`ContextReductionJob` 仍是既有 Hook 的 reduction plan 数据；图片/视频供应商的真实异步 Job 状态继续保留。历史冻结证据不代表本轮直接接口的最终验收结果；本轮结果由最终源码清单和门禁日志确认。
