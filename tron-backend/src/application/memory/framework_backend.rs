//! Host-bound namespaces adapt the default original store and RAG service to Framework.
//! Third-party implementations can provide LongTermMemory at this boundary.
use super::*;
use crate::application::rag::*;
use async_trait::async_trait;
use eliteforge_agent_core as fw;
use sha2::{Digest, Sha256};
use std::sync::Arc;
#[derive(Clone)]
pub struct MemoryBackendBinding {
    pub namespace: String,
    pub backend: Arc<dyn fw::LongTermMemory>,
    pub diagnostics: Arc<std::sync::Mutex<Vec<String>>>,
}
struct DefaultMemoryBackend<R, E> {
    service: MemoryService<R, E>,
    context: MemoryRunContext,
    namespace: String,
    scope: Option<MemoryScope>,
    scope_code: Option<String>,
    diagnostics: Arc<std::sync::Mutex<Vec<String>>>,
    run: Option<fw::RunContext>,
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
    pub fn bind_backend(
        &self,
        context: MemoryRunContext,
        scope: Option<MemoryScope>,
        scope_code: Option<String>,
    ) -> MemoryBackendBinding {
        self.bind_backend_in_run(context, scope, scope_code, None)
    }
    pub fn bind_backend_in_run(
        &self,
        context: MemoryRunContext,
        scope: Option<MemoryScope>,
        scope_code: Option<String>,
        run: Option<fw::RunContext>,
    ) -> MemoryBackendBinding {
        let diagnostics = Arc::new(std::sync::Mutex::new(vec![]));
        let namespace = format!(
            "tron-memory:{:x}",
            Sha256::digest(
                format!(
                    "{}:{}:{scope:?}:{scope_code:?}",
                    context.user_code, context.workspace_code
                )
                .as_bytes()
            )
        );
        let backend = Arc::new(DefaultMemoryBackend {
            service: self.clone(),
            context,
            namespace: namespace.clone(),
            scope,
            scope_code,
            diagnostics: diagnostics.clone(),
            run,
        });
        MemoryBackendBinding {
            namespace,
            backend,
            diagnostics,
        }
    }
}
fn map_error(e: MemoryError) -> fw::MemoryError {
    match e {
        MemoryError::NotFound(_) => fw::MemoryError::NotFound,
        MemoryError::Conflict(_) => fw::MemoryError::Conflict,
        MemoryError::Validation(s) | MemoryError::Forbidden(s) => fw::MemoryError::Invalid(s),
        e => fw::MemoryError::Backend(e.to_string()),
    }
}
impl<R, E> DefaultMemoryBackend<R, E>
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
    fn check(&self, namespace: &str) -> Result<(), fw::MemoryError> {
        if namespace == self.namespace {
            Ok(())
        } else {
            Err(fw::MemoryError::Invalid(
                "namespace is outside the host scope".into(),
            ))
        }
    }
    fn write_scope(&self) -> (MemoryScope, String) {
        let scope = self.scope.unwrap_or(MemoryScope::User);
        let code = self.scope_code.clone().unwrap_or_else(|| match scope {
            MemoryScope::User => self.context.user_code.clone(),
            MemoryScope::Space => self.context.workspace_code.clone(),
            MemoryScope::WorkDomain => DEFAULT_WORK_DOMAIN_CODE.into(),
        });
        (scope, code)
    }
    fn find(&self, id: &str) -> Result<MemoryRecord, fw::MemoryError> {
        let (scope, code) = self.write_scope();
        let record = self
            .service
            .store
            .find_semantic(
                &self.context.user_code,
                scope,
                &code,
                &format!("framework:{id}"),
            )
            .map_err(map_error)?;
        let r = match record {
            Some(r) => self.service.get(&self.context, &r.memory_code),
            None => self.service.get(&self.context, id),
        }
        .map_err(map_error)?;
        if self.scope.is_some_and(|s| s != r.scope)
            || self.scope_code.as_ref().is_some_and(|c| *c != r.scope_code)
        {
            return Err(fw::MemoryError::NotFound);
        }
        if !r.enabled
            || r.expires_at
                .is_some_and(|e| e <= chrono::Utc::now().timestamp_millis())
        {
            return Err(fw::MemoryError::NotFound);
        }
        Ok(r)
    }
    fn entry(&self, r: MemoryRecord, id: String) -> fw::MemoryEntry {
        fw::MemoryEntry {
            namespace: self.namespace.clone(),
            version: format!("{}:{}", r.memory_code, r.version),
            updated_at_ms: r.updated_at as u64,
            value: fw::MemoryWrite {
                id,
                content: r.text,
                source: fw::MemorySource {
                    id: serde_json::to_string(&serde_json::json!({"sourceLabel":r.source_label,"references":r.source_refs.iter().map(|value|{
                        let mut reference=serde_json::Map::new();
                        for key in ["sourceCode","workspaceCode","version","kind","sessionCode","runCode","userCode","id","type","locator"] {if let Some(value)=value.get(key){reference.insert(key.into(),value.clone());}}
                        serde_json::Value::Object(reference)
                    }).collect::<Vec<_>>()})).unwrap_or_else(|_|r.source_label.clone()),
                    version: Some(r.version.to_string()),
                },
                expires_at_ms: r.expires_at.map(|v| v as u64),
            },
        }
    }
}
#[async_trait]
impl<R, E> fw::LongTermMemory for DefaultMemoryBackend<R, E>
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
    async fn get(&self, namespace: &str, id: &str) -> Result<fw::MemoryEntry, fw::MemoryError> {
        self.check(namespace)?;
        Ok(self.entry(self.find(id)?, id.into()))
    }
    async fn search(
        &self,
        namespace: &str,
        q: &fw::MemoryQuery,
    ) -> Result<Vec<fw::MemoryEntry>, fw::MemoryError> {
        self.check(namespace)?;
        if q.max_bytes == 0 || q.limit == 0 {
            return Ok(vec![]);
        }
        let result = self
            .service
            .search_in_run(
                self.context.clone(),
                MemorySearchCommand {
                    query: q.text.clone(),
                    scope: self.scope,
                    scope_code: self.scope_code.clone(),
                    limit: Some(q.limit.min(100)),
                },
                self.run.as_ref(),
            )
            .await
            .map_err(map_error)?;
        *self
            .diagnostics
            .lock()
            .map_err(|_| fw::MemoryError::Backend("memory diagnostics unavailable".into()))? =
            result.warnings.clone();
        if !result.warnings.is_empty() {
            tracing::warn!(warnings=?result.warnings,"Memory backend retrieval diagnostics");
        }
        let mut entries = vec![];
        let mut bytes = 0;
        for hit in result.items {
            let mut r = match self.find(&hit.memory_code) {
                Ok(r) if r.version == hit.version => r,
                Ok(_) | Err(fw::MemoryError::NotFound) => continue,
                Err(e) => return Err(e),
            };
            r.text = hit.text;
            let id = r
                .semantic_key
                .strip_prefix("framework:")
                .unwrap_or(&r.memory_code)
                .to_owned();
            let e = self.entry(r, id);
            let size = serde_json::to_vec(&e)
                .map_err(|e| fw::MemoryError::Backend(e.to_string()))?
                .len();
            if bytes + size <= q.max_bytes {
                bytes += size;
                entries.push(e);
            }
        }
        let current=self.service.recallable_versions(&self.context).map_err(map_error)?;
        entries.retain(|entry|entry.version.rsplit_once(':').and_then(|(code,version)|version.parse::<u32>().ok().map(|version|current.contains(&(code.into(),version)))).unwrap_or(false));
        Ok(entries)
    }
    async fn save(
        &self,
        namespace: &str,
        value: &fw::MemoryWrite,
        expected: Option<&str>,
    ) -> Result<fw::MemoryEntry, fw::MemoryError> {
        self.check(namespace)?;
        if value.id.trim().is_empty() {
            return Err(fw::MemoryError::Invalid("memory id required".into()));
        }
        let existing = match self.find(&value.id) {
            Ok(r) => Some(r),
            Err(fw::MemoryError::NotFound) => None,
            Err(e) => return Err(e),
        };
        let current = existing
            .as_ref()
            .map(|r| format!("{}:{}", r.memory_code, r.version));
        if current.as_deref() != expected {
            return Err(fw::MemoryError::Conflict);
        }
        let (scope, code) = self.write_scope();
        if expected.is_none() {
            if let Some(old) = self
                .service
                .store
                .find_semantic(
                    &self.context.user_code,
                    scope,
                    &code,
                    &format!("framework:{}", value.id),
                )
                .map_err(map_error)?
            {
                if old
                    .expires_at
                    .is_some_and(|e| e <= chrono::Utc::now().timestamp_millis())
                {
                    self.service
                        .delete(
                            self.context.clone(),
                            MemoryDeleteCommand {
                                memory_code: old.memory_code,
                                version: old.version,
                            },
                        )
                        .await
                        .map_err(map_error)?;
                } else {
                    return Err(fw::MemoryError::Conflict);
                }
            }
        }
        let r = self
            .service
            .save(
                self.context.clone(),
                MemorySaveCommand {
                    memory_code: existing.as_ref().map(|r| r.memory_code.clone()),
                    version: existing.map(|r| r.version),
                    scope: Some(scope),
                    scope_code: Some(code),
                    text: Some(value.content.clone()),
                    semantic_key: Some(format!("framework:{}", value.id)),
                    source_label: "Framework".into(),
                    source_refs: vec![
                        serde_json::to_value(&value.source)
                            .map_err(|e| fw::MemoryError::Invalid(e.to_string()))?,
                    ],
                    expires_at: value
                        .expires_at_ms
                        .map(|v| {
                            i64::try_from(v)
                                .map_err(|_| fw::MemoryError::Invalid("expiry out of range".into()))
                        })
                        .transpose()?,
                    ..Default::default()
                },
            )
            .await
            .map_err(map_error)?;
        Ok(self.entry(r.record, value.id.clone()))
    }
    async fn delete(
        &self,
        namespace: &str,
        id: &str,
        expected: &str,
    ) -> Result<(), fw::MemoryError> {
        self.check(namespace)?;
        let r = self.find(id)?;
        if format!("{}:{}", r.memory_code, r.version) != expected {
            return Err(fw::MemoryError::Conflict);
        }
        self.service
            .delete(
                self.context.clone(),
                MemoryDeleteCommand {
                    memory_code: r.memory_code,
                    version: r.version,
                },
            )
            .await
            .map_err(map_error)?;
        Ok(())
    }
}
