mod access;
mod bundles;
mod evidence;
pub mod graph;
mod catalog;
mod catalog_service;
mod diagnostics;
mod facade;
mod indexing;
mod ingest;
mod lifecycle;
mod management;
mod memory_retrieval;
mod record_ingestion;
mod retrieval;

mod runtime;
mod search;
mod search_filter;
mod source_parts;
pub(crate) use source_parts::selected_spans as selected_source_spans;
mod service;
mod task_content;
pub use access::*;
pub use evidence::*;
pub use catalog::*;
pub use catalog_service::{RagTaskHistoryError, TaskHistoryErrorDescriptor};
pub use diagnostics::*;
pub use facade::*;
pub use indexing::*;
pub use ingest::*;
pub use lifecycle::*;
pub use management::*;
pub use retrieval::*;

pub use runtime::*;
pub use search::*;
pub use search_filter::*;
pub use service::*;
pub use task_content::RagTaskChunksError;

pub use crate::application::document::split_document;

// The shared memory corpus uses platform models for writes and retrieval alike.
// Business workspace codes remain exclusively in access scopes and persisted tasks.
const MEMORY_MODEL_WORKSPACE: &str = "PLATFORM";
