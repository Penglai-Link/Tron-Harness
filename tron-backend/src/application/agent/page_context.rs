//! Submission snapshots are persisted with their input as contributor source
//! records. Only the latest input's snapshot enters the separate page block;
//! snapshots never enter model history or its compression input.
use super::dto::{AgentPageContext, AgentPageTarget};
use crate::domain::agent::{AgentMessage, AgentMessagePart};

pub(crate) const PAGE_CONTEXT_BLOCK: &str = "page_context";
const SNAPSHOT_KIND: &str = "tron_ui_page_context";

pub(crate) fn is_snapshot(part: &AgentMessagePart) -> bool {
    matches!(part, AgentMessagePart::Context { kind, .. } if kind == SNAPSHOT_KIND)
}

fn compact(value: Option<&str>, limit: usize) -> Option<String> {
    let text: String = value?
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .filter(|ch| !ch.is_control())
        .take(limit)
        .collect();
    (!text.is_empty()).then_some(text)
}

pub(crate) fn normalized(snapshot: Option<&AgentPageContext>) -> Option<AgentPageContext> {
    let snapshot = snapshot?;
    let mut result = AgentPageContext {
        title: compact(snapshot.title.as_deref(), 80),
        path: compact(
            snapshot
                .path
                .as_deref()
                .and_then(|path| path.split(['?', '#']).next()),
            160,
        )
        .filter(|path| path.starts_with('/') && !path.starts_with("//")),
        tab_title: compact(snapshot.tab_title.as_deref(), 80),
        workspace_code: compact(snapshot.workspace_code.as_deref(), 128),
        workspace_name: compact(snapshot.workspace_name.as_deref(), 80),
        target: snapshot.target.as_ref().and_then(|target| {
            let key = compact(Some(&target.key), 48)?;
            if !key
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
            {
                return None;
            }
            Some(AgentPageTarget {
                key,
                value: compact(Some(&target.value), 128)?,
            })
        }),
    };
    if result.title == result.tab_title {
        result.tab_title = None;
    }
    // Bound the serialized snapshot, including Unicode and JSON escaping. Drop
    // labels before locators rather than shortening a business identifier again.
    for field in 0..5 {
        if serde_json::to_vec(&result).ok()?.len() <= 2048 {
            break;
        }
        match field {
            0 => result.tab_title = None,
            1 => result.workspace_name = None,
            2 => result.title = None,
            3 => result.workspace_code = None,
            _ => result.target = None,
        }
    }
    if result.title.is_none()
        && result.path.is_none()
        && result.tab_title.is_none()
        && result.workspace_code.is_none()
        && result.workspace_name.is_none()
        && result.target.is_none()
    {
        return None;
    }
    Some(result)
}

pub(crate) fn record_snapshot(
    message: &mut AgentMessage,
    snapshot: Option<&AgentPageContext>,
    locale: &str,
) {
    let Some(snapshot) = normalized(snapshot) else {
        return;
    };
    let metadata = serde_json::to_value(snapshot).expect("page context is JSON serializable");
    let label = if locale.starts_with("en") {
        "UI page at this message's submission (location data only)"
    } else {
        "本条消息发送时的页面（仅位置数据）"
    };
    message.parts.insert(
        0,
        AgentMessagePart::Context {
            kind: SNAPSHOT_KIND.into(),
            text: format!("{label}: {metadata}"),
            metadata: Some(metadata),
        },
    );
}
