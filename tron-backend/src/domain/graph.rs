mod error;
mod lpg;
#[cfg(test)]
mod lpg_tests;
mod ontology;
mod scope;

pub use error::{GraphError, GraphScopeError, OntologyError};
pub use lpg::*;
pub use ontology::*;
pub(crate) use scope::resolve_storage_scope;
pub use scope::{
    GraphScope, GraphScopeCommand, GraphScopeDimension, GraphStorageDimensions,
    GraphStorageProperty, ResolvedGraphScope, STORAGE_ABSENT_SENTINEL,
    ensure_not_reserved_storage_property,
};
