# Agent 协作

Tron 使用 Framework 官方 `agent_spawn` 和 `thread_*` 工具，不另建工具协议。

```mermaid
flowchart LR
    D[工作空间 Agent 目录] --> C[AgentDirectoryContributor]
    C --> M[主 Agent]
    M --> T[官方协作工具 agentCode]
    T --> R[Tron AgentCollaborationRuntime]
    R --> A[授权校验与目标配置加载]
    A --> F[Framework 子运行或独立 Thread]
```

目录每次组装模型上下文时读取当前工作空间已授权的内部 Agent，提供编码、名称、说明和最多 512 字符的职责摘要。目录属于 Data 上下文，不作为系统指令；独立预算为 2048 tokens，放不下的条目整条省略，并提示省略数量。

派发时重新检查工作空间授权。目标 Agent 使用自己的角色、业务工具和 MCP 权限；模型沿用当前调用配置；Person-AI 请求会重新绑定目标会话与运行标识，避免沿用父会话。未知或未授权编码明确报错，不回退到主 Agent。省略 `agentCode` 时使用当前 Agent。

- `agent_spawn` 等待子运行结果。Framework 保留父运行的取消、截止时间、调用关系和预算约束。
- `thread_create` 启动独立会话并立即返回。Thread 固定绑定目标 Agent、用户、工作空间和执行目录，后续消息重新加载目标角色与工具权限。
- `thread_send`、`thread_answer`、`thread_wait`、`thread_read`、`thread_list`、`thread_close` 复用官方工具；已撤销授权的目标不能继续操作，其他用户或工作空间不能读取 Thread。
- MCP 工具租约由原生运行上下文持有，直到运行释放；取消子代理等待不会提前释放仍在使用的工具。

Thread 调度索引和结果缓存当前保存在进程内，最多 128 个 Thread。原生会话记录使用 Framework 共享存储；后端重启后不恢复协作 Thread 索引。关闭完成的 Thread 只移除协作索引，不删除会话历史。配置档关闭时由 Framework 关闭运行和共享服务。

本接入范围是工作空间内部 Agent，系统 CLI Agent 不在目录内。

验证：`make be-check`、`make be-test TEST='--test agent_collaboration'`。

## 本次验证记录

协作集成测试覆盖命名派发、目标工具权限、目录读取后的撤权、父运行取消、Thread 目标绑定及用户隔离、Person-AI 子会话隔离。另运行知识库上下文和工作空间授权回归。

现有 `agent_runtime` 套件为 27/31 通过。使用未修改的分支 HEAD 和同一份 Framework 依赖，在独立源码目录复跑，以下四项出现相同失败，本次未改动这些行为或断言：

- `react_agent_completes_final_answer_and_publishes_metadata_only_events`
- `react_agent_continues_after_unauthorized_parallel_tool_call`
- `react_agent_executes_remaining_parallel_budget_before_failing`
- `framework_main_rich_input_and_remaining_output_budget_reach_actual_http_body`
