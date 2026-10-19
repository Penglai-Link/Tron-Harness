use super::{ToolCatalogDefinition, schema};
use crate::domain::agent::{ToolAvailabilityState, ToolOperationKind};
use serde_json::{Value, json};

pub(super) fn tool_definitions() -> Vec<ToolCatalogDefinition> {
    let mut tools = Vec::new();
    for (code, name, key, kind, required, properties) in [
        (
            "rag.quality.sources",
            "rag_quality_sources",
            "sources",
            ToolOperationKind::Query,
            json!([]),
            json!({"libraryId":{"type":"string"},"offset":{"type":"integer","minimum":0},"limit":{"type":"integer","minimum":1,"maximum":24}}),
        ),
        (
            "rag.quality.cases",
            "rag_quality_cases",
            "cases",
            ToolOperationKind::Command,
            json!(["libraryId"]),
            json!({"libraryId":{"type":"string"},"sourceFingerprint":{"type":"string"},"cases":{"type":"array","minItems":8,"maxItems":64,"items":{"type":"object","required":["id","query","partition","expected"],"properties":{"id":{"type":"string"},"query":{"type":"string"},"partition":{"type":"string","enum":["training","holdout"]},"expected":{"type":"array","items":{"type":"object","required":["documentId","span"],"properties":{"documentId":{"type":"string"},"span":{"type":"object","required":["blockId","start","end"],"properties":{"blockId":{"type":"string"},"start":{"type":"integer","minimum":0},"end":{"type":"integer","minimum":1}},"additionalProperties":false}},"additionalProperties":false}}},"additionalProperties":false}}}),
        ),
        (
            "rag.quality.start",
            "rag_quality_start",
            "start",
            ToolOperationKind::Command,
            json!(["libraryId", "requestId"]),
            json!({"libraryId":{"type":"string"},"requestId":{"type":"string","minLength":1,"maxLength":120},"options":options_schema()}),
        ),
        (
            "rag.quality.status",
            "rag_quality_status",
            "status",
            ToolOperationKind::Query,
            json!(["libraryId"]),
            json!({"libraryId":{"type":"string"},"runId":{"type":"string"},"waitSeconds":{"type":"integer","minimum":0,"maximum":25}}),
        ),
        (
            "rag.quality.cancel",
            "rag_quality_cancel",
            "cancel",
            ToolOperationKind::Command,
            json!(["libraryId", "runId"]),
            json!({"libraryId":{"type":"string"},"runId":{"type":"string"}}),
        ),
    ] {
        let (title, description, summary) = match key {
            "sources" => (
                "agent.toolCatalog.quality.sources.name",
                "agent.toolCatalog.quality.sources.description",
                "agent.toolCatalog.quality.sources.summary",
            ),
            "cases" => (
                "agent.toolCatalog.quality.cases.name",
                "agent.toolCatalog.quality.cases.description",
                "agent.toolCatalog.quality.cases.summary",
            ),
            "start" => (
                "agent.toolCatalog.quality.start.name",
                "agent.toolCatalog.quality.start.description",
                "agent.toolCatalog.quality.start.summary",
            ),
            "status" => (
                "agent.toolCatalog.quality.status.name",
                "agent.toolCatalog.quality.status.description",
                "agent.toolCatalog.quality.status.summary",
            ),
            _ => (
                "agent.toolCatalog.quality.cancel.name",
                "agent.toolCatalog.quality.cancel.description",
                "agent.toolCatalog.quality.cancel.summary",
            ),
        };
        tools.push(ToolCatalogDefinition{tool_code:code,tool_name:name.into(),domain_code:"rag",name_i18n_key:title,description_i18n_key:description,operation_kind:kind,input_schema:schema(description,json!([]),json!({"type":"object","required":required,"properties":properties,"additionalProperties":false})),module_owner:"knowledge-quality",availability_state:ToolAvailabilityState::Available,sample_result_summary_i18n_key:summary});
    }
    tools
}
fn options_schema() -> Value {
    json!({"type":"object","properties":{"targetRecall":{"type":"number","minimum":0,"maximum":1},"minimumImprovement":{"type":"number","minimum":0,"maximum":0.5},"topK":{"type":"integer","minimum":1,"maximum":30},"maxTrials":{"type":"integer","minimum":1,"maximum":6},"maxCases":{"type":"integer","minimum":8,"maximum":64},"candidates":{"type":"array","maxItems":6,"items":{"type":"object","properties":{"mode":{"type":"string"},"maxChars":{"type":"integer","minimum":200,"maximum":8000},"overlapChars":{"type":"integer","minimum":0},"headingDepth":{"type":"integer","minimum":1,"maximum":6}},"additionalProperties":false}},"budget":{"type":"object","properties":{"maxModelOperations":{"type":"integer","minimum":1,"maximum":1000},"maxInputTokens":{"type":"integer","minimum":1000,"maximum":2000000},"maxDurationSeconds":{"type":"integer","minimum":30,"maximum":3600}},"additionalProperties":false}},"additionalProperties":false})
}
