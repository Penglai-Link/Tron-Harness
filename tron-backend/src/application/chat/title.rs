use std::time::Duration;

use crate::{
    application::llm::{LlmRuntimeService, LlmTextGenerationCallCommand},
    domain::chat::{ChatSession, ChatSessionSourceType},
    shared::time::current_timestamp,
};

use super::service::ChatService;

pub(super) const DEFAULT_MANUAL_SESSION_TITLE: &str = "New chat";
pub(super) const DEFAULT_MAIN_AGENT_SESSION_TITLE: &str = "Main Agent";
const MAX_GENERATED_TITLE_CHARS: usize = 64;
const TITLE_GENERATION_TIMEOUT: Duration = Duration::from_secs(30);

impl ChatService {
    pub(super) async fn try_generate_manual_session_title(
        &self,
        session_code: &str,
        workspace_code: &str,
        user_message: &str,
        llm_runtime: LlmRuntimeService,
    ) {
        let prompt = manual_session_title_prompt(user_message);
        let result = tokio::time::timeout(
            TITLE_GENERATION_TIMEOUT,
            llm_runtime.text_generation_call(LlmTextGenerationCallCommand {
                workspace_code: workspace_code.to_string(),
                model_code: None,
                prompt,
            }),
        )
        .await;
        let Ok(Ok(result)) = result else {
            return;
        };
        let Some(title) = normalize_generated_session_title(&result.output) else {
            return;
        };
        let Ok(session) = self.repository().get_session(session_code) else {
            return;
        };
        if !should_generate_manual_session_title(&session) {
            return;
        }
        let updated_at = current_timestamp();
        let _ = self
            .repository()
            .update_session_title(session_code, &title, &updated_at);
    }
}

pub(super) fn should_generate_manual_session_title(session: &ChatSession) -> bool {
    session.source_type == ChatSessionSourceType::Manual
        && session.task_code.is_none()
        && (session
            .title
            .trim()
            .eq_ignore_ascii_case(DEFAULT_MANUAL_SESSION_TITLE)
            || session
                .title
                .trim()
                .eq_ignore_ascii_case(DEFAULT_MAIN_AGENT_SESSION_TITLE))
}

fn manual_session_title_prompt(user_message: &str) -> String {
    format!(
        "Generate a concise chat title from the user's first message.\n\
        Rules:\n\
        - Return only the title.\n\
        - Use 3 to 8 words.\n\
        - No quotes, punctuation-only decorations, markdown, or prefixes.\n\
        - Preserve product names and technical terms.\n\n\
        User message:\n{user_message}"
    )
}

fn normalize_generated_session_title(value: &str) -> Option<String> {
    let first_line = value.lines().map(str::trim).find(|line| !line.is_empty())?;
    let mut title = first_line
        .trim_matches(|character: char| {
            character == '"'
                || character == '\''
                || character == '`'
                || character == '*'
                || character == '#'
                || character == '-'
                || character == ':'
                || character.is_whitespace()
        })
        .trim()
        .to_string();
    for prefix in ["Title:", "title:", "Chat title:", "chat title:"] {
        if let Some(stripped) = title.strip_prefix(prefix) {
            title = stripped.trim().to_string();
            break;
        }
    }
    title = title
        .chars()
        .filter(|character| !character.is_control())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if title.is_empty()
        || title.eq_ignore_ascii_case(DEFAULT_MANUAL_SESSION_TITLE)
        || title.eq_ignore_ascii_case(DEFAULT_MAIN_AGENT_SESSION_TITLE)
    {
        return None;
    }
    if title.chars().count() > MAX_GENERATED_TITLE_CHARS {
        title = title.chars().take(MAX_GENERATED_TITLE_CHARS).collect();
        title = title.trim().to_string();
    }
    (!title.is_empty()).then_some(title)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::chat::ChatSessionStatus;

    fn manual_new_chat_session(last_run_code: Option<String>) -> ChatSession {
        ChatSession {
            id: 1,
            session_code: "SESSION-1".into(),
            workspace_code: "TRON_CORE".into(),
            app_code: None,
            task_code: None,
            source_type: ChatSessionSourceType::Manual,
            source_code: None,
            title: DEFAULT_MANUAL_SESSION_TITLE.into(),
            status: ChatSessionStatus::Active,
            agent_code: Some("main-agent".into()),
            last_run_code,
            last_run_status: None,
            system_agent_session_id: None,
            user_code: Some("user".into()),
            created_at: "1".into(),
            updated_at: "1".into(),
        }
    }

    #[test]
    fn manual_new_chat_title_generation_retries_after_first_run() {
        let session = manual_new_chat_session(Some("RUN-1".into()));

        assert!(should_generate_manual_session_title(&session));
    }

    #[test]
    fn main_agent_default_title_is_generated_after_first_run() {
        let mut session = manual_new_chat_session(Some("RUN-1".into()));
        session.title = DEFAULT_MAIN_AGENT_SESSION_TITLE.into();

        assert!(should_generate_manual_session_title(&session));
    }

    #[test]
    fn renamed_manual_chat_does_not_generate_title_again() {
        let mut session = manual_new_chat_session(Some("RUN-1".into()));
        session.title = "Existing title".into();

        assert!(!should_generate_manual_session_title(&session));
    }
}
