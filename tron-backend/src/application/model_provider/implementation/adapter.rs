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
            let mut distinct = BTreeMap::new();
            for quality in [QualityType::High, QualityType::Middle, QualityType::Low] {
                for model in client
                    .list_models(&context, Some(quality), None, None)
                    .await
                    .map_err(server_api_error)?
                {
                    let key = (
                        model.provider_code.clone().unwrap_or_default(),
                        model.model_code.clone(),
                        quality.as_str(),
                    );
                    distinct.entry(key).or_insert_with(|| ExternalCatalogModel {
                        tags: model.tags.clone(),
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
                distinct.entry(model.model_id.clone()).or_insert_with(|| {
                    ExternalMultimediaCatalogModel {
                        upstream_vendor_code: model.provider_code,
                        upstream_vendor_name: model.provider_name,
                        input_coefficient: model.input_coefficient,
                        cache_hit_coefficient: model.cache_hit_coefficient,
                        output_coefficient: model.output_coefficient,
                        model_id: model.model_id,
                        model_name: model.model_name.unwrap_or_else(|| model.model_code.clone()),
                        model_code: model.model_code,
                        catalog_model_type,
                        dimension: model.dimension,
                        tags: model.tags,
                    }
                });
            }
            Ok(distinct.into_values().collect())
        })
    }
}
