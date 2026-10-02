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
        &eliteforge_agent_memory::MemoryScope::new("WS-1", "SESSION-1").unwrap(),
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
    )
    .unwrap();

    let current_messages = prepared
        .messages
        .iter()
        .filter(|message| message.text_content().contains("second question"))
        .collect::<Vec<_>>();
    assert_eq!(current_messages.len(), 1);
    assert!(current_messages[0].has_image_parts());
    assert!(!prepared.over_budget);
    assert!(
        prepared.message_token_estimate
            > super::assembler::estimate_messages_tokens(&[AgentMessage::user("second question")])
    );

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
        &eliteforge_agent_memory::MemoryScope::new("WS-1", "SESSION-1").unwrap(),
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
    )
    .unwrap();

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

#[test]
fn context_source_cannot_set_its_own_session_or_workspace_scope() {
    let plan = ShortTermMemoryConfig::default().model_context_budget_plan(None, None);
    for (workspace, session) in [("WS-2", "SESSION-1"), ("WS-1", "SESSION-2")] {
        let scope = eliteforge_agent_memory::MemoryScope::new(workspace, session).unwrap();
        let result = assemble_context(
            &scope,
            &[],
            None,
            None,
            None,
            None,
            &[],
            None,
            vec![transcript_entry("ATM-X", None, 1, "user", "data")],
            false,
            &plan,
            false,
        );
        assert!(result.is_err());
    }
}

#[test]
fn complete_rich_input_image_and_attachment_context_fit_a_valid_budget() {
    let scope = eliteforge_agent_memory::MemoryScope::new("WS-1", "SESSION-1").unwrap();
    let input = AgentMessage::new(
        "user",
        vec![
            AgentMessagePart::Text {
                text: "inspect this attached image".into(),
            },
            AgentMessagePart::Image {
                mime_type: "image/png".into(),
                data: "a".repeat(4_000),
            },
            AgentMessagePart::Context {
                kind: "attachment".into(),
                text: "attached notes to preserve".into(),
                metadata: Some(
                    serde_json::json!({"fileRef":{"path":"images/sample.png"},"permission":"authorized"}),
                ),
            },
        ],
    );
    let plan =
        ShortTermMemoryConfig::default().model_context_budget_plan(Some(20_000), Some(1_000));
    let prepared = assemble_context(
        &scope,
        &[AgentMessage::system("inspect authorized files")],
        Some(&input),
        Some("RUN-IMAGE"),
        None,
        None,
        &[],
        None,
        vec![transcript_entry(
            "ATM-IMAGE",
            Some("RUN-IMAGE"),
            1,
            "user",
            "inspect this attached image",
        )],
        false,
        &plan,
        false,
    )
    .unwrap();
    assert!(!prepared.over_budget);
    assert!(prepared.messages.iter().any(AgentMessage::has_image_parts));
    assert!(prepared.messages.contains(&input));
    assert!(prepared.estimated_tokens > 4_000);
    assert_eq!(prepared.output_token_reserve, Some(1_000));
}

#[test]
fn disabled_mandatory_blocks_preserve_rich_input_and_still_reject_prefix_overflow() {
    let scope = eliteforge_agent_memory::MemoryScope::new("WS-1", "SESSION-1").unwrap();
    let input = AgentMessage::new(
        "user",
        vec![
            AgentMessagePart::Text {
                text: "current question".into(),
            },
            AgentMessagePart::Image {
                mime_type: "image/png".into(),
                data: "fixture-image".into(),
            },
        ],
    );
    let prefix = AgentMessage::system("mandatory guidance");
    let mut plan =
        ShortTermMemoryConfig::default().model_context_budget_plan(Some(20000), Some(1000));
    plan.block_budgets.seed_prompt_prefix.enabled = false;
    plan.block_budgets.current_session_history.enabled = false;
    for prefix_limit in [1000, 0] {
        plan.block_budgets.seed_prompt_prefix.effective_limit_tokens = prefix_limit;
        let mut prepared = assemble_context(
            &scope,
            std::slice::from_ref(&prefix),
            Some(&input),
            Some("RUN-1"),
            None,
            None,
            &[],
            None,
            vec![transcript_entry(
                "ATM-1",
                Some("RUN-1"),
                1,
                "user",
                "current question",
            )],
            false,
            &plan,
            false,
        )
        .unwrap();
        assert!(prepared.messages.contains(&prefix));
        assert!(prepared.messages.contains(&input));
        super::service_budget::apply_request_budget_fields(
            &mut prepared,
            &plan,
            &ShortTermMemoryConfig::default(),
            0,
        );
        assert_eq!(prepared.over_budget, prefix_limit == 0);
        if prefix_limit == 0 {
            assert_eq!(
                prepared.over_budget_reason.as_deref(),
                Some("SEED_PROMPT_PREFIX_OVER_BUDGET")
            );
        }
    }
}
