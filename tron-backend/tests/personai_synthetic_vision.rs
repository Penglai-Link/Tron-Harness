//! Opt-in synthetic-only quality probe. No host credentials leave the existing SDK destination.
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use std::{path::Path, sync::Arc, time::Duration};
use tron_backend::{
    adapters::outbound::crypto::Sm4GcmVariableCipher,
    application::{model_provider::ModelProviderService, variable::VariableCipher},
    config::VariableCryptoConfig,
};
#[tokio::test]
#[ignore = "requires explicit TRON_MINERU_SYNTHETIC_VISION=1"]
async fn authorized_sdk_synthetic_text_table_formula_image_quality() {
    assert_eq!(
        std::env::var("TRON_MINERU_SYNTHETIC_VISION").as_deref(),
        Ok("1")
    );
    let evidence = Path::new(
        "/Users/clouds3n/Documents/Codex/2026-10-03/task/knowledge-base-implementation-evidence",
    );
    let source = rusqlite::Connection::open_with_flags(
        "/Users/clouds3n/.tron/tron.sqlite3",
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let isolated = tempfile::tempdir().unwrap();
    let path = isolated.path().join("vision.sqlite3");
    assert!(std::process::Command::new("python3").arg("-c").arg("import sqlite3,sys,os; a=sqlite3.connect('file:/Users/clouds3n/.tron/tron.sqlite3?mode=ro',uri=True); b=sqlite3.connect(sys.argv[1]); a.backup(b); b.close(); a.close(); os.chmod(sys.argv[1],0o600)").arg(&path).status().unwrap().success());
    let yaml: serde_yaml::Value = serde_yaml::from_str(
        &std::fs::read_to_string("/Users/clouds3n/.tron/application.yaml").unwrap(),
    )
    .unwrap();
    let cipher = Arc::new(
        Sm4GcmVariableCipher::from_config(VariableCryptoConfig {
            key_base64: yaml["variableCrypto"]["keyBase64"].as_str().unwrap().into(),
        })
        .unwrap(),
    );
    let encrypted: String = source
        .query_row(
            "SELECT credentials_encrypted FROM provider_connections WHERE provider_code='PersonAi'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let encrypted = serde_json::from_str(encrypted.strip_prefix("enc:v1:").unwrap()).unwrap();
    let plain = cipher
        .decrypt(&encrypted, b"model_provider.credentials")
        .unwrap();
    let credentials: Value = serde_json::from_str(&plain).unwrap();
    let context = crab_serverapi::AuthorizedClientContext::default()
        .with_token(credentials["token"].as_str().unwrap())
        .with_refresh_token(credentials["refresh_token"].as_str().unwrap())
        .with_client_id(credentials["client_id"].as_str().unwrap())
        .with_public_key(credentials["public_key"].as_str().unwrap())
        .with_product_code("tron")
        .with_product_version(env!("CARGO_PKG_VERSION"));
    let client = crab_serverapi::gateway::ChatCompletionsClient::new(
        credentials["server_base_url"].as_str().unwrap(),
    )
    .unwrap();
    let mut result = json!({"syntheticOnly":true,"catalogRequests":1,"modelRequests":0,"scope":"host existing authorization / temporary isolated DB","directMineruProtocolCompatible":false});
    let models = match tokio::time::timeout(
        Duration::from_secs(45),
        client.list_models(
            &context,
            Some(crab_serverapi::gateway::QualityType::High),
            Some(true),
            None,
        ),
    )
    .await
    {
        Ok(Ok(models)) => models,
        _ => {
            result["status"] = json!("vision_catalog_unavailable");
            std::fs::write(
                evidence.join("personai-synthetic-vision-results.json"),
                serde_json::to_vec_pretty(&result).unwrap(),
            )
            .unwrap();
            return;
        }
    };
    result["visionCatalog"] = json!(
        models
            .iter()
            .map(|m| json!({"modelCode":m.model_code,"imageUnderstanding":m.image_understanding}))
            .collect::<Vec<_>>()
    );
    let Some(model) = models
        .iter()
        .find(|m| m.image_understanding == Some(true) && m.model_code == "qwen3.8-flash")
    else {
        result["status"] = json!("no_authorized_high_vision_model");
        std::fs::write(
            evidence.join("personai-synthetic-vision-results.json"),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
        return;
    };
    let providers = ModelProviderService::connect(path.to_str().unwrap(), cipher).unwrap();
    let scope = providers.sdk_model_authorization_scope().unwrap();
    let config=serde_json::from_value(json!({"modelCode":format!("PERSON_AI:FIXED:High:{}",model.model_code),"protocol":"CHAT_COMPLETIONS","baseUrl":"https://person-ai.invalid","apiKey":"","modelName":model.model_code,"maxTokens":2048,"maxRetries":0})).unwrap();
    let transport = providers.sdk_model_transport(&config).await.unwrap();
    let image=std::fs::read("/Users/clouds3n/Documents/Codex/2026-10-03/task/mineru-local-deployment/data/synthetic-probe/vision-composite.png").unwrap();
    let body = json!({"model":model.model_code,"stream":false,"temperature":0,"max_tokens":4096,"enable_thinking":false,"messages":[{"role":"user","content":[{"type":"text","text":"This is synthetic test material. Read the attached image. Return ONLY a JSON object with: text (exact visible report heading), tableHtml (HTML table preserving row/column relationships), formulaLatex (the visible formula as LaTeX), imageAnalysis (object with class, subclass, caption, content describing the diagram's boxes and arrow direction). Do not invent content. Keep literal text exact."},{"type":"image_url","image_url":{"url":format!("data:image/png;base64,{}",STANDARD.encode(image))}}]}]});
    result["modelRequests"] = json!(1);
    result["requestedModel"] = json!(model.model_code);
    match tokio::time::timeout(
        Duration::from_secs(60),
        transport.json("chat/completions", &body),
    )
    .await
    {
        Ok(Ok(response)) => {
            result["reportedModel"] = response["model"].clone();
            result["finishReason"] = response["choices"][0]["finish_reason"].clone();
            let text = response["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("");
            result["responseText"] = json!(text);
            let clean = text
                .trim()
                .trim_start_matches("```json")
                .trim_start_matches("```")
                .trim_end_matches("```")
                .trim();
            if let Ok(parsed) = serde_json::from_str::<Value>(clean) {
                let table = parsed["tableHtml"].as_str().unwrap_or("");
                let formula = parsed["formulaLatex"]
                    .as_str()
                    .unwrap_or("")
                    .replace(' ', "");
                let image = parsed["imageAnalysis"].to_string().to_ascii_lowercase();
                result["contentChecks"] = json!({"heading":parsed["text"].as_str().is_some_and(|s|s.contains("ORBIT REPORT")),"table":table.contains("<table")&&table.contains("Alpha")&&table.contains("120")&&table.contains("Beta")&&table.contains("240"),"formula":formula.contains("E=mc^2")||formula.contains("E=mc^{2}"),"imageDirection":image.contains("upload")&&image.contains("index")&&(image.contains("left")||image.contains("right"))});
                result["status"] = json!("returned_structured_quality_probe");
            } else {
                result["status"] = json!("response_not_json");
            }
            result["rawMineruTagsPresent"] = json!(
                text.contains("<class>")
                    && text.contains("<caption>")
                    && text.contains("<content>")
            );
        }
        Ok(Err(_)) => {
            result["status"] = json!("sdk_model_request_rejected");
        }
        Err(_) => {
            result["status"] = json!("sdk_model_timeout");
        }
    }
    result["authorizationScopeStable"] =
        json!(providers.sdk_model_authorization_scope().unwrap() == scope);
    std::fs::write(
        evidence.join("personai-synthetic-vision-results.json"),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
    println!(
        "Synthetic visual probe finished; credential-free content/quality evidence written. Generic JSON is not MinerU's native task protocol."
    );
}
