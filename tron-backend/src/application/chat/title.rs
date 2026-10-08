use std::{collections::HashMap, time::Duration};

use serde_json::Value;

use crate::{
    application::{
        llm::{LlmRuntimeError, LlmRuntimeService},
        prompt::PromptService,
    },
    domain::{
        chat::{ChatSession, ChatSessionSourceType},
        prompt::PromptRuntimeProvider,
    },
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
        run_code: &str,
        llm_runtime: LlmRuntimeService,
    ) {
        let Some(prompt) =
            manual_session_title_prompt(self.prompt_service.as_ref(), workspace_code, user_message)
        else {
            self.publish_title_generation_failure(
                session_code,
                run_code,
                "TITLE_PROMPT_UNAVAILABLE",
                "标题生成提示词不可用",
            );
            return;
        };
        let Some(llm_router) = self.llm_router.as_ref() else {
            self.publish_title_generation_failure(
                session_code,
                run_code,
                "LLM_MODEL_ROUTER_UNAVAILABLE",
                "标题生成模型路由不可用",
            );
            return;
        };
        let config = match llm_router.route_text(workspace_code, None) {
            Ok(config) => config,
            Err(error) => {
                self.publish_title_generation_failure(
                    session_code,
                    run_code,
                    "LLM_MODEL_UNAVAILABLE",
                    error,
                );
                return;
            }
        };
        let result = tokio::time::timeout(
            TITLE_GENERATION_TIMEOUT,
            llm_runtime.text_generation_call_with_config(config, prompt),
        )
        .await;
        let result = match result {
            Ok(Ok(result)) => result,
            Ok(Err(error)) => {
                let code = title_generation_error_code(&error);
                self.publish_title_generation_failure(
                    session_code,
                    run_code,
                    code,
                    error.to_string(),
                );
                return;
            }
            Err(_) => {
                self.publish_title_generation_failure(
                    session_code,
                    run_code,
                    "LLM_UPSTREAM_TIMEOUT",
                    "标题生成模型调用超时",
                );
                return;
            }
        };
        let Some(title) = normalize_generated_session_title(&result.output) else {
            self.publish_title_generation_failure(
                session_code,
                run_code,
                "TITLE_FORMAT_INVALID",
                "模型返回的标题为空或格式无效",
            );
            return;
        };
        let Ok(session) = self.repository().get_session(session_code) else {
            self.publish_title_generation_failure(
                session_code,
                run_code,
                "TITLE_PERSISTENCE_FAILED",
                "读取会话失败，标题未保存",
            );
            return;
        };
        if !should_generate_manual_session_title(&session) {
            return;
        }
        let updated_at = current_timestamp();
        if self
            .repository()
            .update_session_title(session_code, &title, &updated_at)
            .is_err()
        {
            self.publish_title_generation_failure(
                session_code,
                run_code,
                "TITLE_PERSISTENCE_FAILED",
                "会话标题保存失败",
            );
        }
    }

    fn publish_title_generation_failure(
        &self,
        session_code: &str,
        run_code: &str,
        code: &str,
        message: impl Into<String>,
    ) {
        let message = message.into();
        let _ = self.event_bus.publish(
            crate::shared::event_bus::BusEventDraft::agui(
                "agent.session_title_generation_failed",
                "chat.title_generation",
            )
            .subject("agent_run", run_code)
            .stream_key(session_code)
            .summary(message.clone())
            .outcome(crate::shared::event_bus::BusOutcome::Failure)
            .payload_field("sessionCode", session_code)
            .payload_field("runCode", run_code)
            .payload_field("code", code)
            .payload_field("message", message),
        );
    }
}

fn title_generation_error_code(error: &LlmRuntimeError) -> &'static str {
    match error {
        LlmRuntimeError::TextModelUnavailable => "LLM_MODEL_UNAVAILABLE",
        LlmRuntimeError::ModelResolutionFailed(_) => "LLM_MODEL_ROUTING_FAILED",
        LlmRuntimeError::ProviderFailed(_) => "LLM_UPSTREAM_FAILED",
        LlmRuntimeError::InvalidProviderResponse(_) => "LLM_RESPONSE_INVALID",
        _ => "LLM_TITLE_GENERATION_FAILED",
    }
}

pub(super) fn should_generate_manual_session_title(session: &ChatSession) -> bool {
    session.source_type == ChatSessionSourceType::Manual
        && session.task_code.is_none()
        && is_default_session_title(&session.title)
}

fn is_default_session_title(title: &str) -> bool {
    // Older clients persisted the translated new-chat label as the title.
    [
        DEFAULT_MANUAL_SESSION_TITLE,
        DEFAULT_MAIN_AGENT_SESSION_TITLE,
        "新会话",
        "主智能体",
    ]
    .iter()
    .any(|default| title.trim().eq_ignore_ascii_case(default))
}

fn manual_session_title_prompt(
    prompt_service: Option<&PromptService>,
    workspace_code: &str,
    user_message: &str,
) -> Option<String> {
    let runtime_values = HashMap::from([(
        PromptRuntimeProvider::ChatUserMessage,
        Value::String(user_message.to_string()),
    )]);
    prompt_service?
        .runtime_prompt_text(
            Some(workspace_code),
            "chat.session-title",
            "en-US",
            &runtime_values,
        )
        .ok()?
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
    if title.is_empty() || is_default_session_title(&title) {
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
            project_code: None,
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
    fn localized_default_titles_remain_eligible_after_a_completed_run() {
        for title in ["新会话", " 主智能体 ", "new CHAT"] {
            let mut session = manual_new_chat_session(Some("RUN-1".into()));
            session.title = title.into();
            assert!(should_generate_manual_session_title(&session));
            assert!(normalize_generated_session_title(title).is_none());
        }
    }

    #[test]
    fn localized_custom_and_task_titles_are_not_regenerated() {
        let mut session = manual_new_chat_session(Some("RUN-1".into()));
        session.title = "黄女士会诊".into();
        assert!(!should_generate_manual_session_title(&session));
        assert_eq!(
            normalize_generated_session_title(&session.title).as_deref(),
            Some("黄女士会诊")
        );

        session.title = "新会话".into();
        session.task_code = Some("TASK-1".into());
        assert!(!should_generate_manual_session_title(&session));
        session.task_code = None;
        session.source_type = ChatSessionSourceType::Task;
        assert!(!should_generate_manual_session_title(&session));
    }

    #[test]
    fn renamed_manual_chat_does_not_generate_title_again() {
        let mut session = manual_new_chat_session(Some("RUN-1".into()));
        session.title = "Existing title".into();

        assert!(!should_generate_manual_session_title(&session));
    }
}
