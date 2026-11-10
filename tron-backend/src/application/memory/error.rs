use std::{error::Error, fmt};

use crate::domain::rag::RagError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryError {
    Validation(String),
    NotFound(String),
    Forbidden(String),
    Conflict(String),
    Storage(String),
    Rag(String),
}

impl fmt::Display for MemoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message)
            | Self::NotFound(message)
            | Self::Forbidden(message)
            | Self::Conflict(message)
            | Self::Storage(message)
            | Self::Rag(message) => formatter.write_str(message),
        }
    }
}

impl Error for MemoryError {}

impl From<RagError> for MemoryError {
    fn from(error: RagError) -> Self {
        Self::Rag(error.to_string())
    }
}

pub struct ApplicationErrorDescriptor {
    pub code: &'static str,
    pub message_key: &'static str,
}
impl MemoryError {
    pub fn descriptor(&self) -> ApplicationErrorDescriptor {
        let (code, message_key) = match self {
            Self::Validation(_) => ("101042001", "tron.memory.invalid"),
            Self::NotFound(_) => ("101042002", "tron.memory.not_found"),
            Self::Forbidden(_) => ("101042003", "tron.memory.forbidden"),
            Self::Conflict(_) => ("101042004", "tron.memory.conflict"),
            Self::Storage(_) => ("101042005", "tron.memory.storage"),
            Self::Rag(_) => ("101042006", "tron.memory.index"),
        };
        ApplicationErrorDescriptor { code, message_key }
    }
}
