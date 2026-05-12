use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

const USER_MANAGEMENT_MIGRATIONS: [&str; 3] = [
    "20260511_0001_create_user_management_users.sql",
    "20260511_0002_create_user_management_assignments.sql",
    "20260511_0003_create_user_password_reset_events.sql",
];

const EXPECTED_TABLES: [&str; 4] = [
    "users",
    "user_role_assignments",
    "user_space_assignments",
    "user_password_reset_events",
];

#[test]
fn user_management_schema_contracts_define_only_user_management_tables() {
    let schema = load_user_management_schema();
    let actual = create_table_names(&schema);
    let expected = string_set(EXPECTED_TABLES);

    assert_eq!(
        actual, expected,
        "DB-VERIFY-01: user-management migrations must define only the feature-owned user tables",
    );

    for forbidden in [
        "roles",
        "role_definitions",
        "spaces",
        "space_definitions",
        "permissions",
        "permission_definitions",
        "audits",
        "audit_logs",
        "audit_events",
        "invitations",
        "user_invitations",
    ] {
        assert!(
            !actual.contains(forbidden),
            "DB-VERIFY-01: `{forbidden}` must not be introduced as a definition table",
        );
    }
}

#[test]
fn user_management_schema_contracts_use_sqlite_integer_primary_keys_for_ids() {
    let schema = load_user_management_schema();

    for table in EXPECTED_TABLES {
        let block = table_block(&schema, table);
        assert!(
            block.contains("id INTEGER PRIMARY KEY CHECK (id > 0)"),
            "DB-VERIFY-01: `{table}.id` must use SQLite INTEGER PRIMARY KEY for 64-bit rowid-compatible IDs",
        );
    }
}

#[test]
fn user_management_schema_contracts_keep_user_code_stable_and_permissive() {
    let schema = load_user_management_schema();

    assert!(
        schema.contains("CREATE UNIQUE INDEX IF NOT EXISTS uk_users_user_code")
            && schema.contains("ON users (user_code);"),
        "DB-VERIFY-01: users.user_code must have a dedicated unique index",
    );

    for table in EXPECTED_TABLES {
        let block = table_block(&schema, table);
        assert!(
            block.contains("user_code = upper(user_code)")
                && block.contains("user_code NOT GLOB '*[^A-Z0-9_-]*'"),
            "DB-VERIFY-01: `{table}.user_code` must allow uppercase letters, digits, `_`, and `-` only",
        );
    }
}

#[test]
fn user_management_schema_contracts_constrain_fixed_role_codes() {
    let schema = load_user_management_schema();
    let role_assignments = table_block(&schema, "user_role_assignments");

    assert!(
        role_assignments
            .contains("role_code IN ('ADMIN', 'WORKSPACE_ADMIN', 'DEVELOPER', 'VIEWER')"),
        "DB-VERIFY-01: user role assignments must constrain role_code to the fixed role set",
    );
}

#[test]
fn user_management_schema_contracts_do_not_store_plaintext_temporary_passwords() {
    let schema = load_user_management_schema();
    let reset_events = table_block(&schema, "user_password_reset_events");
    let reset_columns = column_names(&reset_events);

    assert!(
        reset_columns.contains("temporary_password_returned"),
        "DB-VERIFY-01: reset events may record that a temporary password was returned once",
    );

    for forbidden in [
        "password",
        "password_hash",
        "temporary_password",
        "temporary_password_hash",
        "temporary_password_value",
        "plain_text_password",
        "plaintext_password",
        "raw_password",
        "generated_password",
    ] {
        assert!(
            !reset_columns.contains(forbidden),
            "DB-VERIFY-01: reset events must not include `{forbidden}` storage",
        );
    }

    for column in reset_columns {
        assert!(
            !column.contains("plaintext") && !column.contains("plain_text"),
            "DB-VERIFY-01: reset event column `{column}` suggests plaintext password storage",
        );
    }
}

fn load_user_management_schema() -> String {
    USER_MANAGEMENT_MIGRATIONS
        .into_iter()
        .map(read_migration)
        .collect::<Vec<_>>()
        .join("\n")
}

fn read_migration(file_name: &str) -> String {
    let path = migrations_dir().join(file_name);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read migration `{}`: {error}", path.display()))
}

fn migrations_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations")
}

fn create_table_names(schema: &str) -> BTreeSet<String> {
    schema
        .lines()
        .filter_map(|line| {
            line.trim()
                .strip_prefix("CREATE TABLE IF NOT EXISTS ")
                .and_then(|rest| rest.split_whitespace().next())
                .map(|name| name.trim_end_matches('(').to_string())
        })
        .collect()
}

fn table_block(schema: &str, table: &str) -> String {
    let marker = format!("CREATE TABLE IF NOT EXISTS {table} (");
    let start = schema
        .find(&marker)
        .unwrap_or_else(|| panic!("missing CREATE TABLE block for `{table}`"));
    let rest = &schema[start..];
    let end = rest
        .find("\n);")
        .unwrap_or_else(|| panic!("missing CREATE TABLE terminator for `{table}`"));

    rest[..end + "\n);".len()].to_string()
}

fn column_names(table_block: &str) -> BTreeSet<String> {
    table_block
        .lines()
        .skip(1)
        .map(str::trim)
        .take_while(|line| !line.starts_with("CONSTRAINT") && *line != ");")
        .filter(|line| !line.is_empty())
        .filter_map(|line| line.split_whitespace().next())
        .map(|name| name.trim_end_matches(',').to_string())
        .collect()
}

fn string_set<const N: usize>(items: [&str; N]) -> BTreeSet<String> {
    items.into_iter().map(str::to_string).collect()
}
