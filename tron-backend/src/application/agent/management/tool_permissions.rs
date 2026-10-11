use super::*;
use crate::domain::agent::{ToolAvailabilityState, ToolOperationKind};
use eliteforge_agent_core::ToolDescriptor;
use std::collections::{BTreeMap, HashSet};
use tron_harness_capability::{ToolPermission, ToolPermissionSetting, ToolPermissionSource};

impl AgentManagementService {
    pub fn tool_permissions(&self) -> Result<Vec<ToolPermissionSetting>, AgentManagementError> {
        let mut saved: BTreeMap<_, _> = self
            .repository()
            .list_tool_permissions()?
            .into_iter()
            .map(|s| (s.tool_code.clone(), s))
            .collect();
        for code in BusinessToolCatalog::tool_codes().into_iter().chain(
            self.repository()
                .list_discovered_tools()?
                .into_iter()
                .map(|t| t.tool_code),
        ) {
            saved
                .entry(code.clone())
                .or_insert_with(|| ToolPermissionSetting::default_for(code));
        }
        Ok(saved.into_values().collect())
    }
    pub fn replace_tool_permissions(
        &self,
        settings: Vec<ToolPermissionSetting>,
    ) -> Result<Vec<ToolPermissionSetting>, AgentManagementError> {
        let known: HashSet<_> = self
            .tool_permissions()?
            .into_iter()
            .map(|s| s.tool_code)
            .collect();
        let mut seen = HashSet::new();
        for setting in &settings {
            if setting.tool_code.trim() != setting.tool_code
                || !known.contains(&setting.tool_code)
                || !seen.insert(&setting.tool_code)
            {
                return Err(AgentManagementError::ToolPermissionInvalid);
            }
        }
        self.repository().replace_tool_permissions(&settings)?;
        self.tool_permissions()
    }
    pub(crate) fn platform_tool_allowed(&self, name: &str) -> Result<bool, AgentManagementError> {
        let records = self.repository().list_discovered_tools()?;
        let code = BusinessToolCatalog::list_tools(None, None, "en-US")
            .into_iter()
            .chain(records)
            .find(|r| r.tool_name == name)
            .map(|r| r.tool_code);
        let settings = self.repository().list_tool_permissions()?;
        Ok(code.is_none_or(|code| {
            self.capability_available("tool", &code)
                && !settings
                    .iter()
                    .any(|s| s.tool_code == code && s.permission == ToolPermission::No)
        }))
    }
}
impl ToolPermissionSource for AgentManagementService {
    fn setting(&self, descriptor: &ToolDescriptor) -> Result<ToolPermissionSetting, String> {
        let fixed = BusinessToolCatalog::list_tools(None, None, "en-US")
            .into_iter()
            .find(|r| r.tool_name == descriptor.call_name);
        let code = if let Some(record) = fixed {
            record.tool_code
        } else {
            if BusinessToolCatalog::tool(&descriptor.id, "en-US").is_some() {
                return Err("Native descriptor conflicts with a platform tool code".into());
            }
            // Only a real provider lease can register this native descriptor.
            let record = ToolCatalogRecord {
                tool_code: descriptor.id.clone(),
                tool_name: descriptor.call_name.clone(),
                domain_code: if descriptor.id.starts_with("mcp/") {
                    "mcp"
                } else {
                    "native"
                }
                .into(),
                name: descriptor.call_name.clone(),
                description: descriptor.description.clone(),
                operation_kind: ToolOperationKind::Command,
                input_schema: descriptor.input_schema.clone(),
                module_owner: "Framework".into(),
                availability_state: ToolAvailabilityState::Available,
                sample_result_summary: String::new(),
            };
            self.repository()
                .remember_discovered_tool(&record)
                .map_err(|_| "Could not read platform tool permission".to_string())?;
            descriptor.id.clone()
        };
        let settings = self
            .repository()
            .list_tool_permissions()
            .map_err(|_| "Could not read platform tool permission".to_string())?;
        Ok(settings
            .into_iter()
            .find(|s| s.tool_code == code)
            .unwrap_or_else(|| ToolPermissionSetting::default_for(code)))
    }
    fn operation(&self, descriptor: &ToolDescriptor) -> Option<bool> {
        BusinessToolCatalog::list_tools(None, None, "en-US")
            .into_iter()
            .find(|r| r.tool_name == descriptor.call_name)
            .map(|r| r.operation_kind == ToolOperationKind::Command)
    }
}
