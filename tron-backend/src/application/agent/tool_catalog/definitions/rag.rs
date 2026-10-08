use super::{ToolCatalogDefinition, schema};
use crate::domain::agent::{ToolAvailabilityState, ToolOperationKind};
use serde_json::json;
pub(super) fn tool_definitions() -> Vec<ToolCatalogDefinition> {
    vec![ToolCatalogDefinition {
        tool_code: "rag.search",
        tool_name: "rag_search".into(),
        domain_code: "rag",
        name_i18n_key: "agent.toolCatalog.tools.rag.search.name",
        description_i18n_key: "agent.toolCatalog.tools.rag.search.description",
        operation_kind: ToolOperationKind::Query,
        input_schema: schema(
            "agent.toolCatalog.tools.rag.search.description",
            json!([{"query":"查找问题相关证据"}]),
            json!({"type":"object","required":["query"],"properties":{"query":{"type":"string","minLength":1},"corpusIds":{"type":"array","items":{"type":"string"}},"filters":{"type":"object"},"limit":{"type":"integer","minimum":1,"maximum":100,"default":8}},"additionalProperties":false}),
        ),
        module_owner: "rag",
        availability_state: ToolAvailabilityState::Available,
        sample_result_summary_i18n_key: "agent.toolCatalog.tools.rag.search.summary",
    }]
}
