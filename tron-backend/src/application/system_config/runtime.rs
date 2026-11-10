use std::sync::{Arc, Mutex};

use crate::config::AppConfig;

/// Published after a successful settings save; services share this handle.
#[derive(Clone)]
pub struct BackendRuntimeConfig(pub(super) Arc<Mutex<AppConfig>>);

impl BackendRuntimeConfig {
    pub fn new(config: AppConfig) -> Self {
        Self(Arc::new(Mutex::new(config)))
    }

    pub fn snapshot(&self) -> AppConfig {
        self.0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }
}

pub(super) fn supports_hot_update(path: &str) -> bool {
    path.starts_with("ragRetrieval.")
        || path.starts_with("modelInvocation.")
        || path.starts_with("shortTermMemory.")
        || path.starts_with("mineru.imageDescription.")
        || matches!(
            path,
            "mineru.visualContextEnabled"
                | "navigation.maxDepth"
                | "terminalWorkingDirectory"
                | "resource.limits.attributesMaxBytes"
                | "resource.limits.maxBinaryBytes"
                | "resource.limits.maxCommitBytes"
                | "resource.limits.maxBinaryNameLength"
                | "mineru.ingestion.pollMs"
                | "mineru.ingestion.leaseMs"
                | "mineru.ingestion.maxAttempts"
                | "mineru.ingestion.retryBackoffMs"
                | "mineru.ingestion.embeddingResourceGroup"
                | "mineru.ingestion.embeddingResourceGroupCapacity"
        )
        || matches!(
            path,
            "mineru.enabled"
                | "mineru.baseUrl"
                | "mineru.nodes"
                | "mineru.timeoutSeconds"
                | "mineru.maxBytes"
                | "mineru.ocrMode"
                | "mineru.defaultTier"
                | "mineru.parserProfile"
                | "mineru.expectedParserVersion"
        )
}

pub(super) fn apply_hot_settings(current: &mut AppConfig, next: AppConfig, paths: &[String]) {
    let changed = |prefix: &str| paths.iter().any(|path| path.starts_with(prefix));
    if changed("mineru.") {
        let parse_capacity = current.mineru.ingestion.parse_capacity;
        let embedding_capacity = current.mineru.ingestion.embedding_capacity;
        current.mineru = next.mineru;
        current.mineru.ingestion.parse_capacity = parse_capacity;
        current.mineru.ingestion.embedding_capacity = embedding_capacity;
    }
    if changed("ragRetrieval.") {
        current.rag_retrieval = next.rag_retrieval;
    }
    if changed("modelInvocation.") {
        current.model_invocation = next.model_invocation;
    }
    if changed("shortTermMemory.") {
        current.short_term_memory = next.short_term_memory;
    }
    if changed("resource.limits.") {
        current.resource.limits.attributes_max_bytes = next.resource.limits.attributes_max_bytes;
        current.resource.limits.max_binary_bytes = next.resource.limits.max_binary_bytes;
        current.resource.limits.max_commit_bytes = next.resource.limits.max_commit_bytes;
        current.resource.limits.max_binary_name_length =
            next.resource.limits.max_binary_name_length;
    }
    if changed("navigation.") {
        current.navigation = next.navigation;
    }
    if changed("terminalWorkingDirectory") {
        current.terminal_working_directory = next.terminal_working_directory;
    }
}
