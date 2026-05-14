use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{
    application::user::UserService,
    domain::user::{ListUsersQuery, UserStatistics},
    response::ApiResponse,
};

use super::{
    error_mapper::user_error_response,
    request::{CreateUserRequest, ReplaceRolesRequest, ReplaceSpacesRequest, UpdateUserRequest},
    response::{AssignmentDto, CountDto, PagedUsersDto, PasswordResetDto, UserDetailDto},
};

pub async fn list_users(
    State(users): State<UserService>,
    Query(query): Query<ListUsersQuery>,
) -> Response {
    match users.list_users(query) {
        Ok(page) => Json(ApiResponse::success(PagedUsersDto::from(page))).into_response(),
        Err(error) => user_error_response(error),
    }
}

pub async fn total_users(State(users): State<UserService>) -> Response {
    statistic_response(users, |statistics| statistics.total_users)
}

pub async fn active_users(State(users): State<UserService>) -> Response {
    statistic_response(users, |statistics| statistics.active_users)
}

pub async fn role_assignments(State(users): State<UserService>) -> Response {
    statistic_response(users, |statistics| statistics.role_assignments)
}

pub async fn space_assignments(State(users): State<UserService>) -> Response {
    statistic_response(users, |statistics| statistics.space_assignments)
}

fn statistic_response(
    users: UserService,
    select_count: impl FnOnce(UserStatistics) -> usize,
) -> Response {
    match users.statistics() {
        Ok(statistics) => Json(ApiResponse::success(CountDto {
            count: select_count(statistics),
        }))
        .into_response(),
        Err(error) => user_error_response(error),
    }
}

pub async fn create_user(
    State(users): State<UserService>,
    Json(command): Json<CreateUserRequest>,
) -> Response {
    match users.create_user(command.into()) {
        Ok(user) => (
            StatusCode::CREATED,
            Json(ApiResponse::success(UserDetailDto::from(user))),
        )
            .into_response(),
        Err(error) => user_error_response(error),
    }
}

pub async fn get_user(State(users): State<UserService>, Path(user_code): Path<String>) -> Response {
    match users.get_user(&user_code) {
        Ok(user) => Json(ApiResponse::success(UserDetailDto::from(user))).into_response(),
        Err(error) => user_error_response(error),
    }
}

pub async fn patch_user(
    State(users): State<UserService>,
    Path(user_code): Path<String>,
    Json(command): Json<UpdateUserRequest>,
) -> Response {
    match users.patch_user(&user_code, command.into()) {
        Ok(user) => Json(ApiResponse::success(UserDetailDto::from(user))).into_response(),
        Err(error) => user_error_response(error),
    }
}

pub async fn archive_user(
    State(users): State<UserService>,
    Path(user_code): Path<String>,
) -> Response {
    match users.archive_user(&user_code) {
        Ok(user) => Json(ApiResponse::success(UserDetailDto::from(user))).into_response(),
        Err(error) => user_error_response(error),
    }
}

pub async fn restore_user(
    State(users): State<UserService>,
    Path(user_code): Path<String>,
) -> Response {
    match users.restore_user(&user_code) {
        Ok(user) => Json(ApiResponse::success(UserDetailDto::from(user))).into_response(),
        Err(error) => user_error_response(error),
    }
}

pub async fn replace_roles(
    State(users): State<UserService>,
    Path(user_code): Path<String>,
    Json(request): Json<ReplaceRolesRequest>,
) -> Response {
    match users.replace_roles(&user_code, request.role_codes) {
        Ok(user) => Json(ApiResponse::success(AssignmentDto::from(user))).into_response(),
        Err(error) => user_error_response(error),
    }
}

pub async fn replace_spaces(
    State(users): State<UserService>,
    Path(user_code): Path<String>,
    Json(request): Json<ReplaceSpacesRequest>,
) -> Response {
    match users.replace_spaces(&user_code, request.space_codes) {
        Ok(user) => Json(ApiResponse::success(AssignmentDto::from(user))).into_response(),
        Err(error) => user_error_response(error),
    }
}

pub async fn reset_password(
    State(users): State<UserService>,
    Path(user_code): Path<String>,
) -> Response {
    match users.reset_password(&user_code) {
        Ok(result) => Json(ApiResponse::success(PasswordResetDto::from(result))).into_response(),
        Err(error) => user_error_response(error),
    }
}
