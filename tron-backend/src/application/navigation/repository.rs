use crate::domain::navigation::{
    MenuQuery, NavigationDomain, NavigationError, NavigationMenu, NavigationPage,
    NavigationTreeItem,
};

use super::{CopyMenuTreeCommand, DefaultNavigationMenuTreeItem, UpdateMenuItemCommand};

pub trait NavigationRepository: Send {
    fn next_id(&mut self) -> Result<i64, NavigationError>;
    fn list_domains(&mut self) -> Result<Vec<NavigationDomain>, NavigationError>;
    fn get_domain(&mut self, domain_code: &str) -> Result<NavigationDomain, NavigationError>;
    fn save_domain(&mut self, domain: &NavigationDomain) -> Result<(), NavigationError>;
    fn delete_domain(&mut self, domain_code: &str) -> Result<(), NavigationError>;
    fn list_menus(
        &mut self,
        query: &MenuQuery,
    ) -> Result<NavigationPage<NavigationMenu>, NavigationError>;
    fn get_menu(&mut self, menu_code: &str) -> Result<NavigationMenu, NavigationError>;
    fn save_menu(&mut self, menu: &NavigationMenu) -> Result<(), NavigationError>;
    fn delete_menu(&mut self, menu_code: &str) -> Result<(), NavigationError>;
    fn menu_usage_count(&mut self, menu_code: &str) -> Result<i64, NavigationError>;
    fn menu_has_children(&mut self, menu_code: &str) -> Result<bool, NavigationError>;
    fn list_items(&mut self, domain_code: &str)
    -> Result<Vec<NavigationTreeItem>, NavigationError>;
    fn add_item(
        &mut self,
        domain_code: &str,
        item: &NavigationTreeItem,
    ) -> Result<(), NavigationError>;
    fn update_item(
        &mut self,
        domain_code: &str,
        menu_code: &str,
        command: &UpdateMenuItemCommand,
    ) -> Result<(), NavigationError>;
    fn remove_item(&mut self, domain_code: &str, menu_code: &str) -> Result<(), NavigationError>;
    fn copy_tree(
        &mut self,
        target_domain_code: &str,
        command: &CopyMenuTreeCommand,
    ) -> Result<(), NavigationError>;
    fn restore_default_menu_trees(
        &mut self,
        items: &[DefaultNavigationMenuTreeItem],
    ) -> Result<i64, NavigationError>;
}
