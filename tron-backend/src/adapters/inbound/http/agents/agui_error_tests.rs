use super::*;
#[tokio::test]
async fn replay_failure_is_localized_and_never_exposes_storage_detail() {
    let response = agent_agui_error_response(
        AgentAguiStreamError::ReplayIncomplete("private storage detail".into()),
        "zh-CN",
    );
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let bytes = axum::body::to_bytes(response.into_body(), 4096)
        .await
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["code"], "AGENT_AGUI_REPLAY_INCOMPLETE");
    assert_eq!(value["message"], "展示回放不完整，执行结果会单独报告");
    assert!(!String::from_utf8_lossy(&bytes).contains("private storage detail"));
}
