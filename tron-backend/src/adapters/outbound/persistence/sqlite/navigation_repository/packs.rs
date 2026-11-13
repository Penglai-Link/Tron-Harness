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
fn own(c: &Connection, pack: &str, kind: &str, code: &str) -> Result<(), String> {
    c.execute("INSERT INTO navigation_pack_ownership(pack_id,kind,code) VALUES(?1,?2,?3) ON CONFLICT(kind,code) DO NOTHING",params![pack,kind,code]).map_err(fail)?;
    Ok(())
}
fn current_layouts(
    c: &Connection,
    domains: &[String],
) -> Result<BTreeMap<String, Vec<PackItem>>, String> {
    let selected = serde_json::to_string(domains).map_err(fail)?;
    let mut q = c.prepare("SELECT t.tree_code,m.menu_code,p.menu_code,i.position,i.child_display FROM navigation_menu_tree_items i JOIN navigation_menu_trees t ON t.id=i.tree_id JOIN navigation_menus m ON m.id=i.menu_id LEFT JOIN navigation_menu_tree_items pi ON pi.id=i.parent_item_id LEFT JOIN navigation_menus p ON p.id=pi.menu_id WHERE t.tree_code IN (SELECT value FROM json_each(?1)) ORDER BY t.tree_code,m.menu_code").map_err(fail)?;
    let mut layouts = BTreeMap::<String, Vec<PackItem>>::new();
    for row in q
        .query_map([selected], |r| {
            Ok((
                r.get::<_, String>(0)?,
                PackItem {
                    menu_code: r.get(1)?,
                    parent_menu_code: r.get(2)?,
                    position: r.get(3)?,
                    child_display: r.get(4)?,
                },
            ))
        })
        .map_err(fail)?
    {
        let (domain, item) = row.map_err(fail)?;
        layouts.entry(domain).or_default().push(item);
    }
    Ok(layouts)
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
fn remove_menus(c: &Connection, menus: &[String]) -> Result<(), String> {
    if menus.is_empty() {
        return Ok(());
    }
    let selected = serde_json::to_string(menus).map_err(fail)?;
    // Keep all affected trees in memory so successive removals observe earlier
    // child promotions without querying each mount point again.
    let mut items = {
        let mut query=c.prepare("SELECT i.id,i.tree_id,m.menu_code,i.parent_item_id,i.position FROM navigation_menu_tree_items i JOIN navigation_menus m ON m.id=i.menu_id WHERE i.tree_id IN (SELECT x.tree_id FROM navigation_menu_tree_items x JOIN navigation_menus n ON n.id=x.menu_id WHERE n.menu_code IN (SELECT value FROM json_each(?1))) ORDER BY i.id").map_err(fail)?;
        query
            .query_map([&selected], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    (
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<i64>>(3)?,
                        row.get::<_, i64>(4)?,
                    ),
                ))
            })
            .map_err(fail)?
            .collect::<Result<BTreeMap<_, _>, _>>()
            .map_err(fail)?
    };
    for menu in menus {
        let mounts = items
            .iter()
            .filter(|(_, (_, code, _, _))| code == menu)
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();
        for id in mounts {
            let (tree, _, parent, _) = items.get(&id).cloned().ok_or("menu mount disappeared")?;
            let mut children = items
                .iter()
                .filter(|(_, (_, _, p, _))| *p == Some(id))
                .map(|(id, (_, _, _, position))| (*position, *id))
                .collect::<Vec<_>>();
            children.sort();
            let next = items
                .values()
                .filter(|(t, _, p, _)| *t == tree && *p == parent)
                .map(|(_, _, _, position)| *position)
                .max()
                .unwrap_or(-1)
                + 1;
            for (offset, (_, child)) in children.into_iter().enumerate() {
                let position = next + offset as i64;
                c.execute("UPDATE navigation_menu_tree_items SET parent_item_id=?2,position=?3 WHERE id=?1",params![child,parent,position]).map_err(fail)?;
                if let Some(item) = items.get_mut(&child) {
                    item.2 = parent;
                    item.3 = position;
                }
            }
            c.execute("DELETE FROM navigation_menu_tree_items WHERE id=?1", [id])
                .map_err(fail)?;
            items.remove(&id);
        }
    }
    c.execute(
        "DELETE FROM navigation_menus WHERE menu_code IN (SELECT value FROM json_each(?1))",
        [&selected],
    )
    .map_err(fail)?;
    c.execute("DELETE FROM navigation_pack_ownership WHERE kind='Menu' AND code IN (SELECT value FROM json_each(?1))",[&selected]).map_err(fail)?;
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
    // Reuse one transaction-local registry snapshot for collision checks and updates.
    let mut domains = SqliteNavigationRepositorySql { connection: c }
        .all_domains()
        .map_err(fail)?
        .into_iter()
        .map(|record| (record.domain_code.clone(), record))
        .collect::<BTreeMap<_, _>>();
    let mut menus = SqliteNavigationRepositorySql { connection: c }
        .all_menus()
        .map_err(fail)?
        .into_iter()
        .map(|record| (record.menu_code.clone(), record))
        .collect::<BTreeMap<_, _>>();
    let ownership = {
        let mut query = c
            .prepare("SELECT kind,code,pack_id FROM navigation_pack_ownership")
            .map_err(fail)?;
        query
            .query_map([], |row| {
                Ok((
                    (row.get::<_, String>(0)?, row.get::<_, String>(1)?),
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(fail)?
            .collect::<Result<BTreeMap<_, _>, _>>()
            .map_err(fail)?
    };
    for (kind, code, exists) in manifest
        .domains
        .iter()
        .map(|d| ("Domain", d.code.as_str(), domains.contains_key(&d.code)))
        .chain(
            manifest
                .menus
                .iter()
                .map(|m| ("Menu", m.code.as_str(), menus.contains_key(&m.code))),
        )
    {
        let owned = ownership
            .get(&(kind.to_owned(), code.to_owned()))
            .map(String::as_str);
        if owned.is_some_and(|id| id != pack) || (exists && owned != Some(pack)) {
            return Err(format!(
                "{kind} {code} conflicts with an existing navigation resource"
            ));
        }
    }
    // Snapshot before removed contributions change a user's layout.
    let mut current = current_layouts(
        c,
        &old.domains
            .iter()
            .map(|domain| domain.code.clone())
            .collect::<Vec<_>>(),
    )?;
    for domain in &old.domains {
        let state = layouts.entry(domain.code.clone()).or_default();
        state.customized |=
            current.remove(&domain.code).unwrap_or_default() != canonical(state.baseline.clone());
    }
    c.execute("INSERT INTO navigation_pack_state VALUES(?1,?2,?3,?4) ON CONFLICT(pack_id) DO UPDATE SET enabled=excluded.enabled", params![pack,enabled,serde_json::to_string(manifest).map_err(fail)?,"{}"]).map_err(fail)?;
    for domain in &old.domains {
        if !manifest.domains.iter().any(|d| d.code == domain.code) {
            remove_domain(c, &domain.code)?;
            domains.remove(&domain.code);
            layouts.remove(&domain.code);
        }
    }
    let removed = old
        .menus
        .iter()
        .filter(|menu| !manifest.menus.iter().any(|m| m.code == menu.code))
        .map(|menu| menu.code.clone())
        .collect::<Vec<_>>();
    remove_menus(c, &removed)?;
    for code in removed {
        menus.remove(&code);
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
        let existing = domains.get(&domain.code).cloned();
        let position = existing
            .as_ref()
            .map(|d| d.position)
            .unwrap_or(domains.len() as i64);
        let record = NavigationDomain {
            id: match &existing {
                Some(domain) => domain.id,
                None => SnowflakeId::new(repo.next_id().map_err(fail)?),
            },
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
        domains.insert(domain.code.clone(), record);
        own(c, pack, "Domain", &domain.code)?;
    }
    let parents = {
        let mut query = c.prepare("SELECT DISTINCT m.menu_code FROM navigation_menu_tree_items p JOIN navigation_menus m ON m.id=p.menu_id JOIN navigation_menu_tree_items child ON child.parent_item_id=p.id").map_err(fail)?;
        query
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(fail)?
            .collect::<Result<BTreeSet<_>, _>>()
            .map_err(fail)?
    };
    for menu in &manifest.menus {
        let mut repo = SqliteNavigationRepositorySql { connection: c };
        let existing = menus.get(&menu.code).cloned();
        let kind = MenuType::from_storage(&menu.menu_type).map_err(fail)?;
        if kind != MenuType::Group && parents.contains(&menu.code) {
            return Err(format!(
                "menu {} still has children; cannot change its type",
                menu.code
            ));
        }
        let record = NavigationMenu {
            id: match &existing {
                Some(menu) => menu.id,
                None => SnowflakeId::new(repo.next_id().map_err(fail)?),
            },
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
        menus.insert(menu.code.clone(), record);
        own(c, pack, "Menu", &menu.code)?;
    }
    let trees = {
        let mut query = c
            .prepare("SELECT tree_code,id FROM navigation_menu_trees")
            .map_err(fail)?;
        query
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .map_err(fail)?
            .collect::<Result<BTreeMap<_, _>, _>>()
            .map_err(fail)?
    };
    let mut next_id = SqliteNavigationRepositorySql { connection: c }
        .next_id()
        .map_err(fail)?;
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
            let mut records = Vec::<NavigationTreeItem>::new();
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
                let id = next_id;
                next_id += 1;
                let parent_id = records
                    .iter()
                    .find(|record| Some(&record.menu.menu_code) == item.parent_menu_code.as_ref())
                    .map(|record| record.id.value());
                let assignments = ordered_assignments_for_insert(
                    &records,
                    id,
                    item.parent_menu_code.as_deref(),
                    item.position,
                    parent_id,
                );
                let mut record = NavigationTreeItem {
                    id: SnowflakeId::new(id),
                    menu: menus
                        .get(&item.menu_code)
                        .cloned()
                        .ok_or("menu not found")?,
                    parent_menu_code: item.parent_menu_code,
                    position: item.position,
                    child_display: NavigationMenuChildDisplay::from_storage(&item.child_display)
                        .map_err(fail)?,
                    children: vec![],
                };
                for (assigned, _, position) in assignments {
                    if assigned == id {
                        record.position = position;
                    } else if let Some(previous) =
                        records.iter_mut().find(|item| item.id.value() == assigned)
                    {
                        previous.position = position;
                    }
                }
                records.push(record);
                inserted.insert(item.menu_code);
            }
            crate::application::navigation::validate_flat_tree(&records, max_depth)
                .map_err(fail)?;
            let rows = records
                .iter()
                .map(|record| {
                    let parent = records
                        .iter()
                        .find(|item| Some(&item.menu.menu_code) == record.parent_menu_code.as_ref())
                        .map(|item| item.id.value());
                    serde_json::json!([
                        record.id.value(),
                        record.menu.id.value(),
                        parent,
                        record.position,
                        record.child_display.as_storage()
                    ])
                })
                .collect::<Vec<_>>();
            c.execute("INSERT INTO navigation_menu_tree_items(id,tree_id,menu_id,parent_item_id,position,child_display) SELECT json_extract(value,'$[0]'),?1,json_extract(value,'$[1]'),json_extract(value,'$[2]'),json_extract(value,'$[3]'),json_extract(value,'$[4]') FROM json_each(?2)",params![trees.get(&domain.code).ok_or("tree not found")?,serde_json::to_string(&rows).map_err(fail)?]).map_err(fail)?;
            state.baseline = canonical(
                records
                    .into_iter()
                    .map(|record| PackItem {
                        menu_code: record.menu.menu_code,
                        parent_menu_code: record.parent_menu_code,
                        position: record.position,
                        child_display: record.child_display.as_storage().to_owned(),
                    })
                    .collect(),
            );
            continue;
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
            let mut menus=Vec::new();
            for (kind,code) in owned { if kind=="Domain" {remove_domain(c,&code)?;} else {menus.push(code);} }
            remove_menus(c,&menus)?;
            c.execute("DELETE FROM navigation_pack_state WHERE pack_id=?1",[&pack]).map_err(fail)?;Ok(())
        }),||"navigation storage unavailable".into())
    }
}
