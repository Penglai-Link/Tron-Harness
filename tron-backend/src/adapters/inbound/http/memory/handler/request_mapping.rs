async fn mutate_memory_status(
    workspaces: &WorkspaceService,
    memory: &AppMemoryService,
    domains: &MemoryWorkDomainService,
    auth: &RequestAuthContext,
    workspace_code: &str,
    memory_code: &str,
    delete_mode: &str,
    request: Option<MemoryDeleteRequest>,
) -> Response {
    let context = match context_from_memory_code(domains, auth, workspace_code, memory_code) {
        Ok(context) => context,
        Err(error) => return memory_error_response(error),
    };
    let reason = request
        .and_then(|request| normalize_optional(request.reason.as_deref()))
        .unwrap_or_else(|| format!("web requested {}", delete_mode.to_ascii_lowercase()));
    match memory
        .delete(
            context,
            MemoryDeleteCommand {
                memory_code: memory_code.to_string(),
                reason,
                delete_mode: Some(delete_mode.into()),
                source_refs: None,
                metadata: None,
            },
        )
        .await
    {
        Ok(_) => {
            match record_by_code(
                workspaces,
                memory,
                domains,
                auth,
                workspace_code,
                memory_code,
            )
            .await
            {
                Ok(record) => Json(ApiResponse::success(record)).into_response(),
                Err(error) => memory_error_response(error),
            }
        }
        Err(error) => memory_error_response(error),
    }
}

async fn record_by_code(
    workspaces: &WorkspaceService,
    memory: &AppMemoryService,
    domains: &MemoryWorkDomainService,
    auth: &RequestAuthContext,
    workspace_code: &str,
    memory_code: &str,
) -> Result<MemoryRecordResponse, MemoryError> {
    let workspace = workspace_detail(workspaces, workspace_code, auth)?;
    let context = context_from_memory_code(domains, auth, &workspace.workspace_code, memory_code)?;
    let result = memory
        .search(
            context,
            MemorySearchCommand {
                query: None,
                mode: Some("SEARCH".into()),
                memory_code: Some(memory_code.to_string()),
                scope: None,
                scope_code: None,
                memory_type: None,
                memory_status: Some(all_memory_statuses()),
                review_status: Some(all_review_statuses()),
                injectable: None,
                limit: Some(1),
                token_budget: None,
                trace: Some(false),
            },
        )
        .await?;
    let item = result
        .items
        .into_iter()
        .next()
        .ok_or_else(|| MemoryError::NotFound("memory was not found".into()))?;
    let catalog = response_catalog(domains, auth, &workspace)?;
    record_from_item(item, &catalog)
}

fn workspace_detail(
    workspaces: &WorkspaceService,
    workspace_code: &str,
    auth: &RequestAuthContext,
) -> Result<WorkspaceDetail, MemoryError> {
    let workspace_code = normalize_required(workspace_code, "workspaceCode")?;
    let workspace = workspaces
        .get_workspace(&workspace_code)
        .map_err(|_| MemoryError::NotFound("workspace was not found".into()))?;
    if user_has_workspace_access(
        workspaces,
        &workspace.workspace_code,
        &auth.user().user_code,
    ) {
        Ok(workspace)
    } else {
        Err(MemoryError::Forbidden(
            "user cannot access the requested workspace".into(),
        ))
    }
}

fn user_has_workspace_access(
    workspaces: &WorkspaceService,
    workspace_code: &str,
    user_code: &str,
) -> bool {
    workspaces
        .user_has_effective_access(workspace_code, user_code)
        .unwrap_or(false)
}

fn target_from_query(
    domains: &MemoryWorkDomainService,
    auth: &RequestAuthContext,
    workspace: &WorkspaceDetail,
    query: &MemoryRecordListQuery,
) -> Result<TargetScope, MemoryError> {
    let scope = normalize_optional(query.scope.as_deref()).unwrap_or_else(|| "space".into());
    let scope_code = normalize_optional(query.scope_code.as_deref());
    target_scope(domains, auth, workspace, &scope, scope_code.as_deref())
}

fn target_from_mutation(
    domains: &MemoryWorkDomainService,
    auth: &RequestAuthContext,
    workspace: &WorkspaceDetail,
    request: &MemoryRecordMutationRequest,
) -> Result<TargetScope, MemoryError> {
    let scope = normalize_optional(request.scope.as_deref()).unwrap_or_else(|| "space".into());
    let scope_code = normalize_optional(request.scope_code.as_deref());
    target_scope(domains, auth, workspace, &scope, scope_code.as_deref())
}

fn target_scope(
    domains: &MemoryWorkDomainService,
    auth: &RequestAuthContext,
    workspace: &WorkspaceDetail,
    scope: &str,
    scope_code: Option<&str>,
) -> Result<TargetScope, MemoryError> {
    match scope.trim().to_ascii_lowercase().as_str() {
        "space" | "s" | "SPACE" => {
            let code = scope_code.unwrap_or(&workspace.workspace_code);
            if code == workspace.workspace_code {
                Ok(TargetScope::new("SPACE", &workspace.workspace_code))
            } else {
                Err(MemoryError::Forbidden(
                    "space memory scopeCode must match the current workspace".into(),
                ))
            }
        }
        "user" | "u" | "USER" => {
            let code = scope_code.unwrap_or(&auth.user().user_code);
            if code == auth.user().user_code {
                Ok(TargetScope::new("USER", &auth.user().user_code))
            } else {
                Err(MemoryError::Forbidden(
                    "user memory scopeCode must match the current user".into(),
                ))
            }
        }
        "domain" | "work_domain" | "WORK_DOMAIN" | "d" => {
            let code = normalize_required(scope_code.unwrap_or_default(), "scopeCode")?;
            domains.ensure_active_domain(&code)?;
            Ok(TargetScope::new("WORK_DOMAIN", &code))
        }
        _ => Err(MemoryError::Validation(
            "scope must be space, user, or domain".into(),
        )),
    }
}

fn context_from_memory_code(
    domains: &MemoryWorkDomainService,
    auth: &RequestAuthContext,
    workspace_code: &str,
    memory_code: &str,
) -> Result<MemoryRunContext, MemoryError> {
    let (scope, scope_code, _, _) = decode_memory_code_parts(memory_code)?;
    let mut context = memory_context(auth, workspace_code, None);
    match scope.as_str() {
        "USER" if scope_code == auth.user().user_code => Ok(context),
        "SPACE" if scope_code == workspace_code => Ok(context),
        "WORK_DOMAIN" => {
            domains.ensure_active_domain(&scope_code)?;
            context = context.with_work_domain_code(scope_code);
            Ok(context)
        }
        _ => Err(MemoryError::Forbidden(
            "memoryCode is outside the current request scope".into(),
        )),
    }
}

fn memory_context(
    auth: &RequestAuthContext,
    workspace_code: &str,
    work_domain_code: Option<String>,
) -> MemoryRunContext {
    let context = MemoryRunContext::new(auth.user().user_code.clone(), workspace_code.to_string());
    if let Some(work_domain_code) = work_domain_code {
        context.with_work_domain_code(work_domain_code)
    } else {
        context
    }
}

fn search_command_from_query(
    query: &MemoryRecordListQuery,
    target: &TargetScope,
) -> MemorySearchCommand {
    MemorySearchCommand {
        query: normalize_optional(query.query.as_deref()),
        mode: Some("SEARCH".into()),
        memory_code: None,
        scope: Some(target.public_scope.clone()),
        scope_code: Some(target.scope_code.clone()),
        memory_type: normalize_optional(query.memory_type.as_deref()).map(Value::String),
        memory_status: status_filter(query.memory_status.as_deref(), all_memory_statuses),
        review_status: status_filter(query.review_status.as_deref(), all_review_statuses),
        injectable: injectable_filter(query.injectable.as_deref()),
        limit: Some(query.limit.unwrap_or(50).clamp(1, 100)),
        token_budget: None,
        trace: Some(false),
    }
}

fn save_command_from_request(
    memory_code: Option<String>,
    request: MemoryRecordMutationRequest,
    target: &TargetScope,
) -> MemorySaveCommand {
    let metadata = metadata_from_request(&request);
    let derived_source_code = normalize_optional(request.source_label.as_deref());
    MemorySaveCommand {
        memory_type: normalize_optional(request.memory_type.as_deref())
            .or_else(|| Some("FACT".into())),
        text: request
            .text
            .and_then(|value| normalize_optional(Some(&value))),
        memory_code,
        scope: (!target.public_scope.is_empty()).then(|| target.public_scope.clone()),
        scope_code: (!target.scope_code.is_empty()).then(|| target.scope_code.clone()),
        title: request
            .title
            .and_then(|value| normalize_optional(Some(&value))),
        semantic_key: request
            .semantic_key
            .and_then(|value| normalize_optional(Some(&value))),
        memory_status: normalize_optional(request.memory_status.as_deref())
            .or_else(|| Some("ACTIVE".into())),
        review_status: normalize_optional(request.review_status.as_deref())
            .or_else(|| Some("PENDING_REVIEW".into())),
        injectable: Some(request.injectable.unwrap_or(false)),
        importance_score: None,
        confidence_score: None,
        quality_score: None,
        expires_at: None,
        derived_source_type: Some("WEB".into()),
        derived_source_code,
        source_refs: None,
        metadata: Some(metadata),
    }
}

fn metadata_from_request(request: &MemoryRecordMutationRequest) -> Value {
    let tags = request
        .tags
        .iter()
        .filter_map(|tag| normalize_optional(Some(tag)))
        .collect::<Vec<_>>();
    let metadata_items = request
        .metadata
        .iter()
        .filter_map(|item| {
            let label = normalize_optional(item.label.as_deref()).unwrap_or_default();
            let value = normalize_optional(item.value.as_deref()).unwrap_or_default();
            (!label.is_empty() || !value.is_empty())
                .then(|| json!({ "label": label, "value": value }))
        })
        .collect::<Vec<_>>();
    json!({
        "extra": {
            "sourceLabel": normalize_optional(request.source_label.as_deref()).unwrap_or_else(|| "Manual entry".into()),
            "tags": tags,
            "metadataItems": metadata_items
        }
    })
}

fn response_catalog(
    domains: &MemoryWorkDomainService,
    auth: &RequestAuthContext,
    workspace: &WorkspaceDetail,
) -> Result<ResponseCatalog, MemoryError> {
    Ok(ResponseCatalog {
        domains: domains.list_domains(crate::application::memory::MemoryWorkDomainQuery {
            search: None,
            include_disabled: Some(true),
        })?,
        user_code: auth.user().user_code.clone(),
        user_label: user_label(auth),
        workspace_code: workspace.workspace_code.clone(),
        workspace_label: workspace.name.clone(),
    })
}

fn records_from_items(
    items: Vec<MemoryItem>,
    catalog: &ResponseCatalog,
) -> Result<Vec<MemoryRecordResponse>, MemoryError> {
    items
        .into_iter()
        .map(|item| record_from_item(item, catalog))
        .collect()
}

fn record_from_item(
    item: MemoryItem,
    catalog: &ResponseCatalog,
) -> Result<MemoryRecordResponse, MemoryError> {
    let (_, _, source_code, _) = decode_memory_code_parts(&item.memory_code)?;
    let target = catalog.target_for(&item.scope, &item.scope_code);
    let metadata = metadata_object(&item.metadata);
    let semantic_key = string_metadata(&metadata, "semanticKey");
    let tags = metadata
        .get("tags")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    let metadata_items = metadata_items(&metadata);
    let source_label = string_metadata(&metadata, "sourceLabel")
        .or(item.derived_source_code.clone())
        .unwrap_or_else(|| "Manual entry".into());
    Ok(MemoryRecordResponse {
        domain_code: (target.scope == "domain").then(|| item.scope_code.clone()),
        id: item.memory_code.clone(),
        injectable: item.injectable.unwrap_or(false),
        memory_code: item.memory_code,
        memory_status: item.memory_status.unwrap_or_else(|| "ACTIVE".into()),
        memory_type: item.memory_type.unwrap_or_else(|| "FACT".into()),
        metadata: metadata_items,
        review_status: item.review_status.unwrap_or_else(|| "APPROVED".into()),
        scope: target.scope,
        score: item.score.unwrap_or(1.0),
        semantic_key: semantic_key.unwrap_or_default(),
        source_code,
        source_label,
        tags,
        target_code: target.code,
        target_id: target.id,
        target_label: target.label,
        text: item.text.or(item.preview).unwrap_or_default(),
        title: item.title.unwrap_or_else(|| "Untitled memory".into()),
        updated_at: item
            .updated_at
            .map(timestamp_from_millis)
            .unwrap_or_default(),
    })
}

fn metadata_object(value: &Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap_or_default()
}

fn metadata_items(metadata: &Map<String, Value>) -> Vec<MemoryMetadataItemResponse> {
    if let Some(items) = metadata.get("metadataItems").and_then(Value::as_array) {
        return items
            .iter()
            .filter_map(|item| {
                let label = item
                    .get("label")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let value = item
                    .get("value")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                (!label.is_empty() || !value.is_empty()).then(|| MemoryMetadataItemResponse {
                    label: label.to_string(),
                    value: value.to_string(),
                })
            })
            .collect();
    }
    metadata
        .iter()
        .filter(|(key, _)| {
            !matches!(
                key.as_str(),
                "sourceLabel" | "tags" | "semanticKey" | "sourceRefs"
            )
        })
        .filter_map(|(key, value)| primitive_metadata_value(value).map(|value| (key, value)))
        .map(|(key, value)| MemoryMetadataItemResponse {
            label: key.clone(),
            value,
        })
        .collect()
}

fn primitive_metadata_value(value: &Value) -> Option<String> {
    if let Some(value) = value.as_str() {
        Some(value.to_string())
    } else if value.is_number() || value.is_boolean() {
        Some(value.to_string())
    } else {
        None
    }
}

fn string_metadata(metadata: &Map<String, Value>, key: &str) -> Option<String> {
    metadata
        .get(key)
        .and_then(Value::as_str)
        .and_then(|value| normalize_optional(Some(value)))
}

fn status_filter(value: Option<&str>, all_values: fn() -> Value) -> Option<Value> {
    match normalize_optional(value) {
        Some(value) if value.eq_ignore_ascii_case("all") => Some(all_values()),
        Some(value) => Some(Value::String(value)),
        None => Some(all_values()),
    }
}

fn injectable_filter(value: Option<&str>) -> Option<Value> {
    match normalize_optional(value)
        .unwrap_or_else(|| "all".into())
        .to_ascii_lowercase()
        .as_str()
    {
        "injectable" | "true" => Some(Value::Bool(true)),
        "not-injectable" | "false" => Some(Value::Bool(false)),
        _ => None,
    }
}

fn all_memory_statuses() -> Value {
    json!(["ACTIVE", "ARCHIVED", "CANDIDATE", "DISABLED"])
}

fn all_review_statuses() -> Value {
    json!(["APPROVED", "PENDING_REVIEW", "REJECTED"])
}

fn user_label(auth: &RequestAuthContext) -> String {
    normalize_optional(Some(&auth.user().username))
        .or_else(|| normalize_optional(Some(&auth.user().display_name)))
        .unwrap_or_else(|| auth.user().user_code.clone())
}

fn normalize_required(value: &str, field: &str) -> Result<String, MemoryError> {
    normalize_optional(Some(value))
        .ok_or_else(|| MemoryError::Validation(format!("{field} is required")))
}

fn normalize_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}
