use rusqlite::Connection;

use crate::domain::short_term_memory::ShortTermMemoryError;

use super::super::short_term_memory_rows::storage_error;

pub(super) fn list_externalized_tool_output_paths(
    connection: &Connection,
    root_path: &str,
) -> Result<Vec<String>, ShortTermMemoryError> {
    let mut statement = connection
        .prepare(
            "SELECT content_json
             FROM agent_transcript_parts
             WHERE part_type = 'tool_result'
               AND content_json IS NOT NULL",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;
    let mut paths = Vec::new();
    for row in rows {
        let value = serde_json::from_str::<serde_json::Value>(&row)
            .map_err(|_| ShortTermMemoryError::StorageFailure)?;
        let Some(metadata) = value.get("metadata") else {
            continue;
        };
        let Some(stored_root) = metadata.get("rootPath").and_then(|item| item.as_str()) else {
            continue;
        };
        let Some(relative_path) = metadata.get("relativePath").and_then(|item| item.as_str())
        else {
            continue;
        };
        if stored_root == root_path {
            paths.push(relative_path.to_string());
        }
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}
