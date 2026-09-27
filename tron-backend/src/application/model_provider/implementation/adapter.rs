use super::*;

#[derive(Debug, Clone, Default)]
pub struct PersonAiAdapter;

impl ModelProviderAdapter for PersonAiAdapter {
    fn code(&self) -> ModelProviderCode {
        ModelProviderCode::PersonAi
    }

    fn create_authorization<'a>(
        &'a self,
        server_base_url: &'a str,
        client_name: &'a str,
        existing_client_id: Option<&'a str>,
        device_id: &'a str,
    ) -> ProviderFuture<'a, ExternalAuthorizationSession> {
        Box::pin(async move {
            let client = LoginClient::new(server_base_url)
                .map_err(|_| ModelProviderError::InvalidServerBaseUrl)?;
            let session = client
                .create_session(&SessionRequest {
                    client_id: existing_client_id.map(str::to_owned),
                    client_name: client_name.to_owned(),
                    callback_url: None,
                })
                .await
                .map_err(server_api_error)?;
            let computer_name = LoginClient::detect_computer_name();
            let generated_login_url = client.build_login_url_with_device(
                &session.session_id,
                Some(LoginClient::detect_os_type()),
                Some(&computer_name),
                Some(device_id),
            );
            let login_url = match session.login_url {
                Some(login_url) => match Url::parse(&login_url) {
                    Ok(mut login_url) => {
                        let generated_query = Url::parse(&generated_login_url)
                            .map(|url| {
                                url.query_pairs()
                                    .map(|(key, value)| (key.into_owned(), value.into_owned()))
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default();
                        let generated_keys = generated_query
                            .iter()
                            .map(|(key, _)| key.as_str())
                            .collect::<Vec<_>>();
                        let mut query = login_url
                            .query_pairs()
                            .filter(|(key, _)| !generated_keys.contains(&key.as_ref()))
                            .map(|(key, value)| (key.into_owned(), value.into_owned()))
                            .collect::<Vec<_>>();
                        query.extend(generated_query);
                        login_url.query_pairs_mut().clear().extend_pairs(query);
                        login_url.to_string()
                    }
                    Err(_) => generated_login_url,
                },
                None => generated_login_url,
            };
            Ok(ExternalAuthorizationSession {
                session_id: session.session_id,
                login_url,
                status: session.status,
                expires_at: session
                    .expires_at
                    .as_deref()
                    .and_then(parse_remote_timestamp),
            })
        })
    }

    fn poll_authorization<'a>(
        &'a self,
        server_base_url: &'a str,
        session_id: &'a str,
    ) -> ProviderFuture<'a, ExternalAuthorizationPoll> {
        Box::pin(async move {
            let client = LoginClient::new(server_base_url)
                .map_err(|_| ModelProviderError::InvalidServerBaseUrl)?;
            let session = client
                .get_session(session_id)
                .await
                .map_err(server_api_error)?;
            let expires_at = session
                .expires_at
                .as_deref()
                .and_then(parse_remote_timestamp);
            if session.is_completed() {
                let credentials = ProviderCredentials {
                    token: required_remote_value(session.token, "token")?,
                    refresh_token: required_remote_value(session.refresh_token, "refreshToken")?,
                    client_id: required_remote_value(session.client_id, "clientId")?,
                    public_key: required_remote_value(session.public_key, "publicKey")?,
                    server_base_url: server_base_url.to_owned(),
                    authorization_expires_at: session
                        .expires_in
                        .map(|seconds| now_millis().saturating_add(seconds.saturating_mul(1000))),
                };
                return Ok(ExternalAuthorizationPoll::Completed(credentials));
            }
            if session.is_finished() {
                return Ok(ExternalAuthorizationPoll::Ended {
                    status: session.status,
                });
            }
            Ok(ExternalAuthorizationPoll::Pending {
                status: session.status,
                expires_at,
            })
        })
    }

    fn refresh_credentials<'a>(
        &'a self,
        server_base_url: &'a str,
        refresh_token: &'a str,
    ) -> ProviderFuture<'a, ProviderCredentials> {
        Box::pin(async move {
            let client = LoginClient::new(server_base_url)
                .map_err(|_| ModelProviderError::InvalidServerBaseUrl)?;
            let refreshed = client
                .refresh_token(refresh_token)
                .await
                .map_err(server_api_error)?;
            Ok(ProviderCredentials {
                token: refreshed.token,
                refresh_token: refreshed
                    .refresh_token
                    .unwrap_or_else(|| refresh_token.to_owned()),
                client_id: String::new(),
                public_key: String::new(),
                server_base_url: server_base_url.to_owned(),
                authorization_expires_at: refreshed
                    .expires_in
                    .map(|seconds| now_millis().saturating_add(seconds.saturating_mul(1000))),
            })
        })
    }

    fn list_chat_models<'a>(
        &'a self,
        credentials: &'a ProviderCredentials,
    ) -> ProviderFuture<'a, Vec<ExternalCatalogModel>> {
        Box::pin(async move {
            let client = ChatCompletionsClient::new(credentials.server_base_url.clone())
                .map_err(|_| ModelProviderError::InvalidServerBaseUrl)?;
            let context = credentials.context();
            let mut distinct = BTreeMap::<String, ExternalCatalogModel>::new();
            for quality in [QualityType::High, QualityType::Middle, QualityType::Low] {
                for model in client
                    .list_models(&context, Some(quality), None, None)
                    .await
                    .map_err(server_api_error)?
                {
                    distinct.entry(model.model_code.clone()).or_insert_with(|| {
                        ExternalCatalogModel {
                            model_code: model.model_code.clone(),
                            model_name: model.model_name.unwrap_or(model.model_code),
                            protocols: model
                                .protocol_types
                                .unwrap_or_default()
                                .into_iter()
                                .map(agent_protocol_from_sdk)
                                .collect(),
                            upstream_vendor_code: model.provider_code,
                            upstream_vendor_name: model.provider_name,
                            context_window_tokens: model
                                .context_length
                                .and_then(|value| u32::try_from(value).ok()),
                            quality_type: provider_quality_from_sdk(quality),
                            input_coefficient: model.input_coefficient,
                            cache_hit_coefficient: model.cache_hit_coefficient,
                            output_coefficient: model.output_coefficient,
                        }
                    });
                }
            }
            Ok(distinct.into_values().collect())
        })
    }

    fn list_multimedia_models<'a>(
        &'a self,
        credentials: &'a ProviderCredentials,
    ) -> ProviderFuture<'a, Vec<ExternalMultimediaCatalogModel>> {
        Box::pin(async move {
            let client = MultimediaClient::new(credentials.server_base_url.clone())
                .map_err(|_| ModelProviderError::InvalidServerBaseUrl)?;
            let context = credentials.context();
            let mut distinct = BTreeMap::<String, ExternalMultimediaCatalogModel>::new();
            for model in client
                .list_models(&context, None)
                .await
                .map_err(server_api_error)?
            {
                let Some(catalog_model_type) =
                    ProviderCatalogModelType::from_person_ai_media_type(model.media_type)
                else {
                    continue;
                };
                // Embeddings are a Person-AI managed capability, not a
                // selectable remote model. Keep them out of the provider
                // catalogue even if a future upstream list includes type 5.
                if catalog_model_type == ProviderCatalogModelType::Embedding {
                    continue;
                }
                distinct.entry(model.model_id.clone()).or_insert_with(|| {
                    ExternalMultimediaCatalogModel {
                        model_id: model.model_id,
                        model_name: model.model_name.unwrap_or_else(|| model.model_code.clone()),
                        model_code: model.model_code,
                        catalog_model_type,
                    }
                });
            }
            Ok(distinct.into_values().collect())
        })
    }

    fn verify_chat_auto<'a>(
        &'a self,
        credentials: &'a ProviderCredentials,
    ) -> ProviderFuture<'a, ()> {
        Box::pin(async move {
            let client = ChatCompletionsClient::new(credentials.server_base_url.clone())
                .map_err(|_| ModelProviderError::InvalidServerBaseUrl)?;
            let response = client
                .create(
                    &credentials.context(),
                    &json!({
                        "messages": [{
                            "role": "user",
                            "content": "Reply with OK."
                        }],
                        "max_tokens": 8,
                    })
                    .to_string(),
                    Some(QualityType::High),
                    None,
                )
                .await
                .map_err(server_api_error)?;
            if response.get("error").is_some() {
                return Err(ModelProviderError::Remote(
                    "Person-AI high quality Auto chat verification failed".into(),
                ));
            }
            Ok(())
        })
    }

    fn verify_embedding<'a>(
        &'a self,
        credentials: &'a ProviderCredentials,
    ) -> ProviderFuture<'a, ()> {
        Box::pin(async move {
            let client = EmbeddingsClient::new(credentials.server_base_url.clone())
                .map_err(|_| ModelProviderError::InvalidServerBaseUrl)?;
            let response = client
                .create(
                    &credentials.context(),
                    &json!({ "input": "Tron Person-AI embedding connectivity check" }).to_string(),
                )
                .await
                .map_err(server_api_error)?;
            let valid = response
                .get("data")
                .and_then(Value::as_array)
                .and_then(|items| items.first())
                .and_then(|item| item.get("embedding"))
                .and_then(Value::as_array)
                .is_some_and(|embedding| !embedding.is_empty());
            if valid {
                // Person-AI owns the active embedding model and routes this
                // request internally. This only verifies the required
                // capability; it must never synthesize a catalog model.
                Ok(())
            } else {
                Err(ModelProviderError::Remote(
                    "Person-AI embedding verification returned no vector".into(),
                ))
            }
        })
    }

    fn chat<'a>(
        &'a self,
        credentials: &'a ProviderCredentials,
        body: &'a Value,
        quality_type: ProviderModelQuality,
        conversation_id: Option<&'a str>,
    ) -> ProviderFuture<'a, Value> {
        Box::pin(async move {
            let client = ChatCompletionsClient::new(credentials.server_base_url.clone())
                .map_err(|_| ModelProviderError::InvalidServerBaseUrl)?;
            client
                .create(
                    &credentials.context(),
                    &body.to_string(),
                    Some(provider_quality_to_sdk(quality_type)),
                    conversation_id,
                )
                .await
                .map_err(person_ai_runtime_error)
        })
    }

    fn chat_stream<'a>(
        &'a self,
        credentials: &'a ProviderCredentials,
        body: &'a Value,
        quality_type: ProviderModelQuality,
        conversation_id: Option<&'a str>,
        sink: Arc<dyn Fn(Value) -> Result<(), String> + Send + Sync + 'a>,
    ) -> ProviderFuture<'a, ()> {
        Box::pin(async move {
            let client = ChatCompletionsClient::new(credentials.server_base_url.clone())
                .map_err(|_| ModelProviderError::InvalidServerBaseUrl)?;
            client
                .create_stream(
                    &credentials.context(),
                    &body.to_string(),
                    provider_quality_to_sdk(quality_type),
                    conversation_id,
                    |data| {
                        if data.trim().is_empty() || data.trim() == "[DONE]" {
                            return Ok(());
                        }
                        let event = serde_json::from_str(data).map_err(|error| {
                            crab_serverapi::ServerApiError::Validation(format!(
                                "Person-AI stream returned invalid JSON: {error}"
                            ))
                        })?;
                        sink(event).map_err(crab_serverapi::ServerApiError::Validation)
                    },
                )
                .await
                .map_err(person_ai_runtime_error)
        })
    }

    fn embed<'a>(
        &'a self,
        credentials: &'a ProviderCredentials,
        body: &'a Value,
    ) -> ProviderFuture<'a, Value> {
        Box::pin(async move {
            let client = EmbeddingsClient::new(credentials.server_base_url.clone())
                .map_err(|_| ModelProviderError::InvalidServerBaseUrl)?;
            client
                .create(&credentials.context(), &body.to_string())
                .await
                .map_err(server_api_error)
        })
    }
}
