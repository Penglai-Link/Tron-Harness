&[
    (
        "agent.toolCatalog.common.params.current",
        "页码，从 1 开始。",
    ),
    (
        "agent.toolCatalog.common.params.size",
        "每页数量，最大 100。",
    ),
    (
        "agent.toolCatalog.common.params.workspaceCode",
        "可选工作区编码；缺省时使用当前工作区。",
    ),
    ("agent.toolCatalog.common.params.name", "显示名称。"),
    ("agent.toolCatalog.common.params.description", "可选说明。"),
    ("agent.toolCatalog.common.params.tags", "可选标签。"),
    (
        "agent.toolCatalog.common.params.includeArchived",
        "是否包含已归档记录，默认 false。",
    ),
    ("agent.toolCatalog.common.params.enabled", "记录是否启用。"),
    (
        "agent.toolCatalog.tools.workspace.page.name",
        "分页查询工作区",
    ),
    (
        "agent.toolCatalog.tools.workspace.page.description",
        "分页查询当前操作者可见的工作区。",
    ),
    (
        "agent.toolCatalog.tools.workspace.page.sampleResultSummary",
        "返回分页后的工作区树节点。",
    ),
    (
        "agent.toolCatalog.tools.workspace.page.params.includeArchived",
        "是否包含已归档工作区，默认 false。",
    ),
    (
        "agent.toolCatalog.tools.workspace.page.params.search",
        "可选的搜索文本，用于匹配工作区名称和元数据。",
    ),
    (
        "agent.toolCatalog.tools.workspace.page.params.rootOnly",
        "是否只返回根工作区，默认 true。",
    ),
    ("agent.toolCatalog.tools.app.page.name", "分页查询应用"),
    (
        "agent.toolCatalog.tools.app.page.description",
        "分页查询当前或指定工作区中的应用。",
    ),
    (
        "agent.toolCatalog.tools.app.page.sampleResultSummary",
        "返回分页后的应用记录。",
    ),
    (
        "agent.toolCatalog.tools.app.page.params.search",
        "可选的应用搜索文本。",
    ),
    (
        "agent.toolCatalog.tools.app.page.params.includeArchived",
        "是否包含已归档应用，默认 false。",
    ),
    (
        "agent.toolCatalog.tools.app.common.params.appCode",
        "应用编码。",
    ),
    (
        "agent.toolCatalog.tools.app.common.params.localDirectory",
        "应用本地目录。",
    ),
    (
        "agent.toolCatalog.tools.app.common.params.runtimeRole",
        "应用运行角色，可选值：frontend、backend、mono。",
    ),
    (
        "agent.toolCatalog.tools.app.common.params.repositoryUrl",
        "可选仓库 URL。",
    ),
    (
        "agent.toolCatalog.tools.app.common.params.defaultBranch",
        "可选默认分支。",
    ),
    ("agent.toolCatalog.tools.app.create.name", "创建应用"),
    (
        "agent.toolCatalog.tools.app.create.description",
        "在当前或指定工作区中创建应用。",
    ),
    (
        "agent.toolCatalog.tools.app.create.sampleResultSummary",
        "返回新建应用详情。",
    ),
    ("agent.toolCatalog.tools.app.update.name", "修改应用"),
    (
        "agent.toolCatalog.tools.app.update.description",
        "修改应用元数据。",
    ),
    (
        "agent.toolCatalog.tools.app.update.sampleResultSummary",
        "返回修改后的应用详情。",
    ),
    ("agent.toolCatalog.tools.app.delete.name", "删除应用"),
    (
        "agent.toolCatalog.tools.app.delete.description",
        "通过归档软删除应用。",
    ),
    (
        "agent.toolCatalog.tools.app.delete.sampleResultSummary",
        "返回应用归档结果。",
    ),
    ("agent.toolCatalog.tools.app.restore.name", "恢复应用"),
    (
        "agent.toolCatalog.tools.app.restore.description",
        "恢复已归档应用。",
    ),
    (
        "agent.toolCatalog.tools.app.restore.sampleResultSummary",
        "返回应用恢复结果。",
    ),
    ("agent.toolCatalog.tools.task.page.name", "分页查询任务"),
    (
        "agent.toolCatalog.tools.task.page.description",
        "分页查询当前或指定工作区中的任务。",
    ),
    (
        "agent.toolCatalog.tools.task.page.sampleResultSummary",
        "返回分页后的任务记录。",
    ),
    (
        "agent.toolCatalog.tools.task.page.params.search",
        "可选的任务搜索文本。",
    ),
    (
        "agent.toolCatalog.tools.task.page.params.status",
        "可选的任务状态筛选值。",
    ),
    (
        "agent.toolCatalog.tools.task.page.params.assigneeUserCode",
        "可选的负责人用户编码筛选。",
    ),
    (
        "agent.toolCatalog.tools.task.common.params.taskCode",
        "任务编码。",
    ),
    (
        "agent.toolCatalog.tools.task.common.params.parentTaskCode",
        "可选父任务编码。",
    ),
    (
        "agent.toolCatalog.tools.task.common.params.title",
        "任务标题。",
    ),
    (
        "agent.toolCatalog.tools.task.common.params.priority",
        "任务优先级，可选值：low、medium、high、extra_high。",
    ),
    (
        "agent.toolCatalog.tools.task.common.params.taskType",
        "任务类型，可选值：feature、bug。",
    ),
    (
        "agent.toolCatalog.tools.task.common.params.sourceType",
        "来源类型，可选值：internal、github、gitlab。",
    ),
    (
        "agent.toolCatalog.tools.task.common.params.sourceUrl",
        "可选来源 URL。",
    ),
    (
        "agent.toolCatalog.tools.task.common.params.assigneeUserCodes",
        "可选负责人用户编码。",
    ),
    (
        "agent.toolCatalog.tools.task.common.params.agentCodes",
        "可选关联 Agent 编码。",
    ),
    (
        "agent.toolCatalog.tools.task.common.params.appCodes",
        "可选关联应用编码。",
    ),
    (
        "agent.toolCatalog.tools.task.common.params.appCode",
        "可选关联应用编码。",
    ),
    (
        "agent.toolCatalog.tools.task.common.params.progress",
        "可选进度，范围 0 到 100。",
    ),
    ("agent.toolCatalog.tools.task.update.name", "修改任务"),
    (
        "agent.toolCatalog.tools.task.update.description",
        "修改任务元数据，并可选择更新状态。",
    ),
    (
        "agent.toolCatalog.tools.task.update.sampleResultSummary",
        "返回修改后的任务详情。",
    ),
    ("agent.toolCatalog.tools.task.delete.name", "删除任务"),
    (
        "agent.toolCatalog.tools.task.delete.description",
        "通过归档软删除任务。",
    ),
    (
        "agent.toolCatalog.tools.task.delete.sampleResultSummary",
        "返回任务归档结果。",
    ),
    ("agent.toolCatalog.tools.task.restore.name", "恢复任务"),
    (
        "agent.toolCatalog.tools.task.restore.description",
        "恢复已归档任务。",
    ),
    (
        "agent.toolCatalog.tools.task.restore.sampleResultSummary",
        "返回任务恢复结果。",
    ),
    ("agent.toolCatalog.tools.model.page.name", "分页查询模型"),
    (
        "agent.toolCatalog.tools.model.page.description",
        "分页查询当前或指定工作区中的生效模型定义。",
    ),
    (
        "agent.toolCatalog.tools.model.page.sampleResultSummary",
        "返回分页后的模型记录。",
    ),
    (
        "agent.toolCatalog.tools.model.page.params.search",
        "可选的模型搜索文本。",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.modelCode",
        "模型编码。",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.displayName",
        "模型显示名称。",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.vendor",
        "模型供应商。",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.protocol",
        "模型协议，可选值：openai_compatible、responses、rerank。",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.modelType",
        "模型类型，可选值：text、embedding、rerank、image。",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.modelCapabilities",
        "可选模型能力；未传时按 modelType 使用默认能力。",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.baseUrl",
        "供应商基础 URL。",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.apiPath",
        "可选 API 路径。",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.apiKeyRef",
        "保存 API Key 的变量键。",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.modelName",
        "上游供应商模型名称。",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.thinkingEnabled",
        "是否启用 thinking 模式。",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.supportedReasoningEfforts",
        "当前具体模型支持的智力程度取值。",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.defaultReasoningEffort",
        "默认智力程度，必须包含在 supportedReasoningEfforts 中。",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.structuredOutputEnabled",
        "是否启用结构化输出。",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.responsesStatefulEnabled",
        "是否启用 Responses 有状态模式。",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.temperature",
        "可选 temperature。",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.maxTokens",
        "可选最大输出 token 数。",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.contextWindowTokens",
        "可选上下文窗口 token 数。",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.topP",
        "可选 top-p 值。",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.maxRetries",
        "可选最大重试次数。",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.allowInheritance",
        "子工作区是否可继承该模型。",
    ),
    ("agent.toolCatalog.tools.model.create.name", "创建模型"),
    (
        "agent.toolCatalog.tools.model.create.description",
        "创建工作区本地模型定义。",
    ),
    (
        "agent.toolCatalog.tools.model.create.sampleResultSummary",
        "返回新建模型。",
    ),
    ("agent.toolCatalog.tools.model.update.name", "修改模型"),
    (
        "agent.toolCatalog.tools.model.update.description",
        "修改工作区模型定义。",
    ),
    (
        "agent.toolCatalog.tools.model.update.sampleResultSummary",
        "返回修改后的模型。",
    ),
    ("agent.toolCatalog.tools.model.enable.name", "启用模型"),
    (
        "agent.toolCatalog.tools.model.enable.description",
        "启用工作区模型。",
    ),
    (
        "agent.toolCatalog.tools.model.enable.sampleResultSummary",
        "返回启用后的模型。",
    ),
    ("agent.toolCatalog.tools.model.disable.name", "停用模型"),
    (
        "agent.toolCatalog.tools.model.disable.description",
        "停用工作区模型。",
    ),
    (
        "agent.toolCatalog.tools.model.disable.sampleResultSummary",
        "返回停用后的模型。",
    ),
    ("agent.toolCatalog.tools.model.delete.name", "删除模型"),
    (
        "agent.toolCatalog.tools.model.delete.description",
        "删除工作区模型或覆盖项。",
    ),
]
