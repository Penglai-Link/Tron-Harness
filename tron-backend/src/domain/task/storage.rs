use super::{TaskError, TaskPriority, TaskSourceType, TaskType};

pub fn task_priority_to_storage(priority: TaskPriority) -> &'static str {
    match priority {
        TaskPriority::Low => "Low",
        TaskPriority::Medium => "Medium",
        TaskPriority::High => "High",
        TaskPriority::ExtraHigh => "Extra High",
    }
}

pub fn task_priority_from_storage(value: &str) -> Result<TaskPriority, TaskError> {
    match value {
        "Low" => Ok(TaskPriority::Low),
        "Medium" => Ok(TaskPriority::Medium),
        "High" => Ok(TaskPriority::High),
        "Extra High" => Ok(TaskPriority::ExtraHigh),
        _ => Err(TaskError::InvalidRequest),
    }
}

pub fn task_type_to_storage(task_type: TaskType) -> &'static str {
    match task_type {
        TaskType::Feature => "Feature",
        TaskType::Bug => "Bug",
    }
}

pub fn task_type_from_storage(value: &str) -> Result<TaskType, TaskError> {
    match value {
        "Feature" => Ok(TaskType::Feature),
        "Bug" => Ok(TaskType::Bug),
        _ => Err(TaskError::InvalidRequest),
    }
}

pub fn task_source_type_to_storage(source_type: TaskSourceType) -> &'static str {
    match source_type {
        TaskSourceType::Internal => "Internal",
        TaskSourceType::GitHub => "GitHub",
        TaskSourceType::GitLab => "GitLab",
    }
}

pub fn task_source_type_from_storage(value: &str) -> Result<TaskSourceType, TaskError> {
    match value {
        "Internal" => Ok(TaskSourceType::Internal),
        "GitHub" => Ok(TaskSourceType::GitHub),
        "GitLab" => Ok(TaskSourceType::GitLab),
        _ => Err(TaskError::InvalidRequest),
    }
}
