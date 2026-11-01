//! Opt-in diagnosis of the selected local PersonAI authorization. Credentials
//! stay in memory; this probe never refreshes tokens or changes local storage.
use crab_serverapi::auth::LoginClient;
use rusqlite::{Connection, OpenFlags};
use serde_json::Value;
use std::{path::PathBuf, time::Duration};
use tron_backend::{
    adapters::outbound::crypto::Sm4GcmVariableCipher,
    application::{model_provider::ModelProviderService, variable::VariableCipher},
    config::VariableCryptoConfig,
};

#[tokio::test]
#[ignore = "requires TRON_PERSONAI_AUTH_REPAIR=1; uses the selected local account for one tiny model response"]
async fn selected_personai_model_recovers_a_rotated_encryption_key() {
    use eliteforge_agent_core::{ModelError, ProviderErrorCategory};
    use std::sync::Arc;
    use tron_backend::domain::agent::AgentModelConfig;
    assert_eq!(
        std::env::var("TRON_PERSONAI_AUTH_REPAIR").as_deref(),
        Ok("1")
    );
    let directory = PathBuf::from(std::env::var("TRON_PERSONAI_AUTH_DIRECTORY").unwrap());
    let yaml: serde_yaml::Value =
        serde_yaml::from_str(&std::fs::read_to_string(directory.join("application.yaml")).unwrap())
            .unwrap();
    let cipher = Arc::new(
        Sm4GcmVariableCipher::from_config(VariableCryptoConfig {
            key_base64: yaml["variableCrypto"]["keyBase64"].as_str().unwrap().into(),
        })
        .unwrap(),
    );
    let service =
        ModelProviderService::connect(directory.join("tron.sqlite3").to_str().unwrap(), cipher)
            .unwrap();
    let model = std::env::var("TRON_PERSONAI_AUTH_MODEL").unwrap();
    let config: AgentModelConfig = serde_json::from_value(serde_json::json!({
        "modelCode":format!("PERSON_AI:FIXED:High:{model}"),
        "modelName":model,"protocol":"CHAT_COMPLETIONS","apiKey":"",
        "baseUrl":"https://person-ai.invalid","apiPath":"chat/completions",
    }))
    .unwrap();
    let request = serde_json::json!({
        "model":model,"messages":[{"role":"user","content":"Reply with OK only."}],
        "max_tokens":128,"thinking":{"type":"disabled"},
    });
    let first = tokio::time::timeout(Duration::from_secs(45), async {
        service
            .sdk_model_transport(&config)
            .await?
            .json("chat/completions", &request)
            .await
    })
    .await
    .unwrap();
    let mut recovered = false;
    let response = match first {
        Ok(response) => response,
        Err(error @ ModelError::Provider(_))
            if matches!(&error,
            ModelError::Provider(detail) if detail.category == ProviderErrorCategory::Authentication
                && detail.error_type.as_deref() == Some("decrypt_error")) =>
        {
            assert!(
                service
                    .sdk_recover_model_authentication(&error)
                    .await
                    .unwrap()
            );
            recovered = true;
            tokio::time::timeout(Duration::from_secs(45), async {
                service
                    .sdk_model_transport(&config)
                    .await?
                    .json("chat/completions", &request)
                    .await
            })
            .await
            .unwrap()
            .unwrap()
        }
        Err(error) => panic!("PersonAI probe failed: {error}"),
    };
    assert!(
        response
            .pointer("/choices/0/message/content")
            .and_then(Value::as_str)
            .is_some_and(|text| !text.trim().is_empty())
    );
    println!("PERSONAI_MODEL_RECEIPT keyRecovered={recovered} contentPresent=true model={model}");
}

#[tokio::test]
#[ignore = "requires TRON_PERSONAI_AUTH_VALIDATE=1 and an existing local authorization"]
async fn saved_personai_encryption_key_matches_the_authorized_device() {
    assert_eq!(
        std::env::var("TRON_PERSONAI_AUTH_VALIDATE").as_deref(),
        Ok("1")
    );
    let directory = PathBuf::from(std::env::var("TRON_PERSONAI_AUTH_DIRECTORY").unwrap());
    let source = Connection::open_with_flags(
        directory.join("tron.sqlite3"),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let yaml: serde_yaml::Value =
        serde_yaml::from_str(&std::fs::read_to_string(directory.join("application.yaml")).unwrap())
            .unwrap();
    let cipher = Sm4GcmVariableCipher::from_config(VariableCryptoConfig {
        key_base64: yaml["variableCrypto"]["keyBase64"].as_str().unwrap().into(),
    })
    .unwrap();
    let encrypted: String = source.query_row(
        "SELECT credentials_encrypted FROM person_ai_profile_credentials ORDER BY updated_at DESC LIMIT 1",
        [], |row| row.get(0),
    ).unwrap();
    let envelope = serde_json::from_str(encrypted.strip_prefix("enc:v1:").unwrap()).unwrap();
    let plain = cipher
        .decrypt(&envelope, b"model_provider.credentials")
        .unwrap();
    let saved: Value = serde_json::from_str(&plain).unwrap();
    let session_id: String = source.query_row(
        "SELECT session_id FROM provider_authorization_sessions WHERE status='Completed' ORDER BY created_at DESC LIMIT 1",
        [], |row| row.get(0),
    ).unwrap();
    let client = LoginClient::new(saved["server_base_url"].as_str().unwrap()).unwrap();
    let session = tokio::time::timeout(Duration::from_secs(30), client.get_session(&session_id))
        .await
        .unwrap()
        .expect("authorization session lookup failed");
    let same_device = session.client_id.as_deref() == saved["client_id"].as_str();
    let same_public_key =
        session.public_key.as_deref().map(str::trim) == saved["public_key"].as_str().map(str::trim);
    println!("PERSONAI_AUTH_RECEIPT sameDevice={same_device} samePublicKey={same_public_key}");
    assert!(same_device, "stored session belongs to another device");
    assert!(
        same_public_key,
        "saved encryption key is stale for this authorized device"
    );
}
