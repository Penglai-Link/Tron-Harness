//! Bridge product navigation resources into package lifecycle operations.
use super::*;
use crate::application::navigation::packs::{NavigationContribution, RESOURCE_PATH};
use eliteforge_agent_capability_runtime::CapabilityPackManifest;

pub(crate) fn read_navigation(
    path: &Path,
    manifest: &CapabilityPackManifest,
) -> Result<NavigationContribution, String> {
    super::host_pages::read_pages(path, manifest)?;
    if !manifest.resources.iter().any(|r| r == RESOURCE_PATH) {
        return Ok(NavigationContribution {
            version: 1,
            ..Default::default()
        });
    }
    let root = path.canonicalize().map_err(|e| e.to_string())?;
    let file = path
        .join(RESOURCE_PATH)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if !file.starts_with(&root) {
        return Err("navigation resource escapes capability package".into());
    }
    use std::io::Read;
    let mut bytes = Vec::new();
    fs::File::open(file)
        .map_err(|e| e.to_string())?
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 1024 * 1024 {
        return Err("navigation resource exceeds 1 MiB".into());
    }
    let mut navigation: NavigationContribution = serde_json::from_slice(&bytes).map_err(|e| format!("invalid {RESOURCE_PATH}: {e}"))?;
    let pages = super::host_pages::read_pages(path, manifest)?;
    for menu in &mut navigation.menus {
        if menu.menu_type == "PackPage" {
            let id = menu.page_code.as_deref().ok_or("pageCode is required")?;
            if !pages.iter().any(|page| page.id == id) { return Err("unknown capability page".into()); }
            menu.page_code = Some(format!("{}/{}", manifest.id, id));
        }
    }
    Ok(navigation)
}
impl FrameworkHost {
    pub fn bind_navigation(
        &self,
        navigation: crate::application::navigation::NavigationService,
    ) -> Result<(), String> {
        self.0
            .navigation
            .set(navigation)
            .map_err(|_| "navigation already bound")?;
        self.sync_navigation_packs()
    }
    pub(super) fn sync_navigation_packs(&self) -> Result<(), String> {
        let Some(navigation) = self.0.navigation.get() else {
            return Ok(());
        };
        let manager =
            eliteforge_agent_capability_runtime::CapabilityPackManager::new(&self.0.pack_root)
                .map_err(|e| e.to_string())?;
        let mut selected = BTreeMap::new();
        for pack in manager.installed().map_err(|e| e.to_string())? {
            let slot = selected
                .entry(pack.manifest.id.clone())
                .or_insert_with(|| pack.clone());
            if pack.manifest.version > slot.manifest.version {
                *slot = pack;
            }
        }
        let (state, _) = self.capability_state();
        for (id, pack) in selected {
            let manifest = read_navigation(&pack.path, &pack.manifest)?;
            navigation.sync_pack(&id, &manifest, state.get(&id).copied().unwrap_or(false))?;
        }
        Ok(())
    }
}
