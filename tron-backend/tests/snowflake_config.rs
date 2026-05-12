use std::{
    fs,
    path::{Path, PathBuf},
};

fn backend_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn fixture_path(file_name: &str) -> PathBuf {
    backend_root()
        .join("tests")
        .join("fixtures")
        .join("snowflake")
        .join(file_name)
}

fn read_fixture(file_name: &str) -> String {
    fs::read_to_string(fixture_path(file_name))
        .unwrap_or_else(|error| panic!("failed to read snowflake fixture `{file_name}`: {error}"))
}

fn assert_snowflake_config_loader_is_implemented(filter_name: &str) {
    let config_module = backend_root().join("src").join("shared").join("id").join("config.rs");

    assert!(
        config_module.exists(),
        "{filter_name}: expected `tron-backend/src/shared/id/config.rs` to implement Snowflake YAML loading, but it does not exist yet (RED expected)",
    );
}

#[test]
fn snowflake_config_valid_loads_generator_config_from_yaml_fixture() {
    let fixture = read_fixture("valid.yaml");
    assert!(
        fixture.contains("instance_id: 1") && fixture.contains("epoch_ms: 1767225600000"),
        "snowflake_config_valid: fixture must cover accepted generator keys and values",
    );

    assert_snowflake_config_loader_is_implemented("snowflake_config_valid");
}

#[test]
fn snowflake_config_missing_instance_id_rejects_incomplete_yaml() {
    let fixture = read_fixture("missing-instance-id.yaml");
    assert!(
        !fixture.contains("instance_id:") && fixture.contains("epoch_ms:"),
        "snowflake_config_missing_instance_id: fixture must omit `instance_id` while keeping `epoch_ms`",
    );

    assert_snowflake_config_loader_is_implemented("snowflake_config_missing_instance_id");
}

#[test]
fn snowflake_config_invalid_instance_id_rejects_out_of_range_value() {
    let fixture = read_fixture("invalid-instance-id.yaml");
    assert!(
        fixture.contains("instance_id: -1"),
        "snowflake_config_invalid_instance_id: fixture must provide an invalid `instance_id`",
    );

    assert_snowflake_config_loader_is_implemented("snowflake_config_invalid_instance_id");
}

#[test]
fn snowflake_config_invalid_epoch_rejects_future_epoch_value() {
    let fixture = read_fixture("invalid-epoch.yaml");
    assert!(
        fixture.contains("epoch_ms: 4102444800000"),
        "snowflake_config_invalid_epoch: fixture must provide an invalid future `epoch_ms`",
    );

    assert_snowflake_config_loader_is_implemented("snowflake_config_invalid_epoch");
}
