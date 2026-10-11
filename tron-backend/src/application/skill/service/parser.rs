use super::*;

pub(super) fn parse_skill_frontmatter(markdown: &str) -> SkillFrontmatter {
    let mut lines = markdown.lines();
    if lines.next() != Some("---") {
        return SkillFrontmatter::default();
    }

    let mut yaml_lines = Vec::new();
    for line in lines {
        if line == "---" {
            let yaml = yaml_lines.join("\n");
            let value = match serde_yaml::from_str::<serde_yaml::Value>(&yaml) {
                Ok(value) => value,
                Err(_) => return SkillFrontmatter::default(),
            };
            let metadata = value.get("metadata");
            return SkillFrontmatter {
                declared_version: yaml_value_as_string(value.get("version"))
                    .or_else(|| metadata.and_then(|item| yaml_mapping_value(item, "version"))),
                description: yaml_value_as_string(value.get("description"))
                    .or_else(|| metadata.and_then(|item| yaml_mapping_value(item, "description"))),
            };
        }
        yaml_lines.push(line.to_owned());
    }

    SkillFrontmatter::default()
}

pub(super) fn yaml_value_as_string(value: Option<&serde_yaml::Value>) -> Option<String> {
    match value {
        Some(serde_yaml::Value::String(value)) => Some(value.clone()),
        Some(other) => serde_json::to_string(other).ok(),
        None => None,
    }
}

pub(super) fn yaml_mapping_value(value: &serde_yaml::Value, key: &str) -> Option<String> {
    match value {
        serde_yaml::Value::Mapping(mapping) => mapping
            .get(serde_yaml::Value::String(key.to_owned()))
            .and_then(|item| yaml_value_as_string(Some(item))),
        _ => None,
    }
}

pub(super) fn parse_search_output(stdout: &[u8]) -> Vec<SkillSearchCandidate> {
    let normalized = strip_ansi(&String::from_utf8_lossy(stdout));
    let mut candidates = Vec::new();
    let mut pending: Option<SearchLine> = None;

    for raw_line in normalized.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with("Install with") {
            continue;
        }
        if let Some(detail_url) = line.strip_prefix("└ ") {
            if let Some(pending_line) = pending.take()
                && let Ok((source_ref, skill_name)) = split_package_ref(&pending_line.package_ref)
            {
                candidates.push(SkillSearchCandidate {
                    package_ref: pending_line.package_ref,
                    source_ref,
                    skill_name,
                    description: None,
                    installs_label: pending_line.installs_label,
                    detail_url: Some(detail_url.trim().to_owned()),
                });
            }
            continue;
        }

        if let Some(search_line) = parse_search_line(line) {
            pending = Some(search_line);
        }
    }

    if let Some(pending_line) = pending
        && let Ok((source_ref, skill_name)) = split_package_ref(&pending_line.package_ref)
    {
        candidates.push(SkillSearchCandidate {
            package_ref: pending_line.package_ref,
            source_ref,
            skill_name,
            description: None,
            installs_label: pending_line.installs_label,
            detail_url: None,
        });
    }

    candidates
}

pub(super) fn parse_repo_listing_output(
    stdout: &[u8],
    source_ref: &str,
) -> Vec<SkillSearchCandidate> {
    let normalized = strip_ansi(&String::from_utf8_lossy(stdout));
    let source_ref = source_ref.trim();
    let mut candidates = Vec::new();
    let mut in_skills_section = false;
    let mut current_skill: Option<String> = None;
    let mut description_lines: Vec<String> = Vec::new();

    fn flush_candidate(
        candidates: &mut Vec<SkillSearchCandidate>,
        source_ref: &str,
        current_skill: &mut Option<String>,
        description_lines: &mut Vec<String>,
    ) {
        let Some(skill_name) = current_skill.take() else {
            return;
        };
        let description = join_description_lines(description_lines);
        candidates.push(SkillSearchCandidate {
            package_ref: format!("{source_ref}@{skill_name}"),
            source_ref: source_ref.to_owned(),
            skill_name,
            description,
            installs_label: None,
            detail_url: None,
        });
        description_lines.clear();
    }

    for raw_line in normalized.lines() {
        let content = trim_cli_line(raw_line);
        if content.is_empty() {
            continue;
        }
        if !in_skills_section {
            if content == "Available Skills" {
                in_skills_section = true;
            }
            continue;
        }
        if content.starts_with("Use --skill ") {
            break;
        }
        if is_repo_skill_name(content) {
            flush_candidate(
                &mut candidates,
                source_ref,
                &mut current_skill,
                &mut description_lines,
            );
            current_skill = Some(content.to_owned());
            continue;
        }
        if current_skill.is_some() {
            description_lines.push(content.to_owned());
        }
    }

    flush_candidate(
        &mut candidates,
        source_ref,
        &mut current_skill,
        &mut description_lines,
    );

    candidates
}

pub(super) fn parse_search_line(line: &str) -> Option<SearchLine> {
    if let Some(prefix) = line.strip_suffix(" installs")
        && let Some((package_ref, install_count)) = prefix.rsplit_once(' ')
    {
        return Some(SearchLine {
            package_ref: package_ref.trim().to_owned(),
            installs_label: Some(format!("{install_count} installs")),
        });
    }

    normalized_required(line).map(|package_ref| SearchLine {
        package_ref,
        installs_label: None,
    })
}

pub(super) fn trim_cli_line(line: &str) -> &str {
    line.trim()
        .trim_start_matches(['│', '└', '◇', '●', '◒', '◐', '◓', '◑', ' '])
        .trim()
}

pub(super) fn is_repo_skill_name(line: &str) -> bool {
    !line.is_empty()
        && !line.contains(char::is_whitespace)
        && line
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
}

pub(super) fn join_description_lines(lines: &mut Vec<String>) -> Option<String> {
    if lines.is_empty() {
        return None;
    }
    let joined = lines
        .iter()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    lines.clear();
    normalized_required(joined)
}

pub(super) fn strip_ansi(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            if chars.peek().is_some_and(|next| *next == '[') {
                let _ = chars.next();
                for next in chars.by_ref() {
                    if ('@'..='~').contains(&next) {
                        break;
                    }
                }
                continue;
            }
            continue;
        }
        output.push(ch);
    }

    output
}

pub(super) fn split_package_ref(package_ref: &str) -> Result<(String, String), SkillError> {
    let package_ref = normalized_required(package_ref).ok_or(SkillError::SourceRefRequired)?;
    let (source_ref, skill_name) = package_ref
        .rsplit_once('@')
        .ok_or(SkillError::SkillNameRequired)?;
    let source_ref = normalized_required(source_ref).ok_or(SkillError::SourceRefRequired)?;
    let skill_name = normalized_required(skill_name).ok_or(SkillError::SkillNameRequired)?;
    Ok((source_ref, skill_name))
}

pub(super) fn build_package_ref(
    source_ref: Option<String>,
    skill_name: Option<String>,
) -> Option<String> {
    let source_ref = optional_trimmed(source_ref)?;
    let skill_name = optional_trimmed(skill_name)?;
    Some(format!("{source_ref}@{skill_name}"))
}

pub(super) fn normalized_required(value: impl Into<String>) -> Option<String> {
    let value = value.into();
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

pub(super) fn optional_trimmed(value: Option<String>) -> Option<String> {
    value.and_then(normalized_required)
}

pub(super) fn normalized_search(value: Option<String>) -> Option<String> {
    optional_trimmed(value).map(|search| search.to_lowercase())
}

pub(super) fn parse_skill_name(markdown: &str) -> Result<String, SkillError> {
    let fm = markdown
        .strip_prefix("---")
        .and_then(|s| s.split_once("---"))
        .ok_or(SkillError::SkillNameRequired)?
        .0;
    let yaml: serde_yaml::Value =
        serde_yaml::from_str(fm).map_err(|_| SkillError::SkillNameRequired)?;
    let name = yaml["name"]
        .as_str()
        .filter(|s| !s.trim().is_empty() && s.trim() == *s)
        .ok_or(SkillError::SkillNameRequired)?;
    Ok(name.into())
}
