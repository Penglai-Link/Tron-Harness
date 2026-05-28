use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskStatus {
    Open,
    Designing,
    Red,
    Green,
    Regressioning,
    Review,
    InProgress,
    Reopen,
    Completed,
    Archived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskPriority {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskType {
    Feature,
    Bug,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskSourceType {
    Internal,
    GitHub,
    GitLab,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskRole {
    FeatureRoot,
    FeatureChild,
    Bug,
}
