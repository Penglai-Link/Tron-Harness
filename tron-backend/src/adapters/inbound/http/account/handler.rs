use axum::{
    Json,
    extract::State,
    response::{IntoResponse, Response},
};

use crate::{
    adapters::inbound::http::auth::RequestAuthContext,
    application::user::{AccountError, UserService},
    response::ApiResponse,
};

use super::{
    error_mapper::account_error_response,
    request::{ChangePasswordRequest, UpdateAccountProfileRequest},
    response::{AccountProfileDto, PasswordChangeDto},
};

pub async fn get_profile(State(users): State<UserService>, auth: RequestAuthContext) -> Response {
    match users.get_user(&auth.user().user_code) {
        Ok(user) => Json(ApiResponse::success(AccountProfileDto::new(
            user,
            auth.session.auth_method,
        )))
        .into_response(),
        Err(error) => account_error_response(AccountError::from(error)),
    }
}

pub async fn update_profile(
    State(users): State<UserService>,
    auth: RequestAuthContext,
    Json(request): Json<UpdateAccountProfileRequest>,
) -> Response {
    match users.update_account_profile(auth.user(), request.into()) {
        Ok(user) => Json(ApiResponse::success(AccountProfileDto::new(
            user,
            auth.session.auth_method,
        )))
        .into_response(),
        Err(error) => account_error_response(error),
    }
}

pub async fn change_password(
    State(users): State<UserService>,
    auth: RequestAuthContext,
    Json(request): Json<ChangePasswordRequest>,
) -> Response {
    match users.change_account_password(auth.user(), auth.session.auth_method, request.into()) {
        Ok(result) => Json(ApiResponse::success(PasswordChangeDto::from(result))).into_response(),
        Err(error) => account_error_response(error),
    }
}
