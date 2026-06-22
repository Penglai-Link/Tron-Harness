use super::{RagStrategyRegistry, StrategyRegistration, StrategyRegistrationStatus};

#[derive(Clone, Default)]
pub struct DefaultRagStrategyRegistry;

impl RagStrategyRegistry for DefaultRagStrategyRegistry {
    fn resolve_stage(&self, strategy_id: &str) -> StrategyRegistration {
        default_registration(
            strategy_id,
            &[
                "INGEST_CHUNK",
                "RAG_VALIDATE_SOURCE",
                "RAG_TEXT_CONTRACT_PAYLOAD_V1",
                "RAG_DOCUMENT_REF_CONTRACT_PAYLOAD_V1",
                "RAG_SPLIT_TEXT",
                "RAG_SPLIT_PLAIN_TEXT_V1",
                "RAG_SPLIT_MARKDOWN_HEADING_V1",
                "RAG_FULLTEXT_INDEX",
                "RAG_MARKDOWN_TREE_EXTRACT_V1",
                "RAG_VECTOR_INDEX",
                "RAG_PERSIST_TREE_GRAPH_V1",
                "RAG_PERSIST_SOURCE",
                "PROJECT_DOC_RESOLVE_SOURCE",
                "RAG_PARSE_MARKDOWN_DOCUMENT",
                "RAG_PARSE_MINERU_DOCUMENT",
                "RAG_QUESTION_GENERALIZATION_SIMPLE_V1",
                "RAG_QUERY_REWRITE_TEMPLATE_V1",
            ],
        )
    }

    fn resolve_payload_strategy(&self, strategy_id: &str) -> StrategyRegistration {
        default_registration(
            strategy_id,
            &[
                "RAG_TEXT_CONTRACT_PAYLOAD_V1",
                "RAG_DOCUMENT_REF_CONTRACT_PAYLOAD_V1",
            ],
        )
    }

    fn resolve_retriever(&self, strategy_id: &str) -> StrategyRegistration {
        default_registration(
            strategy_id,
            &[
                "FULLTEXT",
                "VECTOR",
                "FULLTEXT_SQLITE_LIKE_V1",
                "FULLTEXT_SQLITE_FTS5_SIMPLE_V1",
                "VECTOR_SQLITE_COSINE_V1",
                "VECTOR_SQLITE_VEC0_COSINE_V1",
                "FULLTEXT_GRAPH_SINGLE_NODE_V1",
                "VECTOR_GRAPH_SINGLE_NODE_V1",
                "STRUCTURED_FILTER_SQLITE_V1",
            ],
        )
    }

    fn resolve_policy(&self, strategy_id: &str) -> StrategyRegistration {
        default_registration(
            strategy_id,
            &[
                "FAIL_FAST",
                "PARTIAL_ALLOWED",
                "PARTIAL_INDEX_ALLOWED",
                "SKIP_VECTOR_WITH_WARNING",
                "SKIP_VECTOR_ON_UNAVAILABLE",
                "DEDUP_BY_SOURCE",
                "HYDRATE_TEXT_UNIT",
                "CONTEXT_PACK_BASIC",
            ],
        )
    }
}

fn default_registration(strategy_id: &str, enabled_ids: &[&str]) -> StrategyRegistration {
    let status = if enabled_ids.contains(&strategy_id) {
        StrategyRegistrationStatus::Enabled
    } else if matches!(strategy_id, "GRAPH" | "SUMMARY_TREE") {
        StrategyRegistrationStatus::Reserved
    } else {
        StrategyRegistrationStatus::Unsupported
    };
    StrategyRegistration {
        strategy_id: strategy_id.to_string(),
        status,
    }
}
