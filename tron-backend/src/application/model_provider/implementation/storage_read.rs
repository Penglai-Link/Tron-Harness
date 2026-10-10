use super::*;

pub(super) fn normalize_server_base_url(raw: Option<&str>) -> Result<String, ModelProviderError> {
    let candidate = raw
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(PERSON_AI_DEFAULT_SERVER_BASE_URL);
    let parsed = Url::parse(candidate).map_err(|_| ModelProviderError::InvalidServerBaseUrl)?;
    if !matches!(parsed.scheme(), "https" | "http") || parsed.host_str().is_none() {
        return Err(ModelProviderError::InvalidServerBaseUrl);
    }
    Ok(candidate.trim_end_matches('/').to_owned())
}

pub(super) fn read_connection(
    connection: &Connection,
    provider_code: ModelProviderCode,
) -> Result<Option<ProviderConnectionRow>, ModelProviderError> {
    connection
        .query_row(
            "SELECT provider_code, server_base_url, status, credentials_encrypted, authorized_at,
                    authorization_expires_at, last_sync_at, last_error, updated_at,
                    person_ai_user_id, person_ai_username
             FROM provider_connections WHERE provider_code = ?1",
            params![provider_code.storage_value()],
            |row| {
                let stored_code = row.get::<_, String>(0)?;
                let status = row.get::<_, String>(2)?;
                Ok(ProviderConnectionRow {
                    provider_code: ModelProviderCode::from_storage(&stored_code)
                        .ok_or(rusqlite::Error::InvalidQuery)?,
                    person_ai_user_id: row.get(9)?,
                    person_ai_username: row.get(10)?,
                    server_base_url: row.get(1)?,
                    status: ProviderConnectionStatus::from_storage(&status)
                        .ok_or(rusqlite::Error::InvalidQuery)?,
                    credentials_encrypted: row.get(3)?,
                    authorized_at: row.get(4)?,
                    authorization_expires_at: row.get(5)?,
                    last_sync_at: row.get(6)?,
                    last_error: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            },
        )
        .optional()
        .map_err(|_| ModelProviderError::Storage)
}

pub(super) fn read_person_ai_profile_credentials(
    connection: &Connection,
    person_ai_user_id: &str,
) -> Result<Option<PersonAiProfileCredentialsRow>, ModelProviderError> {
    connection
        .query_row(
            "SELECT person_ai_user_id, person_ai_username, server_base_url, credentials_encrypted,
                    status, authorized_at, authorization_expires_at, last_error, updated_at
             FROM person_ai_profile_credentials WHERE person_ai_user_id = ?1",
            params![person_ai_user_id],
            |row| {
                let status = row.get::<_, String>(4)?;
                Ok(PersonAiProfileCredentialsRow {
                    server_base_url: row.get(2)?,
                    credentials_encrypted: row.get(3)?,
                    status: ProviderConnectionStatus::from_storage(&status)
                        .ok_or(rusqlite::Error::InvalidQuery)?,
                    authorized_at: row.get(5)?,
                    authorization_expires_at: row.get(6)?,
                    last_error: row.get(7)?,
                })
            },
        )
        .optional()
        .map_err(|_| ModelProviderError::Storage)
}

pub(super) fn read_authorization_session(
    connection: &Connection,
    session_id: &str,
) -> Result<ProviderAuthorizationSessionRow, ModelProviderError> {
    connection
        .query_row(
            "SELECT session_id, provider_code, server_base_url, login_url, status, expires_at, created_at, updated_at
             FROM provider_authorization_sessions WHERE session_id = ?1",
            params![session_id],
            |row| {
                let provider_code = row.get::<_, String>(1)?;
                Ok(ProviderAuthorizationSessionRow {
                    session_id: row.get(0)?,
                    provider_code: ModelProviderCode::from_storage(&provider_code)
                        .ok_or(rusqlite::Error::InvalidQuery)?,
                    server_base_url: row.get(2)?,
                    login_url: row.get(3)?,
                    status: row.get(4)?,
                    expires_at: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            },
        )
        .optional()
        .map_err(|_| ModelProviderError::Storage)?
        .ok_or(ModelProviderError::AuthorizationSessionNotFound)
}

pub(super) fn catalog_model_count(
    connection: &Connection,
    provider_code: ModelProviderCode,
) -> Result<usize, ModelProviderError> {
    let count = connection
        .query_row(
            "SELECT
                 (SELECT COUNT(*) FROM provider_catalog_models WHERE provider_code = ?1) +
                 (SELECT COUNT(*) FROM provider_multimedia_catalog_models WHERE provider_code = ?1)",
            params![provider_code.storage_value()],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|_| ModelProviderError::Storage)?;
    Ok(usize::try_from(count).unwrap_or(usize::MAX))
}

/// Older builds represented Person-AI's managed embedding route as a remote
/// multimedia catalogue entry (`media_type = 5`). The route remains a default
/// selection, but it is not a physical model and must not appear in the
/// synchronised catalogue.
pub(super) fn remove_legacy_person_ai_embedding_catalog_models(
    connection: &Connection,
) -> Result<(), ModelProviderError> {
    connection
        .execute(
            "DELETE FROM provider_multimedia_catalog_models
             WHERE provider_code = ?1 AND media_type = 5",
            params![ModelProviderCode::PersonAi.storage_value()],
        )
        .map_err(|_| ModelProviderError::Storage)?;
    Ok(())
}

/// Sync history is operational diagnostics, not an audit log. Retain a small,
/// provider-scoped rolling window so scheduled synchronization cannot grow the
/// local database indefinitely.
pub(super) fn trim_provider_sync_runs(
    connection: &Connection,
    provider_code: ModelProviderCode,
) -> Result<(), ModelProviderError> {
    connection
        .execute(
            "DELETE FROM provider_sync_runs
             WHERE provider_code = ?1
               AND sync_run_code NOT IN (
                   SELECT sync_run_code
                   FROM provider_sync_runs
                   WHERE provider_code = ?1
                   ORDER BY started_at DESC, sync_run_code DESC
                   LIMIT ?2
               )",
            params![provider_code.storage_value(), MAX_PROVIDER_SYNC_RUNS],
        )
        .map_err(|_| ModelProviderError::Storage)?;
    Ok(())
}

pub(super) fn connection_view(
    row: Option<ProviderConnectionRow>,
    catalog_model_count: usize,
) -> ProviderConnectionView {
    let row = row.unwrap_or_else(|| ProviderConnectionRow {
        provider_code: ModelProviderCode::PersonAi,
        person_ai_user_id: None,
        person_ai_username: None,
        server_base_url: PERSON_AI_DEFAULT_SERVER_BASE_URL.into(),
        status: ProviderConnectionStatus::Disconnected,
        credentials_encrypted: None,
        authorized_at: None,
        authorization_expires_at: None,
        last_sync_at: None,
        last_error: None,
        updated_at: 0,
    });
    ProviderConnectionView {
        provider_code: row.provider_code,
        person_ai_user_id: row.person_ai_user_id,
        person_ai_username: row.person_ai_username,
        display_name: row.provider_code.display_name().into(),
        status: row.status,
        server_base_url: row.server_base_url,
        authorized_at: row.authorized_at,
        authorization_expires_at: row.authorization_expires_at,
        last_sync_at: row.last_sync_at,
        last_error: row.last_error,
        catalog_model_count,
        updated_at: row.updated_at,
    }
}

pub(super) fn read_catalog_models(
    connection: &Connection,
    provider_code: ModelProviderCode,
) -> Result<Vec<ProviderCatalogModelView>, ModelProviderError> {
    let mut models = read_chat_catalog_models(connection, provider_code)?;
    models.extend(read_multimedia_catalog_models(connection, provider_code)?);
    models.sort_by(|left, right| {
        left.status
            .storage_value()
            .cmp(right.status.storage_value())
            .then_with(|| {
                left.catalog_model_type
                    .person_ai_media_type()
                    .unwrap_or(-1)
                    .cmp(
                        &right
                            .catalog_model_type
                            .person_ai_media_type()
                            .unwrap_or(-1),
                    )
            })
            .then_with(|| left.model_name.cmp(&right.model_name))
            .then_with(|| left.model_code.cmp(&right.model_code))
    });
    Ok(models)
}

pub(super) fn read_chat_catalog_models(
    connection: &Connection,
    provider_code: ModelProviderCode,
) -> Result<Vec<ProviderCatalogModelView>, ModelProviderError> {
    let mut statement = connection
        .prepare(
            "SELECT provider_code, model_code, model_name, upstream_vendor_code, upstream_vendor_name,
                    context_window_tokens, protocols, quality_type, input_coefficient, cache_hit_coefficient,
                    output_coefficient, status, first_seen_at, last_seen_at, updated_at
             FROM provider_catalog_models WHERE provider_code = ?1
             ORDER BY CASE status WHEN 'Available' THEN 0 ELSE 1 END,
                      CASE quality_type WHEN 'High' THEN 0 WHEN 'Middle' THEN 1 ELSE 2 END,
                      upstream_vendor_name COLLATE NOCASE, model_name COLLATE NOCASE, model_code",
        )
        .map_err(|_| ModelProviderError::Storage)?;
    statement
        .query_map(params![provider_code.storage_value()], |row| {
            let provider_code = row.get::<_, String>(0)?;
            let quality_type = row.get::<_, String>(7)?;
            let status = row.get::<_, String>(11)?;
            Ok(ProviderCatalogModelView {
                provider_code: ModelProviderCode::from_storage(&provider_code)
                    .ok_or(rusqlite::Error::InvalidQuery)?,
                catalog_model_type: ProviderCatalogModelType::Chat,
                model_id: None,
                model_code: row.get(1)?,
                model_name: row.get(2)?,
                protocols: serde_json::from_str(&row.get::<_, String>(6)?)
                    .unwrap_or_else(|_| vec![AgentModelProtocol::ChatCompletions]),
                upstream_vendor_code: row.get::<_, Option<String>>(3)?.filter(|code| !code.is_empty()),
                upstream_vendor_name: row.get(4)?,
                context_window_tokens: row
                    .get::<_, Option<i64>>(5)?
                    .and_then(|value| u32::try_from(value).ok()),
                quality_type: Some(
                    ProviderModelQuality::from_storage(&quality_type)
                        .ok_or(rusqlite::Error::InvalidQuery)?,
                ),
                input_coefficient: row.get(8)?,
                cache_hit_coefficient: row.get(9)?,
                output_coefficient: row.get(10)?,
                status: ProviderCatalogModelStatus::from_storage(&status)
                    .ok_or(rusqlite::Error::InvalidQuery)?,
                first_seen_at: row.get(12)?,
                last_seen_at: row.get(13)?,
                updated_at: row.get(14)?,
            })
        })
        .map_err(|_| ModelProviderError::Storage)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| ModelProviderError::Storage)
}

pub(super) fn read_multimedia_catalog_models(
    connection: &Connection,
    provider_code: ModelProviderCode,
) -> Result<Vec<ProviderCatalogModelView>, ModelProviderError> {
    let mut statement = connection
        .prepare(
            "SELECT provider_code, model_id, model_code, model_name, media_type, status,
                    first_seen_at, last_seen_at, updated_at
             FROM provider_multimedia_catalog_models WHERE provider_code = ?1 AND media_type != 5
             ORDER BY CASE status WHEN 'Available' THEN 0 ELSE 1 END,
                      media_type, model_name COLLATE NOCASE, model_code",
        )
        .map_err(|_| ModelProviderError::Storage)?;
    statement
        .query_map(params![provider_code.storage_value()], |row| {
            let provider_code = row.get::<_, String>(0)?;
            let media_type = row.get::<_, i32>(4)?;
            let status = row.get::<_, String>(5)?;
            Ok(ProviderCatalogModelView {
                provider_code: ModelProviderCode::from_storage(&provider_code)
                    .ok_or(rusqlite::Error::InvalidQuery)?,
                catalog_model_type: ProviderCatalogModelType::from_person_ai_media_type(media_type)
                    .ok_or(rusqlite::Error::InvalidQuery)?,
                model_id: Some(row.get(1)?),
                model_code: row.get(2)?,
                model_name: row.get(3)?,
                protocols: Vec::new(),
                upstream_vendor_code: None,
                upstream_vendor_name: None,
                context_window_tokens: None,
                quality_type: None,
                input_coefficient: None,
                cache_hit_coefficient: None,
                output_coefficient: None,
                status: ProviderCatalogModelStatus::from_storage(&status)
                    .ok_or(rusqlite::Error::InvalidQuery)?,
                first_seen_at: row.get(6)?,
                last_seen_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })
        .map_err(|_| ModelProviderError::Storage)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| ModelProviderError::Storage)
}

pub(super) fn read_catalog_model(
    connection: &Connection,
    provider_code: ModelProviderCode,
    model_code: &str,
    quality_type: Option<ProviderModelQuality>,
) -> Result<Option<ProviderCatalogModelView>, ModelProviderError> {
    let mut matches = read_chat_catalog_models(connection, provider_code)?
        .into_iter()
        .filter(|model| {
            (model.model_code == model_code || model.selection_model_code() == model_code)
                && quality_type.is_none_or(|quality| model.quality_type == Some(quality))
        });
    let first = matches.next();
    if matches.next().is_some() {
        return Err(ModelProviderError::InvalidSelection(format!(
            "ambiguous model selection {model_code}; specify upstream vendor and quality when applicable"
        )));
    }
    Ok(first)
}

pub(super) fn read_sync_runs(
    connection: &Connection,
    provider_code: ModelProviderCode,
) -> Result<Vec<ProviderSyncRunView>, ModelProviderError> {
    let mut statement = connection
        .prepare(
            "SELECT sync_run_code, provider_code, status, started_at, completed_at, model_count, error_summary
             FROM provider_sync_runs WHERE provider_code = ?1
             ORDER BY started_at DESC, sync_run_code DESC LIMIT 10",
        )
        .map_err(|_| ModelProviderError::Storage)?;
    statement
        .query_map(params![provider_code.storage_value()], |row| {
            let provider_code = row.get::<_, String>(1)?;
            let status = row.get::<_, String>(2)?;
            Ok(ProviderSyncRunView {
                sync_run_code: row.get(0)?,
                provider_code: ModelProviderCode::from_storage(&provider_code)
                    .ok_or(rusqlite::Error::InvalidQuery)?,
                status: ProviderSyncStatus::from_storage(&status)
                    .ok_or(rusqlite::Error::InvalidQuery)?,
                started_at: row.get(3)?,
                completed_at: row.get(4)?,
                model_count: row
                    .get::<_, Option<i64>>(5)?
                    .and_then(|value| usize::try_from(value).ok()),
                error_summary: row.get(6)?,
            })
        })
        .map_err(|_| ModelProviderError::Storage)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| ModelProviderError::Storage)
}
