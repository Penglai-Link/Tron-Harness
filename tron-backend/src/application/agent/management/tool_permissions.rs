use super::*;
use crate::domain::agent::{ToolAvailabilityState, ToolCatalogIssue, ToolOperationKind};
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
    pub(super) fn validate_selected_categories(
        &self,
        categories: &Option<Vec<String>>,
    ) -> Result<(), AgentManagementError> {
        if let Some(categories) = categories {
            let known = self
                .list_tools(None, None, "en-US")
                .into_iter()
                .filter_map(|tool| tool.category.map(|category| category.code))
                .collect::<HashSet<_>>();
            if categories
                .iter()
                .any(|code| code != "readonly" && !known.contains(code))
            {
                return Err(AgentManagementError::InternalAgentPolicyInvalid);
            }
        }
        Ok(())
    }
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
        if let Some(mut record) = fixed {
            record.display = descriptor.display.clone();
            return Ok(record);
        }
        if super::super::tool_catalog::is_fixed_code(&descriptor.id.to_ascii_lowercase()) {
            return Err("Native descriptor conflicts with a platform tool code".into());
        }
        let cached = self
            .cached_tool_by_name(&descriptor.call_name)
            .map_err(|_| "Could not read tool catalog".to_string())?;
        // Only a real provider lease can register this native descriptor.
        let record = ToolCatalogRecord {
            display: descriptor.display.clone(),
            tool_code: descriptor.id.clone(),
            tool_name: descriptor.call_name.clone(),
            domain_code: cached
                .as_ref()
                .map(|record| record.domain_code.clone())
                .unwrap_or_else(|| "native".into()),
            category: if eliteforge_agent_core::is_gateway_descriptor(descriptor) {
                Some(crate::application::agent::tool_catalog::builtin_category(
                    "execution",
                ))
            } else {
                cached.as_ref().and_then(|record| record.category.clone())
            },
            source: cached
                .as_ref()
                .map(|record| record.source.clone())
                .unwrap_or_default(),
            name: descriptor.call_name.clone(),
            description: descriptor.description.clone(),
            operation_kind: match descriptor.operation_kind {
                eliteforge_agent_core::ToolOperationKind::Query => ToolOperationKind::Query,
                eliteforge_agent_core::ToolOperationKind::Command => ToolOperationKind::Command,
                eliteforge_agent_core::ToolOperationKind::Unknown => ToolOperationKind::Unknown,
            },
            input_schema: descriptor.input_schema.clone(),
            module_owner: String::new(),
            availability_state: ToolAvailabilityState::Available,
            sample_result_summary: String::new(),
        };
        Ok(record)
    }
    pub(crate) fn describe_tool_binding(
        &self,
        binding: &eliteforge_agent_core::ToolBinding,
    ) -> Result<ToolCatalogRecord, String> {
        let mut record = self.describe_tool_descriptor(&binding.descriptor)?;
        if let Some(domain) = &binding.domain_code {
            record.domain_code = domain.clone();
        }
        if binding.category.is_some() {
            record.category = binding.category.clone();
        }
        if binding.source.provider_id.is_some() {
            record.source.provider_id = binding.source.provider_id.clone();
        }
        if binding.source.plugin_id.is_some() {
            record.source.plugin_id = binding.source.plugin_id.clone();
        }
        if binding.source.pack_id.is_some() {
            record.source.pack_id = binding.source.pack_id.clone();
        }
        if binding.source.adapter_id.is_some() {
            record.source.adapter_id = binding.source.adapter_id.clone();
        }
        Ok(record)
    }

    pub(crate) fn register_tool_bindings(
        &self,
        bindings: &[eliteforge_agent_core::ToolBinding],
    ) -> Result<(), String> {
        let records = bindings
            .iter()
            .map(|binding| self.describe_tool_binding(binding))
            .collect::<Result<Vec<_>, _>>()?;
        self.register_catalog_records(&records)
    }

    pub(crate) fn register_tool_catalog_entries(
        &self,
        entries: &[(
            eliteforge_agent_core::ToolCatalogEntry,
            eliteforge_agent_core::ToolSource,
        )],
    ) -> Result<(Vec<ToolCatalogRecord>, Vec<ToolCatalogIssue>), String> {
        let mut records = Vec::new();
        let mut issues = Vec::new();
        for (entry, source) in entries {
            // Resolve existing call names to their stable platform toolCode so
            // refreshed plugin metadata preserves permissions and assignments.
            let mut record = match self.describe_tool_descriptor(&entry.descriptor) {
                Ok(record) => record,
                Err(message) => {
                    issues.push(ToolCatalogIssue {
                        blocks_tool: true,
                        tool_code: Some(entry.descriptor.id.clone()),
                        tool_name: Some(entry.descriptor.call_name.clone()),
                        provider_id: source.provider_id.clone(),
                        pack_id: source.pack_id.clone(),
                        message,
                    });
                    continue;
                }
            };
            if let Some(domain) = &entry.domain_code {
                record.domain_code = domain.clone();
            }
            if entry.category.is_some() {
                record.category = entry.category.clone();
            }
            let declared_provider =
                fixed_tool(&entry.descriptor.call_name).and_then(|tool| tool.source.provider_id);
            record.source = source.clone();
            if let Some(provider) = declared_provider {
                record.source.provider_id = Some(provider);
            }
            records.push(record);
        }
        let errors = self.catalog_validation_errors(&records)?;
        let mut valid = Vec::new();
        for (record, error) in records.into_iter().zip(errors) {
            if let Some(message) = error {
                issues.push(ToolCatalogIssue {
                    blocks_tool: true,
                    tool_code: Some(record.tool_code),
                    tool_name: Some(record.tool_name),
                    provider_id: record.source.provider_id,
                    pack_id: record.source.pack_id,
                    message,
                });
            } else {
                valid.push(record);
            }
        }
        self.remember_tools(&valid)
            .map_err(|_| "Could not save tool bindings".to_string())?;
        Ok((valid, issues))
    }

    fn register_catalog_records(&self, records: &[ToolCatalogRecord]) -> Result<(), String> {
        if let Some(error) = self
            .catalog_validation_errors(records)?
            .into_iter()
            .flatten()
            .next()
        {
            return Err(error);
        }
        self.remember_tools(records)
            .map_err(|_| "Could not save tool bindings".to_string())
    }

    fn catalog_validation_errors(
        &self,
        records: &[ToolCatalogRecord],
    ) -> Result<Vec<Option<String>>, String> {
        // A provider may reuse an existing category. A new plugin category must
        // be under that plugin or capability pack's stable namespace.
        let mut known: HashSet<String> = [
            "tron.platform",
            "tron.assistant",
            "system.command",
            "document.media",
            "network",
            "agent.collaboration",
            "interaction",
            "execution",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        known.extend(
            self.cached_discovered_tools()
                .map_err(|_| "Could not read tool catalog".to_string())?
                .into_iter()
                .filter_map(|tool| tool.category.map(|category| category.code)),
        );
        let mut errors = Vec::with_capacity(records.len());
        for record in records {
            let Some(category) = &record.category else {
                errors.push(Some(format!(
                    "Tool {} must declare a category",
                    record.tool_code
                )));
                continue;
            };
            if category.code.is_empty()
                || category.code.len() > 128
                || ["all", "readonly", "uncategorized"].contains(&category.code.as_str())
                || !category
                    .code
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
            {
                errors.push(Some("Invalid tool category code".into()));
                continue;
            }
            errors.push(None);
            // A package may use its own id as the category root. Accept that
            // equal name as its namespace; child categories keep the dotted
            // prefix rule.
            if [
                &record.source.plugin_id,
                &record.source.pack_id,
                &record.source.provider_id,
            ]
            .into_iter()
            .flatten()
            .any(|owner| {
                category.code == owner.as_str() || category.code.starts_with(&format!("{owner}."))
            }) {
                known.insert(category.code.clone());
            }
        }
        for (record, error) in records.iter().zip(&mut errors) {
            if error.is_some() {
                continue;
            }
            if let Some(category) = &record.category {
                if !known.contains(&category.code)
                    && (record.source.plugin_id.is_some() || record.source.pack_id.is_some())
                {
                    *error = Some(format!(
                        "Tool category {} must use its provider namespace",
                        category.code
                    ));
                }
            }
        }
        Ok(errors)
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
