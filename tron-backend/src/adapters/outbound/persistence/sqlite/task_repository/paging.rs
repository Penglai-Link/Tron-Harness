use super::*;
use crate::{application::task::TaskReadPage, domain::task::TaskListQuery};
use rusqlite::{functions::FunctionFlags, types::Value};

/// Count, page, hierarchy and links share one WAL snapshot. Only the current page
/// and its live descendants are decoded; unrelated tasks never enter the service.
pub(super) fn read_page(
    connection: &mut Connection,
    query: &TaskListQuery,
    window: Option<(usize, usize)>,
) -> Result<TaskReadPage, TaskError> {
    let tx =
        crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(
            connection,
            rusqlite::TransactionBehavior::Deferred,
        )
        .map_err(storage_error)?;
    let owner = normalized(query.owner_assistant_code.as_deref());
    let workspace = normalized(query.workspace_code.as_deref());
    let mut predicates = Vec::new();
    let mut values = Vec::<Value>::new();
    let mut equal = |column: &str, value: Option<String>| {
        if let Some(value) = value {
            values.push(Value::Text(value));
            predicates.push(format!("{column} = ?{}", values.len()));
        } else {
            predicates.push(format!("{column} IS NULL"));
        }
    };
    equal("owner_assistant_code", owner.clone());
    if owner.is_none() || workspace.is_some() {
        equal("workspace_code", workspace);
    }
    if !query.include_archived.unwrap_or(false) {
        predicates.push("archived_at IS NULL".into());
    }
    for (column, value) in [
        ("status", query.status.clone()),
        (
            "priority",
            query
                .priority
                .map(|v| task_priority_to_storage(v).to_owned()),
        ),
        (
            "task_type",
            query.task_type.map(|v| task_type_to_storage(v).to_owned()),
        ),
        (
            "source_type",
            query
                .source_type
                .map(|v| task_source_type_to_storage(v).to_owned()),
        ),
    ] {
        if let Some(value) = value {
            values.push(Value::Text(value));
            predicates.push(format!("{column} = ?{}", values.len()));
        }
    }
    if let Some(parent) = normalized(query.parent_task_code.as_deref()) {
        if parent.eq_ignore_ascii_case("__ROOT__") {
            predicates.push("parent_task_code IS NULL".into());
        } else {
            values.push(Value::Text(parent));
            predicates.push(format!("parent_task_code = ?{}", values.len()));
        }
    }
    // This adapter has no persisted user-assignment relation (its assignment port
    // returns an empty set); preserve that filter's existing empty-result semantics.
    if normalized(query.assignee_user_code.as_deref()).is_some() {
        predicates.push("0".into());
    }
    if let Some(search) = normalized(query.search.as_deref()) {
        tx.create_scalar_function(
            "tron_task_contains",
            2,
            FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
            |ctx| {
                Ok(ctx
                    .get::<String>(0)?
                    .to_lowercase()
                    .contains(&ctx.get::<String>(1)?))
            },
        )
        .map_err(storage_error)?;
        values.push(Value::Text(search.to_lowercase()));
        predicates.push(format!("tron_task_contains(task_code || ' ' || COALESCE(workspace_code,'') || ' ' || title || ' ' || COALESCE(description,'') || ' ' || COALESCE(execution_result,'') || ' ' || COALESCE(source_url,''), ?{})",values.len()));
    }
    let filter = predicates.join(" AND ");
    let (start, size) = window.unwrap_or((0, usize::MAX));
    values.push(Value::Integer(if window.is_none() {
        -1
    } else {
        i64::try_from(size).unwrap_or(i64::MAX)
    }));
    let limit = values.len();
    values.push(Value::Integer(i64::try_from(start).unwrap_or(i64::MAX)));
    let offset = values.len();
    let sql = format!("WITH RECURSIVE
        page AS MATERIALIZED (SELECT task_code FROM tasks WHERE {filter} ORDER BY task_code LIMIT ?{limit} OFFSET ?{offset}),
        tree(code) AS (SELECT task_code FROM page UNION SELECT child.task_code FROM tasks child JOIN tree ON child.parent_task_code = tree.code WHERE child.archived_at IS NULL)
        SELECT t.id,t.task_code,t.workspace_code,t.parent_task_code,t.title,NULL,NULL,t.flow_code,t.status,t.previous_status,t.priority,t.task_type,t.source_type,t.source_url,t.agent_code,t.model_code,t.progress_value,t.created_at,t.updated_at,t.archived_at,t.owner_assistant_code,t.due_at,t.planned_start_at,t.next_followup_at,t.work_session_code,t.schedule_version,
        t.task_code IN (SELECT task_code FROM page), total.n
        FROM (SELECT count(*) AS n FROM tasks WHERE {filter}) total
        LEFT JOIN tasks t ON t.task_code IN (SELECT code FROM tree)");
    let mut result = TaskReadPage {
        tasks: Vec::new(),
        page_codes: Vec::new(),
        total: 0,
        relations: Default::default(),
    };
    {
        let mut statement = tx.prepare(&sql).map_err(storage_error)?;
        let mut rows = statement
            .query(rusqlite::params_from_iter(values))
            .map_err(storage_error)?;
        while let Some(row) = rows.next().map_err(storage_error)? {
            result.total = row.get::<_, i64>(27).map_err(storage_error)? as usize;
            if row
                .get::<_, Option<i64>>(0)
                .map_err(storage_error)?
                .is_some()
            {
                let task = row_to_task(row).map_err(storage_error)?;
                if row.get::<_, bool>(26).map_err(storage_error)? {
                    result.page_codes.push(task.task_code.clone());
                }
                result.tasks.push(task);
            }
        }
    }
    result.page_codes.sort();
    result.relations = read_task_relations(&tx, &result.page_codes)?;
    tx.commit().map_err(storage_error)?;
    Ok(result)
}
fn normalized(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
}
