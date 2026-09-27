use super::*;

impl ModelProviderService {
    pub fn record_runtime_snapshot(
        &self,
        run_code: Option<&str>,
        workspace_code: &str,
        config: &AgentModelConfig,
        response: &Value,
    ) -> Result<(), ModelProviderError> {
        let Some(run_code) = run_code.map(str::trim).filter(|value| !value.is_empty()) else {
            return Ok(());
        };
        let (selection_kind, requested_model_code, route_code) =
            if config.model_code == "PERSON_AI:ROUTE:ChatAuto" {
                (
                    ModelSelectionKind::ProviderRoute,
                    None,
                    Some(ProviderRouteCode::ChatAuto),
                )
            } else {
                (
                    ModelSelectionKind::Fixed,
                    Some(config.model_name.clone()),
                    None,
                )
            };
        let quality_type = person_ai_runtime_quality(config);
        let actual_model_code = response
            .get("model")
            .or_else(|| response.get("model_code"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        let actual_model_name = response
            .get("model_name")
            .or_else(|| response.pointer("/model/name"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        let actual_vendor_code = response
            .get("provider_code")
            .or_else(|| response.pointer("/provider/code"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        let actual_vendor_name = response
            .get("provider_name")
            .or_else(|| response.pointer("/provider/name"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        let now = now_millis();
        let connection = self
            .connection
            .lock()
            .map_err(|_| ModelProviderError::Storage)?;
        connection
            .execute(
                "INSERT INTO provider_model_run_snapshots (
                    run_code, session_code, workspace_code, selection_kind, provider_code,
                    requested_model_code, quality_type, route_code, resolved_model_code, resolved_model_name,
                    resolved_upstream_vendor_code, resolved_upstream_vendor_name, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13)
                 ON CONFLICT(run_code) DO UPDATE SET
                    resolved_model_code = COALESCE(excluded.resolved_model_code, provider_model_run_snapshots.resolved_model_code),
                    resolved_model_name = COALESCE(excluded.resolved_model_name, provider_model_run_snapshots.resolved_model_name),
                    resolved_upstream_vendor_code = COALESCE(excluded.resolved_upstream_vendor_code, provider_model_run_snapshots.resolved_upstream_vendor_code),
                    resolved_upstream_vendor_name = COALESCE(excluded.resolved_upstream_vendor_name, provider_model_run_snapshots.resolved_upstream_vendor_name),
                    updated_at = excluded.updated_at",
                params![
                    run_code,
                    Option::<String>::None,
                    workspace_code,
                    selection_kind.storage_value(),
                    ModelProviderCode::PersonAi.storage_value(),
                    requested_model_code,
                    quality_type.map(ProviderModelQuality::storage_value),
                    route_code.map(ProviderRouteCode::storage_value),
                    actual_model_code,
                    actual_model_name,
                    actual_vendor_code,
                    actual_vendor_name,
                    now,
                ],
            )
            .map_err(|_| ModelProviderError::Storage)?;
        Ok(())
    }

    /// Persist the choice before invocation. Stream callbacks later enrich this
    /// row with the actual Person-AI response model, so catalog changes cannot
    /// erase the model lineage of a finished run.
    pub fn record_requested_runtime_snapshot(
        &self,
        run_code: Option<&str>,
        session_code: &str,
        workspace_code: &str,
        config: &AgentModelConfig,
    ) -> Result<(), ModelProviderError> {
        let Some(run_code) = run_code.map(str::trim).filter(|value| !value.is_empty()) else {
            return Ok(());
        };
        if !self.is_person_ai_config(&config.model_code) {
            return Ok(());
        }
        let (selection_kind, requested_model_code, route_code) =
            if config.model_code == "PERSON_AI:ROUTE:ChatAuto" {
                (
                    ModelSelectionKind::ProviderRoute,
                    None,
                    Some(ProviderRouteCode::ChatAuto),
                )
            } else {
                (
                    ModelSelectionKind::Fixed,
                    Some(config.model_name.clone()),
                    None,
                )
            };
        let quality_type = person_ai_runtime_quality(config);
        let fixed_catalog_model = requested_model_code
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(|model_code| {
                let connection = self
                    .connection
                    .lock()
                    .map_err(|_| ModelProviderError::Storage)?;
                read_catalog_model(
                    &connection,
                    ModelProviderCode::PersonAi,
                    model_code,
                    quality_type,
                )
            })
            .transpose()?
            .flatten();
        let now = now_millis();
        let connection = self
            .connection
            .lock()
            .map_err(|_| ModelProviderError::Storage)?;
        connection
            .execute(
                "INSERT INTO provider_model_run_snapshots (
                    run_code, session_code, workspace_code, selection_kind, provider_code,
                    requested_model_code, quality_type, route_code, resolved_model_code, resolved_model_name,
                    resolved_upstream_vendor_code, resolved_upstream_vendor_name, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13)
                 ON CONFLICT(run_code) DO UPDATE SET
                    session_code = excluded.session_code,
                    workspace_code = excluded.workspace_code,
                    selection_kind = excluded.selection_kind,
                    provider_code = excluded.provider_code,
                    requested_model_code = excluded.requested_model_code,
                    quality_type = excluded.quality_type,
                    route_code = excluded.route_code,
                    resolved_model_code = COALESCE(excluded.resolved_model_code, provider_model_run_snapshots.resolved_model_code),
                    resolved_model_name = COALESCE(excluded.resolved_model_name, provider_model_run_snapshots.resolved_model_name),
                    resolved_upstream_vendor_code = COALESCE(excluded.resolved_upstream_vendor_code, provider_model_run_snapshots.resolved_upstream_vendor_code),
                    resolved_upstream_vendor_name = COALESCE(excluded.resolved_upstream_vendor_name, provider_model_run_snapshots.resolved_upstream_vendor_name),
                    updated_at = excluded.updated_at",
                params![
                    run_code,
                    session_code,
                    workspace_code,
                    selection_kind.storage_value(),
                    ModelProviderCode::PersonAi.storage_value(),
                    requested_model_code,
                    quality_type.map(ProviderModelQuality::storage_value),
                    route_code.map(ProviderRouteCode::storage_value),
                    fixed_catalog_model.as_ref().map(|model| model.model_code.clone()),
                    fixed_catalog_model.as_ref().map(|model| model.model_name.clone()),
                    fixed_catalog_model.as_ref().and_then(|model| model.upstream_vendor_code.clone()),
                    fixed_catalog_model.as_ref().and_then(|model| model.upstream_vendor_name.clone()),
                    now,
                ],
            )
            .map_err(|_| ModelProviderError::Storage)?;
        Ok(())
    }
}
