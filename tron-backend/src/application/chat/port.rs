use crate::domain::chat::{ChatError, ChatMessage, ChatSession};

pub trait ChatRepository: Send {
    fn next_session_id(&mut self) -> Result<i64, ChatError>;
    fn next_message_id(&mut self) -> Result<i64, ChatError>;
    fn create_session(&mut self, session: &ChatSession) -> Result<(), ChatError>;
    fn delete_session(&mut self, session_code: &str) -> Result<(), ChatError>;
    fn list_sessions(&mut self) -> Result<Vec<ChatSession>, ChatError>;
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
}
