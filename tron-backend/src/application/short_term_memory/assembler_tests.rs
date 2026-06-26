use crate::{
    application::short_term_memory::ShortTermMemoryConfig,
    domain::{
        agent::{AgentMessage, AgentMessagePart},
        short_term_memory::{ShortTermMemoryMessage, ShortTermMemoryPart},
    },
};

use super::assembler::assemble_context;

#[test]
fn current_input_replaces_text_only_transcript_message() {
    let budget_plan = ShortTermMemoryConfig::default().model_context_budget_plan(None, None);
    let current_input = AgentMessage::new(
        "user",
        vec![
            AgentMessagePart::Text {
                text: "second question".into(),
            },
            AgentMessagePart::Context {
                kind: "page_comment".into(),
                text: "<page-comment>selected target</page-comment>".into(),
                metadata: None,
            },
            AgentMessagePart::Image {
                mime_type: "image/png".into(),
                data: "iVBORw0KGgo=".into(),
            },
        ],
    );

    let prepared = assemble_context(
        &[AgentMessage::system("system guidance")],
        Some(&current_input),
        Some("RUN-2"),
        None,
        None,
        &[],
        None,
        vec![
            transcript_entry("ATM-1", Some("RUN-1"), 1, "user", "first question"),
            transcript_entry("ATM-2", Some("RUN-1"), 2, "assistant", "first answer"),
            transcript_entry("ATM-3", Some("RUN-2"), 3, "user", "second question"),
            transcript_entry("ATM-4", Some("RUN-2"), 4, "assistant", "tool call response"),
        ],
        false,
        &budget_plan,
        false,
    );

    let current_messages = prepared
        .messages
        .iter()
        .filter(|message| message.text_content().contains("second question"))
        .collect::<Vec<_>>();
    assert_eq!(current_messages.len(), 1);
    assert!(current_messages[0].has_image_parts());

    let current_index = prepared
        .messages
        .iter()
        .position(|message| message.has_image_parts())
        .expect("current rich input should be present");
    let assistant_after_current = prepared
        .messages
        .iter()
        .skip(current_index + 1)
        .any(|message| message.text_content().contains("tool call response"));
    assert!(assistant_after_current);
}

#[test]
fn current_input_without_rich_replacement_keeps_transcript_message() {
    let budget_plan = ShortTermMemoryConfig::default().model_context_budget_plan(None, None);

    let prepared = assemble_context(
        &[AgentMessage::system("system guidance")],
        None,
        Some("RUN-2"),
        None,
        None,
        &[],
        None,
        vec![
            transcript_entry("ATM-1", Some("RUN-1"), 1, "user", "first question"),
            transcript_entry("ATM-2", Some("RUN-2"), 2, "user", "second question"),
        ],
        false,
        &budget_plan,
        false,
    );

    let text = prepared
        .messages
        .iter()
        .map(AgentMessage::text_content)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(text.contains("first question"));
    assert!(text.contains("second question"));
}

fn transcript_entry(
    message_code: &str,
    run_code: Option<&str>,
    seq: i64,
    role: &str,
    text: &str,
) -> (ShortTermMemoryMessage, Vec<ShortTermMemoryPart>) {
    let now = "2026-06-26T00:00:00Z".to_string();
    (
        ShortTermMemoryMessage {
            id: seq,
            message_code: message_code.into(),
            session_code: "SESSION-1".into(),
            workspace_code: "WS-1".into(),
            run_code: run_code.map(str::to_string),
            seq,
            role: role.into(),
            message_type: "normal".into(),
            status: "completed".into(),
            agent_code: None,
            model_code: None,
            model_provider: None,
            model_name: None,
            model_protocol: None,
            finish_reason: None,
            error_json: None,
            input_tokens: None,
            output_tokens: None,
            reasoning_tokens: None,
            total_tokens: None,
            metadata_json: None,
            created_at: now.clone(),
            completed_at: Some(now.clone()),
        },
        vec![ShortTermMemoryPart {
            id: seq,
            part_code: format!("ATP-{seq}"),
            session_code: "SESSION-1".into(),
            workspace_code: "WS-1".into(),
            message_code: message_code.into(),
            seq: 1,
            part_type: "text".into(),
            status: "completed".into(),
            content_text: Some(text.into()),
            content_json: None,
            provider_metadata_json: None,
            visibility: "model".into(),
            created_at: now.clone(),
            completed_at: Some(now),
        }],
    )
}
