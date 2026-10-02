use super::*;
use crate::domain::agent::{AgentNativeRunOutcome,AgentNativeRunStatus,AgentNativeInteraction,AgentNativeInteractionKind};
fn paused(kind:AgentNativeInteractionKind,presentation:Option<Value>)->AgentInvocationResult {
    AgentInvocationResult {run_code:"RUN-1".into(),conversation_code:Some("CONV-1".into()),workspace_code:"W".into(),model_code:"M".into(),runtime_adapter_key:"REACT".into(),reasoning_effort:None,
        status:"AWAITING_USER_INPUT",final_output:String::new(),step_count:1,token_usage:None,latest_request_estimate:None,latest_usage_audit:None,system_agent_session_id:None,observations:vec![],sdk_execution:None,
        native:Some(AgentNativeRunOutcome{run_id:"native-run".into(),status:AgentNativeRunStatus::AwaitingInput,interactions:vec![AgentNativeInteraction{id:"native-input".into(),kind,prompt:json!([{"type":"text","text":"preserve the native prompt"}]),response_schema:json!({"type":"boolean"}),presentation,expires_at_unix_ms:Some(999999)}]})}
}
#[test]
fn generic_input_and_approval_do_not_require_legacy_question_observations_or_change_native_schema() {
    for kind in [AgentNativeInteractionKind::Input,AgentNativeInteractionKind::Approval{invocation_id:"original-tool-call".into()}] {
        let run=paused(kind,None);let original=serde_json::to_value(&run.native).unwrap();
        let (_,card)=paused_run_presentation(&run).unwrap();assert!(card.is_none());
        assert_eq!(serde_json::to_value(&run.native).unwrap(),original);
        assert_eq!(run.native.as_ref().unwrap().interactions[0].response_schema,json!({"type":"boolean"}));
    }
}
#[test]
fn native_question_presentation_retains_legacy_card_and_original_interaction_id() {
    let run=paused(AgentNativeInteractionKind::Input,Some(json!({"kind":"ask_user","intro":"Choose","questions":[{"id":"q1","question":"Which?","options":[{"id":"a","label":"A","description":null}]}]})));
    let (content,card)=paused_run_presentation(&run).unwrap();let card=card.unwrap();
    assert_eq!(content,"Choose");assert_eq!(card.framework_interaction_id.as_deref(),Some("native-input"));assert_eq!(card.questions.len(),1);
}
#[test]
fn malformed_legacy_or_empty_native_pauses_remain_explicit_errors() {
    let mut run=paused(AgentNativeInteractionKind::Input,None);run.native.as_mut().unwrap().interactions.clear();assert!(paused_run_presentation(&run).is_err());
    run.native=None;assert!(paused_run_presentation(&run).is_err());
}

#[test]
fn final_goal_response_preserves_actual_child_pause_identity_and_schema() {
    let mut initial=paused(AgentNativeInteractionKind::Input,None);
    initial.run_code="INITIAL".into();initial.status="COMPLETED";
    initial.native.as_mut().unwrap().status=AgentNativeRunStatus::Completed;
    initial.native.as_mut().unwrap().interactions.clear();
    let history=initial.clone();
    let mut child=paused(AgentNativeInteractionKind::Approval{invocation_id:"actual-child-tool".into()},None);
    child.run_code="CHILD".into();child.native.as_mut().unwrap().run_id="actual-sdk-child".into();
    let expected=serde_json::to_value(&child).unwrap();
    let mut response=initial;let mut messages=vec![];let mut card=None;
    GoalContinuationResult {messages:vec![],user_input_request:None,final_run:Some(child)}.merge_into(&mut response,&mut messages,&mut card);
    assert_eq!(serde_json::to_value(&response).unwrap(),expected);
    assert_eq!(response.status,"AWAITING_USER_INPUT");assert!(card.is_none());
    assert_eq!(history.run_code,"INITIAL");assert_eq!(history.status,"COMPLETED");
    assert_eq!(response.native.as_ref().unwrap().interactions[0].response_schema,json!({"type":"boolean"}));
}
#[test]
fn final_goal_response_preserves_actual_child_limits_and_empty_result_keeps_initial() {
    let initial=paused(AgentNativeInteractionKind::Input,None);
    let mut response=initial.clone();let mut messages=vec![];let mut card=None;
    GoalContinuationResult {messages:vec![],user_input_request:None,final_run:None}.merge_into(&mut response,&mut messages,&mut card);
    assert_eq!(serde_json::to_value(&response).unwrap(),serde_json::to_value(&initial).unwrap());
    for (status,native_status) in [("STEP_LIMIT",AgentNativeRunStatus::StepLimit),("TOOL_CALL_LIMIT",AgentNativeRunStatus::ToolCallLimit)] {
        let mut child=initial.clone();child.run_code="LIMIT-CHILD".into();child.status=status;
        child.native.as_mut().unwrap().status=native_status;
        let expected=serde_json::to_value(&child).unwrap();
        GoalContinuationResult {messages:vec![],user_input_request:None,final_run:Some(child)}.merge_into(&mut response,&mut messages,&mut card);
        assert_eq!(serde_json::to_value(&response).unwrap(),expected);
    }
}
