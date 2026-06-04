use super::*;

impl ModelService {
    async fn verify_model_definition(&self, model: &Model) -> Result<VerifyOutcome, ModelError> {
        let endpoint_path = model
            .api_path
            .clone()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| default_verify_path(model.protocol, model.model_type).into());
        let url = join_verify_url(&model.base_url, &endpoint_path)?;
        let payload = verify_request_payload(model.protocol, model.model_type, &model.model_name);
        let attempts = model.max_retries.unwrap_or(0).saturating_add(1);
        let api_key = self.resolve_api_key_value(model)?;
        let method_label = format!("POST {}", url.path());
        let mut last_error = None;

        for _ in 0..attempts {
            let response = self
                .http_client
                .post(url.clone())
                .bearer_auth(api_key.clone())
                .json(&payload)
                .send()
                .await;
            match response {
                Ok(response) if response.status().is_success() => {
                    return Ok(VerifyOutcome {
                        message: format!("Verified via {method_label}"),
                        status: ModelVerifyStatus::Succeeded,
                    });
                }
                Ok(response) => {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    last_error = Some(format!(
                        "{method_label} returned {status}{}",
                        if body.trim().is_empty() {
                            String::new()
                        } else {
                            format!(": {}", truncate_message(&body))
                        }
                    ));
                }
                Err(error) => {
                    last_error = Some(format!(
                        "{method_label} failed: {}",
                        truncate_message(&error.to_string())
                    ));
                }
            }
        }

        Err(ModelError::VerificationFailed(
            last_error.unwrap_or_else(|| format!("{method_label} failed")),
        ))
    }

    fn resolve_api_key_value(&self, model: &Model) -> Result<String, ModelError> {
        let reference = model.api_key_ref.trim();
        if reference.is_empty() {
            return Err(ModelError::ApiKeyRefRequired);
        }
        Ok(reference.to_string())
    }

    pub(super) async fn verify_model_and_persist(&self, model: Model) -> Result<Model, ModelError> {
        match self.verify_model_definition(&model).await {
            Ok(outcome) => self.apply_verify_outcome(model, outcome),
            Err(ModelError::VerificationFailed(message)) => {
                self.apply_verify_outcome(
                    model,
                    VerifyOutcome {
                        message: message.clone(),
                        status: ModelVerifyStatus::Failed,
                    },
                )?;
                Err(ModelError::VerificationFailed(message))
            }
            Err(error) => Err(error),
        }
    }

    pub(super) async fn verify_model_preview(&self, model: Model) -> Result<Model, ModelError> {
        match self.verify_model_definition(&model).await {
            Ok(outcome) => Ok(apply_verify_outcome_to_model(model, outcome)),
            Err(error) => Err(error),
        }
    }

    fn apply_verify_outcome(
        &self,
        model: Model,
        outcome: VerifyOutcome,
    ) -> Result<Model, ModelError> {
        let model = apply_verify_outcome_to_model(model, outcome);
        self.repository
            .lock()
            .expect("model repository mutex poisoned")
            .update_model(&model)?;
        Ok(model)
    }
}
