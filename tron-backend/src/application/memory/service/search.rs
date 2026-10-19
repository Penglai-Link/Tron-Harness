use serde_json::{Map, Value, json};

use crate::application::rag::{
    RagDiagnosticsRepository, RagEmbeddingRuntime, RagIngestRepository, RagSearchContextRequest,
    RagSearchRepository, RagSearchRequest, RagSearchResult, RagSearchRuntime,
    RagSearchTraceRequest,
};

use super::{DEFAULT_CONTEXT_TOKEN_BUDGET, MEMORY_RETURN_FIELDS, MemoryService};
use crate::application::memory::{
    code::{
        MEMORY_SOURCE_VERSION, MemoryLocator, MemoryScope, SearchMode, access_context,
        decode_memory_code, encode_memory_code, normalize_search_mode, normalized_optional,
        validate_known_scope_code, validate_limit, validate_locator, validate_scope_code,
    },
    dto::{
        MemoryContextPack, MemoryContextPackItem, MemoryRunContext, MemorySearchCommand,
        MemorySearchResponse,
    },
    error::MemoryError,
    payload::{
        agent_facing_metadata, context_pack_from_rag, current_matches_filters,
        memory_item_from_current, memory_item_from_hit, merge_context_pack_search_responses,
    },
};

impl<R, E> MemoryService<R, E>
where
    R: Send + Sync + RagIngestRepository + RagSearchRepository + RagDiagnosticsRepository + Clone,
    E: RagEmbeddingRuntime + crate::application::rag::RagGraphRuntime + RagSearchRuntime,
{
    pub async fn search(
        &self,
        context: MemoryRunContext,
        command: MemorySearchCommand,
    ) -> Result<MemorySearchResponse, MemoryError> {
        let mode = normalize_search_mode(command.mode.as_deref())?;
        let limit = validate_limit(command.limit.unwrap_or(10))?;
        let token_budget = command.token_budget.unwrap_or(DEFAULT_CONTEXT_TOKEN_BUDGET);
        if token_budget == 0 {
            return Err(MemoryError::Validation(
                "tokenBudget must be greater than 0".into(),
            ));
        }
        let query = normalized_optional(command.query.as_deref());
        let locator = command
            .memory_code
            .as_deref()
            .map(decode_memory_code)
            .transpose()?;

        if let Some(locator) = &locator {
            validate_locator(locator, &context)?;
        }

        if let (Some(locator), None) = (&locator, &query) {
            return self
                .search_current(context, locator, &command, &mode, token_budget)
                .await;
        }

        if mode == SearchMode::Search {
            return self
                .execute_search(
                    &context,
                    &command,
                    query.unwrap_or_default(),
                    locator.as_ref(),
                    "RESULT_LIST",
                    None,
                    limit,
                    token_budget,
                )
                .await;
        }

        let baseline = self
            .execute_search(
                &context,
                &command,
                String::new(),
                locator.as_ref(),
                "MEMORY_CONTEXT_PACK",
                Some(true),
                limit,
                token_budget,
            )
            .await?;
        let related = if let Some(query) = query {
            Some(
                self.execute_search(
                    &context,
                    &command,
                    query,
                    locator.as_ref(),
                    "RESULT_LIST",
                    None,
                    limit,
                    token_budget,
                )
                .await?,
            )
        } else {
            None
        };
        Ok(merge_context_pack_search_responses(
            baseline,
            related,
            limit,
            token_budget,
        ))
    }

    async fn search_current(
        &self,
        context: MemoryRunContext,
        locator: &MemoryLocator,
        command: &MemorySearchCommand,
        mode: &SearchMode,
        token_budget: usize,
    ) -> Result<MemorySearchResponse, MemoryError> {
        let Some(current) = self.load_current_optional(&context, locator).await? else {
            return Ok(empty_search_response());
        };
        if !current_matches_filters(&current, command, mode) {
            return Ok(empty_search_response());
        }
        let item = memory_item_from_current(&current);
        let context_pack = (*mode == SearchMode::ContextPack).then(|| {
            let tokens = (current.token_count.max(1) as usize).min(token_budget);
            MemoryContextPack {
                token_count: tokens,
                items: vec![MemoryContextPackItem {
                    memory_code: Some(encode_memory_code(locator)),
                    text: current.text.clone(),
                    tokens,
                    metadata: agent_facing_metadata(&current.metadata),
                }],
            }
        });
        Ok(MemorySearchResponse {
            success: true,
            items: vec![item],
            context_pack,
            count: 1,
            warnings: Vec::new(),
            trace_code: None,
        })
    }

    async fn execute_search(
        &self,
        context: &MemoryRunContext,
        command: &MemorySearchCommand,
        query: String,
        locator: Option<&MemoryLocator>,
        strategy: &str,
        injectable: Option<bool>,
        limit: usize,
        token_budget: usize,
    ) -> Result<MemorySearchResponse, MemoryError> {
        let filters = self.search_filters(context, command, locator, injectable)?;
        let result = self
            .rag
            .search_memory(RagSearchRequest {
                workspace_code: context.workspace_code.clone(),
                query,
                access_context: access_context(context),
                filters,
                limit,
                token_budget,
                context: RagSearchContextRequest {
                    strategy: strategy.into(),
                    return_fields: MEMORY_RETURN_FIELDS
                        .iter()
                        .map(|field| (*field).to_string())
                        .collect(),
                },
                trace: RagSearchTraceRequest {
                    enabled: command.trace.unwrap_or(false),
                },
            })
            .await?;
        Ok(Self::search_response_from_rag(context, result))
    }

    fn search_response_from_rag(
        context: &MemoryRunContext,
        result: RagSearchResult,
    ) -> MemorySearchResponse {
        let items = result
            .hits
            .iter()
            .map(|hit| memory_item_from_hit(hit, context))
            .collect::<Vec<_>>();
        let context_pack = result
            .context_pack
            .map(|pack| context_pack_from_rag(pack, &result.hits, context));
        MemorySearchResponse {
            success: true,
            count: items.len(),
            items,
            context_pack,
            warnings: result.warnings,
            trace_code: result.trace_code,
        }
    }

    fn search_filters(
        &self,
        context: &MemoryRunContext,
        command: &MemorySearchCommand,
        locator: Option<&MemoryLocator>,
        injectable: Option<bool>,
    ) -> Result<Value, MemoryError> {
        let mut filters = Map::new();
        if let Some(locator) = locator {
            filters.insert("scopeType".into(), json!(locator.scope.as_rag()));
            filters.insert("scopeCode".into(), json!(locator.scope_code));
            filters.insert("sourceCode".into(), json!(locator.source_code));
            filters.insert("sourceVersion".into(), json!(locator.source_version));
        } else {
            filters.insert("sourceVersion".into(), json!(MEMORY_SOURCE_VERSION));
            if let Some(scope) = normalized_optional(command.scope.as_deref()) {
                let scope = MemoryScope::parse(&scope)?;
                filters.insert("scopeType".into(), json!(scope.as_rag()));
                if let Some(scope_code) = normalized_optional(command.scope_code.as_deref()) {
                    validate_scope_code(scope, &scope_code, context)?;
                    filters.insert("scopeCode".into(), json!(scope_code));
                }
            } else if let Some(scope_code) = normalized_optional(command.scope_code.as_deref()) {
                validate_known_scope_code(&scope_code, context)?;
                filters.insert("scopeCode".into(), json!(scope_code));
            }
        }
        if let Some(memory_type) = &command.memory_type {
            filters.insert("memoryType".into(), memory_type.clone());
        }
        filters.insert(
            "memoryStatus".into(),
            command
                .memory_status
                .clone()
                .unwrap_or_else(|| json!("ACTIVE")),
        );
        filters.insert(
            "reviewStatus".into(),
            command
                .review_status
                .clone()
                .unwrap_or_else(|| json!("APPROVED")),
        );
        if let Some(injectable) = injectable {
            filters.insert("injectable".into(), json!(injectable));
        } else if let Some(injectable) = &command.injectable {
            filters.insert("injectable".into(), injectable.clone());
        }
        Ok(Value::Object(filters))
    }
}

fn empty_search_response() -> MemorySearchResponse {
    MemorySearchResponse {
        success: true,
        items: Vec::new(),
        context_pack: None,
        count: 0,
        warnings: Vec::new(),
        trace_code: None,
    }
}
