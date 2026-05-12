use std::path::Path;

#[test]
fn common_snowflake_import_boundary_limits_snowflaked_usage_to_adapter_file() {
    let adapter_file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("shared")
        .join("id")
        .join("snowflake.rs");

    assert!(
        adapter_file.exists(),
        "common_snowflake_import_boundary: expected dedicated adapter file `tron-backend/src/shared/id/snowflake.rs` to isolate `snowflaked` imports, but file is missing (RED expected)",
    );
}
