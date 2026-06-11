use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::params;

use super::*;
use crate::shared::time::current_timestamp;

fn temp_database_path(test_name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("tron-open-app-repo-{test_name}-{nanos}.sqlite3"))
}

fn fixture_open_app(now: &str) -> (OpenApp, OpenAppCredential) {
    let open_app = OpenApp {
        id: SnowflakeId::new(734_103_002_743_197_000),
        open_app_code: "OPENAPP-TEST-0001".into(),
        name: "Repo Fixture".into(),
        description: Some("repo test".into()),
        status: crate::domain::open_app::OpenAppStatus::Enabled,
        created_at: now.to_owned(),
        updated_at: now.to_owned(),
        last_rotated_at: Some(now.to_owned()),
    };
    let credential = OpenAppCredential {
        id: SnowflakeId::new(734_103_002_743_197_001),
        credential_code: "OPENAPP-CRED-TEST-0001".into(),
        open_app_code: open_app.open_app_code.clone(),
        status: OpenAppCredentialStatus::Active,
        secret_ciphertext: "ciphertext-one".into(),
        secret_fingerprint: "fingerprint-one".into(),
        created_at: now.to_owned(),
        revoked_at: None,
    };
    (open_app, credential)
}

#[test]
fn repository_persists_auth_state_and_rejects_replayed_nonces() {
    let database_path = temp_database_path("auth-state");
    let mut repository = SqliteOpenAppRepository::connect(
        database_path.to_str().expect("path should be valid UTF-8"),
    )
    .expect("repository should connect");
    let now = current_timestamp();
    let (open_app, credential) = fixture_open_app(&now);

    repository
        .create_open_app(&open_app, &credential)
        .expect("fixture open-app should insert");
    repository
        .insert_user_grant(&OpenAppUserGrant {
            id: SnowflakeId::new(734_103_002_743_197_002),
            grant_code: "OPENAPP-USER-GRANT-TEST-0001".into(),
            open_app_code: open_app.open_app_code.clone(),
            user_code: "USR-0001".into(),
            created_at: now.clone(),
        })
        .expect("fixture user grant should insert");
    repository
        .insert_capability_grant(&OpenAppCapabilityGrant {
            id: SnowflakeId::new(734_103_002_743_197_003),
            grant_code: "OPENAPP-CAP-GRANT-TEST-0001".into(),
            open_app_code: open_app.open_app_code.clone(),
            capability_code: "agent-chat".into(),
            operation_code: "session.read".into(),
            created_at: now.clone(),
        })
        .expect("fixture capability grant should insert");

    let auth_state = repository
        .load_auth_state(&open_app.open_app_code)
        .expect("auth state should load")
        .expect("fixture auth state should exist");
    assert_eq!(auth_state.open_app.open_app_code, open_app.open_app_code);
    assert_eq!(
        auth_state
            .active_credential
            .expect("active credential should exist")
            .credential_code,
        credential.credential_code
    );
    assert_eq!(auth_state.user_grants.len(), 1);
    assert_eq!(auth_state.capability_grants.len(), 1);

    let nonce = OpenApiNonceRecord {
        id: SnowflakeId::new(734_103_002_743_197_004),
        nonce_code: "OPENAPP-NONCE-TEST-0001".into(),
        open_app_code: open_app.open_app_code.clone(),
        nonce: "nonce-1".into(),
        signed_at: 1_700_000_000,
        expires_at: 1_700_000_300,
        created_at: now.clone(),
    };
    repository
        .insert_nonce(&nonce)
        .expect("first nonce insert should succeed");
    let replay = repository
        .insert_nonce(&nonce)
        .expect_err("duplicate nonce should fail");
    assert_eq!(replay, OpenAppError::NonceReplayed);

    let _ = fs::remove_file(database_path);
}

#[test]
fn rotate_credential_keeps_single_active_record_and_retains_history() {
    let database_path = temp_database_path("rotation");
    let mut repository = SqliteOpenAppRepository::connect(
        database_path.to_str().expect("path should be valid UTF-8"),
    )
    .expect("repository should connect");
    let now = current_timestamp();
    let (open_app, credential) = fixture_open_app(&now);
    repository
        .create_open_app(&open_app, &credential)
        .expect("fixture open-app should insert");

    let rotated_at = current_timestamp();
    let new_credential = OpenAppCredential {
        id: SnowflakeId::new(734_103_002_743_197_005),
        credential_code: "OPENAPP-CRED-TEST-0002".into(),
        open_app_code: open_app.open_app_code.clone(),
        status: OpenAppCredentialStatus::Active,
        secret_ciphertext: "ciphertext-two".into(),
        secret_fingerprint: "fingerprint-two".into(),
        created_at: rotated_at.clone(),
        revoked_at: None,
    };
    repository
        .rotate_credential(&open_app.open_app_code, &rotated_at, &new_credential)
        .expect("credential rotation should succeed");

    let auth_state = repository
        .load_auth_state(&open_app.open_app_code)
        .expect("auth state should load")
        .expect("fixture auth state should exist");
    assert_eq!(
        auth_state
            .active_credential
            .expect("active credential should exist")
            .credential_code,
        new_credential.credential_code
    );

    let (active_count, revoked_count): (i64, i64) = repository
        .connection
        .query_row(
            "SELECT
                SUM(CASE WHEN status = 'active' THEN 1 ELSE 0 END),
                SUM(CASE WHEN status = 'revoked' THEN 1 ELSE 0 END)
             FROM platform_open_app_credentials
             WHERE open_app_code = ?1",
            params![open_app.open_app_code],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("credential counts should load");
    assert_eq!(active_count, 1);
    assert_eq!(revoked_count, 1);

    let _ = fs::remove_file(database_path);
}
