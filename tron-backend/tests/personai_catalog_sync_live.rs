//! Opt-in catalogue validation against the existing authorized gateway.
//! Only list requests are sent; migrations and sync writes use a temporary DB.
use std::{collections::BTreeSet, path::PathBuf, sync::Arc, time::Duration};

use crab_serverapi::{
    AuthorizedClientContext,
    gateway::{ChatCompletionsClient, QualityType},
};
use rusqlite::{Connection, OpenFlags};
use serde_json::Value;
use tron_backend::{
    adapters::outbound::crypto::Sm4GcmVariableCipher,
    application::{model_provider::ModelProviderService, variable::VariableCipher},
    config::VariableCryptoConfig,
    domain::model_provider::{
        ModelProviderCode, ProviderCatalogModelStatus, ProviderCatalogModelType,
    },
};

#[tokio::test]
#[ignore = "requires TRON_PERSONAI_CATALOG_VALIDATE=1 and an existing authorized profile"]
async fn authorized_personai_catalog_sync_matches_remote_vendor_tiers() {
    assert_eq!(
        std::env::var("TRON_PERSONAI_CATALOG_VALIDATE").as_deref(),
        Ok("1")
    );
    let directory = PathBuf::from(std::env::var("TRON_PERSONAI_CATALOG_DIRECTORY").unwrap());
    let source = Connection::open_with_flags(
        directory.join("tron.sqlite3"),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let isolated = tempfile::tempdir().unwrap();
    let database = isolated.path().join("catalog.sqlite3");
    source
        .execute("VACUUM INTO ?1", [database.to_str().unwrap()])
        .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&database, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    let yaml: serde_yaml::Value =
        serde_yaml::from_str(&std::fs::read_to_string(directory.join("application.yaml")).unwrap())
            .unwrap();
    let cipher = Arc::new(
        Sm4GcmVariableCipher::from_config(VariableCryptoConfig {
            key_base64: yaml["variableCrypto"]["keyBase64"].as_str().unwrap().into(),
        })
        .unwrap(),
    );
    let copy = Connection::open(&database).unwrap();
    let encrypted: String = copy
        .query_row(
            "SELECT credentials_encrypted FROM provider_connections WHERE provider_code='PersonAi'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let envelope = serde_json::from_str(encrypted.strip_prefix("enc:v1:").unwrap()).unwrap();
    let plain = cipher
        .decrypt(&envelope, b"model_provider.credentials")
        .unwrap();
    let mut credentials: Value = serde_json::from_str(&plain).unwrap();
    let context = AuthorizedClientContext::default()
        .with_token(credentials["token"].as_str().unwrap())
        .with_refresh_token(credentials["refresh_token"].as_str().unwrap())
        .with_client_id(credentials["client_id"].as_str().unwrap())
        .with_public_key(credentials["public_key"].as_str().unwrap())
        .with_product_code("tron")
        .with_product_version(env!("CARGO_PKG_VERSION"));
    let client =
        ChatCompletionsClient::new(credentials["server_base_url"].as_str().unwrap()).unwrap();
    let mut expected = BTreeSet::new();
    for (quality, name) in [
        (QualityType::High, "High"),
        (QualityType::Middle, "Middle"),
        (QualityType::Low, "Low"),
    ] {
        let models = tokio::time::timeout(
            Duration::from_secs(30),
            client.list_models(&context, Some(quality), None, None),
        )
        .await
        .unwrap()
        .unwrap();
        println!("remote {name}: {} entries", models.len());
        for model in models {
            expected.insert((
                model.model_code,
                model.provider_code.unwrap_or_default(),
                name.to_owned(),
            ));
        }
    }
    // Suppress token rotation in the isolated probe. The gateway still validates
    // the real access token; an expired/rejected token makes this test fail.
    credentials["authorization_expires_at"] =
        serde_json::json!(chrono::Utc::now().timestamp_millis() + 30 * 24 * 60 * 60 * 1000);
    let encrypted = cipher
        .encrypt(&credentials.to_string(), b"model_provider.credentials")
        .unwrap();
    copy.execute(
        "UPDATE provider_connections SET credentials_encrypted=?1 WHERE provider_code='PersonAi'",
        [format!(
            "enc:v1:{}",
            serde_json::to_string(&encrypted).unwrap()
        )],
    )
    .unwrap();
    drop(copy);
    let service = ModelProviderService::connect(database.to_str().unwrap(), cipher).unwrap();
    let run = tokio::time::timeout(
        Duration::from_secs(60),
        service.sync_now(ModelProviderCode::PersonAi),
    )
    .await
    .unwrap()
    .unwrap();
    let catalog = service
        .list_catalog_models(ModelProviderCode::PersonAi)
        .unwrap();
    let actual = catalog
        .iter()
        .filter(|model| {
            model.catalog_model_type == ProviderCatalogModelType::Chat
                && model.status == ProviderCatalogModelStatus::Available
        })
        .map(|model| {
            (
                model.model_code.clone(),
                model.upstream_vendor_code.clone().unwrap_or_default(),
                model.quality_type.unwrap().storage_value().to_owned(),
            )
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(actual, expected);
    let high = actual
        .iter()
        .filter(|(_, _, quality)| quality == "High")
        .count();
    println!(
        "synced High: {high}; available chat: {}; full sync: {}",
        actual.len(),
        run.model_count.unwrap()
    );
    for option in service
        .list_text_select_options("")
        .unwrap()
        .iter()
        .filter(|option| option.enabled && option.selection.model_code.is_some())
    {
        let config = service
            .resolve_chat_selection("", Some(&option.selection), None, None)
            .unwrap()
            .unwrap();
        assert_eq!(
            Some(&config.model_name),
            option.selection.model_code.as_ref()
        );
    }
}
