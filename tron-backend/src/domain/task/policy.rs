use super::{TaskPriority, TaskSourceType, TaskType};
pub const TASK_PRIORITIES: [TaskPriority; 4] = [
    TaskPriority::Low,
    TaskPriority::Medium,
    TaskPriority::High,
    TaskPriority::ExtraHigh,
];
pub const TASK_TYPES: [TaskType; 2] = [TaskType::Feature, TaskType::Bug];
pub const TASK_SOURCE_TYPES: [TaskSourceType; 3] = [
    TaskSourceType::Internal,
    TaskSourceType::GitHub,
    TaskSourceType::GitLab,
];
