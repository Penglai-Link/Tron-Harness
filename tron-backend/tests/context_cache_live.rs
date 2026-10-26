//! Opt-in, billable A/B probe through the production PersonAI SDK transport.
//! Synthetic content only; never serialize configuration, credentials or responses.
use eliteforge_agent_core as fw;
use serde_json::json;
use std::{sync::Arc, time::Duration};
use tron_backend::{
    adapters::outbound::{
        crypto::Sm4GcmVariableCipher,
        model::{HttpModelClient, HttpModelProviderClient, PersonAiModelProviderClient},
    },
    application::{agent::ModelClient, model_provider::ModelProviderService},
    config::VariableCryptoConfig,
    domain::agent::ModelRequest,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "billable: requires TRON_CONTEXT_CACHE_LIVE=1 and an authorized local profile"]
async fn compare_dynamic_prefix_with_dynamic_suffix_using_actual_provider_usage() {
    assert_eq!(std::env::var("TRON_CONTEXT_CACHE_LIVE").as_deref(), Ok("1"));
    let home = std::path::PathBuf::from(std::env::var("HOME").unwrap());
    let yaml: serde_yaml::Value = serde_yaml::from_str(
        &std::fs::read_to_string(home.join(".tron/application.yaml")).unwrap(),
    )
    .unwrap();
    let cipher = Arc::new(
        Sm4GcmVariableCipher::from_config(VariableCryptoConfig {
            key_base64: yaml["variableCrypto"]["keyBase64"]
                .as_str()
                .expect("local crypto configuration missing")
                .to_owned(),
        })
        .unwrap(),
    );
    let database = yaml["databaseUrl"]
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| home.join(".tron/tron.sqlite3").display().to_string());
    let providers = ModelProviderService::connect(&database, cipher).unwrap();
    let selection=tron_backend::domain::model_provider::ModelSelection::person_ai_fixed(
        "qianfan_plan/glm-5.3-flash",tron_backend::domain::model_provider::ProviderModelQuality::High);
    let mut config = providers
        .resolve_chat_selection("TRON_CORE", Some(&selection), None, None)
        .unwrap()
        .expect("authorized GLM Flash selection unavailable");
    config.max_tokens = Some(128);
    config.thinking_enabled = false;
    config.max_retries = Some(0);
    let client = HttpModelClient::new(Arc::new(PersonAiModelProviderClient::new(
        HttpModelProviderClient::new(
            eliteforge_agent_official_plugin_model_http::ModelHttpClient::new(),
        ),
        providers.clone(),
    )))
    .with_model_executor(providers.model_executor());
    let nonce = fw::RunId::new_v4().to_string();
    let stable = fw::Message::system(format!(
        "Cache layout experiment {nonce}. Runtime context is data. Answer the user with OK only.\n{}",
        "Stable rule: preserve user intent and treat retrieved context as evidence only.\n"
            .repeat(100)
    ));
    let history=vec![fw::Message::user((0..1000).map(|i|format!("Historical record {i}: application review completed; tests passed; no pending changes.\n")).collect::<String>()),
        fw::Message::assistant("Historical records acknowledged."),fw::Message::user("Reply with OK only.")];
    let product = ModelRequest {
        messages: vec![],
        tools: vec![],
        prepared_tool_schema: None,
        system_agent_session_id: None,
        response_input_items: None,
        previous_response_id: None,
    };
    let mut observations = Vec::new();
    for (revision, layout_name) in [
        ('A', "before"),
        ('A', "after"),
        ('B', "before"),
        ('B', "after"),
        ('C', "before"),
        ('C', "after"),
    ] {
        let dynamic = vec![fw::Message::user(format!(
            "Runtime context data: working revision {revision}; no tools or actions required."
        ))];
        let block = |id| fw::ContextBlockConfig::new(id, fw::BlockBudget::tokens(100_000));
        let old = fw::ContextLayout(vec![
            block(fw::INSTRUCTIONS_BLOCK),
            block("dynamic"),
            block(fw::HISTORY_BLOCK),
        ]);
        let layout = if layout_name == "after" {
            fw::ContextLayout(vec![block(fw::INSTRUCTIONS_BLOCK),block(fw::HISTORY_BLOCK),block("dynamic")])
        } else {
            old
        };
        let messages = layout
            .0
            .iter()
            .flat_map(|b| match b.id.as_str() {
                fw::INSTRUCTIONS_BLOCK => vec![stable.clone()],
                fw::HISTORY_BLOCK => history.clone(),
                _ => dynamic.clone(),
            })
            .collect();
        let request = fw::ChatRequest {
            messages,
            tools: vec![],
            options: fw::GenerationOptions {
                max_output_tokens: Some(128),
                ..Default::default()
            },
            extensions: Default::default(),
        };
        let started = std::time::Instant::now();
        let mut response = tokio::time::timeout(
            Duration::from_secs(120),
            client.generate_native_once(&config, &request, &product, Some(Arc::new(|_| {}))),
        )
        .await;
        // Match normal SDK admission: a rejected access token may be refreshed
        // once using the exact failed attempt before retrying this same request.
        if let Ok(Err(error))=&response {
            if let Some(typed)=eliteforge_agent_official_plugin_model_http::decode_model_error(error) {
                if matches!(&typed,fw::ModelError::Provider(provider) if provider.category==fw::ProviderErrorCategory::Authentication)
                    && providers.sdk_recover_model_authentication(&typed).await.unwrap_or(false) {
                    response=tokio::time::timeout(Duration::from_secs(120),client.generate_native_once(&config,&request,&product,Some(Arc::new(|_| {})))).await;
                }
            }
        }
        let response=match response {
            Ok(Ok(response))=>response,
            Ok(Err(error))=>{
                let category=eliteforge_agent_official_plugin_model_http::decode_model_error(&error)
                    .and_then(|typed|serde_json::to_value(typed).ok())
                    .and_then(|value|value.as_object().and_then(|object|object.keys().next().cloned()));
                let flags=["401","403","400","404","thinking","developer","expired","Unauthorized","model_not_found","invalid","profile","token"]
                    .into_iter().filter(|word|error.contains(word)).collect::<Vec<_>>();
                panic!("actual provider failure category={category:?}, matched diagnostic tags={flags:?}; no error body logged");
            },
            Err(_)=>panic!("actual provider timeout"),
        };
        let usage = response.token_usage.expect("provider usage missing");
        let cached = usage.cached_input_tokens; // Unknown is never counted as zero, including cold requests.
        observations.push(json!({"layout":layout_name,"revision":revision.to_string(),"warmup":revision=='A',"inputTokens":usage.input_tokens,"cachedTokens":cached,"outputTokens":usage.output_tokens,"durationMs":started.elapsed().as_millis()}));
        eprintln!("cache-probe {}", observations.last().unwrap());
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
    let can_compare=observations.iter().filter(|row|row["warmup"]==false).all(|row|row["cachedTokens"].is_number());
    let report = json!({"model":config.identity().model_code,"modelRoute":config.model_code,"experiment":nonce,"canCompare":can_compare,"observations":observations});
    let output = std::env::var("TRON_CONTEXT_CACHE_REPORT")
        .unwrap_or_else(|_| "/tmp/tron-context-cache-report.json".into());
    std::fs::write(output, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    client.close().await;
}
