use super::*;
use crate::domain::agent::{
    AgentNativeInteraction, AgentNativeInteractionKind, AgentNativeRunOutcome, AgentNativeRunStatus,
};
#[test]
fn missing_presentation_preserves_native_pause_and_approval_identity() {
    let native = AgentNativeRunOutcome {
        run_id: "LOGICAL".into(),
        status: AgentNativeRunStatus::AwaitingInput,
        interactions: vec![AgentNativeInteraction {
            id: "INTERACTION".into(),
            kind: AgentNativeInteractionKind::Approval {
                invocation_id: "TOOL".into(),
            },
            prompt: serde_json::to_value(eliteforge_agent_core::Content::from(
                "Approve this operation",
            ))
            .unwrap(),
            response_schema: json!({"type":"boolean"}),
            presentation: Some(json!({"kind":"approval"})),
            expires_at_unix_ms: None,
        }],
    };
    let outcome = native_interrupt_outcome(Some(&native)).unwrap();
    assert_eq!(outcome["type"], "interrupt");
    assert_eq!(outcome["interrupts"][0]["id"], "INTERACTION");
    assert_eq!(outcome["interrupts"][0]["toolCallId"], "TOOL");
    assert_eq!(
        outcome["interrupts"][0]["responseSchema"],
        json!({"type":"boolean"})
    );
    assert_eq!(
        outcome["interrupts"][0]["message"],
        "Approve this operation"
    );
    assert_eq!(native.status, AgentNativeRunStatus::AwaitingInput);
}
