use std::{error::Error, fmt};

use crate::domain::rag::RagError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryError {
    Validation(String),
    NotFound(String),
    Rag(String),
}

impl fmt::Display for MemoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) | Self::NotFound(message) | Self::Rag(message) => {
                formatter.write_str(message)
            }
        }
    }
}

impl Error for MemoryError {}

impl From<RagError> for MemoryError {
    fn from(error: RagError) -> Self {
        Self::Rag(error.to_string())
    }
}
