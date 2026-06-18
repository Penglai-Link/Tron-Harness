use super::*;

impl SkillRegistryService {
    pub(super) fn sync_registry(
        &self,
        checked: bool,
        updated: bool,
    ) -> Result<SkillRegistrySyncResult, SkillError> {
        self.ensure_registry_root()?;
        let existing = self
            .repository
            .lock()
            .expect("skill registry repository mutex poisoned")
            .list_registry_skills()?
            .into_iter()
            .map(|skill| (skill.skill_name.clone(), skill))
            .collect::<HashMap<_, _>>();
        let lock_entries = read_skills_lock(&self.registry_lock_path())?;
        let skill_dirs = list_skill_directories(&self.skills_dir())?;
        let mut seen = BTreeSet::new();

        for skill_dir in &skill_dirs {
            let Some(skill_name) = skill_dir.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            seen.insert(skill_name.to_owned());
            let lock_entry = lock_entries.get(skill_name);
            let existing_skill = existing.get(skill_name);
            let skill_markdown = read_skill_markdown(skill_dir)?;
            let frontmatter = parse_skill_frontmatter(&skill_markdown);
            let managed_by = if lock_entry.is_some() {
                SkillManagedBy::Cli
            } else {
                SkillManagedBy::Manual
            };
            let source_ref = lock_entry
                .map(|entry| entry.source.clone())
                .or_else(|| existing_skill.map(|skill| skill.source_ref.clone()))
                .unwrap_or_else(|| skill_dir.display().to_string());
            let source_type = lock_entry
                .and_then(|entry| entry.source_type.as_deref())
                .and_then(parse_source_type)
                .unwrap_or(if managed_by == SkillManagedBy::Manual {
                    SkillSourceType::Manual
                } else {
                    SkillSourceType::GitHub
                });
            let update_status = if managed_by == SkillManagedBy::Manual {
                SkillUpdateStatus::Manual
            } else if updated {
                SkillUpdateStatus::UpToDate
            } else {
                existing_skill
                    .map(|skill| skill.update_status)
                    .unwrap_or(SkillUpdateStatus::UpToDate)
            };
            let skill = RegistrySkill {
                skill_name: skill_name.to_owned(),
                source_ref,
                source_type,
                registry_path: skill_dir.display().to_string(),
                declared_version: frontmatter.declared_version,
                update_status,
                managed_by,
                last_checked_at: if checked || updated {
                    Some(current_timestamp())
                } else {
                    existing_skill.and_then(|skill| skill.last_checked_at.clone())
                },
                last_updated_at: if updated {
                    Some(current_timestamp())
                } else {
                    existing_skill.and_then(|skill| skill.last_updated_at.clone())
                },
                last_error: None,
                created_at: existing_skill
                    .map(|skill| skill.created_at.clone())
                    .unwrap_or_else(current_timestamp),
                updated_at: current_timestamp(),
            };
            self.repository
                .lock()
                .expect("skill registry repository mutex poisoned")
                .upsert_registry_skill(&skill)?;
        }

        let mut removed_count = 0;
        for skill_name in existing.keys() {
            if seen.contains(skill_name) {
                continue;
            }
            removed_count += usize::from(
                self.repository
                    .lock()
                    .expect("skill registry repository mutex poisoned")
                    .delete_registry_skill(skill_name)?,
            );
        }

        Ok(SkillRegistrySyncResult {
            registry_root: self.registry_root.display().to_string(),
            discovered_count: seen.len(),
            removed_count,
            checked,
            updated,
            rag_index: None,
        })
    }
}
