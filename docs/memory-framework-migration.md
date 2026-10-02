# Framework memory integration

短期记忆与压缩归属 `eliteforge-agent-memory`。Tron 保留产品触发、配置覆盖、权限与
family membership、完整历史/附件 DTO 映射、业务 goal/todo/session summary、事务存储和展示。
普通会话标题与业务 session summary 继续原业务语义，通过统一 SDK model job 执行。

| 产品入口 | Framework memory 公共能力 |
| --- | --- |
| STM config/model context budget | `ContextBlockBudgets::plan` |
| history retention / requested tail | `select_history` / 完整最新 turn 与 exact tool ID 边界 |
| assembler | 可信 scope 的独立来源 DTO → `MemoryContextContributor` + builtin instructions/history → 唯一 `DefaultContextAssembler` |
| related-session block | 产品权限检索 → scoped contributor → SDK source-aware quota / 每块 reducer |
| manual HTTP compaction | ModelClient memory job → `MemoryCompactor` |
| automatic / provider overflow compaction | Framework ContextReductionJob workflow → 同一个 `MemoryCompactor` |
| checkpoint write | `ShortTermMemoryEngine::write_projection` → product `ShortTermMemoryProjectionStore` → SQLite atomic CAS |

控制预算计算包含模型 output reserve、tool schema、tool arguments、附件 metadata 和图片
序列化数据。图片采用有限的保守序列化估计；宿主 model catalog 的 estimator 使用同一
STM estimate。保留普通有效图片/附件的回归，另由原生 SDK 做最终输入容量检查。
相关会话的业务授权、有效期、来源标签和 goal/todo 展示内容仍由宿主提供；通用预算、
截断、分块顺序和历史保留算法由 SDK memory 提供。

生产 namespace 绑定为 workspace + session；先从宿主 `ensure_session` 验证可信 workspace，
再逐历史 message/part 检查 workspace/session 一致性。用户授权继续由已有宿主 ACL 和
family membership 控制。SDK 可使用 user namespace，其回归证明该公共接口不能交叉读写
其他用户 namespace 或 session，不额外改造产品权限协议。

压缩只读取 model-visible 语义数据。audit/internal、raw reasoning 和 provider-owned 元数据
不发送到摘要模型。当前授权 rich input 保留完整图片和附件映射。历史真实 user turn
保持分段边界；tool record 作为 untrusted JSON data，避免跨 summary 模型传原生 orphan
工具协议。reason/operator note/retained recent boundary 通过 SDK `MemoryCompactionMetadata`
作为不可信数据参与摘要，没有产品自定义压缩 prompt 或产品侧 summary 执行器。

Checkpoint 采用严格 snapshot CAS：prepare 保存 previous checkpoint generation 与
`selection_source_end_seq`（包括 raw tail 的完整来源末尾）；commit 在同一个
`BEGIN IMMEDIATE` 事务比较 trusted scope、当前 checkpoint 与 completed non-compaction
来源最大 seq。压缩期间追加新的 completed 历史或另一摘要先提交时返回 conflict，
迟到结果不写 checkpoint、不覆盖新检查点、不删除原历史。`source_end_seq` 是压缩
head 的结尾，不能拿它和完整来源最大 seq 比较。自动当前 run 的完整 turn 被固定保留。

旧 checkpoint `full_source_includes_tail` / `head_only_tail_replayed` coverage 和 selector
版本读兼容保留；新摘要持久化后原 transcript 与附件记录不被重写或删除。

回归入口：`tests/memory_framework_boundary.rs`、STM assembler unit tests、
`tests/short_term_memory_transcript.rs` 的 two-connection CAS 和既有 checkpoint/replay
用例；Framework `memory/tests/{short_term,compaction}.rs` 与既有 context compression
用例。精确命令、cwd、环境、exit/log 旧模型统一证据保存在 task/model-unification-evidence；provider 修正历史证据保存在 task/context-provider-correction-evidence；本轮统一 ContextContributor、真实 seed 配额与原生历史保真的检查以 task/sdk-integration-correction-evidence 为准。

Compaction usage distinguishes conservative SDK ledger charges from provider reports.
Only a complete aggregate of reported nonzero usage is written to checkpoint actual
usage. Any unknown segment leaves product token usage unavailable; ledger reservations
remain charged. Mandatory seed and current-history blocks survive disabled plan flags,
including rich input; their estimates still participate in final model admission.


## Provider 配置与控制

同步 STM prepare 只读取已授权来源、映射 DTO 与准备历史选择资料。异步 Framework
`DefaultContextAssembler` 读取每块一次、协调配额与触发、执行该块策略并校验最终窗口。
文件 `shortTermMemory.context.providers.order` 决定实际模型消息块顺序；tool schema
保留原生协议位置。builtin instructions 合并宿主授权 prefix 与已注册 context/skill 指令，
builtin history 保留真实角色、tool pair 和完整最新 turn。data contributor 的 JSON 不能
升级成工具协议，错误在模型 HTTP 发送前拒绝。

`context.blocks` 的 `maxRatio` 百分比字符串及旧 `maxRatioBps` 继续兼容；实际配额基数
为模型 window 减 output reserve、再减 SDK 估算的 tools/options 协议成本，`maxTokens`
仍为更严格硬上限。固定块未使用配额借给 history，数据压缩后 history 再取实际余量。
seed quota 由不可压缩的系统指令与 `application_context` 共享；后者只包含原 user 类型
执行范围事实，保留 Keep 策略与授权约束，虽可配置顺序但没有独立压缩配额或策略。

checkpoint、family、working、history 各自配置 `strategy`、`triggerRatio`、`targetRatio`
和 `minGainTokens`。默认 checkpoint/family/working 为 Bound、history 为
Checkpoint；data 可选 Semantic，真实工具历史仅支持 Keep 或 Checkpoint。单块阈值
独立于全局 `compaction.triggerRatio`；target 必须低于 trigger，形成迟滞。全局协调所有
配置了压缩策略的块，Keep/无可移除历史可以 no-op；未达最小收益保留原来源。
最终窗口仍超硬预算则明确失败，不能压系统指令、拆最新完整工具 turn 或静默扩大容量。

Semantic 使用 SDK memory compressor 与官方 SQLite SummaryStore；稳定内容摘要加
新 raw 来源复用，cache identity 包含 scope、该块 body/policy 和实际摘要模型配置。
无关 history append 或新的 history checkpoint 不使未变化的 working/family 摘要失效。
动态来源在模型调用前后重查真实 scope/body revision；迟到结果拒绝，checkpoint commit
沿用同一事务 CAS。压缩/refresh 不建立额外 job，仍共享当前 run usage、取消及调用预算。

`maxAuxiliaryCallsPerStep` 是额外收紧上限，默认 16 不是允许增加模型花费；实际仍受原
SDK 每步 `max(3, maxModelRetries + 1)`、当前 run 总预算和主回答预留 1 次限制。
默认无重试的步骤最多 2 次辅助加 1 次主回答；多块或分段超过余量直接预算失败。
未触发时零辅助调用，summary cache 命中不重复计模型用量或重发审计事件。

`prices` 可选 input/cachedInput/output 非负十进制字符串及 currency，经验证作为宿主
估算输入保存；当前决策采用 capacity、稳定 prefix/cache、target/minGain 和共享调用上限
启发式，不执行按价格自动最优路由，不计算无来源的价格或节约金额。缺价格、信息重取
价格或 provider cache 指标时费用未知。已有 ProviderUsagePanel 继续读取原 audit 事件，
无新增统计/UI。provider actual usage 与 SDK 保守 ledger charge 分开；cache/reasoning
字段只有全部摘要段报告时才有值，未知/失败段不能当 0 或完整 actual。现有 session UI
累计的是已报告 subtotal，不能推断覆盖所有未知调用的总费用。

旧文件仅配置 `compaction.retainedHistoryRatio` 时，history target 继承该值；显式 `context.providers.history.targetRatio` 优先。旧版 100% 保留且未显式配置新 history strategy/target 时映射为 Keep，不发摘要请求；显式 Checkpoint 的 target 必须低于 trigger。同步预览的全局压缩预测保持旧阈值语义，独立块触发和最终投影由 DefaultContextAssembler 裁定。
