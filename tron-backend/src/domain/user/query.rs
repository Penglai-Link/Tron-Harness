use serde::Deserialize;

use super::{User, UserStatus};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListUsersQuery {
    pub page: Option<usize>,
    pub current: Option<usize>,
    pub size: Option<usize>,
    pub status: Option<UserStatus>,
    pub search: Option<String>,
    pub role_code: Option<String>,
    pub space_code: Option<String>,
    pub include_archived: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct PagedUsers {
    pub current: usize,
    pub size: usize,
    pub total: usize,
    pub records: Vec<User>,
}

#[derive(Debug, Clone, Copy)]
pub struct UserStatistics {
    pub total_users: usize,
    pub active_users: usize,
    pub role_assignments: usize,
    pub space_assignments: usize,
}
