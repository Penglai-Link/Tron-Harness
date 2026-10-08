#[cfg(test)]
mod tests {
    use rusqlite::{Connection, params};

    use super::{
        PROMPTS_MIGRATION, SqlitePromptRepository, migrate_prompt_agent_role_category,
        migrate_prompt_catalog, migrate_prompt_category, migrate_prompt_variable_bindings,
        migrate_prompt_variable_metadata,
    };

    #[test]
    fn agent_role_upgrade_preserves_bindings_translations_and_inheritance() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection.execute_batch(PROMPTS_MIGRATION).unwrap();
        migrate_prompt_variable_metadata(&mut connection).unwrap();
        migrate_prompt_catalog(&mut connection).unwrap();
        migrate_prompt_category(&mut connection).unwrap();
        migrate_prompt_variable_bindings(&mut connection).unwrap();
        connection.execute_batch(
            "INSERT INTO prompts (id, scope_kind, workspace_code, prompt_code, display_name,
                category, prompt_body, enabled, source_kind, source_prompt_code, allow_inheritance,
                version, created_at, updated_at, built_in, runtime_required, catalog_key)
             VALUES (1, 'PLATFORM', NULL, 'LEGACY', 'Legacy', 'TEMPLATE', 'Original', 1,
                'PLATFORM', NULL, NULL, 3, 1, 1, 1, 0, 'legacy.catalog'),
                (2, 'WORKSPACE', 'TRON_CORE', 'OVERRIDE', 'Local', 'TEMPLATE', 'Override', 1,
                'WORKSPACE_OVERRIDE', 'LEGACY', NULL, 4, 1, 1, 0, 0, NULL);
             INSERT INTO prompt_variables (prompt_code, variable_name, variable_type, description,
                position, source_kind, managed_variable_key, runtime_provider)
             VALUES ('LEGACY', 'title', 'STRING', 'Title', 0, 'RUNTIME', NULL, 'RAG_TREE_TITLE'),
                ('OVERRIDE', 'name', 'STRING', 'Name', 0, 'MANAGED', 'MY_NAME', NULL);
             INSERT INTO prompt_parameters (prompt_code, parameter_name, parameter_type, description, position)
             VALUES ('LEGACY', 'title', 'STRING', 'Title', 0);
             INSERT INTO prompt_translations (prompt_code, locale, display_name, prompt_body,
                translation_status, source_version, updated_at)
             VALUES ('LEGACY', 'zh-CN', '旧模板', '旧正文', 'PUBLISHED', 3, 1);"
        ).unwrap();

        migrate_prompt_agent_role_category(&mut connection).unwrap();
        migrate_prompt_agent_role_category(&mut connection).unwrap();
        let bindings: Vec<(String, Option<String>, Option<String>)> = connection
            .prepare("SELECT source_kind, managed_variable_key, runtime_provider FROM prompt_variables ORDER BY prompt_code")
            .unwrap().query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap().collect::<Result<_, _>>().unwrap();
        assert_eq!(
            bindings,
            vec![
                ("RUNTIME".into(), None, Some("RAG_TREE_TITLE".into())),
                ("MANAGED".into(), Some("MY_NAME".into()), None)
            ]
        );
        let translation: (String, u64) = connection.query_row(
            "SELECT prompt_body, source_version FROM prompt_translations WHERE prompt_code = 'LEGACY'",
            [], |row| Ok((row.get(0)?, row.get(1)?)),
        ).unwrap();
        assert_eq!(translation, ("旧正文".into(), 3));
        let identity: (String, bool, String) = connection
            .query_row(
                "SELECT category, built_in, catalog_key FROM prompts WHERE prompt_code = 'LEGACY'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(identity, ("TEMPLATE".into(), true, "legacy.catalog".into()));
        let source: String = connection
            .query_row(
                "SELECT source_prompt_code FROM prompts WHERE prompt_code = 'OVERRIDE'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(source, "LEGACY");
        assert!(
            connection
                .execute(
                    "UPDATE prompts SET category = 'AGENT_ROLE' WHERE prompt_code = 'LEGACY'",
                    []
                )
                .is_err()
        );
        connection.execute(
            "INSERT INTO prompts (id, scope_kind, prompt_code, display_name, category, prompt_body,
                enabled, source_kind, version, created_at, updated_at)
             VALUES (3, 'PLATFORM', 'ROLE', 'Role', 'AGENT_ROLE', 'Role body', 1, 'PLATFORM', 1, 1, 1)", []
        ).unwrap();
        let violation_count = connection
            .prepare("PRAGMA foreign_key_check")
            .unwrap()
            .query_map([], |_| Ok(()))
            .unwrap()
            .count();
        assert_eq!(violation_count, 0);
        assert_eq!(
            connection
                .query_row("PRAGMA foreign_keys", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            1
        );
        connection
            .execute("DELETE FROM prompts WHERE prompt_code = 'LEGACY'", [])
            .unwrap();
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM prompts WHERE prompt_code = 'OVERRIDE'",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
        for table in [
            "prompt_variables",
            "prompt_parameters",
            "prompt_translations",
        ] {
            assert_eq!(
                connection
                    .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row
                        .get::<_, i64>(0))
                    .unwrap(),
                0
            );
        }
    }

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
