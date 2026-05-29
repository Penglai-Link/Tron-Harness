mod scope_resolver;
mod service;
#[cfg(test)]
mod service_tests;

pub use scope_resolver::{ScopeResolver, WorkspaceDetailProvider};
pub use service::*;
