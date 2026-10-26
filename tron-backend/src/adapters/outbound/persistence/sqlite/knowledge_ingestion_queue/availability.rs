use super::*;

pub(super) fn eligible(task: &IngestionTask, endpoint: &ParseEndpointCapacity) -> bool {
    endpoint.capacity > 0
        && endpoint.weight > 0
        && endpoint.resource_group_capacity > 0
        && task
            .endpoint_id
            .as_ref()
            .is_none_or(|id| id == &endpoint.endpoint_id)
        && task.configuration["endpointIds"]
            .as_array()
            .is_none_or(|ids| {
                ids.iter()
                    .any(|id| id.as_str() == Some(endpoint.endpoint_id.as_str()))
            })
        && (endpoint.supported_tiers.is_empty()
            || task.configuration["parsingTier"]
                .as_str()
                .is_none_or(|tier| endpoint.supported_tiers.iter().any(|t| t == tier)))
}

pub(super) fn waiting_validated(
    tx: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
    task: &mut IngestionTask,
    reason: IngestionWaitReason,
) -> Result<(), IngestionQueueError> {
    if task.state == "QUEUED" && task.wait_reason.as_ref() == Some(&reason) {
        return Ok(());
    }
    task.wait_reason = Some(reason);
    task.state = "QUEUED".into();
    clear_lease(task);
    write(tx, task)?;
    document_state_validated(tx, task, "QUEUED", task.error.as_deref())
}

pub(super) fn report(
    queue: &SqliteKnowledgeIngestionQueueSql<'_>,
    now: i64,
    endpoints: &[ParseEndpointCapacity],
    unavailable: &[(Option<String>, IngestionWaitReason)],
) -> Result<(), IngestionQueueError> {
    let conn = &*queue.connection;
    let tx =
        crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(
            &conn,
            TransactionBehavior::Immediate,
        )
        .map_err(storage)?;
    let tasks = candidates(&tx, "('Parsing','Chunking')", now, None, false)?;
    for (mut task, valid) in tasks {
        if !valid {
            continue;
        }
        if endpoints.iter().any(|e| eligible(&task, e)) {
            continue;
        }
        let allowed = |id: &str| {
            task.endpoint_id.as_ref().map_or_else(
                || {
                    task.configuration["endpointIds"]
                        .as_array()
                        .is_none_or(|ids| ids.iter().any(|i| i.as_str() == Some(id)))
                },
                |original| original == id,
            )
        };
        let details: Vec<IngestionWaitReason> = unavailable
            .iter()
            .filter(|(id, _)| id.as_deref().is_none_or(&allowed))
            .map(|(id, reason)| {
                let mut reason = reason.clone();
                if let Some(id) = id {
                    if !reason.params.is_object() {
                        reason.params = serde_json::json!({});
                    }
                    reason.params["endpointId"] = id.clone().into();
                }
                reason
            })
            .collect();
        let reason = if !details.is_empty() {
            IngestionWaitReason {
                code: WaitReasonCode::MineruUnavailable,
                params: Value::Null,
                details,
            }
        } else if endpoints.iter().any(|e| allowed(&e.endpoint_id)) {
            IngestionWaitReason::with_params(
                WaitReasonCode::UnsupportedTier,
                serde_json::json!({"tier": task.configuration["parsingTier"]}),
            )
        } else if let Some(original) = &task.endpoint_id {
            IngestionWaitReason::with_params(
                WaitReasonCode::OriginalNodeUnavailable,
                serde_json::json!({"endpointId":original}),
            )
        } else {
            IngestionWaitReason::new(WaitReasonCode::NodeSetUnavailable)
        };
        waiting_validated(&tx, &mut task, reason)?;
    }
    tx.commit().map_err(storage)
}
