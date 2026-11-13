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
        let Some(prompt) = manual_session_title_prompt(self.prompt_service.as_ref(), user_message)
        else {
            self.publish_title_generation_failure(
                session_code,
                run_code,
                "TITLE_PROMPT_UNAVAILABLE",
                "标题生成提示词不可用",
            );
            return;
        };
        let preferred = match self
            .llm_router
            .as_ref()
            .ok_or_else(|| "文本生成自动路由未配置".to_string())
            .and_then(|router| {
                router.route_text(workspace_code, Some("PERSON_AI:ROUTE:TextGenerationAuto"))
            }) {
            Ok(model) => model,
            Err(message) => {
                self.publish_title_generation_failure(
                    session_code,
                    run_code,
                    "LLM_MODEL_ROUTING_FAILED",
                    message,
                );
                return;
            }
        };
        let title = match generate_title(&llm_runtime, vec![preferred], &prompt).await {
            Ok(title) => title,
            Err((code, message)) => {
                self.publish_title_generation_failure(session_code, run_code, code, message);
                return;
            }
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
        tracing::error!("session title generation failed for {session_code}: {code}");
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

async fn generate_title(
    runtime: &LlmRuntimeService,
    candidates: Vec<crate::application::model::RuntimeModelConfig>,
    prompt: &str,
) -> Result<String, (&'static str, String)> {
    let mut failure = ("LLM_MODEL_UNAVAILABLE", "标题生成模型不可用".into());
    for mut config in candidates {
        // Keep the no-reasoning preference local to this title-generation call.
        // Providers that require reasoning may apply their own default; title parsing
        // removes any returned reasoning before persisting the result.
        config.thinking_enabled = false;
        config.reasoning_effort = (config.protocol
            == crate::domain::model::ModelProtocol::Responses)
            .then_some(crate::domain::reasoning::ReasoningEffort::None);
        match tokio::time::timeout(
            TITLE_GENERATION_TIMEOUT,
            runtime.text_generation_call_with_config(config, prompt.into()),
        )
        .await
        {
            Ok(Ok(result)) => {
                if let Some(title) = normalize_generated_session_title(&result.output) {
                    return Ok(title);
                }
                failure = (
                    "TITLE_FORMAT_INVALID",
                    "模型返回的标题为空或格式无效".into(),
                );
            }
            Ok(Err(error)) => failure = (title_generation_error_code(&error), error.to_string()),
            Err(_) => failure = ("LLM_UPSTREAM_TIMEOUT", "标题生成模型调用超时".into()),
        }
    }
    Err(failure)
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
    user_message: &str,
) -> Option<String> {
    let runtime_values = HashMap::from([(
        PromptRuntimeProvider::ChatUserMessage,
        Value::String(user_message.to_string()),
    )]);
    prompt_service?
        .runtime_prompt_text("chat.session-title", "en-US", &runtime_values)
        .ok()?
}

fn normalize_generated_session_title(value: &str) -> Option<String> {
    let value = strip_title_reasoning(value)?;
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

fn strip_title_reasoning(mut remaining: &str) -> Option<String> {
    let mut output = String::new();
    let mut blocks = Vec::new();
    while let Some(start) = remaining.find('<') {
        if blocks.is_empty() {
            output.push_str(&remaining[..start]);
        }
        remaining = &remaining[start..];
        let Some(end) = remaining.find('>') else {
            // A truncated tag must not become a title either.
            return None;
        };
        let tag = remaining[1..end].trim().to_ascii_lowercase();
        let (closing, name) = match tag.strip_prefix('/') {
            Some(name) => (true, name),
            None => (false, tag.as_str()),
        };
        if matches!(name, "think" | "thinking" | "analysis" | "reasoning") {
            if closing {
                if blocks.pop().as_deref() != Some(name) {
                    return None;
                }
                if blocks.is_empty() {
                    output.push(' ');
                }
            } else {
                blocks.push(name.to_owned());
            }
        } else if blocks.is_empty() {
            output.push_str(&remaining[..=end]);
        }
        remaining = &remaining[end + 1..];
    }
    if !blocks.is_empty() {
        return None;
    }
    output.push_str(remaining);
    Some(output)
}
