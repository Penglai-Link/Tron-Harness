use std::path::Path;

#[test]
fn fixed_snowflake_id_provider_returns_deterministic_fixture_ids() {
    let testing_module = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("shared")
        .join("id")
        .join("testing.rs");

    assert!(
        testing_module.exists(),
        "fixed_snowflake_id_provider: expected deterministic test provider at `tron-backend/src/shared/id/testing.rs`, but it is not implemented yet (RED expected)",
    );
}
