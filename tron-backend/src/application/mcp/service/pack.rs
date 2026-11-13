//! Refresh a package-owned entrypoint when the package changes.
use super::{
    McpHealthStatus, McpServerTransportKind, McpService, PlatformMcpServerRecord, current_timestamp,
};
use eliteforge_agent_capability_runtime::{McpDeclaration, McpDeclarationTransport};
use std::path::Path;

impl McpService {
    /// Keep a package-owned server aligned with its current manifest declaration.
    ///
    /// Enablement, credentials, environment and user labels are preserved.
    /// Commands that no longer point inside the package are treated as explicit
    /// user overrides and are never rewritten.
    pub(super) fn reconciled_pack_entrypoint(
        &self,
        server: &PlatformMcpServerRecord,
        declaration: &McpDeclaration,
        directory: &Path,
        pack_id: &str,
    ) -> Option<PlatformMcpServerRecord> {
        let owner = self.capability_resource(&declaration.id)?;
        if owner.pack_id != pack_id {
            return None;
        }
        let (transport_kind, url, command, args) = match &declaration.transport {
            McpDeclarationTransport::Stdio { program, args } => (
                McpServerTransportKind::Stdio,
                String::new(),
                program.clone(),
                args.clone(),
            ),
            McpDeclarationTransport::Http { url } => (
                McpServerTransportKind::StreamableHttp,
                url.clone(),
                String::new(),
                vec![],
            ),
        };
        if transport_kind == McpServerTransportKind::Stdio
            && !Path::new(&command).starts_with(directory)
        {
            return None;
        }
        if server.transport_kind == McpServerTransportKind::Stdio {
            let previous = Path::new(server.working_directory.as_deref()?);
            if !Path::new(&server.command).starts_with(previous) {
                return None;
            }
        }
        let working_directory = Some(directory.display().to_string());
        if server.transport_kind == transport_kind
            && server.url == url
            && server.command == command
            && server.args == args
            && server.working_directory == working_directory
        {
            return None;
        }
        Some(PlatformMcpServerRecord {
            transport_kind,
            url,
            command,
            args,
            working_directory,
            last_health_status: McpHealthStatus::NeverChecked,
            last_health_message: None,
            last_health_checked_at: None,
            updated_at: current_timestamp(),
            ..server.clone()
        })
    }
}
