//! Profile-scoped read-through metadata. Writes invalidate under the same lock;
//! capability changes also invalidate because uninstall removes tool settings.
use super::*;
use tron_harness_capability::ToolPermissionSetting;

#[derive(Default)]
pub(super) struct ToolMetadataCache {
    revision: u64,
    permissions: Option<Vec<ToolPermissionSetting>>,
    discovered: Option<Vec<ToolCatalogRecord>>,
}
impl AgentManagementService {
    fn metadata_cache(&self) -> std::sync::MutexGuard<'_, ToolMetadataCache> {
        let mut cache = self.tool_metadata.lock().unwrap();
        let revision = self.capabilities.get().map_or(0, |c| c.revision());
        if cache.revision != revision {
            *cache = ToolMetadataCache {
                revision,
                ..Default::default()
            };
        }
        cache
    }
    pub(super) fn cached_tool_permissions(
        &self,
    ) -> Result<Vec<ToolPermissionSetting>, AgentManagementError> {
        let mut cache = self.metadata_cache();
        if cache.permissions.is_none() {
            cache.permissions = Some(self.repository().list_tool_permissions()?);
        }
        Ok(cache.permissions.as_ref().unwrap().clone())
    }
    pub(super) fn cached_discovered_tools(
        &self,
    ) -> Result<Vec<ToolCatalogRecord>, AgentManagementError> {
        let mut cache = self.metadata_cache();
        if cache.discovered.is_none() {
            cache.discovered = Some(self.repository().list_discovered_tools()?);
        }
        Ok(cache.discovered.as_ref().unwrap().clone())
    }
    pub(super) fn cached_tool_by_name(
        &self,
        name: &str,
    ) -> Result<Option<ToolCatalogRecord>, AgentManagementError> {
        let mut cache = self.metadata_cache();
        if cache.discovered.is_none() {
            cache.discovered = Some(self.repository().list_discovered_tools()?);
        }
        Ok(cache
            .discovered
            .as_ref()
            .unwrap()
            .iter()
            .find(|r| r.tool_name == name)
            .cloned())
    }
    pub(super) fn cached_tool_permission(
        &self,
        code: &str,
    ) -> Result<ToolPermissionSetting, AgentManagementError> {
        let mut cache = self.metadata_cache();
        if cache.permissions.is_none() {
            cache.permissions = Some(self.repository().list_tool_permissions()?);
        }
        Ok(cache
            .permissions
            .as_ref()
            .unwrap()
            .iter()
            .find(|s| s.tool_code == code)
            .cloned()
            .unwrap_or_else(|| ToolPermissionSetting::default_for(code)))
    }
    pub(super) fn save_tool_permissions(
        &self,
        settings: &[ToolPermissionSetting],
    ) -> Result<(), AgentManagementError> {
        let mut cache = self.metadata_cache();
        cache.permissions = None;
        self.repository().replace_tool_permissions(settings)?;
        cache.permissions = Some(settings.to_vec());
        Ok(())
    }
    pub(super) fn remember_tool(
        &self,
        tool: &ToolCatalogRecord,
    ) -> Result<(), AgentManagementError> {
        self.remember_tools(std::slice::from_ref(tool))
    }
    pub(crate) fn remember_tools(
        &self,
        tools: &[ToolCatalogRecord],
    ) -> Result<(), AgentManagementError> {
        let mut cache = self.metadata_cache();
        if cache.discovered.is_none() {
            cache.discovered = Some(self.repository().list_discovered_tools()?);
        }
        let records = cache.discovered.as_ref().unwrap();
        let changed = tools
            .iter()
            .filter(|tool| {
                !records.iter().any(|r| {
                    r.tool_code == tool.tool_code
                        && serde_json::to_value(r).ok() == serde_json::to_value(tool).ok()
                })
            })
            .cloned()
            .collect::<Vec<_>>();
        if changed.is_empty() {
            return Ok(());
        }
        let mut records = cache.discovered.take().unwrap();
        self.repository().remember_discovered_tools(&changed)?;
        for tool in changed {
            records.retain(|r| r.tool_code != tool.tool_code);
            records.push(tool);
        }
        records.sort_by(|a, b| a.tool_code.cmp(&b.tool_code));
        cache.discovered = Some(records);
        Ok(())
    }
}
