pub(crate) fn redact_sensitive_text(value: &str) -> String {
    value
        .lines()
        .map(redact_sensitive_line)
        .collect::<Vec<_>>()
        .join("\n")
}

fn redact_sensitive_line(line: &str) -> String {
    let lowered = line.to_ascii_lowercase();
    // URLs in provider diagnostics can carry basic credentials without a named key.
    if line.split_whitespace().any(|word| {
        word.split_once("://").is_some_and(|(_, rest)| {
            rest.split(['/', '?', '#'])
                .next()
                .is_some_and(|authority| authority.contains('@'))
        })
    }) {
        return "[REDACTED SENSITIVE LINE]".into();
    }
    const SENSITIVE_KEYS: &[&str] = &[
        "api_key",
        "apikey",
        "api key",
        "password",
        "passwd",
        "authorization",
        "access_token",
        "refresh_token",
        "private_key",
        "client_secret",
        "secret",
    ];
    if let Some(separator) = line.find([':', '=']) {
        if SENSITIVE_KEYS
            .iter()
            .any(|key| lowered[..separator].contains(key))
        {
            return format!("{}: [REDACTED]", line[..separator].trim_end());
        }
    }
    if SENSITIVE_KEYS
        .iter()
        .any(|key| contains_sensitive_assignment(&lowered, key))
        || lowered.contains("?token=")
        || lowered.contains("&token=")
    {
        return "[REDACTED SENSITIVE LINE]".into();
    }
    let normalized_start = lowered.trim_start_matches(|character: char| {
        character.is_whitespace() || matches!(character, '"' | '\'' | '`' | '-' | '*' | '#')
    });
    if SENSITIVE_KEYS
        .iter()
        .any(|key| normalized_start.starts_with(key))
    {
        return "[REDACTED SENSITIVE LINE]".into();
    }
    let mut redact_next = false;
    line.split_whitespace()
        .map(|word| {
            let normalized = word.trim_matches(|character: char| {
                matches!(
                    character,
                    '"' | '\'' | '`' | ',' | ';' | '(' | ')' | '[' | ']'
                )
            });
            let lowered = normalized.to_ascii_lowercase();
            if redact_next || lowered.starts_with("sk-") || lowered.starts_with("bearer-") {
                redact_next = false;
                "[REDACTED]"
            } else if lowered == "bearer" {
                redact_next = true;
                word
            } else {
                word
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn contains_sensitive_assignment(line: &str, key: &str) -> bool {
    line.match_indices(key).any(|(index, _)| {
        line[index + key.len()..]
            .trim_start_matches(|character: char| {
                character.is_whitespace() || matches!(character, '"' | '\'' | '`')
            })
            .chars()
            .next()
            .is_some_and(|character| matches!(character, ':' | '='))
    })
}
