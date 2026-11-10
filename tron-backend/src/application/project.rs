mod directory;
pub use directory::{
    ApplicationDirectoryInspector, ProjectInstructionFile, inspect_current_application,
};

mod service;
pub use service::*;

// Active topics reserve names; archived topics do not.
pub fn duplicate_name_descriptor() -> crate::application::chat::history_error::ApplicationErrorDescriptor {
    crate::application::chat::history_error::ApplicationErrorDescriptor {
        code: "101046001",
        message_key: "tron.projects.duplicate_name",
    }
}
