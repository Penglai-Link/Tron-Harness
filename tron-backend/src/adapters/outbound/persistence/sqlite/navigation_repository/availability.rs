//! Query visibility follows pack activation; stored layouts remain untouched.
use super::*;

pub(super) fn inactive_codes(
    connection: &Connection,
    kind: &str,
) -> Result<BTreeSet<String>, NavigationError> {
    let mut query = connection.prepare("SELECT o.code FROM navigation_pack_ownership o LEFT JOIN navigation_pack_state s ON s.pack_id=o.pack_id WHERE o.kind=?1 AND COALESCE(s.enabled,0)=0").map_err(storage_error)?;
    query
        .query_map([kind], |row| row.get(0))
        .map_err(storage_error)?
        .collect::<Result<_, _>>()
        .map_err(storage_error)
}
fn inactive_trees(connection: &Connection) -> Result<BTreeSet<String>, NavigationError> {
    let mut query = connection.prepare("SELECT t.tree_code FROM navigation_menu_trees t JOIN navigation_domains d ON d.id=t.domain_id JOIN navigation_pack_ownership o ON o.kind='Domain' AND o.code=d.domain_code LEFT JOIN navigation_pack_state s ON s.pack_id=o.pack_id WHERE COALESCE(s.enabled,0)=0").map_err(storage_error)?;
    query
        .query_map([], |row| row.get(0))
        .map_err(storage_error)?
        .collect::<Result<_, _>>()
        .map_err(storage_error)
}
pub(super) fn domains(
    connection: &mut Connection,
) -> Result<Vec<NavigationDomain>, NavigationError> {
    let inactive = inactive_codes(connection, "Domain")?;
    Ok(SqliteNavigationRepositorySql { connection }
        .list_domains()?
        .into_iter()
        .filter(|domain| !inactive.contains(&domain.domain_code))
        .collect())
}
pub(super) fn domain(
    connection: &mut Connection,
    code: &str,
) -> Result<NavigationDomain, NavigationError> {
    if inactive_codes(connection, "Domain")?.contains(code) {
        return Err(NavigationError::DomainNotFound);
    }
    SqliteNavigationRepositorySql { connection }.get_domain(code)
}
pub(super) fn trees(
    connection: &mut Connection,
) -> Result<Vec<NavigationMenuTree>, NavigationError> {
    let inactive = inactive_trees(connection)?;
    Ok(SqliteNavigationRepositorySql { connection }
        .list_menu_trees()?
        .into_iter()
        .filter(|tree| !inactive.contains(&tree.tree_code))
        .collect())
}
pub(super) fn tree(
    connection: &mut Connection,
    code: &str,
) -> Result<NavigationMenuTree, NavigationError> {
    if inactive_trees(connection)?.contains(code) {
        return Err(NavigationError::DomainNotFound);
    }
    SqliteNavigationRepositorySql { connection }.get_menu_tree(code)
}
pub(super) fn visible_items(
    connection: &Connection,
    mut items: Vec<NavigationTreeItem>,
) -> Result<Vec<NavigationTreeItem>, NavigationError> {
    let mut hidden = inactive_codes(connection, "Menu")?;
    // Children of an inactive group are hidden with it, without rewriting parents.
    loop {
        let count = hidden.len();
        for item in &items {
            if item
                .parent_menu_code
                .as_ref()
                .is_some_and(|parent| hidden.contains(parent))
            {
                hidden.insert(item.menu.menu_code.clone());
            }
        }
        if count == hidden.len() {
            break;
        }
    }
    items.retain(|item| !hidden.contains(&item.menu.menu_code));
    Ok(items)
}
pub(super) fn items(
    connection: &mut Connection,
    code: &str,
) -> Result<Vec<NavigationTreeItem>, NavigationError> {
    tree(connection, code)?;
    let items = SqliteNavigationRepositorySql { connection }.list_items(code)?;
    visible_items(connection, items)
}
