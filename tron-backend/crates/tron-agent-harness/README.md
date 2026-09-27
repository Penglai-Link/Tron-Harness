# tron-agent-harness

Tron 自有的 native 能力包，负责连接宿主上下文与业务工具。通用执行器使用 Agent Framework 插件。
当前已完成能力包装配边界，尚未连接生产后端运行循环，完整进度见 [接入文档](../../../docs/agent-framework-ui-integration.md)。

宿主通过 `TronHarness::new(context_provider, tool_provider, shared_tool_catalog)` 创建能力包，
将 `.capability_pack()` 交给 `AgentBuilder`。每个 `RunRequest` 必须由受信任的宿主代码附加 `TronRunScope`。
提供者依旧负责当前权限、数据查询、fileRef 与业务资源归属；不能信任模型输入自行声明用户或工作空间。

能力包保留 context 的有序消息与内容类型，以及工具的 hidden/parallel_safe 标记和资源清理。
工具绑定不得跨运行作用域复用。所有本地公共工具默认保留名称；其他公共工具需传入其真实描述进行冲突检查。
目录或命令权限不会因为描述出现在 catalog 中而被授予。

本 crate 使用独立 workspace 进行装配测试，避免把测试专用 SQLite 版本引入 Tron 生产数据库。
根目录运行 `make agent-deps` 后，用 `make agent-harness-check` / `make agent-harness-test` 检查。
