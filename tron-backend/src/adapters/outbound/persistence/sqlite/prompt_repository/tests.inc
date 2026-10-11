#[cfg(test)]
mod tests {
    use super::SqlitePromptRepository;

    #[test]
    fn action_category_is_supported_by_the_prompt_schema() {
        let repository =
            SqlitePromptRepository::connect(":memory:").expect("prompt schema should migrate");
        repository
            .connection
            .runtime()
            .call_blocking(|connection| {
                connection.execute(
                    "INSERT INTO prompts (
                    id, prompt_code, display_name, category,
                    prompt_body, description, enabled, version, created_at, updated_at, built_in,
                    runtime_required, catalog_key
                ) VALUES (1, 'PROMPT-ACTION', 'Action prompt', 'ACTION',
                    'Run {{command}}', NULL, 1, 1, 1, 1, 0, 0, NULL)",
                    [],
                )
            })
            .expect("action prompt category should satisfy the schema");

        let category = repository
            .connection
            .runtime()
            .call_blocking(|connection| {
                connection.query_row(
                    "SELECT category FROM prompts WHERE prompt_code = 'PROMPT-ACTION'",
                    [],
                    |row| row.get::<_, String>(0),
                )
            })
            .expect("action prompt should be readable");
        assert_eq!(category, "ACTION");
    }
}
