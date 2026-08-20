pub(super) const EN_US: &[(&str, &str)] = &[
    (
        "agent.toolCatalog.common.params.current",
        "Page number, starting from 1.",
    ),
    (
        "agent.toolCatalog.common.params.size",
        "Page size. Maximum 100.",
    ),
    (
        "agent.toolCatalog.common.params.workspaceCode",
        "Optional workspace code. Uses the current workspace when omitted.",
    ),
    ("agent.toolCatalog.common.params.name", "Display name."),
    (
        "agent.toolCatalog.common.params.description",
        "Optional description.",
    ),
    ("agent.toolCatalog.common.params.tags", "Optional tags."),
    (
        "agent.toolCatalog.common.params.includeArchived",
        "Whether archived records should be included. Defaults to false.",
    ),
    (
        "agent.toolCatalog.common.params.enabled",
        "Whether the record is enabled.",
    ),
    (
        "agent.toolCatalog.tools.workspace.page.name",
        "Page workspaces",
    ),
    (
        "agent.toolCatalog.tools.workspace.page.description",
        "Page workspaces visible to the current actor.",
    ),
    (
        "agent.toolCatalog.tools.workspace.page.sampleResultSummary",
        "Returns a paged set of workspace tree nodes.",
    ),
    (
        "agent.toolCatalog.tools.workspace.page.params.includeArchived",
        "Whether archived workspaces should be included. Defaults to false.",
    ),
    (
        "agent.toolCatalog.tools.workspace.page.params.search",
        "Optional search text matched against workspace labels and metadata.",
    ),
    (
        "agent.toolCatalog.tools.workspace.page.params.rootOnly",
        "Whether only root workspaces should be returned. Defaults to true.",
    ),
    ("agent.toolCatalog.tools.app.page.name", "Page apps"),
    (
        "agent.toolCatalog.tools.app.page.description",
        "Page apps in the current or specified workspace.",
    ),
    (
        "agent.toolCatalog.tools.app.page.sampleResultSummary",
        "Returns paged app records.",
    ),
    (
        "agent.toolCatalog.tools.app.page.params.search",
        "Optional app search text.",
    ),
    (
        "agent.toolCatalog.tools.app.page.params.includeArchived",
        "Whether archived apps should be included. Defaults to false.",
    ),
    (
        "agent.toolCatalog.tools.app.common.params.appCode",
        "App code.",
    ),
    (
        "agent.toolCatalog.tools.app.common.params.localDirectory",
        "Local app directory.",
    ),
    (
        "agent.toolCatalog.tools.app.common.params.runtimeRole",
        "App runtime role. Allowed values: frontend, backend, mono.",
    ),
    (
        "agent.toolCatalog.tools.app.common.params.repositoryUrl",
        "Optional repository URL.",
    ),
    (
        "agent.toolCatalog.tools.app.common.params.defaultBranch",
        "Optional default branch.",
    ),
    ("agent.toolCatalog.tools.app.create.name", "Create app"),
    (
        "agent.toolCatalog.tools.app.create.description",
        "Create an app in the current or specified workspace.",
    ),
    (
        "agent.toolCatalog.tools.app.create.sampleResultSummary",
        "Returns the created app detail.",
    ),
    ("agent.toolCatalog.tools.app.update.name", "Update app"),
    (
        "agent.toolCatalog.tools.app.update.description",
        "Update app metadata.",
    ),
    (
        "agent.toolCatalog.tools.app.update.sampleResultSummary",
        "Returns the updated app detail.",
    ),
    ("agent.toolCatalog.tools.app.delete.name", "Delete app"),
    (
        "agent.toolCatalog.tools.app.delete.description",
        "Soft-delete an app by archiving it.",
    ),
    (
        "agent.toolCatalog.tools.app.delete.sampleResultSummary",
        "Returns the app archive result.",
    ),
    ("agent.toolCatalog.tools.app.restore.name", "Restore app"),
    (
        "agent.toolCatalog.tools.app.restore.description",
        "Restore an archived app.",
    ),
    (
        "agent.toolCatalog.tools.app.restore.sampleResultSummary",
        "Returns the app restore result.",
    ),
    ("agent.toolCatalog.tools.task.page.name", "Page tasks"),
    (
        "agent.toolCatalog.tools.task.page.description",
        "Page tasks in the current or specified workspace.",
    ),
    (
        "agent.toolCatalog.tools.task.page.sampleResultSummary",
        "Returns paged task records.",
    ),
    (
        "agent.toolCatalog.tools.task.page.params.search",
        "Optional task search text.",
    ),
    (
        "agent.toolCatalog.tools.task.page.params.status",
        "Optional task status filter.",
    ),
    (
        "agent.toolCatalog.tools.task.page.params.assigneeUserCode",
        "Optional assignee user code filter.",
    ),
    (
        "agent.toolCatalog.tools.task.common.params.taskCode",
        "Task code.",
    ),
    (
        "agent.toolCatalog.tools.task.common.params.parentTaskCode",
        "Optional parent task code.",
    ),
    (
        "agent.toolCatalog.tools.task.common.params.title",
        "Task title.",
    ),
    (
        "agent.toolCatalog.tools.task.common.params.priority",
        "Task priority. Allowed values: low, medium, high, extra_high.",
    ),
    (
        "agent.toolCatalog.tools.task.common.params.taskType",
        "Task type. Allowed values: feature, bug.",
    ),
    (
        "agent.toolCatalog.tools.task.common.params.sourceType",
        "Source type. Allowed values: internal, github, gitlab.",
    ),
    (
        "agent.toolCatalog.tools.task.common.params.sourceUrl",
        "Optional source URL.",
    ),
    (
        "agent.toolCatalog.tools.task.common.params.assigneeUserCodes",
        "Optional assignee user codes.",
    ),
    (
        "agent.toolCatalog.tools.task.common.params.agentCodes",
        "Optional linked agent codes.",
    ),
    (
        "agent.toolCatalog.tools.task.common.params.appCodes",
        "Optional linked app codes.",
    ),
    (
        "agent.toolCatalog.tools.task.common.params.appCode",
        "Optional linked app code.",
    ),
    (
        "agent.toolCatalog.tools.task.common.params.progress",
        "Optional progress from 0 to 100.",
    ),
    ("agent.toolCatalog.tools.task.update.name", "Update task"),
    (
        "agent.toolCatalog.tools.task.update.description",
        "Update task metadata and optionally update status.",
    ),
    (
        "agent.toolCatalog.tools.task.update.sampleResultSummary",
        "Returns the updated task detail.",
    ),
    ("agent.toolCatalog.tools.task.delete.name", "Delete task"),
    (
        "agent.toolCatalog.tools.task.delete.description",
        "Soft-delete a task by archiving it.",
    ),
    (
        "agent.toolCatalog.tools.task.delete.sampleResultSummary",
        "Returns the task archive result.",
    ),
    ("agent.toolCatalog.tools.task.restore.name", "Restore task"),
    (
        "agent.toolCatalog.tools.task.restore.description",
        "Restore an archived task.",
    ),
    (
        "agent.toolCatalog.tools.task.restore.sampleResultSummary",
        "Returns the task restore result.",
    ),
    ("agent.toolCatalog.tools.model.page.name", "Page models"),
    (
        "agent.toolCatalog.tools.model.page.description",
        "Page effective model definitions for the current or specified workspace.",
    ),
    (
        "agent.toolCatalog.tools.model.page.sampleResultSummary",
        "Returns paged model records.",
    ),
    (
        "agent.toolCatalog.tools.model.page.params.search",
        "Optional model search text.",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.modelCode",
        "Model code.",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.displayName",
        "Model display name.",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.vendor",
        "Model vendor.",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.protocol",
        "Model protocol. Allowed values: openai_compatible, responses, rerank.",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.modelType",
        "Model type. Allowed values: text, embedding, rerank, image.",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.modelCapabilities",
        "Optional model capabilities. Defaults from modelType when omitted.",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.baseUrl",
        "Provider base URL.",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.apiPath",
        "Optional API path.",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.apiKeyRef",
        "Variable key that stores the API key.",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.modelName",
        "Upstream provider model name.",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.thinkingEnabled",
        "Whether thinking mode is enabled.",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.supportedReasoningEfforts",
        "Reasoning effort values supported by this specific model.",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.defaultReasoningEffort",
        "Default reasoning effort; it must be included in supportedReasoningEfforts.",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.structuredOutputEnabled",
        "Whether structured output is enabled.",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.responsesStatefulEnabled",
        "Whether Responses stateful mode is enabled.",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.temperature",
        "Optional temperature.",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.maxTokens",
        "Optional max output tokens.",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.contextWindowTokens",
        "Optional context window tokens.",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.topP",
        "Optional top-p value.",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.maxRetries",
        "Optional max retry count.",
    ),
    (
        "agent.toolCatalog.tools.model.common.params.allowInheritance",
        "Whether child workspaces may inherit this model.",
    ),
    ("agent.toolCatalog.tools.model.create.name", "Create model"),
    (
        "agent.toolCatalog.tools.model.create.description",
        "Create a workspace-local model definition.",
    ),
    (
        "agent.toolCatalog.tools.model.create.sampleResultSummary",
        "Returns the created model.",
    ),
    ("agent.toolCatalog.tools.model.update.name", "Update model"),
    (
        "agent.toolCatalog.tools.model.update.description",
        "Update a workspace model definition.",
    ),
    (
        "agent.toolCatalog.tools.model.update.sampleResultSummary",
        "Returns the updated model.",
    ),
    ("agent.toolCatalog.tools.model.enable.name", "Enable model"),
    (
        "agent.toolCatalog.tools.model.enable.description",
        "Enable a workspace model.",
    ),
    (
        "agent.toolCatalog.tools.model.enable.sampleResultSummary",
        "Returns the enabled model.",
    ),
    (
        "agent.toolCatalog.tools.model.disable.name",
        "Disable model",
    ),
    (
        "agent.toolCatalog.tools.model.disable.description",
        "Disable a workspace model.",
    ),
    (
        "agent.toolCatalog.tools.model.disable.sampleResultSummary",
        "Returns the disabled model.",
    ),
    ("agent.toolCatalog.tools.model.delete.name", "Delete model"),
    (
        "agent.toolCatalog.tools.model.delete.description",
        "Delete a workspace model or override.",
    ),
    (
        "agent.toolCatalog.tools.model.delete.sampleResultSummary",
        "Returns the model delete result.",
    ),
    ("agent.toolCatalog.tools.skill.page.name", "Page skills"),
    (
        "agent.toolCatalog.tools.skill.page.description",
        "Page workspace skill bindings and registry entries.",
    ),
    (
        "agent.toolCatalog.tools.skill.page.sampleResultSummary",
        "Returns paged workspace skill records.",
    ),
    (
        "agent.toolCatalog.tools.skill.page.params.search",
        "Optional skill search text.",
    ),
    (
        "agent.toolCatalog.tools.skill.page.params.includeDisabled",
        "Whether disabled skills should be included. Defaults to false.",
    ),
    (
        "agent.toolCatalog.tools.variable.page.name",
        "Page variables",
    ),
    (
        "agent.toolCatalog.tools.variable.page.description",
        "Page effective variables in the current or specified workspace.",
    ),
    (
        "agent.toolCatalog.tools.variable.page.sampleResultSummary",
        "Returns paged effective variables.",
    ),
    (
        "agent.toolCatalog.tools.variable.page.params.search",
        "Optional variable search text.",
    ),
    (
        "agent.toolCatalog.tools.variable.common.params.key",
        "Variable key.",
    ),
    (
        "agent.toolCatalog.tools.variable.common.params.value",
        "Variable value.",
    ),
    (
        "agent.toolCatalog.tools.variable.common.params.isSensitive",
        "Whether the value is sensitive.",
    ),
    (
        "agent.toolCatalog.tools.variable.common.params.valueMode",
        "replace writes a new value; preserve keeps the current value.",
    ),
    (
        "agent.toolCatalog.tools.variable.create.name",
        "Create variable",
    ),
    (
        "agent.toolCatalog.tools.variable.create.description",
        "Create a workspace-local variable.",
    ),
    (
        "agent.toolCatalog.tools.variable.create.sampleResultSummary",
        "Returns the created variable.",
    ),
    (
        "agent.toolCatalog.tools.variable.update.name",
        "Update variable",
    ),
    (
        "agent.toolCatalog.tools.variable.update.description",
        "Update a workspace-local variable.",
    ),
    (
        "agent.toolCatalog.tools.variable.update.sampleResultSummary",
        "Returns the updated variable.",
    ),
    (
        "agent.toolCatalog.tools.variable.delete.name",
        "Delete variable",
    ),
    (
        "agent.toolCatalog.tools.variable.delete.description",
        "Delete a workspace-local variable.",
    ),
    (
        "agent.toolCatalog.tools.variable.delete.sampleResultSummary",
        "Returns the variable delete result.",
    ),
    (
        "agent.toolCatalog.tools.platform.variable.page.name",
        "Page platform variables",
    ),
    (
        "agent.toolCatalog.tools.platform.variable.page.description",
        "Page variables configured in the platform settings scope.",
    ),
    (
        "agent.toolCatalog.tools.platform.variable.page.sampleResultSummary",
        "Returns paged platform variable records with sensitive values masked.",
    ),
    (
        "agent.toolCatalog.tools.platform.variable.page.params.search",
        "Optional platform variable key or description search text.",
    ),
    (
        "agent.toolCatalog.tools.platform.variable.create.name",
        "Create platform variable",
    ),
    (
        "agent.toolCatalog.tools.platform.variable.create.description",
        "Create a variable in the platform settings scope.",
    ),
    (
        "agent.toolCatalog.tools.platform.variable.create.sampleResultSummary",
        "Returns the created platform variable with sensitive values masked.",
    ),
    (
        "agent.toolCatalog.tools.platform.variable.update.name",
        "Update platform variable",
    ),
    (
        "agent.toolCatalog.tools.platform.variable.update.description",
        "Update a variable in the platform settings scope.",
    ),
    (
        "agent.toolCatalog.tools.platform.variable.update.sampleResultSummary",
        "Returns the updated platform variable with sensitive values masked.",
    ),
    (
        "agent.toolCatalog.tools.platform.variable.delete.name",
        "Delete platform variable",
    ),
    (
        "agent.toolCatalog.tools.platform.variable.delete.description",
        "Delete a variable from the platform settings scope.",
    ),
    (
        "agent.toolCatalog.tools.platform.variable.delete.sampleResultSummary",
        "Returns the platform variable delete result.",
    ),
    (
        "agent.toolCatalog.tools.platform.model.page.name",
        "Page platform models",
    ),
    (
        "agent.toolCatalog.tools.platform.model.page.description",
        "Page models configured in platform settings.",
    ),
    (
        "agent.toolCatalog.tools.platform.model.page.sampleResultSummary",
        "Returns paged platform models.",
    ),
    (
        "agent.toolCatalog.tools.platform.model.get.name",
        "Get platform model",
    ),
    (
        "agent.toolCatalog.tools.platform.model.get.description",
        "Read one platform model configuration.",
    ),
    (
        "agent.toolCatalog.tools.platform.model.get.sampleResultSummary",
        "Returns one platform model.",
    ),
    (
        "agent.toolCatalog.tools.platform.model.create.name",
        "Create platform model",
    ),
    (
        "agent.toolCatalog.tools.platform.model.create.description",
        "Create a model in platform settings.",
    ),
    (
        "agent.toolCatalog.tools.platform.model.create.sampleResultSummary",
        "Returns the created platform model.",
    ),
    (
        "agent.toolCatalog.tools.platform.model.update.name",
        "Update platform model",
    ),
    (
        "agent.toolCatalog.tools.platform.model.update.description",
        "Update a platform model configuration.",
    ),
    (
        "agent.toolCatalog.tools.platform.model.update.sampleResultSummary",
        "Returns the updated platform model.",
    ),
    (
        "agent.toolCatalog.tools.platform.model.enable.name",
        "Enable platform model",
    ),
    (
        "agent.toolCatalog.tools.platform.model.enable.description",
        "Enable a platform model.",
    ),
    (
        "agent.toolCatalog.tools.platform.model.enable.sampleResultSummary",
        "Returns the enabled platform model.",
    ),
    (
        "agent.toolCatalog.tools.platform.model.disable.name",
        "Disable platform model",
    ),
    (
        "agent.toolCatalog.tools.platform.model.disable.description",
        "Disable a platform model.",
    ),
    (
        "agent.toolCatalog.tools.platform.model.disable.sampleResultSummary",
        "Returns the disabled platform model.",
    ),
    (
        "agent.toolCatalog.tools.platform.model.delete.name",
        "Delete platform model",
    ),
    (
        "agent.toolCatalog.tools.platform.model.delete.description",
        "Delete a platform model.",
    ),
    (
        "agent.toolCatalog.tools.platform.model.delete.sampleResultSummary",
        "Returns the platform model delete result.",
    ),
    (
        "agent.toolCatalog.tools.platform.prompt.page.name",
        "Page platform prompts",
    ),
    (
        "agent.toolCatalog.tools.platform.prompt.page.description",
        "Page prompts configured in platform settings.",
    ),
    (
        "agent.toolCatalog.tools.platform.prompt.page.sampleResultSummary",
        "Returns paged platform prompts.",
    ),
    (
        "agent.toolCatalog.tools.platform.prompt.get.name",
        "Get platform prompt",
    ),
    (
        "agent.toolCatalog.tools.platform.prompt.get.description",
        "Read one platform prompt.",
    ),
    (
        "agent.toolCatalog.tools.platform.prompt.get.sampleResultSummary",
        "Returns one platform prompt.",
    ),
    (
        "agent.toolCatalog.tools.platform.prompt.create.name",
        "Create platform prompt",
    ),
    (
        "agent.toolCatalog.tools.platform.prompt.create.description",
        "Create a prompt in platform settings.",
    ),
    (
        "agent.toolCatalog.tools.platform.prompt.create.sampleResultSummary",
        "Returns the created platform prompt.",
    ),
    (
        "agent.toolCatalog.tools.platform.prompt.update.name",
        "Update platform prompt",
    ),
    (
        "agent.toolCatalog.tools.platform.prompt.update.description",
        "Update a platform prompt.",
    ),
    (
        "agent.toolCatalog.tools.platform.prompt.update.sampleResultSummary",
        "Returns the updated platform prompt.",
    ),
    (
        "agent.toolCatalog.tools.platform.prompt.delete.name",
        "Delete platform prompt",
    ),
    (
        "agent.toolCatalog.tools.platform.prompt.delete.description",
        "Delete a platform prompt.",
    ),
    (
        "agent.toolCatalog.tools.platform.prompt.delete.sampleResultSummary",
        "Returns the platform prompt delete result.",
    ),
    (
        "agent.toolCatalog.tools.platform.mcp.page.name",
        "Page platform MCP servers",
    ),
    (
        "agent.toolCatalog.tools.platform.mcp.page.description",
        "Page MCP servers configured in platform settings.",
    ),
    (
        "agent.toolCatalog.tools.platform.mcp.page.sampleResultSummary",
        "Returns paged platform MCP servers.",
    ),
    (
        "agent.toolCatalog.tools.platform.mcp.get.name",
        "Get platform MCP server",
    ),
    (
        "agent.toolCatalog.tools.platform.mcp.get.description",
        "Read one platform MCP server.",
    ),
    (
        "agent.toolCatalog.tools.platform.mcp.get.sampleResultSummary",
        "Returns one platform MCP server.",
    ),
    (
        "agent.toolCatalog.tools.platform.mcp.create.name",
        "Create platform MCP server",
    ),
    (
        "agent.toolCatalog.tools.platform.mcp.create.description",
        "Create an MCP server in platform settings.",
    ),
    (
        "agent.toolCatalog.tools.platform.mcp.create.sampleResultSummary",
        "Returns the created platform MCP server.",
    ),
    (
        "agent.toolCatalog.tools.platform.mcp.update.name",
        "Update platform MCP server",
    ),
    (
        "agent.toolCatalog.tools.platform.mcp.update.description",
        "Update a platform MCP server.",
    ),
    (
        "agent.toolCatalog.tools.platform.mcp.update.sampleResultSummary",
        "Returns the updated platform MCP server.",
    ),
    (
        "agent.toolCatalog.tools.platform.mcp.delete.name",
        "Delete platform MCP server",
    ),
    (
        "agent.toolCatalog.tools.platform.mcp.delete.description",
        "Delete a platform MCP server.",
    ),
    (
        "agent.toolCatalog.tools.platform.mcp.delete.sampleResultSummary",
        "Returns the platform MCP delete result.",
    ),
    (
        "agent.toolCatalog.tools.platform.open_app.page.name",
        "Page platform OpenAPI apps",
    ),
    (
        "agent.toolCatalog.tools.platform.open_app.page.description",
        "Page OpenAPI apps configured in platform settings.",
    ),
    (
        "agent.toolCatalog.tools.platform.open_app.page.sampleResultSummary",
        "Returns paged platform OpenAPI apps.",
    ),
    (
        "agent.toolCatalog.tools.platform.open_app.get.name",
        "Get platform OpenAPI app",
    ),
    (
        "agent.toolCatalog.tools.platform.open_app.get.description",
        "Read one platform OpenAPI app.",
    ),
    (
        "agent.toolCatalog.tools.platform.open_app.get.sampleResultSummary",
        "Returns one platform OpenAPI app.",
    ),
    (
        "agent.toolCatalog.tools.platform.open_app.create.name",
        "Create platform OpenAPI app",
    ),
    (
        "agent.toolCatalog.tools.platform.open_app.create.description",
        "Create an OpenAPI app in platform settings.",
    ),
    (
        "agent.toolCatalog.tools.platform.open_app.create.sampleResultSummary",
        "Returns the created platform OpenAPI app without exposing its secret key.",
    ),
    (
        "agent.toolCatalog.tools.platform.open_app.update.name",
        "Update platform OpenAPI app",
    ),
    (
        "agent.toolCatalog.tools.platform.open_app.update.description",
        "Update a platform OpenAPI app.",
    ),
    (
        "agent.toolCatalog.tools.platform.open_app.update.sampleResultSummary",
        "Returns the updated platform OpenAPI app.",
    ),
    (
        "agent.toolCatalog.tools.platform.open_app.disable.name",
        "Disable platform OpenAPI app",
    ),
    (
        "agent.toolCatalog.tools.platform.open_app.disable.description",
        "Disable a platform OpenAPI app.",
    ),
    (
        "agent.toolCatalog.tools.platform.open_app.disable.sampleResultSummary",
        "Returns the disabled platform OpenAPI app.",
    ),
    (
        "agent.toolCatalog.tools.platform.resource.query.name",
        "Query platform resources",
    ),
    (
        "agent.toolCatalog.tools.platform.resource.query.description",
        "List resources in the platform settings scope.",
    ),
    (
        "agent.toolCatalog.tools.platform.resource.query.sampleResultSummary",
        "Returns matching platform resources.",
    ),
    (
        "agent.toolCatalog.tools.platform.resource.get.name",
        "Get platform resource",
    ),
    (
        "agent.toolCatalog.tools.platform.resource.get.description",
        "Read one platform resource.",
    ),
    (
        "agent.toolCatalog.tools.platform.resource.get.sampleResultSummary",
        "Returns one platform resource.",
    ),
    (
        "agent.toolCatalog.tools.platform.resource.create.name",
        "Create platform resource",
    ),
    (
        "agent.toolCatalog.tools.platform.resource.create.description",
        "Create a file resource in platform settings.",
    ),
    (
        "agent.toolCatalog.tools.platform.resource.create.sampleResultSummary",
        "Returns the created platform resource.",
    ),
    (
        "agent.toolCatalog.tools.platform.resource.update.name",
        "Update platform resource",
    ),
    (
        "agent.toolCatalog.tools.platform.resource.update.description",
        "Update platform resource metadata.",
    ),
    (
        "agent.toolCatalog.tools.platform.resource.update.sampleResultSummary",
        "Returns the updated platform resource.",
    ),
    (
        "agent.toolCatalog.tools.platform.resource.delete.name",
        "Delete platform resource",
    ),
    (
        "agent.toolCatalog.tools.platform.resource.delete.description",
        "Delete a platform resource.",
    ),
    (
        "agent.toolCatalog.tools.platform.resource.delete.sampleResultSummary",
        "Returns the platform resource delete result.",
    ),
    (
        "agent.toolCatalog.tools.platform.resource.binary.name",
        "Manage platform resource file",
    ),
    (
        "agent.toolCatalog.tools.platform.resource.binary.description",
        "Upload or download platform resource content.",
    ),
    (
        "agent.toolCatalog.tools.platform.resource.binary.sampleResultSummary",
        "Returns platform resource file operation status.",
    ),
    (
        "agent.toolCatalog.tools.automation.page.name",
        "Page automations",
    ),
    (
        "agent.toolCatalog.tools.automation.page.description",
        "Page scheduled automations in the current or specified workspace.",
    ),
    (
        "agent.toolCatalog.tools.automation.page.sampleResultSummary",
        "Returns paged automation records.",
    ),
    (
        "agent.toolCatalog.tools.automation.page.params.search",
        "Optional automation search text.",
    ),
    (
        "agent.toolCatalog.tools.automation.get.name",
        "Get automation",
    ),
    (
        "agent.toolCatalog.tools.automation.get.description",
        "Read one automation detail by automation code.",
    ),
    (
        "agent.toolCatalog.tools.automation.get.sampleResultSummary",
        "Returns one automation detail record.",
    ),
    (
        "agent.toolCatalog.tools.automation.create.name",
        "Create automation",
    ),
    (
        "agent.toolCatalog.tools.automation.create.description",
        "Create an in-platform scheduled task or automation in the current or specified workspace. Use for every-N-minutes/hours/days jobs, cron schedules, reminders, and monitors.",
    ),
    (
        "agent.toolCatalog.tools.automation.create.sampleResultSummary",
        "Returns the created automation detail.",
    ),
    (
        "agent.toolCatalog.tools.automation.update.name",
        "Update automation",
    ),
    (
        "agent.toolCatalog.tools.automation.update.description",
        "Update an in-platform scheduled task or automation, including metadata, prompt, schedule, execution agent/model, or status.",
    ),
    (
        "agent.toolCatalog.tools.automation.update.sampleResultSummary",
        "Returns the updated automation detail.",
    ),
    (
        "agent.toolCatalog.tools.automation.delete.name",
        "Delete automation",
    ),
    (
        "agent.toolCatalog.tools.automation.delete.description",
        "Soft-delete an automation by archiving it.",
    ),
    (
        "agent.toolCatalog.tools.automation.delete.sampleResultSummary",
        "Returns the automation archive result.",
    ),
    (
        "agent.toolCatalog.tools.automation.restore.name",
        "Restore automation",
    ),
    (
        "agent.toolCatalog.tools.automation.restore.description",
        "Restore an archived automation.",
    ),
    (
        "agent.toolCatalog.tools.automation.restore.sampleResultSummary",
        "Returns the automation restore result.",
    ),
    (
        "agent.toolCatalog.tools.automation.execute.name",
        "Execute automation",
    ),
    (
        "agent.toolCatalog.tools.automation.execute.description",
        "Run an automation immediately and record a manual execution.",
    ),
    (
        "agent.toolCatalog.tools.automation.execute.sampleResultSummary",
        "Returns the automation detail with the latest execution rollup.",
    ),
    (
        "agent.toolCatalog.tools.automation.common.params.automationCode",
        "Automation code.",
    ),
    (
        "agent.toolCatalog.tools.automation.common.params.agentCode",
        "Agent code used to execute the automation. Defaults to main-agent.",
    ),
    (
        "agent.toolCatalog.tools.automation.common.params.modelCode",
        "Model code for main-agent, or system model id for a selected system agent.",
    ),
    (
        "agent.toolCatalog.tools.automation.common.params.prompt",
        "Prompt to run when the automation executes.",
    ),
    (
        "agent.toolCatalog.tools.automation.common.params.cronExpression",
        "Cron expression. For example, every 2 minutes can be */2 * * * *. Five-field values are normalized to UTC seconds precision.",
    ),
    (
        "agent.toolCatalog.tools.automation.common.params.status",
        "Automation status. Allowed values: active, paused, archived.",
    ),
];
