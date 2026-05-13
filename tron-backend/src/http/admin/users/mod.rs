use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use serde::{Deserialize, Serialize};

use crate::{
    app::AppState,
    domain::user::{
        CreateUserCommand, ListUsersQuery, PagedUsers, PasswordResetResult, UpdateUserCommand,
        User, UserError, UserService,
    },
    response::ApiResponse,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/admin/users", get(list_users).post(create_user))
        .route(
            "/api/v1/admin/users/{user_code}",
            get(get_user).patch(patch_user),
        )
        .route(
            "/api/v1/admin/users/{user_code}/archive",
            post(archive_user),
        )
        .route(
            "/api/v1/admin/users/{user_code}/restore",
            post(restore_user),
        )
        .route("/api/v1/admin/users/{user_code}/roles", put(replace_roles))
        .route(
            "/api/v1/admin/users/{user_code}/spaces",
            put(replace_spaces),
        )
        .route(
            "/api/v1/admin/users/{user_code}/password/reset",
            post(reset_password),
        )
}

async fn list_users(
    State(users): State<UserService>,
    Query(query): Query<ListUsersQuery>,
) -> Response {
    match users.list_users(query) {
        Ok(page) => Json(ApiResponse::success(PagedUsersDto::from(page))).into_response(),
        Err(error) => user_error_response(error),
    }
}

async fn create_user(
    State(users): State<UserService>,
    Json(command): Json<CreateUserCommand>,
) -> Response {
    match users.create_user(command) {
        Ok(user) => (
            StatusCode::CREATED,
            Json(ApiResponse::success(UserDetailDto::from(user))),
        )
            .into_response(),
        Err(error) => user_error_response(error),
    }
}

async fn get_user(State(users): State<UserService>, Path(user_code): Path<String>) -> Response {
    match users.get_user(&user_code) {
        Ok(user) => Json(ApiResponse::success(UserDetailDto::from(user))).into_response(),
        Err(error) => user_error_response(error),
    }
}

async fn patch_user(
    State(users): State<UserService>,
    Path(user_code): Path<String>,
    Json(command): Json<UpdateUserCommand>,
) -> Response {
    match users.patch_user(&user_code, command) {
        Ok(user) => Json(ApiResponse::success(UserDetailDto::from(user))).into_response(),
        Err(error) => user_error_response(error),
    }
}

async fn archive_user(State(users): State<UserService>, Path(user_code): Path<String>) -> Response {
    match users.archive_user(&user_code) {
        Ok(user) => Json(ApiResponse::success(UserDetailDto::from(user))).into_response(),
        Err(error) => user_error_response(error),
    }
}

async fn restore_user(State(users): State<UserService>, Path(user_code): Path<String>) -> Response {
    match users.restore_user(&user_code) {
        Ok(user) => Json(ApiResponse::success(UserDetailDto::from(user))).into_response(),
        Err(error) => user_error_response(error),
    }
}

async fn replace_roles(
    State(users): State<UserService>,
    Path(user_code): Path<String>,
    Json(request): Json<ReplaceRolesRequest>,
) -> Response {
    match users.replace_roles(&user_code, request.role_codes) {
        Ok(user) => Json(ApiResponse::success(AssignmentDto::from(user))).into_response(),
        Err(error) => user_error_response(error),
    }
}

async fn replace_spaces(
    State(users): State<UserService>,
    Path(user_code): Path<String>,
    Json(request): Json<ReplaceSpacesRequest>,
) -> Response {
    match users.replace_spaces(&user_code, request.space_codes) {
        Ok(user) => Json(ApiResponse::success(AssignmentDto::from(user))).into_response(),
        Err(error) => user_error_response(error),
    }
}

async fn reset_password(
    State(users): State<UserService>,
    Path(user_code): Path<String>,
) -> Response {
    match users.reset_password(&user_code) {
        Ok(result) => Json(ApiResponse::success(PasswordResetDto::from(result))).into_response(),
        Err(error) => user_error_response(error),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReplaceRolesRequest {
    #[serde(default)]
    role_codes: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReplaceSpacesRequest {
    #[serde(default)]
    space_codes: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PagedUsersDto {
    current: usize,
    size: usize,
    total: usize,
    records: Vec<UserListItemDto>,
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
    phone: Option<crate::domain::user::PhoneNumber>,
    avatar_url: Option<String>,
    locale: String,
    timezone: String,
    status: crate::domain::user::UserStatus,
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
struct UserDetailDto {
    id: String,
    user_code: String,
    username: String,
    first_name: String,
    last_name: String,
    display_name: String,
    email: String,
    phone: Option<crate::domain::user::PhoneNumber>,
    avatar_url: Option<String>,
    locale: String,
    timezone: String,
    status: crate::domain::user::UserStatus,
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
struct AssignmentDto {
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
struct PasswordResetDto {
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

fn user_error_response(error: UserError) -> Response {
    let (status, code, message) = match error {
        UserError::NotFound => (
            StatusCode::NOT_FOUND,
            "ADMIN_USER_NOT_FOUND",
            "user was not found",
        ),
        UserError::InvalidUserCode => (
            StatusCode::BAD_REQUEST,
            "ADMIN_USER_CODE_INVALID",
            "userCode must contain uppercase letters, digits, underscore, or hyphen only",
        ),
        UserError::DuplicateUserCode => (
            StatusCode::CONFLICT,
            "ADMIN_USER_CODE_DUPLICATE",
            "userCode already exists",
        ),
        UserError::DuplicateUsername => (
            StatusCode::CONFLICT,
            "ADMIN_USER_USERNAME_DUPLICATE",
            "username already exists",
        ),
        UserError::DuplicateEmail => (
            StatusCode::CONFLICT,
            "ADMIN_USER_EMAIL_DUPLICATE",
            "email already exists",
        ),
        UserError::RequiredProfileField => (
            StatusCode::BAD_REQUEST,
            "ADMIN_USER_PROFILE_FIELD_REQUIRED",
            "username, firstName, lastName, displayName, and email are required",
        ),
        UserError::WeakInitialPassword => (
            StatusCode::BAD_REQUEST,
            "ADMIN_USER_PASSWORD_WEAK",
            "initialPassword does not satisfy the MVP password policy",
        ),
        UserError::InvalidStatus => (
            StatusCode::BAD_REQUEST,
            "ADMIN_USER_STATUS_INVALID",
            "status must be Active or Archived",
        ),
        UserError::InvalidRoleCode => (
            StatusCode::BAD_REQUEST,
            "ADMIN_USER_ROLE_CODE_INVALID",
            "roleCodes must contain fixed MVP role codes only",
        ),
        UserError::InvalidSpaceCode => (
            StatusCode::BAD_REQUEST,
            "ADMIN_USER_SPACE_CODE_INVALID",
            "spaceCodes must be non-empty uppercase stable codes",
        ),
        UserError::PasswordHashFailed => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "ADMIN_USER_PASSWORD_HASH_FAILED",
            "password hash could not be generated",
        ),
        UserError::StorageFailure => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "ADMIN_USER_STORAGE_FAILED",
            "user storage operation failed",
        ),
    };

    (status, Json(ApiResponse::error(code, message))).into_response()
}
