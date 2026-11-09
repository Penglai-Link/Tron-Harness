//! Product-specific navigation contributions shipped as a capability resource.
use super::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const RESOURCE_PATH: &str = "tron/navigation.json";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NavigationContribution {
    pub version: u32,
    #[serde(default)]
    pub domains: Vec<PackDomain>,
    #[serde(default)]
    pub menus: Vec<PackMenu>,
    #[serde(default)]
    pub layouts: BTreeMap<String, Vec<PackItem>>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackDomain {
    pub code: String,
    pub name: PackText,
    #[serde(default)]
    pub description: PackText,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackMenu {
    pub code: String,
    pub name: PackText,
    #[serde(default)]
    pub description: PackText,
    pub menu_type: String,
    pub page_code: Option<String>,
    pub url: Option<String>,
    pub icon: Option<String>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackText {
    #[serde(rename = "zh-CN")]
    pub zh_cn: String,
    #[serde(rename = "en-US")]
    pub en_us: String,
}
impl PackText {
    pub fn localized(&self) -> LocalizedText {
        LocalizedText {
            zh_cn: self.zh_cn.clone(),
            en_us: self.en_us.clone(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackItem {
    pub menu_code: String,
    pub parent_menu_code: Option<String>,
    pub position: i64,
    #[serde(default = "inline")]
    pub child_display: String,
}
fn inline() -> String {
    "Inline".into()
}

impl NavigationContribution {
    pub fn validate(&self, max_depth: usize) -> Result<(), String> {
        if self.version != 1 {
            return Err("unsupported navigation contribution version".into());
        }
        if self.domains.len() > 100 || self.menus.len() > 1000 {
            return Err("too many navigation contributions".into());
        }
        let mut domains = BTreeSet::new();
        let mut menus = BTreeMap::new();
        for domain in &self.domains {
            if crate::domain::navigation::validate_domain_code(&domain.code)
                .map_err(|e| e.to_string())?
                != domain.code
                || !domains.insert(&domain.code)
            {
                return Err(format!(
                    "duplicate or noncanonical domain code: {}",
                    domain.code
                ));
            }
            validate_translations(&domain.name.localized()).map_err(|e| e.to_string())?;
        }
        for menu in &self.menus {
            if validate_menu_code(&menu.code).map_err(|e| e.to_string())? != menu.code
                || menus.insert(menu.code.clone(), menu).is_some()
            {
                return Err(format!(
                    "duplicate or noncanonical menu code: {}",
                    menu.code
                ));
            }
            validate_translations(&menu.name.localized()).map_err(|e| e.to_string())?;
            let kind = MenuType::from_storage(&menu.menu_type).map_err(|e| e.to_string())?;
            validate_menu_target(kind, menu.page_code.as_deref(), menu.url.as_deref())
                .map_err(|e| e.to_string())?;
        }
        for (domain, layout) in &self.layouts {
            if !domains.contains(domain) {
                return Err(format!(
                    "layout domain {domain} must be declared by this pack"
                ));
            }
            let mut items = Vec::new();
            for item in layout {
                let menu = menus
                    .get(&item.menu_code)
                    .ok_or_else(|| format!("unknown menu {}", item.menu_code))?;
                let kind = MenuType::from_storage(&menu.menu_type).map_err(|e| e.to_string())?;
                let display = NavigationMenuChildDisplay::from_storage(&item.child_display)
                    .map_err(|e| e.to_string())?;
                if display == NavigationMenuChildDisplay::Flyout
                    && !matches!(kind, MenuType::Group | MenuType::Widget)
                {
                    return Err("flyout requires a group or widget".into());
                }
                items.push(NavigationTreeItem {
                    id: crate::shared::id::SnowflakeId::new(1),
                    menu: NavigationMenu {
                        id: crate::shared::id::SnowflakeId::new(1),
                        menu_code: menu.code.clone(),
                        source: MenuSource::CapabilityPack,
                        menu_type: kind,
                        translations: menu.name.localized(),
                        description: menu.description.localized(),
                        page_code: menu.page_code.clone(),
                        url: menu.url.clone(),
                        icon: menu.icon.clone(),
                        enabled: true,
                        created_at: String::new(),
                        updated_at: String::new(),
                    },
                    parent_menu_code: item.parent_menu_code.clone(),
                    position: item.position,
                    child_display: display,
                    children: vec![],
                });
            }
            validate_flat_tree(&items, max_depth).map_err(|e| e.to_string())?;
            let mut positions = BTreeSet::new();
            if items
                .iter()
                .any(|i| !positions.insert((i.parent_menu_code.clone(), i.position)))
            {
                return Err("duplicate sibling position".into());
            }
        }
        Ok(())
    }
}

impl NavigationService {
    pub fn sync_pack(
        &self,
        id: &str,
        manifest: &NavigationContribution,
        enabled: bool,
    ) -> Result<(), String> {
        manifest.validate(self.max_depth())?;
        self.lock_repository()
            .sync_pack(id, manifest, enabled, self.max_depth(), false)
    }
    pub fn validate_pack(&self, id: &str, manifest: &NavigationContribution) -> Result<(), String> {
        manifest.validate(self.max_depth())?;
        self.lock_repository()
            .sync_pack(id, manifest, false, self.max_depth(), true)
    }
    pub fn set_pack_enabled(&self, id: &str, enabled: bool) -> Result<(), String> {
        self.lock_repository().set_pack_enabled(id, enabled)
    }
    pub fn remove_pack(&self, id: &str) -> Result<(), String> {
        self.lock_repository().remove_pack(id)
    }
}
