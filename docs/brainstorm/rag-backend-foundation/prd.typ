= PRD: RAG 后端底座 MVP

#import "@preview/mmdr:0.2.2": mermaid

== Summary
- Product: Tron 后端 RAG 底座 MVP。
- Problem: Memory 和 Project Doc 需要一个统一的知识写入、存储、索引、检索和 Context Pack 组装底座；现有 Graph 服务不在本阶段范围内，RAG 不能把代码图谱能力提前卷入。
- Target users: Memory 模块、Project Doc 调用方、Agent 运行时、后端开发者和后续知识检索模块。
- Outcome: 后端可以通过稳定 API 写入 `memory-card` 与 `project-doc`，按 Profile 执行全文/向量检索，并返回可直接供 Agent 使用的检索结果或 Context Pack。

== Background
- Why now: ONES《RAG MVP 设计方案》已经明确 RAG 是多模块共享的知识存储与检索底座，不是 Memory 的子模块。
- Current pain: Memory、项目文档和未来代码图谱都需要知识检索能力；如果各模块各自实现存储、切分、索引、scope、向量和上下文组装，会产生重复逻辑和不一致边界。
- Business context: 第一阶段聚焦后端底座，先支持 Memory 和 Project Doc 的真实写入与检索，为后续 SummaryTree、Code Graph 和可替换存储适配器留下扩展点。

== Goals
- 支持 `memory-card` 和 `project-doc` 两个真实 Corpus 的写入、索引、检索和上下文组装。
- 提供 `POST /api/v1/rag/ingest` 与 `POST /api/v1/rag/search` 作为第一阶段主入口。
- 使用 SQLite 实现第一版独立 Corpus 表、Profile 配置、warnings、trace 和索引数据。
- 使用同步 DAG 执行写入流程，并通过有限 strategy registry 控制可执行策略。
- 复用现有 `LlmRuntimeService` 完成 TextUnit embedding 和可选 rerank。
- 使用 `userCode`、`agentCode`、`workspaceCode` 和 `scope_type/scope_code` 做 RAG 数据隔离。
- 让 ingest/indexing warnings、单 Source 索引重试、轻量 search trace 和 EventBus 事件可用。
- 保持 `code-graph`、`GRAPH` retriever、SummaryTree 和外部存储引擎为后续扩展点。

== Scope
- In scope: RAG 后端模块、SQLite 存储、`memory-card` 表、`project-doc` 表、Profile 表、策略注册表、同步 ingest DAG、全文索引、向量索引、检索编排、Context Pack、debug API、indexing retry API、warnings、trace、EventBus 事件、workspace 访问校验。
- Out of scope: 代码图谱写入、`GraphService` 集成、真实 `GRAPH` retriever、SummaryTree 构建、复杂多跳检索、agentic retrieval、Profile 管理 UI、完整权限系统、内容脱敏、Elasticsearch/Neo4j/PostgreSQL/pgvector/Qdrant/Milvus adapter、latest 版本语义、Memory 治理状态流转。

== Feature Requirements
=== Module: RAG 后端底座
==== Feature: RAG 后端底座 - Corpus 存储与 Profile 基础
- name: Corpus 存储与 Profile 配置。
- description: 为 `memory-card` 和 `project-doc` 建立独立 SQLite 表、通用字段、filterable 字段、metadata 规则、Profile 表和 CorpusTableBinding。
- user or role: 后端 RAG 服务、Memory 模块、Project Doc 调用方。
- entry point: RAG 应用服务和 SQLite persistence adapter。
- trigger: 系统启动、RAG 写入、RAG 检索、debug 查询。
- preconditions: SQLite 连接可用；迁移已执行；内置 Profile 和 strategy registry 可加载。
- main flow: 加载 Corpus 配置，解析表绑定，校验 filterSchema，使用模板化 adapter 读写 Source、Document、TextUnit、FTS、Vector、warnings 和 trace。
- inputs and outputs: Corpus ID、Profile ID、filterSchema、table binding 输入；稳定的 repository/index adapter 能力输出。
- permissions or visibility: 数据通过 `scope_type` 和 `scope_code` 隔离；HTTP 层负责 workspace 访问校验。
- related page headings: `Page: Backend API - RAG Debug`。
- related business rule headings: `Rule: RAG 后端底座 - Corpus 边界`, `Rule: RAG 后端底座 - Profile 与 Registry 边界`, `Rule: RAG 后端底座 - Filter Schema`。
- related exception headings: `Exception: RAG 后端底座 - Profile 缺失或策略未注册`, `Exception: RAG 后端底座 - 非法过滤字段`。

==== Feature: RAG 后端底座 - 同步写入 DAG
- name: RAG ingest。
- description: 通过同步 DAG 将调用方提交的 Memory 或 Project Doc 内容解析、规范化、切分、持久化并建立全文/向量索引。
- user or role: Memory 模块、Project Doc 模块、后端调用方。
- entry point: `POST /api/v1/rag/ingest`。
- trigger: 调用方提交 `corpusId`、`ingestProfileId`、`AccessContext`、target scope、`sourceCode`、`sourceVersion` 和原始 payload。
- preconditions: 调用用户有 workspace 访问权；写入目标 scope 与 AccessContext 匹配；Ingest Profile 存在；DAG 无环；必需 Stage 已注册。
- main flow: 加载 Profile，校验 scope，编译 DAG，初始化 `RAW_SOURCE` artifact，同步执行 source、parse、normalize、split、persist、fulltextIndex、vectorIndex，最后更新 Source 状态并返回结果。
- inputs and outputs: 写入请求输入；Source 状态、TextUnit、索引记录、warnings 和 ingest result 输出。
- permissions or visibility: 写入不应用空间继承；USER、AGENT、SPACE 只能写入当前 AccessContext 对应的 code。
- related page headings: `Page: Backend API - RAG Ingest`。
- related business rule headings: `Rule: RAG 后端底座 - 写入 Scope`, `Rule: RAG 后端底座 - Source 版本幂等`, `Rule: RAG 后端底座 - Embedding 降级`。
- related exception headings: `Exception: RAG 后端底座 - DAG 配置非法`, `Exception: RAG 后端底座 - Embedding 失败`, `Exception: RAG 后端底座 - 重复写入同版本`。

==== Feature: RAG 后端底座 - 检索编排与 Context Pack
- name: RAG search。
- description: 按 Retrieval Profile 执行单轮 fan-out 检索，合并全文和向量结果，hydrate 正文，并组装检索结果或 Context Pack。
- user or role: Agent 运行时、Memory 上下文调用方、Project Doc 问答调用方。
- entry point: `POST /api/v1/rag/search`。
- trigger: 调用方提交 `profileId`、query、AccessContext、request filters、limit、tokenBudget 和 context 策略。
- preconditions: Retrieval Profile 存在；请求字段在 filterSchema 中声明；请求或 Profile 提供固定 `sourceVersion`；Profile 至少包含可用 route。
- main flow: 加载 Profile，生成 scope filter，合并过滤条件，执行 query rewrite，生成 routes，并行召回 CandidateHit，执行 fusion、dedup、可选 rerank、hydrate 和 Source active 校验，返回 `RESULT_LIST` 或 Context Pack。
- inputs and outputs: 检索请求输入；CandidateHit、hydrate 结果、warnings、trace 和 context 输出。
- permissions or visibility: 检索按 USER、AGENT、SPACE scope 过滤；RAG 不做角色、字段或内容权限。
- related page headings: `Page: Backend API - RAG Search`。
- related business rule headings: `Rule: RAG 后端底座 - 检索 Scope`, `Rule: RAG 后端底座 - Version Filter`, `Rule: RAG 后端底座 - CandidateHit 统一结构`。
- related exception headings: `Exception: RAG 后端底座 - 缺少版本过滤`, `Exception: RAG 后端底座 - 向量 route 不可用`, `Exception: RAG 后端底座 - Hydrate 后 Source 不可见`。

==== Feature: RAG 后端底座 - Memory 与 Project Doc 领域字段
- name: Memory 和 Project Doc Corpus 字段。
- description: 为 Memory 存储治理字段，为 Project Doc 存储文档字段，让 RAG 可过滤、排序、hydrate，但不接管上层业务状态流转。
- user or role: Memory 模块、Project Doc 模块、RAG 检索调用方。
- entry point: RAG ingest payload、Profile filters、search filters、hydrate 输出。
- trigger: 调用方写入 Memory card 或项目文档，或检索时按领域字段过滤。
- preconditions: filterable 字段已在 filterSchema 中声明；长尾字段进入 metadata。
- main flow: RAG 将核心字段写入物理列，将低频扩展写入 metadata；检索时只允许声明字段参与过滤；hydrate 时返回调用方需要的来源信息。
- inputs and outputs: Memory 治理字段、Project Doc 标题/路径/类型等输入；可过滤列、metadata 和 hydrate payload 输出。
- permissions or visibility: Memory 治理流转由 Memory 模块负责；RAG 只存储字段和值。
- related page headings: `Page: Backend API - RAG Ingest`, `Page: Backend API - RAG Search`。
- related business rule headings: `Rule: RAG 后端底座 - Memory 治理字段只存储`, `Rule: RAG 后端底座 - Metadata 使用边界`。
- related exception headings: `Exception: RAG 后端底座 - 未声明过滤字段`, `Exception: RAG 后端底座 - Metadata 字段误用`。

==== Feature: RAG 后端底座 - 诊断、重试、Trace 与事件
- name: RAG 运行诊断。
- description: 提供只读 debug、单 Source 索引重试、轻量 search trace 和 EventBus 事件，让后端可定位写入和检索问题。
- user or role: 后端开发者、平台运维、调用方问题排查人员。
- entry point: read-only debug API、indexing retry API、trace repository、EventBus。
- trigger: 用户查看 Profile/Source/TextUnits/warnings，或对一个 Source 重试索引，或系统执行 ingest/search/retry。
- preconditions: 调用用户有 workspace 访问权；debug 与 retry 不绕过 RAG scope；retry 的 Source 和 TextUnits 已存在。
- main flow: debug API 返回 Profile、Source、TextUnits、warnings；retry API 基于已持久化 TextUnits 重建指定索引；search trace 保存轻量诊断数据并按 TTL 清理；事件只发布非正文、非向量、非密钥数据。
- inputs and outputs: sourceCode、sourceVersion、stages、trace 开关输入；warnings、retry result、trace rows 和事件输出。
- permissions or visibility: debug 与 retry 使用同一 workspace access guard；trace 不保存正文、Context Pack body、embedding、provider 原始响应或 API keys。
- related page headings: `Page: Backend API - RAG Debug`, `Page: Backend API - RAG Indexing Retry`。
- related business rule headings: `Rule: RAG 后端底座 - Warnings 持久化`, `Rule: RAG 后端底座 - Trace 隐私边界`, `Rule: RAG 后端底座 - Retry 不重跑解析`。
- related exception headings: `Exception: RAG 后端底座 - Retry Source 不存在`, `Exception: RAG 后端底座 - Trace TTL 到期`, `Exception: RAG 后端底座 - Event payload 越界`。

== Page Prototypes And Interactions
=== Page: Backend API - RAG Ingest
- entry: `POST /api/v1/rag/ingest`。
- layout: 后端 JSON API，无前端页面。
- data display: 响应返回 sourceCode、sourceVersion、status、stage results、warnings 和必要索引结果摘要。
- actions: 写入 Memory card 或 Project Doc；按 Profile 执行同步 DAG。
- states: success、failed、partial indexed with warnings、workspace denied、profile invalid、scope invalid。

==== Interaction: RAG Ingest - 提交写入请求
- trigger control: HTTP POST。
- preconditions: 用户已认证并有 workspace 访问权；请求提供 corpus、profile、scope、sourceCode、sourceVersion 和 payload。
- button or control behavior: 不适用。
- form rules: `corpusId`、`ingestProfileId`、`workspaceCode`、`sourceCode`、`sourceVersion` 必填；target scope 必须匹配 AccessContext。
- confirmation behavior: 不适用。
- success feedback: 返回统一响应体，`data.status` 为 active 或带 warnings 的 active。
- failure feedback: 返回统一错误响应，说明配置、scope、payload 或上游模型错误。
- navigation or refresh behavior: 不适用。

=== Page: Backend API - RAG Search
- entry: `POST /api/v1/rag/search`。
- layout: 后端 JSON API，无前端页面。
- data display: 响应返回结果列表或 Context Pack，同时返回 route warnings、traceCode 和分页/limit 相关摘要。
- actions: 按 Profile 检索 Memory 或 Project Doc。
- states: success、empty result、partial route degraded、workspace denied、profile invalid、version filter missing。

==== Interaction: RAG Search - 提交检索请求
- trigger control: HTTP POST。
- preconditions: 用户已认证并有 workspace 访问权；Profile 存在；请求或 Profile 提供固定 sourceVersion。
- button or control behavior: 不适用。
- form rules: query 非空；filters 字段必须在 filterSchema 中声明；limit 和 tokenBudget 不得超过 Profile 限制。
- confirmation behavior: 不适用。
- success feedback: 返回统一响应体，`data` 包含结果或 Context Pack。
- failure feedback: 返回统一错误响应，说明版本、过滤字段、Profile 或 route 错误。
- navigation or refresh behavior: 不适用。

=== Page: Backend API - RAG Debug
- entry: read-only debug API。
- layout: 后端 JSON API，无前端页面。
- data display: Profile 列表、单 Source、Source TextUnits、ingest/indexing warnings。
- actions: 只读查看，不允许修改 RAG 数据。
- states: success、empty、workspace denied、source not found。

==== Interaction: RAG Debug - 查看诊断数据
- trigger control: HTTP GET。
- preconditions: 用户已认证并有 workspace 访问权。
- button or control behavior: 不适用。
- form rules: 查询参数必须包含 workspaceCode；Source 查询需要 corpusId、sourceCode 和 sourceVersion。
- confirmation behavior: 不适用。
- success feedback: 返回统一响应体和诊断数据。
- failure feedback: 返回统一错误响应。
- navigation or refresh behavior: 不适用。

=== Page: Backend API - RAG Indexing Retry
- entry: single-source indexing retry API。
- layout: 后端 JSON API，无前端页面。
- data display: retry result、stage results、warnings。
- actions: 对一个 Source 的 fulltextIndex、vectorIndex 或两者重试。
- states: success、partial warnings、workspace denied、source not found、unsupported stage。

==== Interaction: RAG Indexing Retry - 重试单 Source 索引
- trigger control: HTTP POST。
- preconditions: 用户已认证并有 workspace 访问权；Source 和 TextUnits 已存在。
- button or control behavior: 不适用。
- form rules: 请求包含 corpusId、sourceCode、sourceVersion 和 stages；stages 只允许索引阶段。
- confirmation behavior: 不适用。
- success feedback: 返回统一响应体和重试结果。
- failure feedback: 返回统一错误响应，说明 Source、Stage 或模型错误。
- navigation or refresh behavior: 不适用。

== Business Rules
=== Rule: RAG 后端底座 - Corpus 边界
- condition: Profile、API 或调用方引用 Corpus。
- decision or calculation: 第一阶段只允许 `memory-card` 和 `project-doc` 作为真实写入与检索 Corpus；`code-graph` 只保留扩展位。
- state transition: 不适用。
- permission or data boundary: RAG 不依赖 `GraphService`。
- conflict or priority rule: 与 ONES 设计中未来 Code Graph 能力冲突时，以第一阶段已确认范围为准。
- example: `retriever=GRAPH` 的 route 可以出现在类型枚举中，但不能被启用执行。

=== Rule: RAG 后端底座 - Profile 与 Registry 边界
- condition: RAG 加载 Ingest Profile 或 Retrieval Profile。
- decision or calculation: SQLite 保存 Profile 配置；Rust strategy registry 保存可执行策略；Profile 只能引用已注册 strategy ID。
- state transition: 配置非法时请求失败，不进入执行。
- permission or data boundary: Profile 不能执行用户自定义代码。
- conflict or priority rule: 数据库配置优先决定编排，代码 registry 决定是否可执行。
- example: Profile 引用未知 `SUMMARY_TREE_BUILDER` 时，第一阶段返回配置错误。

=== Rule: RAG 后端底座 - 写入 Scope
- condition: 调用方提交 ingest。
- decision or calculation: USER 只能写当前 `userCode`，AGENT 只能写当前 `agentCode`，SPACE 只能写当前 `workspaceCode`。
- state transition: scope 不匹配时写入失败。
- permission or data boundary: 写入不应用空间继承。
- conflict or priority rule: HTTP workspace access guard 先于 RAG 写入 scope 校验。
- example: 用户在 `workspace-a` 中不能写入 `workspace-b` 的 SPACE 数据。

=== Rule: RAG 后端底座 - Source 版本幂等
- condition: 写入请求包含 `sourceCode` 和 `sourceVersion`。
- decision or calculation: 稳定键由 corpus、scope、sourceCode、sourceVersion 组成；同版本 upsert，多版本共存。
- state transition: Source 可在 pending、active、failed、disabled、archived、deleted 状态间按 RAG 写入结果更新。
- permission or data boundary: 普通检索只读取 active Source。
- conflict or priority rule: 没有 latest 标记，也不自动比较版本大小。
- example: `sourceVersion=2026-05-30` 与 `sourceVersion=2026-05-31` 是两个可共存版本。

=== Rule: RAG 后端底座 - Version Filter
- condition: 执行 search。
- decision or calculation: 请求传入 sourceVersion filter 时使用请求值；请求未传时必须使用 Retrieval Profile 中的固定 sourceVersion。
- state transition: 两者都缺失时返回配置错误。
- permission or data boundary: 版本过滤参与各 route 查询。
- conflict or priority rule: 请求版本条件优先于 Profile 默认版本。
- example: 请求未传版本且 Profile 未配置版本时，RAG 不猜测最新版本。

=== Rule: RAG 后端底座 - Filter Schema
- condition: Profile filters 或 Request filters 包含字段。
- decision or calculation: 所有字段必须在当前 Corpus 的 filterSchema 中声明；同字段取交集，不同字段 AND。
- state transition: 未声明字段导致请求失败。
- permission or data boundary: metadata 中未声明字段不能参与过滤。
- conflict or priority rule: Request filters 不能放宽 Profile filters。
- example: Profile 限定 `memoryType=["RULE","DECISION"]`，请求限定 `memoryType=["RULE"]`，合并后只查 RULE。

=== Rule: RAG 后端底座 - Embedding 降级
- condition: vectorIndex 或 vector route 调用 embedding。
- decision or calculation: `project-doc` 允许向量失败后保留全文能力并记录 warning；`memory-card` 要求 embedding 成功。
- state transition: Project Doc 可 active with warnings；Memory embedding 失败时 failed 或不可见。
- permission or data boundary: warning 不能包含 provider 原始响应或密钥。
- conflict or priority rule: Corpus/Profile degrade policy 覆盖默认行为。
- example: Project Doc embedding provider 超时后，全文索引成功时仍可通过 fulltext route 检索。

=== Rule: RAG 后端底座 - CandidateHit 统一结构
- condition: 多 route 召回结果返回。
- decision or calculation: 所有 route 命中都转换为 CandidateHit，包含 corpusId、sourceId/sourceCode、targetType、targetId、dedupKey、routeId、rank、score、scope、preview 和 metadata。
- state transition: CandidateHit 经过 fusion、dedup、rerank、hydrate 后进入 context assembly。
- permission or data boundary: hydrate 后必须再次校验 Source active。
- conflict or priority rule: 同一 dedupKey 命中时按 fusion/rerank 策略保留更优结果。
- example: fulltext 和 vector 命中同一 TextUnit 时，只返回一个合并后的候选。

=== Rule: RAG 后端底座 - Memory 治理字段只存储
- condition: Memory 写入包含治理字段。
- decision or calculation: RAG 保存治理字段用于过滤、排序和 hydrate，但不决定候选审核、冲突解决、版本晋级或记忆状态流转。
- state transition: 状态流转由 Memory 模块负责。
- permission or data boundary: RAG 不越权修改 Memory 业务状态。
- conflict or priority rule: Memory 模块传入的字段值是 RAG 存储来源。
- example: `review_status` 可作为过滤字段，但 RAG 不把 pending 自动改成 approved。

=== Rule: RAG 后端底座 - Warnings 持久化
- condition: ingest、indexing 或 retry 出现可诊断 warning。
- decision or calculation: warning 写入持久表，并在即时响应中返回摘要。
- state transition: 根据 degrade policy 决定 Source 是否 active。
- permission or data boundary: warning 不保存正文、embedding、provider 原始响应或密钥。
- conflict or priority rule: 持久化 warning 用于 debug API 查看。
- example: vectorIndex 跳过时记录 stage、warningCode、message、sourceCode 和 sourceVersion。

=== Rule: RAG 后端底座 - Trace 隐私边界
- condition: search trace 开启。
- decision or calculation: trace 保存 query text、route counts、timings、warnings、candidate references、result count 和 token usage，TTL 为 7 天。
- state transition: TTL 到期后可清理。
- permission or data boundary: trace 不保存 hydrate 正文、Context Pack body、embedding、provider 原始响应或 API keys。
- conflict or priority rule: 隐私边界优先于排障便利性。
- example: trace 可以记录命中 `sourceCode=doc-1`，不能保存该文档正文。

=== Rule: RAG 后端底座 - Retry 不重跑解析
- condition: 调用 indexing retry API。
- decision or calculation: retry 只重建 fulltextIndex 和/或 vectorIndex；不重新执行 parse、normalize、split。
- state transition: retry 成功后更新索引状态和 warnings。
- permission or data boundary: retry 不改变原始 Source payload 和 TextUnit 内容。
- conflict or priority rule: 缺少 TextUnits 时 retry 失败。
- example: embedding 模型临时不可用恢复后，对同一 Source 重试 vectorIndex。

== Exceptions And Boundaries
=== Exception: RAG 后端底座 - Workspace 访问被拒绝
- scenario: 当前用户无请求 workspaceCode 的访问权。
- expected product behavior: HTTP handler 拒绝请求，RAG 应用服务不执行写入、检索、debug 或 retry。
- user-visible message or fallback: 返回统一错误响应，message 指向 workspace access denied。
- data consistency expectation: 不产生 Source、TextUnit、index、warning 或 trace 写入。
- retry, recovery, or compensation behavior: 授权后重新请求。
- logging/audit expectation: 可记录拒绝事件，但不记录正文。

=== Exception: RAG 后端底座 - Profile 缺失或策略未注册
- scenario: 请求引用不存在 Profile，或 Profile 引用未注册 strategy/retriever/policy。
- expected product behavior: 返回配置错误，不执行 DAG 或 route。
- user-visible message or fallback: 返回统一错误响应，指出 profileId 或 strategyId。
- data consistency expectation: 不产生部分写入。
- retry, recovery, or compensation behavior: 修复 Profile 或 registry 后重试。
- logging/audit expectation: 记录配置错误和请求摘要。

=== Exception: RAG 后端底座 - DAG 配置非法
- scenario: Ingest DAG 有环、缺少 persist Stage、Artifact 输入输出不兼容，或缺少索引 Stage 且未显式 noIndex。
- expected product behavior: 编译期失败，ingest 不开始执行。
- user-visible message or fallback: 返回统一错误响应，说明非法 DAG 原因。
- data consistency expectation: 不创建 pending Source。
- retry, recovery, or compensation behavior: 修复 Profile 后重试。
- logging/audit expectation: 记录 DAG 校验失败。

=== Exception: RAG 后端底座 - 非法过滤字段
- scenario: Request filters 或 Profile filters 使用 filterSchema 未声明字段。
- expected product behavior: search 请求失败。
- user-visible message or fallback: 返回统一错误响应，指出非法字段。
- data consistency expectation: 不产生 trace 正文或候选结果。
- retry, recovery, or compensation behavior: 移除字段或将字段加入 filterSchema 和物理列。
- logging/audit expectation: 记录字段名和 corpusId。

=== Exception: RAG 后端底座 - 缺少版本过滤
- scenario: search 请求没有 sourceVersion filter，Retrieval Profile 也没有固定 sourceVersion。
- expected product behavior: search 请求失败，不猜测 latest。
- user-visible message or fallback: 返回统一错误响应，说明 sourceVersion required。
- data consistency expectation: 不执行 route 查询。
- retry, recovery, or compensation behavior: 请求传版本或修复 Profile。
- logging/audit expectation: 记录 profileId 和 corpusId。

=== Exception: RAG 后端底座 - Embedding 失败
- scenario: `LlmRuntimeService` 返回模型不可用、provider 失败、超时或响应非法。
- expected product behavior: 按 Corpus/Profile degrade policy 处理；Memory 失败不可见，Project Doc 可全文降级。
- user-visible message or fallback: ingest/search 响应包含错误或 warning 摘要。
- data consistency expectation: 已成功的全文索引和 Source 状态按事务边界保留；失败 stage 记录 warning 或 error。
- retry, recovery, or compensation behavior: provider 恢复后使用 retry API 重建 vectorIndex。
- logging/audit expectation: 记录模型 code、stage 和错误类别，不记录 provider 原始响应。

=== Exception: RAG 后端底座 - Hydrate 后 Source 不可见
- scenario: route 召回候选后，hydrate 时发现 Source 不再 active。
- expected product behavior: 丢弃该候选，并在 warnings 或 trace 中记录。
- user-visible message or fallback: 若所有候选都被丢弃，返回空结果而不是过期内容。
- data consistency expectation: 不改变 Source 状态。
- retry, recovery, or compensation behavior: 调用方按需要重新检索。
- logging/audit expectation: trace 记录候选被过滤的数量。

=== Exception: RAG 后端底座 - Retry Source 不存在
- scenario: retry 请求指定的 Source 或 TextUnits 不存在。
- expected product behavior: retry 请求失败。
- user-visible message or fallback: 返回统一错误响应，说明 source not found 或 text units missing。
- data consistency expectation: 不创建新的 Source。
- retry, recovery, or compensation behavior: 先重新 ingest。
- logging/audit expectation: 记录 retry 请求摘要。

=== Exception: RAG 后端底座 - Event payload 越界
- scenario: EventBus payload 准备包含正文、embedding、provider 原始响应或 API keys。
- expected product behavior: 事件构造应拒绝或剔除越界字段。
- user-visible message or fallback: 不需要用户可见反馈，除非事件发送阻塞主流程。
- data consistency expectation: 主数据不因非关键事件失败而回滚，除非实现显式要求。
- retry, recovery, or compensation behavior: 修复事件 payload 后重放或继续后续事件。
- logging/audit expectation: 记录越界字段类别，不记录值。

== Shared Requirements
- 认证与 workspace access guard: 所有 HTTP RAG API 共用；consumers: ingest、search、debug、retry；decision: build first。
- SQLite persistence adapter: Source、Document、TextUnit、FTS、Vector、Profile、warnings 和 trace 共用；consumers: 所有 RAG 后端能力；decision: build first。
- `LlmRuntimeService`: embedding 和 rerank 共用；consumers: ingest vectorIndex、search vector route、optional rerank；decision: build first。
- EventBus: 生命周期事件共用；consumers: ingest、search、retry、stage、route；decision: build first。
- 统一响应体: HTTP API 共用；consumers: ingest、search、debug、retry；decision: build first。
- code-based identifiers: `userCode`、`agentCode`、`workspaceCode` 贯穿 RAG scope；consumers: storage、API、Profile、trace；decision: build first。

== Feature Inventory
1. Corpus 存储与 Profile 基础: module RAG 后端底座; source heading `Feature: RAG 后端底座 - Corpus 存储与 Profile 基础`; pages `Page: Backend API - RAG Debug`; rules `Rule: RAG 后端底座 - Corpus 边界`, `Rule: RAG 后端底座 - Profile 与 Registry 边界`, `Rule: RAG 后端底座 - Filter Schema`; exceptions `Exception: RAG 后端底座 - Profile 缺失或策略未注册`, `Exception: RAG 后端底座 - 非法过滤字段`; goal 建立可替换存储和配置底座; in scope SQLite/Profile/registry/table binding; out of scope Profile UI; depends on SQLite persistence adapter。
2. 同步写入 DAG: module RAG 后端底座; source heading `Feature: RAG 后端底座 - 同步写入 DAG`; pages `Page: Backend API - RAG Ingest`; rules `Rule: RAG 后端底座 - 写入 Scope`, `Rule: RAG 后端底座 - Source 版本幂等`, `Rule: RAG 后端底座 - Embedding 降级`; exceptions `Exception: RAG 后端底座 - DAG 配置非法`, `Exception: RAG 后端底座 - Embedding 失败`; goal 写入并索引 Memory 与 Project Doc; in scope source/parse/normalize/split/persist/fulltextIndex/vectorIndex; out of scope SummaryTree 和 Graph; depends on Feature 1、`LlmRuntimeService`。
3. 检索编排与 Context Pack: module RAG 后端底座; source heading `Feature: RAG 后端底座 - 检索编排与 Context Pack`; pages `Page: Backend API - RAG Search`; rules `Rule: RAG 后端底座 - 检索 Scope`, `Rule: RAG 后端底座 - Version Filter`, `Rule: RAG 后端底座 - CandidateHit 统一结构`; exceptions `Exception: RAG 后端底座 - 缺少版本过滤`, `Exception: RAG 后端底座 - Hydrate 后 Source 不可见`; goal 返回可用检索结果和 Context Pack; in scope fulltext/vector/fusion/dedup/rerank/hydrate/context; out of scope GRAPH 和 agentic retrieval; depends on Feature 1、Feature 2。
4. Memory 与 Project Doc 领域字段: module RAG 后端底座; source heading `Feature: RAG 后端底座 - Memory 与 Project Doc 领域字段`; pages `Page: Backend API - RAG Ingest`, `Page: Backend API - RAG Search`; rules `Rule: RAG 后端底座 - Memory 治理字段只存储`, `Rule: RAG 后端底座 - Filter Schema`; exceptions `Exception: RAG 后端底座 - 未声明过滤字段`; goal 支持 Memory 和 Project Doc 的真实业务过滤与 hydrate; in scope 核心物理列和 metadata 边界; out of scope Memory 状态流转; depends on Feature 1。
5. 诊断、重试、Trace 与事件: module RAG 后端底座; source heading `Feature: RAG 后端底座 - 诊断、重试、Trace 与事件`; pages `Page: Backend API - RAG Debug`, `Page: Backend API - RAG Indexing Retry`; rules `Rule: RAG 后端底座 - Warnings 持久化`, `Rule: RAG 后端底座 - Trace 隐私边界`, `Rule: RAG 后端底座 - Retry 不重跑解析`; exceptions `Exception: RAG 后端底座 - Retry Source 不存在`, `Exception: RAG 后端底座 - Event payload 越界`; goal 让 RAG 可排障、可重试、可观测; in scope warnings/debug/retry/trace/events; out of scope 管理后台; depends on Feature 1、Feature 2、Feature 3。

== Dependency Map
#mermaid(
  "flowchart LR
    SR1[\"Shared Requirement: Workspace Access Guard\"] --> F2[\"Feature: 同步写入 DAG\"]
    SR1 --> F3[\"Feature: 检索编排与 Context Pack\"]
    SR2[\"Shared Requirement: SQLite Persistence Adapter\"] --> F1[\"Feature: Corpus 存储与 Profile 基础\"]
    F1 --> F2
    F1 --> F3
    F1 --> F4[\"Feature: Memory 与 Project Doc 领域字段\"]
    SR3[\"Shared Requirement: LlmRuntimeService\"] --> F2
    SR3 --> F3
    F2 --> F3
    F2 --> F5[\"Feature: 诊断、重试、Trace 与事件\"]
    F3 --> F5"
)

== Recommended Implementation Order
1. Corpus 存储与 Profile 基础 - 后续写入、检索和诊断都依赖表、Profile、filterSchema 和 adapter 边界。
2. Memory 与 Project Doc 领域字段 - 字段边界必须先稳定，避免后续 ingest/search 反复改 schema。
3. 同步写入 DAG - 先跑通 Source、TextUnit、FTS 和 Vector 写入，形成可检索数据。
4. 检索编排与 Context Pack - 基于已持久化数据实现 fulltext/vector fan-out、hydrate 和 context assembly。
5. 诊断、重试、Trace 与事件 - 在核心链路可跑通后补齐运维能力和排障闭环。

== Acceptance
=== Acceptance: RAG 后端底座 - 第一阶段范围
- 系统只对 `memory-card` 和 `project-doc` 启用真实写入与检索。
- `code-graph`、`GRAPH` retriever、SummaryTree 和 `GraphService` 集成保持不可执行扩展位。
- 文档、Profile 和 API 行为都不能暗示第一阶段支持代码图谱。

=== Acceptance: RAG 后端底座 - 写入链路
- 调用方可以通过 ingest API 写入 Memory card 或 Project Doc。
- 写入请求按同步 DAG 执行，并产生 Source、TextUnit、全文索引、向量索引和 warnings。
- 同一 sourceCode/sourceVersion 重试写入保持幂等。

=== Acceptance: RAG 后端底座 - 检索链路
- 调用方可以通过 search API 使用固定 Retrieval Profile 检索 Memory 或 Project Doc。
- 检索结果按 scope、filterSchema 和 sourceVersion 约束返回。
- 返回内容支持 `RESULT_LIST`、`TOKEN_BUDGET_PACK`、`MEMORY_CONTEXT_PACK` 和 `DOC_QA_CONTEXT_PACK`。

=== Acceptance: RAG 后端底座 - Embedding 与 Rerank
- RAG embedding 和可选 rerank 都复用 `LlmRuntimeService`。
- Project Doc 向量失败可降级为全文可用并持久化 warning。
- Memory card embedding 失败不会进入普通可见检索结果。

=== Acceptance: RAG 后端底座 - Scope 与访问边界
- 所有 HTTP RAG API 都校验 workspace access。
- RAG 存储层使用 `scope_type` 和 `scope_code` 隔离 USER、AGENT、SPACE 数据。
- RAG 不执行 corpus、source、repo、role、field 或内容脱敏权限。

=== Acceptance: RAG 后端底座 - 诊断与安全边界
- debug API 只读返回 Profile、Source、TextUnits 和 warnings。
- retry API 只重试索引阶段，不重跑 parse、normalize 或 split。
- search trace 和 EventBus payload 不保存正文、Context Pack body、embedding、provider 原始响应或 API keys。

== Risks And Open Questions
- 第一阶段不接 `GraphService`，后续做 `code-graph` 时需要单独 PRD/AC 衔接 Graph scope 与 RAG scope。
- SQLite brute-force vector 适合 MVP，但数据量增长后需要替换为 pgvector、Qdrant、Milvus 或其他向量存储。
- Profile 固定 sourceVersion 简化了第一阶段版本语义，但调用方必须显式管理版本选择。
- Memory 治理字段只存储不流转，需要 Memory 模块在后续接入时保证字段语义稳定。
