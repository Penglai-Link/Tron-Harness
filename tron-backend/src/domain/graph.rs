mod error;
mod scope;

pub use error::GraphScopeError;
pub(crate) use scope::resolve_storage_scope;
pub use scope::{
    GraphScope, GraphScopeCommand, GraphScopeDimension, GraphStorageDimensions,
    GraphStorageProperty, ResolvedGraphScope, STORAGE_ABSENT_SENTINEL,
    ensure_not_reserved_storage_property,
};
