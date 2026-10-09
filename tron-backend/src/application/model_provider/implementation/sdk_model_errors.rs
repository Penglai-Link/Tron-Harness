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

#[cfg(test)]
mod tests {
    use super::*;

    fn provider(message: &str, error_type: &str) -> fw::ModelError {
        fw::ModelError::Provider(Box::new(fw::ProviderModelError {
            authorization_attempt: Some("attempt".into()),
            provider: "personai".into(),
            category: fw::ProviderErrorCategory::Authentication,
            http_status: Some(401),
            business_code: None,
            error_type: Some(error_type.into()),
            model: None,
            message: message.into(),
        }))
    }

    #[test]
    fn upstream_model_401_allows_replacement_without_refreshing_user_credentials() {
        let error = classify_upstream_failure(provider(
            "上游返回 HTTP 401: {\"error\":{\"code\":\"\",\"message\":\"The model does not exist or you do not have access to it.\",\"param\":\"model\",\"type\":\"invalid_request_error\"},\"id\":\"fixture\"}",
            "gateway_error",
        ));
        assert!(error.is_unavailable());
        let fw::ModelError::Provider(detail) = error else {
            panic!("provider error expected")
        };
        assert_eq!(detail.authorization_attempt.as_deref(), Some("attempt"));
        assert_eq!(detail.http_status, Some(401));
        assert!(!detail.message.contains('{'));
    }

    #[test]
    fn direct_upstream_model_401_also_allows_replacement() {
        let error = classify_upstream_failure(provider(
            "The model does not exist or you do not have access to it.",
            "invalid_request_error",
        ));
        assert!(error.is_unavailable());
    }

    #[test]
    fn real_auth_device_and_unknown_gateway_failures_keep_their_classification() {
        for (message, kind) in [
            ("access token expired", "authentication_error"),
            ("device_unbound", "device_unbound"),
            ("API key is invalid", "invalid_request_error"),
            (
                "上游返回 HTTP 401: {\"error\":{\"message\":\"API key is invalid\"}}",
                "gateway_error",
            ),
            ("malformed upstream response {", "gateway_error"),
        ] {
            let original = provider(message, kind);
            let classified = classify_upstream_failure(original.clone());
            assert_eq!(classified.to_string(), original.to_string());
            let fw::ModelError::Provider(detail) = classified else {
                panic!("provider error expected")
            };
            assert_eq!(detail.category, fw::ProviderErrorCategory::Authentication);
            assert_eq!(detail.error_type.as_deref(), Some(kind));
        }
    }
}
