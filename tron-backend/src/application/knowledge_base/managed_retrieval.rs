//! Management recall uses ownership, independently of Agent grants and assignments.
use super::*;
use std::collections::BTreeSet;

struct ManagedRagAccess<'a>(&'a KnowledgeBaseService);

impl KnowledgeBaseService {
    pub async fn recall_for_user(
        &self,
        user: &str,
        input: RagQueryInput,
    ) -> Result<RagQueryResult, RagError> {
        self.rag
            .search(
                &ManagedRagAccess(self),
                &RagIdentity {
                    user_code: user.into(),
                    workspace_code: "PLATFORM".into(),
                    agent_code: String::new(),
                },
                input,
                None,
            )
            .await
    }

    pub async fn graph_search_for_user(
        &self,
        user: &str,
        input: crate::application::rag::graph::GraphSearchInput,
    ) -> Result<
        crate::application::rag::graph::GraphQueryOutput,
        crate::application::rag::graph::GraphQueryError,
    > {
        self.rag
            .graph_search(
                &ManagedRagAccess(self),
                &RagIdentity {
                    user_code: user.into(),
                    workspace_code: "PLATFORM".into(),
                    agent_code: String::new(),
                },
                input,
                None,
            )
            .await
    }
    pub async fn graph_expand_for_user(
        &self,
        user: &str,
        input: crate::application::rag::graph::GraphExpandInput,
    ) -> Result<
        crate::application::rag::graph::GraphQueryOutput,
        crate::application::rag::graph::GraphQueryError,
    > {
        self.rag
            .graph_expand(
                &ManagedRagAccess(self),
                &RagIdentity {
                    user_code: user.into(),
                    workspace_code: "PLATFORM".into(),
                    agent_code: String::new(),
                },
                input,
            )
            .await
    }

    pub async fn graph_browse_for_user(
        &self,
        user: &str,
        input: crate::application::rag::graph::GraphBrowseInput,
    ) -> Result<
        crate::application::rag::graph::GraphQueryOutput,
        crate::application::rag::graph::GraphQueryError,
    > {
        self.rag
            .graph_browse(
                &ManagedRagAccess(self),
                &RagIdentity {
                    user_code: user.into(),
                    workspace_code: "PLATFORM".into(),
                    agent_code: String::new(),
                },
                input,
            )
            .await
    }

    fn managed_libraries(&self, user: &str) -> Result<Vec<KnowledgeBase>, RagError> {
        self.require_user(user)
            .map_err(|_| RagError::SearchScopeInvalid)?;
        self.repository
            .platform_libraries_with_documents(user)
            .map_err(|_| RagError::StorageFailure)
    }
}

impl RagAccessResolver for ManagedRagAccess<'_> {
    fn resolve(&self, identity: &RagIdentity) -> Result<Vec<AuthorizedRagRange>, RagError> {
        let libraries = self.0.managed_libraries(&identity.user_code)?;
        let (mut ranges, _, _) = self
            .0
            .rag_ranges_for_libraries(libraries)
            .map_err(|_| RagError::StorageFailure)?;
        // Record task ownership is server-side, including globally managed SYSTEM skills.
        let repository = self.0.rag.repository_handle();
        let mut tasks = repository.record_tasks(&identity.user_code)?;
        if identity.user_code != "SYSTEM" {
            tasks.extend(
                repository
                    .record_tasks("SYSTEM")?
                    .into_iter()
                    .filter(|task| task.corpus_id == "skill"),
            );
        }
        for corpus in ["memory-card", "skill"] {
            let settings = self.0.rag.settings_for_corpus(corpus)?;
            let mut scopes = BTreeMap::<(String, String), BTreeSet<String>>::new();
            for task in tasks.iter().filter(|task| task.corpus_id == corpus) {
                scopes
                    .entry((task.scope_type.clone(), task.scope_code.clone()))
                    .or_default()
                    .insert(task.source_code.clone());
            }
            if scopes.is_empty() {
                scopes.insert(("USER".into(), identity.user_code.clone()), BTreeSet::new());
            }
            for ((scope_type, scope_code), sources) in scopes {
                let mut filters = json!({"sourceCode":{"op":"in","value":sources}});
                if corpus == "memory-card" {
                    filters["memoryStatus"] = json!("ACTIVE");
                    filters["reviewStatus"] = json!("APPROVED");
                    filters["injectable"] = json!(true);
                    filters["expiresAt"] =
                        json!({"op":"gtOrNull","value":chrono::Utc::now().timestamp_millis()});
                }
                ranges.extend(self.0.rag.partition_ranges(AuthorizedRagRange {
                    corpus_id: corpus.into(),
                    scopes: vec![RagSearchScope {
                        scope_type,
                        scope_code,
                    }],
                    source_version: "v1".into(),
                    filters,
                    embedding_model_code: settings.embedding_model_code.clone(),
                    embedding_dimension: Some(TRON_VECTOR_INDEX_DIMENSION),
                    provider_model: None,
                    embedding_fingerprint: None,
                    return_fields: if corpus == "memory-card" {
                        vec!["expiresAt".into()]
                    } else {
                        vec![]
                    },
                    fulltext: settings.fulltext,
                    vector: settings.vector,
                    graph: settings.graph,
                })?);
            }
        }
        Ok(ranges)
    }

    fn recheck_sources(
        &self,
        identity: &RagIdentity,
        result: &mut RagEvidenceResult,
    ) -> Result<(), RagError> {
        // Re-resolve ownership and publication after asynchronous model calls.
        let ranges = self.resolve(identity)?;
        let repository = self.0.rag.repository_handle();
        let mut tasks = repository.record_tasks(&identity.user_code)?;
        if identity.user_code != "SYSTEM" {
            tasks.extend(
                repository
                    .record_tasks("SYSTEM")?
                    .into_iter()
                    .filter(|task| task.corpus_id == "skill"),
            );
        }
        let owned_tasks: BTreeSet<_> = tasks.into_iter().map(|task| task.id).collect();
        let now = chrono::Utc::now().timestamp_millis();
        result.hits.retain(|hit| {
            ranges.iter().any(|range| {
                range.corpus_id == hit.row.corpus_id
                    && range.scopes.iter().any(|scope| {
                        scope.scope_type == hit.row.scope_type
                            && scope.scope_code == hit.row.scope_code
                    })
                    && range.filters["sourceCode"]["value"]
                        .as_array()
                        .is_some_and(|sources| {
                            sources
                                .iter()
                                .any(|source| source.as_str() == Some(&hit.row.source_code))
                        })
            }) && (!matches!(hit.row.corpus_id.as_str(), "memory-card" | "skill")
                || hit.row.source_metadata["indexTaskId"]
                    .as_str()
                    .is_some_and(|id| owned_tasks.contains(id)))
                && (hit.row.corpus_id != "memory-card"
                    || hit
                        .row
                        .field_values
                        .get("expiresAt")
                        .and_then(Value::as_i64)
                        .is_none_or(|expiry| expiry > now))
        });

        Ok(())
    }
    fn recheck(
        &self,
        identity: &RagIdentity,
        result: &mut RagEvidenceResult,
    ) -> Result<(), RagError> {
        self.recheck_sources(identity, result)
    }
    fn assemble_context(
        &self,
        identity: &RagIdentity,
        result: &mut RagEvidenceResult,
    ) -> Result<(), RagError> {
        self.0
            .expand_context_evidence(&identity.user_code, result)
            .map_err(|_| RagError::StorageFailure)
    }
}
