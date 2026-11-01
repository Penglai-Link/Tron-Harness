&[
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
]
