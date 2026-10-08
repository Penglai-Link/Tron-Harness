use super::*;

pub(super) fn read_run_snapshot(
    connection: &Connection,
    run_code: &str,
) -> Result<Option<ProviderModelRunSnapshotView>, ModelProviderError> {
    connection
        .query_row(
            "SELECT run_code, session_code, workspace_code, selection_kind, provider_code,
                    requested_model_code, quality_type, route_code, resolved_model_code, resolved_model_name,
                    resolved_upstream_vendor_code, resolved_upstream_vendor_name, created_at, updated_at
             FROM provider_model_run_snapshots WHERE run_code = ?1",
            params![run_code],
            |row| {
                let selection_kind = row.get::<_, String>(3)?;
                let provider_code = row.get::<_, String>(4)?;
                let quality_type = row.get::<_, Option<String>>(6)?;
                let route_code = row.get::<_, Option<String>>(7)?;
                let route_code = match route_code.as_deref() {
                    Some(value) => Some(
                        ProviderRouteCode::from_storage(value)
                            .ok_or(rusqlite::Error::InvalidQuery)?,
                    ),
                    None => None,
                };
                let quality_type = match quality_type.as_deref() {
                    Some(value) => Some(
                        ProviderModelQuality::from_storage(value)
                            .ok_or(rusqlite::Error::InvalidQuery)?,
                    ),
                    None => None,
                };
                Ok(ProviderModelRunSnapshotView {
                    run_code: row.get(0)?,
                    session_code: row.get(1)?,
                    workspace_code: row.get(2)?,
                    model_selection: ModelSelection {
                        kind: ModelSelectionKind::from_storage(&selection_kind)
                            .ok_or(rusqlite::Error::InvalidQuery)?,
                        provider_code: Some(
                            ModelProviderCode::from_storage(&provider_code)
                                .ok_or(rusqlite::Error::InvalidQuery)?,
                        ),
                        model_code: row.get(5)?,
                        quality_type,
                        route_code,
                    },
                    resolved_model_code: row.get(8)?,
                    resolved_model_name: row.get(9)?,
                    resolved_upstream_vendor_code: row.get(10)?,
                    resolved_upstream_vendor_name: row.get(11)?,
                    created_at: row.get(12)?,
                    updated_at: row.get(13)?,
                })
            },
        )
        .optional()
        .map_err(|_| ModelProviderError::Storage)
}

pub(super) fn apply_catalog_snapshot(
    transaction: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
    provider_code: ModelProviderCode,
    catalog: &[ExternalCatalogModel],
    now: i64,
) -> Result<(), ModelProviderError> {
    transaction
        .execute(
            "UPDATE provider_catalog_models SET status = ?2, updated_at = ?3 WHERE provider_code = ?1",
            params![
                provider_code.storage_value(),
                ProviderCatalogModelStatus::Unavailable.storage_value(),
                now,
            ],
        )
        .map_err(|_| ModelProviderError::Storage)?;
    for model in catalog {
        transaction
            .execute(
                "INSERT INTO provider_catalog_models (
                    provider_code, model_code, model_name, upstream_vendor_code, upstream_vendor_name,
                    context_window_tokens, protocols, quality_type, input_coefficient, cache_hit_coefficient,
                    output_coefficient, status, first_seen_at, last_seen_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13, ?13)
                 ON CONFLICT(provider_code, model_code, upstream_vendor_code, quality_type) DO UPDATE SET
                    model_name = excluded.model_name,
                    upstream_vendor_code = excluded.upstream_vendor_code,
                    upstream_vendor_name = excluded.upstream_vendor_name,
                    context_window_tokens = excluded.context_window_tokens,
                    protocols = excluded.protocols,
                    quality_type = excluded.quality_type,
                    input_coefficient = excluded.input_coefficient,
                    cache_hit_coefficient = excluded.cache_hit_coefficient,
                    output_coefficient = excluded.output_coefficient,
                    status = excluded.status,
                    last_seen_at = excluded.last_seen_at,
                    updated_at = excluded.updated_at",
                params![
                    provider_code.storage_value(),
                    model.model_code,
                    model.model_name,
                    model.upstream_vendor_code.as_deref().unwrap_or_default(),
                    model.upstream_vendor_name,
                    model.context_window_tokens.map(i64::from),
                    serde_json::to_string(&model.protocols).unwrap_or_else(|_| "[\"ChatCompletions\"]".into()),
                    model.quality_type.storage_value(),
                    model.input_coefficient,
                    model.cache_hit_coefficient,
                    model.output_coefficient,
                    ProviderCatalogModelStatus::Available.storage_value(),
                    now,
                ],
            )
            .map_err(|_| ModelProviderError::Storage)?;
    }
    Ok(())
}

pub(super) fn apply_multimedia_catalog_snapshot(
    transaction: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
    provider_code: ModelProviderCode,
    catalog: &[ExternalMultimediaCatalogModel],
    now: i64,
) -> Result<(), ModelProviderError> {
    remove_legacy_person_ai_embedding_catalog_models(transaction)?;
    transaction
        .execute(
            "UPDATE provider_multimedia_catalog_models
             SET status = ?2, updated_at = ?3 WHERE provider_code = ?1",
            params![
                provider_code.storage_value(),
                ProviderCatalogModelStatus::Unavailable.storage_value(),
                now,
            ],
        )
        .map_err(|_| ModelProviderError::Storage)?;
    for model in catalog {
        let media_type = model
            .catalog_model_type
            .person_ai_media_type()
            .expect("multimedia catalog entries cannot use the Chat type");
        transaction
            .execute(
                "INSERT INTO provider_multimedia_catalog_models (
                    provider_code, model_id, model_code, model_name, media_type, status,
                    first_seen_at, last_seen_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, ?7)
                 ON CONFLICT(provider_code, model_id) DO UPDATE SET
                    model_code = excluded.model_code,
                    model_name = excluded.model_name,
                    media_type = excluded.media_type,
                    status = excluded.status,
                    last_seen_at = excluded.last_seen_at,
                    updated_at = excluded.updated_at",
                params![
                    provider_code.storage_value(),
                    model.model_id,
                    model.model_code,
                    model.model_name,
                    media_type,
                    ProviderCatalogModelStatus::Available.storage_value(),
                    now,
                ],
            )
            .map_err(|_| ModelProviderError::Storage)?;
    }
    Ok(())
}

pub(super) fn replace_authorized_connection(
    transaction: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
    provider_code: ModelProviderCode,
    credentials: &ProviderCredentials,
    person_ai_user_id: &str,
    person_ai_username: &str,
    encrypted: &str,
    authorization_session_id: &str,
    catalog: &[ExternalCatalogModel],
    multimedia_catalog: &[ExternalMultimediaCatalogModel],
    sync_run_code: &str,
    now: i64,
) -> Result<(), ModelProviderError> {
    transaction
        .execute(
            "INSERT INTO provider_connections (
                provider_code, person_ai_user_id, person_ai_username, server_base_url, status, credentials_encrypted, authorized_at,
                authorization_expires_at, last_sync_at, last_error, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?7, NULL, ?7, ?7)
             ON CONFLICT(provider_code) DO UPDATE SET
                person_ai_user_id = excluded.person_ai_user_id,
                person_ai_username = excluded.person_ai_username,
                server_base_url = excluded.server_base_url,
                status = excluded.status,
                credentials_encrypted = excluded.credentials_encrypted,
                authorized_at = excluded.authorized_at,
                authorization_expires_at = excluded.authorization_expires_at,
                last_sync_at = excluded.last_sync_at,
                last_error = NULL,
                updated_at = excluded.updated_at",
            params![
                provider_code.storage_value(),
                person_ai_user_id,
                person_ai_username,
                credentials.server_base_url,
                ProviderConnectionStatus::Connected.storage_value(),
                encrypted,
                now,
                credentials.authorization_expires_at,
            ],
        )
        .map_err(|_| ModelProviderError::Storage)?;
    if provider_code == ModelProviderCode::PersonAi {
        transaction
            .execute(
                "INSERT INTO person_ai_profile_credentials (
                    person_ai_user_id, person_ai_username, server_base_url, credentials_encrypted,
                    status, authorized_at, authorization_expires_at, last_error, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, ?6)
                 ON CONFLICT(person_ai_user_id) DO UPDATE SET
                    person_ai_username = excluded.person_ai_username,
                    server_base_url = excluded.server_base_url,
                    credentials_encrypted = excluded.credentials_encrypted,
                    status = excluded.status,
                    authorized_at = excluded.authorized_at,
                    authorization_expires_at = excluded.authorization_expires_at,
                    last_error = NULL,
                    updated_at = excluded.updated_at",
                params![
                    person_ai_user_id,
                    person_ai_username,
                    credentials.server_base_url,
                    encrypted,
                    ProviderConnectionStatus::Connected.storage_value(),
                    now,
                    credentials.authorization_expires_at,
                ],
            )
            .map_err(|_| ModelProviderError::Storage)?;
    }
    apply_catalog_snapshot(transaction, provider_code, catalog, now)?;
    apply_multimedia_catalog_snapshot(transaction, provider_code, multimedia_catalog, now)?;
    transaction
        .execute(
            "INSERT INTO provider_sync_runs (
                sync_run_code, provider_code, status, started_at, completed_at, model_count, error_summary
             ) VALUES (?1, ?2, ?3, ?4, ?4, ?5, NULL)",
            params![
                sync_run_code,
                provider_code.storage_value(),
                ProviderSyncStatus::Succeeded.storage_value(),
                now,
                i64::try_from(catalog.len().saturating_add(multimedia_catalog.len()))
                    .unwrap_or(i64::MAX),
            ],
    )
    .map_err(|_| ModelProviderError::Storage)?;
    trim_provider_sync_runs(transaction, provider_code)?;
    transaction
        .execute(
            "UPDATE provider_authorization_sessions SET status = 'Completed', updated_at = ?2 WHERE session_id = ?1",
            params![authorization_session_id, now],
        )
        .map_err(|_| ModelProviderError::Storage)?;
    for (model_type, route_code) in [
        (ModelType::Text, ProviderRouteCode::ChatAuto),
        (ModelType::Embedding, ProviderRouteCode::EmbeddingAuto),
    ] {
        transaction
            .execute(
                "INSERT INTO default_model_selections (
                    scope_key, workspace_code, model_type, selection_kind, provider_code, model_code,
                    quality_type, route_code, created_at, updated_at
                 ) VALUES ('platform', NULL, ?1, ?2, ?3, NULL, NULL, ?4, ?5, ?5)
                 ON CONFLICT(scope_key, model_type) DO UPDATE SET
                    selection_kind = excluded.selection_kind,
                    provider_code = excluded.provider_code,
                    model_code = NULL,
                    quality_type = NULL,
                    route_code = excluded.route_code,
                    updated_at = excluded.updated_at",
                params![
                    model_type_storage(model_type),
                    ModelSelectionKind::ProviderRoute.storage_value(),
                    provider_code.storage_value(),
                    route_code.storage_value(),
                    now,
                ],
            )
            .map_err(|_| ModelProviderError::Storage)?;
    }
    Ok(())
}

pub(super) fn effective_default_selection(
    connection: &Connection,
    workspace_code: &str,
    model_type: ModelType,
) -> Result<Option<ModelSelection>, ModelProviderError> {
    let workspace_code = workspace_code.trim();
    let workspace = if workspace_code.is_empty() {
        None
    } else {
        Some(workspace_code)
    };
    let rows = [workspace, Some("platform")];
    for scope_key in rows.into_iter().flatten() {
        let row = connection
            .query_row(
                "SELECT selection_kind, provider_code, model_code, quality_type, route_code
                 FROM default_model_selections WHERE scope_key = ?1 AND model_type = ?2",
                params![scope_key, model_type_storage(model_type)],
                |row| {
                    let kind = row.get::<_, String>(0)?;
                    let provider_code = row.get::<_, Option<String>>(1)?;
                    let quality_type = row.get::<_, Option<String>>(3)?;
                    let route_code = row.get::<_, Option<String>>(4)?;
                    let provider_code = match provider_code {
                        Some(provider_code) => Some(
                            ModelProviderCode::from_storage(&provider_code)
                                .ok_or(rusqlite::Error::InvalidQuery)?,
                        ),
                        None => None,
                    };
                    let route_code = match route_code {
                        Some(route_code) => Some(
                            ProviderRouteCode::from_storage(&route_code)
                                .ok_or(rusqlite::Error::InvalidQuery)?,
                        ),
                        None => None,
                    };
                    let quality_type = match quality_type {
                        Some(quality_type) => Some(
                            ProviderModelQuality::from_storage(&quality_type)
                                .ok_or(rusqlite::Error::InvalidQuery)?,
                        ),
                        None => None,
                    };
                    Ok(ModelSelection {
                        kind: ModelSelectionKind::from_storage(&kind)
                            .ok_or(rusqlite::Error::InvalidQuery)?,
                        provider_code,
                        model_code: row.get(2)?,
                        quality_type,
                        route_code,
                    })
                },
            )
            .optional()
            .map_err(|_| ModelProviderError::Storage)?;
        if row.is_some() {
            return Ok(row);
        }
    }
    Ok(None)
}
