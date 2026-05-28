use super::*;

pub(super) fn paginate_records<T: Clone + SearchableRecord>(
    records: Vec<T>,
    query: WorkspaceAccessListQuery,
) -> PagedWorkspaceRecords<T> {
    let current = query.current.unwrap_or(1).max(1);
    let size = query.size.unwrap_or(20).clamp(1, 100);
    let search = query.search.map(|value| value.to_ascii_lowercase());
    let mut filtered = records
        .into_iter()
        .filter(|record| {
            search
                .as_ref()
                .is_none_or(|search| record.matches_search(search))
        })
        .collect::<Vec<_>>();
    filtered.sort_by(|left, right| left.sort_key().cmp(&right.sort_key()));
    let total = filtered.len();
    let start = ((current - 1) * size).min(total);
    let end = (start + size).min(total);

    PagedWorkspaceRecords {
        current,
        size,
        total,
        records: filtered[start..end].to_vec(),
    }
}

pub(super) trait SearchableRecord {
    fn matches_search(&self, search: &str) -> bool;
    fn sort_key(&self) -> String;
}

impl SearchableRecord for WorkspaceUserAccessListItem {
    fn matches_search(&self, search: &str) -> bool {
        self.user_code.to_ascii_lowercase().contains(search)
            || self.display_name.to_ascii_lowercase().contains(search)
            || self.email.to_ascii_lowercase().contains(search)
    }

    fn sort_key(&self) -> String {
        self.display_name.clone()
    }
}

impl SearchableRecord for WorkspaceEffectiveUser {
    fn matches_search(&self, search: &str) -> bool {
        self.user_code.to_ascii_lowercase().contains(search)
            || self.display_name.to_ascii_lowercase().contains(search)
            || self.email.to_ascii_lowercase().contains(search)
            || self
                .matched_role_codes
                .iter()
                .any(|code| code.to_ascii_lowercase().contains(search))
    }

    fn sort_key(&self) -> String {
        self.display_name.clone()
    }
}
