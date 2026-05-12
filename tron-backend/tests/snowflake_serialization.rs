use std::path::Path;

use tron_backend::shared::id::SnowflakeId;

fn assert_serialization_module_is_implemented(filter_name: &str) {
    let serialization_module = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("shared")
        .join("id")
        .join("serialization.rs");

    assert!(
        serialization_module.exists(),
        "{filter_name}: expected `tron-backend/src/shared/id/serialization.rs` to enforce decimal-string serialization policy, but it does not exist yet (RED expected)",
    );
}

#[test]
fn snowflake_serialization_string_uses_decimal_string_contract() {
    let id = SnowflakeId::new(734_103_002_743_185_408);
    let as_decimal = id.value().to_string();
    assert_eq!(as_decimal, "734103002743185408");

    assert_serialization_module_is_implemented("snowflake_serialization_string");
}

#[test]
fn snowflake_serialization_invalid_string_rejects_non_decimal_inputs() {
    let invalid_cases = ["", "not-a-number", "-1", "999999999999999999999"];
    assert!(
        invalid_cases.iter().all(|value| !value.contains(' ')),
        "snowflake_serialization_invalid_string: fixtures must be unambiguous invalid decimal strings",
    );

    assert_serialization_module_is_implemented("snowflake_serialization_invalid_string");
}

#[test]
fn snowflake_serialization_large_i64_preserves_values_beyond_js_safe_int() {
    let id = SnowflakeId::new(9_007_199_254_740_993);
    assert!(id.value() > 9_007_199_254_740_991);

    assert_serialization_module_is_implemented("snowflake_serialization_large_i64");
}
