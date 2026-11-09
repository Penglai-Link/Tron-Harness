//! Atomic package registration and default-layout reconciliation.
use super::*;
use crate::application::navigation::packs::{NavigationContribution, PackItem};
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Serialize, Deserialize)]
struct LayoutState {
    baseline: Vec<PackItem>,
    customized: bool,
}
type Layouts = BTreeMap<String, LayoutState>;
fn fail(e: impl std::fmt::Display) -> String {
    e.to_string()
}

pub(super) fn atomic<T>(
    c: &mut Connection,
    dry_run: bool,
    f: impl FnOnce(&mut Connection) -> Result<T, String>,
) -> Result<T, String> {
    c.execute_batch("SAVEPOINT navigation_pack_mutation")
        .map_err(fail)?;
    let result = f(c);
    if dry_run || result.is_err() {
        c.execute_batch("ROLLBACK TO navigation_pack_mutation; RELEASE navigation_pack_mutation")
            .map_err(fail)?;
    } else {
        c.execute_batch("RELEASE navigation_pack_mutation")
            .map_err(fail)?;
    }
    result
}
fn owner(c: &Connection, kind: &str, code: &str) -> Result<Option<String>, String> {
    c.query_row(
        "SELECT pack_id FROM navigation_pack_ownership WHERE kind=?1 AND code=?2",
        params![kind, code],
        |r| r.get(0),
    )
    .optional()
    .map_err(fail)
}
fn own(c: &Connection, pack: &str, kind: &str, code: &str) -> Result<(), String> {
    c.execute("INSERT INTO navigation_pack_ownership(pack_id,kind,code) VALUES(?1,?2,?3) ON CONFLICT(kind,code) DO NOTHING",params![pack,kind,code]).map_err(fail)?;
    Ok(())
}
fn current_layout(c: &Connection, domain: &str) -> Result<Vec<PackItem>, String> {
    let mut q = c.prepare("SELECT m.menu_code,p.menu_code,i.position,i.child_display FROM navigation_menu_tree_items i JOIN navigation_menu_trees t ON t.id=i.tree_id JOIN navigation_menus m ON m.id=i.menu_id LEFT JOIN navigation_menu_tree_items pi ON pi.id=i.parent_item_id LEFT JOIN navigation_menus p ON p.id=pi.menu_id WHERE t.tree_code=?1 ORDER BY m.menu_code").map_err(fail)?;
    q.query_map([domain], |r| {
        Ok(PackItem {
            menu_code: r.get(0)?,
            parent_menu_code: r.get(1)?,
            position: r.get(2)?,
            child_display: r.get(3)?,
        })
    })
    .map_err(fail)?
    .collect::<Result<_, _>>()
    .map_err(fail)
}
fn canonical(mut items: Vec<PackItem>) -> Vec<PackItem> {
    items.sort_by(|a, b| a.menu_code.cmp(&b.menu_code));
    items
}
fn clear_tree(c: &Connection, domain: &str) -> Result<(), String> {
    // Free sibling slots before flattening a hierarchy with repeated positions.
    c.execute("UPDATE navigation_menu_tree_items SET position=id WHERE tree_id=(SELECT id FROM navigation_menu_trees WHERE tree_code=?1)",[domain]).map_err(fail)?;
    c.execute("UPDATE navigation_menu_tree_items SET parent_item_id=NULL WHERE tree_id=(SELECT id FROM navigation_menu_trees WHERE tree_code=?1)",[domain]).map_err(fail)?;
    c.execute("DELETE FROM navigation_menu_tree_items WHERE tree_id=(SELECT id FROM navigation_menu_trees WHERE tree_code=?1)",[domain]).map_err(fail)?;
    Ok(())
}
fn remove_domain(c: &Connection, domain: &str) -> Result<(), String> {
    clear_tree(c, domain)?;
    c.execute(
        "DELETE FROM navigation_domains WHERE domain_code=?1",
        [domain],
    )
    .map_err(fail)?;
    c.execute(
        "DELETE FROM navigation_pack_ownership WHERE kind='Domain' AND code=?1",
        [domain],
    )
    .map_err(fail)?;
    Ok(())
}
fn remove_menu(c: &Connection, menu: &str) -> Result<(), String> {
    // Preserve user/other-pack children when removing an owned group from any tree.
    let rows = {
        let mut q=c.prepare("SELECT i.id,i.parent_item_id FROM navigation_menu_tree_items i JOIN navigation_menus m ON m.id=i.menu_id WHERE m.menu_code=?1").map_err(fail)?;
        q.query_map([menu], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, Option<i64>>(1)?))
        })
        .map_err(fail)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(fail)?
    };
    for (id, parent) in rows {
        let children = {
            let mut query = c.prepare("SELECT id FROM navigation_menu_tree_items WHERE parent_item_id=?1 ORDER BY position,id").map_err(fail)?;
            query
                .query_map([id], |row| row.get::<_, i64>(0))
                .map_err(fail)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(fail)?
        };
        let next: i64 = c.query_row("SELECT COALESCE(MAX(position),-1)+1 FROM navigation_menu_tree_items WHERE tree_id=(SELECT tree_id FROM navigation_menu_tree_items WHERE id=?1) AND parent_item_id IS ?2",params![id,parent],|row|row.get(0)).map_err(fail)?;
        for (offset, child) in children.into_iter().enumerate() {
            c.execute(
                "UPDATE navigation_menu_tree_items SET parent_item_id=?2,position=?3 WHERE id=?1",
                params![child, parent, next + offset as i64],
            )
            .map_err(fail)?;
        }
        c.execute("DELETE FROM navigation_menu_tree_items WHERE id=?1", [id])
            .map_err(fail)?;
    }
    c.execute("DELETE FROM navigation_menus WHERE menu_code=?1", [menu])
        .map_err(fail)?;
    c.execute(
        "DELETE FROM navigation_pack_ownership WHERE kind='Menu' AND code=?1",
        [menu],
    )
    .map_err(fail)?;
    Ok(())
}
fn set_enabled(c: &Connection, pack: &str, enabled: bool) -> Result<(), String> {
    c.execute(
        "UPDATE navigation_pack_state SET enabled=?2 WHERE pack_id=?1",
        params![pack, enabled],
    )
    .map_err(fail)?;
    c.execute("UPDATE navigation_domains SET enabled=?2 WHERE domain_code IN (SELECT code FROM navigation_pack_ownership WHERE pack_id=?1 AND kind='Domain')",params![pack,enabled]).map_err(fail)?;
    c.execute("UPDATE navigation_menu_trees SET enabled=?2 WHERE tree_code IN (SELECT code FROM navigation_pack_ownership WHERE pack_id=?1 AND kind='Domain')",params![pack,enabled]).map_err(fail)?;
    c.execute("UPDATE navigation_menus SET enabled=?2 WHERE menu_code IN (SELECT code FROM navigation_pack_ownership WHERE pack_id=?1 AND kind='Menu')",params![pack,enabled]).map_err(fail)?;
    Ok(())
}
fn apply(
    c: &mut Connection,
    pack: &str,
    manifest: &NavigationContribution,
    enabled: bool,
    max_depth: usize,
) -> Result<(), String> {
    let previous: Option<(String, String)> = c
        .query_row(
            "SELECT manifest_json,layouts_json FROM navigation_pack_state WHERE pack_id=?1",
            [pack],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(fail)?;
    let (old, mut layouts): (NavigationContribution, Layouts) = match previous {
        Some((manifest, layouts)) => (
            serde_json::from_str(&manifest).map_err(fail)?,
            serde_json::from_str(&layouts).map_err(fail)?,
        ),
        None => (NavigationContribution::default(), Layouts::new()),
    };
    // Reject collisions before touching an existing domain or menu.
    for (kind, code, table, column) in manifest
        .domains
        .iter()
        .map(|d| {
            (
                "Domain",
                d.code.as_str(),
                "navigation_domains",
                "domain_code",
            )
        })
        .chain(
            manifest
                .menus
                .iter()
                .map(|m| ("Menu", m.code.as_str(), "navigation_menus", "menu_code")),
        )
    {
        let exists: bool = c
            .query_row(
                &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE {column}=?1)"),
                [code],
                |r| r.get(0),
            )
            .map_err(fail)?;
        let owned = owner(c, kind, code)?;
        if owned.as_deref().is_some_and(|id| id != pack)
            || (exists && owned.as_deref() != Some(pack))
        {
            return Err(format!(
                "{kind} {code} conflicts with an existing navigation resource"
            ));
        }
    }
    // Snapshot before removed contributions change a user's layout.
    for domain in &old.domains {
        let state = layouts.entry(domain.code.clone()).or_default();
        state.customized |= current_layout(c, &domain.code)? != canonical(state.baseline.clone());
    }
    c.execute("INSERT INTO navigation_pack_state VALUES(?1,?2,?3,?4) ON CONFLICT(pack_id) DO UPDATE SET enabled=excluded.enabled", params![pack,enabled,serde_json::to_string(manifest).map_err(fail)?,"{}"]).map_err(fail)?;
    for domain in &old.domains {
        if !manifest.domains.iter().any(|d| d.code == domain.code) {
            remove_domain(c, &domain.code)?;
            layouts.remove(&domain.code);
        }
    }
    for menu in &old.menus {
        if !manifest.menus.iter().any(|m| m.code == menu.code) {
            remove_menu(c, &menu.code)?;
        }
    }
    // Untouched defaults can be rebuilt before changing a group's menu type.
    // Customized layouts remain intact, and incompatible type changes are rejected.
    for domain in &manifest.domains {
        if !layouts
            .get(&domain.code)
            .is_some_and(|state| state.customized)
        {
            clear_tree(c, &domain.code)?;
        }
    }
    let now = crate::shared::time::current_timestamp();
    for domain in &manifest.domains {
        let mut repo = SqliteNavigationRepositorySql { connection: c };
        let existing = match repo.get_domain(&domain.code) {
            Ok(d) => Some(d),
            Err(NavigationError::DomainNotFound) => None,
            Err(e) => return Err(fail(e)),
        };
        let position = existing
            .as_ref()
            .map(|d| d.position)
            .unwrap_or(repo.list_domains().map_err(fail)?.len() as i64);
        let record = NavigationDomain {
            id: existing
                .as_ref()
                .map(|d| d.id)
                .unwrap_or(SnowflakeId::new(repo.next_id().map_err(fail)?)),
            domain_code: domain.code.clone(),
            provider_pack_id: Some(pack.into()),
            name: domain.name.localized(),
            description: domain.description.localized(),
            built_in: false,
            enabled,
            is_default: existing.as_ref().is_some_and(|d| d.is_default),
            position,
            created_at: existing.map(|d| d.created_at).unwrap_or(now.clone()),
            updated_at: now.clone(),
        };
        repo.save_domain(&record).map_err(fail)?;
        own(c, pack, "Domain", &domain.code)?;
    }
    for menu in &manifest.menus {
        let mut repo = SqliteNavigationRepositorySql { connection: c };
        let existing = match repo.get_menu(&menu.code) {
            Ok(m) => Some(m),
            Err(NavigationError::MenuNotFound) => None,
            Err(e) => return Err(fail(e)),
        };
        let kind = MenuType::from_storage(&menu.menu_type).map_err(fail)?;
        if kind != MenuType::Group && repo.menu_has_children(&menu.code).map_err(fail)? {
            return Err(format!(
                "menu {} still has children; cannot change its type",
                menu.code
            ));
        }
        let record = NavigationMenu {
            id: existing
                .as_ref()
                .map(|m| m.id)
                .unwrap_or(SnowflakeId::new(repo.next_id().map_err(fail)?)),
            menu_code: menu.code.clone(),
            source: MenuSource::CapabilityPack,
            menu_type: kind,
            translations: menu.name.localized(),
            description: menu.description.localized(),
            page_code: menu.page_code.clone(),
            url: menu.url.clone(),
            icon: menu.icon.clone(),
            enabled,
            created_at: existing.map(|m| m.created_at).unwrap_or(now.clone()),
            updated_at: now.clone(),
        };
        repo.save_menu(&record).map_err(fail)?;
        own(c, pack, "Menu", &menu.code)?;
    }
    for domain in &manifest.domains {
        let state = layouts.entry(domain.code.clone()).or_default();
        if !state.customized {
            let desired = manifest
                .layouts
                .get(&domain.code)
                .cloned()
                .unwrap_or_default();
            clear_tree(c, &domain.code)?;
            let mut pending = desired.clone();
            pending.sort_by(|a, b| {
                a.position
                    .cmp(&b.position)
                    .then_with(|| a.menu_code.cmp(&b.menu_code))
            });
            let mut inserted = BTreeSet::new();
            while !pending.is_empty() {
                let index = pending
                    .iter()
                    .position(|i| {
                        i.parent_menu_code
                            .as_ref()
                            .is_none_or(|p| inserted.contains(p))
                    })
                    .ok_or("cyclic menu layout")?;
                let item = pending.remove(index);
                let mut repo = SqliteNavigationRepositorySql { connection: c };
                let record = NavigationTreeItem {
                    id: SnowflakeId::new(repo.next_id().map_err(fail)?),
                    menu: repo.get_menu(&item.menu_code).map_err(fail)?,
                    parent_menu_code: item.parent_menu_code,
                    position: item.position,
                    child_display: NavigationMenuChildDisplay::from_storage(&item.child_display)
                        .map_err(fail)?,
                    children: vec![],
                };
                repo.add_item(&domain.code, &record).map_err(fail)?;
                inserted.insert(item.menu_code);
            }
            // Store the actual normalized positions, not arbitrary author positions.
            state.baseline = current_layout(c, &domain.code)?;
        }
        let items = SqliteNavigationRepositorySql { connection: c }
            .list_items(&domain.code)
            .map_err(fail)?;
        crate::application::navigation::validate_flat_tree(&items, max_depth).map_err(fail)?;
    }
    c.execute(
        "UPDATE navigation_pack_state SET manifest_json=?2,layouts_json=?3 WHERE pack_id=?1",
        params![
            pack,
            serde_json::to_string(manifest).map_err(fail)?,
            serde_json::to_string(&layouts).map_err(fail)?
        ],
    )
    .map_err(fail)?;
    set_enabled(c, pack, enabled)
}

impl SqliteNavigationRepository {
    pub(super) fn sync_navigation_pack(
        &mut self,
        pack: &str,
        manifest: &NavigationContribution,
        enabled: bool,
        max_depth: usize,
        dry_run: bool,
    ) -> Result<(), String> {
        let pack = pack.to_owned();
        let manifest = manifest.clone();
        self.connection.call(
            move |c| {
                atomic(c, dry_run, |c| {
                    apply(c, &pack, &manifest, enabled, max_depth)
                })
            },
            || "navigation storage unavailable".into(),
        )
    }
    pub(super) fn toggle_navigation_pack(
        &mut self,
        pack: &str,
        enabled: bool,
    ) -> Result<(), String> {
        let pack = pack.to_owned();
        self.connection.call(
            move |c| atomic(c, false, |c| set_enabled(c, &pack, enabled)),
            || "navigation storage unavailable".into(),
        )
    }
    pub(super) fn remove_navigation_pack(&mut self, pack: &str) -> Result<(), String> {
        let pack = pack.to_owned();
        self.connection.call(move |c| atomic(c,false,|c|{
            let owned={let mut q=c.prepare("SELECT kind,code FROM navigation_pack_ownership WHERE pack_id=?1 ORDER BY kind,code").map_err(fail)?;q.query_map([&pack],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).map_err(fail)?.collect::<Result<Vec<_>,_>>().map_err(fail)?};
            for (kind,code) in owned { if kind=="Domain" {remove_domain(c,&code)?;} else {remove_menu(c,&code)?;} }
            c.execute("DELETE FROM navigation_pack_state WHERE pack_id=?1",[&pack]).map_err(fail)?;Ok(())
        }),||"navigation storage unavailable".into())
    }
}
