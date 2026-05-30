use std::{error::Error, fmt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RagScopeFields {
    pub scope_type: String,
    pub scope_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RagError {
    StorageFailure,
}

impl fmt::Display for RagError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StorageFailure => formatter.write_str("RAG storage failure"),
        }
    }
}

impl Error for RagError {}
