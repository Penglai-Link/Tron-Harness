use crate::{
    domain::mcp::{McpError, McpTransportAvailability, PlatformMcpGatewayConfig, UpdatePlatformMcpGatewayCommand},
    shared::time::current_timestamp,
};

use super::McpService;

impl McpService {
    pub fn get_gateway_config(&self) -> Result<PlatformMcpGatewayConfig, McpError> {
        let mut repository = self
            .repository
            .lock()
            .expect("MCP repository mutex poisoned");
        if let Some(config) = repository.get_gateway_config()? {
            return Ok(config);
        }

        let now = current_timestamp();
        let config = PlatformMcpGatewayConfig {
            gateway_enabled: false,
            variables_endpoint_enabled: false,
            created_at: Some(now.clone()),
            updated_at: Some(now),
        };
        repository.upsert_gateway_config(&config)?;
        Ok(config)
    }

    pub fn update_gateway_config(
        &self,
        command: UpdatePlatformMcpGatewayCommand,
    ) -> Result<PlatformMcpGatewayConfig, McpError> {
        let current = self.get_gateway_config()?;
        let now = current_timestamp();
        let updated = PlatformMcpGatewayConfig {
            gateway_enabled: command.gateway_enabled,
            variables_endpoint_enabled: command.variables_endpoint_enabled,
            created_at: current.created_at,
            updated_at: Some(now),
        };
        self.repository
            .lock()
            .expect("MCP repository mutex poisoned")
            .upsert_gateway_config(&updated)?;
        Ok(updated)
    }

    pub fn variables_transport_availability(&self) -> Result<McpTransportAvailability, McpError> {
        let config = self.get_gateway_config()?;
        Ok(McpTransportAvailability {
            gateway_enabled: config.gateway_enabled,
            variables_endpoint_enabled: config.variables_endpoint_enabled,
        })
    }
}
