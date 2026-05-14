use serde::Serialize;

use crate::domain::user::{PagedUsers, PasswordResetResult, PhoneNumber, User, UserStatus};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PagedUsersDto {
    current: usize,
    size: usize,
    total: usize,
    records: Vec<UserListItemDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CountDto {
    pub count: usize,
}

impl From<PagedUsers> for PagedUsersDto {
    fn from(page: PagedUsers) -> Self {
        Self {
            current: page.current,
            size: page.size,
            total: page.total,
            records: page
                .records
                .into_iter()
                .map(UserListItemDto::from)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UserListItemDto {
    id: String,
    user_code: String,
    username: String,
    first_name: String,
    last_name: String,
    display_name: String,
    email: String,
    phone: Option<PhoneNumber>,
    avatar_url: Option<String>,
    locale: String,
    timezone: String,
    status: UserStatus,
    role_codes: Vec<String>,
    space_codes: Vec<String>,
    password_reset_required: bool,
    last_active_at: Option<String>,
    created_at: String,
    updated_at: String,
}

impl From<User> for UserListItemDto {
    fn from(user: User) -> Self {
        Self {
            id: user.id.as_decimal_string(),
            user_code: user.user_code,
            username: user.username,
            first_name: user.first_name,
            last_name: user.last_name,
            display_name: user.display_name,
            email: user.email,
            phone: user.phone,
            avatar_url: user.avatar_url,
            locale: user.locale,
            timezone: user.timezone,
            status: user.status,
            role_codes: user.role_codes,
            space_codes: user.space_codes,
            password_reset_required: user.password_reset_required,
            last_active_at: user.last_active_at,
            created_at: user.created_at,
            updated_at: user.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDetailDto {
    id: String,
    user_code: String,
    username: String,
    first_name: String,
    last_name: String,
    display_name: String,
    email: String,
    phone: Option<PhoneNumber>,
    avatar_url: Option<String>,
    locale: String,
    timezone: String,
    status: UserStatus,
    role_codes: Vec<String>,
    space_codes: Vec<String>,
    role_assignments: Vec<RoleAssignmentDto>,
    space_assignments: Vec<SpaceAssignmentDto>,
    password: PasswordStateDto,
    password_reset_required: bool,
    last_active_at: Option<String>,
    created_at: String,
    updated_at: String,
    archived_at: Option<String>,
}

impl From<User> for UserDetailDto {
    fn from(user: User) -> Self {
        let role_assignments = user
            .role_codes
            .iter()
            .cloned()
            .map(|role_code| RoleAssignmentDto { role_code })
            .collect();
        let space_assignments = user
            .space_codes
            .iter()
            .cloned()
            .map(|space_code| SpaceAssignmentDto { space_code })
            .collect();

        Self {
            id: user.id.as_decimal_string(),
            user_code: user.user_code,
            username: user.username,
            first_name: user.first_name,
            last_name: user.last_name,
            display_name: user.display_name,
            email: user.email,
            phone: user.phone,
            avatar_url: user.avatar_url,
            locale: user.locale,
            timezone: user.timezone,
            status: user.status,
            role_codes: user.role_codes,
            space_codes: user.space_codes,
            role_assignments,
            space_assignments,
            password: PasswordStateDto {
                reset_required: user.password_reset_required,
                updated_at: user.updated_at.clone(),
                last_reset_at: None,
            },
            password_reset_required: user.password_reset_required,
            last_active_at: user.last_active_at,
            created_at: user.created_at,
            updated_at: user.updated_at,
            archived_at: user.archived_at,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentDto {
    user_code: String,
    role_codes: Vec<String>,
    space_codes: Vec<String>,
}

impl From<User> for AssignmentDto {
    fn from(user: User) -> Self {
        Self {
            user_code: user.user_code,
            role_codes: user.role_codes,
            space_codes: user.space_codes,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RoleAssignmentDto {
    role_code: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SpaceAssignmentDto {
    space_code: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PasswordStateDto {
    reset_required: bool,
    updated_at: String,
    last_reset_at: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordResetDto {
    user_code: String,
    temporary_password: String,
    password_reset_required: bool,
}

impl From<PasswordResetResult> for PasswordResetDto {
    fn from(result: PasswordResetResult) -> Self {
        Self {
            user_code: result.user_code,
            temporary_password: result.temporary_password,
            password_reset_required: result.password_reset_required,
        }
    }
}
