use super::{ToolCatalogDefinition, schema};
use crate::domain::agent::{ToolAvailabilityState, ToolOperationKind};
use serde_json::json;
pub(super) fn tool_definitions() -> Vec<ToolCatalogDefinition> {
    let options = json!({"type":"object","properties":{
        "seedCount":{"type":"integer","minimum":1,"maximum":100,"default":10},
        "incomingDepth":{"type":"integer","minimum":0,"maximum":6,"default":3},
        "outgoingDepth":{"type":"integer","minimum":0,"maximum":6,"default":3},
        "nodeLimit":{"type":"integer","minimum":1,"maximum":500,"default":100},
        "edgeLimit":{"type":"integer","minimum":1,"maximum":1000,"default":200},
        "relationshipTypes":{"type":"array","maxItems":32,"uniqueItems":true,"items":{"type":"string","minLength":1,"maxLength":128}},
        "includeStructure":{"type":"boolean","default":false}},"additionalProperties":false});
    [("rag.graph.search","rag_graph_search","agent.toolCatalog.ragGraph.search.name","agent.toolCatalog.ragGraph.search.description","agent.toolCatalog.ragGraph.search.summary",true),
     ("rag.graph.expand","rag_graph_expand","agent.toolCatalog.ragGraph.expand.name","agent.toolCatalog.ragGraph.expand.description","agent.toolCatalog.ragGraph.expand.summary",false)]
    .into_iter().map(|(code,name,title,description,summary,search)|{
        let mut properties=json!({"corpusIds":{"type":"array","maxItems":100,"items":{"type":"string"}},"filters":{"type":"object"},"options":options});
        if search{properties["query"]=json!({"type":"string","minLength":1,"maxLength":16000});}
        else{properties["nodeIds"]=json!({"type":"array","minItems":1,"maxItems":100,"uniqueItems":true,"items":{"type":"string"}});properties["cursor"]=json!({"type":"string","minLength":64,"maxLength":64});}
        let mut input=json!({"type":"object","properties":properties,"additionalProperties":false});
        if search{input["required"]=json!(["query"]);}else{input["oneOf"]=json!([{"required":["nodeIds"],"not":{"required":["cursor"]}},{"required":["cursor"],"not":{"anyOf":[{"required":["nodeIds"]},{"required":["corpusIds"]},{"required":["filters"]},{"required":["options"]}]}}]);}
        ToolCatalogDefinition{tool_code:code,tool_name:name.into(),domain_code:"harness_rag", category_code: "tron.platform",name_i18n_key:title,description_i18n_key:description,operation_kind:ToolOperationKind::Query,
            input_schema:schema(description,if search{json!([{"query":"查找相关实体及原文依据"}])}else{json!([{"nodeIds":["node-id"],"options":{"incomingDepth":1,"outgoingDepth":3}}])},input),
            module_owner:"rag",availability_state:ToolAvailabilityState::Available,sample_result_summary_i18n_key:summary}
    }).collect()
}
