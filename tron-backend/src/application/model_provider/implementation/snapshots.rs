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
        let mut metadata = RuntimeSnapshotMetadata::default();
        metadata.merge(response);
        if metadata == RuntimeSnapshotMetadata::default() {
            return Ok(());
        }
        let RuntimeSnapshotMetadata {
            model: actual_model_code,
            model_name: actual_model_name,
            provider_code: actual_vendor_code,
            provider_name: actual_vendor_name,
        } = metadata;
        let now = now_millis();

        let this = self.clone();
        let worker_run_code = run_code.to_owned();
        let worker_workspace_code = workspace_code.to_owned();
        let worker_quality_type = quality_type.clone();
        let worker_now = now.clone();
        this.connection.clone().call(move |connection| { let run_code=worker_run_code.as_str(); let workspace_code=&worker_workspace_code; let quality_type=worker_quality_type;  let now=worker_now;
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
     },||ModelProviderError::Storage)
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
                let this = self.clone();
                let worker_quality_type = quality_type.clone();
                let worker_model_code = model_code.to_owned();
                this.connection.clone().call(
                    move |connection| {
                        let quality_type = worker_quality_type;
                        let model_code = worker_model_code.as_str();
                        read_catalog_model(
                            &connection,
                            ModelProviderCode::PersonAi,
                            model_code,
                            quality_type,
                        )
                    },
                    || ModelProviderError::Storage,
                )
            })
            .transpose()?
            .flatten();
        let now = now_millis();

        let this = self.clone();
        let worker_run_code = run_code.to_owned();
        let worker_session_code = session_code.to_owned();
        let worker_workspace_code = workspace_code.to_owned();
        this.connection.clone().call(move |connection| { let run_code=worker_run_code.as_str(); let session_code=&worker_session_code; let workspace_code=&worker_workspace_code;
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
     },||ModelProviderError::Storage)
    }
}

/// Only model lineage belongs in this row, never token deltas. Missing fields
/// preserve metadata learned earlier in the same stream (including partial failures).
#[derive(Clone, Default, PartialEq, Eq)]
pub(super) struct RuntimeSnapshotMetadata {
    model: Option<String>,
    model_name: Option<String>,
    provider_code: Option<String>,
    provider_name: Option<String>,
}
impl RuntimeSnapshotMetadata {
    pub(super) fn merge(&mut self, event: &Value) {
        let value = event
            .get("response")
            .filter(|v| v.is_object())
            .unwrap_or(event);
        for (target, candidate) in [
            (
                &mut self.model,
                value
                    .get("model")
                    .filter(|v| v.is_string())
                    .or_else(|| value.get("model_code")),
            ),
            (
                &mut self.model_name,
                value
                    .get("model_name")
                    .or_else(|| value.pointer("/model/name")),
            ),
            (
                &mut self.provider_code,
                value
                    .get("provider_code")
                    .or_else(|| value.pointer("/provider/code")),
            ),
            (
                &mut self.provider_name,
                value
                    .get("provider_name")
                    .or_else(|| value.pointer("/provider/name")),
            ),
        ] {
            if let Some(text) = candidate
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
            {
                *target = Some(text.to_owned());
            }
        }
    }
    pub(super) fn value(&self) -> Value {
        serde_json::json!({"model":self.model,"model_name":self.model_name,
            "provider_code":self.provider_code,"provider_name":self.provider_name})
    }
}
