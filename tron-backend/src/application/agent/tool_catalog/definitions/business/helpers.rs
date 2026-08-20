use serde_json::{Value, json};

use crate::domain::agent::{ToolAvailabilityState, ToolOperationKind};

use super::super::{ToolCatalogDefinition, schema};

pub(super) fn definition(
    tool_code: &'static str,
    domain_code: &'static str,
    operation_kind: ToolOperationKind,
    input_schema: Value,
) -> ToolCatalogDefinition {
    ToolCatalogDefinition {
        tool_code,
        domain_code,
        name_i18n_key: tool_key(tool_code, "name"),
        description_i18n_key: tool_key(tool_code, "description"),
        operation_kind,
        input_schema,
        module_owner: domain_code,
        availability_state: ToolAvailabilityState::Available,
        sample_result_summary_i18n_key: tool_key(tool_code, "sampleResultSummary"),
    }
}

fn tool_key(tool_code: &'static str, suffix: &'static str) -> &'static str {
    match (tool_code, suffix) {
        ("workspace.page", "name") => "agent.toolCatalog.tools.workspace.page.name",
        ("workspace.page", "description") => "agent.toolCatalog.tools.workspace.page.description",
        ("workspace.page", "sampleResultSummary") => {
            "agent.toolCatalog.tools.workspace.page.sampleResultSummary"
        }
        ("workspace.get", "name") => "agent.toolCatalog.tools.workspace.get.name",
        ("workspace.get", "description") => "agent.toolCatalog.tools.workspace.get.description",
        ("workspace.get", "sampleResultSummary") => {
            "agent.toolCatalog.tools.workspace.get.sampleResultSummary"
        }
        ("app.page", "name") => "agent.toolCatalog.tools.app.page.name",
        ("app.page", "description") => "agent.toolCatalog.tools.app.page.description",
        ("app.page", "sampleResultSummary") => {
            "agent.toolCatalog.tools.app.page.sampleResultSummary"
        }
        ("app.create", "name") => "agent.toolCatalog.tools.app.create.name",
        ("app.create", "description") => "agent.toolCatalog.tools.app.create.description",
        ("app.create", "sampleResultSummary") => {
            "agent.toolCatalog.tools.app.create.sampleResultSummary"
        }
        ("app.update", "name") => "agent.toolCatalog.tools.app.update.name",
        ("app.update", "description") => "agent.toolCatalog.tools.app.update.description",
        ("app.update", "sampleResultSummary") => {
            "agent.toolCatalog.tools.app.update.sampleResultSummary"
        }
        ("app.delete", "name") => "agent.toolCatalog.tools.app.delete.name",
        ("app.delete", "description") => "agent.toolCatalog.tools.app.delete.description",
        ("app.delete", "sampleResultSummary") => {
            "agent.toolCatalog.tools.app.delete.sampleResultSummary"
        }
        ("app.restore", "name") => "agent.toolCatalog.tools.app.restore.name",
        ("app.restore", "description") => "agent.toolCatalog.tools.app.restore.description",
        ("app.restore", "sampleResultSummary") => {
            "agent.toolCatalog.tools.app.restore.sampleResultSummary"
        }
        ("automation.page", "name") => "agent.toolCatalog.tools.automation.page.name",
        ("automation.page", "description") => "agent.toolCatalog.tools.automation.page.description",
        ("automation.page", "sampleResultSummary") => {
            "agent.toolCatalog.tools.automation.page.sampleResultSummary"
        }
        ("automation.get", "name") => "agent.toolCatalog.tools.automation.get.name",
        ("automation.get", "description") => "agent.toolCatalog.tools.automation.get.description",
        ("automation.get", "sampleResultSummary") => {
            "agent.toolCatalog.tools.automation.get.sampleResultSummary"
        }
        ("automation.create", "name") => "agent.toolCatalog.tools.automation.create.name",
        ("automation.create", "description") => {
            "agent.toolCatalog.tools.automation.create.description"
        }
        ("automation.create", "sampleResultSummary") => {
            "agent.toolCatalog.tools.automation.create.sampleResultSummary"
        }
        ("automation.update", "name") => "agent.toolCatalog.tools.automation.update.name",
        ("automation.update", "description") => {
            "agent.toolCatalog.tools.automation.update.description"
        }
        ("automation.update", "sampleResultSummary") => {
            "agent.toolCatalog.tools.automation.update.sampleResultSummary"
        }
        ("automation.delete", "name") => "agent.toolCatalog.tools.automation.delete.name",
        ("automation.delete", "description") => {
            "agent.toolCatalog.tools.automation.delete.description"
        }
        ("automation.delete", "sampleResultSummary") => {
            "agent.toolCatalog.tools.automation.delete.sampleResultSummary"
        }
        ("automation.restore", "name") => "agent.toolCatalog.tools.automation.restore.name",
        ("automation.restore", "description") => {
            "agent.toolCatalog.tools.automation.restore.description"
        }
        ("automation.restore", "sampleResultSummary") => {
            "agent.toolCatalog.tools.automation.restore.sampleResultSummary"
        }
        ("automation.execute", "name") => "agent.toolCatalog.tools.automation.execute.name",
        ("automation.execute", "description") => {
            "agent.toolCatalog.tools.automation.execute.description"
        }
        ("automation.execute", "sampleResultSummary") => {
            "agent.toolCatalog.tools.automation.execute.sampleResultSummary"
        }
        ("task.page", "name") => "agent.toolCatalog.tools.task.page.name",
        ("task.page", "description") => "agent.toolCatalog.tools.task.page.description",
        ("task.page", "sampleResultSummary") => {
            "agent.toolCatalog.tools.task.page.sampleResultSummary"
        }
        ("task.create", "name") => "agent.toolCatalog.tools.task.create.name",
        ("task.create", "description") => "agent.toolCatalog.tools.task.create.description",
        ("task.create", "sampleResultSummary") => {
            "agent.toolCatalog.tools.task.create.sampleResultSummary"
        }
        ("task.update", "name") => "agent.toolCatalog.tools.task.update.name",
        ("task.update", "description") => "agent.toolCatalog.tools.task.update.description",
        ("task.update", "sampleResultSummary") => {
            "agent.toolCatalog.tools.task.update.sampleResultSummary"
        }
        ("task.delete", "name") => "agent.toolCatalog.tools.task.delete.name",
        ("task.delete", "description") => "agent.toolCatalog.tools.task.delete.description",
        ("task.delete", "sampleResultSummary") => {
            "agent.toolCatalog.tools.task.delete.sampleResultSummary"
        }
        ("task.restore", "name") => "agent.toolCatalog.tools.task.restore.name",
        ("task.restore", "description") => "agent.toolCatalog.tools.task.restore.description",
        ("task.restore", "sampleResultSummary") => {
            "agent.toolCatalog.tools.task.restore.sampleResultSummary"
        }
        ("model.page", "name") => "agent.toolCatalog.tools.model.page.name",
        ("model.page", "description") => "agent.toolCatalog.tools.model.page.description",
        ("model.page", "sampleResultSummary") => {
            "agent.toolCatalog.tools.model.page.sampleResultSummary"
        }
        ("model.create", "name") => "agent.toolCatalog.tools.model.create.name",
        ("model.create", "description") => "agent.toolCatalog.tools.model.create.description",
        ("model.create", "sampleResultSummary") => {
            "agent.toolCatalog.tools.model.create.sampleResultSummary"
        }
        ("model.update", "name") => "agent.toolCatalog.tools.model.update.name",
        ("model.update", "description") => "agent.toolCatalog.tools.model.update.description",
        ("model.update", "sampleResultSummary") => {
            "agent.toolCatalog.tools.model.update.sampleResultSummary"
        }
        ("model.enable", "name") => "agent.toolCatalog.tools.model.enable.name",
        ("model.enable", "description") => "agent.toolCatalog.tools.model.enable.description",
        ("model.enable", "sampleResultSummary") => {
            "agent.toolCatalog.tools.model.enable.sampleResultSummary"
        }
        ("model.disable", "name") => "agent.toolCatalog.tools.model.disable.name",
        ("model.disable", "description") => "agent.toolCatalog.tools.model.disable.description",
        ("model.disable", "sampleResultSummary") => {
            "agent.toolCatalog.tools.model.disable.sampleResultSummary"
        }
        ("model.delete", "name") => "agent.toolCatalog.tools.model.delete.name",
        ("model.delete", "description") => "agent.toolCatalog.tools.model.delete.description",
        ("model.delete", "sampleResultSummary") => {
            "agent.toolCatalog.tools.model.delete.sampleResultSummary"
        }
        ("skill.page", "name") => "agent.toolCatalog.tools.skill.page.name",
        ("skill.page", "description") => "agent.toolCatalog.tools.skill.page.description",
        ("skill.page", "sampleResultSummary") => {
            "agent.toolCatalog.tools.skill.page.sampleResultSummary"
        }
        ("skill.view", "name") => "agent.toolCatalog.tools.skill.view.name",
        ("skill.view", "description") => "agent.toolCatalog.tools.skill.view.description",
        ("skill.view", "sampleResultSummary") => {
            "agent.toolCatalog.tools.skill.view.sampleResultSummary"
        }
        ("variable.page", "name") => "agent.toolCatalog.tools.variable.page.name",
        ("variable.page", "description") => "agent.toolCatalog.tools.variable.page.description",
        ("variable.page", "sampleResultSummary") => {
            "agent.toolCatalog.tools.variable.page.sampleResultSummary"
        }
        ("variable.create", "name") => "agent.toolCatalog.tools.variable.create.name",
        ("variable.create", "description") => "agent.toolCatalog.tools.variable.create.description",
        ("variable.create", "sampleResultSummary") => {
            "agent.toolCatalog.tools.variable.create.sampleResultSummary"
        }
        ("variable.update", "name") => "agent.toolCatalog.tools.variable.update.name",
        ("variable.update", "description") => "agent.toolCatalog.tools.variable.update.description",
        ("variable.update", "sampleResultSummary") => {
            "agent.toolCatalog.tools.variable.update.sampleResultSummary"
        }
        ("variable.delete", "name") => "agent.toolCatalog.tools.variable.delete.name",
        ("variable.delete", "description") => "agent.toolCatalog.tools.variable.delete.description",
        ("variable.delete", "sampleResultSummary") => {
            "agent.toolCatalog.tools.variable.delete.sampleResultSummary"
        }
        ("platform.variable.page", "name") => "agent.toolCatalog.tools.platform.variable.page.name",
        ("platform.variable.page", "description") => {
            "agent.toolCatalog.tools.platform.variable.page.description"
        }
        ("platform.variable.page", "sampleResultSummary") => {
            "agent.toolCatalog.tools.platform.variable.page.sampleResultSummary"
        }
        ("platform.variable.create", "name") => {
            "agent.toolCatalog.tools.platform.variable.create.name"
        }
        ("platform.variable.create", "description") => {
            "agent.toolCatalog.tools.platform.variable.create.description"
        }
        ("platform.variable.create", "sampleResultSummary") => {
            "agent.toolCatalog.tools.platform.variable.create.sampleResultSummary"
        }
        ("platform.variable.update", "name") => {
            "agent.toolCatalog.tools.platform.variable.update.name"
        }
        ("platform.variable.update", "description") => {
            "agent.toolCatalog.tools.platform.variable.update.description"
        }
        ("platform.variable.update", "sampleResultSummary") => {
            "agent.toolCatalog.tools.platform.variable.update.sampleResultSummary"
        }
        ("platform.variable.delete", "name") => {
            "agent.toolCatalog.tools.platform.variable.delete.name"
        }
        ("platform.variable.delete", "description") => {
            "agent.toolCatalog.tools.platform.variable.delete.description"
        }
        ("platform.variable.delete", "sampleResultSummary") => {
            "agent.toolCatalog.tools.platform.variable.delete.sampleResultSummary"
        }
        ("platform.model.page", "name") => "agent.toolCatalog.tools.platform.model.page.name",
        ("platform.model.page", "description") => {
            "agent.toolCatalog.tools.platform.model.page.description"
        }
        ("platform.model.page", "sampleResultSummary") => {
            "agent.toolCatalog.tools.platform.model.page.sampleResultSummary"
        }
        ("platform.model.get", "name") => "agent.toolCatalog.tools.platform.model.get.name",
        ("platform.model.get", "description") => {
            "agent.toolCatalog.tools.platform.model.get.description"
        }
        ("platform.model.get", "sampleResultSummary") => {
            "agent.toolCatalog.tools.platform.model.get.sampleResultSummary"
        }
        ("platform.model.create", "name") => "agent.toolCatalog.tools.platform.model.create.name",
        ("platform.model.create", "description") => {
            "agent.toolCatalog.tools.platform.model.create.description"
        }
        ("platform.model.create", "sampleResultSummary") => {
            "agent.toolCatalog.tools.platform.model.create.sampleResultSummary"
        }
        ("platform.model.update", "name") => "agent.toolCatalog.tools.platform.model.update.name",
        ("platform.model.update", "description") => {
            "agent.toolCatalog.tools.platform.model.update.description"
        }
        ("platform.model.update", "sampleResultSummary") => {
            "agent.toolCatalog.tools.platform.model.update.sampleResultSummary"
        }
        ("platform.model.enable", "name") => "agent.toolCatalog.tools.platform.model.enable.name",
        ("platform.model.enable", "description") => {
            "agent.toolCatalog.tools.platform.model.enable.description"
        }
        ("platform.model.enable", "sampleResultSummary") => {
            "agent.toolCatalog.tools.platform.model.enable.sampleResultSummary"
        }
        ("platform.model.disable", "name") => "agent.toolCatalog.tools.platform.model.disable.name",
        ("platform.model.disable", "description") => {
            "agent.toolCatalog.tools.platform.model.disable.description"
        }
        ("platform.model.disable", "sampleResultSummary") => {
            "agent.toolCatalog.tools.platform.model.disable.sampleResultSummary"
        }
        ("platform.model.delete", "name") => "agent.toolCatalog.tools.platform.model.delete.name",
        ("platform.model.delete", "description") => {
            "agent.toolCatalog.tools.platform.model.delete.description"
        }
        ("platform.model.delete", "sampleResultSummary") => {
            "agent.toolCatalog.tools.platform.model.delete.sampleResultSummary"
        }
        ("platform.prompt.page", "name") => "agent.toolCatalog.tools.platform.prompt.page.name",
        ("platform.prompt.page", "description") => {
            "agent.toolCatalog.tools.platform.prompt.page.description"
        }
        ("platform.prompt.page", "sampleResultSummary") => {
            "agent.toolCatalog.tools.platform.prompt.page.sampleResultSummary"
        }
        ("platform.prompt.get", "name") => "agent.toolCatalog.tools.platform.prompt.get.name",
        ("platform.prompt.get", "description") => {
            "agent.toolCatalog.tools.platform.prompt.get.description"
        }
        ("platform.prompt.get", "sampleResultSummary") => {
            "agent.toolCatalog.tools.platform.prompt.get.sampleResultSummary"
        }
        ("platform.prompt.create", "name") => "agent.toolCatalog.tools.platform.prompt.create.name",
        ("platform.prompt.create", "description") => {
            "agent.toolCatalog.tools.platform.prompt.create.description"
        }
        ("platform.prompt.create", "sampleResultSummary") => {
            "agent.toolCatalog.tools.platform.prompt.create.sampleResultSummary"
        }
        ("platform.prompt.update", "name") => "agent.toolCatalog.tools.platform.prompt.update.name",
        ("platform.prompt.update", "description") => {
            "agent.toolCatalog.tools.platform.prompt.update.description"
        }
        ("platform.prompt.update", "sampleResultSummary") => {
            "agent.toolCatalog.tools.platform.prompt.update.sampleResultSummary"
        }
        ("platform.prompt.delete", "name") => "agent.toolCatalog.tools.platform.prompt.delete.name",
        ("platform.prompt.delete", "description") => {
            "agent.toolCatalog.tools.platform.prompt.delete.description"
        }
        ("platform.prompt.delete", "sampleResultSummary") => {
            "agent.toolCatalog.tools.platform.prompt.delete.sampleResultSummary"
        }
        ("platform.mcp.page", "name") => "agent.toolCatalog.tools.platform.mcp.page.name",
        ("platform.mcp.page", "description") => {
            "agent.toolCatalog.tools.platform.mcp.page.description"
        }
        ("platform.mcp.page", "sampleResultSummary") => {
            "agent.toolCatalog.tools.platform.mcp.page.sampleResultSummary"
        }
        ("platform.mcp.get", "name") => "agent.toolCatalog.tools.platform.mcp.get.name",
        ("platform.mcp.get", "description") => {
            "agent.toolCatalog.tools.platform.mcp.get.description"
        }
        ("platform.mcp.get", "sampleResultSummary") => {
            "agent.toolCatalog.tools.platform.mcp.get.sampleResultSummary"
        }
        ("platform.mcp.create", "name") => "agent.toolCatalog.tools.platform.mcp.create.name",
        ("platform.mcp.create", "description") => {
            "agent.toolCatalog.tools.platform.mcp.create.description"
        }
        ("platform.mcp.create", "sampleResultSummary") => {
            "agent.toolCatalog.tools.platform.mcp.create.sampleResultSummary"
        }
        ("platform.mcp.update", "name") => "agent.toolCatalog.tools.platform.mcp.update.name",
        ("platform.mcp.update", "description") => {
            "agent.toolCatalog.tools.platform.mcp.update.description"
        }
        ("platform.mcp.update", "sampleResultSummary") => {
            "agent.toolCatalog.tools.platform.mcp.update.sampleResultSummary"
        }
        ("platform.mcp.delete", "name") => "agent.toolCatalog.tools.platform.mcp.delete.name",
        ("platform.mcp.delete", "description") => {
            "agent.toolCatalog.tools.platform.mcp.delete.description"
        }
        ("platform.mcp.delete", "sampleResultSummary") => {
            "agent.toolCatalog.tools.platform.mcp.delete.sampleResultSummary"
        }
        ("platform.open_app.page", "name") => "agent.toolCatalog.tools.platform.open_app.page.name",
        ("platform.open_app.page", "description") => {
            "agent.toolCatalog.tools.platform.open_app.page.description"
        }
        ("platform.open_app.page", "sampleResultSummary") => {
            "agent.toolCatalog.tools.platform.open_app.page.sampleResultSummary"
        }
        ("platform.open_app.get", "name") => "agent.toolCatalog.tools.platform.open_app.get.name",
        ("platform.open_app.get", "description") => {
            "agent.toolCatalog.tools.platform.open_app.get.description"
        }
        ("platform.open_app.get", "sampleResultSummary") => {
            "agent.toolCatalog.tools.platform.open_app.get.sampleResultSummary"
        }
        ("platform.open_app.create", "name") => {
            "agent.toolCatalog.tools.platform.open_app.create.name"
        }
        ("platform.open_app.create", "description") => {
            "agent.toolCatalog.tools.platform.open_app.create.description"
        }
        ("platform.open_app.create", "sampleResultSummary") => {
            "agent.toolCatalog.tools.platform.open_app.create.sampleResultSummary"
        }
        ("platform.open_app.update", "name") => {
            "agent.toolCatalog.tools.platform.open_app.update.name"
        }
        ("platform.open_app.update", "description") => {
            "agent.toolCatalog.tools.platform.open_app.update.description"
        }
        ("platform.open_app.update", "sampleResultSummary") => {
            "agent.toolCatalog.tools.platform.open_app.update.sampleResultSummary"
        }
        ("platform.open_app.disable", "name") => {
            "agent.toolCatalog.tools.platform.open_app.disable.name"
        }
        ("platform.open_app.disable", "description") => {
            "agent.toolCatalog.tools.platform.open_app.disable.description"
        }
        ("platform.open_app.disable", "sampleResultSummary") => {
            "agent.toolCatalog.tools.platform.open_app.disable.sampleResultSummary"
        }
        ("platform.resource.query", "name") => {
            "agent.toolCatalog.tools.platform.resource.query.name"
        }
        ("platform.resource.query", "description") => {
            "agent.toolCatalog.tools.platform.resource.query.description"
        }
        ("platform.resource.query", "sampleResultSummary") => {
            "agent.toolCatalog.tools.platform.resource.query.sampleResultSummary"
        }
        ("platform.resource.get", "name") => "agent.toolCatalog.tools.platform.resource.get.name",
        ("platform.resource.get", "description") => {
            "agent.toolCatalog.tools.platform.resource.get.description"
        }
        ("platform.resource.get", "sampleResultSummary") => {
            "agent.toolCatalog.tools.platform.resource.get.sampleResultSummary"
        }
        ("platform.resource.create", "name") => {
            "agent.toolCatalog.tools.platform.resource.create.name"
        }
        ("platform.resource.create", "description") => {
            "agent.toolCatalog.tools.platform.resource.create.description"
        }
        ("platform.resource.create", "sampleResultSummary") => {
            "agent.toolCatalog.tools.platform.resource.create.sampleResultSummary"
        }
        ("platform.resource.update", "name") => {
            "agent.toolCatalog.tools.platform.resource.update.name"
        }
        ("platform.resource.update", "description") => {
            "agent.toolCatalog.tools.platform.resource.update.description"
        }
        ("platform.resource.update", "sampleResultSummary") => {
            "agent.toolCatalog.tools.platform.resource.update.sampleResultSummary"
        }
        ("platform.resource.delete", "name") => {
            "agent.toolCatalog.tools.platform.resource.delete.name"
        }
        ("platform.resource.delete", "description") => {
            "agent.toolCatalog.tools.platform.resource.delete.description"
        }
        ("platform.resource.delete", "sampleResultSummary") => {
            "agent.toolCatalog.tools.platform.resource.delete.sampleResultSummary"
        }
        ("platform.resource.binary", "name") => {
            "agent.toolCatalog.tools.platform.resource.binary.name"
        }
        ("platform.resource.binary", "description") => {
            "agent.toolCatalog.tools.platform.resource.binary.description"
        }
        ("platform.resource.binary", "sampleResultSummary") => {
            "agent.toolCatalog.tools.platform.resource.binary.sampleResultSummary"
        }
        ("resource_definition.query", "name") => {
            "agent.toolCatalog.tools.resource_definition.query.name"
        }
        ("resource_definition.query", "description") => {
            "agent.toolCatalog.tools.resource_definition.query.description"
        }
        ("resource_definition.query", "sampleResultSummary") => {
            "agent.toolCatalog.tools.resource_definition.query.sampleResultSummary"
        }
        ("resource_definition.draft", "name") => {
            "agent.toolCatalog.tools.resource_definition.draft.name"
        }
        ("resource_definition.draft", "description") => {
            "agent.toolCatalog.tools.resource_definition.draft.description"
        }
        ("resource_definition.draft", "sampleResultSummary") => {
            "agent.toolCatalog.tools.resource_definition.draft.sampleResultSummary"
        }
        ("resource_definition.publish", "name") => {
            "agent.toolCatalog.tools.resource_definition.publish.name"
        }
        ("resource_definition.publish", "description") => {
            "agent.toolCatalog.tools.resource_definition.publish.description"
        }
        ("resource_definition.publish", "sampleResultSummary") => {
            "agent.toolCatalog.tools.resource_definition.publish.sampleResultSummary"
        }
        ("resource.query", "name") => "agent.toolCatalog.tools.resource.query.name",
        ("resource.query", "description") => "agent.toolCatalog.tools.resource.query.description",
        ("resource.query", "sampleResultSummary") => {
            "agent.toolCatalog.tools.resource.query.sampleResultSummary"
        }
        ("resource.get", "name") => "agent.toolCatalog.tools.resource.get.name",
        ("resource.get", "description") => "agent.toolCatalog.tools.resource.get.description",
        ("resource.get", "sampleResultSummary") => {
            "agent.toolCatalog.tools.resource.get.sampleResultSummary"
        }
        ("resource.create", "name") => "agent.toolCatalog.tools.resource.create.name",
        ("resource.create", "description") => "agent.toolCatalog.tools.resource.create.description",
        ("resource.create", "sampleResultSummary") => {
            "agent.toolCatalog.tools.resource.create.sampleResultSummary"
        }
        ("resource.update", "name") => "agent.toolCatalog.tools.resource.update.name",
        ("resource.update", "description") => "agent.toolCatalog.tools.resource.update.description",
        ("resource.update", "sampleResultSummary") => {
            "agent.toolCatalog.tools.resource.update.sampleResultSummary"
        }
        ("resource.delete", "name") => "agent.toolCatalog.tools.resource.delete.name",
        ("resource.delete", "description") => "agent.toolCatalog.tools.resource.delete.description",
        ("resource.delete", "sampleResultSummary") => {
            "agent.toolCatalog.tools.resource.delete.sampleResultSummary"
        }
        ("resource.draft", "name") => "agent.toolCatalog.tools.resource.draft.name",
        ("resource.draft", "description") => "agent.toolCatalog.tools.resource.draft.description",
        ("resource.draft", "sampleResultSummary") => {
            "agent.toolCatalog.tools.resource.draft.sampleResultSummary"
        }
        ("resource.publish", "name") => "agent.toolCatalog.tools.resource.publish.name",
        ("resource.publish", "description") => {
            "agent.toolCatalog.tools.resource.publish.description"
        }
        ("resource.publish", "sampleResultSummary") => {
            "agent.toolCatalog.tools.resource.publish.sampleResultSummary"
        }
        ("resource.binary", "name") => "agent.toolCatalog.tools.resource.binary.name",
        ("resource.binary", "description") => "agent.toolCatalog.tools.resource.binary.description",
        ("resource.binary", "sampleResultSummary") => {
            "agent.toolCatalog.tools.resource.binary.sampleResultSummary"
        }
        ("resource.data_source.create", "name") => {
            "agent.toolCatalog.tools.resource.data_source.create.name"
        }
        ("resource.data_source.create", "description") => {
            "agent.toolCatalog.tools.resource.data_source.create.description"
        }
        ("resource.data_source.create", "sampleResultSummary") => {
            "agent.toolCatalog.tools.resource.data_source.create.sampleResultSummary"
        }
        ("resource.data_source.update", "name") => {
            "agent.toolCatalog.tools.resource.data_source.update.name"
        }
        ("resource.data_source.update", "description") => {
            "agent.toolCatalog.tools.resource.data_source.update.description"
        }
        ("resource.data_source.update", "sampleResultSummary") => {
            "agent.toolCatalog.tools.resource.data_source.update.sampleResultSummary"
        }
        ("resource.server.create", "name") => "agent.toolCatalog.tools.resource.server.create.name",
        ("resource.server.create", "description") => {
            "agent.toolCatalog.tools.resource.server.create.description"
        }
        ("resource.server.create", "sampleResultSummary") => {
            "agent.toolCatalog.tools.resource.server.create.sampleResultSummary"
        }
        ("resource.server_account.create", "name") => {
            "agent.toolCatalog.tools.resource.server_account.create.name"
        }
        ("resource.server_account.create", "description") => {
            "agent.toolCatalog.tools.resource.server_account.create.description"
        }
        ("resource.server_account.create", "sampleResultSummary") => {
            "agent.toolCatalog.tools.resource.server_account.create.sampleResultSummary"
        }
        _ => "agent.toolCatalog.tools.unknown.name",
    }
}

pub(super) fn page_properties(extra: Value) -> Value {
    let mut properties = json!({
        "current": { "type": "integer", "minimum": 1, "default": 1, "x-i18n-description-key": "agent.toolCatalog.common.params.current" },
        "size": { "type": "integer", "minimum": 1, "maximum": 100, "default": 50, "x-i18n-description-key": "agent.toolCatalog.common.params.size" }
    });
    if let (Value::Object(target), Value::Object(extra)) = (&mut properties, extra) {
        target.extend(extra);
    }
    properties
}

pub(super) fn workspace_scoped_page_schema(extra: Value) -> Value {
    json!({
        "type": "object",
        "properties": page_properties({
            let mut object = json!({
                "workspaceCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.common.params.workspaceCode" }
            });
            if let (Value::Object(target), Value::Object(extra)) = (&mut object, extra) {
                target.extend(extra);
            }
            object
        }),
        "additionalProperties": false
    })
}

pub(super) fn workspace_code_schema(description_key: &'static str, examples: Value) -> Value {
    schema(
        description_key,
        examples,
        json!({
            "type": "object",
            "required": ["workspaceCode"],
            "properties": {
                "workspaceCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.workspace.get.params.workspaceCode" }
            },
            "additionalProperties": false
        }),
    )
}

pub(super) fn app_code_definition(
    tool_code: &'static str,
    operation_kind: ToolOperationKind,
) -> ToolCatalogDefinition {
    definition(
        tool_code,
        "app",
        operation_kind,
        schema(
            tool_key(tool_code, "description"),
            json!([{ "workspaceCode": "TRON_CORE", "appCode": "APP-1" }]),
            json!({
                "type": "object",
                "required": ["appCode"],
                "properties": {
                    "workspaceCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.common.params.workspaceCode" },
                    "appCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.app.common.params.appCode" }
                },
                "additionalProperties": false
            }),
        ),
    )
}

pub(super) fn task_code_definition(
    tool_code: &'static str,
    operation_kind: ToolOperationKind,
) -> ToolCatalogDefinition {
    definition(
        tool_code,
        "task",
        operation_kind,
        schema(
            tool_key(tool_code, "description"),
            json!([{ "workspaceCode": "TRON_CORE", "taskCode": "TSK-1" }]),
            json!({
                "type": "object",
                "required": ["taskCode"],
                "properties": {
                    "workspaceCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.common.params.workspaceCode" },
                    "taskCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.task.common.params.taskCode" }
                },
                "additionalProperties": false
            }),
        ),
    )
}

pub(super) fn model_code_definition(
    tool_code: &'static str,
    operation_kind: ToolOperationKind,
) -> ToolCatalogDefinition {
    definition(
        tool_code,
        "model",
        operation_kind,
        schema(
            tool_key(tool_code, "description"),
            json!([{ "workspaceCode": "TRON_CORE", "modelCode": "MDL-1" }]),
            json!({
                "type": "object",
                "required": ["modelCode"],
                "properties": {
                    "workspaceCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.common.params.workspaceCode" },
                    "modelCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.modelCode" }
                },
                "additionalProperties": false
            }),
        ),
    )
}

pub(super) fn app_mutation_schema(create: bool) -> Value {
    let required = if create {
        json!(["name", "localDirectory", "runtimeRole"])
    } else {
        json!([])
    };
    json!({
        "type": "object",
        "required": required,
        "properties": {
            "workspaceCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.common.params.workspaceCode" },
            "name": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.common.params.name" },
            "description": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.common.params.description" },
            "localDirectory": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.app.common.params.localDirectory" },
            "runtimeRole": { "type": "string", "enum": ["frontend", "backend", "mono"], "x-i18n-description-key": "agent.toolCatalog.tools.app.common.params.runtimeRole" },
            "repositoryUrl": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.app.common.params.repositoryUrl" },
            "defaultBranch": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.app.common.params.defaultBranch" },
            "tags": { "type": "array", "items": { "type": "string" }, "default": [], "x-i18n-description-key": "agent.toolCatalog.common.params.tags" }
        },
        "additionalProperties": false
    })
}

pub(super) fn app_update_schema() -> Value {
    let mut value = app_mutation_schema(false);
    if let Value::Object(map) = &mut value {
        map.insert("required".to_string(), json!(["appCode"]));
        if let Some(Value::Object(properties)) = map.get_mut("properties") {
            properties.insert(
                "appCode".to_string(),
                json!({ "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.app.common.params.appCode" }),
            );
        }
    }
    value
}

pub(super) fn task_mutation_schema(create: bool) -> Value {
    let required = if create {
        json!(["title"])
    } else {
        json!(["taskCode"])
    };
    json!({
        "type": "object",
        "required": required,
        "properties": {
            "workspaceCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.common.params.workspaceCode" },
            "taskCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.task.common.params.taskCode" },
            "parentTaskCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.task.common.params.parentTaskCode" },
            "title": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.task.common.params.title" },
            "description": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.common.params.description" },
            "priority": { "type": "string", "enum": ["low", "medium", "high", "extra_high"], "x-i18n-description-key": "agent.toolCatalog.tools.task.common.params.priority" },
            "taskType": { "type": "string", "enum": ["feature", "bug"], "x-i18n-description-key": "agent.toolCatalog.tools.task.common.params.taskType" },
            "sourceType": { "type": "string", "enum": ["internal", "github", "gitlab"], "x-i18n-description-key": "agent.toolCatalog.tools.task.common.params.sourceType" },
            "sourceUrl": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.task.common.params.sourceUrl" },
            "assigneeUserCodes": { "type": "array", "items": { "type": "string" }, "x-i18n-description-key": "agent.toolCatalog.tools.task.common.params.assigneeUserCodes" },
            "agentCodes": { "type": "array", "items": { "type": "string" }, "x-i18n-description-key": "agent.toolCatalog.tools.task.common.params.agentCodes" },
            "appCodes": { "type": "array", "items": { "type": "string" }, "x-i18n-description-key": "agent.toolCatalog.tools.task.common.params.appCodes" },
            "appCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.task.common.params.appCode" },
            "status": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.task.page.params.status" },
            "progress": { "type": "integer", "minimum": 0, "maximum": 100, "x-i18n-description-key": "agent.toolCatalog.tools.task.common.params.progress" }
        },
        "additionalProperties": false
    })
}

pub(super) fn model_mutation_schema(create: bool) -> Value {
    let required = if create {
        json!([
            "displayName",
            "vendor",
            "protocol",
            "modelType",
            "baseUrl",
            "apiKeyRef",
            "modelName"
        ])
    } else {
        json!(["modelCode"])
    };
    json!({
        "type": "object",
        "required": required,
        "properties": {
            "workspaceCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.common.params.workspaceCode" },
            "modelCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.modelCode" },
            "displayName": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.displayName" },
            "vendor": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.vendor" },
            "protocol": { "type": "string", "enum": ["openai_compatible", "responses", "rerank"], "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.protocol" },
            "modelType": { "type": "string", "enum": ["text", "embedding", "rerank", "image"], "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.modelType" },
            "modelCapabilities": { "type": "array", "items": { "type": "string" }, "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.modelCapabilities" },
            "baseUrl": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.baseUrl" },
            "apiPath": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.apiPath" },
            "apiKeyRef": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.apiKeyRef" },
            "modelName": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.modelName" },
            "enabled": { "type": "boolean", "default": true, "x-i18n-description-key": "agent.toolCatalog.common.params.enabled" },
            "description": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.common.params.description" },
            "thinkingEnabled": { "type": "boolean", "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.thinkingEnabled" },
            "supportedReasoningEfforts": { "type": "array", "items": { "type": "string", "enum": ["none", "minimal", "low", "medium", "high", "xhigh", "max", "ultra"] }, "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.supportedReasoningEfforts" },
            "defaultReasoningEffort": { "type": "string", "enum": ["none", "minimal", "low", "medium", "high", "xhigh", "max", "ultra"], "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.defaultReasoningEffort" },
            "structuredOutputEnabled": { "type": "boolean", "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.structuredOutputEnabled" },
            "responsesStatefulEnabled": { "type": "boolean", "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.responsesStatefulEnabled" },
            "temperature": { "type": "number", "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.temperature" },
            "maxTokens": { "type": "integer", "minimum": 1, "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.maxTokens" },
            "contextWindowTokens": { "type": "integer", "minimum": 1, "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.contextWindowTokens" },
            "topP": { "type": "number", "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.topP" },
            "maxRetries": { "type": "integer", "minimum": 0, "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.maxRetries" },
            "allowInheritance": { "type": "boolean", "x-i18n-description-key": "agent.toolCatalog.tools.model.common.params.allowInheritance" }
        },
        "additionalProperties": false
    })
}

pub(super) fn variable_mutation_schema(create: bool) -> Value {
    let required = if create {
        json!(["key", "value"])
    } else {
        json!(["key"])
    };
    json!({
        "type": "object",
        "required": required,
        "properties": {
            "workspaceCode": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.common.params.workspaceCode" },
            "key": { "type": "string", "minLength": 1, "x-i18n-description-key": "agent.toolCatalog.tools.variable.common.params.key" },
            "value": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.tools.variable.common.params.value" },
            "description": { "type": "string", "x-i18n-description-key": "agent.toolCatalog.common.params.description" },
            "isSensitive": { "type": "boolean", "x-i18n-description-key": "agent.toolCatalog.tools.variable.common.params.isSensitive" },
            "valueMode": { "type": "string", "enum": ["replace", "preserve"], "x-i18n-description-key": "agent.toolCatalog.tools.variable.common.params.valueMode" }
        },
        "additionalProperties": false
    })
}
