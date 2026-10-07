mod messages;
mod model;
pub(crate) use model::NativeModelScope;
mod presentation;
pub(crate) use presentation::consume_native_agui_group;

mod context;
mod context_auxiliary;
mod context_projection;
mod context_providers;

mod tools;

mod capability_catalog;
mod host;
pub use capability_catalog::{
    CapabilityPackInfo, ExtensionImplementationInfo, capability_pack_catalog,
    capability_pack_catalog_with_state, extension_implementation_catalog,
    extension_implementation_catalog_with_state, install_capability_pack,
    installed_capability_packs,
};
pub use host::FrameworkHost;

mod external;
mod native_sessions;
pub(crate) use external::FrameworkExternalRuntime;

mod question;

#[cfg(test)]
mod context_auth_recovery_tests;

mod auxiliary_models;
pub(crate) use auxiliary_models::attach_auxiliary_model;

pub(crate) mod summary_loop;
