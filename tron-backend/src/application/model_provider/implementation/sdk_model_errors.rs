use eliteforge_agent_core as fw;
use serde_json::Value;

/// A gateway's upstream 401 can describe an unavailable model, rather than a
/// rejected Person-AI access token. Keep account recovery scoped to real auth
/// failures so Framework can apply the host-authorized Auto replacement policy.
pub(super) fn classify_upstream_failure(mut error: fw::ModelError) -> fw::ModelError {
    let fw::ModelError::Provider(detail) = &mut error else {
        return error;
    };
    let direct_message = detail.message.to_ascii_lowercase();
    let direct_model_error = detail.error_type.as_deref() == Some("invalid_request_error")
        && direct_message.contains("the model does not exist or you do not have access to it");
    if !direct_model_error && detail.error_type.as_deref() != Some("gateway_error") {
        return error;
    }
    let upstream = detail
        .message
        .find('{')
        .and_then(|start| serde_json::from_str::<Value>(&detail.message[start..]).ok());
    let unavailable = direct_model_error
        || upstream.as_ref().is_some_and(|body| {
            let code = body.pointer("/error/code").and_then(Value::as_str);
            let message = body
                .pointer("/error/message")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_ascii_lowercase();
            matches!(
                code,
                Some("model_not_found" | "model_not_available" | "invalid_model")
            ) || (body.pointer("/error/param").and_then(Value::as_str) == Some("model")
                && (message.contains("does not exist")
                    || message.contains("not found")
                    || message.contains("do not have access")))
        });
    if unavailable {
        detail.category = fw::ProviderErrorCategory::Unavailable;
        detail.error_type = Some("upstream_model_unavailable".into());
        detail.message = "The selected upstream model does not exist or is not accessible".into();
    }
    error
}
