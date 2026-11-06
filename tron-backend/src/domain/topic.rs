use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TopicReferenceKind {
    Directory,
    Repository,
    Resource,
    Task,
    Link,
}

impl TopicReferenceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Directory => "Directory",
            Self::Repository => "Repository",
            Self::Resource => "Resource",
            Self::Task => "Task",
            Self::Link => "Link",
        }
    }
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "Directory" => Some(Self::Directory),
            "Repository" => Some(Self::Repository),
            "Resource" => Some(Self::Resource),
            "Task" => Some(Self::Task),
            "Link" => Some(Self::Link),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopicReference {
    pub reference_code: String,
    pub kind: TopicReferenceKind,
    pub target_code: String,
    pub connection_code: Option<String>,
    pub label: String,
    pub url: Option<String>,
    pub available: bool,
    pub directory_path: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopicMemo {
    pub project_code: String,
    pub notes: String,
    pub revision: u64,
    pub references: Vec<TopicReference>,
}

#[derive(Debug)]
pub enum TopicError {
    NotFound,
    InvalidReference,
    InvalidNotes,
    Conflict,
    StorageFailure,
}
