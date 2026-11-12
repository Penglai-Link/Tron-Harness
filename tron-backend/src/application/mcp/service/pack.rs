//! Refresh a package-owned default entrypoint when a new version is selected.
use super::{
    McpHealthStatus, McpServerTransportKind, McpService, PlatformMcpServerRecord, current_timestamp,
};
use eliteforge_agent_capability_runtime::{McpDeclaration, McpDeclarationTransport};
use std::path::Path;

impl McpService {
    pub(super) fn upgraded_pack_entrypoint(
        &self,
        server: &PlatformMcpServerRecord,
        declaration: &McpDeclaration,
        directory: &Path,
    ) -> Option<PlatformMcpServerRecord> {
        self.capability_resource(&declaration.id)?;
        let McpDeclarationTransport::Stdio { program, args } = &declaration.transport else {
            return None;
        };
        let previous = Path::new(server.working_directory.as_deref()?);
        if server.transport_kind != McpServerTransportKind::Stdio
            || previous == directory
            || previous.parent() != directory.parent()
            || !Path::new(&server.command).starts_with(previous)
            || !Path::new(program).starts_with(directory)
        {
            return None;
        }
        // Keep platform/workspace enablement, credentials, environment and user
        // labels. Explicit commands or working directories outside the old pack
        // are user overrides and do not match the package entrypoint above.
        Some(PlatformMcpServerRecord {
            command: program.clone(),
            args: args.clone(),
            working_directory: Some(directory.display().to_string()),
            last_health_status: McpHealthStatus::NeverChecked,
            last_health_message: None,
            last_health_checked_at: None,
            updated_at: current_timestamp(),
            ..server.clone()
        })
    }
}
