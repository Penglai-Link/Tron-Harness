use super::{
    TaskDefaultStatuses, TaskOptions, TaskPriority, TaskRole, TaskSourceType, TaskStatus,
    TaskStatusRules, TaskType,
};

pub const TASK_STATUSES: [TaskStatus; 10] = [
    TaskStatus::Open,
    TaskStatus::Designing,
    TaskStatus::Red,
    TaskStatus::Green,
    TaskStatus::Regressioning,
    TaskStatus::Review,
    TaskStatus::InProgress,
    TaskStatus::Reopen,
    TaskStatus::Completed,
    TaskStatus::Archived,
];
pub const FEATURE_ROOT_STATUSES: [TaskStatus; 8] = [
    TaskStatus::Open,
    TaskStatus::Designing,
    TaskStatus::Red,
    TaskStatus::Green,
    TaskStatus::Regressioning,
    TaskStatus::Review,
    TaskStatus::Completed,
    TaskStatus::Archived,
];
pub const FEATURE_CHILD_STATUSES: [TaskStatus; 4] = [
    TaskStatus::Open,
    TaskStatus::InProgress,
    TaskStatus::Completed,
    TaskStatus::Archived,
];
pub const BUG_STATUSES: [TaskStatus; 5] = [
    TaskStatus::Open,
    TaskStatus::InProgress,
    TaskStatus::Reopen,
    TaskStatus::Completed,
    TaskStatus::Archived,
];
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

pub fn infer_task_role(task_type: TaskType, parent_task_code: Option<&str>) -> TaskRole {
    match task_type {
        TaskType::Bug => TaskRole::Bug,
        TaskType::Feature => {
            let has_parent = parent_task_code.is_some_and(|value| !value.trim().is_empty());
            if has_parent {
                TaskRole::FeatureChild
            } else {
                TaskRole::FeatureRoot
            }
        }
    }
}

pub fn task_statuses_for_role(role: TaskRole) -> &'static [TaskStatus] {
    match role {
        TaskRole::FeatureRoot => &FEATURE_ROOT_STATUSES,
        TaskRole::FeatureChild => &FEATURE_CHILD_STATUSES,
        TaskRole::Bug => &BUG_STATUSES,
    }
}

pub fn task_default_status_for_role(role: TaskRole) -> TaskStatus {
    match role {
        TaskRole::FeatureRoot | TaskRole::FeatureChild | TaskRole::Bug => TaskStatus::Open,
    }
}

pub fn task_status_allowed_for_role(role: TaskRole, status: TaskStatus) -> bool {
    task_statuses_for_role(role).contains(&status)
}

pub fn task_options() -> TaskOptions {
    TaskOptions {
        statuses: TASK_STATUSES.to_vec(),
        priorities: TASK_PRIORITIES.to_vec(),
        task_types: TASK_TYPES.to_vec(),
        source_types: TASK_SOURCE_TYPES.to_vec(),
        status_rules: TaskStatusRules {
            feature_root: FEATURE_ROOT_STATUSES.to_vec(),
            feature_child: FEATURE_CHILD_STATUSES.to_vec(),
            bug: BUG_STATUSES.to_vec(),
        },
        default_statuses: TaskDefaultStatuses {
            feature_root: task_default_status_for_role(TaskRole::FeatureRoot),
            feature_child: task_default_status_for_role(TaskRole::FeatureChild),
            bug: task_default_status_for_role(TaskRole::Bug),
        },
    }
}
