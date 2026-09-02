impl SqliteNavigationRepository {
    pub fn connect(database_url: &str) -> Result<Self, NavigationError> {
        ensure_database_parent(database_url)?;
        let connection = if database_url == ":memory:" || database_url.starts_with("file:") {
            Connection::open_with_flags(
                database_url,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
                    | rusqlite::OpenFlags::SQLITE_OPEN_CREATE
                    | rusqlite::OpenFlags::SQLITE_OPEN_URI,
            )
        } else {
            Connection::open(database_url)
        }
        .map_err(storage_error)?;
        connection
            .execute_batch(NAVIGATION_MIGRATION)
            .map_err(storage_error)?;
        connection
            .execute_batch(NAVIGATION_AGENTS_SPLIT_MIGRATION)
            .map_err(storage_error)?;
        Ok(Self { connection })
    }

    fn all_domains(&self) -> Result<Vec<NavigationDomain>, NavigationError> {
        let mut statement = self.connection.prepare(
            "SELECT id, domain_code, built_in, enabled, is_default, position, visible_roles, created_at, updated_at,
                    (SELECT name FROM navigation_domain_translations WHERE domain_id = d.id AND locale = 'zh-CN'),
                    (SELECT name FROM navigation_domain_translations WHERE domain_id = d.id AND locale = 'en-US'),
                    (SELECT description FROM navigation_domain_translations WHERE domain_id = d.id AND locale = 'zh-CN'),
                    (SELECT description FROM navigation_domain_translations WHERE domain_id = d.id AND locale = 'en-US')
             FROM navigation_domains d
             ORDER BY position ASC, domain_code ASC",
        ).map_err(storage_error)?;
        statement
            .query_map([], |row| {
                Ok(NavigationDomain {
                    id: SnowflakeId::new(row.get(0)?),
                    domain_code: row.get(1)?,
                    built_in: row.get::<_, i64>(2)? == 1,
                    enabled: row.get::<_, i64>(3)? == 1,
                    is_default: row.get::<_, i64>(4)? == 1,
                    position: row.get(5)?,
                    visible_roles: parse_roles(&row.get::<_, String>(6)?)
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                    name: LocalizedText {
                        zh_cn: row.get::<_, Option<String>>(9)?.unwrap_or_default(),
                        en_us: row.get::<_, Option<String>>(10)?.unwrap_or_default(),
                    },
                    description: LocalizedText {
                        zh_cn: row.get::<_, Option<String>>(11)?.unwrap_or_default(),
                        en_us: row.get::<_, Option<String>>(12)?.unwrap_or_default(),
                    },
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn all_menus(&self) -> Result<Vec<NavigationMenu>, NavigationError> {
        let mut statement = self.connection.prepare(
            "SELECT id, menu_code, source, menu_type, page_code, url, icon, enabled, created_at, updated_at,
                    (SELECT label FROM navigation_menu_translations WHERE menu_id = m.id AND locale = 'zh-CN'),
                    (SELECT label FROM navigation_menu_translations WHERE menu_id = m.id AND locale = 'en-US')
             FROM navigation_menus m
             ORDER BY menu_code ASC",
        ).map_err(storage_error)?;
        statement
            .query_map([], |row| {
                Ok(NavigationMenu {
                    id: SnowflakeId::new(row.get(0)?),
                    menu_code: row.get(1)?,
                    source: MenuSource::from_storage(&row.get::<_, String>(2)?)
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    menu_type: MenuType::from_storage(&row.get::<_, String>(3)?)
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    page_code: row.get(4)?,
                    url: row.get(5)?,
                    icon: row.get(6)?,
                    enabled: row.get::<_, i64>(7)? == 1,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                    translations: LocalizedText {
                        zh_cn: row.get::<_, Option<String>>(10)?.unwrap_or_default(),
                        en_us: row.get::<_, Option<String>>(11)?.unwrap_or_default(),
                    },
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn load_items(&self, domain_code: &str) -> Result<Vec<NavigationTreeItem>, NavigationError> {
        let mut statement = self.connection.prepare(
            "SELECT i.id, i.position, i.visible_roles, pm.menu_code,
                    m.id, m.menu_code, m.source, m.menu_type, m.page_code, m.url, m.icon, m.enabled, m.created_at, m.updated_at,
                    (SELECT label FROM navigation_menu_translations WHERE menu_id = m.id AND locale = 'zh-CN'),
                    (SELECT label FROM navigation_menu_translations WHERE menu_id = m.id AND locale = 'en-US')
             FROM navigation_domain_menu_items i
             JOIN navigation_domains d ON d.id = i.domain_id
             JOIN navigation_menus m ON m.id = i.menu_id
             LEFT JOIN navigation_domain_menu_items p ON p.id = i.parent_item_id
             LEFT JOIN navigation_menus pm ON pm.id = p.menu_id
             WHERE d.domain_code = ?1
             ORDER BY i.position ASC, m.menu_code ASC",
        ).map_err(storage_error)?;
        statement
            .query_map([domain_code], |row| {
                Ok(NavigationTreeItem {
                    id: SnowflakeId::new(row.get(0)?),
                    position: row.get(1)?,
                    visible_roles: parse_roles(&row.get::<_, String>(2)?)
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    parent_menu_code: row.get(3)?,
                    menu: NavigationMenu {
                        id: SnowflakeId::new(row.get(4)?),
                        menu_code: row.get(5)?,
                        source: MenuSource::from_storage(&row.get::<_, String>(6)?)
                            .map_err(|_| rusqlite::Error::InvalidQuery)?,
                        menu_type: MenuType::from_storage(&row.get::<_, String>(7)?)
                            .map_err(|_| rusqlite::Error::InvalidQuery)?,
                        page_code: row.get(8)?,
                        url: row.get(9)?,
                        icon: row.get(10)?,
                        enabled: row.get::<_, i64>(11)? == 1,
                        created_at: row.get(12)?,
                        updated_at: row.get(13)?,
                        translations: LocalizedText {
                            zh_cn: row.get::<_, Option<String>>(14)?.unwrap_or_default(),
                            en_us: row.get::<_, Option<String>>(15)?.unwrap_or_default(),
                        },
                    },
                    children: Vec::new(),
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn next_id_tx(tx: &rusqlite::Transaction<'_>) -> Result<i64, NavigationError> {
        tx.query_row(
            "SELECT COALESCE(MAX(id), 734500000000000000) FROM (
                 SELECT id FROM navigation_domains
                 UNION ALL SELECT id FROM navigation_domain_translations
                 UNION ALL SELECT id FROM navigation_menus
                 UNION ALL SELECT id FROM navigation_menu_translations
                 UNION ALL SELECT id FROM navigation_domain_menu_items
             )",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|value| value + 1)
        .map_err(storage_error)
    }

    fn normalize_domain_positions(
        tx: &rusqlite::Transaction<'_>,
        domain_id: i64,
    ) -> Result<(), NavigationError> {
        let mut statement = tx
            .prepare(
                "SELECT id, parent_item_id, position FROM navigation_domain_menu_items
             WHERE domain_id = ?1 ORDER BY parent_item_id, position, id",
            )
            .map_err(storage_error)?;
        let rows = statement
            .query_map([domain_id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<i64>>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        let mut groups: BTreeMap<Option<i64>, Vec<(i64, i64)>> = BTreeMap::new();
        for (id, parent, position) in rows {
            groups.entry(parent).or_default().push((id, position));
        }
        let mut assignments = Vec::new();
        for (parent_id, entries) in groups {
            for (index, (id, _)) in entries.iter().enumerate() {
                assignments.push((*id, parent_id, index as i64));
            }
        }
        move_positions_out_of_way(tx, domain_id)?;
        apply_position_assignments(tx, domain_id, &mut assignments)?;
        Ok(())
    }

    fn domain_id(&self, domain_code: &str) -> Result<i64, NavigationError> {
        self.connection
            .query_row(
                "SELECT id FROM navigation_domains WHERE domain_code = ?1",
                [domain_code],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?
            .ok_or(NavigationError::DomainNotFound)
    }

    fn parent_item_id(
        &self,
        domain_id: i64,
        parent_menu_code: Option<&str>,
    ) -> Result<Option<i64>, NavigationError> {
        parent_menu_code.map(|code| self.connection.query_row("SELECT i.id FROM navigation_domain_menu_items i JOIN navigation_menus m ON m.id = i.menu_id WHERE i.domain_id = ?1 AND m.menu_code = ?2", params![domain_id, code], |row| row.get(0)).optional().map_err(storage_error)?.ok_or(NavigationError::InvalidParent)).transpose()
    }
}

fn move_positions_out_of_way(
    tx: &rusqlite::Transaction<'_>,
    domain_id: i64,
) -> Result<(), NavigationError> {
    tx.execute(
        "UPDATE navigation_domain_menu_items SET position = position + ?2 WHERE domain_id = ?1",
        params![domain_id, POSITION_WORK_OFFSET],
    )
    .map_err(storage_error)?;
    Ok(())
}

fn apply_position_assignments(
    tx: &rusqlite::Transaction<'_>,
    domain_id: i64,
    assignments: &mut [(i64, Option<i64>, i64)],
) -> Result<(), NavigationError> {
    assignments.sort_by_key(|(_, parent_id, position)| (*parent_id, *position));
    for &(id, parent_id, position) in assignments.iter() {
        tx.execute(
            "UPDATE navigation_domain_menu_items SET parent_item_id = ?2, position = ?3 WHERE domain_id = ?4 AND id = ?1",
            params![id, parent_id, position, domain_id],
        )
        .map_err(storage_error)?;
    }
    Ok(())
}

fn parent_id_for_code(items: &[NavigationTreeItem], parent_code: Option<&str>) -> Option<i64> {
    parent_code.and_then(|code| {
        items
            .iter()
            .find(|item| item.menu.menu_code == code)
            .map(|item| item.id.value())
    })
}

fn ordered_assignments_for_insert(
    items: &[NavigationTreeItem],
    item_id: i64,
    parent_code: Option<&str>,
    requested_position: i64,
    parent_id: Option<i64>,
) -> Vec<(i64, Option<i64>, i64)> {
    let mut groups: BTreeMap<Option<String>, Vec<(i64, i64, String)>> = BTreeMap::new();
    for item in items {
        groups
            .entry(item.parent_menu_code.clone())
            .or_default()
            .push((item.id.value(), item.position, item.menu.menu_code.clone()));
    }
    let group = groups
        .entry(parent_code.map(ToOwned::to_owned))
        .or_default();
    group.sort_by_key(|(id, position, _)| (*position, *id));
    let insert_at = requested_position.clamp(0, group.len() as i64) as usize;
    group.insert(insert_at, (item_id, requested_position, String::new()));

    groups
        .into_iter()
        .flat_map(|(group_parent_code, entries)| {
            let group_parent_id = if group_parent_code.as_deref() == parent_code {
                parent_id
            } else {
                parent_code_for_id(items, group_parent_code.as_deref())
            };
            entries
                .into_iter()
                .enumerate()
                .map(move |(position, (id, _, _))| (id, group_parent_id, position as i64))
        })
        .collect()
}

fn ordered_assignments_for_move(
    items: &[NavigationTreeItem],
    item_code: &str,
    parent_code: Option<&str>,
    requested_position: i64,
    parent_id: Option<i64>,
) -> Vec<(i64, Option<i64>, i64)> {
    let mut groups: BTreeMap<Option<String>, Vec<(i64, i64, String)>> = BTreeMap::new();
    for item in items {
        if item.menu.menu_code == item_code {
            continue;
        }
        groups
            .entry(item.parent_menu_code.clone())
            .or_default()
            .push((item.id.value(), item.position, item.menu.menu_code.clone()));
    }
    let group = groups
        .entry(parent_code.map(ToOwned::to_owned))
        .or_default();
    group.sort_by_key(|(id, position, _)| (*position, *id));
    let insert_at = requested_position.clamp(0, group.len() as i64) as usize;
    let moved_id = items
        .iter()
        .find(|item| item.menu.menu_code == item_code)
        .map(|item| item.id.value())
        .unwrap_or_default();
    group.insert(
        insert_at,
        (moved_id, requested_position, item_code.to_string()),
    );

    groups
        .into_iter()
        .flat_map(|(group_parent_code, entries)| {
            let group_parent_id = if group_parent_code.as_deref() == parent_code {
                parent_id
            } else {
                parent_code_for_id(items, group_parent_code.as_deref())
            };
            entries
                .into_iter()
                .enumerate()
                .map(move |(position, (id, _, _))| (id, group_parent_id, position as i64))
        })
        .collect()
}

fn parent_code_for_id(items: &[NavigationTreeItem], parent_code: Option<&str>) -> Option<i64> {
    parent_code_for_code(items, parent_code).map(|item| item.id.value())
}

fn parent_code_for_code<'a>(
    items: &'a [NavigationTreeItem],
    parent_code: Option<&str>,
) -> Option<&'a NavigationTreeItem> {
    parent_code.and_then(|code| items.iter().find(|item| item.menu.menu_code == code))
}
