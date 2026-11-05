use crate::domain::chat::{ChatError, ChatMessage, ChatSession, ChatUserInputRequest};

pub struct ChatSessionTitle {
    pub session_code: String,
    pub workspace_code: String,
    pub title: String,
}

pub trait ChatRepository: Send {
    fn validate_annotation_sources(&mut self, session: &str, annotations: &[eliteforge_agent_core::MessageAnnotation]) -> Result<(), ChatError>;
    fn session_titles(&mut self, codes: &[String]) -> Result<Vec<ChatSessionTitle>, ChatError>;
    fn sessions_by_codes(&mut self, codes: &[String]) -> Result<Vec<ChatSession>, ChatError>;
    fn pending_input_counts(
        &mut self,
        codes: &[String],
    ) -> Result<std::collections::BTreeMap<String, u32>, ChatError>;
    fn create_session(&mut self, session: &ChatSession) -> Result<(), ChatError>;
    fn delete_session(&mut self, session_code: &str) -> Result<(), ChatError>;
    fn delete_expired_heartbeat_session(&mut self, session_code: &str) -> Result<(), ChatError> {
        self.delete_session(session_code)
    }
    fn list_sessions(&mut self) -> Result<Vec<ChatSession>, ChatError>;
    fn find_source_session(
        &mut self,
        user: &str,
        source: &str,
    ) -> Result<Option<ChatSession>, ChatError>;
    fn list_archived_sessions(
        &mut self,
        workspace_code: Option<&str>,
        offset: u32,
        limit: u32,
    ) -> Result<(Vec<ChatSession>, u64), ChatError>;
    fn get_session(&mut self, session_code: &str) -> Result<ChatSession, ChatError>;
    fn update_session_status(
        &mut self,
        session_code: &str,
        status: crate::domain::chat::ChatSessionStatus,
        updated_at: &str,
    ) -> Result<(), ChatError>;
    fn update_session_title(
        &mut self,
        session_code: &str,
        title: &str,
        updated_at: &str,
    ) -> Result<(), ChatError> {
        self.update_session_settings(session_code, Some(title), None, updated_at)
    }
    fn update_session_settings(
        &mut self,
        session_code: &str,
        title: Option<&str>,
        execution_mode: Option<crate::domain::agent::ToolExecutionMode>,
        updated_at: &str,
    ) -> Result<(), ChatError>;
    fn update_session_run(
        &mut self,
        session_code: &str,
        run_code: &str,
        run_status: &str,
        agent_code: Option<&str>,
        system_agent_session_id: Option<&str>,
        updated_at: &str,
    ) -> Result<(), ChatError>;
    fn runtime_session_id(
        &mut self,
        conversation_code: &str,
        runtime_adapter_key: &str,
        agent_code: &str,
    ) -> Result<Option<String>, ChatError>;
    fn upsert_runtime_session(
        &mut self,
        conversation_code: &str,
        runtime_adapter_key: &str,
        agent_code: &str,
        runtime_session_id: &str,
        last_run_code: Option<&str>,
        updated_at: &str,
    ) -> Result<(), ChatError>;
    fn create_message(&mut self, message: &ChatMessage) -> Result<(), ChatError>;
    fn list_messages(&mut self, session_code: &str) -> Result<Vec<ChatMessage>, ChatError>;
    fn list_user_input_requests(
        &mut self,
        session_code: &str,
    ) -> Result<Vec<ChatUserInputRequest>, ChatError>;
    fn create_user_input_request(
        &mut self,
        request: &ChatUserInputRequest,
    ) -> Result<(), ChatError>;
}
