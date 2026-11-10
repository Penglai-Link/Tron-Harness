use super::*;
use crate::application::rag::*;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::Arc;

#[derive(Clone)]
pub struct MemoryService<R, E> {
    pub(super) rag: RagService<R, E>,
    pub(super) store: Arc<dyn MemoryRecordRepository>,
    index_runtime: Option<tokio::runtime::Handle>,
}
impl<R, E> MemoryService<R, E>
where
    R: Send
        + Sync
        + RagIngestRepository
        + RagSearchRepository
        + RagDiagnosticsRepository
        + Clone
        + 'static,
    E: RagEmbeddingRuntime + RagGraphRuntime + RagSearchRuntime,
{
    pub fn new(rag: RagService<R, E>) -> Self {
        let store = rag.memories.clone().expect("host memory repository");
        Self {
            rag,
            store,
            index_runtime: tokio::runtime::Handle::try_current().ok(),
        }
    }
    pub fn rag_service(&self) -> RagService<R, E> {
        self.rag.clone()
    }
    pub fn owned_record(&self, user: &str, code: &str) -> Result<MemoryRecord, MemoryError> {
        self.store
            .get(user, code)?
            .ok_or_else(|| MemoryError::NotFound("memory.not_found".into()))
    }
    pub fn get(&self, c: &MemoryRunContext, code: &str) -> Result<MemoryRecord, MemoryError> {
        let r = self
            .store
            .get(&c.user_code, code)?
            .ok_or_else(|| MemoryError::NotFound("memory.not_found".into()))?;
        self.authorize(c, r.scope, &r.scope_code)?;
        Ok(r)
    }
    pub fn query(
        &self,
        c: &MemoryRunContext,
        q: MemoryRecordQuery,
    ) -> Result<MemoryRecordPage, MemoryError> {
        if q.current == 0 || q.current > 1_000_000 || q.size == 0 || q.size > 100 {
            return Err(MemoryError::Validation("memory.pagination_invalid".into()));
        }
        self.authorize(c, q.scope, &q.scope_code)?;
        self.store.query(&c.user_code, q)
    }
    fn authorize(
        &self,
        c: &MemoryRunContext,
        scope: MemoryScope,
        code: &str,
    ) -> Result<(), MemoryError> {
        if c.user_code.is_empty() {
            return Err(MemoryError::Forbidden("memory.scope_invalid".into()));
        }
        let valid = match scope {
            MemoryScope::User => code == c.user_code,
            MemoryScope::Space => !c.workspace_code.is_empty() && code == c.workspace_code,
            MemoryScope::WorkDomain => self.store.domain_exists(code)?,
        };
        if valid {
            Ok(())
        } else {
            Err(MemoryError::Forbidden("memory.scope_invalid".into()))
        }
    }
    pub async fn save(
        &self,
        c: MemoryRunContext,
        cmd: MemorySaveCommand,
    ) -> Result<MemorySaveResponse, MemoryError> {
        let expected = cmd.version;
        let mut r = if let Some(code) = &cmd.memory_code {
            let r = self.get(&c, code)?;
            if expected != Some(r.version) {
                return Err(MemoryError::Conflict("memory.version_conflict".into()));
            }
            r
        } else {
            if expected.is_some() {
                return Err(MemoryError::Validation("memory.version_invalid".into()));
            }
            let scope = cmd.scope.unwrap_or(MemoryScope::User);
            let scope_code = cmd.scope_code.clone().unwrap_or_else(|| match scope {
                MemoryScope::User => c.user_code.clone(),
                MemoryScope::Space => c.workspace_code.clone(),
                MemoryScope::WorkDomain => DEFAULT_WORK_DOMAIN_CODE.into(),
            });
            self.authorize(&c, scope, &scope_code)?;
            let text = cmd.text.clone().unwrap_or_default();
            let semantic_key = cmd
                .semantic_key
                .clone()
                .unwrap_or_else(|| format!("{:x}", Sha256::digest(text.as_bytes())));
            let code = format!("memory_{}", eliteforge_agent_core::RunId::new_v4());
            if let Some(existing) =
                self.store
                    .find_semantic(&c.user_code, scope, &scope_code, &semantic_key)?
            {
                if existing.text == text {
                    return Ok(MemorySaveResponse {
                        saved: true,
                        searchable: existing.index_status == "Ready",
                        action: "DuplicateSkipped".into(),
                        record: existing,
                    });
                }
                return Err(MemoryError::Conflict("memory.semantic_conflict".into()));
            }
            let now = chrono::Utc::now().timestamp_millis();
            MemoryRecord {
                memory_code: code,
                user_code: c.user_code.clone(),
                scope,
                scope_code,
                title: String::new(),
                text,
                enabled: true,
                version: 0,
                source_refs: cmd.source_refs,
                source_label: cmd.source_label,
                expires_at: cmd.expires_at,
                semantic_key,
                created_at: now,
                updated_at: now,
                index_status: "Pending".into(),
                index_error: None,
            }
        };
        if let Some(text) = cmd.text {
            r.text = text;
        }
        if r.text.trim().is_empty() || r.text.len() > 2 * 1024 * 1024 {
            return Err(MemoryError::Validation("memory.text_invalid".into()));
        }
        if let Some(title) = cmd.title {
            r.title = title;
        }
        if r.title.len() > 1000 {
            return Err(MemoryError::Validation("memory.title_invalid".into()));
        }
        if let Some(enabled) = cmd.enabled {
            r.enabled = enabled;
        }
        r.version = r
            .version
            .checked_add(1)
            .ok_or_else(|| MemoryError::Conflict("memory.version_invalid".into()))?;
        r.updated_at = chrono::Utc::now().timestamp_millis();
        let task = if r.enabled {
            Some(self.rag.build_record_task(RagIngestRequest{
            corpus_id:"memory-card".into(),workspace_code:c.workspace_code.clone(),
            access_context:RagIngestAccessContext{user_code:c.user_code.clone(),workspace_code:c.workspace_code.clone(),agent_code:r.scope_code.clone()},
            target_scope:RagIngestScope{scope_type:r.scope.rag_type().into(),scope_code:r.scope_code.clone()},
            source:RagIngestSource{code:r.memory_code.clone(),version:"v1".into()},
            payload:json!({"title":r.title,"text":r.text,"metadata":{"recordVersion":r.version,"sourceRefs":r.source_refs,"sourceLabel":r.source_label},"domainFields":{"expiresAt":r.expires_at}}),
        },None)?)
        } else {
            None
        };
        if let Err(error) = self.store.save(&r, expected, task.as_ref()) {
            if expected.is_none() && matches!(error, MemoryError::Conflict(_)) {
                if let Some(existing) = self.store.find_semantic(
                    &c.user_code,
                    r.scope,
                    &r.scope_code,
                    &r.semantic_key,
                )? {
                    if existing.text == r.text {
                        return Ok(MemorySaveResponse {
                            saved: true,
                            searchable: existing.enabled && existing.index_status == "Ready",
                            action: "DuplicateSkipped".into(),
                            record: existing,
                        });
                    }
                }
            }
            return Err(error);
        }
        if let Some(task) = task {
            let rag = self.rag.clone();
            let code = r.memory_code.clone();
            let work = async move {
                if let Err(error) = rag.execute_record_task(task).await {
                    tracing::warn!(memory_code=%code,%error,"Memory content saved; indexing requires retry");
                }
            };
            if let Some(runtime) = &self.index_runtime {
                let _runtime=runtime.enter();
                crate::shared::auth_holder::AuthHolder::spawn(work);
            } else {
                crate::shared::auth_holder::AuthHolder::spawn(work);
            }
        }
        let current = self.get(&c, &r.memory_code)?;
        if current.version != r.version {
            return Err(MemoryError::Conflict("memory.version_conflict".into()));
        }
        Ok(MemorySaveResponse {
            saved: true,
            searchable: current.index_status == "Ready",
            action: if expected.is_some() {
                "Updated"
            } else {
                "Created"
            }
            .into(),
            record: current,
        })
    }
    pub async fn delete(
        &self,
        c: MemoryRunContext,
        cmd: MemoryDeleteCommand,
    ) -> Result<MemoryDeleteResponse, MemoryError> {
        self.get(&c, &cmd.memory_code)?;
        self.store
            .delete(&c.user_code, &cmd.memory_code, cmd.version)?;
        Ok(MemoryDeleteResponse {
            memory_code: cmd.memory_code,
            deleted: true,
        })
    }
    pub async fn search(
        &self,
        c: MemoryRunContext,
        cmd: MemorySearchCommand,
    ) -> Result<MemorySearchResponse, MemoryError> {
        self.search_in_run(c, cmd, None).await
    }
    pub fn recallable_versions(
        &self,
        c: &MemoryRunContext,
    ) -> Result<std::collections::BTreeSet<(String, u32)>, MemoryError> {
        Ok(self
            .store
            .recallable(&RagIdentity {
                user_code: c.user_code.clone(),
                workspace_code: c.workspace_code.clone(),
                agent_code: String::new(),
            })?
            .into_iter()
            .map(|r| (r.memory_code, r.version))
            .collect())
    }
    pub async fn search_in_run(
        &self,
        c: MemoryRunContext,
        cmd: MemorySearchCommand,
        run: Option<&eliteforge_agent_core::RunContext>,
    ) -> Result<MemorySearchResponse, MemoryError> {
        let mut filters = json!({});
        if let Some(scope) = cmd.scope {
            filters["scopeType"] = json!(scope.rag_type());
            if let Some(code) = &cmd.scope_code {
                self.authorize(&c, scope, code)?;
                filters["scopeCode"] = json!(code);
            }
        } else if let Some(code) = &cmd.scope_code {
            filters["scopeCode"] = json!(code);
        }
        let result = self
            .rag
            .search(
                &rag_access::MemoryOnlyAccess,
                &RagIdentity {
                    user_code: c.user_code.clone(),
                    workspace_code: c.workspace_code.clone(),
                    agent_code: String::new(),
                },
                RagQueryInput {
                    query: cmd.query,
                    corpus_ids: vec!["memory-card".into()],
                    filters,
                    limit: cmd.limit.unwrap_or(8),
                },
                run,
            )
            .await?;
        let mut items = Vec::new();
        let current = self.store.recallable(&RagIdentity {
            user_code: c.user_code,
            workspace_code: c.workspace_code,
            agent_code: String::new(),
        })?;
        for hit in result.evidence {
            if items
                .iter()
                .any(|item: &MemoryItem| item.memory_code == hit.source_code)
            {
                continue;
            }
            if let Some(r) = current.iter().find(|r| {
                r.memory_code == hit.source_code
                    && hit.source["publication"]["recordVersion"].as_u64() == Some(r.version as u64)
            }) {
                items.push(MemoryItem {
                    memory_code: r.memory_code.clone(),
                    scope: r.scope,
                    scope_code: r.scope_code.clone(),
                    title: r.title.clone(),
                    text: hit.text,
                    source_refs: r.source_refs.clone(),
                    source_label: r.source_label.clone(),
                    version: r.version,
                });
            }
        }
        Ok(MemorySearchResponse {
            items,
            warnings: result
                .warnings
                .into_iter()
                .filter(|w| w != "当前授权及已发布范围内没有命中证据")
                .collect(),
        })
    }
}
