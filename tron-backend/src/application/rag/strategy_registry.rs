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
                "RAG_SPLIT_TEXT",
                "RAG_FULLTEXT_INDEX",
                "RAG_VECTOR_INDEX",
                "RAG_PERSIST_SOURCE",
                "PROJECT_DOC_VALIDATE_SOURCE_V1",
                "PROJECT_DOC_MARKDOWN_RESOLVE_SOURCE_V1",
                "PROJECT_DOC_MARKDOWN_PARSE_DOCUMENT_V1",
                "PROJECT_DOC_MARKDOWN_SPLIT_TEXT_V1",
            ],
        )
    }

    fn resolve_payload_strategy(&self, strategy_id: &str) -> StrategyRegistration {
        default_registration(
            strategy_id,
            &[
                "MEMORY_PLAIN_TEXT_PAYLOAD_V1",
                "PROJECT_DOC_MARKDOWN_REF_PAYLOAD_V1",
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
                "VECTOR_SQLITE_COSINE_V1",
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
