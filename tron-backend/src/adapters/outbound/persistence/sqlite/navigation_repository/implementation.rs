impl NavigationRepository for SqliteNavigationRepository {
    fn next_id(&mut self) -> Result<i64, NavigationError> {
        self.connection
            .query_row(
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

    fn list_domains(&mut self) -> Result<Vec<NavigationDomain>, NavigationError> {
        self.all_domains()
    }

    fn get_domain(&mut self, domain_code: &str) -> Result<NavigationDomain, NavigationError> {
        self.all_domains()?
            .into_iter()
            .find(|domain| domain.domain_code == domain_code)
            .ok_or(NavigationError::DomainNotFound)
    }

    fn save_domain(&mut self, domain: &NavigationDomain) -> Result<(), NavigationError> {
        let tx = self.connection.transaction().map_err(storage_error)?;
        if domain.is_default {
            tx.execute(
                "UPDATE navigation_domains SET is_default = 0 WHERE domain_code <> ?1",
                [&domain.domain_code],
            )
            .map_err(storage_error)?;
        }
        tx.execute(
            "INSERT INTO navigation_domains (id, domain_code, built_in, enabled, is_default, position, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(domain_code) DO UPDATE SET enabled = excluded.enabled, is_default = excluded.is_default,
                 position = excluded.position, updated_at = excluded.updated_at",
            params![domain.id.value(), domain.domain_code, i64::from(domain.built_in), i64::from(domain.enabled), i64::from(domain.is_default), domain.position, domain.created_at, domain.updated_at],
        ).map_err(storage_error)?;
        let domain_id: i64 = tx
            .query_row(
                "SELECT id FROM navigation_domains WHERE domain_code = ?1",
                [&domain.domain_code],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        tx.execute(
            "DELETE FROM navigation_domain_translations WHERE domain_id = ?1",
            [domain_id],
        )
        .map_err(storage_error)?;
        let mut id = Self::next_id_tx(&tx)?;
        for (locale, name, description) in [
            ("zh-CN", &domain.name.zh_cn, &domain.description.zh_cn),
            ("en-US", &domain.name.en_us, &domain.description.en_us),
        ] {
            tx.execute("INSERT INTO navigation_domain_translations (id, domain_id, locale, name, description) VALUES (?1, ?2, ?3, ?4, ?5)", params![id, domain_id, locale, name, description]).map_err(storage_error)?;
            id += 1;
        }
        tx.commit().map_err(storage_error)
    }

    fn delete_domain(&mut self, domain_code: &str) -> Result<(), NavigationError> {
        let changed = self
            .connection
            .execute(
                "DELETE FROM navigation_domains WHERE domain_code = ?1",
                [domain_code],
            )
            .map_err(storage_error)?;
        if changed == 0 {
            Err(NavigationError::DomainNotFound)
        } else {
            Ok(())
        }
    }

    fn list_menus(
        &mut self,
        query: &MenuQuery,
    ) -> Result<NavigationPage<NavigationMenu>, NavigationError> {
        let mut records = self
            .all_menus()?
            .into_iter()
            .filter(|menu| {
                let keyword_match = query.keyword.trim().is_empty()
                    || menu.menu_code.contains(&query.keyword.to_ascii_uppercase())
                    || menu.translations.zh_cn.contains(&query.keyword)
                    || menu
                        .translations
                        .en_us
                        .to_ascii_lowercase()
                        .contains(&query.keyword.to_ascii_lowercase());
                keyword_match
                    && query.source.is_none_or(|source| menu.source == source)
                    && query.menu_type.is_none_or(|kind| menu.menu_type == kind)
                    && query.enabled.is_none_or(|enabled| menu.enabled == enabled)
            })
            .collect::<Vec<_>>();
        let total = records.len() as i64;
        let start = ((query.current - 1) * query.size) as usize;
        records = records
            .into_iter()
            .skip(start)
            .take(query.size as usize)
            .collect();
        Ok(NavigationPage {
            current: query.current,
            size: query.size,
            total,
            records,
        })
    }

    fn get_menu(&mut self, menu_code: &str) -> Result<NavigationMenu, NavigationError> {
        self.all_menus()?
            .into_iter()
            .find(|menu| menu.menu_code == menu_code)
            .ok_or(NavigationError::MenuNotFound)
    }

    fn save_menu(&mut self, menu: &NavigationMenu) -> Result<(), NavigationError> {
        let tx = self.connection.transaction().map_err(storage_error)?;
        tx.execute(
            "INSERT INTO navigation_menus (id, menu_code, source, menu_type, page_code, url, icon, enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(menu_code) DO UPDATE SET menu_type = excluded.menu_type, page_code = excluded.page_code,
                 url = excluded.url, icon = excluded.icon, enabled = excluded.enabled, updated_at = excluded.updated_at",
            params![menu.id.value(), menu.menu_code, menu.source.as_storage(), menu.menu_type.as_storage(), menu.page_code, menu.url, menu.icon, i64::from(menu.enabled), menu.created_at, menu.updated_at],
        ).map_err(|error| if error.sqlite_error_code() == Some(ErrorCode::ConstraintViolation) { NavigationError::DuplicateMenuCode } else { NavigationError::StorageFailure })?;
        let menu_id: i64 = tx
            .query_row(
                "SELECT id FROM navigation_menus WHERE menu_code = ?1",
                [&menu.menu_code],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        tx.execute(
            "DELETE FROM navigation_menu_translations WHERE menu_id = ?1",
            [menu_id],
        )
        .map_err(storage_error)?;
        let mut id = Self::next_id_tx(&tx)?;
        for (locale, label) in [
            ("zh-CN", &menu.translations.zh_cn),
            ("en-US", &menu.translations.en_us),
        ] {
            tx.execute("INSERT INTO navigation_menu_translations (id, menu_id, locale, label) VALUES (?1, ?2, ?3, ?4)", params![id, menu_id, locale, label]).map_err(storage_error)?;
            id += 1;
        }
        tx.commit().map_err(storage_error)
    }

    fn delete_menu(&mut self, menu_code: &str) -> Result<(), NavigationError> {
        let changed = self
            .connection
            .execute(
                "DELETE FROM navigation_menus WHERE menu_code = ?1",
                [menu_code],
            )
            .map_err(|error| {
                if error.sqlite_error_code() == Some(ErrorCode::ConstraintViolation) {
                    NavigationError::MenuInUse
                } else {
                    NavigationError::StorageFailure
                }
            })?;
        if changed == 0 {
            Err(NavigationError::MenuNotFound)
        } else {
            Ok(())
        }
    }

    fn menu_usage_count(&mut self, menu_code: &str) -> Result<i64, NavigationError> {
        self.connection.query_row("SELECT COUNT(*) FROM navigation_domain_menu_items i JOIN navigation_menus m ON m.id = i.menu_id WHERE m.menu_code = ?1", [menu_code], |row| row.get(0)).map_err(storage_error)
    }

    fn menu_has_children(&mut self, menu_code: &str) -> Result<bool, NavigationError> {
        self.connection.query_row("SELECT EXISTS(SELECT 1 FROM navigation_domain_menu_items child JOIN navigation_domain_menu_items parent ON parent.id = child.parent_item_id JOIN navigation_menus m ON m.id = parent.menu_id WHERE m.menu_code = ?1)", [menu_code], |row| row.get::<_, i64>(0)).map(|value| value == 1).map_err(storage_error)
    }

    fn list_items(
        &mut self,
        domain_code: &str,
    ) -> Result<Vec<NavigationTreeItem>, NavigationError> {
        self.domain_id(domain_code)?;
        self.load_items(domain_code)
    }

    fn add_item(
        &mut self,
        domain_code: &str,
        item: &NavigationTreeItem,
    ) -> Result<(), NavigationError> {
        let domain_id = self.domain_id(domain_code)?;
        let parent_id = self.parent_item_id(domain_id, item.parent_menu_code.as_deref())?;
        let existing = self.load_items(domain_code)?;
        let mut assignments = ordered_assignments_for_insert(
            &existing,
            item.id.value(),
            item.parent_menu_code.as_deref(),
            item.position,
            parent_id,
        );
        let tx = self.connection.transaction().map_err(storage_error)?;
        move_positions_out_of_way(&tx, domain_id)?;
        tx.execute("INSERT INTO navigation_domain_menu_items (id, domain_id, menu_id, parent_item_id, position) SELECT ?1, ?2, id, ?3, ?4 FROM navigation_menus WHERE menu_code = ?5", params![item.id.value(), domain_id, parent_id, 2 * POSITION_WORK_OFFSET, item.menu.menu_code]).map_err(|error| if error.sqlite_error_code() == Some(ErrorCode::ConstraintViolation) { NavigationError::DuplicateDomainItem } else { NavigationError::StorageFailure })?;
        apply_position_assignments(&tx, domain_id, &mut assignments)?;
        tx.commit().map_err(storage_error)
    }

    fn update_item(
        &mut self,
        domain_code: &str,
        menu_code: &str,
        command: &UpdateMenuItemCommand,
    ) -> Result<(), NavigationError> {
        let domain_id = self.domain_id(domain_code)?;
        let parent_id = self.parent_item_id(domain_id, command.parent_menu_code.as_deref())?;
        let items = self.load_items(domain_code)?;
        let item = items
            .iter()
            .find(|item| item.menu.menu_code == menu_code)
            .ok_or(NavigationError::ItemNotFound)?;
        let item_id = item.id.value();
        let mut assignments = ordered_assignments_for_move(
            &items,
            menu_code,
            command.parent_menu_code.as_deref(),
            command.position,
            parent_id,
        );
        let tx = self.connection.transaction().map_err(storage_error)?;
        move_positions_out_of_way(&tx, domain_id)?;
        tx.execute("UPDATE navigation_domain_menu_items SET parent_item_id = ?2, position = ?3 WHERE id = ?1", params![item_id, parent_id, 2 * POSITION_WORK_OFFSET]).map_err(storage_error)?;
        apply_position_assignments(&tx, domain_id, &mut assignments)?;
        tx.commit().map_err(storage_error)
    }

    fn remove_item(&mut self, domain_code: &str, menu_code: &str) -> Result<(), NavigationError> {
        let domain_id = self.domain_id(domain_code)?;
        let item_id: i64 = self.connection.query_row("SELECT i.id FROM navigation_domain_menu_items i JOIN navigation_menus m ON m.id = i.menu_id WHERE i.domain_id = ?1 AND m.menu_code = ?2", params![domain_id, menu_code], |row| row.get(0)).optional().map_err(storage_error)?.ok_or(NavigationError::ItemNotFound)?;
        let tx = self.connection.transaction().map_err(storage_error)?;
        tx.execute(
            "DELETE FROM navigation_domain_menu_items WHERE id = ?1",
            [item_id],
        )
        .map_err(storage_error)?;
        Self::normalize_domain_positions(&tx, domain_id)?;
        tx.commit().map_err(storage_error)
    }

    fn copy_tree(
        &mut self,
        target_domain_code: &str,
        command: &CopyMenuTreeCommand,
    ) -> Result<(), NavigationError> {
        let source = self.load_items(&command.source_domain_code)?;
        let target_domain_id = self.domain_id(target_domain_code)?;
        let target = self.load_items(target_domain_code)?;
        let target_parent_id =
            self.parent_item_id(target_domain_id, command.target_parent_menu_code.as_deref())?;
        let selected = selected_codes(&source, command.source_menu_code.as_deref())?;
        let selected_set = selected.iter().cloned().collect::<BTreeSet<_>>();
        let mut roots = source
            .iter()
            .filter(|item| {
                selected_set.contains(&item.menu.menu_code)
                    && item
                        .parent_menu_code
                        .as_deref()
                        .is_none_or(|parent| !selected_set.contains(parent))
            })
            .collect::<Vec<_>>();
        roots.sort_by_key(|item| item.position);
        let tx = self.connection.transaction().map_err(storage_error)?;
        move_positions_out_of_way(&tx, target_domain_id)?;
        let mut next_id = Self::next_id_tx(&tx)?;
        let mut mapped = BTreeMap::new();
        fn visit(
            tx: &rusqlite::Transaction<'_>,
            source: &[NavigationTreeItem],
            selected: &BTreeSet<String>,
            item: &NavigationTreeItem,
            root_position: i64,
            target_parent_id: Option<i64>,
            target_domain_code: &str,
            mapped: &mut BTreeMap<String, i64>,
            next_id: &mut i64,
        ) -> Result<(), NavigationError> {
            let parent_id = item
                .parent_menu_code
                .as_deref()
                .and_then(|parent| mapped.get(parent).copied())
                .or(target_parent_id);
            let position = if item
                .parent_menu_code
                .as_deref()
                .is_none_or(|parent| !selected.contains(parent))
            {
                2000000 + root_position
            } else {
                2000000 + item.position
            };
            let id = *next_id;
            *next_id += 1;
            tx.execute("INSERT INTO navigation_domain_menu_items (id, domain_id, menu_id, parent_item_id, position) SELECT ?1, (SELECT id FROM navigation_domains WHERE domain_code = ?2), m.id, ?3, ?4 FROM navigation_menus m WHERE m.menu_code = ?5", params![id, target_domain_code, parent_id, position, item.menu.menu_code]).map_err(storage_error)?;
            mapped.insert(item.menu.menu_code.clone(), id);
            let mut children = source
                .iter()
                .filter(|child| {
                    child.parent_menu_code.as_deref() == Some(item.menu.menu_code.as_str())
                        && selected.contains(&child.menu.menu_code)
                })
                .collect::<Vec<_>>();
            children.sort_by_key(|child| child.position);
            for child in children {
                visit(
                    tx,
                    source,
                    selected,
                    child,
                    root_position,
                    target_parent_id,
                    target_domain_code,
                    mapped,
                    next_id,
                )?;
            }
            Ok(())
        }
        for (index, root) in roots.iter().enumerate() {
            visit(
                &tx,
                &source,
                &selected_set,
                root,
                command.position + index as i64,
                target_parent_id,
                target_domain_code,
                &mut mapped,
                &mut next_id,
            )?;
        }
        let assignments = copy_position_assignments(
            &target,
            &source,
            &selected_set,
            &mapped,
            target_parent_id,
            command.position,
        );
        let mut assignments = assignments;
        apply_position_assignments(&tx, target_domain_id, &mut assignments)?;
        tx.commit().map_err(storage_error)
    }

    fn restore_default_menu_trees(
        &mut self,
        items: &[DefaultNavigationMenuTreeItem],
    ) -> Result<i64, NavigationError> {
        let domain_codes = items
            .iter()
            .map(|item| item.domain_code)
            .collect::<BTreeSet<_>>();
        let tx = self.connection.transaction().map_err(storage_error)?;
        let mut domain_ids = BTreeMap::new();
        for domain_code in domain_codes {
            let domain_id = tx
                .query_row(
                    "SELECT id FROM navigation_domains WHERE domain_code = ?1",
                    [domain_code],
                    |row| row.get::<_, i64>(0),
                )
                .optional()
                .map_err(storage_error)?
                .ok_or(NavigationError::DomainNotFound)?;
            domain_ids.insert(domain_code, domain_id);
        }
        for domain_id in domain_ids.values() {
            move_positions_out_of_way(&tx, *domain_id)?;
        }

        let mut next_id = Self::next_id_tx(&tx)?;
        let mut restored_item_count = 0;
        for item in items {
            let domain_id = domain_ids
                .get(item.domain_code)
                .copied()
                .ok_or(NavigationError::DomainNotFound)?;
            let exists = tx
                .query_row(
                    "SELECT EXISTS(
                        SELECT 1
                        FROM navigation_domain_menu_items i
                        JOIN navigation_menus m ON m.id = i.menu_id
                        WHERE i.domain_id = ?1 AND m.menu_code = ?2
                    )",
                    params![domain_id, item.menu_code],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(storage_error)?
                == 1;
            if exists {
                continue;
            }
            let parent_item_id = item
                .parent_menu_code
                .map(|parent_menu_code| {
                    tx.query_row(
                        "SELECT i.id
                         FROM navigation_domain_menu_items i
                         JOIN navigation_menus m ON m.id = i.menu_id
                         WHERE i.domain_id = ?1 AND m.menu_code = ?2",
                        params![domain_id, parent_menu_code],
                        |row| row.get::<_, i64>(0),
                    )
                    .optional()
                    .map_err(storage_error)?
                    .ok_or(NavigationError::InvalidParent)
                })
                .transpose()?;
            let menu_id = tx
                .query_row(
                    "SELECT id FROM navigation_menus WHERE menu_code = ?1",
                    [item.menu_code],
                    |row| row.get::<_, i64>(0),
                )
                .optional()
                .map_err(storage_error)?
                .ok_or(NavigationError::MenuNotFound)?;
            tx.execute(
                "INSERT INTO navigation_domain_menu_items
                    (id, domain_id, menu_id, parent_item_id, position)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    next_id,
                    domain_id,
                    menu_id,
                    parent_item_id,
                    item.position,
                ],
            )
            .map_err(storage_error)?;
            next_id += 1;
            restored_item_count += 1;
        }
        for domain_id in domain_ids.values() {
            Self::normalize_domain_positions(&tx, *domain_id)?;
        }
        tx.commit().map_err(storage_error)?;
        Ok(restored_item_count)
    }
}

fn copy_position_assignments(
    target: &[NavigationTreeItem],
    source: &[NavigationTreeItem],
    selected: &BTreeSet<String>,
    mapped: &BTreeMap<String, i64>,
    target_parent_id: Option<i64>,
    requested_position: i64,
) -> Vec<(i64, Option<i64>, i64)> {
    let mut groups: BTreeMap<Option<i64>, Vec<(i64, i64, bool)>> = BTreeMap::new();
    for item in target {
        groups
            .entry(parent_id_for_code(target, item.parent_menu_code.as_deref()))
            .or_default()
            .push((item.id.value(), item.position, false));
    }

    for item in source
        .iter()
        .filter(|item| selected.contains(&item.menu.menu_code))
    {
        let Some(id) = mapped.get(&item.menu.menu_code).copied() else {
            continue;
        };
        let is_root = item
            .parent_menu_code
            .as_deref()
            .is_none_or(|parent| !selected.contains(parent));
        let parent_id = if is_root {
            target_parent_id
        } else {
            item.parent_menu_code
                .as_deref()
                .and_then(|parent| mapped.get(parent).copied())
        };
        groups
            .entry(parent_id)
            .or_default()
            .push((id, item.position, is_root));
    }

    let mut assignments = Vec::new();
    for (parent_id, entries) in groups {
        let mut existing = entries
            .iter()
            .filter(|(_, _, is_root)| !is_root)
            .map(|(id, position, _)| (*id, *position))
            .collect::<Vec<_>>();
        existing.sort_by_key(|(id, position)| (*position, *id));
        let mut copied_roots = entries
            .iter()
            .filter(|(_, _, is_root)| *is_root)
            .map(|(id, position, _)| (*id, *position))
            .collect::<Vec<_>>();
        copied_roots.sort_by_key(|(_, position)| *position);

        let ordered = if parent_id == target_parent_id && !copied_roots.is_empty() {
            let insert_at = requested_position.clamp(0, existing.len() as i64) as usize;
            let mut ordered = existing;
            for (offset, (id, _)) in copied_roots.into_iter().enumerate() {
                ordered.insert(insert_at + offset, (id, requested_position + offset as i64));
            }
            ordered
        } else {
            let mut ordered = entries
                .into_iter()
                .map(|(id, position, _)| (id, position))
                .collect::<Vec<_>>();
            ordered.sort_by_key(|(id, position)| (*position, *id));
            ordered
        };
        for (position, (id, _)) in ordered.into_iter().enumerate() {
            assignments.push((id, parent_id, position as i64));
        }
    }
    assignments
}

fn selected_codes(
    items: &[NavigationTreeItem],
    root: Option<&str>,
) -> Result<Vec<String>, NavigationError> {
    let mut result = Vec::new();
    fn visit(items: &[NavigationTreeItem], code: &str, result: &mut Vec<String>) {
        result.push(code.to_string());
        let mut children = items
            .iter()
            .filter(|item| item.parent_menu_code.as_deref() == Some(code))
            .collect::<Vec<_>>();
        children.sort_by_key(|item| item.position);
        for child in children {
            visit(items, &child.menu.menu_code, result);
        }
    }
    if let Some(root) = root {
        if !items.iter().any(|item| item.menu.menu_code == root) {
            return Err(NavigationError::ItemNotFound);
        }
        visit(items, root, &mut result);
    } else {
        let mut roots = items
            .iter()
            .filter(|item| item.parent_menu_code.is_none())
            .collect::<Vec<_>>();
        roots.sort_by_key(|item| item.position);
        for root in roots {
            visit(items, &root.menu.menu_code, &mut result);
        }
    }
    Ok(result)
}

fn ensure_database_parent(database_url: &str) -> Result<(), NavigationError> {
    if database_url == ":memory:" || database_url.starts_with("file:") {
        return Ok(());
    }
    if let Some(parent) = Path::new(database_url).parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|_| NavigationError::StorageFailure)?;
    }
    Ok(())
}

fn storage_error(_error: rusqlite::Error) -> NavigationError {
    NavigationError::StorageFailure
}
