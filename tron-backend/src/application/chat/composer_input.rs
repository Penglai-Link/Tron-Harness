use super::history_error::ApplicationErrorDescriptor;
use crate::domain::chat::ChatError;

pub fn descriptor(error: &ChatError) -> Option<ApplicationErrorDescriptor> {
    let (code, message_key) = match error {
        ChatError::AnnotationsInvalid => ("101032006", "tron.chat.annotations_invalid"),
        ChatError::RequestedSkillsInvalid => ("101032001", "tron.chat.requested_skills_invalid"),
        ChatError::RequestedSkillForbidden => ("101032002", "tron.chat.requested_skill_forbidden"),
        ChatError::GoalCommandInvalid => ("101032003", "tron.chat.goal_command_invalid"),
        ChatError::GoalCommandConflict => ("101032005", "tron.chat.goal_command_conflict"),
        ChatError::GoalCommandUnavailable => ("101032004", "tron.chat.goal_command_unavailable"),
        _ => return None,
    };
    Some(ApplicationErrorDescriptor { code, message_key })
}

pub(super) fn goal_objective(content: &str) -> Result<Option<&str>, ChatError> {
    let content = content.trim();
    let end = content.find(char::is_whitespace).unwrap_or(content.len());
    if !content[..end].eq_ignore_ascii_case("/goal") {
        return Ok(None);
    }
    let objective = content[end..].trim();
    if objective.is_empty() {
        return Err(ChatError::GoalCommandInvalid);
    }
    Ok(Some(objective))
}

pub(super) fn goal_error(
    error: crate::domain::short_term_memory::ShortTermMemoryError,
) -> ChatError {
    use crate::domain::short_term_memory::ShortTermMemoryError;
    match error {
        ShortTermMemoryError::GoalConflict
        | ShortTermMemoryError::GoalStateConflict
        | ShortTermMemoryError::GoalNotFound => ChatError::GoalCommandConflict,
        ShortTermMemoryError::GoalInvalid => ChatError::GoalCommandInvalid,
        ShortTermMemoryError::SessionNotFound => ChatError::SessionNotFound,
        _ => ChatError::StorageFailure,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn goal_requires_a_complete_command_and_objective() {
        assert!(matches!(
            goal_objective("/goal  "),
            Err(ChatError::GoalCommandInvalid)
        ));
        assert_eq!(
            goal_objective(" /GOAL build\na preview ").unwrap(),
            Some("build\na preview")
        );
        assert_eq!(goal_objective("/goalkeeper check").unwrap(), None);
        assert_eq!(goal_objective("explain /goal").unwrap(), None);
    }
}
