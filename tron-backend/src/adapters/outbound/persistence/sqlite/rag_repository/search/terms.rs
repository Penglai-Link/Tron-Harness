pub(super) fn search_terms(query: &str) -> Vec<String> {
    query_terms(query)
        .into_iter()
        .map(|term| term.to_ascii_lowercase())
        .collect()
}

pub(super) fn fts_query(query: &str) -> String {
    query_terms(query)
        .into_iter()
        .map(|term| {
            let escaped = term.replace('"', "\"\"");
            format!("\"{escaped}\"")
        })
        .collect::<Vec<_>>()
        .join(" OR ")
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum QueryTermClass {
    Ascii,
    Unicode,
}

fn query_terms(query: &str) -> Vec<String> {
    let mut terms = Vec::new();
    let mut current = String::new();
    let mut current_class = None;
    for character in query.chars() {
        let class = if character.is_ascii_alphanumeric() || character == '_' {
            Some(QueryTermClass::Ascii)
        } else if character.is_alphanumeric() {
            Some(QueryTermClass::Unicode)
        } else {
            None
        };
        let Some(class) = class else {
            push_query_term(&mut terms, &mut current);
            current_class = None;
            continue;
        };
        if current_class.is_some_and(|existing| existing != class) {
            push_query_term(&mut terms, &mut current);
        }
        current.push(character);
        current_class = Some(class);
    }
    push_query_term(&mut terms, &mut current);
    terms
}

fn push_query_term(terms: &mut Vec<String>, current: &mut String) {
    let term = current.trim();
    if !term.is_empty() {
        terms.push(term.to_string());
    }
    current.clear();
}
