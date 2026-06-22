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
        "App runtime role. Allowed values: frontend, backend.",
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
        "Task priority. Allowed values: low, medium, high.",
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
];
