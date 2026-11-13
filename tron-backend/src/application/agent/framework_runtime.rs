mod messages;
pub(crate) use messages::decode as decode_native_message;
mod model;
pub(crate) use model::NativeModelScope;
mod presentation;
pub(crate) use presentation::consume_native_agui_group;

mod context;
mod context_auxiliary;
mod context_projection;
mod context_providers;
pub(crate) mod token_estimation;

#[derive(Debug, Clone)]
pub(crate) struct ContextLayoutContributorInfo {
    pub provider_id: String,
    pub block_id: String,
    pub metadata: eliteforge_agent_core::ContextBlockMetadata,
    pub kind: eliteforge_agent_core::ContextSourceKind,
}

mod dude_context;
mod onboarding;

mod tools;

mod capability_catalog;
mod host;
pub use capability_catalog::{
    CapabilityPackInfo, ExtensionImplementationInfo, capability_pack_catalog,
    capability_pack_catalog_with_state, extension_implementation_catalog,
    extension_implementation_catalog_with_state, install_capability_pack,
    installed_capability_packs, installed_extension_implementations,
    query_installed_capability_packs,
};
pub use host::{
    DudeThreadCatalogItem, FrameworkHost, SubagentRecord, SubagentView, ThreadFeedback,
    ThreadFeedbackSink,
};
pub(crate) use host::{PendingSudoInput, SudoInputError};

mod external;
mod native_session_scope;
mod native_sessions;
pub(crate) use external::FrameworkExternalRuntime;
pub(crate) use native_session_scope::NativeCollaborationScope;

mod question;

mod auxiliary_models;
pub(crate) use auxiliary_models::attach_auxiliary_model;

pub(crate) mod summary_loop;

mod command_risk_review;

/// Product policies use zero for unlimited; the SDK reserves zero for no tool access.
fn runtime_count_limit(value: u32) -> usize {
    if value == 0 {
        eliteforge_agent_framework::RunLimits::UNLIMITED_COUNT
    } else {
        value as usize
    }
}
