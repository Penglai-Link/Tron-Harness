use std::{
    fs,
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use tron_backend::adapters::outbound::config::YamlSystemConfigStore;
use tron_backend::application::system_config::{
    SystemConfigChange, SystemConfigError, SystemConfigService,
};

#[test]
fn exposes_structured_items_and_masks_sensitive_values() {
    let fixture = ConfigFixture::new("structured");
    let document = fixture.service.get().expect("configuration should load");

    assert!(
        document
            .items
            .iter()
            .any(|item| item.path == "port" && item.value_type == "number")
    );
    let secret = document
        .items
        .iter()
        .find(|item| item.path == "variableCrypto.keyBase64")
        .expect("secret item should exist");
    assert!(secret.sensitive);
    assert!(secret.value.is_none());
    assert!(
        !document
            .items
            .iter()
            .any(|item| item.path.starts_with("auth."))
    );
    assert!(
        !fs::read_to_string(&fixture.path)
            .expect("yaml should be readable")
            .contains("auth:")
    );
}

#[test]
fn updates_yaml_with_revision_control_and_reports_restart_effect() {
    let fixture = ConfigFixture::new("update");
    let document = fixture.service.get().expect("configuration should load");
    let result = fixture
        .service
        .update(
            &document.revision,
            vec![SystemConfigChange {
                path: "port".into(),
                value: 8181.into(),
            }],
            false,
        )
        .expect("configuration should update");

    assert_eq!(result.changed_paths, vec!["port"]);
    assert_eq!(result.restart_paths, vec!["port"]);
    assert!(
        fs::read_to_string(&fixture.path)
            .expect("yaml should be readable")
            .contains("port: 8181")
    );

    let conflict = fixture.service.update(
        &document.revision,
        vec![SystemConfigChange {
            path: "port".into(),
            value: 8282.into(),
        }],
        false,
    );
    assert!(matches!(conflict, Err(SystemConfigError::RevisionConflict)));
}

#[test]
fn migration_fields_require_explicit_confirmation() {
    let fixture = ConfigFixture::new("migration");
    let document = fixture.service.get().expect("configuration should load");
    let change = || SystemConfigChange {
        path: "databaseUrl".into(),
        value: "next.sqlite3".into(),
    };

    let rejected = fixture
        .service
        .update(&document.revision, vec![change()], false);
    assert!(
        matches!(rejected, Err(SystemConfigError::MigrationConfirmationRequired(paths)) if paths == vec!["databaseUrl"])
    );

    let result = fixture
        .service
        .update(&document.revision, vec![change()], true)
        .expect("confirmed migration change should persist");
    assert_eq!(result.migration_paths, vec!["databaseUrl"]);
    assert!(result.restart_paths.is_empty());
}

struct ConfigFixture {
    path: PathBuf,
    service: SystemConfigService,
}

impl ConfigFixture {
    fn new(name: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be valid")
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "tron-system-config-{name}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).expect("fixture directory should be created");
        let path = directory.join("application.yaml");
        fs::write(&path, "bindAddress: 127.0.0.1\nport: 8080\ndatabaseUrl: tron.sqlite3\nvariableCrypto:\n  keyBase64: dGVzdC1zZWNyZXQ=\nauth:\n  jwtSecret: test-secret\n").expect("fixture should be written");
        Self {
            service: SystemConfigService::new(Arc::new(YamlSystemConfigStore::new(path.clone()))),
            path,
        }
    }
}

impl Drop for ConfigFixture {
    fn drop(&mut self) {
        if let Some(directory) = self.path.parent() {
            let _ = fs::remove_dir_all(directory);
        }
    }
}
