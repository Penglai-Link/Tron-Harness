use rusqlite::{OptionalExtension, params};

use super::{ModelProviderError, ModelProviderService, now_millis};
use crate::domain::{agent::AgentModelConfig, model_provider::ModelSelection};

impl ModelProviderService {
    pub fn chat_auto_route_selection(
        &self,
        workspace_code: &str,
        session_code: &str,
        agent_code: &str,
    ) -> Result<Option<ModelSelection>, ModelProviderError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ModelProviderError::Storage)?;
        let encoded = connection
            .query_row(
                "SELECT selection_json FROM chat_session_auto_model_routes
                 WHERE workspace_code = ?1 AND session_code = ?2 AND agent_code = ?3",
                params![workspace_code, session_code, agent_code],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|_| ModelProviderError::Storage)?;
        encoded
            .map(|value| serde_json::from_str(&value).map_err(|_| ModelProviderError::Storage))
            .transpose()
    }

    pub fn save_chat_auto_route_selection(
        &self,
        workspace_code: &str,
        session_code: &str,
        agent_code: &str,
        selection: &ModelSelection,
        model: &AgentModelConfig,
    ) -> Result<(), ModelProviderError> {
        let context_window_tokens = model
            .context_window_tokens
            .filter(|value| *value > 0)
            .ok_or_else(|| {
                ModelProviderError::InvalidSelection(
                    "Auto model route requires a valid context window".into(),
                )
            })?;
        let selection_json =
            serde_json::to_string(selection).map_err(|_| ModelProviderError::Storage)?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| ModelProviderError::Storage)?;
        connection
            .execute(
                "INSERT INTO chat_session_auto_model_routes (
                    workspace_code, session_code, agent_code, selection_json,
                    model_code, context_window_tokens, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(workspace_code, session_code, agent_code) DO UPDATE SET
                    selection_json = excluded.selection_json,
                    model_code = excluded.model_code,
                    context_window_tokens = excluded.context_window_tokens,
                    updated_at = excluded.updated_at",
                params![
                    workspace_code,
                    session_code,
                    agent_code,
                    selection_json,
                    model.model_code,
                    context_window_tokens,
                    now_millis(),
                ],
            )
            .map_err(|_| ModelProviderError::Storage)?;
        Ok(())
    }

    pub fn delete_chat_auto_routes_for_session(
        &self,
        session_code: &str,
    ) -> Result<(), ModelProviderError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ModelProviderError::Storage)?;
        connection
            .execute(
                "DELETE FROM chat_session_auto_model_routes WHERE session_code = ?1",
                params![session_code],
            )
            .map_err(|_| ModelProviderError::Storage)?;
        Ok(())
    }
}
