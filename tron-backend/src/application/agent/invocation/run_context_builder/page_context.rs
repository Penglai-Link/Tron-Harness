use super::*;

pub(super) fn page_context_message(
    page_context: Option<&AgentPageContext>,
) -> Option<AgentMessage> {
    let page_context = page_context?;
    let title = truncate_chars(page_context.title.trim(), 80);
    let path = page_context
        .path
        .trim()
        .split(['?', '#'])
        .next()
        .unwrap_or_default();
    let path = truncate_chars(path, 160);
    if title.is_empty() && path.is_empty() {
        return None;
    }

    let metadata = serde_json::json!({ "title": title, "path": path });
    Some(AgentMessage::system(format!(
        "Opened from UI page (metadata only): {metadata}"
    )))
}

fn truncate_chars(value: &str, limit: usize) -> String {
    value.chars().take(limit).collect()
}
