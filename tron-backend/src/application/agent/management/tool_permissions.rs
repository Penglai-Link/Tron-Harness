use super::*;
use crate::domain::agent::{ToolAvailabilityState, ToolOperationKind};
use eliteforge_agent_core::ToolDescriptor;
use std::collections::{BTreeMap, HashSet};
use tron_harness_capability::{ToolPermission, ToolPermissionSetting, ToolPermissionSource};

// Built-in descriptors are immutable; constructing their schemas for every
// admission is as unnecessary as querying all persisted tools for each binding.
fn fixed_tool(name: &str) -> Option<ToolCatalogRecord> {
    static TOOLS: std::sync::OnceLock<BTreeMap<String, ToolCatalogRecord>> =
        std::sync::OnceLock::new();
    TOOLS
        .get_or_init(|| {
            BusinessToolCatalog::list_tools(None, None, "en-US")
                .into_iter()
                .map(|tool| (tool.tool_name.clone(), tool))
                .collect()
        })
        .get(name)
        .cloned()
}

impl AgentManagementService {
    pub fn tool_permissions(&self) -> Result<Vec<ToolPermissionSetting>, AgentManagementError> {
        let mut saved: BTreeMap<_, _> = self
            .cached_tool_permissions()?
            .into_iter()
            .map(|s| (s.tool_code.clone(), s))
            .collect();
        for code in BusinessToolCatalog::tool_codes().into_iter().chain(
            self.cached_discovered_tools()?
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
        self.save_tool_permissions(&settings)?;
        self.tool_permissions()
    }
    pub(crate) fn platform_tool_allowed(&self, name: &str) -> Result<bool, AgentManagementError> {
        let tool = match fixed_tool(name) {
            Some(tool) => Some(tool),
            None => self.cached_tool_by_name(name)?,
        };
        let Some(tool) = tool else {
            return Ok(true);
        };
        self.platform_tool_record_allowed(&tool)
    }
    pub(crate) fn platform_tool_record_allowed(
        &self,
        tool: &ToolCatalogRecord,
    ) -> Result<bool, AgentManagementError> {
        Ok(self.capability_available("tool", &tool.tool_code)
            && self.cached_tool_permission(&tool.tool_code)?.permission != ToolPermission::No)
    }
}
impl AgentManagementService {
    pub(crate) fn describe_tool_descriptor(
        &self,
        descriptor: &ToolDescriptor,
    ) -> Result<ToolCatalogRecord, String> {
        let fixed = fixed_tool(&descriptor.call_name);
        if let Some(record) = fixed {
            return Ok(record);
        }
        if BusinessToolCatalog::tool(&descriptor.id, "en-US").is_some() {
            return Err("Native descriptor conflicts with a platform tool code".into());
        }
        // Only a real provider lease can register this native descriptor.
        let record = ToolCatalogRecord {
            tool_code: descriptor.id.clone(),
            tool_name: descriptor.call_name.clone(),
            domain_code: self
                .cached_tool_by_name(&descriptor.call_name)
                .map_err(|_| "Could not read tool catalog".to_string())?
                .map(|record| record.domain_code)
                .unwrap_or_else(|| "native".into()),
            name: descriptor.call_name.clone(),
            description: descriptor.description.clone(),
            operation_kind: match descriptor.operation_kind {
                eliteforge_agent_core::ToolOperationKind::Query => ToolOperationKind::Query,
                eliteforge_agent_core::ToolOperationKind::Command => ToolOperationKind::Command,
                eliteforge_agent_core::ToolOperationKind::Unknown => ToolOperationKind::Unknown,
            },
            input_schema: descriptor.input_schema.clone(),
            module_owner: "Framework".into(),
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary: String::new(),
        };
        Ok(record)
    }
    pub(crate) fn register_tool_descriptor(
        &self,
        descriptor: &ToolDescriptor,
    ) -> Result<ToolCatalogRecord, String> {
        let record = self.describe_tool_descriptor(descriptor)?;
        if fixed_tool(&descriptor.call_name).is_none() {
            self.remember_tool(&record)
                .map_err(|_| "Could not save tool descriptor".to_string())?;
        }
        Ok(record)
    }
    pub(crate) fn register_tool_descriptors(
        &self,
        descriptors: &[ToolDescriptor],
    ) -> Result<Vec<ToolCatalogRecord>, String> {
        self.register_labeled_tool_descriptors(descriptors, &[])
    }
    pub(crate) fn register_labeled_tool_descriptors(
        &self,
        descriptors: &[ToolDescriptor],
        tools: &[crate::domain::agent::EffectiveTool],
    ) -> Result<Vec<ToolCatalogRecord>, String> {
        let labels: BTreeMap<_, _> = tools
            .iter()
            .filter_map(|tool| {
                tool.domain_code
                    .as_deref()
                    .map(str::trim)
                    .filter(|code| !code.is_empty())
                    .map(|code| (tool.tool_name.as_str(), code))
            })
            .collect();
        let records = descriptors
            .iter()
            .map(|descriptor| {
                let mut record = self.describe_tool_descriptor(descriptor)?;
                if let Some(code) = labels.get(descriptor.call_name.as_str()) {
                    record.domain_code = (*code).into();
                }
                Ok(record)
            })
            .collect::<Result<Vec<_>, String>>()?;
        let discovered = records
            .iter()
            .filter(|r| fixed_tool(&r.tool_name).is_none())
            .cloned()
            .collect::<Vec<_>>();
        self.remember_tools(&discovered)
            .map_err(|_| "Could not save tool descriptors".to_string())?;
        Ok(records)
    }
}
impl ToolPermissionSource for AgentManagementService {
    fn setting(&self, descriptor: &ToolDescriptor) -> Result<ToolPermissionSetting, String> {
        let code = self.register_tool_descriptor(descriptor)?.tool_code;
        self.cached_tool_permission(&code)
            .map_err(|_| "Could not read platform tool permission".to_string())
    }
    fn operation(&self, descriptor: &ToolDescriptor) -> Option<bool> {
        let kind = fixed_tool(&descriptor.call_name).map(|r| r.operation_kind);
        match kind {
            Some(ToolOperationKind::Query) => Some(false),
            Some(ToolOperationKind::Command) => Some(true),
            _ => match descriptor.operation_kind {
                eliteforge_agent_core::ToolOperationKind::Query => Some(false),
                eliteforge_agent_core::ToolOperationKind::Command => Some(true),
                eliteforge_agent_core::ToolOperationKind::Unknown => None,
            },
        }
    }
}
