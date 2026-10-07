#[cfg(test)]
mod tests {
    use rusqlite::{Connection, params};

    use super::{SqlitePromptRepository, migrate_prompt_variable_metadata};

    #[test]
    fn legacy_prompt_variables_are_migrated_to_typed_metadata() {
        let mut connection = Connection::open_in_memory().expect("legacy database should open");
        connection
            .execute_batch(
                "PRAGMA foreign_keys = ON;
                 CREATE TABLE prompts (prompt_code TEXT PRIMARY KEY);
                 CREATE TABLE prompt_variables (
                    prompt_code TEXT NOT NULL REFERENCES prompts(prompt_code) ON DELETE CASCADE,
                    variable_name TEXT NOT NULL,
                    variable_type TEXT NOT NULL DEFAULT 'STRING'
                        CHECK (variable_type = 'STRING'),
                    position INTEGER NOT NULL,
                    PRIMARY KEY (prompt_code, variable_name)
                 );
                 CREATE INDEX idx_prompt_variables_prompt_position
                    ON prompt_variables(prompt_code, position);
                 INSERT INTO prompts (prompt_code) VALUES ('PROMPT-LEGACY');
                 INSERT INTO prompt_variables (
                    prompt_code, variable_name, variable_type, position
                 ) VALUES ('PROMPT-LEGACY', 'name', 'STRING', 0);",
            )
            .expect("legacy prompt schema should be created");

        migrate_prompt_variable_metadata(&mut connection)
            .expect("legacy variable metadata should migrate");

        let migrated = connection
            .query_row(
                "SELECT variable_type, description FROM prompt_variables
                 WHERE prompt_code = ?1 AND variable_name = ?2",
                params!["PROMPT-LEGACY", "name"],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .expect("migrated variable should remain readable");
        assert_eq!(
            migrated,
            ("STRING".to_string(), "Variable name".to_string())
        );

        connection
            .execute(
                "INSERT INTO prompt_variables (
                    prompt_code, variable_name, variable_type, description, position
                 ) VALUES (?1, ?2, ?3, ?4, ?5)",
                params!["PROMPT-LEGACY", "count", "NUMBER", "Item count", 1],
            )
            .expect("migrated schema should accept supported variable types");

        migrate_prompt_variable_metadata(&mut connection)
            .expect("reapplying the migration should be safe");
        let preserved = connection
            .query_row(
                "SELECT variable_type, description FROM prompt_variables
                 WHERE prompt_code = ?1 AND variable_name = ?2",
                params!["PROMPT-LEGACY", "count"],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .expect("reapplied migration should preserve typed metadata");
        assert_eq!(preserved, ("NUMBER".to_string(), "Item count".to_string()));
    }

    #[test]
    fn action_category_is_supported_by_the_prompt_schema() {
        let repository =
            SqlitePromptRepository::connect(":memory:").expect("prompt schema should migrate");
        repository
            .connection
            .runtime()
            .call_blocking(|connection| {
                connection.execute(
                    "INSERT INTO prompts (
                    id, scope_kind, workspace_code, prompt_code, display_name, category,
                    prompt_body, description, enabled, source_kind, source_prompt_code,
                    allow_inheritance, version, created_at, updated_at, built_in,
                    runtime_required, catalog_key
                ) VALUES (1, 'PLATFORM', NULL, 'PROMPT-ACTION', 'Action prompt', 'ACTION',
                    'Run {{command}}', NULL, 1, 'PLATFORM', NULL, NULL, 1, 1, 1, 0, 0, NULL)",
                    [],
                )
            })
            .expect("action prompt category should satisfy the schema");

        let category = repository
            .connection
            .runtime()
            .call_blocking(|connection| {
                connection.query_row(
                    "SELECT category FROM prompts WHERE prompt_code = 'PROMPT-ACTION'",
                    [],
                    |row| row.get::<_, String>(0),
                )
            })
            .expect("action prompt should be readable");
        assert_eq!(category, "ACTION");
    }
}
