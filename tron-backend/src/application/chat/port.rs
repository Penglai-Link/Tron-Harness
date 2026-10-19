use crate::domain::chat::{ChatError, ChatMessage, ChatSession, ChatUserInputRequest};

pub struct ChatSessionTitle {
    pub session_code: String,
    pub workspace_code: String,
    pub title: String,
}

pub trait ChatRepository: Send {
    fn validate_annotation_sources(&mut self, session: &str, annotations: &[eliteforge_agent_core::MessageAnnotation]) -> Result<(), ChatError>;
    fn session_titles(&mut self, codes: &[String]) -> Result<Vec<ChatSessionTitle>, ChatError> {
        Ok(codes.iter().filter_map(|code| self.get_session(code).ok()).map(|session| ChatSessionTitle {
            session_code: session.session_code, workspace_code: session.workspace_code, title: session.title,
        }).collect())
    }
    fn create_session(&mut self, session: &ChatSession) -> Result<(), ChatError>;
    fn delete_session(&mut self, session_code: &str) -> Result<(), ChatError>;
    fn list_sessions(&mut self) -> Result<Vec<ChatSession>, ChatError>;
    fn find_source_session(&mut self,user:&str,source:&str)->Result<Option<ChatSession>,ChatError>{
        Ok(self.list_sessions()?.into_iter().find(|session|session.user_code.as_deref()==Some(user) && session.source_code.as_deref()==Some(source) && session.status==crate::domain::chat::ChatSessionStatus::Active))
    }
    fn list_archived_sessions(
        &mut self,
        workspace_code: Option<&str>,
        offset: u32,
        limit: u32,
    ) -> Result<(Vec<ChatSession>, u64), ChatError> {
        let mut sessions = self
            .list_sessions()?
            .into_iter()
            .filter(|session| {
                session.status == crate::domain::chat::ChatSessionStatus::Archived
                    && !session
                        .source_code
                        .as_deref()
                        .is_some_and(|source| source.starts_with("__dude:"))
                    && workspace_code.is_none_or(|code| session.workspace_code == code)
            })
            .collect::<Vec<_>>();
        let total = sessions.len() as u64;
        let start = usize::try_from(offset)
            .unwrap_or(usize::MAX)
            .min(sessions.len());
        let end = start
            .saturating_add(usize::try_from(limit).unwrap_or(usize::MAX))
            .min(sessions.len());
        Ok((sessions.drain(start..end).collect(), total))
    }
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
    fn list_user_input_requests(
        &mut self,
        session_code: &str,
    ) -> Result<Vec<ChatUserInputRequest>, ChatError>;
    fn create_user_input_request(
        &mut self,
        request: &ChatUserInputRequest,
    ) -> Result<(), ChatError>;
}
