use std::collections::{BTreeSet, HashMap};

use super::TemplateError;

pub fn analyze_template(template: &str) -> Result<Vec<String>, TemplateError> {
    let mut names = Vec::new();
    walk_template(template, |segment| {
        if !names.iter().any(|name| name == segment) {
            names.push(segment.to_string());
        }
        Ok(String::new())
    })?;
    Ok(names)
}

pub fn render_template(
    template: &str,
    values: &HashMap<String, String>,
) -> Result<String, TemplateError> {
    let names = analyze_template(template)?;
    let expected = names.iter().cloned().collect::<BTreeSet<_>>();
    let provided = values.keys().cloned().collect::<BTreeSet<_>>();
    let missing = expected.difference(&provided).cloned().collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(TemplateError::MissingVariables(missing));
    }
    let unknown = provided.difference(&expected).cloned().collect::<Vec<_>>();
    if !unknown.is_empty() {
        return Err(TemplateError::UnknownVariables(unknown));
    }
    walk_template(template, |name| {
        Ok(values
            .get(name)
            .expect("template values were validated")
            .clone())
    })
}

fn walk_template(
    template: &str,
    mut placeholder: impl FnMut(&str) -> Result<String, TemplateError>,
) -> Result<String, TemplateError> {
    let bytes = template.as_bytes();
    let mut output = String::with_capacity(template.len());
    let mut cursor = 0usize;
    while cursor < bytes.len() {
        if bytes[cursor] == b'\\' && bytes.get(cursor + 1..cursor + 3) == Some(b"{{") {
            output.push_str("{{");
            cursor += 3;
            continue;
        }
        if bytes.get(cursor..cursor + 2) == Some(b"{{") {
            let content_start = cursor + 2;
            let Some(relative_end) = template[content_start..].find("}}") else {
                return Err(TemplateError::UnclosedPlaceholder);
            };
            let content_end = content_start + relative_end;
            let name = &template[content_start..content_end];
            if !valid_variable_name(name) {
                return Err(TemplateError::InvalidVariableName(name.to_string()));
            }
            output.push_str(&placeholder(name)?);
            cursor = content_end + 2;
            continue;
        }
        let character = template[cursor..]
            .chars()
            .next()
            .expect("cursor is in bounds");
        output.push(character);
        cursor += character.len_utf8();
    }
    Ok(output)
}

fn valid_variable_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 64 {
        return false;
    }
    let mut characters = name.bytes();
    let first = characters.next().expect("name is not empty");
    (first.is_ascii_alphabetic() || first == b'_')
        && characters.all(|character| character.is_ascii_alphanumeric() || character == b'_')
}
