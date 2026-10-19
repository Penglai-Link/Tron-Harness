//! Retire all obsolete attempts and detach their exclusive resources before commit.
use super::*;
use crate::application::knowledge_base::deletion::DeletionCleanupPlan;

pub(super) fn remove_old_generations(
    tx: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
    task: &IngestionTask,
    current: &Value,
    source: &str,
) -> Result<(), IngestionQueueError> {
    let table = format!("rag_{}_units", task.knowledge_base_id);
    let sources = {
        let mut stmt = tx.prepare(&format!(
            "SELECT DISTINCT source_code FROM \"{table}\" WHERE source_code!=?1 AND (json_extract(metadata,'$.documentId')=?2 OR substr(source_code,1,length(?2)+1)=?2||'_' OR source_code=?3) AND scope_code=?4 AND source_code NOT IN (SELECT coalesce(json_extract(payload_json,'$.activeSourceCode'),'') FROM knowledge_base_documents WHERE id!=?2)"
        )).map_err(storage)?;
        stmt.query_map(
            params![
                source,
                task.document_id,
                current["activeSourceCode"].as_str().unwrap_or(""),
                task.workspace_code
            ],
            |r| r.get::<_, String>(0),
        )
        .map_err(storage)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage)?
    };
    for old in sources {
        publication_indexes::retire_indexes(tx, &task.knowledge_base_id, &old)?;
        tx.execute(
            &format!("DELETE FROM \"{table}\" WHERE source_code=?1"),
            [old],
        )
        .map_err(storage)?;
    }
    Ok(())
}

pub(super) fn detach_obsolete_resources(
    tx: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
    task: &IngestionTask,
    ready: &Value,
) -> Result<PublicationCleanup, IngestionQueueError> {
    detach_resources(tx, task, ready, false)
}

pub(super) fn detach_optimization_resources(
    tx: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
    task: &IngestionTask,
    ready: &Value,
) -> Result<PublicationCleanup, IngestionQueueError> {
    detach_resources(tx, task, ready, true)
}

fn detach_resources(
    tx: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
    task: &IngestionTask,
    ready: &Value,
    retain_history: bool,
) -> Result<PublicationCleanup, IngestionQueueError> {
    let mut retained = std::collections::BTreeSet::new();
    crate::application::knowledge_base::deletion::collect_codes(ready, &mut retained);
    let historical = {
        let mut stmt = tx.prepare("SELECT payload_json FROM knowledge_base_ingestion_tasks WHERE document_id=?1 AND id!=?2 AND (?3 IS NULL OR id!=?3)").map_err(storage)?;
        stmt.query_map(
            params![task.document_id, task.id, task.parsing_task_id],
            |r| r.get::<_, String>(0),
        )
        .map_err(storage)?
        .map(|raw| serde_json::from_str::<IngestionTask>(&raw.map_err(storage)?).map_err(storage))
        .collect::<Result<Vec<_>, _>>()?
    };
    let cancellations = historical
        .iter()
        .filter(|t| t.cancellation_pending || t.reconciling)
        .cloned()
        .collect();
    if retain_history {
        // Audit rows are distinct from old index versions. Keep their original
        // ordering and diagnostics, and detach obsolete resumable checkpoints.
        for mut old in historical {
            old.parsed_resource_code = None;
            old.chunks_resource_code = None;
            old.embedding_resource_code = None;
            old.cancellation_pending = false;
            old.reconciling = false;
            write(tx, &old)?;
        }
    } else {
        tx.execute(
        "DELETE FROM knowledge_base_ingestion_tasks WHERE document_id=?1 AND id!=?2 AND (?3 IS NULL OR id!=?3)",
        params![task.document_id, task.id,task.parsing_task_id],
      ).map_err(storage)?;
    }
    if let Some(parse_id) = &task.parsing_task_id {
        let mut parsing = read(tx, parse_id)?;
        parsing.parsed_resource_code = None;
        parsing.chunks_resource_code = None;
        write(tx, &parsing)?;
    }
    tx.execute(
        "DELETE FROM knowledge_base_ingestion_intents WHERE document_id=?1",
        [&task.document_id],
    )
    .map_err(storage)?;
    let resources = {
        let mut stmt = tx.prepare("SELECT resource_code FROM knowledge_base_derived_resources WHERE document_id=?1 AND workspace_code=?2").map_err(storage)?;
        stmt.query_map(params![task.document_id, task.workspace_code], |r| {
            r.get::<_, String>(0)
        })
        .map_err(storage)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage)?
    };
    let plan = DeletionCleanupPlan {
        document_id: task.document_id.clone(),
        knowledge_base_id: task.knowledge_base_id.clone(),
        workspace_code: task.workspace_code.clone(),
        document_revision: task.document_revision,
        original_resource_code: ready["resourceCode"].as_str().unwrap_or("").into(),
        resources: vec![],
        objects: vec![],
        cancellations: vec![],
    };
    let mut objects = Vec::new();
    for code in resources {
        if !retained.contains(&code) && code != plan.original_resource_code {
            if let Some(locator) =
                super::super::knowledge_base_repository::deletion::plan_derived_cleanup(
                    tx, &plan, &code,
                )
                .map_err(storage)?
            {
                objects.push(locator);
            }
        }
    }
    objects.sort();
    objects.dedup();
    Ok(PublicationCleanup {
        objects,
        cancellations,
    })
}
