mod search;

use serde_json::{Map, Value, json};

use crate::application::rag::{
    RagDiagnosticsRepository, RagEmbeddingRuntime, RagIngestRepository, RagIngestRequest,
    RagIngestScope, RagIngestSource, RagSearchContextRequest,
    RagSearchRepository, RagSearchRequest, RagSearchRuntime, RagSearchTraceRequest, RagService,
};

use super::{
    code::{
        CurrentMemory, MEMORY_SOURCE_VERSION, MemoryLocator, MemoryScope, PayloadOverrides,
        access_context, bool_from_payload, decode_memory_code, derive_source_code,
        encode_memory_code, ensure_array_or_object_array, insert_optional_i64,
        insert_optional_string, merge_optional_metadata, normalize_delete_mode,
        normalized_optional, normalized_required, object_from_value, resolve_scope_code,
        save_response, string_from_payload, validate_locator,
    },
    dto::{
        MemoryDeleteCommand, MemoryDeleteResponse, MemoryRagWriteResult, MemoryRunContext,
        MemorySaveCommand, MemorySaveResponse,
    },
    error::MemoryError,
};

const MEMORY_CORPUS_ID: &str = "memory-card";
const DEFAULT_CONTEXT_TOKEN_BUDGET: usize = 1200;
const MEMORY_RETURN_FIELDS: &[&str] = &[
    "memoryType",
    "memoryStatus",
    "reviewStatus",
    "injectable",
    "importanceScore",
    "confidenceScore",
    "qualityScore",
    "expiresAt",
    "derivedSourceType",
    "derivedSourceCode",
    "sourceUpdatedAt",
];

#[derive(Clone)]
pub struct MemoryService<R, E> {
    rag: RagService<R, E>,
}

impl<R, E> MemoryService<R, E>
where
    R: Send + Sync + RagIngestRepository + RagSearchRepository + RagDiagnosticsRepository + Clone,
    E: RagEmbeddingRuntime + crate::application::rag::RagGraphRuntime + RagSearchRuntime,
{
    pub fn new(rag: RagService<R, E>) -> Self {
        Self { rag }
    }

    pub fn rag_service(&self) -> RagService<R, E>
    where
        E: Clone,
    {
        self.rag.clone()
    }

    pub async fn save(
        &self,
        context: MemoryRunContext,
        command: MemorySaveCommand,
    ) -> Result<MemorySaveResponse, MemoryError> {
        if let Some(memory_code) = normalized_optional(command.memory_code.as_deref()) {
            self.upsert(context, memory_code, command).await
        } else {
            self.create(context, command).await
        }
    }

    pub async fn delete(
        &self,
        context: MemoryRunContext,
        command: MemoryDeleteCommand,
    ) -> Result<MemoryDeleteResponse, MemoryError> {
        let reason = normalized_required(&command.reason, "reason")?;
        let mode = normalize_delete_mode(command.delete_mode.as_deref())?;
        let locator = decode_memory_code(&command.memory_code)?;
        validate_locator(&locator, &context)?;
        let current = self.load_current_required(&context, &locator).await?;

        let mut metadata = object_from_value(current.metadata.clone())?;
        let original_source_refs = metadata.get("sourceRefs").cloned();
        merge_optional_metadata(&mut metadata, command.metadata.as_ref())?;
        if let Some(source_refs) = original_source_refs {
            metadata.insert("sourceRefs".into(), source_refs);
        } else {
            metadata.remove("sourceRefs");
        }
        if let Some(source_refs) = command.source_refs {
            ensure_array_or_object_array(&source_refs, "sourceRefs")?;
            metadata.insert("deleteSourceRefs".into(), source_refs);
        }
        metadata.insert("deleteReason".into(), json!(reason));
        metadata.insert("deleteMode".into(), json!(mode));
        metadata.insert("deleted".into(), json!(true));

        let payload = self.complete_payload(
            &current,
            PayloadOverrides {
                memory_status: Some(if mode == "ARCHIVE" {
                    "ARCHIVED".into()
                } else {
                    "DISABLED".into()
                }),
                review_status: Some("REJECTED".into()),
                injectable: Some(false),
                metadata: Some(Value::Object(metadata)),
                ..Default::default()
            },
            false,
        )?;
        let result = self
            .ingest_payload(&context, &locator, payload.clone())
            .await?;
        Ok(MemoryDeleteResponse {
            success: true,
            action: mode,
            memory_code: encode_memory_code(&locator),
            scope: locator.scope.as_public().into(),
            scope_code: locator.scope_code,
            memory_type: string_from_payload(&payload, "memoryType"),
            memory_status: string_from_payload(&payload, "memoryStatus"),
            review_status: string_from_payload(&payload, "reviewStatus"),
            injectable: bool_from_payload(&payload, "injectable"),
            deleted: true,
            delete_reason: reason,
            warnings: result.rag.warnings,
        })
    }

    async fn create(
        &self,
        context: MemoryRunContext,
        command: MemorySaveCommand,
    ) -> Result<MemorySaveResponse, MemoryError> {
        let text = normalized_optional(command.text.as_deref())
            .ok_or_else(|| MemoryError::Validation("text is required".into()))?;
        let scope = MemoryScope::parse(command.scope.as_deref().unwrap_or("USER"))?;
        let scope_code = resolve_scope_code(scope, command.scope_code.as_deref(), &context)?;
        let locator = MemoryLocator {
            scope,
            scope_code: scope_code.clone(),
            source_code: derive_source_code(
                command.semantic_key.as_deref(),
                scope,
                Some(scope_code.as_str()),
                &text,
            )?,
            source_version: MEMORY_SOURCE_VERSION.into(),
        };
        let payload = self.complete_payload(
            &CurrentMemory::blank(&locator, text),
            PayloadOverrides::from_save(command)?,
            true,
        )?;
        let result = self
            .ingest_payload(&context, &locator, payload.clone())
            .await?;
        Ok(save_response("CREATED", locator, payload, result.rag))
    }

    async fn upsert(
        &self,
        context: MemoryRunContext,
        memory_code: String,
        command: MemorySaveCommand,
    ) -> Result<MemorySaveResponse, MemoryError> {
        let locator = decode_memory_code(&memory_code)?;
        validate_locator(&locator, &context)?;
        let overrides = PayloadOverrides::from_save(command)?;
        let (current, create, action) = if let Some(current) =
            self.load_current_optional(&context, &locator).await?
        {
            (current, false, "UPDATED")
        } else {
            let text = overrides
                .text
                .clone()
                .and_then(|value| normalized_optional(Some(value.as_str())))
                .ok_or_else(|| {
                    MemoryError::NotFound(
                        "memoryCode does not match an existing memory and text is required to create it".into(),
                    )
                })?;
            (CurrentMemory::blank(&locator, text), true, "CREATED")
        };
        let payload = self.complete_payload(&current, overrides, create)?;
        let result = self
            .ingest_payload(&context, &locator, payload.clone())
            .await?;
        Ok(save_response(action, locator, payload, result.rag))
    }

    async fn ingest_payload(
        &self,
        context: &MemoryRunContext,
        locator: &MemoryLocator,
        payload: Value,
    ) -> Result<MemorySaveResponse, MemoryError> {
        let result = self
            .rag
            .ingest(RagIngestRequest {
                corpus_id: MEMORY_CORPUS_ID.into(),
                workspace_code: context.workspace_code.clone(),
                access_context: access_context(context),
                target_scope: RagIngestScope {
                    scope_type: locator.scope.as_rag().into(),
                    scope_code: locator.scope_code.clone(),
                },
                source: RagIngestSource {
                    code: locator.source_code.clone(),
                    version: locator.source_version.clone(),
                },
                payload: payload.clone(),
            })
            .await?;
        Ok(save_response(
            "UPSERTED",
            locator.clone(),
            payload,
            MemoryRagWriteResult {
                status: result.status,
                text_unit_count: result.text_unit_count,
                fulltext_status: result.fulltext_status,
                embedding_status: result.embedding_status,
                warnings: result.warnings,
            },
        ))
    }

    fn complete_payload(
        &self,
        current: &CurrentMemory,
        overrides: PayloadOverrides,
        create: bool,
    ) -> Result<Value, MemoryError> {
        let text = overrides
            .text
            .or_else(|| Some(current.text.clone()))
            .and_then(|value| normalized_optional(Some(value.as_str())))
            .ok_or_else(|| MemoryError::Validation("text is required".into()))?;
        let mut payload = Map::new();
        payload.insert(
            "source".into(),
            json!({
                "code": current.locator.source_code,
                "version": current.locator.source_version
            }),
        );
        insert_optional_string(
            &mut payload,
            "title",
            overrides.title.or(current.title.clone()),
        );
        payload.insert("text".into(), json!(text));
        let mut domain_fields = Map::new();
        domain_fields.insert("version".into(), json!(MEMORY_SOURCE_VERSION));
        insert_optional_string(
            &mut domain_fields,
            "memoryType",
            overrides.memory_type.or(current.memory_type.clone()),
        );
        insert_optional_string(
            &mut domain_fields,
            "memoryStatus",
            overrides
                .memory_status
                .or(current.memory_status.clone())
                .or_else(|| create.then(|| "ACTIVE".into())),
        );
        insert_optional_string(
            &mut domain_fields,
            "reviewStatus",
            overrides
                .review_status
                .or(current.review_status.clone())
                .or_else(|| create.then(|| "APPROVED".into())),
        );
        if let Some(value) = overrides
            .injectable
            .or(current.injectable)
            .or_else(|| create.then_some(true))
        {
            domain_fields.insert("injectable".into(), json!(value));
        }
        insert_optional_i64(
            &mut domain_fields,
            "importanceScore",
            overrides.importance_score.or(current.importance_score),
        );
        insert_optional_i64(
            &mut domain_fields,
            "confidenceScore",
            overrides.confidence_score.or(current.confidence_score),
        );
        insert_optional_i64(
            &mut domain_fields,
            "qualityScore",
            overrides.quality_score.or(current.quality_score),
        );
        insert_optional_i64(
            &mut domain_fields,
            "expiresAt",
            overrides.expires_at.or(current.expires_at),
        );
        insert_optional_string(
            &mut domain_fields,
            "derivedSourceType",
            overrides
                .derived_source_type
                .or(current.derived_source_type.clone()),
        );
        insert_optional_string(
            &mut domain_fields,
            "derivedSourceCode",
            overrides
                .derived_source_code
                .or(current.derived_source_code.clone()),
        );
        if !is_active_and_approved(&domain_fields) {
            domain_fields.insert("injectable".into(), json!(false));
        }
        payload.insert("domainFields".into(), Value::Object(domain_fields));
        let mut metadata = object_from_value(current.metadata.clone())?;
        merge_optional_metadata(&mut metadata, overrides.metadata.as_ref())?;
        metadata.remove("pinLevel");
        metadata.remove("pin_level");
        if let Some(source_refs) = overrides.source_refs {
            ensure_array_or_object_array(&source_refs, "sourceRefs")?;
            metadata.insert("sourceRefs".into(), source_refs);
        }
        if let Some(semantic_key) = overrides.semantic_key {
            metadata.insert("semanticKey".into(), json!(semantic_key));
        }
        if !metadata.is_empty() {
            payload.insert("metadata".into(), Value::Object(metadata));
        }
        Ok(Value::Object(payload))
    }

    async fn load_current_required(
        &self,
        context: &MemoryRunContext,
        locator: &MemoryLocator,
    ) -> Result<CurrentMemory, MemoryError> {
        self.load_current_optional(context, locator)
            .await?
            .ok_or_else(|| {
                MemoryError::NotFound("memoryCode does not match an existing memory".into())
            })
    }

    async fn load_current_optional(
        &self,
        context: &MemoryRunContext,
        locator: &MemoryLocator,
    ) -> Result<Option<CurrentMemory>, MemoryError> {
        let result = self
            .rag
            .search_memory(RagSearchRequest {
                workspace_code: context.workspace_code.clone(),
                query: String::new(),
                access_context: access_context(context),
                filters: json!({
                    "scopeType": locator.scope.as_rag(),
                    "scopeCode": locator.scope_code.clone(),
                    "sourceCode": locator.source_code.clone(),
                    "sourceVersion": locator.source_version.clone(),
                }),
                limit: 100,
                token_budget: 1,
                context: RagSearchContextRequest {
                    strategy: "RESULT_LIST".into(),
                    return_fields: MEMORY_RETURN_FIELDS
                        .iter()
                        .map(|field| (*field).to_string())
                        .collect(),
                },
                trace: RagSearchTraceRequest { enabled: false },
            })
            .await?;
        result
            .hits
            .iter()
            .find(|hit| {
                hit.source_code == locator.source_code
                    && hit.source_version == locator.source_version
            })
            .map(|hit| CurrentMemory::from_hit(locator.clone(), hit))
            .transpose()
    }
}

fn is_active_and_approved(domain_fields: &Map<String, Value>) -> bool {
    domain_fields
        .get("memoryStatus")
        .and_then(Value::as_str)
        .is_some_and(|value| value == "ACTIVE")
        && domain_fields
            .get("reviewStatus")
            .and_then(Value::as_str)
            .is_some_and(|value| value == "APPROVED")
}
