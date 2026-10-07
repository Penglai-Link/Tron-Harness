use super::*;

impl<'a> SqliteTaskRepositorySql<'a> {
    pub(super) fn migrate_legacy_completed_task_descriptions(&mut self) -> Result<(), TaskError> {
        self.connection
            .execute(
                "UPDATE tasks SET execution_result = description, description = NULL WHERE execution_result IS NULL AND status = 'Completed' AND description IS NOT NULL AND trim(description) <> ''",
                [],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    pub(super) fn reset_legacy_schema_if_needed(&mut self) -> Result<(), TaskError> {
        let create_sql: Option<String> = self
            .connection
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'tasks'",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?;
        if create_sql
            .as_deref()
            .is_some_and(uses_legacy_task_schema_definition)
        {
            self.connection
                .execute_batch(TASKS_SUPERPOWER_RESET_MIGRATION)
                .map_err(storage_error)?;
        }
        Ok(())
    }

    pub(super) fn migrate_priority_extra_high_constraint_if_needed(
        &mut self,
    ) -> Result<(), TaskError> {
        let create_sql: Option<String> = self
            .connection
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'tasks'",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?;
        if create_sql
            .as_deref()
            .is_some_and(task_priority_constraint_excludes_extra_high)
        {
            self.rebuild_tasks_table_with_current_priority_constraint()?;
        }
        Ok(())
    }

    pub(super) fn migrate_task_flow_schema(&mut self) -> Result<(), TaskError> {
        let create_sql: Option<String> = self
            .connection
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'tasks'",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?;
        let needs_rebuild = !column_exists(&self.connection, "tasks", "flow_code")?
            || create_sql
                .as_deref()
                .is_some_and(|sql| sql.contains("status TEXT NOT NULL CHECK"));
        if needs_rebuild {
            let foreign_keys_enabled: i64 = self
                .connection
                .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
                .map_err(storage_error)?;
            self.connection
                .execute_batch("PRAGMA foreign_keys = OFF;")
                .map_err(storage_error)?;
            let result = self.rebuild_tasks_table_for_flows();
            if foreign_keys_enabled == 1 {
                self.connection
                    .execute_batch("PRAGMA foreign_keys = ON;")
                    .map_err(storage_error)?;
            }
            result?;
        }
        Ok(())
    }

    pub(super) fn rebuild_tasks_table_for_flows(&mut self) -> Result<(), TaskError> {
        let tx = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&self.connection, rusqlite::TransactionBehavior::Immediate).map_err(storage_error)?;
        tx.execute_batch(
            "
            CREATE TABLE tasks_rebuild (
                id INTEGER PRIMARY KEY,
                task_code TEXT NOT NULL UNIQUE,
                workspace_code TEXT NOT NULL,
                parent_task_code TEXT NULL REFERENCES tasks(task_code),
                title TEXT NOT NULL,
                description TEXT NULL,
                execution_result TEXT NULL,
                flow_code TEXT NOT NULL,
                status TEXT NOT NULL,
                previous_status TEXT NULL,
                priority TEXT NOT NULL CHECK (priority IN ('Low', 'Medium', 'High', 'Extra High')),
                task_type TEXT NOT NULL CHECK (task_type IN ('Feature', 'Bug')),
                source_type TEXT NOT NULL CHECK (source_type IN ('Internal', 'GitHub', 'GitLab')),
                source_url TEXT NULL,
                agent_code TEXT NOT NULL DEFAULT 'main-agent',
                model_code TEXT NULL,
                progress_value INTEGER NOT NULL DEFAULT 0 CHECK (progress_value >= 0 AND progress_value <= 100),
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                archived_at INTEGER NULL
            );
            INSERT INTO tasks_rebuild (
                id, task_code, workspace_code, parent_task_code, title, description, execution_result,
                flow_code, status, previous_status, priority, task_type, source_type, source_url,
                agent_code, model_code, progress_value, created_at, updated_at, archived_at
            )
            SELECT
                id, task_code, workspace_code, parent_task_code, title, description, execution_result,
                'DEFAULT',
                CASE status
                    WHEN 'Open' THEN 'NOT_STARTED'
                    WHEN 'Designing' THEN 'DESIGNING'
                    WHEN 'Red' THEN 'RED'
                    WHEN 'Green' THEN 'GREEN'
                    WHEN 'Regressioning' THEN 'REGRESSIONING'
                    WHEN 'Review' THEN 'REVIEW'
                    WHEN 'InProgress' THEN 'IN_PROGRESS'
                    WHEN 'Reopen' THEN 'REOPEN'
                    WHEN 'Completed' THEN 'CLOSED'
                    WHEN 'Archived' THEN 'CLOSED'
                    ELSE 'NOT_STARTED'
                END,
                CASE previous_status
                    WHEN 'Open' THEN 'NOT_STARTED'
                    WHEN 'Designing' THEN 'DESIGNING'
                    WHEN 'Red' THEN 'RED'
                    WHEN 'Green' THEN 'GREEN'
                    WHEN 'Regressioning' THEN 'REGRESSIONING'
                    WHEN 'Review' THEN 'REVIEW'
                    WHEN 'InProgress' THEN 'IN_PROGRESS'
                    WHEN 'Reopen' THEN 'REOPEN'
                    WHEN 'Completed' THEN 'CLOSED'
                    ELSE NULL
                END,
                priority, task_type, source_type, source_url, agent_code, model_code,
                progress_value, created_at, updated_at, archived_at
            FROM tasks;
            DROP TABLE tasks;
            ALTER TABLE tasks_rebuild RENAME TO tasks;
            CREATE INDEX IF NOT EXISTS idx_tasks_workspace_status ON tasks(workspace_code, status);
            CREATE INDEX IF NOT EXISTS idx_tasks_parent_task_code ON tasks(parent_task_code);
            CREATE INDEX IF NOT EXISTS idx_tasks_workspace_flow ON tasks(workspace_code, flow_code);
            ",
        )
        .map_err(storage_error)?;
        tx.commit().map_err(storage_error)
    }

    pub(super) fn assign_default_flow_to_existing_workspaces(&mut self) -> Result<(), TaskError> {
        let timestamp = chrono::Utc::now().timestamp_millis();
        self.connection
            .execute(
                "INSERT OR IGNORE INTO workspace_task_flows (workspace_code, flow_code, updated_at) SELECT DISTINCT workspace_code, 'DEFAULT', ?1 FROM tasks",
                [timestamp],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    pub(super) fn rebuild_tasks_table_with_current_priority_constraint(
        &mut self,
    ) -> Result<(), TaskError> {
        let foreign_keys_enabled: i64 = self
            .connection
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .map_err(storage_error)?;
        self.connection
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .map_err(storage_error)?;
        let result = self.rebuild_tasks_table_with_foreign_keys_disabled();
        if foreign_keys_enabled == 1 {
            self.connection
                .execute_batch("PRAGMA foreign_keys = ON;")
                .map_err(storage_error)?;
        }
        result
    }

    pub(super) fn rebuild_tasks_table_with_foreign_keys_disabled(
        &mut self,
    ) -> Result<(), TaskError> {
        let tx = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&self.connection, rusqlite::TransactionBehavior::Immediate).map_err(storage_error)?;
        tx.execute_batch(
            "
            CREATE TABLE tasks_rebuild (
                id INTEGER PRIMARY KEY,
                task_code TEXT NOT NULL UNIQUE,
                workspace_code TEXT NOT NULL,
                parent_task_code TEXT NULL REFERENCES tasks(task_code),
                title TEXT NOT NULL,
                description TEXT NULL,
                execution_result TEXT NULL,
                status TEXT NOT NULL CHECK (status IN ('Open', 'Designing', 'Red', 'Green', 'Regressioning', 'Review', 'InProgress', 'Reopen', 'Completed', 'Archived')),
                previous_status TEXT NULL CHECK (previous_status IS NULL OR previous_status IN ('Open', 'Designing', 'Red', 'Green', 'Regressioning', 'Review', 'InProgress', 'Reopen', 'Completed')),
                priority TEXT NOT NULL CHECK (priority IN ('Low', 'Medium', 'High', 'Extra High')),
                task_type TEXT NOT NULL CHECK (task_type IN ('Feature', 'Bug')),
                source_type TEXT NOT NULL CHECK (source_type IN ('Internal', 'GitHub', 'GitLab')),
                source_url TEXT NULL,
                agent_code TEXT NOT NULL DEFAULT 'main-agent',
                model_code TEXT NULL,
                progress_value INTEGER NOT NULL DEFAULT 0 CHECK (progress_value >= 0 AND progress_value <= 100),
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                archived_at INTEGER NULL
            );
            INSERT INTO tasks_rebuild (
                id,
                task_code,
                workspace_code,
                parent_task_code,
                title,
                description,
                execution_result,
                status,
                previous_status,
                priority,
                task_type,
                source_type,
                source_url,
                agent_code,
                model_code,
                progress_value,
                created_at,
                updated_at,
                archived_at
            )
            SELECT
                id,
                task_code,
                workspace_code,
                parent_task_code,
                title,
                description,
                execution_result,
                status,
                previous_status,
                priority,
                task_type,
                source_type,
                source_url,
                agent_code,
                model_code,
                progress_value,
                created_at,
                updated_at,
                archived_at
            FROM tasks;
            DROP TABLE tasks;
            ALTER TABLE tasks_rebuild RENAME TO tasks;
            CREATE INDEX IF NOT EXISTS idx_tasks_workspace_status ON tasks(workspace_code, status);
            CREATE INDEX IF NOT EXISTS idx_tasks_parent_task_code ON tasks(parent_task_code);
            ",
        )
        .map_err(storage_error)?;
        tx.commit().map_err(storage_error)
    }
}
