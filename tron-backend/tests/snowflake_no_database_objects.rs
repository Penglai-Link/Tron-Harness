use std::{fs, path::Path};

#[test]
fn snowflake_no_database_objects_enforces_shared_persistence_policy() {
    let migrations_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
    if migrations_dir.exists() {
        let mut migration_hits = Vec::new();
        if let Ok(entries) = fs::read_dir(&migrations_dir) {
            for entry in entries.flatten() {
                let file_name = entry.file_name();
                if file_name
                    .to_string_lossy()
                    .to_ascii_lowercase()
                    .contains("snowflake")
                {
                    migration_hits.push(file_name.to_string_lossy().into_owned());
                }
            }
        }

        assert!(
            migration_hits.is_empty(),
            "snowflake_no_database_objects: shared capability must not own DB migrations, found: {migration_hits:?}",
        );
    }

    let policy_module = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("shared")
        .join("id")
        .join("persistence_policy.rs");
    assert!(
        policy_module.exists(),
        "snowflake_no_database_objects: expected a shared persistence policy artifact at `tron-backend/src/shared/id/persistence_policy.rs`, but it is not implemented yet (RED expected)",
    );
}
